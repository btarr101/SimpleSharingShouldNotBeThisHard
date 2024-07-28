use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{headers::Range, TypedHeader};
use axum_range::{KnownSize, Ranged};
use axum_thiserror::ErrorStatus;
use chrono::TimeZone;
use opendal::Operator;
use relative_path::RelativePathBuf;
use uuid::Uuid;

use crate::util::get_directory_for_expiration;

#[derive(thiserror::Error, Debug, ErrorStatus)]
pub enum GetStreamError {
    #[error("Invalid filename is not UUID.EXT.")]
    #[status(StatusCode::BAD_REQUEST)]
    InvalidFileName,
    #[error("UUID needs to be v7.")]
    #[status(StatusCode::BAD_REQUEST)]
    InvalidUUIDVersion,
    #[error("UUID has an invalid timestamp.")]
    #[status(StatusCode::BAD_REQUEST)]
    InvalidUUIDTimestamp,
    #[error("File not found.")]
    #[status(StatusCode::NOT_FOUND)]
    NotFound,
    #[error(transparent)]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    Unkown(#[from] anyhow::Error),
}

pub async fn get_stream(
    State(storage): State<Operator>,
    range: Option<TypedHeader<Range>>,
    Path(file_name): Path<RelativePathBuf>,
) -> Result<impl IntoResponse, GetStreamError> {
    let uuid = file_name
        .file_stem()
        .and_then(|stem| Uuid::try_parse(stem).ok())
        .ok_or(GetStreamError::InvalidFileName)?;

    let expiration_timestamp = uuid
        .get_timestamp()
        .ok_or(GetStreamError::InvalidUUIDVersion)?;
    let (seconds, subsec_nanos) = expiration_timestamp.to_unix();
    let expiration_datetime = match chrono::Utc.timestamp_opt(seconds as i64, subsec_nanos) {
        chrono::offset::LocalResult::Single(datetime) => Ok(datetime),
        _ => Err(GetStreamError::InvalidUUIDTimestamp),
    }?;

    if chrono::Utc::now() >= expiration_datetime {
        Err(GetStreamError::NotFound)
    } else {
        let directory = get_directory_for_expiration(expiration_datetime);
        let file_path = directory.join(&file_name);

        let bytes = storage
            .stat(file_path.as_str())
            .await
            .map_err(|err| GetStreamError::Unkown(err.into()))?
            .content_length();

        // TODO: @ the moment this will read the entire file and only return a portion of it,
        // if I wanted to be efficient I could parse the range myself rather than using the
        // Ranged library. We will see if that's worth though.
        let reader = storage
            .reader(file_path.as_str())
            .await
            .map_err(|err| GetStreamError::Unkown(err.into()))?;

        let body = KnownSize::sized(reader, bytes);
        let range = range.map(|TypedHeader(range)| range);
        Ok(Ranged::new(range, body))
    }
}
