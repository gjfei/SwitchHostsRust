use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use cargo_packager::PackageFormat;
use cargo_packager::config::{Binary, Config, MacOsConfig};

use crate::util::{resolve_path, workspace_apps_package_names, workspace_version};

#[derive(Debug, serde::Deserialize, Clone, Default)]
pub struct DmgSection {
    #[serde(default = "default_dmg_background")]
    pub background: PathBuf,
}

#[derive(Debug, serde::Deserialize)]
pub struct PlatformFormats {
    #[serde(default = "default_windows_formats")]
    pub formats: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MacOsPackager {
    #[serde(default = "default_mac_formats")]
    pub formats: Vec<String>,
    #[serde(flatten)]
    pub config: MacOsConfig,
}

#[derive(Debug, serde::Deserialize)]
pub struct PackagerManifest {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default = "default_out_dir", alias = "out-dir", alias = "out_dir")]
    pub out_dir: PathBuf,
    pub category: Option<String>,
    pub copyright: Option<String>,
    #[serde(default)]
    pub macos: Option<MacOsPackager>,
    #[serde(default)]
    pub windows: Option<PlatformFormats>,
    #[serde(default)]
    pub dmg: Option<DmgSection>,
    pub apps: Vec<AppSpec>,
}

#[derive(Debug, serde::Deserialize)]
pub struct AppSpec {
    pub name: String,
    #[serde(alias = "product-name", alias = "product_name")]
    pub product_name: String,
    #[serde(alias = "cargo-package", alias = "cargo_package")]
    pub cargo_package: String,
    pub bin: String,
    pub identifier: String,
    pub description: Option<String>,
    #[serde(alias = "long-description", alias = "long_description")]
    pub long_description: Option<String>,
    #[serde(default)]
    pub icons: Vec<String>,
    #[serde(default)]
    pub formats: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub macos: Option<MacOsConfig>,
}

pub fn load_manifest(root: &Path) -> Result<PackagerManifest> {
    let path = root.join("Packager.toml");
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut manifest: PackagerManifest =
        toml::from_str(&content).with_context(|| format!("parse {}", path.display()))?;
    if manifest.version.is_none() {
        manifest.version = Some(workspace_version(root)?);
    }
    if manifest.apps.is_empty() {
        bail!("Packager.toml 未定义 [[apps]]");
    }
    filter_apps_dir_only(root, &mut manifest)?;
    Ok(manifest)
}

fn filter_apps_dir_only(root: &Path, manifest: &mut PackagerManifest) -> Result<()> {
    let allowed = workspace_apps_package_names(root)?;
    if allowed.is_empty() {
        bail!("workspace `app/` 下没有可打包的 crate");
    }

    let mut skipped = Vec::new();
    manifest.apps.retain(|app| {
        if allowed.iter().any(|p| p == &app.cargo_package) {
            true
        } else {
            skipped.push(app.cargo_package.clone());
            false
        }
    });

    for pkg in &skipped {
        eprintln!(
            "warning: 跳过 `{pkg}`（仅打包 app/ 下的 crate；可用: {}）",
            allowed.join(", ")
        );
    }

    if manifest.apps.is_empty() {
        bail!(
            "Packager.toml [[apps]] 中没有 app/ 下的 crate（可用: {}）",
            allowed.join(", ")
        );
    }
    Ok(())
}

pub fn select_apps<'a>(
    manifest: &'a PackagerManifest,
    filter: &[String],
) -> Result<Vec<&'a AppSpec>> {
    let selected: Vec<_> = manifest
        .apps
        .iter()
        .filter(|app| app.enabled)
        .filter(|app| filter.is_empty() || filter.iter().any(|f| f == &app.name))
        .collect();
    if selected.is_empty() {
        if filter.is_empty() {
            bail!("没有启用的 app 可打包（检查 Packager.toml [[apps]] enabled）");
        }
        bail!(
            "未找到匹配的 app: {}（可用: {}）",
            filter.join(", "),
            manifest
                .apps
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    Ok(selected)
}

impl AppSpec {
    pub fn to_packager_config(
        &self,
        manifest: &PackagerManifest,
        root: &Path,
        target_dir: &Path,
        profile: &str,
        out_dir: &Path,
        formats: &[PackageFormat],
    ) -> Result<Config> {
        let icons = if self.icons.is_empty() {
            None
        } else {
            Some(
                self.icons
                    .iter()
                    .map(|icon| resolve_path(root, PathBuf::from(icon)).display().to_string())
                    .collect(),
            )
        };

        let mut macos = manifest
            .macos
            .as_ref()
            .map(|section| section.config.clone());
        if let Some(app_macos) = &self.macos {
            if macos.is_none() {
                macos = Some(app_macos.clone());
            } else if let Some(base) = macos.as_mut() {
                merge_macos(base, app_macos);
            }
        }

        let mut config = Config::default();
        config.product_name = self.product_name.clone();
        config.version = manifest.version.clone().unwrap_or_default();
        config.binaries = vec![Binary::new(&self.bin).main(true)];
        config.identifier = Some(self.identifier.clone());
        config.before_packaging_command = None;
        config.formats = Some(formats.to_vec());
        config.out_dir = out_dir.to_path_buf();
        config.binaries_dir = Some(target_dir.join(profile));
        config.description = self.description.clone();
        config.long_description = self.long_description.clone();
        config.copyright = manifest.copyright.clone();
        config.category = manifest.category.as_deref().and_then(parse_category);
        config.icons = icons;
        config.macos = macos;
        Ok(config)
    }
}

impl DmgSection {
    pub fn resolved_background(&self, root: &Path) -> PathBuf {
        resolve_path(root, self.background.clone())
    }
}

pub fn resolve_mac_packager_formats(app_only: bool) -> Result<Vec<PackageFormat>> {
    if app_only {
        return Ok(vec![PackageFormat::App]);
    }
    Ok(vec![PackageFormat::App])
}

pub fn resolve_windows_packager_formats(manifest: &PackagerManifest) -> Result<Vec<PackageFormat>> {
    let default = default_windows_formats();
    let names = manifest
        .windows
        .as_ref()
        .map(|w| &w.formats)
        .unwrap_or(&default);
    parse_format_names(names)
}

/// 配置中是否包含 dmg（用于决定是否调用 hdiutil）
pub fn wants_dmg(manifest: &PackagerManifest, app: &AppSpec, app_only: bool) -> bool {
    if app_only {
        return false;
    }
    let default = default_mac_formats();
    let names = app
        .formats
        .as_ref()
        .or_else(|| manifest.macos.as_ref().map(|m| &m.formats))
        .unwrap_or(&default);
    names.iter().any(|f| f.eq_ignore_ascii_case("dmg"))
}

fn parse_format_names(names: &[String]) -> Result<Vec<PackageFormat>> {
    let mut formats = Vec::new();
    for name in names {
        formats.push(parse_format_name(name)?);
    }
    if formats.is_empty() {
        bail!("formats 不能为空");
    }
    Ok(formats)
}

fn parse_format_name(name: &str) -> Result<PackageFormat> {
    match name.to_ascii_lowercase().as_str() {
        "app" => Ok(PackageFormat::App),
        "dmg" => Ok(PackageFormat::Dmg),
        "nsis" => Ok(PackageFormat::Nsis),
        other => bail!("未知 format `{other}`（支持 app、dmg、nsis）"),
    }
}

fn parse_category(name: &str) -> Option<cargo_packager::config::AppCategory> {
    use cargo_packager::config::AppCategory;
    Some(match name {
        "DeveloperTool" | "Developer Tool" => AppCategory::DeveloperTool,
        "Utility" => AppCategory::Utility,
        other => {
            let _ = other;
            return None;
        }
    })
}

fn merge_macos(base: &mut MacOsConfig, overlay: &MacOsConfig) {
    if overlay.minimum_system_version.is_some() {
        base.minimum_system_version = overlay.minimum_system_version.clone();
    }
    if overlay.entitlements.is_some() {
        base.entitlements = overlay.entitlements.clone();
    }
    if overlay.info_plist_path.is_some() {
        base.info_plist_path = overlay.info_plist_path.clone();
    }
}

fn default_dmg_background() -> PathBuf {
    PathBuf::from("crates/xtask/assets/dmg/background.png")
}

fn default_out_dir() -> PathBuf {
    PathBuf::from("dist")
}

fn default_mac_formats() -> Vec<String> {
    vec!["dmg".into()]
}

fn default_windows_formats() -> Vec<String> {
    vec!["nsis".into()]
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::workspace_root;

    #[test]
    fn load_workspace_manifest() {
        let root = workspace_root();
        let manifest = load_manifest(&root).expect("parse Packager.toml");
        assert!(!manifest.apps.is_empty());
        assert!(manifest.apps.iter().any(|a| a.name == "egui-app"));
        assert!(!manifest.apps.iter().any(|a| a.cargo_package == "cli"));
    }
}
