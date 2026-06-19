//! Apply history persistence: `internal/histories/system-hosts.json`.
//!
//! On-disk format mirrors SwitchHosts — a bare JSON array of journal entries:
//!
//! ```json
//! [
//!   { "id": "uuid", "content": "...", "add_time_ms": 1700000000000 },
//!   ...
//! ]
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::storage::atomic::atomic_write;
use crate::storage::error::StorageError;

use super::error::ApplyError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyHistoryItem {
    pub id: String,
    pub content: String,
    pub add_time_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

pub fn system_hosts_history_path(histories_dir: &Path) -> PathBuf {
    histories_dir.join("system-hosts.json")
}

pub fn load(path: &Path) -> Result<Vec<ApplyHistoryItem>, ApplyError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(path).map_err(|e| ApplyError::Storage(StorageError::io(path.display().to_string(), e)))?;
    match serde_json::from_slice::<Vec<ApplyHistoryItem>>(&bytes) {
        Ok(v) => Ok(v),
        Err(_) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(Value::Array(arr)) => Ok(arr
                .into_iter()
                .filter_map(|v| serde_json::from_value::<ApplyHistoryItem>(v).ok())
                .collect()),
            _ => {
                tracing::warn!("{} could not be parsed; treating as empty.", path.display());
                Ok(Vec::new())
            }
        },
    }
}

pub fn save(path: &Path, items: &[ApplyHistoryItem]) -> Result<(), ApplyError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(items)?;
    atomic_write(path, &bytes).map_err(ApplyError::from)
}

/// Journal order: oldest first, newest last.
pub fn list_history(histories_dir: &Path) -> Result<Vec<ApplyHistoryItem>, ApplyError> {
    let path = system_hosts_history_path(histories_dir);
    if path.exists() {
        return load(&path);
    }
    migrate_legacy_files(histories_dir, &path)
}

pub fn insert(
    path: &Path,
    item: ApplyHistoryItem,
    history_limit: u32,
) -> Result<(), ApplyError> {
    let mut items = if path.exists() {
        load(path)?
    } else {
        migrate_legacy_files(path.parent().unwrap_or(path), path)?
    };
    items.push(item);
    if history_limit > 0 && items.len() > history_limit as usize {
        let drop_count = items.len() - history_limit as usize;
        items.drain(0..drop_count);
    }
    save(path, &items)
}

/// Remove the entry with `id`. Returns true if a row was removed.
pub fn delete_by_id(histories_dir: &Path, id: &str) -> Result<bool, ApplyError> {
    let path = system_hosts_history_path(histories_dir);
    let mut items = list_history(histories_dir)?;
    let before = items.len();
    items.retain(|i| i.id != id);
    if items.len() == before {
        return Ok(false);
    }
    save(&path, &items)?;
    Ok(true)
}

pub fn append_history(
    histories_dir: &Path,
    content: &str,
    limit: u32,
) -> Result<(), ApplyError> {
    let path = system_hosts_history_path(histories_dir);
    let item = ApplyHistoryItem {
        id: Uuid::new_v4().to_string(),
        content: content.to_owned(),
        add_time_ms: Utc::now().timestamp_millis(),
        label: None,
    };
    insert(&path, item, limit)
}

fn migrate_legacy_files(
    histories_dir: &Path,
    target_path: &Path,
) -> Result<Vec<ApplyHistoryItem>, ApplyError> {
    let mut items = Vec::new();
    let mut legacy_paths = Vec::new();

    if histories_dir.is_dir() {
        for entry in fs::read_dir(histories_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.file_name().is_some_and(|n| n == "system-hosts.json") {
                continue;
            }
            if path.extension().is_some_and(|x| x == "json") {
                if let Some(item) = parse_legacy_entry(&path) {
                    items.push(item);
                    legacy_paths.push(path);
                }
            }
        }
    }

    items.sort_by_key(|i| i.add_time_ms);
    if !items.is_empty() {
        save(target_path, &items)?;
        for legacy in legacy_paths {
            let _ = fs::remove_file(legacy);
        }
    }
    Ok(items)
}

fn parse_legacy_entry(path: &Path) -> Option<ApplyHistoryItem> {
    let bytes = fs::read(path).ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    let content = value.get("content")?.as_str()?.to_owned();
    let add_time_ms = value
        .get("add_time_ms")
        .and_then(|v| v.as_i64())
        .or_else(|| {
            value
                .get("writtenAt")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp_millis())
        })
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.parse::<i64>().ok())
        })
        .unwrap_or_else(|| Utc::now().timestamp_millis());
    Some(ApplyHistoryItem {
        id: value
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string()),
        content,
        add_time_ms,
        label: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn respects_history_limit() {
        let tmp = TempDir::new().unwrap();
        for i in 0..5 {
            append_history(tmp.path(), &format!("content-{i}"), 2).unwrap();
        }
        let items = list_history(tmp.path()).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].content, "content-3");
        assert_eq!(items[1].content, "content-4");
    }

    #[test]
    fn delete_by_id_removes_entry() {
        let tmp = TempDir::new().unwrap();
        append_history(tmp.path(), "a", 10).unwrap();
        append_history(tmp.path(), "b", 10).unwrap();
        let items = list_history(tmp.path()).unwrap();
        assert_eq!(items.len(), 2);
        assert!(delete_by_id(tmp.path(), &items[0].id).unwrap());
        let remaining = list_history(tmp.path()).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].content, "b");
    }

    #[test]
    fn migrates_legacy_per_file_entries() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path()).unwrap();
        let legacy = tmp.path().join("1700000000001.json");
        fs::write(
            &legacy,
            r#"{"content":"legacy","writtenAt":"2023-11-14T22:13:20.001Z"}"#,
        )
        .unwrap();
        let items = list_history(tmp.path()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content, "legacy");
        assert!(!legacy.exists());
        assert!(system_hosts_history_path(tmp.path()).exists());
    }
}
