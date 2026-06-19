//! macOS 窗口 chrome 调整（对齐 SwitchHosts Tauri `traffic_light_position`）。

use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use crate::theme::{TOP_BAR_HEIGHT, TOP_BAR_TRAFFIC_LIGHT_X, TOP_BAR_TRAFFIC_LIGHT_Y};

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
        inset_traffic_lights(
            &window,
            f64::from(TOP_BAR_TRAFFIC_LIGHT_X),
            f64::from(TOP_BAR_TRAFFIC_LIGHT_Y),
            f64::from(TOP_BAR_HEIGHT),
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
    fn traffic_light_constants_match_switchhosts() {
        assert_eq!(TOP_BAR_TRAFFIC_LIGHT_X, 12.0);
        assert_eq!(TOP_BAR_TRAFFIC_LIGHT_Y, 18.0);
    }
}
