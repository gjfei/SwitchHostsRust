//! 应用与托盘图标 — 直接嵌入 SwitchHosts `src-tauri/icons/` 副本（见 `scripts/sync-icons.sh`）。

use std::path::PathBuf;

use tray_icon::Icon as TrayIcon;

// 对齐 SwitchHosts `src-tauri/src/tray.rs`
#[cfg(target_os = "macos")]
const TRAY_PNG: &[u8] = include_bytes!("../icons/tray-mac.png");
#[cfg(not(target_os = "macos"))]
const TRAY_PNG: &[u8] = include_bytes!("../icons/tray.png");

/// Windows / Linux 窗口图标（SwitchHosts `icon.png`）。
#[cfg(not(target_os = "macos"))]
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");

/// 开发态 / 非 bundle 下的 icns 路径（避免在二进制内再嵌入一份 ~228KB）。
pub fn dock_icns_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.icns")
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
    let img = image::load_from_memory(WINDOW_ICON_PNG).expect("window icon png");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    egui::IconData {
        width,
        height,
        rgba: rgba.into_raw(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_icon_builds_without_panic() {
        let _ = tray_icon();
    }

    #[test]
    fn dock_icns_exists_in_source_tree() {
        assert!(dock_icns_path().is_file());
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn window_icon_has_reasonable_size() {
        let icon = window_icon();
        assert!(icon.width >= 128 && icon.height >= 128);
    }
}
