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
pub const ERR_NEW_VERSION: &str = "new_version";
pub const ERR_INVALID_DATA_KEY: &str = "invalid_data_key";
pub const ERR_INVALID_V3_DATA: &str = "invalid_v3_data";
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
///
/// Accepts SwitchHosts v3 (`version[0] === 3`), v4 (`version[0] === 4`), and v5
/// (`format === "switchhosts-backup"`) backup JSON.
pub fn import_backup_bytes(bytes: &[u8], paths: &AppPaths) -> Result<Value, StorageError> {
    let data: Value = match serde_json::from_slice(bytes) {
        Ok(v) => v,
        Err(_) => return Ok(json!(ERR_PARSE)),
    };

    if !data.is_object() {
        return Ok(json!(ERR_INVALID_DATA));
    }

    if data.get("format").and_then(Value::as_str) == Some(BACKUP_FORMAT) {
        if data.get("manifest").is_some() {
            return import_switchhosts_v5(&data, paths);
        }
        if data.get("list").is_some() {
            return import_legacy_v5(&data, paths);
        }
        return Ok(json!(ERR_INVALID_DATA));
    }

    let Some(version) = data.get("version").and_then(Value::as_array) else {
        return Ok(json!(ERR_INVALID_DATA));
    };
    let major = version.first().and_then(Value::as_u64).unwrap_or(0);

    match major {
        3 => import_v3(&data, paths),
        4 => import_v4(&data, paths),
        n if n > 4 => Ok(json!(ERR_NEW_VERSION)),
        _ => Ok(json!(ERR_INVALID_DATA)),
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

fn import_v3(data: &Value, paths: &AppPaths) -> Result<Value, StorageError> {
    let Some(list) = data.get("list").and_then(Value::as_array) else {
        return Ok(json!(ERR_INVALID_V3_DATA));
    };

    paths.ensure_layout()?;

    let mut converted = Vec::with_capacity(list.len());
    for node in list {
        converted.push(convert_v3_node(node, paths)?);
    }

    Trashcan::default().save(&paths.trashcan_file)?;
    Manifest {
        root: converted,
        ..Default::default()
    }
    .save(paths)?;

    Ok(json!(true))
}

fn convert_v3_node(node: &Value, paths: &AppPaths) -> Result<Value, StorageError> {
    let Some(obj) = node.as_object() else {
        return Ok(node.clone());
    };
    let mut out = Map::with_capacity(obj.len());

    for (key, value) in obj {
        match key.as_str() {
            "where" | "content" => continue,
            "refresh_interval" => {
                let hours = value.as_u64().unwrap_or(0);
                out.insert("refresh_interval".into(), json!(hours * 3600));
            }
            "children" => {
                if let Some(children) = value.as_array() {
                    let mut new_children = Vec::with_capacity(children.len());
                    for child in children {
                        new_children.push(convert_v3_node(child, paths)?);
                    }
                    out.insert("children".into(), Value::Array(new_children));
                } else {
                    out.insert("children".into(), value.clone());
                }
            }
            _ => {
                out.insert(key.clone(), value.clone());
            }
        }
    }

    if let Some(where_val) = obj.get("where") {
        out.insert("type".into(), where_val.clone());
    }

    if let Some(id) = obj.get("id").and_then(Value::as_str) {
        if id != "0" {
            if let Some(content) = obj.get("content").and_then(Value::as_str) {
                entries::write_entry(&paths.entries_dir, id, content)?;
            }
        }
    }

    Ok(Value::Object(out))
}

fn import_v4(data: &Value, paths: &AppPaths) -> Result<Value, StorageError> {
    let Some(inner) = data.get("data").filter(|v| v.is_object()) else {
        return Ok(json!(ERR_INVALID_DATA_KEY));
    };

    paths.ensure_layout()?;

    let tree = inner
        .get("list")
        .and_then(|l| l.get("tree"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let trashcan_items = inner
        .get("list")
        .and_then(|l| l.get("trashcan"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let hosts_data = inner
        .get("collection")
        .and_then(|c| c.get("hosts"))
        .and_then(|h| h.get("data"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let history_data = inner
        .get("collection")
        .and_then(|c| c.get("history"))
        .and_then(|h| h.get("data"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    for entry in &hosts_data {
        let id = entry.get("id").and_then(Value::as_str);
        let content = entry.get("content").and_then(Value::as_str);
        if let (Some(id), Some(content)) = (id, content) {
            if id == "0" {
                continue;
            }
            entries::write_entry(&paths.entries_dir, id, content)?;
        }
    }

    Trashcan {
        items: parse_trashcan_items(trashcan_items),
        ..Default::default()
    }
    .save(&paths.trashcan_file)?;

    Manifest {
        root: tree,
        ..Default::default()
    }
    .save(paths)?;

    if !history_data.is_empty() {
        write_history(
            &paths.histories_dir.join("system-hosts.json"),
            &history_data,
        )?;
    }

    Ok(json!(true))
}

fn write_history(path: &Path, items: &[Value]) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| StorageError::io(parent.display().to_string(), e))?;
    }
    let payload = serde_json::to_vec_pretty(items)
        .map_err(|e| StorageError::serialize(path.display().to_string(), e))?;
    atomic_write(path, &payload)
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

    #[test]
    fn dispatcher_returns_new_version_for_future_major() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        let bytes = serde_json::to_vec(&json!({ "version": [99, 0, 0] })).unwrap();
        let result = import_backup_bytes(&bytes, &paths).unwrap();
        assert_eq!(result, json!(ERR_NEW_VERSION));
    }

    #[test]
    fn import_v3_promotes_where_to_type_and_converts_interval() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        let backup = json!({
            "version": [3, 0],
            "list": [
                { "id": "0", "where": "system", "title": "System Hosts" },
                {
                    "id": "abc",
                    "where": "local",
                    "title": "Local",
                    "content": "127.0.0.1 example.test\n",
                    "on": true,
                },
                {
                    "id": "rem",
                    "where": "remote",
                    "title": "Remote",
                    "url": "https://example.com/hosts",
                    "refresh_interval": 2,
                },
            ],
        });

        let result = import_backup_bytes(&serde_json::to_vec(&backup).unwrap(), &paths).unwrap();
        assert_eq!(result, json!(true));

        let loaded = Manifest::load(&paths).unwrap();
        let local = loaded.root.iter().find(|n| n["id"] == "abc").unwrap();
        assert_eq!(local.get("type").and_then(Value::as_str), Some("local"));
        assert!(local.get("where").is_none());
        assert!(local.get("content").is_none());
        assert_eq!(
            entries::read_entry(&paths.entries_dir, "abc").unwrap(),
            "127.0.0.1 example.test\n"
        );

        let remote = loaded.root.iter().find(|n| n["id"] == "rem").unwrap();
        assert_eq!(
            remote.get("refresh_interval").and_then(Value::as_u64),
            Some(7200)
        );
        assert!(!paths.entry_file("0").exists());
    }

    #[test]
    fn import_v3_recurses_into_folder_children() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        let backup = json!({
            "version": [3, 0],
            "list": [{
                "id": "f",
                "where": "folder",
                "title": "Folder",
                "children": [{
                    "id": "child",
                    "where": "local",
                    "content": "child-content",
                }]
            }]
        });

        import_backup_bytes(&serde_json::to_vec(&backup).unwrap(), &paths).unwrap();

        let loaded = Manifest::load(&paths).unwrap();
        let folder = &loaded.root[0];
        assert_eq!(folder.get("type").and_then(Value::as_str), Some("folder"));
        let child = &folder["children"][0];
        assert_eq!(child.get("type").and_then(Value::as_str), Some("local"));
        assert!(child.get("content").is_none());
        assert_eq!(
            entries::read_entry(&paths.entries_dir, "child").unwrap(),
            "child-content"
        );
    }

    #[test]
    fn import_v3_returns_invalid_v3_data_when_list_missing() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        let bytes = serde_json::to_vec(&json!({ "version": [3, 0] })).unwrap();
        let result = import_backup_bytes(&bytes, &paths).unwrap();
        assert_eq!(result, json!(ERR_INVALID_V3_DATA));
    }

    #[test]
    fn import_v4_extracts_content_tree_and_trashcan() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        let backup = json!({
            "version": [4, 0, 0],
            "data": {
                "list": {
                    "tree": [{ "id": "abc", "type": "local", "title": "Local", "on": true }],
                    "trashcan": [{
                        "id": "trashed",
                        "data": { "id": "trashed", "type": "local" },
                        "add_time_ms": 0
                    }],
                },
                "collection": {
                    "hosts": {
                        "data": [
                            { "id": "0", "content": "system-content-skipped" },
                            { "id": "abc", "content": "abc-content" },
                        ]
                    },
                    "history": {
                        "data": [{ "id": "h1", "content": "old", "add_time_ms": 0 }]
                    }
                }
            }
        });

        let result = import_backup_bytes(&serde_json::to_vec(&backup).unwrap(), &paths).unwrap();
        assert_eq!(result, json!(true));

        let loaded = Manifest::load(&paths).unwrap();
        assert_eq!(loaded.root[0]["id"], "abc");
        assert_eq!(
            entries::read_entry(&paths.entries_dir, "abc").unwrap(),
            "abc-content"
        );
        assert!(!paths.entry_file("0").exists());

        let trashcan = Trashcan::load(&paths.trashcan_file);
        assert_eq!(trashcan.items.len(), 1);
        assert_eq!(trashcan.items[0].id, "trashed");

        assert!(paths.histories_dir.join("system-hosts.json").exists());
    }

    #[test]
    fn import_v4_returns_invalid_data_key_when_data_missing() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        let bytes = serde_json::to_vec(&json!({ "version": [4, 0] })).unwrap();
        let result = import_backup_bytes(&bytes, &paths).unwrap();
        assert_eq!(result, json!(ERR_INVALID_DATA_KEY));
    }
}
