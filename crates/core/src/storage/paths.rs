use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::storage::error::StorageError;

pub const DATA_DIR_NAME: &str = "SwitchHostsRust";

/// SwitchHostsRust 数据根下的路径解析。
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub root: PathBuf,
    pub manifest_file: PathBuf,
    pub entries_dir: PathBuf,
    pub trashcan_file: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
    pub histories_dir: PathBuf,
    pub dev_test_hosts: PathBuf,
}

impl AppPaths {
    /// 默认用户数据根：`~/.SwitchHostsRust`（Windows：`%USERPROFILE%\.SwitchHostsRust`）。
    pub fn default_user() -> Result<Self, StorageError> {
        let home = dirs::home_dir().ok_or_else(|| StorageError::InvalidPath {
            reason: "cannot resolve home directory".into(),
        })?;
        Ok(Self::new(home.join(format!(".{DATA_DIR_NAME}"))))
    }

    /// 可注入根路径，供测试与自定义安装使用。
    pub fn new(root: PathBuf) -> Self {
        let internal = root.join("internal");
        Self {
            manifest_file: root.join("manifest.json"),
            entries_dir: root.join("entries"),
            trashcan_file: root.join("trashcan.json"),
            config_file: internal.join("config.json"),
            state_file: internal.join("state.json"),
            histories_dir: internal.join("histories"),
            dev_test_hosts: internal.join("dev").join("test.hosts"),
            root,
        }
    }

    pub fn ensure_layout(&self) -> Result<(), StorageError> {
        for dir in [
            &self.root,
            &self.entries_dir,
            self.config_file.parent().unwrap(),
            self.state_file.parent().unwrap(),
            &self.histories_dir,
            self.dev_test_hosts.parent().unwrap(),
        ] {
            fs::create_dir_all(dir).map_err(|e| StorageError::io(dir.display().to_string(), e))?;
        }
        Ok(())
    }

    pub fn entry_file(&self, id: &str) -> PathBuf {
        self.entries_dir.join(format!("{id}.hosts"))
    }
}

pub fn resolve_hosts_file_from_env() -> Option<PathBuf> {
    std::env::var("SWITCH_HOSTS_RUST_HOSTS_FILE")
        .ok()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn new_paths_under_root() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        assert!(paths.manifest_file.ends_with("manifest.json"));
        assert!(paths.entries_dir.ends_with("entries"));
        assert!(paths.dev_test_hosts.ends_with("internal/dev/test.hosts"));
    }

    #[test]
    fn ensure_layout_creates_dirs() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        paths.ensure_layout().unwrap();
        assert!(paths.entries_dir.is_dir());
        assert!(paths.dev_test_hosts.parent().unwrap().is_dir());
    }
}
