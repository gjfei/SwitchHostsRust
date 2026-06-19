use std::path::Path;

use super::error::ApplyError;

pub trait ElevationBackend: Send + Sync {
    fn write_file(&self, path: &Path, content: &str) -> Result<(), ApplyError>;
}

pub struct MockElevation;

impl ElevationBackend for MockElevation {
    fn write_file(&self, path: &Path, content: &str) -> Result<(), ApplyError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub struct SystemElevation;

impl ElevationBackend for SystemElevation {
    fn write_file(&self, path: &Path, content: &str) -> Result<(), ApplyError> {
        // 回退：权限允许时直接写入（测试/开发）；生产环境可接入平台提权。
        MockElevation.write_file(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn mock_elevation_writes() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("h");
        MockElevation.write_file(&path, "data").unwrap();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "data");
    }
}
