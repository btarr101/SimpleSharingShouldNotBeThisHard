use axum::{
    extract::{multipart::Field, Multipart, State},
    response::{IntoResponse, Redirect, Response},
};
use chrono::{DateTime, Utc};
use futures::{Stream, TryStreamExt};
use maud::{html, Markup, Render};
use opendal::Operator;
use relative_path::RelativePath;

use crate::{
    components::page::page,
    util::{get_directory_for_expiration, DatetimeUUIDv7GeneratorExt},
};

static SHARE_FOR_OPTIONS: phf::OrderedMap<&str, chrono::Duration> = phf::phf_ordered_map! {
    "30 minutes" => chrono::Duration::minutes(30),
    "1 hour" => chrono::Duration::hours(1),
    "6 hours" => chrono::Duration::hours(6),
    "12 hours" => chrono::Duration::hours(12),
    "1 day" => chrono::Duration::days(1),
    "3 days" => chrono::Duration::days(3)
};

fn index_page(error: Option<&dyn Render>) -> Markup {
    page(
        html! {
            form method="post" enctype="multipart/form-data"
            _="on submit(event) set #progress.value to 0 then js chunkedSubmitHandler(event) end on htmx:xhr:progress(loaded, total) set #progress.value to (loaded/total)*100" {
                fieldset {
                    h2 { "Share file" }
                    label for="share-for" { "Share for: " }
                    select id="share-for" name="Share for" {
                        @for &share_for_option in SHARE_FOR_OPTIONS.keys() {
                            option { (share_for_option) }
                        }
                    }
                    br;br;
                    label for="file" { "File: " }
                    input id="file" type="file" accept="*" name="File" required;
                    br;br;
                    input type="submit" data-loading-disable data-loading-aria-busy;
                    br;
                    br;
                    progress id="progress" data-loading value=(0) max=(100) {};
                    @if let Some(error) = error {
                        em id="error" data-loading-hidden {
                            (error)
                        }
                        br;br;
                    }
                }
            }
        },
        true,
    )
}

pub async fn get() -> Markup { index_page(None) }

#[derive(thiserror::Error, Debug)]
pub enum PostError {
    #[error("'{0}' is required!")]
    MissingField(&'static str),
    #[error("Missing file name.")]
    MissingFileName,
    #[error("Unkown file type.")]
    UnknownFileType,
    #[error("Unkown error.")]
    Unkown(#[from] anyhow::Error),
}

impl IntoResponse for PostError {
    fn into_response(self) -> Response {
        if let PostError::Unkown(error) = &self {
            tracing::error!("Unkown error encountered for user: {error}");
        }

        index_page(Some(&self.to_string())).into_response()
    }
}

pub async fn post(
    State(storage): State<Operator>,
    mut multipart: Multipart,
) -> Result<Redirect, PostError> {
    // Use the Share For field to create a timestamped UUID with the expiration date
    // This lets us avoid needing to use any sort of other persistance such as a
    // database.
    let share_for_field = get_and_validate_multipart_field("Share for", &mut multipart).await?;
    let share_for_field_value = share_for_field
        .text()
        .await
        .map_err(|err| PostError::Unkown(err.into()))?;

    let share_for = *SHARE_FOR_OPTIONS
        .get(share_for_field_value.as_str())
        .unwrap_or(
            SHARE_FOR_OPTIONS
                .values()
                .next()
                .expect("at least one share for option"),
        );
    let expiration_datetime = chrono::Utc::now() + share_for;

    let field = get_next_multipart_field(&mut multipart)
        .await?
        .ok_or(PostError::MissingField("File or Parts"))?;
    match field.name() {
        Some("File") => {
            upload_file_in_single_part_and_redirect(field, expiration_datetime, &storage).await
        }
        Some("Parts") => todo!(),
        _ => Err(PostError::MissingField("File or Parts")),
    }
}

async fn upload_file_in_single_part_and_redirect<'a>(
    file_field: Field<'a>,
    expiration_datetime: DateTime<Utc>,
    storage: &Operator,
) -> Result<Redirect, PostError> {
    let file_name = file_field
        .file_name()
        .ok_or(PostError::MissingFileName)?
        .to_string();

    if file_name.is_empty() {
        return Err(PostError::MissingFileName);
    }

    let extension = RelativePath::new(&file_name)
        .extension()
        .ok_or(PostError::UnknownFileType)?
        .to_string();

    let body_with_io_error = file_field
        .map_err(|err| opendal::Error::new(opendal::ErrorKind::Unexpected, &err.body_text()));

    let directory = get_directory_for_expiration(expiration_datetime);
    let uuid_string = expiration_datetime.generate_uuidv7().to_string();
    let file_path = directory
        .join(format!("{uuid_string}.{extension}"))
        .join("0");

    write_file(&file_path, body_with_io_error, storage)
        .await
        .map_err(|err| PostError::Unkown(err.into()))?;

    Ok(Redirect::to(&format!(
        "/file/{uuid_string}.{extension}/view"
    )))
}

async fn get_and_validate_multipart_field<'a>(
    field_name: &'static str,
    multipart: &'a mut Multipart,
) -> Result<Field<'a>, PostError> {
    let field = get_next_multipart_field(multipart)
        .await?
        .ok_or(PostError::MissingField(field_name))?;

    if field.name() != Some(field_name) {
        Err(PostError::MissingField(field_name))
    } else {
        Ok(field)
    }
}

async fn get_next_multipart_field(
    multipart: &mut Multipart,
) -> Result<Option<Field<'_>>, PostError> {
    multipart
        .next_field()
        .await
        .map_err(|err| PostError::Unkown(err.into()))
}

async fn write_file<S, T>(
    file_path: &RelativePath,
    body: S,
    storage: &Operator,
) -> Result<(), opendal::Error>
where
    S: Stream<Item = opendal::Result<T>>,
    T: Into<axum::body::Bytes>,
{
    let mut writer = storage
        .writer_with(file_path.as_str())
        .buffer(625000)
        .concurrent(4) // 50 mb so s3 doesn't whine
        .await?;
    let sink_result = writer.sink(body).await;
    writer.close().await?;

    // We want to make sure the writer is closed before propagating an error,
    // which is why we don't propagate the sink result until after the close
    // operation.
    sink_result.map(|_| ())
}
