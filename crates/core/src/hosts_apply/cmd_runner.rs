use std::path::Path;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::storage::atomic::atomic_write;
use crate::storage::error::StorageError;

use super::error::ApplyError;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RECORDS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRunResult {
    #[serde(rename = "_id")]
    pub id: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub add_time_ms: i64,
}

pub fn cmd_history_path(histories_dir: &Path) -> std::path::PathBuf {
    histories_dir.join("cmd-after-apply.json")
}

pub fn run_after_apply(cmd: &str, histories_dir: &Path) -> Result<(), ApplyError> {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let result = run_and_capture(trimmed);
    let path = cmd_history_path(histories_dir);
    let _ = insert(&path, result.clone());
    if !result.success {
        let detail = if result.stderr.is_empty() {
            result.stdout
        } else {
            result.stderr
        };
        return Err(ApplyError::Elevation(format!(
            "post-apply command failed: {detail}"
        )));
    }
    Ok(())
}

fn run_and_capture(cmd: &str) -> CommandRunResult {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let id = make_id(now_ms);
    match run_with_timeout(cmd) {
        Ok(output) => CommandRunResult {
            id,
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            add_time_ms: now_ms,
        },
        Err(err) => CommandRunResult {
            id,
            success: false,
            stdout: String::new(),
            stderr: err,
            add_time_ms: now_ms,
        },
    }
}

fn run_with_timeout(cmd: &str) -> Result<Output, String> {
    let mut child = spawn_shell(cmd).map_err(|e| format!("failed to spawn shell: {e}"))?;
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("wait failed: {e}"))?
        {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            if let Some(mut out) = child.stdout.take() {
                let _ = std::io::Read::read_to_end(&mut out, &mut stdout);
            }
            if let Some(mut err) = child.stderr.take() {
                let _ = std::io::Read::read_to_end(&mut err, &mut stderr);
            }
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        if started.elapsed() >= TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("command timed out after {}s", TIMEOUT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(target_os = "windows"))]
fn spawn_shell(cmd: &str) -> std::io::Result<std::process::Child> {
    Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
}

#[cfg(target_os = "windows")]
fn spawn_shell(cmd: &str) -> std::io::Result<std::process::Child> {
    Command::new("cmd")
        .arg("/d")
        .arg("/s")
        .arg("/c")
        .arg(cmd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
}

fn make_id(now_ms: i64) -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("cmd_{now_ms}_{seq}")
}

pub fn load(path: &Path) -> Result<Vec<CommandRunResult>, StorageError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path).map_err(|e| StorageError::io(path.display().to_string(), e))?;
    match serde_json::from_slice::<Vec<CommandRunResult>>(&bytes) {
        Ok(v) => Ok(v),
        Err(_) => match serde_json::from_slice::<Value>(&bytes) {
            Ok(Value::Array(arr)) => Ok(arr
                .into_iter()
                .filter_map(|v| serde_json::from_value::<CommandRunResult>(v).ok())
                .collect()),
            _ => Ok(Vec::new()),
        },
    }
}

pub fn save(path: &Path, items: &[CommandRunResult]) -> Result<(), StorageError> {
    let bytes = serde_json::to_vec_pretty(items)
        .map_err(|e| StorageError::serialize(path.display().to_string(), e))?;
    atomic_write(path, &bytes)
}

pub fn insert(path: &Path, item: CommandRunResult) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| StorageError::io(parent.display().to_string(), e))?;
    }
    let mut items = load(path)?;
    items.push(item);
    if items.len() > MAX_RECORDS {
        let drop_count = items.len() - MAX_RECORDS;
        items.drain(0..drop_count);
    }
    save(path, &items)
}

pub fn delete_by_id(path: &Path, id: &str) -> Result<bool, StorageError> {
    let mut items = load(path)?;
    let before = items.len();
    items.retain(|i| i.id != id);
    if items.len() == before {
        return Ok(false);
    }
    save(path, &items)?;
    Ok(true)
}

pub fn clear(path: &Path) -> Result<(), StorageError> {
    if !path.exists() {
        return Ok(());
    }
    save(path, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_cmd_ok() {
        let tmp = TempDir::new().unwrap();
        run_after_apply("", &tmp.path().join("histories")).unwrap();
    }

    #[test]
    fn history_insert_and_delete() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("cmd-after-apply.json");
        let item = CommandRunResult {
            id: "cmd_1".into(),
            success: true,
            stdout: "ok".into(),
            stderr: String::new(),
            add_time_ms: 1,
        };
        insert(&path, item).unwrap();
        let items = load(&path).unwrap();
        assert_eq!(items.len(), 1);
        assert!(delete_by_id(&path, "cmd_1").unwrap());
        assert!(load(&path).unwrap().is_empty());
    }
}
