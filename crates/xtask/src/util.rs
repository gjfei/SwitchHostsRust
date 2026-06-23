use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use anyhow::{Context, Result, bail};

pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("xtask crate lives in crates/xtask")
        .to_path_buf()
}

pub fn workspace_apps_package_names(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .context("failed to run cargo metadata")?;
    if !output.status.success() {
        bail!("cargo metadata failed");
    }
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let app_dir = root.join("app").canonicalize().ok();
    let packages = meta["packages"]
        .as_array()
        .context("missing packages in cargo metadata")?;

    let mut names = Vec::new();
    for pkg in packages {
        let Some(manifest) = pkg["manifest_path"].as_str() else {
            continue;
        };
        let manifest_path = PathBuf::from(manifest);
        let in_app = app_dir.as_ref().is_some_and(|app| {
            manifest_path
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .is_some_and(|crate_dir| crate_dir.parent().is_some_and(|parent| parent == *app))
        });
        if in_app {
            if let Some(name) = pkg["name"].as_str() {
                names.push(name.to_string());
            }
        }
    }
    Ok(names)
}

pub fn cargo_target_dir(root: &Path) -> Result<PathBuf> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .context("failed to run cargo metadata")?;
    if !output.status.success() {
        bail!("cargo metadata failed");
    }
    let meta: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    meta["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .context("missing target_directory in cargo metadata")
}

pub fn workspace_version(root: &Path) -> Result<String> {
    let content = std::fs::read_to_string(root.join("Cargo.toml"))
        .context("read workspace Cargo.toml")?;
    let workspace: toml::Value = toml::from_str(&content)?;
    workspace
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .context("missing workspace.package.version")
}

pub fn resolve_path(root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

pub fn run_cargo(root: &Path, args: &[&str]) -> Result<ExitStatus> {
    let status = Command::new("cargo")
        .current_dir(root)
        .args(args)
        .status()
        .with_context(|| format!("failed to run cargo {}", args.join(" ")))?;
    Ok(status)
}

pub fn ensure_cargo_watch() -> Result<()> {
    let ok = Command::new("cargo")
        .args(["watch", "--help"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        return Ok(());
    }
    eprintln!("未检测到 cargo-watch，正在安装（仅需一次）…");
    let status = Command::new("cargo")
        .args(["install", "cargo-watch", "--locked"])
        .status()
        .context("failed to install cargo-watch")?;
    if !status.success() {
        bail!("cargo install cargo-watch failed");
    }
    Ok(())
}
