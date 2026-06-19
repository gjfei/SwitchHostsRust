use std::path::PathBuf;

use crate::storage::paths::AppPaths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostsTarget {
    System,
    File(PathBuf),
}

impl HostsTarget {
    pub fn system_default() -> Self {
        Self::System
    }

    pub fn dev_default(paths: &AppPaths) -> Self {
        Self::File(paths.dev_test_hosts.clone())
    }

    pub fn resolve(paths: &AppPaths, system_flag: bool, hosts_file: Option<PathBuf>) -> Self {
        if system_flag {
            return Self::System;
        }
        if let Some(p) = hosts_file.or_else(|| crate::storage::paths::resolve_hosts_file_from_env())
        {
            return Self::File(p);
        }
        Self::dev_default(paths)
    }

    pub fn path(&self) -> PathBuf {
        match self {
            Self::System => system_hosts_path(),
            Self::File(p) => p.clone(),
        }
    }

    pub fn needs_elevation(&self) -> bool {
        matches!(self, Self::System)
    }
}

#[cfg(target_os = "windows")]
pub fn system_hosts_path() -> PathBuf {
    PathBuf::from(r"C:\Windows\System32\drivers\etc\hosts")
}

#[cfg(not(target_os = "windows"))]
pub fn system_hosts_path() -> PathBuf {
    PathBuf::from("/etc/hosts")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::paths::AppPaths;
    use tempfile::TempDir;

    #[test]
    fn dev_default_uses_internal_dev() {
        let tmp = TempDir::new().unwrap();
        let paths = AppPaths::new(tmp.path().to_path_buf());
        match HostsTarget::dev_default(&paths) {
            HostsTarget::File(p) => assert!(p.ends_with("internal/dev/test.hosts")),
            _ => panic!("expected file target"),
        }
    }
}
