use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplyError {
    #[error(transparent)]
    Storage(#[from] crate::storage::error::StorageError),
    #[error("elevation failed: {0}")]
    Elevation(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
