use std::time::Duration;

use axum::{
    extract::{Multipart, State},
    response::{IntoResponse, Response},
};
use maud::{html, Markup, Render};
use opendal::Operator;
use relative_path::RelativePath;

use crate::{
    components::page::page,
    util::{
        get_and_validate_multipart_field, get_directory_for_expiration, DatetimeUUIDv7GeneratorExt,
        MultipartError,
    },
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
            fieldset {
                form method="post" enctype="multipart/form-data" hx-swap="none"
                _="on htmx:configRequest(event) if event.detail.elt is me js configMultipartRequest(event) end end" {
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
                    br;br;
                    @if let Some(error) = error {
                        em id="error" data-loading-hidden {
                            (error)
                        }
                        br;br;
                    }
                }
                div id="file-uploader" {}
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

impl From<MultipartError> for PostError {
    fn from(error: MultipartError) -> Self {
        match error {
            MultipartError::MissingField(field) => PostError::MissingField(field),
            MultipartError::Unkown(error) => PostError::Unkown(error),
        }
    }
}

/// Generates a presigned URL for uploading a new file.
pub async fn post(
    State(storage): State<Operator>,
    mut multipart: Multipart,
) -> Result<Markup, PostError> {
    // Use the Share For field to create a timestamped UUID with the expiration date
    // This lets us avoid needing to use any sort of other persistance such as a
    // database.
    let share_for_field = get_and_validate_multipart_field("Share for", &mut multipart)
        .await
        .map_err(PostError::from)?;
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

    let file_name_field = get_and_validate_multipart_field("Filename", &mut multipart)
        .await
        .map_err(PostError::from)?;
    let file_name = file_name_field
        .text()
        .await
        .map_err(|err| PostError::Unkown(err.into()))?;

    if file_name.is_empty() {
        return Err(PostError::MissingFileName);
    }

    let extension = RelativePath::new(&file_name)
        .extension()
        .ok_or(PostError::UnknownFileType)?
        .to_string();

    let directory = get_directory_for_expiration(expiration_datetime);
    let uuid_string = expiration_datetime.generate_uuidv7().to_string();
    let file_name = format!("{uuid_string}.{extension}");
    let file_path = directory.join(&file_name);

    let mut presigned_request =
        storage.presign_write_with(file_path.as_str(), Duration::new(3600, 0));

    if let Some(mime_type) = mime_guess::from_ext(&extension).first() {
        presigned_request = presigned_request.content_type(mime_type.essence_str())
    }

    let presigned_request = presigned_request
        .await
        .map_err(|err| PostError::Unkown(err.into()))?;

    Ok(html!(
        div
        id="file-uploader"
        hx-put=(presigned_request.uri())
        hx-include="#file"
        _=(format!("
            on htmx:beforeRequest(event) call beforePresignedRequest(event)
            on htmx:xhr:progress(loaded, total, detail)
                if detail.elt is me
                    set #progress.value to (loaded/total)*100
                end
            on htmx:beforeSwap(event)
                if event.detail.xhr.status is not 200
                    set #progress.value to 0
                else
                    go to url /file/{file_name}/view
                end
        "))
        hx-trigger="load delay:0.1s"
        hx-swap-oob="true" {
            progress id="progress" value=(0) max=(100) {}
        }
    ))
}
