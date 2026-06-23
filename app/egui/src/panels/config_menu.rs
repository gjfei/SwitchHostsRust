//! 设置菜单（对齐 `SwitchHosts/src/renderer/components/TopBar/ConfigMenu.tsx`）。

use eframe::egui::{Response, Ui};

use crate::icons::Icon;
use crate::panels::menu::{self};

/// 设置菜单项触发的动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigMenuAction {
    #[default]
    None,
    OpenAbout,
    CheckUpdate,
    OpenFeedback,
    OpenHomepage,
    Export,
    Import,
    ImportFromUrl,
    OpenPreferences,
    Quit,
}

/// 点击左侧栏设置图标后，在右侧弹出菜单（对齐 `menuPosition="right-end"`）。
pub fn show_config_menu(ui: &Ui, anchor: &Response) -> ConfigMenuAction {
    let popup_id = ui.make_persistent_id("config_menu");
    menu::toggle_click_menu(ui, popup_id, anchor);
    menu::show_menu_if_open(ui, popup_id, Some(anchor), draw_config_menu_items)
        .unwrap_or(ConfigMenuAction::None)
}

fn draw_config_menu_items(m: &mut dyn menu::MenuContents) -> ConfigMenuAction {
    if m.item_icon(Icon::InfoCircle, "关于") {
        return ConfigMenuAction::OpenAbout;
    }

    m.divider();

    if m.item_icon(Icon::Refresh, "检查更新") {
        return ConfigMenuAction::CheckUpdate;
    }
    if m.item_icon(Icon::Message2, "反馈") {
        return ConfigMenuAction::OpenFeedback;
    }
    if m.item_icon(Icon::Home, "主页") {
        return ConfigMenuAction::OpenHomepage;
    }

    m.divider();

    if m.item_icon(Icon::Upload, "导出") {
        return ConfigMenuAction::Export;
    }
    if m.item_icon(Icon::Download, "导入") {
        return ConfigMenuAction::Import;
    }
    if m.item_icon(Icon::CloudDownload, "从 URL 导入") {
        return ConfigMenuAction::ImportFromUrl;
    }

    m.divider();

    if m.item_icon(Icon::Adjustments, "偏好设置") {
        return ConfigMenuAction::OpenPreferences;
    }

    m.divider();

    if m.item_icon(Icon::Logout, "退出") {
        return ConfigMenuAction::Quit;
    }

    ConfigMenuAction::None
}
