//! 扫描 `app/<name>/` 目录自动发现 GUI crate，无需注册表。

use std::path::Path;

use anyhow::{Context, Result};

use crate::util::workspace_root;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
    /// `app/` 下的目录名
    pub id: String,
    /// `[package].name`
    pub package: String,
}

pub fn discover_apps() -> Result<Vec<AppEntry>> {
    let app_root = workspace_root().join("app");
    if !app_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut apps = Vec::new();
    for entry in std::fs::read_dir(&app_root).with_context(|| format!("读取 {}", app_root.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let manifest = entry.path().join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let package = read_package_name(&manifest)?;
        apps.push(AppEntry { id, package });
    }
    apps.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(apps)
}

pub fn resolve(name: &str) -> Result<AppEntry> {
    let apps = discover_apps()?;
    apps.into_iter()
        .find(|a| a.id == name || a.package == name)
        .ok_or_else(|| {
            let known = format_known_apps(&discover_apps().unwrap_or_default());
            anyhow::anyhow!("未知 app `{name}`，可用: {known}")
        })
}

pub fn format_known_apps(apps: &[AppEntry]) -> String {
    if apps.is_empty() {
        return "(app/ 下暂无 crate)".into();
    }
    apps.iter()
        .map(|a| format!("{} ({})", a.id, a.package))
        .collect::<Vec<_>>()
        .join(", ")
}

fn read_package_name(manifest: &Path) -> Result<String> {
    let content = std::fs::read_to_string(manifest)
        .with_context(|| format!("读取 {}", manifest.display()))?;
    let value: toml::Value = toml::from_str(&content)
        .with_context(|| format!("解析 {}", manifest.display()))?;
    value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_owned)
        .with_context(|| format!("{} 缺少 [package].name", manifest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_app_crates_by_directory() {
        let apps = discover_apps().expect("discover");
        assert!(apps.iter().any(|a| a.id == "egui" && a.package == "egui-app"));
        assert!(apps.iter().any(|a| a.id == "gpui" && a.package == "gpui-app"));
    }

    #[test]
    fn resolves_by_directory_or_package_name() {
        let egui = resolve("egui").expect("egui");
        assert_eq!(egui.package, "egui-app");
        let gpui = resolve("gpui-app").expect("gpui-app");
        assert_eq!(gpui.id, "gpui");
    }
}
