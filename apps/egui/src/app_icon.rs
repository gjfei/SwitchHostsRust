//! 应用与托盘位图 — 消费 [`ui_assets::app_icons`] 字节。

use std::path::PathBuf;

use tray_icon::Icon as TrayIcon;
use ui_assets::app_icons;

pub fn dock_icns_path() -> PathBuf {
    app_icons::dock_icns_path()
}

pub fn tray_icon() -> TrayIcon {
    let img = image::load_from_memory(app_icons::tray_png_bytes()).expect("tray icon png");
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    TrayIcon::from_rgba(rgba.into_raw(), width, height).expect("tray icon rgba")
}

#[cfg(not(target_os = "macos"))]
pub fn window_icon() -> egui::IconData {
    let img = image::load_from_memory(app_icons::window_icon_png_bytes()).expect("window icon png");
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
