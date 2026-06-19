//! 偏好设置变更后的运行时副作用。

use eframe::egui;

use switch_hosts_core::hosts_apply::target::HostsTarget;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;

use crate::http_api_runtime::HttpApiRuntime;
use crate::lifecycle;
use crate::theme;

pub fn apply_config_side_effects(
    ctx: &egui::Context,
    config: &AppConfig,
    paths: &AppPaths,
    target: &HostsTarget,
    system_dark: bool,
    http_api: &mut HttpApiRuntime,
) {
    theme::apply_theme(ctx, &config.theme, system_dark);
    if let Ok(exe) = std::env::current_exe() {
        let _ = lifecycle::sync_launch_at_login(config, &exe);
    }
    http_api.sync(config, paths, target);
    let _ = paths;
}

pub fn reveal_path_in_file_manager(path: &std::path::Path) {
    if path.as_os_str().is_empty() {
        return;
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg("-R").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let dir = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        };
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = path;
    }
}
