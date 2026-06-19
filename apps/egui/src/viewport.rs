//! 根窗口 viewport 配置（对齐 SwitchHosts `lifecycle::create_main_window`）。

use switch_hosts_core::storage::config::AppConfig;
use eframe::egui::ViewportBuilder;

/// 构建主窗口 viewport：默认自定义标题栏，TopBar 绘制在系统标题栏区域。
pub fn root_viewport_builder(config: &AppConfig) -> ViewportBuilder {
    let mut builder = ViewportBuilder::default()
        .with_title("SwitchHosts")
        .with_inner_size([960.0, 640.0])
        .with_min_inner_size([300.0, 200.0]);

    if config.use_system_window_frame {
        return builder;
    }

    #[cfg(target_os = "macos")]
    {
        // 对齐 Tauri TitleBarStyle::Overlay + hidden_title
        builder = builder
            .with_fullsize_content_view(true)
            .with_title_shown(false)
            .with_titlebar_shown(false)
            .with_titlebar_buttons_shown(true);
    }

    #[cfg(not(target_os = "macos"))]
    {
        builder = builder.with_decorations(false);
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_frame_when_not_system_frame() {
        let config = AppConfig {
            use_system_window_frame: false,
            ..AppConfig::default()
        };
        let builder = root_viewport_builder(&config);
        assert!(builder.title.as_deref() == Some("SwitchHosts"));

        #[cfg(target_os = "macos")]
        {
            assert_eq!(builder.fullsize_content_view, Some(true));
            assert_eq!(builder.title_shown, Some(false));
            assert_eq!(builder.titlebar_shown, Some(false));
            assert_eq!(builder.titlebar_buttons_shown, Some(true));
        }

        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(builder.decorations, Some(false));
        }
    }

    #[test]
    fn system_frame_leaves_viewport_defaults() {
        let config = AppConfig {
            use_system_window_frame: true,
            ..AppConfig::default()
        };
        let builder = root_viewport_builder(&config);
        assert_eq!(builder.fullsize_content_view, None);
        assert_eq!(builder.decorations, None);
    }
}
