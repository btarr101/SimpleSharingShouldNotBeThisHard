use std::str::from_utf8;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, TimeZone, Utc};
use humantime::format_duration;
use maud::{html, Markup};
use mime_guess::{mime, Mime};
use opendal::Operator;
use relative_path::{RelativePath, RelativePathBuf};
use uuid::Uuid;

use crate::{
    components::{error_page::error_page, page::page},
    util::get_directory_for_expiration,
};

#[derive(thiserror::Error, Debug)]
pub enum GetError {
    #[error("Invalid filename is not UUID.EXT.")]
    InvalidFileName,
    #[error("UUID needs to be v7.")]
    InvalidUUIDVersion,
    #[error("UUID has an invalid timestamp.")]
    InvalidUUIDTimestamp,
    #[error(transparent)]
    Unkown(#[from] anyhow::Error),
}

impl IntoResponse for GetError {
    fn into_response(self) -> Response {
        match self {
            GetError::InvalidFileName
            | GetError::InvalidUUIDTimestamp
            | GetError::InvalidUUIDVersion => error_page(
                StatusCode::NOT_FOUND,
                "File name is invalid and therefore could not be found.",
            ),
            GetError::Unkown(_) => error_page(
                StatusCode::INTERNAL_SERVER_ERROR,
                "An unknown error occured when trying view file.",
            ),
        }
        .into_response()
    }
}

pub async fn get(
    State(storage): State<Operator>,
    Path(file_name): Path<RelativePathBuf>,
) -> Result<Markup, GetError> {
    let uuid = file_name
        .file_stem()
        .and_then(|stem| Uuid::try_parse(stem).ok())
        .ok_or(GetError::InvalidFileName)?;

    let expiration_timestamp = uuid.get_timestamp().ok_or(GetError::InvalidUUIDVersion)?;
    let (seconds, subsec_nanos) = expiration_timestamp.to_unix();
    let expiration_datetime = match chrono::Utc.timestamp_opt(seconds as i64, subsec_nanos) {
        chrono::offset::LocalResult::Single(datetime) => Ok(datetime),
        _ => Err(GetError::InvalidUUIDTimestamp),
    }?;

    let now = chrono::Utc::now();
    let file_exists = if now < expiration_datetime {
        let directory = get_directory_for_expiration(expiration_datetime);
        let path = directory.join(format!("{file_name}/"));

        match storage.stat(path.as_str()).await {
            Ok(_) => Ok(true),
            Err(err) => match err.kind() {
                opendal::ErrorKind::NotFound => Ok(false),
                _ => Err(GetError::Unkown(err.into())),
            },
        }?
    } else {
        false
    };

    let file_source = format!("/file/{file_name}");
    let expires_in = (expiration_datetime - now)
        .to_std()
        .map(|duration| {
            format_duration(std::time::Duration::from_secs(duration.as_secs())).to_string()
        })
        .unwrap_or_else(|_| "UNABLE TO PARSE".into());

    let mime_type = file_name
        .extension()
        .and_then(|extension| mime_guess::from_ext(extension).first());

    tracing::debug!("{:?}", mime_type);

    let file_viewer = if let Some(possible_viewer) =
        mime_type.map(|mime| file_viewer(&file_name, mime, expiration_datetime, &storage))
    {
        Some(possible_viewer.await)
    } else {
        None
    }
    .transpose()
    .inspect_err(|err| tracing::error!("Failed to create viewer for {}: {}", &file_name, err))
    .ok()
    .flatten()
    .flatten();

    let timer_script = format!(
        "init repeat forever wait 1s then js return formatDuration(new Date(\"{}\") - new Date()) end then put it into me end",
        expiration_datetime
    );

    Ok(page(
        html! {
            fieldset {
                h2 { "Viewing " code { (file_name) }}
                @if file_exists {
                    p {
                        "This file expires in "
                        time _=(timer_script) {
                            (expires_in)
                        }
                        "."
                    }
                    ul {
                        li { a href=(file_source) download=(file_name) { "Download" } }
                        br;
                        li {
                            a href="" { "Share" }
                            " (Right click and choose \"Copy Link Address\")"
                        }
                    }
                    @if let Some(file_viewer) = file_viewer {
                        br;
                        (file_viewer)
                        br;
                    }
                } @else {
                    p { "This file has either expired or never even existed in the first place." }
                }
            }
        },
        false,
    ))
}

async fn file_viewer(
    file_name: &RelativePath,
    mime: Mime,
    expiration_datetime: DateTime<Utc>,
    storage: &Operator,
) -> anyhow::Result<Option<Markup>> {
    let file_source = format!("/file/{file_name}");
    match (mime.type_(), mime.subtype()) {
        (mime::VIDEO, _) => Ok(Some(html!(
            center {
                video controls {
                    source src=(file_source) type=(mime.to_string());
                }
            }
        ))),
        (mime::IMAGE, _) => Ok(Some(html!(
            center {
                img src=(file_source) alt="Shared image";
            }
        ))),
        (mime::AUDIO, _) => Ok(Some(html!(
            center {
                audio controls {
                    source src=(file_source) type=(mime.to_string());
                }
            }
        ))),
        (mime::TEXT, _) => {
            let directory = get_directory_for_expiration(expiration_datetime);
            let file_path = directory.join(file_name);

            let bytes = storage.read(file_path.as_str()).await?;
            let content = from_utf8(&bytes)?;

            Ok(Some(html!(
                hr;
                pre {
                    code {
                        (content)
                    }
                }
                hr;
            )))
        }
        _ => Ok(None),
    }
}
