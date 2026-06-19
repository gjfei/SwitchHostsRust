use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::error::ApplyError;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub content: String,
    pub written_at: String,
}

pub fn append_history(
    histories_dir: &Path,
    content: &str,
    limit: u32,
) -> Result<(), ApplyError> {
    fs::create_dir_all(histories_dir)?;
    let file = histories_dir.join(format!("{}.json", Utc::now().timestamp_millis()));
    let entry = serde_json::json!({
        "content": content,
        "writtenAt": Utc::now().to_rfc3339(),
    });
    fs::write(&file, serde_json::to_vec_pretty(&entry)?)?;

    let mut files: Vec<PathBuf> = fs::read_dir(histories_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    while files.len() > limit as usize {
        if let Some(old) = files.first() {
            let _ = fs::remove_file(old);
            files.remove(0);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn respects_history_limit() {
        let tmp = TempDir::new().unwrap();
        for _ in 0..5 {
            append_history(tmp.path(), "x", 2).unwrap();
        }
        let count = fs::read_dir(tmp.path()).unwrap().count();
        assert!(count <= 2);
    }
}
