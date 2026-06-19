//! 系统 Hosts 历史抽屉（对齐 `SwitchHosts/src/renderer/components/History.tsx`）。

use switch_hosts_core::hosts_apply::{delete_by_id, list_history, ApplyHistoryItem};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;
use eframe::egui::{self, Color32, CornerRadius, Id, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::panels::drawer::{
    backdrop_dismiss_clicked, drawer_frame, drawer_select, draw_confirm_modal,
    draw_drawer_header, outline_button, outline_button_with_icon, paint_side_drawer_backdrop,
    side_drawer_geometry, ConfirmModalResult, DRAWER_BTN_H, DRAWER_INPUT_TEXT, DRAWER_SHADOW,
};
use crate::panels::editor::draw_readonly_hosts_viewer;
use crate::panels::status_bar::{draw_status_bar, EditorStatus};
use crate::panels::widgets::format_bytes;
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme::{
    DRAWER_FOOTER_HEIGHT, DRAWER_OFFSET, DRAWER_PAD, DRAWER_WEAK_TEXT, DRAWER_WIDTH, SEPARATOR,
    STATUS_BAR_HEIGHT, TOP_BAR_ICON_HOVER, TOP_BAR_ICON_RADIUS, TREE_HOVER,
};

const HISTORY_LIST_WIDTH: f32 = 200.0;
/// 对齐 `History.tsx` 面板 `borderRadius: 6`
const HISTORY_PANEL_RADIUS: CornerRadius = CornerRadius::same(6);
/// 对齐 `History.tsx` 预览与列表间距 `marginRight: 12`
const HISTORY_PANEL_GAP: f32 = 12.0;
const HISTORY_ITEM_PAD_X: f32 = 12.0;
const HISTORY_ITEM_PAD_Y: f32 = 8.0;
const HISTORY_ITEM_GAP: f32 = 8.0;
const HISTORY_ITEM_META: f32 = 9.0;
const HISTORY_LIMIT_SELECT_W: f32 = 100.0;

#[derive(Debug, Default)]
pub struct HistoryState {
    pub open: bool,
    open_last_frame: bool,
    items: Vec<ApplyHistoryItem>,
    selected_id: Option<String>,
    viewer_text: String,
    pending_delete_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryResult {
    #[default]
    None,
    Closed,
    ConfigChanged,
}

impl HistoryState {
    pub fn open_drawer(&mut self) {
        self.open = true;
    }

    fn reload(&mut self, paths: &AppPaths) {
        self.items = list_history(&paths.histories_dir).unwrap_or_default();
        let selected_still_exists = self
            .selected_id
            .as_ref()
            .is_some_and(|id| self.items.iter().any(|i| &i.id == id));
        if !selected_still_exists {
            self.selected_id = self.items.last().map(|i| i.id.clone());
        }
        self.sync_viewer();
    }

    fn sync_viewer(&mut self) {
        self.viewer_text = self
            .selected_id
            .as_ref()
            .and_then(|id| self.items.iter().find(|i| &i.id == id))
            .map(|i| i.content.clone())
            .unwrap_or_default();
    }

    fn select(&mut self, id: String) {
        self.selected_id = Some(id);
        self.sync_viewer();
    }

    fn close(&mut self) {
        self.open = false;
        self.pending_delete_id = None;
    }
}

fn history_drawer_width(ctx: &egui::Context) -> f32 {
    let screen = ctx.input(|i| i.screen_rect());
    let inset = screen.shrink2(Vec2::splat(DRAWER_OFFSET));
    inset.width().clamp(DRAWER_WIDTH, 720.0)
}

pub fn draw_history_drawer(
    ctx: &egui::Context,
    state: &mut HistoryState,
    paths: &AppPaths,
    config: &mut AppConfig,
) -> HistoryResult {
    if !state.open {
        state.open_last_frame = false;
        return HistoryResult::None;
    }

    if !state.open_last_frame {
        state.reload(paths);
    }

    let mut result = HistoryResult::None;
    let allow_backdrop_dismiss = state.open_last_frame;
    let width = history_drawer_width(ctx);
    let geom = side_drawer_geometry(ctx, width);

    paint_side_drawer_backdrop(ctx, "history_backdrop", geom.backdrop_rect);
    let delete_modal_open = state.pending_delete_id.is_some();
    if allow_backdrop_dismiss
        && !delete_modal_open
        && backdrop_dismiss_clicked(ctx, geom.backdrop_rect, geom.drawer_rect, true)
    {
        state.close();
        result = HistoryResult::Closed;
    }

    egui::Area::new(Id::new("history_drawer"))
        .order(egui::Order::Foreground)
        .fixed_pos(geom.area_rect.min)
        .show(ctx, |ui| {
            ui.set_min_size(geom.area_rect.size());
            ui.set_max_size(geom.area_rect.size());

            drawer_frame()
                .outer_margin(geom.shadow_margin)
                .shadow(DRAWER_SHADOW)
                .show(ui, |ui| {
                    ui.set_width(geom.drawer_rect.width());
                    ui.set_height(geom.drawer_rect.height());

                    ui.vertical(|ui| {
                        if draw_drawer_header(ui, Icon::History, "系统 Hosts 历史", "history_close") {
                            state.close();
                            result = HistoryResult::Closed;
                        }

                        let body_h = ui.available_height() - DRAWER_FOOTER_HEIGHT;
                        let body_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(geom.drawer_rect.width(), body_h.max(0.0)),
                        );
                        ui.painter()
                            .rect_filled(body_rect, 0.0, Color32::WHITE);
                        ui.allocate_new_ui(
                            egui::UiBuilder::new().max_rect(body_rect),
                            |ui| {
                                draw_history_body(
                                    ui,
                                    geom.drawer_rect.width(),
                                    body_h.max(0.0),
                                    state,
                                );
                            },
                        );

                        match draw_footer(ui, state, config) {
                            FooterAction::Close => result = HistoryResult::Closed,
                            FooterAction::ConfigChanged => {
                                result = HistoryResult::ConfigChanged
                            }
                            FooterAction::None => {}
                        }
                    });
                });
        });

    if let Some(delete_id) = state.pending_delete_id.clone() {
        match draw_confirm_modal(
            ctx,
            "history_delete_confirm",
            "删除",
            "确实要删除该项记录吗？",
            "删除",
            true,
        ) {
            ConfirmModalResult::Confirmed => {
                match delete_by_id(&paths.histories_dir, &delete_id) {
                    Ok(true) => {
                        let idx = state
                            .items
                            .iter()
                            .rev()
                            .position(|i| i.id == delete_id)
                            .unwrap_or(0);
                        state.selected_id = None;
                        state.reload(paths);
                        let reversed: Vec<_> = state.items.iter().rev().collect();
                        let pick = reversed
                            .get(idx)
                            .or_else(|| reversed.get(idx.saturating_sub(1)));
                        if let Some(item) = pick {
                            state.select(item.id.clone());
                        }
                        state.pending_delete_id = None;
                    }
                    Ok(false) => {
                        tracing::warn!("history delete: id not found: {delete_id}");
                        state.pending_delete_id = None;
                    }
                    Err(err) => {
                        tracing::warn!("history delete failed: {err}");
                        state.pending_delete_id = None;
                    }
                }
            }
            ConfirmModalResult::Cancelled => {
                state.pending_delete_id = None;
            }
            ConfirmModalResult::None => {}
        }
    }

    state.open_last_frame = state.open;
    result
}

/// 对齐 `History.tsx` → `HistoryList`：`Flex h="100%"` 左右同高面板。
fn draw_history_body(ui: &mut Ui, drawer_w: f32, panel_h: f32, state: &mut HistoryState) {
    let inner_w = drawer_w - DRAWER_PAD * 2.0;
    let viewer_w = (inner_w - HISTORY_LIST_WIDTH - HISTORY_PANEL_GAP).max(0.0);

    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
        ui.add_space(DRAWER_PAD);
        ui.allocate_ui_with_layout(
            Vec2::new(viewer_w, panel_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| draw_history_viewer_panel(ui, viewer_w, panel_h, state),
        );
        ui.add_space(HISTORY_PANEL_GAP);
        ui.allocate_ui_with_layout(
            Vec2::new(HISTORY_LIST_WIDTH, panel_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| draw_history_list_panel(ui, HISTORY_LIST_WIDTH, panel_h, state),
        );
        ui.add_space(DRAWER_PAD);
    });
}

/// 对齐 `HostsViewer`：边框内 editor + StatusBar。
fn draw_history_viewer_panel(ui: &mut Ui, width: f32, height: f32, state: &mut HistoryState) {
    egui::Frame::new()
        .stroke(Stroke::new(1.0, SEPARATOR))
        .corner_radius(HISTORY_PANEL_RADIUS)
        .fill(Color32::WHITE)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;

            let origin = ui.cursor().min;
            let editor_h = (height - STATUS_BAR_HEIGHT).max(0.0);
            let editor_rect =
                egui::Rect::from_min_size(origin, Vec2::new(width, editor_h));
            let status_rect = egui::Rect::from_min_size(
                egui::pos2(origin.x, origin.y + editor_h),
                Vec2::new(width, STATUS_BAR_HEIGHT),
            );

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(editor_rect), |ui| {
                if state.items.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("暂无记录")
                                .size(16.0)
                                .color(DRAWER_WEAK_TEXT),
                        );
                    });
                } else {
                    draw_readonly_hosts_viewer(ui, &mut state.viewer_text);
                }
            });

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(status_rect), |ui| {
                let status = viewer_status(&state.viewer_text);
                draw_status_bar(ui, &status);
            });
        });
}

fn viewer_status(text: &str) -> EditorStatus {
    EditorStatus {
        line_count: if text.is_empty() {
            0
        } else {
            text.lines().count().max(1)
        },
        bytes: text.len(),
        read_only: true,
    }
}

fn draw_history_list_panel(ui: &mut Ui, width: f32, height: f32, state: &mut HistoryState) {
    egui::Frame::new()
        .stroke(Stroke::new(1.0, SEPARATOR))
        .corner_radius(HISTORY_PANEL_RADIUS)
        .inner_margin(egui::Margin::same(4))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            let inner_w = (width - 8.0).max(0.0);
            let inner_h = (height - 8.0).max(0.0);

            if state.items.is_empty() {
                ui.allocate_ui_with_layout(
                    Vec2::new(width, height),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(RichText::new("暂无记录").color(DRAWER_WEAK_TEXT));
                        });
                    },
                );
                return;
            }

            ScrollArea::vertical()
                .id_salt("history_list")
                .auto_shrink([false; 2])
                .max_height(inner_h)
                .show(ui, |ui| {
                    ui.set_width(inner_w);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.set_width(inner_w);
                        let items: Vec<_> = state.items.iter().rev().cloned().collect();
                        for item in items {
                            if draw_history_item(
                                ui,
                                inner_w,
                                &item,
                                state.selected_id.as_deref(),
                            ) {
                                state.select(item.id);
                            }
                        }
                    });
                });
        });
}

fn draw_history_item(
    ui: &mut Ui,
    width: f32,
    item: &ApplyHistoryItem,
    selected_id: Option<&str>,
) -> bool {
    let selected = selected_id == Some(item.id.as_str());
    let line_count = item.content.lines().count().max(1);
    let meta = format!(
        "{} lines  {}",
        line_count,
        format_bytes(item.content.len())
    );
    let time = format_timestamp(item.add_time_ms);
    let row_h = HISTORY_ITEM_PAD_Y * 2.0 + 16.0 + 14.0;

    let response = ui.allocate_response(Vec2::new(width, row_h), Sense::click());
    let rect = response.rect;
    if !ui.is_rect_visible(rect) {
        return response.clicked();
    }

    if selected {
        ui.painter()
            .rect_filled(rect, HISTORY_PANEL_RADIUS, TREE_HOVER);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, HISTORY_PANEL_RADIUS, TOP_BAR_ICON_HOVER);
    }

    let text_x = rect.left() + HISTORY_ITEM_PAD_X + 16.0 + HISTORY_ITEM_GAP;
    icons::paint_icon(
        ui,
        Icon::FileText,
        egui::pos2(
            rect.left() + HISTORY_ITEM_PAD_X + 8.0,
            rect.top() + HISTORY_ITEM_PAD_Y + 8.0,
        ),
        16.0,
        Color32::from_rgb(80, 80, 90),
    );

    let time_galley = text_align::layout_vcentered_galley(
        ui,
        time,
        ui_font_id(13.0),
        DRAWER_INPUT_TEXT,
        16.0,
    );
    text_align::paint_galley_row_centered(
        ui,
        text_x,
        rect.top() + HISTORY_ITEM_PAD_Y + 8.0,
        time_galley,
        DRAWER_INPUT_TEXT,
    );

    let meta_galley = text_align::layout_vcentered_galley(
        ui,
        meta,
        ui_font_id(HISTORY_ITEM_META),
        DRAWER_WEAK_TEXT,
        14.0,
    );
    text_align::paint_galley_row_centered(
        ui,
        text_x,
        rect.top() + HISTORY_ITEM_PAD_Y + 24.0,
        meta_galley,
        DRAWER_WEAK_TEXT,
    );

    response.clicked()
}

enum FooterAction {
    None,
    Close,
    ConfigChanged,
}

fn draw_footer(
    ui: &mut Ui,
    state: &mut HistoryState,
    config: &mut AppConfig,
) -> FooterAction {
    let mut action = FooterAction::None;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, DRAWER_FOOTER_HEIGHT), Sense::hover());
    let half = (rect.width() - DRAWER_PAD * 2.0) * 0.5;
    let left = egui::Rect::from_min_size(
        egui::pos2(rect.left() + DRAWER_PAD, rect.top() + 16.0),
        Vec2::new(half, DRAWER_BTN_H),
    );
    let right = egui::Rect::from_min_size(
        egui::pos2(rect.left() + DRAWER_PAD + half, rect.top() + 16.0),
        Vec2::new(half, DRAWER_BTN_H),
    );

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("历史记录上限")
                    .size(14.0)
                    .color(DRAWER_INPUT_TEXT),
            );
            ui.add_space(8.0);

            let mut limits = vec![10_u32, 50, 100, 500];
            if !limits.contains(&config.history_limit) {
                limits.push(config.history_limit);
                limits.sort_unstable();
            }
            drawer_select(
                ui,
                "history_limit",
                HISTORY_LIMIT_SELECT_W,
                &config.history_limit.to_string(),
                |ui| {
                    for limit in limits {
                        if ui
                            .selectable_value(
                                &mut config.history_limit,
                                limit,
                                limit.to_string(),
                            )
                            .changed()
                        {
                            action = FooterAction::ConfigChanged;
                        }
                    }
                },
            );
            ui.add_space(4.0);
            draw_footer_help(ui);
        });
    });

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right), |ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if outline_button(ui, "关闭").clicked() {
                state.close();
                action = FooterAction::Close;
            }
            ui.add_space(12.0);
            let can_delete = state.selected_id.is_some();
            if outline_button_with_icon(ui, Icon::X, "删除", true, can_delete).clicked()
                && can_delete
            {
                if let Some(id) = state.selected_id.clone() {
                    state.pending_delete_id = Some(id);
                }
            }
        });
    });

    action
}

fn draw_footer_help(ui: &mut Ui) {
    let help_rect = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover()).0;
    let help = ui.interact(help_rect, ui.id().with("history_help"), Sense::hover());
    if help.hovered() {
        ui.painter()
            .rect_filled(help_rect, TOP_BAR_ICON_RADIUS, TOP_BAR_ICON_HOVER);
    }
    let help_galley = text_align::layout_vcentered_galley(
        ui,
        "?".to_string(),
        ui_font_id(14.0),
        DRAWER_WEAK_TEXT,
        ICON_ROW_LINE_HEIGHT,
    );
    text_align::paint_galley_row_centered(
        ui,
        help_rect.center().x - help_galley.size().x * 0.5,
        help_rect.center().y,
        help_galley,
        DRAWER_WEAK_TEXT,
    );
    if help.hovered() {
        egui::show_tooltip_at_pointer(
            ui.ctx(),
            ui.layer_id(),
            Id::new("history_help_tip"),
            |ui| {
                ui.set_max_width(260.0);
                ui.label("每次写入系统 Hosts 时保存一份快照；超出上限时删除最旧的记录。");
            },
        );
    }
}

fn format_timestamp(ms: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ms.to_string())
}
