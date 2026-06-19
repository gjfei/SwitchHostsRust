//! 原生系统托盘（tray-icon + muda）。

use std::collections::HashMap;

use switch_hosts_core::storage::manifest::Manifest;
use tray_icon::menu::{
    CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem,
};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::tray::build_tray_menu;

/// 托盘菜单触发的动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrayAction {
    ShowWindow,
    Quit,
    ToggleScheme(String),
}

struct MenuIds {
    show_window: MenuId,
    quit: MenuId,
    schemes: HashMap<MenuId, String>,
}

/// 原生托盘控制器；持有 `TrayIcon` 生命周期。
pub struct TrayController {
    tray: TrayIcon,
    ids: MenuIds,
}

impl TrayController {
    /// 创建托盘；CI 或无系统托盘环境时返回 `None`。
    pub fn try_new(manifest: &Manifest) -> Option<Self> {
        if std::env::var("SWITCH_HOSTS_RUST_DISABLE_TRAY").is_ok() {
            return None;
        }
        let icon = default_tray_icon();
        let (menu, ids) = build_native_menu(manifest)?;
        let tray = TrayIconBuilder::new()
            .with_tooltip("SwitchHostsRust")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .ok()?;
        Some(Self { tray, ids })
    }

    /// 方案变更后刷新托盘菜单。
    pub fn refresh(&mut self, manifest: &Manifest) {
        if let Some((menu, ids)) = build_native_menu(manifest) {
            let _ = self.tray.set_menu(Some(Box::new(menu)));
            self.ids = ids;
        }
    }

    /// 将菜单事件映射为业务动作。
    pub fn map_menu_event(&self, event: &MenuEvent) -> Option<TrayAction> {
        if event.id == self.ids.show_window {
            return Some(TrayAction::ShowWindow);
        }
        if event.id == self.ids.quit {
            return Some(TrayAction::Quit);
        }
        self.ids
            .schemes
            .get(&event.id)
            .cloned()
            .map(TrayAction::ToggleScheme)
    }

    /// 双击托盘图标显示主窗口。
    pub fn map_tray_event(event: &TrayIconEvent) -> Option<TrayAction> {
        match event {
            TrayIconEvent::DoubleClick { .. } => Some(TrayAction::ShowWindow),
            _ => None,
        }
    }
}

fn build_native_menu(manifest: &Manifest) -> Option<(Menu, MenuIds)> {
    let menu = Menu::new();
    let show_window = MenuItem::new("显示主窗口", true, None);
    let quit = MenuItem::new("退出", true, None);
    menu.append(&show_window).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;

    let mut schemes = HashMap::new();
    for entry in build_tray_menu(manifest) {
        let item = CheckMenuItem::new(entry.label, true, entry.checked, None);
        let id = item.id().clone();
        menu.append(&item).ok()?;
        schemes.insert(id, entry.id);
    }

    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit).ok()?;

    Some((
        menu,
        MenuIds {
            show_window: show_window.id().clone(),
            quit: quit.id().clone(),
            schemes,
        },
    ))
}

/// 生成简单蓝色方块托盘图标（无外部资源文件）。
pub fn default_tray_icon() -> Icon {
    let size = 32u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let i = ((y * size + x) * 4) as usize;
            rgba[i] = 48;
            rgba[i + 1] = 128;
            rgba[i + 2] = 220;
            rgba[i + 3] = 255;
        }
    }
    Icon::from_rgba(rgba, size, size).expect("tray icon rgba")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_icon_builds_without_panic() {
        let _icon = default_tray_icon();
    }
}
