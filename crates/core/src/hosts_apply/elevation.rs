use std::io::ErrorKind;
use std::path::Path;

use super::error::ApplyError;
use super::platform_write;

fn is_permission_denied(e: &std::io::Error) -> bool {
    e.kind() == ErrorKind::PermissionDenied || matches!(e.raw_os_error(), Some(1) | Some(13))
}

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
        match MockElevation.write_file(path, content) {
            Ok(()) => Ok(()),
            Err(ApplyError::Io(e)) if is_permission_denied(&e) => {
                platform_write::elevated_write(path, content)
            }
            Err(e) => Err(e),
        }
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
