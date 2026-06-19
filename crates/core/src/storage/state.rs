use std::path::Path;

use serde::{Deserialize, Serialize};

use super::atomic::atomic_write;
use super::error::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateFile {
    #[serde(default)]
    pub tree: TreeState,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TreeState {
    #[serde(default, rename = "collapsedNodeIds")]
    pub collapsed_node_ids: Vec<String>,
}

impl StateFile {
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| StorageError::serialize(path.display().to_string(), e))?;
        atomic_write(path, &bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn collapsed_ids_persist() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("state.json");
        let mut s = StateFile::default();
        s.tree.collapsed_node_ids = vec!["f1".into()];
        s.save(&path).unwrap();
        assert_eq!(StateFile::load(&path).tree.collapsed_node_ids, vec!["f1"]);
    }
}
