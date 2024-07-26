use futures::StreamExt;
use opendal::{Entry, Operator};
use relative_path::{Component, RelativePath, RelativePathBuf};

use crate::util::{get_expiration_for_directory, GetDirectoryExpirationError};

#[derive(thiserror::Error, Debug)]
pub enum CleanupError {
    #[error("Unable to list directories: {0}")]
    UnableToListDirectories(opendal::Error),
}

pub async fn cleanup(storage: Operator) -> Result<(), CleanupError> {
    tracing::debug!("Entering cleanup...");

    let lister = storage
        .lister("")
        .await
        .map_err(CleanupError::UnableToListDirectories)?;

    lister
        .for_each(|entry| {
            let storage = storage.clone();
            async move {
                match entry {
                    Err(err) => {
                        tracing::error!("Error listing entry: {err}");
                    }
                    Ok(entry) => {
                        if let Err(err) = cleanup_entry(entry, storage).await {
                            tracing::error!("{err}");
                        }
                    }
                }
            }
        })
        .await;

    Ok(())
}

#[allow(clippy::enum_variant_names)]
#[derive(thiserror::Error, Debug)]
pub enum CleanupEntryError {
    #[error("Issue reading directory from entry path: {0}")]
    UnableToParseDirectory(RelativePathBuf),
    #[error("Unable to parse directory expiration for '{0}': {1}")]
    UnableToParseDirectoryExpiration(RelativePathBuf, GetDirectoryExpirationError),
    #[error("Unable to remove directory '{0}': {1}")]
    UnableToRemoveDirectory(RelativePathBuf, opendal::Error),
}

async fn cleanup_entry(entry: Entry, storage: Operator) -> Result<(), CleanupEntryError> {
    let path = entry.path();
    if let Some(Component::Normal(directory)) = RelativePath::new(path).components().next() {
        let directory_expiration = get_expiration_for_directory(RelativePath::new(directory))
            .map_err(|err| {
                CleanupEntryError::UnableToParseDirectoryExpiration(directory.into(), err)
            })?;

        if chrono::Utc::now() >= directory_expiration {
            storage
                .remove_all(&format!("{directory}/"))
                .await
                .map_err(|err| CleanupEntryError::UnableToRemoveDirectory(directory.into(), err))?;

            tracing::info!(
                "Removed expired directory '{directory}', which expired {directory_expiration}"
            );
        } else {
            tracing::debug!("'{directory}' lives for now... at least until {directory_expiration}");
        }

        Ok(())
    } else {
        Err(CleanupEntryError::UnableToParseDirectory(path.into()))
    }
}
