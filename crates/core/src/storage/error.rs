use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("parse error at {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("serialize error at {path}: {source}")]
    Serialize {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("invalid path: {reason}")]
    InvalidPath { reason: String },
    #[error("unknown config key: {key}")]
    UnknownConfigKey { key: String },
    #[error("invalid config value for {key}: {reason}")]
    InvalidConfigValue { key: String, reason: String },
    #[error("node not found: {id}")]
    NodeNotFound { id: String },
    #[error("backup format unsupported: {reason}")]
    UnsupportedBackup { reason: String },
}

impl StorageError {
    pub fn io(path: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    pub fn parse(path: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Parse {
            path: path.into(),
            source,
        }
    }

    pub fn serialize(path: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Serialize {
            path: path.into(),
            source,
        }
    }
}
