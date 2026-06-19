use std::fs;
use std::io::Read;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::storage::atomic::atomic_write;
use crate::storage::entries;
use crate::storage::error::StorageError;
use crate::storage::manifest::{collect_content_ids, Manifest};
use crate::storage::paths::AppPaths;
use crate::storage::trashcan::{TrashItem, Trashcan};
use crate::storage::tree_format::v5_root_to_legacy;

pub const BACKUP_FORMAT: &str = "switchhosts-backup";
pub const BACKUP_SCHEMA_VERSION: u32 = 1;

pub const ERR_PARSE: &str = "parse_error";
pub const ERR_INVALID_DATA: &str = "invalid_data";
pub const MAX_IMPORT_BACKUP_BYTES: usize = 64 * 1024 * 1024;

/// Legacy CLI / stdout export shape (`list` + `content`).
pub fn export_v5_backup(manifest: &Manifest, paths: &AppPaths) -> Result<Value, StorageError> {
    let mut content_map = serde_json::Map::new();
    let mut ids = Vec::new();
    collect_content_ids(&manifest.root, &mut ids);
    for id in ids {
        if id == "0" {
            continue;
        }
        content_map.insert(
            id.clone(),
            json!(entries::read_entry(&paths.entries_dir, &id)?),
        );
    }
    Ok(json!({
        "format": BACKUP_FORMAT,
        "schemaVersion": BACKUP_SCHEMA_VERSION,
        "list": manifest.root,
        "content": content_map,
    }))
}

/// Write a SwitchHosts-compatible v5 backup JSON to `dest`.
pub fn export_to_file(dest: &Path, paths: &AppPaths) -> Result<(), StorageError> {
    let manifest = Manifest::load(paths).unwrap_or_default();
    let trashcan = Trashcan::load(&paths.trashcan_file);

    let mut ids = Vec::new();
    collect_content_ids(&manifest.root, &mut ids);

    let mut entries_map = serde_json::Map::with_capacity(ids.len());
    for id in ids {
        if id == "0" {
            continue;
        }
        let content = entries::read_entry(&paths.entries_dir, &id)?;
        entries_map.insert(id, Value::String(content));
    }

    let backup = json!({
        "format": BACKUP_FORMAT,
        "schemaVersion": BACKUP_SCHEMA_VERSION,
        "version": [5, 0, 0, 0],
        "exportedAt": chrono::Utc::now().to_rfc3339(),
        "manifest": {
            "format": "switchhosts-data",
            "schemaVersion": 1,
            "root": manifest.root,
        },
        "entries": Value::Object(entries_map),
        "trashcan": {
            "format": "switchhosts-trashcan",
            "schemaVersion": 1,
            "items": trashcan.items,
        },
    });

    let bytes = serde_json::to_vec_pretty(&backup)
        .map_err(|e| StorageError::serialize(dest.display().to_string(), e))?;
    atomic_write(dest, &bytes)
}

/// Import backup bytes. Soft failures return `Ok(Value::String(error_code))`.
pub fn import_backup_bytes(bytes: &[u8], paths: &AppPaths) -> Result<Value, StorageError> {
    let data: Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(_) => return Ok(json!(ERR_PARSE)),
    };

    if !data.is_object() {
        return Ok(json!(ERR_INVALID_DATA));
    }

    if data.get("format").and_then(Value::as_str) != Some(BACKUP_FORMAT) {
        return Ok(json!(ERR_INVALID_DATA));
    }

    if data.get("manifest").is_some() {
        import_switchhosts_v5(&data, paths)
    } else if data.get("list").is_some() {
        import_legacy_v5(&data, paths)
    } else {
        Ok(json!(ERR_INVALID_DATA))
    }
}

/// CLI helper: writes entries and returns manifest (caller saves manifest).
pub fn import_v5_backup(paths: &AppPaths, backup: &Value) -> Result<Manifest, StorageError> {
    paths.ensure_layout()?;

    if backup.get("manifest").is_some() {
        let manifest_obj = backup
            .get("manifest")
            .and_then(Value::as_object)
            .ok_or_else(|| StorageError::UnsupportedBackup {
                reason: "missing manifest object".into(),
            })?;
        let root = normalize_imported_root(
            manifest_obj
                .get("root")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        );
        if let Some(entries_obj) = backup.get("entries").and_then(Value::as_object) {
            replace_entries_from_map(paths, entries_obj)?;
        }
        Ok(Manifest {
            root,
            ..Default::default()
        })
    } else {
        let list = normalize_imported_root(
            backup
                .get("list")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
        );
        if let Some(content) = backup.get("content").and_then(Value::as_object) {
            replace_entries_from_map(paths, content)?;
        }
        Ok(Manifest {
            root: list,
            ..Default::default()
        })
    }
}

fn import_switchhosts_v5(data: &Value, paths: &AppPaths) -> Result<Value, StorageError> {
    let Some(manifest_obj) = data.get("manifest").filter(|v| v.is_object()) else {
        return Ok(json!(ERR_INVALID_DATA));
    };

    let root = normalize_imported_root(
        manifest_obj
            .get("root")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
    );

    paths.ensure_layout()?;

    if let Some(entries_obj) = data.get("entries").and_then(Value::as_object) {
        replace_entries_from_map(paths, entries_obj)?;
    } else {
        clear_entry_files(paths)?;
    }

    let trashcan_items = data
        .get("trashcan")
        .and_then(|t| t.get("items"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Trashcan {
        items: parse_trashcan_items(trashcan_items),
        ..Default::default()
    }
    .save(&paths.trashcan_file)?;

    Manifest {
        root,
        ..Default::default()
    }
    .save(paths)?;

    Ok(json!(true))
}

fn import_legacy_v5(data: &Value, paths: &AppPaths) -> Result<Value, StorageError> {
    paths.ensure_layout()?;
    if let Some(content) = data.get("content").and_then(Value::as_object) {
        replace_entries_from_map(paths, content)?;
    } else {
        clear_entry_files(paths)?;
    }
    Trashcan::default().save(&paths.trashcan_file)?;
    let manifest = import_v5_backup(paths, data)?;
    manifest.save(paths)?;
    Ok(json!(true))
}

fn normalize_imported_root(root: Vec<Value>) -> Vec<Value> {
    if root.is_empty() {
        return root;
    }
    let looks_v5 = root.iter().any(|node| {
        node.get("isSys").is_some()
            || node.get("contentFile").is_some()
            || node.get("source").is_some()
            || node.get("group").is_some()
            || node.get("folder").is_some()
    });
    if looks_v5 {
        v5_root_to_legacy(&root, &[])
    } else {
        root
    }
}

fn parse_trashcan_items(items: Vec<Value>) -> Vec<TrashItem> {
    items.into_iter().filter_map(parse_trash_item).collect()
}

fn parse_trash_item(value: Value) -> Option<TrashItem> {
    if let Ok(item) = serde_json::from_value::<TrashItem>(value.clone()) {
        if !item.id.is_empty() {
            return Some(item);
        }
    }

    let obj = value.as_object()?;
    let node = obj
        .get("node")
        .or_else(|| obj.get("data"))
        .cloned()?;
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .or_else(|| node.get("id").and_then(Value::as_str))?
        .to_string();
    let parent_id = obj
        .get("parentId")
        .or_else(|| obj.get("parent_id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Some(TrashItem {
        id,
        node,
        parent_id,
        deleted_at: obj
            .get("deletedAt")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn replace_entries_from_map(
    paths: &AppPaths,
    entries_obj: &Map<String, Value>,
) -> Result<(), StorageError> {
    clear_entry_files(paths)?;
    write_entries_map(paths, entries_obj)
}

fn write_entries_map(
    paths: &AppPaths,
    entries_obj: &Map<String, Value>,
) -> Result<(), StorageError> {
    for (id, body) in entries_obj {
        if id == "0" {
            continue;
        }
        let text = body.as_str().unwrap_or("");
        entries::write_entry(&paths.entries_dir, id, text)?;
    }
    Ok(())
}

fn clear_entry_files(paths: &AppPaths) -> Result<(), StorageError> {
    if !paths.entries_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&paths.entries_dir).map_err(|e| {
        StorageError::io(paths.entries_dir.display().to_string(), e)
    })? {
        let entry = entry.map_err(|e| StorageError::io(paths.entries_dir.display().to_string(), e))?;
        let path = entry.path();
        if path.is_file() {
            fs::remove_file(&path).map_err(|e| StorageError::io(path.display().to_string(), e))?;
        }
    }
    Ok(())
}

pub fn read_file_with_limit(path: &Path, max_bytes: usize) -> Result<Vec<u8>, StorageError> {
    let mut file =
        fs::File::open(path).map_err(|e| StorageError::io(path.display().to_string(), e))?;
    let len = file
        .metadata()
        .map_err(|e| StorageError::io(path.display().to_string(), e))?
        .len() as usize;
    if len > max_bytes {
        return Err(StorageError::UnsupportedBackup {
            reason: format!("file exceeds {max_bytes} byte limit"),
        });
    }
    let mut buf = Vec::with_capacity(len);
    file.read_to_end(&mut buf)
        .map_err(|e| StorageError::io(path.display().to_string(), e))?;
    Ok(buf)
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
    fn export_import_round_trip_legacy() {
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
        let result = import_backup_bytes(&serde_json::to_vec(&backup).unwrap(), &paths2).unwrap();
        assert_eq!(result, json!(true));
        let loaded = Manifest::load(&paths2).unwrap();
        assert_eq!(loaded.root[0]["id"], "1");
        assert!(entries::read_entry(&paths2.entries_dir, "1")
            .unwrap()
            .contains("127.0.0.1"));
    }

    #[test]
    fn export_to_file_round_trips_switchhosts_format() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        let manifest = Manifest {
            root: json!([{ "id": "abc", "type": "local", "on": true }])
                .as_array()
                .cloned()
                .unwrap(),
            ..Default::default()
        };
        manifest.save(&paths).unwrap();
        entries::write_entry(&paths.entries_dir, "abc", "127.0.0.1 example.test\n").unwrap();
        Trashcan {
            items: vec![TrashItem {
                id: "trashed".into(),
                node: json!({ "id": "trashed", "type": "local" }),
                parent_id: None,
                deleted_at: None,
            }],
            ..Default::default()
        }
        .save(&paths.trashcan_file)
        .unwrap();

        let backup_path = paths.root.join("backup.json");
        export_to_file(&backup_path, &paths).unwrap();
        let bytes = fs::read(&backup_path).unwrap();

        std::fs::remove_file(&paths.manifest_file).ok();
        std::fs::remove_file(&paths.trashcan_file).ok();
        let _ = fs::remove_file(paths.entries_dir.join("abc.hosts"));

        let result = import_backup_bytes(&bytes, &paths).unwrap();
        assert_eq!(result, json!(true));

        let loaded = Manifest::load(&paths).unwrap();
        assert_eq!(loaded.root[0]["id"], "abc");
        assert_eq!(
            entries::read_entry(&paths.entries_dir, "abc").unwrap(),
            "127.0.0.1 example.test\n"
        );
        let trashcan = Trashcan::load(&paths.trashcan_file);
        assert_eq!(trashcan.items.len(), 1);
        assert_eq!(trashcan.items[0].id, "trashed");
    }

    #[test]
    fn import_v5_skips_system_id_zero_in_entries() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        let backup = json!({
            "format": BACKUP_FORMAT,
            "schemaVersion": 1,
            "manifest": {
                "root": [{ "id": "abc", "type": "local" }]
            },
            "entries": {
                "0": "system-content-must-not-be-written",
                "abc": "abc-content",
            },
            "trashcan": { "items": [] },
        });
        let result = import_backup_bytes(&serde_json::to_vec(&backup).unwrap(), &paths).unwrap();
        assert_eq!(result, json!(true));
        assert_eq!(entries::read_entry(&paths.entries_dir, "abc").unwrap(), "abc-content");
    }

    #[test]
    fn import_clears_stale_entry_files() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        entries::write_entry(&paths.entries_dir, "stale", "stale content\n").unwrap();

        let backup = json!({
            "format": BACKUP_FORMAT,
            "schemaVersion": 1,
            "list": [{ "id": "1", "type": "local" }],
            "content": { "1": "127.0.0.1 fresh\n" },
        });
        import_backup_bytes(&serde_json::to_vec(&backup).unwrap(), &paths).unwrap();
        assert!(!paths.entries_dir.join("stale.hosts").exists());
        assert_eq!(
            entries::read_entry(&paths.entries_dir, "1").unwrap(),
            "127.0.0.1 fresh\n"
        );
    }
}
