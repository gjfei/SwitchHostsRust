//! macOS 窗口 chrome 调整（对齐 SwitchHosts Tauri `traffic_light_position`）。

use std::path::PathBuf;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::app_icon;
use crate::theme::layout;

/// 是否在 `.app` bundle 内运行（Mission Control 图标依赖 bundle 元数据）。
pub fn running_in_app_bundle() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()
                .and_then(|macos| macos.parent())
                .map(|contents| contents.join("Info.plist"))
        })
        .is_some_and(|plist| plist.is_file())
}

/// 配置 Dock / Mission Control / Cmd+Tab 应用身份与图标。
///
/// - 使用 SwitchHosts `icon.icns`
/// - 强制 Regular 激活策略（避免被当作后台/终端附属进程）
/// - 进程名显示为 SwitchHosts
pub fn configure_macos_app() -> bool {
    use objc2::AnyThread;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSImage};
    use objc2_foundation::{NSProcessInfo, NSString};

    let Some(mtm) = MainThreadMarker::new() else {
        tracing::warn!("不在主线程，跳过 macOS 应用配置");
        return false;
    };

    let path = dock_icns_path();
    let path_str = path.to_string_lossy();
    unsafe {
        let image = NSImage::initWithContentsOfFile(
            NSImage::alloc(),
            &NSString::from_str(&path_str),
        );
        let Some(image) = image else {
            tracing::warn!("无法从 {} 加载应用图标", path.display());
            return false;
        };

        extern "C" {
            static NSApp: Option<&'static NSApplication>;
        }
        let app = if let Some(app) = NSApp {
            app
        } else {
            &*NSApplication::sharedApplication(mtm)
        };

        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        app.setApplicationIconImage(Some(&image));
        NSProcessInfo::processInfo().setProcessName(&NSString::from_str("SwitchHosts"));
    }

    crate::macos_delegate::install_app_delegate();

    if !running_in_app_bundle() {
        tracing::debug!(
            "未在 .app 内运行，Mission Control 可能不显示图标；请用 ./scripts/run-gui-macos.sh"
        );
    }

    true
}

/// 退出前清理：移除托盘并终止 NSApplication 事件循环。
pub fn quit_app() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    NSApplication::sharedApplication(mtm).terminate(None);
}

/// Dock 点击重新打开时，将应用激活到前台并显示窗口。
pub fn show_main_window() {
    crate::macos_delegate::show_windows_at_appkit_level();
}

/// 激活应用到前台（不强制显示窗口）。
pub fn activate_app() {
    show_main_window();
}

/// `.app` 内用 bundle Resources；`cargo run` 时用源码树 `icons/icon.icns`。
fn dock_icns_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(resources) = exe
            .parent()
            .and_then(|macos| macos.parent())
            .map(|contents| contents.join("Resources/icon.icns"))
        {
            if resources.is_file() {
                return resources;
            }
        }
    }

    let dev = app_icon::dock_icns_path();
    if dev.is_file() {
        return dev;
    }

    std::env::temp_dir().join("SwitchHostsRust-dock.icns")
}

/// 将交通灯移入 40px 顶栏区域并垂直居中（对齐 `lifecycle.rs`）。
/// 窗口尚未就绪时会失败，调用方应重试。
pub fn position_traffic_lights(handle: &impl HasWindowHandle) -> bool {
    let Ok(raw) = handle.window_handle().map(|h| h.as_raw()) else {
        tracing::warn!("无法获取窗口 handle，跳过交通灯定位");
        return false;
    };
    let RawWindowHandle::AppKit(appkit) = raw else {
        tracing::warn!("非 AppKit 窗口，跳过交通灯定位");
        return false;
    };

    unsafe {
        let ns_view = appkit.ns_view.as_ptr() as *const objc2_app_kit::NSView;
        if ns_view.is_null() {
            tracing::warn!("NSView 为空，跳过交通灯定位");
            return false;
        }
        let Some(window) = (&*ns_view).window() else {
            tracing::warn!("无法从 NSView 获取 NSWindow，跳过交通灯定位");
            return false;
        };
        crate::macos_delegate::register_main_ns_window(&window);
        inset_traffic_lights(
            &window,
            f64::from(layout::TOP_BAR_TRAFFIC_LIGHT_X),
            f64::from(layout::TOP_BAR_TRAFFIC_LIGHT_Y),
            f64::from(layout::TOP_BAR_HEIGHT),
        )
    }
}

/// 改编自 wry/tao `inset_traffic_lights`，额外在 `top_bar_height` 内垂直居中按钮。
unsafe fn inset_traffic_lights(
    window: &objc2_app_kit::NSWindow,
    x: f64,
    y: f64,
    top_bar_height: f64,
) -> bool {
    use objc2_app_kit::NSWindowButton;

    let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else {
        tracing::warn!("找不到关闭按钮，跳过交通灯定位");
        return false;
    };
    let Some(miniaturize) = window.standardWindowButton(NSWindowButton::MiniaturizeButton) else {
        tracing::warn!("找不到最小化按钮，跳过交通灯定位");
        return false;
    };
    let zoom = window.standardWindowButton(NSWindowButton::ZoomButton);

    let Some(title_bar_container_view) = close.superview().and_then(|v| v.superview()) else {
        tracing::warn!("找不到 title bar container，跳过交通灯定位");
        return false;
    };

    let close_rect = close.frame();
    let title_bar_frame_height = close_rect.size.height + y;
    let mut title_bar_rect = title_bar_container_view.frame();
    title_bar_rect.size.height = title_bar_frame_height.max(top_bar_height);
    title_bar_rect.origin.y = window.frame().size.height - title_bar_rect.size.height;
    title_bar_container_view.setFrame(title_bar_rect);

    let space_between = miniaturize.frame().origin.x - close_rect.origin.x;
    let button_y = (top_bar_height - close_rect.size.height) / 2.0;

    let mut window_buttons = vec![close, miniaturize];
    if let Some(zoom) = zoom {
        window_buttons.push(zoom);
    }

    for (i, button) in window_buttons.into_iter().enumerate() {
        let mut rect = button.frame();
        rect.origin.x = x + (i as f64 * space_between);
        rect.origin.y = button_y;
        button.setFrameOrigin(rect.origin);
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dock_icns_path_prefers_embedded_bytes_when_not_bundled() {
        let path = dock_icns_path();
        assert!(path.ends_with("SwitchHostsRust-dock.icns") || path.ends_with("icon.icns"));
    }
}
