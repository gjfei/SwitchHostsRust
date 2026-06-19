//! 回收站列表（对齐 `LeftPanel/Trashcan` + `TrashcanItem` + `List` 行样式）。

use switch_hosts_core::storage::trashcan::{TrashItem, Trashcan};
use eframe::egui::{self, Color32, Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::panels::drawer::{draw_confirm_modal, ConfirmModalResult};
use crate::panels::widgets::ellipsize_text;
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme::{self, layout};

const ROW_ICON: f32 = 16.0;
const ROW_ICON_GAP: f32 = 8.0;

/// 回收站交互结果。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TrashEvent {
    #[default]
    None,
    SelectionChanged,
    RestoreRequested(String),
    DeleteRequested(String),
    ClearRequested,
}

pub fn draw_trash_panel(
    ui: &mut egui::Ui,
    trashcan: &Trashcan,
    selected_id: &mut Option<String>,
) -> TrashEvent {
    let t = theme::app(ui.ctx());
    let mut event = TrashEvent::None;
    ui.painter().rect_filled(ui.max_rect(), 0.0, t.sidebar_bg);

    ui.vertical(|ui| {
        ui.set_width(ui.max_rect().width());
        if let Some(header_event) = draw_trash_header(ui, trashcan.items.is_empty(), &t) {
            event = header_event;
        }

        ui.add_space(layout::TRASH_BODY_PAD_Y);
        egui::Frame::new()
            .fill(t.sidebar_bg)
            .inner_margin(egui::Margin::symmetric(10, 0))
            .show(ui, |ui| {
                if trashcan.items.is_empty() {
                    draw_trash_empty(ui, &t);
                } else {
                    let body_event = draw_trash_list(ui, trashcan, selected_id, &t);
                    if body_event != TrashEvent::None {
                        event = body_event;
                    }
                }
            });
    });

    event
}

fn draw_trash_header(ui: &mut Ui, is_empty: bool, t: &theme::AppTheme) -> Option<TrashEvent> {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(width, layout::TRASH_HEADER_HEIGHT),
        Sense::hover(),
    );
    if !ui.is_rect_visible(rect) {
        return None;
    }

    ui.painter().line_segment(
        [
            egui::pos2(rect.left(), rect.bottom()),
            egui::pos2(rect.right(), rect.bottom()),
        ],
        Stroke::new(1.0, t.separator),
    );

    let title_x = rect.left() + layout::TRASH_HEADER_PAD_X;
    let title_galley = text_align::layout_vcentered_galley(
        ui,
        "回收站".to_string(),
        ui_font_id(layout::TRASH_HEADER_FONT_SIZE),
        t.trash_header_text,
        ICON_ROW_LINE_HEIGHT,
    );
    text_align::paint_galley_row_centered(
        ui,
        title_x,
        rect.center().y,
        title_galley,
        t.trash_header_text,
    );

    let clear_rect = egui::Rect::from_center_size(
        egui::pos2(
            rect.right() - layout::TRASH_HEADER_PAD_X - layout::TRASH_CLEAR_HIT * 0.5,
            rect.center().y,
        ),
        Vec2::splat(layout::TRASH_CLEAR_HIT),
    );
    let clear_resp = ui.interact(
        clear_rect,
        ui.id().with("trash_clear"),
        if is_empty {
            Sense::hover()
        } else {
            Sense::click()
        },
    );

    if ui.is_rect_visible(clear_rect) {
        if !is_empty && clear_resp.hovered() {
            ui.painter()
                .rect_filled(clear_rect, t.corner_icon(), t.icon_hover_bg);
        }
        let tint = if is_empty {
            Color32::from_rgb(200, 200, 205)
        } else {
            t.accent
        };
        icons::paint_icon(
            ui,
            Icon::TrashX,
            clear_rect.center(),
            layout::TRASH_CLEAR_ICON,
            tint,
        );
    }

    if !is_empty && clear_resp.clicked() {
        Some(TrashEvent::ClearRequested)
    } else {
        None
    }
}

fn draw_trash_empty(ui: &mut Ui, t: &theme::AppTheme) {
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), 80.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.add_space(24.0);
            ui.label(
                egui::RichText::new("回收站为空")
                    .size(layout::TREE_FONT_SIZE * 0.9)
                    .color(t.trash_header_text),
            );
        },
    );
}

fn draw_trash_list(
    ui: &mut Ui,
    trashcan: &Trashcan,
    selected_id: &mut Option<String>,
    t: &theme::AppTheme,
) -> TrashEvent {
    let mut event = TrashEvent::None;

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = layout::TREE_ROW_GAP;
            for item in &trashcan.items {
                if let Some(row_event) = draw_trash_row(ui, item, selected_id, t) {
                    event = row_event;
                }
            }
        });

    event
}

fn draw_trash_row(
    ui: &mut Ui,
    item: &TrashItem,
    selected_id: &mut Option<String>,
    t: &theme::AppTheme,
) -> Option<TrashEvent> {
    let title = item
        .node
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&item.id)
        .to_string();
    let is_selected = selected_id.as_deref() == Some(item.id.as_str());

    let row_width = ui.available_width();
    let response = ui.allocate_response(Vec2::new(row_width, layout::TREE_ROW_HEIGHT), Sense::click());
    let rect = response.rect;

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
        let cy = rect.center().y;
        let mut x = rect.left() + layout::TREE_INDENT_PAD;

        icons::paint_icon(
            ui,
            icons::node_icon(&item.node, true),
            egui::pos2(x + ROW_ICON * 0.5, cy),
            ROW_ICON,
            text_color,
        );
        x += ROW_ICON + ROW_ICON_GAP;

        let title_rect = egui::Rect::from_min_max(
            egui::pos2(x, rect.top()),
            egui::pos2(rect.right(), rect.bottom()),
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
    }

    let mut event = None;
    response.context_menu(|ui| {
        if trash_menu_item(ui, "恢复").clicked() {
            event = Some(TrashEvent::RestoreRequested(item.id.clone()));
            ui.close_menu();
        }
        ui.separator();
        if trash_menu_item(ui, "删除").clicked() {
            event = Some(TrashEvent::DeleteRequested(item.id.clone()));
            ui.close_menu();
        }
    });

    if response.clicked() {
        *selected_id = Some(item.id.clone());
        return Some(TrashEvent::SelectionChanged);
    }

    event
}

fn trash_menu_item(ui: &mut Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(label)
            .frame(false)
            .fill(Color32::TRANSPARENT),
    )
}

/// 永久删除确认对话框（对齐 `ConfirmModal`）。
pub fn draw_trash_delete_confirm(
    ctx: &egui::Context,
    item_id: &str,
    title: &str,
) -> TrashDeleteConfirmResult {
    draw_trash_confirm_dialog(
        ctx,
        "删除",
        &format!("确定永久删除「{title}」？此操作不可撤销。"),
        item_id.to_string(),
    )
}

/// 清空回收站确认（对齐 `Trashcan` clear `ConfirmModal`）。
pub fn draw_trash_clear_confirm(ctx: &egui::Context) -> TrashDeleteConfirmResult {
    draw_trash_confirm_dialog(
        ctx,
        "清空回收站",
        "确定清空回收站？所有项目将被永久删除，此操作不可撤销。",
        String::new(),
    )
}

fn draw_trash_confirm_dialog(
    ctx: &egui::Context,
    window_title: &str,
    message: &str,
    confirm_token: String,
) -> TrashDeleteConfirmResult {
    match draw_confirm_modal(
        ctx,
        &format!("trash_confirm_{window_title}"),
        window_title,
        message,
        "删除",
        true,
    ) {
        ConfirmModalResult::None => TrashDeleteConfirmResult::None,
        ConfirmModalResult::Cancelled => TrashDeleteConfirmResult::Cancelled,
        ConfirmModalResult::Confirmed => TrashDeleteConfirmResult::Confirmed(confirm_token),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TrashDeleteConfirmResult {
    #[default]
    None,
    Cancelled,
    Confirmed(String),
}
