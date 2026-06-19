use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::atomic::atomic_write;
use super::error::StorageError;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Trashcan {
    #[serde(default)]
    pub items: Vec<TrashItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrashItem {
    pub id: String,
    pub node: Value,
    #[serde(default, rename = "parentId")]
    pub parent_id: Option<String>,
    #[serde(default, rename = "deletedAt")]
    pub deleted_at: Option<String>,
}

impl Trashcan {
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

    pub fn push(&mut self, item: TrashItem) {
        self.items.push(item);
    }

    pub fn remove(&mut self, id: &str) -> Option<TrashItem> {
        if let Some(pos) = self.items.iter().position(|i| i.id == id) {
            Some(self.items.remove(pos))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn trash_round_trip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("trash.json");
        let mut t = Trashcan::default();
        t.push(TrashItem {
            id: "1".into(),
            node: json!({"id":"1","type":"local"}),
            parent_id: None,
            deleted_at: None,
        });
        t.save(&path).unwrap();
        assert_eq!(Trashcan::load(&path).items.len(), 1);
    }
}
