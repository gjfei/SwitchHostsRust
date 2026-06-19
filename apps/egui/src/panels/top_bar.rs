//! 全局顶栏（对齐 SwitchHosts `TopBar`，绘制在系统标题栏区域）。

use switch_hosts_core::manifest_edit::{is_editor_read_only, SYSTEM_NODE_ID};
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use switch_hosts_core::toggle::toggle_item;
use eframe::egui::{self, Align2, Color32, FontId, PointerButton, Sense, Ui, Vec2, ViewportCommand};

use crate::icons::{self, Icon};
use crate::panels::widgets::toggle_switch;
use crate::theme::{
    TOP_BAR_CLUSTER_WIDTH, TOP_BAR_HEIGHT, TOP_BAR_ICON_HIT, TOP_BAR_ICON_HOVER,
    TOP_BAR_ICON_RADIUS, TOP_BAR_MAC_PAD_LEFT, TOP_BAR_PAD_X,
};

pub struct TopBarAction {
    pub toggle_left_panel: bool,
    pub add_new: bool,
    pub toggle_right_panel: bool,
    pub toggled_current: bool,
}

const BAR_ICON: f32 = 20.0;
const CLUSTER_GAP: f32 = 8.0;
const TITLE_FONT_SIZE: f32 = 14.0;
const READONLY_BADGE_FONT_SIZE: f32 = 11.0;

pub fn draw_top_bar(
    ui: &mut Ui,
    manifest: &mut Manifest,
    selected_id: &Option<String>,
    left_panel_visible: bool,
    right_panel_visible: bool,
    choice_mode: u8,
    use_system_window_frame: bool,
) -> TopBarAction {
    let mut action = TopBarAction {
        toggle_left_panel: false,
        add_new: false,
        toggle_right_panel: false,
        toggled_current: false,
    };

    let gray = Color32::from_rgb(100, 100, 110);
    let bar_rect = ui.max_rect();
    ui.set_width(bar_rect.width());
    ui.set_min_height(TOP_BAR_HEIGHT);
    ui.set_max_height(TOP_BAR_HEIGHT);
    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

    if !use_system_window_frame {
        handle_title_bar_drag(ui, bar_rect);
    }

    let pad_left = if use_system_window_frame {
        TOP_BAR_PAD_X
    } else {
        TOP_BAR_PAD_X + mac_titlebar_pad_left()
    };

    let left_rect = egui::Rect::from_min_max(
        egui::pos2(bar_rect.left(), bar_rect.top()),
        egui::pos2(bar_rect.left() + TOP_BAR_CLUSTER_WIDTH, bar_rect.bottom()),
    );
    let right_rect = egui::Rect::from_min_max(
        egui::pos2(bar_rect.right() - TOP_BAR_CLUSTER_WIDTH, bar_rect.top()),
        egui::pos2(bar_rect.right(), bar_rect.bottom()),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
        ui.set_min_height(TOP_BAR_HEIGHT);
        ui.set_max_height(TOP_BAR_HEIGHT);
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.add_space(pad_left);
            if icon_btn(
                ui,
                if left_panel_visible {
                    Icon::SidebarLeftCollapse
                } else {
                    Icon::SidebarLeftExpand
                },
                gray,
            )
            .on_hover_text("显示/隐藏方案列表")
            .clicked()
            {
                action.toggle_left_panel = true;
            }
            ui.add_space(CLUSTER_GAP);
            if icon_btn(ui, Icon::Plus, gray)
                .on_hover_text("添加 hosts")
                .clicked()
            {
                action.add_new = true;
            }
        });
    });

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
        ui.set_min_height(TOP_BAR_HEIGHT);
        ui.set_max_height(TOP_BAR_HEIGHT);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(TOP_BAR_PAD_X);

            if show_window_controls(use_system_window_frame) {
                draw_window_controls(ui, gray);
                ui.add_space(4.0);
            }

            if icon_btn(
                ui,
                if right_panel_visible {
                    Icon::SidebarRightCollapse
                } else {
                    Icon::SidebarRightExpand
                },
                gray,
            )
            .on_hover_text("显示/隐藏详情面板")
            .clicked()
            {
                action.toggle_right_panel = true;
            }

            if !left_panel_visible {
                if let Some(id) = selected_id.clone() {
                    if id != SYSTEM_NODE_ID {
                        if let Some(node) = find_node(&manifest.root, &id) {
                            let on = node.get("on").and_then(|v| v.as_bool()).unwrap_or(false);
                            ui.add_space(12.0);
                            if toggle_switch(ui, on).clicked() {
                                toggle_item(&mut manifest.root, &id, choice_mode);
                                action.toggled_current = true;
                            }
                        }
                    }
                }
            }
        });
    });

    paint_centered_title(ui, bar_rect, manifest, selected_id, gray);

    action
}

/// 对齐 `.title { max-width: calc(100vw - ($w + $p) * 2) }`
fn title_area_rect(bar_rect: egui::Rect) -> egui::Rect {
    let max_w =
        (bar_rect.width() - 2.0 * TOP_BAR_CLUSTER_WIDTH - 2.0 * TOP_BAR_PAD_X).max(0.0);
    egui::Rect::from_center_size(bar_rect.center(), egui::vec2(max_w, TOP_BAR_HEIGHT))
}

fn paint_centered_title(
    ui: &Ui,
    bar_rect: egui::Rect,
    manifest: &Manifest,
    selected_id: &Option<String>,
    gray: Color32,
) {
    let (title, node_icon) = current_title(manifest, selected_id.as_deref());
    let node = selected_id
        .as_deref()
        .and_then(|id| find_node(&manifest.root, id));
    let read_only = is_editor_read_only(selected_id.as_deref(), node.as_ref());

    let title_rect = title_area_rect(bar_rect);
    if !ui.is_rect_visible(title_rect) {
        return;
    }

    let text_color = Color32::from_rgb(30, 30, 35);
    let font_id = FontId::proportional(TITLE_FONT_SIZE);
    let painter = ui.painter();

    let title_galley = painter.layout_no_wrap(title.clone(), font_id.clone(), text_color);
    let readonly_galley = if read_only {
        Some(painter.layout_no_wrap(
            "只读".to_string(),
            FontId::proportional(READONLY_BADGE_FONT_SIZE),
            Color32::from_rgb(120, 120, 130),
        ))
    } else {
        None
    };

    let title_width = title_galley.size().x;
    let mut total_w = Icon::DEFAULT_SIZE + CLUSTER_GAP + title_width;
    if let Some(g) = readonly_galley.as_ref() {
        total_w += CLUSTER_GAP + g.size().x + 8.0;
    }
    total_w = total_w.min(title_rect.width());

    let mut x = title_rect.center().x - total_w / 2.0;
    let y = title_rect.center().y;

    let icon_center = egui::pos2(x + Icon::DEFAULT_SIZE / 2.0, y);
    icons::paint_icon(ui, node_icon, icon_center, Icon::DEFAULT_SIZE, gray);
    x += Icon::DEFAULT_SIZE + CLUSTER_GAP;

    painter.galley(
        egui::pos2(x, y - title_galley.size().y / 2.0),
        title_galley,
        text_color,
    );
    x += title_width;

    if read_only {
        x += CLUSTER_GAP;
        let badge_galley = readonly_galley.expect("readonly galley");
        let badge_size = badge_galley.size() + egui::vec2(8.0, 4.0);
        let badge_rect = egui::Rect::from_min_size(
            egui::pos2(x, y - badge_size.y / 2.0),
            badge_size,
        );
        painter.rect_filled(
            badge_rect,
            4.0,
            Color32::from_rgb(233, 233, 236),
        );
        painter.galley(
            badge_rect.min + egui::vec2(4.0, 2.0),
            badge_galley,
            Color32::from_rgb(120, 120, 130),
        );
    }
}

fn mac_titlebar_pad_left() -> f32 {
    if cfg!(target_os = "macos") {
        TOP_BAR_MAC_PAD_LEFT
    } else {
        0.0
    }
}

fn show_window_controls(use_system_window_frame: bool) -> bool {
    !use_system_window_frame && !cfg!(target_os = "macos")
}

/// 对齐 `data-tauri-drag-region`：空白区域可拖动窗口。
fn handle_title_bar_drag(ui: &mut Ui, bar_rect: egui::Rect) {
    let response = ui.interact(
        bar_rect,
        ui.id().with("title_bar_drag"),
        Sense::click_and_drag(),
    );
    if response.drag_started_by(PointerButton::Primary) {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
}

fn draw_window_controls(ui: &mut Ui, gray: Color32) {
    if icon_btn(ui, Icon::X, gray)
        .on_hover_text("关闭")
        .clicked()
    {
        ui.ctx().send_viewport_cmd(ViewportCommand::Close);
    }
    let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
    if subtle_text_btn(
        ui,
        if maximized { "❐" } else { "□" },
        14.0,
        gray,
    )
    .on_hover_text(if maximized { "还原" } else { "最大化" })
    .clicked()
    {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    if subtle_text_btn(ui, "−", 14.0, gray)
        .on_hover_text("最小化")
        .clicked()
    {
        ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
    }
}

fn icon_btn(ui: &mut Ui, icon: Icon, tint: Color32) -> egui::Response {
    icons::subtle_icon_button(
        ui,
        icon,
        BAR_ICON,
        tint,
        TOP_BAR_ICON_HOVER,
        TOP_BAR_ICON_HIT,
        TOP_BAR_ICON_RADIUS,
    )
}

fn subtle_text_btn(ui: &mut Ui, label: &str, size: f32, color: Color32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::splat(TOP_BAR_ICON_HIT), Sense::click());
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, TOP_BAR_ICON_RADIUS, TOP_BAR_ICON_HOVER);
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(size),
            color,
        );
    }
    response
}

fn current_title(manifest: &Manifest, selected_id: Option<&str>) -> (String, Icon) {
    if selected_id == Some(SYSTEM_NODE_ID) || selected_id.is_none() {
        return ("系统 Hosts".to_string(), Icon::DeviceDesktop);
    }
    if let Some(id) = selected_id {
        if let Some(node) = find_node(&manifest.root, id) {
            let title = node
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(id)
                .to_string();
            return (title, icons::node_icon(&node, false));
        }
    }
    ("系统 Hosts".to_string(), Icon::DeviceDesktop)
}
