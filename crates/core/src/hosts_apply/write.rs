use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};

use super::elevation::ElevationBackend;
use super::error::ApplyError;
use super::target::HostsTarget;

pub const SWITCHHOSTS_MARKER: &str = "# --- SWITCHHOSTS_CONTENT_START ---";

pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn read_existing(path: &Path) -> Result<String, ApplyError> {
    if !path.exists() {
        return Ok(String::new());
    }
    Ok(fs::read_to_string(path)?)
}

pub fn build_final_content(
    existing: &str,
    new_content: &str,
    write_mode: &str,
) -> String {
    if write_mode == "overwrite" {
        return new_content.to_string();
    }
    // append 模式
    if existing.is_empty() {
        return format!("{SWITCHHOSTS_MARKER}\n{new_content}");
    }
    if let Some(idx) = existing.find(SWITCHHOSTS_MARKER) {
        let prefix = &existing[..idx];
        format!("{prefix}{SWITCHHOSTS_MARKER}\n{new_content}")
    } else {
        format!("{existing}\n{SWITCHHOSTS_MARKER}\n{new_content}")
    }
}

pub fn write_hosts(
    target: &HostsTarget,
    new_content: &str,
    write_mode: &str,
    elevation: &dyn ElevationBackend,
) -> Result<bool, ApplyError> {
    let path = target.path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = read_existing(&path)?;
    let final_content = build_final_content(&existing, new_content, write_mode);

    if content_hash(&existing) == content_hash(&final_content) {
        return Ok(false);
    }

    if target.needs_elevation() {
        elevation.write_file(&path, &final_content)?;
    } else {
        fs::write(&path, &final_content)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hosts_apply::elevation::MockElevation;
    use tempfile::TempDir;

    #[test]
    fn append_uses_marker() {
        let out = build_final_content("127.0.0.1 localhost\n", "127.0.0.1 dev.local", "append");
        assert!(out.contains(SWITCHHOSTS_MARKER));
        assert!(out.contains("dev.local"));
    }

    #[test]
    fn skip_write_when_hash_unchanged() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("hosts");
        let target = HostsTarget::File(path.clone());
        write_hosts(&target, "same\n", "overwrite", &MockElevation).unwrap();
        let written = write_hosts(&target, "same\n", "overwrite", &MockElevation).unwrap();
        assert!(!written);
    }

    #[test]
    fn writes_to_file_target() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("hosts");
        let target = HostsTarget::File(path.clone());
        let written = write_hosts(
            &target,
            "127.0.0.1 x.test\n",
            "overwrite",
            &MockElevation,
        )
        .unwrap();
        assert!(written);
        assert!(fs::read_to_string(path).unwrap().contains("x.test"));
    }
}
