use std::fs;
use std::path::Path;

use crate::storage::error::StorageError;

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| StorageError::io(parent.display().to_string(), e))?;
    }
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes).map_err(|e| StorageError::io(tmp.display().to_string(), e))?;
    fs::rename(&tmp, path).map_err(|e| StorageError::io(path.display().to_string(), e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn atomic_write_replaces_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("data.json");
        atomic_write(&path, b"{}").unwrap();
        atomic_write(&path, b"{\"a\":1}").unwrap();
        assert_eq!(fs::read_to_string(path).unwrap(), "{\"a\":1}");
    }
}
