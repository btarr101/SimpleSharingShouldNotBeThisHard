use chrono::{DateTime, DurationRound, TimeDelta, TimeZone, Utc};
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
