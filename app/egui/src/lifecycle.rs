//! 应用生命周期：开机启动、启动时隐藏窗口等。

use std::path::Path;

use switch_hosts_core::storage::config::AppConfig;

const APP_NAME: &str = "SwitchHostsRust";

/// 根据配置同步「登录时启动」。
pub fn sync_launch_at_login(config: &AppConfig, exe_path: &Path) -> Result<(), String> {
    use auto_launch::AutoLaunchBuilder;

    let auto = AutoLaunchBuilder::new()
        .set_app_name(APP_NAME)
        .set_app_path(exe_path.to_string_lossy().as_ref())
        .build()
        .map_err(|e| e.to_string())?;

    let enabled = auto.is_enabled().map_err(|e| e.to_string())?;
    if config.launch_at_login && !enabled {
        auto.enable().map_err(|e| e.to_string())?;
    } else if !config.launch_at_login && enabled {
        auto.disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 启动时主窗口是否可见（`hide_at_launch` 为 true 则隐藏）。
pub fn initial_window_visible(config: &AppConfig) -> bool {
    !config.hide_at_launch
}

#[cfg(test)]
mod tests {
    use super::*;
    use switch_hosts_core::storage::config::AppConfig;

    #[test]
    fn hide_at_launch_hides_window() {
        let mut config = AppConfig::default();
        assert!(initial_window_visible(&config));
        config.hide_at_launch = true;
        assert!(!initial_window_visible(&config));
    }
}
