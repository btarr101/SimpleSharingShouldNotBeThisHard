use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{headers::Range, TypedHeader};
use axum_range::{KnownSize, Ranged};
use axum_thiserror::ErrorStatus;
use chrono::TimeZone;
use futures::TryStreamExt;
use maud::{html, Markup};
use opendal::Operator;
use relative_path::RelativePathBuf;
use uuid::Uuid;

use crate::util::{
    get_and_validate_multipart_field, get_directory_for_expiration, write_file, MultipartError,
};

#[derive(thiserror::Error, Debug, ErrorStatus)]
pub enum GetError {
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

pub async fn get(
    State(storage): State<Operator>,
    range: Option<TypedHeader<Range>>,
    Path(file_name): Path<RelativePathBuf>,
) -> Result<impl IntoResponse, GetError> {
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

    if chrono::Utc::now() >= expiration_datetime {
        Err(GetError::NotFound)
    } else {
        let directory = get_directory_for_expiration(expiration_datetime);
        // TODO: Here we need to read the various parts and reconstruct the file
        let file_path = directory.join(&file_name).join("0"); // TEMP

        let bytes = storage
            .stat(file_path.as_str())
            .await
            .map_err(|err| GetError::Unkown(err.into()))?
            .content_length();

        // TODO: @ the moment this will read the entire file and only return a portion of it,
        // if I wanted to be efficient I could parse the range myself rather than using the
        // Ranged library. We will see if that's worth though.
        let reader = storage
            .reader(file_path.as_str())
            .await
            .map_err(|err| GetError::Unkown(err.into()))?;

        let body = KnownSize::sized(reader, bytes);
        let range = range.map(|TypedHeader(range)| range);
        Ok(Ranged::new(range, body))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PostError {
    #[error("'{0}' is required!")]
    MissingField(&'static str),
    #[error("Invalid part field.")]
    InvalidPartField,
    #[error("Missing file name.")]
    InvalidFileName,
    #[error("UUID needs to be v7.")]
    InvalidUUIDVersion,
    #[error("UUID has an invalid timestamp.")]
    InvalidUUIDTimestamp,
    #[error("Unexpected error: {0}")]
    Unkown(#[from] anyhow::Error),
}

impl IntoResponse for PostError {
    fn into_response(self) -> axum::response::Response {
        let (status_code, include_retry_button) = match self {
            PostError::Unkown(_) => (StatusCode::INTERNAL_SERVER_ERROR, true),
            _ => (StatusCode::BAD_REQUEST, false),
        };

        (
            status_code,
            html! {
                em {
                    (self.to_string())
                }
                " "
                @if include_retry_button {
                    button type="submit" data-loading-disable data-loading-aria-busy { "Retry" }
                }
                br;
                br;
            },
        )
            .into_response()
    }
}

impl From<MultipartError> for PostError {
    fn from(error: MultipartError) -> Self {
        match error {
            MultipartError::MissingField(field) => PostError::MissingField(field),
            MultipartError::Unkown(err) => PostError::Unkown(err),
        }
    }
}

pub async fn post(
    State(storage): State<Operator>,
    Path(file_name): Path<RelativePathBuf>,
    mut multipart: Multipart,
) -> Result<Markup, PostError> {
    let part_field = get_and_validate_multipart_field("Part", &mut multipart).await?;
    let part = part_field
        .text()
        .await
        .map_err(|_| PostError::InvalidPartField)?
        .parse::<usize>()
        .map_err(|_| PostError::InvalidPartField)?;

    // Useful for testing retry handling...
    // let uuid = Uuid::new_v4().as_u128();
    // if uuid % 2 == 0 {
    //     Err(PostError::Unkown(anyhow::anyhow!("DEBUG")))?;
    // }

    let uuid = file_name
        .file_stem()
        .and_then(|stem| Uuid::try_parse(stem).ok())
        .ok_or(PostError::InvalidFileName)?;

    let expiration_timestamp = uuid.get_timestamp().ok_or(PostError::InvalidUUIDVersion)?;
    let (seconds, subsec_nanos) = expiration_timestamp.to_unix();
    let expiration_datetime = match chrono::Utc.timestamp_opt(seconds as i64, subsec_nanos) {
        chrono::offset::LocalResult::Single(datetime) => Ok(datetime),
        _ => Err(PostError::InvalidUUIDTimestamp),
    }?;

    // TODO stat this directory???

    let file_field = get_and_validate_multipart_field("File", &mut multipart).await?;
    let body_with_io_error = file_field
        .map_err(|err| opendal::Error::new(opendal::ErrorKind::Unexpected, &err.body_text()));

    let directory = get_directory_for_expiration(expiration_datetime);
    let part_path = directory.join(file_name.as_str()).join(part.to_string());

    tracing::info!("Writing part {}", part);

    write_file(&part_path, body_with_io_error, &storage)
        .await
        .map_err(|err| PostError::Unkown(err.into()))?;

    tracing::info!("Finished upload with part {}", part);

    // TODO DEBUG NO PART 0 but FOLDER!!!
    // Also error responses!

    Ok(html! {})
}
