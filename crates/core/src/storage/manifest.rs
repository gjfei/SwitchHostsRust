use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::atomic::atomic_write;
use super::error::StorageError;
use super::paths::AppPaths;
use super::state::StateFile;
use super::tree_format::{legacy_root_to_v5, v5_root_to_legacy};

pub const MANIFEST_FORMAT: &str = "switchhosts-data";
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_format")]
    #[allow(dead_code)]
    pub format: String,
    #[serde(default = "default_schema_version", rename = "schemaVersion")]
    #[allow(dead_code)]
    pub schema_version: u32,
    #[serde(default)]
    pub root: Vec<Value>,
}

fn default_format() -> String {
    MANIFEST_FORMAT.to_string()
}

fn default_schema_version() -> u32 {
    MANIFEST_SCHEMA_VERSION
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            format: default_format(),
            schema_version: default_schema_version(),
            root: Vec::new(),
        }
    }
}

impl Manifest {
    pub fn load(paths: &AppPaths) -> Result<Self, StorageError> {
        let path = &paths.manifest_file;
        if !path.exists() {
            return Ok(Self::default());
        }
        let bytes =
            std::fs::read(path).map_err(|e| StorageError::io(path.display().to_string(), e))?;
        let raw: Manifest = serde_json::from_slice(&bytes)
            .map_err(|e| StorageError::parse(path.display().to_string(), e))?;

        let state = StateFile::load(&paths.state_file);
        let root = v5_root_to_legacy(&raw.root, &state.tree.collapsed_node_ids);

        Ok(Self {
            format: raw.format,
            schema_version: raw.schema_version,
            root,
        })
    }

    pub fn save(&self, paths: &AppPaths) -> Result<(), StorageError> {
        let (v5_root, collapsed_ids) = legacy_root_to_v5(&self.root);

        let mut state = StateFile::load(&paths.state_file);
        state.tree.collapsed_node_ids = collapsed_ids;
        state.save(&paths.state_file)?;

        let envelope = json!({
            "format": MANIFEST_FORMAT,
            "schemaVersion": MANIFEST_SCHEMA_VERSION,
            "root": v5_root,
        });
        let bytes = serde_json::to_vec_pretty(&envelope).map_err(|e| {
            StorageError::serialize(paths.manifest_file.display().to_string(), e)
        })?;
        atomic_write(&paths.manifest_file, &bytes)
    }
}

pub fn find_node(nodes: &[Value], id: &str) -> Option<Value> {
    for node in nodes {
        if node_id(node) == Some(id) {
            return Some(node.clone());
        }
        if let Some(children) = node_children(node) {
            if let Some(found) = find_node(children, id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn set_node_on(nodes: &mut [Value], id: &str, on: bool) -> bool {
    for node in nodes {
        if node_id(node) == Some(id) {
            if let Some(obj) = node.as_object_mut() {
                obj.insert("on".into(), json!(on));
                return true;
            }
        }
        if let Some(children) = node.as_object_mut().and_then(|o| o.get_mut("children")) {
            if let Some(arr) = children.as_array_mut() {
                if set_node_on(arr, id, on) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn collect_content_ids(nodes: &[Value], out: &mut Vec<String>) {
    for node in nodes {
        let kind = node.get("type").and_then(Value::as_str);
        if matches!(kind, Some("local") | Some("remote")) {
            if let Some(id) = node_id(node) {
                if node.get("is_sys").and_then(Value::as_bool) != Some(true) {
                    out.push(id.to_string());
                }
            }
        }
        if let Some(children) = node_children(node) {
            collect_content_ids(children, out);
        }
    }
}

pub fn flatten_nodes(nodes: &[Value], out: &mut Vec<Value>) {
    for node in nodes {
        out.push(node.clone());
        if let Some(children) = node_children(node) {
            flatten_nodes(children, out);
        }
    }
}

fn node_id(node: &Value) -> Option<&str> {
    node.get("id").and_then(Value::as_str)
}

fn node_children(node: &Value) -> Option<&Vec<Value>> {
    node.get("children").and_then(Value::as_array)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::paths::AppPaths;
    use tempfile::TempDir;

    #[test]
    fn save_and_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();

        let mut m = Manifest::default();
        m.root = json!([
            { "id": "1", "type": "local", "title": "test", "on": true }
        ])
        .as_array()
        .cloned()
        .unwrap();
        m.save(&paths).unwrap();

        let loaded = Manifest::load(&paths).unwrap();
        assert_eq!(loaded.root[0]["title"], "test");
    }
}
