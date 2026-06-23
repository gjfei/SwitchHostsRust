//! 方案列表侧栏（对齐 `List` + `ListItem`）。

use switch_hosts_core::manifest_edit::SYSTEM_NODE_ID;
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::manifest::Manifest;
use eframe::egui::{self, Sense, Stroke, Ui, Vec2};
use serde_json::{json, Value};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::panels::menu::{self};
use crate::panels::widgets::{ellipsize_text, toggle_switch_at};
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme::{self, layout};

const ROW_ICON: f32 = 16.0;
const TREE_EDIT_SIZE: f32 = 20.0;
const TREE_TITLE_GAP: f32 = 4.0;

/// 侧栏交互结果。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TreeEvent {
    #[default]
    None,
    SelectionChanged,
    ToggleRequested(String),
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
    let t = theme::app(ui.ctx());
    let mut event = TreeEvent::None;
    let mut pending_toggle_id = None;

    let _scroll = egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            let bg = ui.available_rect_before_wrap();
            let bg_resp = ui.interact(bg, ui.id().with("tree_bg"), Sense::click());
            menu::open_context_menu(ui, &bg_resp);
            menu::show_context_menu_if_open(ui, &bg_resp, |m| {
                if m.item("添加方案") {
                    event = TreeEvent::AddRequested;
                }
            });

            ui.spacing_mut().item_spacing.y = layout::TREE_ROW_GAP;
            if draw_system_row(ui, selected_id, &mut event, &t) {
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
                    &t,
                );
            }
        });

    if let Some(id) = pending_toggle_id {
        event = TreeEvent::ToggleRequested(id);
    }

    event
}

fn draw_system_row(
    ui: &mut Ui,
    selected_id: &mut Option<String>,
    event: &mut TreeEvent,
    t: &theme::AppTheme,
) -> bool {
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
        t,
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
    t: &theme::AppTheme,
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
        t,
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
                    t,
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
    t: &theme::AppTheme,
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

    let indent = depth as f32 * layout::TREE_INDENT + layout::TREE_INDENT_PAD;
    let row_width = ui.available_width();
    let response = ui.allocate_response(Vec2::new(row_width, layout::TREE_ROW_HEIGHT), Sense::click());
    let rect = response.rect;

    let mut switch_clicked = false;
    let cy = rect.center().y;

    if ui.is_rect_visible(rect) {
        let row_bg = if is_selected {
            Some(t.accent)
        } else if response.hovered() {
            Some(t.tree_hover)
        } else {
            None
        };
        if let Some(bg) = row_bg {
            ui.painter().rect_filled(rect, layout::TREE_ROW_RADIUS, bg);
        }

        let text_color = if is_selected {
            t.text_selected
        } else {
            t.text
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

        let switch_left = rect.right() - layout::TREE_STATUS_RIGHT - layout::SWITCH_WIDTH;
        let title_right = if is_system {
            rect.right()
        } else if is_selected {
            switch_left - layout::TREE_STATUS_GAP - TREE_EDIT_SIZE - TREE_TITLE_GAP
        } else {
            switch_left - TREE_TITLE_GAP
        };
        let title_rect = egui::Rect::from_min_max(
            egui::pos2(x, rect.top()),
            egui::pos2(title_right.max(x), rect.bottom()),
        );
        let font_id = ui_font_id(layout::TREE_FONT_SIZE);
        let display_title = ellipsize_text(ui, &title, font_id.clone(), title_rect.width());
        let galley = text_align::layout_vcentered_galley(
            ui,
            display_title,
            font_id,
            text_color,
            ICON_ROW_LINE_HEIGHT,
        );
        text_align::paint_galley_row_centered_clipped(ui, title_rect, x, cy, galley, text_color);

        if !is_system {
            if is_selected {
                let edit_size = TREE_EDIT_SIZE;
                let edit_left = switch_left - layout::TREE_STATUS_GAP - edit_size;
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
                egui::pos2(switch_left, cy - layout::SWITCH_HEIGHT * 0.5),
                Vec2::new(layout::SWITCH_WIDTH, layout::SWITCH_HEIGHT),
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
        menu::open_context_menu(ui, &response);
        menu::show_context_menu_if_open(ui, &response, |m| {
            if m.item("编辑") {
                *event = TreeEvent::EditRequested(id.clone());
            }
            if m.item_if(is_remote, "刷新") {
                *event = TreeEvent::RefreshRequested(id.clone());
            }
            m.divider();
            if m.item("移至回收站") {
                *event = TreeEvent::MoveToTrashRequested(vec![id.clone()]);
            }
        });
    }

    if response.clicked() && !switch_clicked {
        *selected_id = Some(id);
        *event = TreeEvent::SelectionChanged;
    }
}

/// 列表区装饰边框（左/上/下，纯装饰；右侧由 SidePanel 分隔线绘制）。
pub fn paint_list_panel_border(ui: &Ui) {
    let t = theme::app(ui.ctx());
    let rect = ui.max_rect();
    if !ui.is_rect_visible(rect) {
        return;
    }
    let stroke = Stroke::new(1.0, t.separator);
    let p = ui.painter();
    p.vline(rect.left() + 0.5, rect.y_range(), stroke);
    p.hline(rect.x_range(), rect.top() + 0.5, stroke);
    p.hline(rect.x_range(), rect.bottom() - 0.5, stroke);
}

/// 绘制方案列表面板。
pub fn draw_hosts_sidebar(
    ui: &mut Ui,
    manifest: &mut Manifest,
    selected_id: &mut Option<String>,
    config: &AppConfig,
) -> TreeEvent {
    let t = theme::app(ui.ctx());
    egui::Frame::new()
        .fill(t.sidebar_bg)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| draw_hosts_tree(ui, manifest, selected_id, config))
        .inner
}
