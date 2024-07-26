use axum::{
    extract::{Path, State},
    response::{IntoResponse, Response},
};
use chrono::TimeZone;
use maud::{html, Markup};
use opendal::Operator;
use relative_path::RelativePathBuf;
use uuid::Uuid;

use crate::{
    components::{internal_server_error::internal_server_error, not_found::not_found, page::page},
    util::get_directory_for_expiration,
};

#[derive(thiserror::Error, Debug)]
pub enum GetSharedError {
    #[error("Invalid filename is not UUID.EXT.")]
    InvalidFileName,
    #[error("UUID has an invalid timestamp.")]
    InvalidUUIDTimestamp,
    #[error(transparent)]
    Unkown(#[from] anyhow::Error),
}

impl IntoResponse for GetSharedError {
    fn into_response(self) -> Response {
        match self {
            GetSharedError::InvalidFileName | GetSharedError::InvalidUUIDTimestamp => not_found(),
            GetSharedError::Unkown(_) => internal_server_error(),
        }
        .into_response()
    }
}

pub async fn get_shared(
    State(storage): State<Operator>,
    Path(file_name): Path<RelativePathBuf>,
) -> Result<Markup, GetSharedError> {
    let uuid = file_name
        .file_stem()
        .and_then(|stem| Uuid::try_parse(stem).ok())
        .ok_or(GetSharedError::InvalidFileName)?;

    let expiration_timestamp = uuid.get_timestamp().unwrap();
    let (seconds, subsec_nanos) = expiration_timestamp.to_unix();
    let expiration_datetime = match chrono::Utc.timestamp_opt(seconds as i64, subsec_nanos) {
        chrono::offset::LocalResult::Single(datetime) => Ok(datetime),
        _ => Err(GetSharedError::InvalidUUIDTimestamp),
    }?;

    let file_exists = if chrono::Utc::now() < expiration_datetime {
        let directory = get_directory_for_expiration(expiration_datetime);

        storage
            .is_exist(directory.join(&file_name).as_str())
            .await
            .map_err(|err| GetSharedError::Unkown(err.into()))?
    } else {
        false
    };

    let file_source = format!("/stream/{file_name}");

    Ok(page(html! {
        h1 { (file_name)}
        @if file_exists {
            video controls {
                source src=(file_source) type="video/mp4" {}
            }
            div {
                a href=(file_source) download=(file_name) { "Download" };
            }
        } @else {
            p { "Expired or not found" }
        }
    }))
}
