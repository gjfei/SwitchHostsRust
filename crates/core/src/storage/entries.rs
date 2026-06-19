use std::fs;
use std::path::Path;

use crate::storage::error::StorageError;
use crate::storage::tree_format::SYSTEM_NODE_ID;

pub fn read_entry(entries_dir: &Path, id: &str) -> Result<String, StorageError> {
    if id == SYSTEM_NODE_ID {
        return Ok(String::new());
    }
    let path = entries_dir.join(format!("{id}.hosts"));
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(&path).map_err(|e| StorageError::io(path.display().to_string(), e))
}

pub fn write_entry(entries_dir: &Path, id: &str, content: &str) -> Result<(), StorageError> {
    if id == SYSTEM_NODE_ID {
        return Ok(());
    }
    fs::create_dir_all(entries_dir)
        .map_err(|e| StorageError::io(entries_dir.display().to_string(), e))?;
    let path = entries_dir.join(format!("{id}.hosts"));
    fs::write(&path, content).map_err(|e| StorageError::io(path.display().to_string(), e))
}

pub fn delete_entry(entries_dir: &Path, id: &str) -> Result<(), StorageError> {
    if id == SYSTEM_NODE_ID {
        return Ok(());
    }
    let path = entries_dir.join(format!("{id}.hosts"));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| StorageError::io(path.display().to_string(), e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn read_missing_returns_empty() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(read_entry(tmp.path(), "x").unwrap(), "");
    }

    #[test]
    fn write_and_read() {
        let tmp = TempDir::new().unwrap();
        write_entry(tmp.path(), "1", "127.0.0.1 localhost\n").unwrap();
        assert!(read_entry(tmp.path(), "1").unwrap().contains("localhost"));
    }

    #[test]
    fn system_node_has_no_file() {
        let tmp = TempDir::new().unwrap();
        write_entry(tmp.path(), SYSTEM_NODE_ID, "ignored").unwrap();
        assert_eq!(read_entry(tmp.path(), SYSTEM_NODE_ID).unwrap(), "");
        assert!(!tmp.path().join("0.hosts").exists());
    }
}
