//! 应用与托盘位图（SwitchHosts `src-tauri/icons/` 副本，见 `cargo sync-icons`）。

use std::path::PathBuf;

#[cfg(target_os = "macos")]
const TRAY_PNG: &[u8] = include_bytes!("../app-icons/tray-mac.png");
#[cfg(not(target_os = "macos"))]
const TRAY_PNG: &[u8] = include_bytes!("../app-icons/tray.png");

const WINDOW_ICON_PNG: &[u8] = include_bytes!("../app-icons/icon.png");
const APP_ICON_ICNS: &[u8] = include_bytes!("../app-icons/icon.icns");
const APP_ICON_ICO: &[u8] = include_bytes!("../app-icons/icon.ico");

/// 托盘 PNG（macOS: tray-mac.png，其它: tray.png）。
pub fn tray_png_bytes() -> &'static [u8] {
    TRAY_PNG
}

/// 窗口图标 PNG（全平台通用资源）。
pub fn window_icon_png_bytes() -> &'static [u8] {
    WINDOW_ICON_PNG
}

/// macOS 应用图标 ICNS 字节。
pub fn app_icon_icns_bytes() -> &'static [u8] {
    APP_ICON_ICNS
}

/// Windows 应用图标 ICO 字节。
pub fn app_icon_ico_bytes() -> &'static [u8] {
    APP_ICON_ICO
}

/// 源码树内 `icon.icns` 路径（开发 / 非 bundle 运行时使用）。
pub fn dock_icns_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("app-icons/icon.icns")
}

/// 应用位图资源目录。
pub fn app_icons_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("app-icons")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_png_is_non_empty() {
        assert!(tray_png_bytes().len() > 100);
    }

    #[test]
    fn window_icon_png_is_non_empty() {
        assert!(window_icon_png_bytes().len() > 1000);
    }

    #[test]
    fn dock_icns_exists_in_source_tree() {
        assert!(dock_icns_path().is_file());
    }
}
