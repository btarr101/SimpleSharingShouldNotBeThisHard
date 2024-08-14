use axum::extract::{multipart::Field, Multipart};
use chrono::{DateTime, DurationRound, TimeDelta, TimeZone, Utc};
use futures::Stream;
use opendal::Operator;
use relative_path::{RelativePath, RelativePathBuf};
use uuid::{NoContext, Timestamp, Uuid};

pub fn get_directory_for_expiration(datetime: DateTime<Utc>) -> RelativePathBuf {
    let duration = TimeDelta::hours(1);

    (datetime + duration)
        .duration_trunc(duration) // we essentially want to ceil
        .expect("no rounding error")
        .timestamp()
        .to_string()
        .into()
}

#[derive(thiserror::Error, Debug)]
pub enum GetDirectoryExpirationError {
    #[error("Invalid UUID timestamp.")]
    InvalidUUIDTimestamp,
    #[error("Invalid Timestamp.")]
    InvalidTimestamp,
}

pub fn get_expiration_for_directory(
    directory: &RelativePath,
) -> Result<DateTime<Utc>, GetDirectoryExpirationError> {
    let timestamp = directory
        .to_string()
        .parse::<i64>()
        .map_err(|_| GetDirectoryExpirationError::InvalidUUIDTimestamp)?;

    match Utc.timestamp_opt(timestamp, 0) {
        chrono::offset::LocalResult::Single(datetime) => Ok(datetime),
        _ => Err(GetDirectoryExpirationError::InvalidTimestamp),
    }
}

pub trait DatetimeUUIDv7GeneratorExt {
    fn generate_uuidv7(&self) -> Uuid;
}

impl DatetimeUUIDv7GeneratorExt for chrono::DateTime<Utc> {
    fn generate_uuidv7(&self) -> Uuid {
        let timestamp = Timestamp::from_unix(
            NoContext,
            self.timestamp() as u64,
            self.timestamp_subsec_nanos(),
        );
        Uuid::new_v7(timestamp)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MultipartError {
    #[error("'{0}' is required!")]
    MissingField(&'static str),
    #[error("Unkown error.")]
    Unkown(#[from] anyhow::Error),
}

pub async fn get_and_validate_multipart_field<'a>(
    field_name: &'static str,
    multipart: &'a mut Multipart,
) -> Result<Field<'a>, MultipartError> {
    let field = get_next_multipart_field(multipart)
        .await?
        .ok_or(MultipartError::MissingField(field_name))?;

    if field.name() != Some(field_name) {
        Err(MultipartError::MissingField(field_name))
    } else {
        Ok(field)
    }
}

pub async fn get_next_multipart_field(
    multipart: &mut Multipart,
) -> Result<Option<Field<'_>>, MultipartError> {
    multipart
        .next_field()
        .await
        .map_err(|err| MultipartError::Unkown(err.into()))
}

pub async fn write_file<S, T>(
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
        .concurrent(1) // 50 mb so s3 doesn't whine
        .await?;
    let sink_result = writer.sink(body).await;
    writer.close().await?;

    // We want to make sure the writer is closed before propagating an error,
    // which is why we don't propagate the sink result until after the close
    // operation.
    sink_result.map(|_| ())
}
