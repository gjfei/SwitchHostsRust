//! 应用与托盘图标 — 直接嵌入 SwitchHosts `src-tauri/icons/` 副本（见 `scripts/sync-icons.sh`）。

use tray_icon::Icon as TrayIcon;

// 对齐 SwitchHosts `src-tauri/src/tray.rs`
#[cfg(target_os = "macos")]
const TRAY_PNG: &[u8] = include_bytes!("../icons/tray-mac.png");
#[cfg(not(target_os = "macos"))]
const TRAY_PNG: &[u8] = include_bytes!("../icons/tray.png");

/// Dock 图标（SwitchHosts `icon.icns`，与 Tauri bundle 一致）。
const DOCK_ICNS: &[u8] = include_bytes!("../icons/icon.icns");

/// Windows / Linux 窗口图标（SwitchHosts `icon.png`）。
#[cfg(not(target_os = "macos"))]
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");

/// 供 macOS `NSImage::initWithData` 使用的 icns 字节。
pub fn dock_icns_bytes() -> &'static [u8] {
    DOCK_ICNS
}

/// 系统托盘图标（macOS: tray-mac.png，其它: tray.png）。
pub fn tray_icon() -> TrayIcon {
    let img = image::load_from_memory(TRAY_PNG).expect("tray icon png");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    TrayIcon::from_rgba(rgba.into_raw(), width, height).expect("tray icon rgba")
}

/// 窗口图标（Windows / Linux）。
#[cfg(not(target_os = "macos"))]
pub fn window_icon() -> egui::IconData {
    from_png_bytes(WINDOW_ICON_PNG).expect("window icon png")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_icon_builds_without_panic() {
        let _ = tray_icon();
    }

    #[test]
    fn dock_icns_has_magic() {
        assert_eq!(&dock_icns_bytes()[0..4], b"icns");
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn window_icon_has_reasonable_size() {
        let icon = window_icon();
        assert!(icon.width >= 128 && icon.height >= 128);
    }
}
