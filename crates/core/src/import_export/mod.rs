use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::storage::entries;
use crate::storage::error::StorageError;
use crate::storage::manifest::{collect_content_ids, Manifest};
use crate::storage::paths::AppPaths;

pub const BACKUP_FORMAT: &str = "switchhosts-backup";
pub const BACKUP_SCHEMA_VERSION: u32 = 1;

pub fn export_v5_backup(manifest: &Manifest, paths: &AppPaths) -> Result<Value, StorageError> {
    let mut content_map = serde_json::Map::new();
    let mut ids = Vec::new();
    collect_content_ids(&manifest.root, &mut ids);
    for id in ids {
        content_map.insert(id.clone(), json!(entries::read_entry(&paths.entries_dir, &id)?));
    }
    Ok(json!({
        "format": BACKUP_FORMAT,
        "schemaVersion": BACKUP_SCHEMA_VERSION,
        "list": manifest.root,
        "content": content_map,
    }))
}

pub fn import_v5_backup(paths: &AppPaths, backup: &Value) -> Result<Manifest, StorageError> {
    let format = backup.get("format").and_then(Value::as_str).unwrap_or("");
    if format != BACKUP_FORMAT {
        return Err(StorageError::UnsupportedBackup {
            reason: format!("expected {BACKUP_FORMAT}, got {format}"),
        });
    }
    let list = backup
        .get("list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if let Some(content) = backup.get("content").and_then(Value::as_object) {
        paths.ensure_layout()?;
        for (id, body) in content {
            let text = body.as_str().unwrap_or("");
            entries::write_entry(&paths.entries_dir, id, text)?;
        }
    }
    Ok(Manifest { root: list, ..Default::default() })
}

pub fn import_from_directory(target: &AppPaths, source: &Path) -> Result<(), StorageError> {
    target.ensure_layout()?;
    for name in ["manifest.json", "trashcan.json"] {
        let src = source.join(name);
        if src.exists() {
            fs::copy(&src, target.root.join(name))
                .map_err(|e| StorageError::io(src.display().to_string(), e))?;
        }
    }
    let src_entries = source.join("entries");
    if src_entries.is_dir() {
        fs::create_dir_all(&target.entries_dir).map_err(|e| {
            StorageError::io(target.entries_dir.display().to_string(), e)
        })?;
        for entry in fs::read_dir(&src_entries).map_err(|e| {
            StorageError::io(src_entries.display().to_string(), e)
        })? {
            let entry = entry.map_err(|e| StorageError::io(src_entries.display().to_string(), e))?;
            fs::copy(entry.path(), target.entries_dir.join(entry.file_name())).map_err(|e| {
                StorageError::io(entry.path().display().to_string(), e)
            })?;
        }
    }
    let src_internal = source.join("internal");
    if src_internal.is_dir() {
        for name in ["config.json", "state.json"] {
            let src = src_internal.join(name);
            if src.exists() {
                let dst = target.root.join("internal").join(name);
                fs::copy(&src, &dst).map_err(|e| StorageError::io(src.display().to_string(), e))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::paths::AppPaths;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn export_import_round_trip() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        entries::write_entry(&paths.entries_dir, "1", "127.0.0.1 x\n").unwrap();
        let manifest = Manifest {
            root: json!([{ "id": "1", "type": "local", "on": true }])
                .as_array()
                .cloned()
                .unwrap(),
            ..Default::default()
        };
        let backup = export_v5_backup(&manifest, &paths).unwrap();
        let tmp2 = TempDir::new().unwrap();
        let paths2 = AppPaths::new(tmp2.path().to_path_buf());
        let imported = import_v5_backup(&paths2, &backup).unwrap();
        assert_eq!(imported.root[0]["id"], "1");
        assert!(entries::read_entry(&paths2.entries_dir, "1")
            .unwrap()
            .contains("127.0.0.1"));
    }
}
