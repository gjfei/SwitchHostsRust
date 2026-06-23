use std::fs;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use regex::Regex;

use crate::packager_config;
use crate::util::workspace_root;

pub struct ReleaseInfo {
    pub version: String,
    /// 空表示 Packager.toml 中全部 app/ 下的 app
    pub apps: Vec<String>,
}

impl ReleaseInfo {
    pub fn release_name(&self) -> String {
        if self.apps.len() == 1 {
            format!("{} v{}", self.apps[0], self.version)
        } else {
            format!("SwitchHostsRust v{}", self.version)
        }
    }
}

/// 解析 git tag，写入版本号，并输出 GitHub Actions `GITHUB_OUTPUT` 变量。
pub fn release_prepare(tag: &str, dry_run: bool) -> Result<()> {
    let root = workspace_root();
    let info = parse_release_tag(tag, &root)?;

    if !dry_run {
        set_workspace_version(&root, &info.version)?;
        set_packager_version(&root, &info.version)?;
        eprintln!("==> 版本已设为 {}", info.version);
    }

    write_github_output(&info)?;
    Ok(())
}

pub fn parse_release_tag(tag: &str, root: &Path) -> Result<ReleaseInfo> {
    let tag = tag.trim();
    if tag.is_empty() {
        bail!("tag 不能为空");
    }

    let semver = r"(?P<ver>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)";
    let all_re = Regex::new(&format!(r"^v{semver}$"))?;
    let app_re = Regex::new(&format!(r"^(?P<app>[a-zA-Z0-9-]+)-v{semver}$"))?;

    let (version, apps) = if let Some(caps) = all_re.captures(tag) {
        (
            caps["ver"].to_string(),
            Vec::new(),
        )
    } else if let Some(caps) = app_re.captures(tag) {
        (
            caps["ver"].to_string(),
            vec![caps["app"].to_string()],
        )
    } else {
        bail!(
            "无法识别的 tag `{tag}`\n\
             支持格式:\n\
               v0.1.0          — 打包全部 app/\n\
               egui-app-v0.1.0 — 仅打包指定 app"
        );
    };

    if !apps.is_empty() {
        let manifest = packager_config::load_manifest(root)?;
        for app in &apps {
            if !manifest.apps.iter().any(|a| a.name == *app) {
                let available = manifest
                    .apps
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                bail!("tag 中的 app `{app}` 不在 Packager.toml（可用: {available}）");
            }
        }
    }

    Ok(ReleaseInfo { version, apps })
}

fn set_workspace_version(root: &Path, version: &str) -> Result<()> {
    let path = root.join("Cargo.toml");
    let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut table: toml::Value =
        toml::from_str(&content).with_context(|| format!("parse {}", path.display()))?;
    table
        .as_table_mut()
        .and_then(|t| t.get_mut("workspace"))
        .and_then(|w| w.as_table_mut())
        .and_then(|w| w.get_mut("package"))
        .and_then(|p| p.as_table_mut())
        .context("[workspace.package]")?
        .insert("version".into(), toml::Value::String(version.to_string()));
    fs::write(&path, toml::to_string_pretty(&table)?).context("write Cargo.toml")?;
    Ok(())
}

fn set_packager_version(root: &Path, version: &str) -> Result<()> {
    let path = root.join("Packager.toml");
    if !path.is_file() {
        return Ok(());
    }
    let content = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut table: toml::Value =
        toml::from_str(&content).with_context(|| format!("parse {}", path.display()))?;
    table
        .as_table_mut()
        .context("Packager.toml root")?
        .insert("version".into(), toml::Value::String(version.to_string()));
    fs::write(&path, toml::to_string_pretty(&table)?).context("write Packager.toml")?;
    Ok(())
}

fn write_github_output(info: &ReleaseInfo) -> Result<()> {
    let apps = info.apps.join(",");
    if let Ok(path) = std::env::var("GITHUB_OUTPUT") {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        writeln!(file, "version={}", info.version)?;
        writeln!(file, "apps={apps}")?;
        writeln!(file, "release_name={}", info.release_name())?;
    } else {
        eprintln!("version={}", info.version);
        eprintln!("apps={apps}");
        eprintln!("release_name={}", info.release_name());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_v_prefix_tag() {
        let root = workspace_root();
        let info = parse_release_tag("v0.2.0", &root).unwrap();
        assert_eq!(info.version, "0.2.0");
        assert!(info.apps.is_empty());
    }

    #[test]
    fn parse_app_tag() {
        let root = workspace_root();
        let info = parse_release_tag("egui-app-v1.0.0-beta.1", &root).unwrap();
        assert_eq!(info.version, "1.0.0-beta.1");
        assert_eq!(info.apps, vec!["egui-app"]);
    }
}
