use axum::{
    extract::{multipart::Field, Multipart, State},
    response::{IntoResponse, Redirect, Response},
};
use futures::TryStreamExt;
use maud::{html, Markup, Render};
use opendal::Operator;
use relative_path::RelativePath;
use uuid::{NoContext, Timestamp, Uuid};

use crate::{components::page::page, util::get_directory_for_expiration};

fn index_page(error: Option<&dyn Render>) -> Markup {
    page(html! {
        form method="post" enctype="multipart/form-data" {
            p {
                label for="share-for" { "Share for " }
                select id="share-for" name="Share for" {
                    option { "30 seconds" }
                    option { "15 minutes" }
                    option { "1 hour" }
                    option { "1 day" }
                    option { "1 week" }
                    option { "DEBUG -2 hours" }
                }
            }
            p {
                input type="file" name="File" {}
            }
            input type="submit" accept="audio/*,video/*,image/*" {}
            @if let Some(error) = error {
                div {
                    (error)
                }
            }
        }
    })
}

#[derive(thiserror::Error, Debug)]
pub enum PostIndexError {
    #[error("'{0}' is required!")]
    MissingField(&'static str),
    #[error("Missing file name.")]
    MissingFileName,
    #[error("Unkown file type.")]
    UnknownFileType,
    #[error(transparent)]
    Unkown(#[from] anyhow::Error),
}

impl IntoResponse for PostIndexError {
    fn into_response(self) -> Response { index_page(Some(&self.to_string())).into_response() }
}

pub async fn get_index() -> Markup { index_page(None) }

async fn get_and_validate_multipart_field<'a>(
    field_name: &'static str,
    multipart: &'a mut Multipart,
) -> Result<Field<'a>, PostIndexError> {
    let field = multipart
        .next_field()
        .await
        .map_err(|err| PostIndexError::Unkown(err.into()))?
        .ok_or(PostIndexError::MissingField(field_name))?;

    if field.name() != Some(field_name) {
        Err(PostIndexError::MissingField(field_name))
    } else {
        Ok(field)
    }
}

pub async fn post_index(
    State(storage): State<Operator>,
    mut multipart: Multipart,
) -> Result<Redirect, PostIndexError> {
    // Use the Share For field to create a timestamped UUID with the expiration date
    // This lets us avoid the need to use any sort of other persistance such as a
    // database.
    let share_for_field = get_and_validate_multipart_field("Share for", &mut multipart).await?;
    let share_for_field_value = share_for_field
        .text()
        .await
        .map_err(|err| PostIndexError::Unkown(err.into()))?;
    let share_for = match share_for_field_value.as_str() {
        "30 seconds" => chrono::Duration::seconds(30),
        "1 hour" => chrono::Duration::hours(1),
        "1 day" => chrono::Duration::days(1),
        "1 week" => chrono::Duration::weeks(1),
        "DEBUG -2 hours" => chrono::Duration::hours(-2),
        _ => chrono::Duration::minutes(15),
    };
    let expiration_datetime = chrono::Utc::now() + share_for;
    let timestamp = Timestamp::from_unix(
        NoContext,
        expiration_datetime.timestamp() as u64,
        expiration_datetime.timestamp_subsec_nanos(),
    );
    let uuid = Uuid::new_v7(timestamp).to_string();

    // Use the UUID as the filename.
    let file_field = get_and_validate_multipart_field("File", &mut multipart).await?;
    let file_name = file_field
        .file_name()
        .ok_or(PostIndexError::MissingFileName)?;

    if file_name.is_empty() {
        return Err(PostIndexError::MissingField("File"));
    }

    let extension = RelativePath::new(file_name)
        .extension()
        .ok_or(PostIndexError::UnknownFileType)?
        .to_string();

    let body_with_io_error = file_field
        .map_err(|err| opendal::Error::new(opendal::ErrorKind::Unexpected, &err.body_text()));

    let directory = get_directory_for_expiration(expiration_datetime);
    let file_path = directory.join(format!("{uuid}.{extension}"));

    let mut writer = storage
        .writer(file_path.as_str())
        .await
        .map_err(|err| PostIndexError::Unkown(err.into()))?;
    let sink_result = writer
        .sink(body_with_io_error)
        .await
        .map_err(|err| PostIndexError::Unkown(err.into()));
    writer
        .close()
        .await
        .map_err(|err| PostIndexError::Unkown(err.into()))?;

    // We want to make sure the writer is closed before propagating an error,
    // which is why we don't propagate the sink result until after the close
    // operation.
    sink_result?;

    Ok(Redirect::to(&format!("/shared/{uuid}.{extension}")))
}
