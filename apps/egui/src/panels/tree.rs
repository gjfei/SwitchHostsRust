//! 方案列表侧栏（对齐 `List` + `ListItem`）。

use switch_hosts_core::manifest_edit::SYSTEM_NODE_ID;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::toggle::toggle_item;
use eframe::egui::{self, Sense, Ui, Vec2};
use serde_json::{json, Value};

use crate::icons::{self, Icon};
use crate::panels::widgets::{ellipsize_text, toggle_switch_at};
use crate::theme::{
    ACCENT, SIDEBAR_BG, SWITCH_HEIGHT, SWITCH_WIDTH, TREE_FONT_SIZE, TREE_INDENT,
    TREE_INDENT_PAD, TREE_ROW_GAP, TREE_ROW_HEIGHT, TREE_ROW_RADIUS, TREE_STATUS_GAP,
    TREE_STATUS_RIGHT, TREE_HOVER, TREE_TEXT, TREE_TEXT_SELECTED,
};

const ROW_ICON: f32 = 16.0;
const TREE_EDIT_SIZE: f32 = 20.0;
const TREE_TITLE_GAP: f32 = 4.0;

/// 侧栏交互结果。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TreeEvent {
    #[default]
    None,
    SelectionChanged,
    Toggled,
    CollapsedChanged,
    EditRequested(String),
    AddRequested,
    MoveToTrashRequested(Vec<String>),
    RefreshRequested(String),
}

pub fn draw_hosts_tree(
    ui: &mut Ui,
    manifest: &mut Manifest,
    selected_id: &mut Option<String>,
    config: &AppConfig,
) -> TreeEvent {
    let mut event = TreeEvent::None;
    let mut pending_toggle_id = None;

    let _scroll = egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let bg = ui.available_rect_before_wrap();
            let bg_resp = ui.interact(bg, ui.id().with("tree_bg"), Sense::click());
            bg_resp.context_menu(|ui| {
                if menu_item(ui, "添加方案").clicked() {
                    event = TreeEvent::AddRequested;
                    ui.close_menu();
                }
            });

            ui.spacing_mut().item_spacing.y = TREE_ROW_GAP;
            if draw_system_row(ui, selected_id, &mut event) {
                // system selected
            }
            for node in manifest.root.iter_mut() {
                if node.get("isSys").and_then(|v| v.as_bool()).unwrap_or(false)
                    || node.get("is_sys").and_then(|v| v.as_bool()).unwrap_or(false)
                {
                    continue;
                }
                render_node(
                    ui,
                    node,
                    selected_id,
                    &mut event,
                    &mut pending_toggle_id,
                    config,
                    0,
                );
            }
        });

    if let Some(id) = pending_toggle_id {
        toggle_item(&mut manifest.root, &id, config.choice_mode);
        event = TreeEvent::Toggled;
    }

    event
}

fn draw_system_row(ui: &mut Ui, selected_id: &mut Option<String>, event: &mut TreeEvent) -> bool {
    let mut system = json!({
        "id": SYSTEM_NODE_ID,
        "title": "System Hosts",
        "isSys": true,
        "type": "local"
    });
    let mut none_toggle: Option<String> = None;
    render_row(
        ui,
        &mut system,
        selected_id,
        event,
        &mut none_toggle,
        false,
        0,
        true,
    );
    selected_id.as_deref() == Some(SYSTEM_NODE_ID)
}

fn render_node(
    ui: &mut Ui,
    node: &mut Value,
    selected_id: &mut Option<String>,
    event: &mut TreeEvent,
    pending_toggle_id: &mut Option<String>,
    config: &AppConfig,
    depth: usize,
) {
    let id = node
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if id.is_empty() {
        return;
    }

    let node_type = node.get("type").and_then(|v| v.as_str()).unwrap_or("local");
    let collapsed = node
        .get("is_collapsed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_folder = node_type == "folder";

    render_row(
        ui,
        node,
        selected_id,
        event,
        pending_toggle_id,
        is_folder,
        depth,
        false,
    );

    if is_folder && !collapsed {
        if let Some(children) = node
            .as_object_mut()
            .and_then(|o| o.get_mut("children"))
            .and_then(|c| c.as_array_mut())
        {
            for child in children.iter_mut() {
                render_node(
                    ui,
                    child,
                    selected_id,
                    event,
                    pending_toggle_id,
                    config,
                    depth + 1,
                );
            }
        }
    }
}

fn render_row(
    ui: &mut Ui,
    node: &mut Value,
    selected_id: &mut Option<String>,
    event: &mut TreeEvent,
    pending_toggle_id: &mut Option<String>,
    is_folder: bool,
    depth: usize,
    is_system: bool,
) {
    let id = node
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let title = node
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&id)
        .to_string();
    let on = node.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
    let collapsed = node
        .get("is_collapsed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let is_selected = selected_id.as_deref() == Some(id.as_str());
    let is_remote = node.get("type").and_then(|v| v.as_str()) == Some("remote");

    let indent = depth as f32 * TREE_INDENT + TREE_INDENT_PAD;
    let row_width = ui.available_width();
    let response = ui.allocate_response(Vec2::new(row_width, TREE_ROW_HEIGHT), Sense::click());
    let rect = response.rect;

    let mut switch_clicked = false;
    let cy = rect.center().y;

    if ui.is_rect_visible(rect) {
        let row_bg = if is_selected {
            Some(ACCENT)
        } else if response.hovered() {
            Some(TREE_HOVER)
        } else {
            None
        };
        if let Some(bg) = row_bg {
            ui.painter().rect_filled(rect, TREE_ROW_RADIUS, bg);
        }

        let text_color = if is_selected {
            TREE_TEXT_SELECTED
        } else {
            TREE_TEXT
        };

        let mut x = rect.left() + indent;

        if is_folder {
            let chevron = if collapsed {
                Icon::ChevronRight
            } else {
                Icon::ChevronDown
            };
            let arrow_rect = egui::Rect::from_min_size(
                egui::pos2(x, cy - 8.0),
                Vec2::new(14.0, 16.0),
            );
            let arrow_resp =
                ui.interact(arrow_rect, ui.id().with(&id).with("arrow"), Sense::click());
            icons::paint_icon(
                ui,
                chevron,
                arrow_rect.center(),
                14.0,
                text_color,
            );
            if arrow_resp.clicked() {
                if let Some(obj) = node.as_object_mut() {
                    let c = obj
                        .get("is_collapsed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    obj.insert("is_collapsed".into(), json!(!c));
                }
                *event = TreeEvent::CollapsedChanged;
            }
            x += 16.0;
        }

        icons::paint_icon(
            ui,
            icons::node_icon(node, collapsed),
            egui::pos2(x + ROW_ICON * 0.5, cy),
            ROW_ICON,
            text_color,
        );
        x += 20.0;

        let switch_left = rect.right() - TREE_STATUS_RIGHT - SWITCH_WIDTH;
        let title_right = if is_system {
            rect.right()
        } else if is_selected {
            switch_left - TREE_STATUS_GAP - TREE_EDIT_SIZE - TREE_TITLE_GAP
        } else {
            switch_left - TREE_TITLE_GAP
        };
        let title_rect = egui::Rect::from_min_max(
            egui::pos2(x, rect.top()),
            egui::pos2(title_right.max(x), rect.bottom()),
        );
        let font_id = egui::FontId::proportional(TREE_FONT_SIZE);
        let display_title = ellipsize_text(ui, &title, font_id.clone(), title_rect.width());
        ui.painter().with_clip_rect(title_rect).text(
            egui::pos2(x, cy),
            egui::Align2::LEFT_CENTER,
            display_title,
            font_id,
            text_color,
        );

        if !is_system {
            if is_selected {
                let edit_size = TREE_EDIT_SIZE;
                let edit_left = switch_left - TREE_STATUS_GAP - edit_size;
                let edit_rect = egui::Rect::from_min_size(
                    egui::pos2(edit_left, cy - edit_size * 0.5),
                    Vec2::new(edit_size, edit_size),
                );
                let edit_resp =
                    ui.interact(edit_rect, ui.id().with(&id).with("edit"), Sense::click());
                icons::paint_icon(
                    ui,
                    Icon::Pencil,
                    edit_rect.center(),
                    14.0,
                    text_color,
                );
                if edit_resp.clicked() {
                    *event = TreeEvent::EditRequested(id.clone());
                }
            }

            let switch_rect = egui::Rect::from_min_size(
                egui::pos2(switch_left, cy - SWITCH_HEIGHT * 0.5),
                Vec2::new(SWITCH_WIDTH, SWITCH_HEIGHT),
            );
            let switch_resp = toggle_switch_at(
                ui,
                switch_rect,
                ui.id().with(&id).with("switch"),
                on,
            );
            switch_clicked = switch_resp.clicked();
            if switch_clicked {
                *pending_toggle_id = Some(id.clone());
            }
        }
    }

    if !is_system {
        response.context_menu(|ui| {
            hosts_item_context_menu(ui, &id, is_remote, selected_id, event);
        });
    }

    if response.clicked() && !switch_clicked {
        *selected_id = Some(id);
        *event = TreeEvent::SelectionChanged;
    }
}

fn hosts_item_context_menu(
    ui: &mut Ui,
    id: &str,
    is_remote: bool,
    _selected_id: &Option<String>,
    event: &mut TreeEvent,
) {
    if menu_item(ui, "编辑").clicked() {
        *event = TreeEvent::EditRequested(id.to_string());
        ui.close_menu();
    }
    if is_remote && menu_item(ui, "刷新").clicked() {
        *event = TreeEvent::RefreshRequested(id.to_string());
        ui.close_menu();
    }
    ui.separator();
    if menu_item(ui, "移至回收站").clicked() {
        *event = TreeEvent::MoveToTrashRequested(vec![id.to_string()]);
        ui.close_menu();
    }
}

fn menu_item(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .frame(false)
            .fill(egui::Color32::TRANSPARENT),
    )
}

/// 绘制方案列表面板。
pub fn draw_hosts_sidebar(
    ui: &mut Ui,
    manifest: &mut Manifest,
    selected_id: &mut Option<String>,
    config: &AppConfig,
) -> TreeEvent {
    egui::Frame::new()
        .fill(SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| draw_hosts_tree(ui, manifest, selected_id, config))
        .inner
}
