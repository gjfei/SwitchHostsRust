//! 系统 Hosts 历史抽屉（对齐 `SwitchHosts/src/renderer/components/History.tsx`）。

use switch_hosts_core::hosts_apply::{delete_by_id, list_history, ApplyHistoryItem};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::paths::AppPaths;
use eframe::egui::{self, Color32, CornerRadius, Id, RichText, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::panels::drawer::{
    backdrop_dismiss_clicked, drawer_panel_frame, drawer_select, draw_confirm_modal,
    draw_drawer_header, outline_button, outline_button_with_icon, paint_side_drawer_backdrop,
    side_drawer_geometry, ConfirmModalResult, DRAWER_BTN_H,
};
use crate::panels::editor::draw_readonly_hosts_viewer;
use crate::panels::status_bar::{
    draw_panel_status_spacer_with_corners, draw_status_bar_with_corners, EditorStatus,
    pin_body_and_status_bar,
};
use crate::panels::widgets::format_bytes;
use crate::text_align::{self, ICON_ROW_LINE_HEIGHT};
use crate::theme::{self, layout};

const HISTORY_LIST_WIDTH: f32 = 200.0;
/// 对齐 `History.tsx` 面板 `borderRadius: 6`
const HISTORY_PANEL_RADIUS: CornerRadius = CornerRadius::same(6);
/// 预览区主体：仅顶部圆角（底部接 status bar）。
const HISTORY_BODY_RADIUS: CornerRadius = CornerRadius {
    nw: 6,
    ne: 6,
    sw: 0,
    se: 0,
};
/// status bar / 底部占位：仅底部圆角。
const HISTORY_STATUS_RADIUS: CornerRadius = CornerRadius {
    nw: 0,
    ne: 0,
    sw: 6,
    se: 6,
};
/// 对齐 `History.tsx` 预览与列表间距 `marginRight: 12`
const HISTORY_PANEL_GAP: f32 = 12.0;
const HISTORY_ITEM_PAD_X: f32 = 12.0;
const HISTORY_ITEM_PAD_Y: f32 = 8.0;
const HISTORY_ITEM_GAP: f32 = 8.0;
const HISTORY_ITEM_META: f32 = 9.0;
const HISTORY_LIMIT_SELECT_W: f32 = 100.0;
/// 对齐 `History.tsx` 列表 `ScrollArea` `padding: 4`（边框内边距，不计入外框高度）。
const HISTORY_LIST_INNER_PAD: f32 = 4.0;

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
    let screen = ctx.input(|i| i.content_rect());
    let inset = screen.shrink2(Vec2::splat(layout::DRAWER_OFFSET));
    inset.width().clamp(layout::DRAWER_WIDTH, 720.0)
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

            drawer_panel_frame(ctx)
                .outer_margin(geom.shadow_margin)
                .show(ui, |ui| {
                    ui.set_width(geom.drawer_rect.width());
                    ui.set_height(geom.drawer_rect.height());

                    ui.vertical(|ui| {
                        if draw_drawer_header(ui, Icon::History, "系统 Hosts 历史", "history_close") {
                            state.close();
                            result = HistoryResult::Closed;
                        }

                        let body_h = ui.available_height() - layout::DRAWER_FOOTER_HEIGHT;
                        let body_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(geom.drawer_rect.width(), body_h.max(0.0)),
                        );
                        ui.painter()
                            .rect_filled(body_rect, 0.0, theme::app(ctx).editor_bg);
                        ui.scope_builder(
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

/// 对齐 `SideDrawer` footer `padding: md`：主体与 footer 之间留白。
const BODY_BOTTOM_PAD: f32 = layout::DRAWER_PAD;

/// 对齐 `History.tsx` → `HistoryList`：`Flex h="100%"` 左右同高面板。
fn draw_history_body(ui: &mut Ui, drawer_w: f32, panel_h: f32, state: &mut HistoryState) {
    let content_h = (panel_h - BODY_BOTTOM_PAD).max(0.0);
    let inner_w = drawer_w - layout::DRAWER_PAD * 2.0;
    let viewer_w = (inner_w - HISTORY_LIST_WIDTH - HISTORY_PANEL_GAP).max(0.0);

    ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
        ui.add_space(layout::DRAWER_PAD);
        let (viewer_rect, _) =
            ui.allocate_exact_size(Vec2::new(viewer_w, content_h), Sense::hover());
        ui.scope_builder(egui::UiBuilder::new().max_rect(viewer_rect), |ui| {
            draw_history_viewer_panel(ui, viewer_w, content_h, state);
        });
        ui.add_space(HISTORY_PANEL_GAP);
        let (list_rect, _) =
            ui.allocate_exact_size(Vec2::new(HISTORY_LIST_WIDTH, content_h), Sense::hover());
        ui.scope_builder(egui::UiBuilder::new().max_rect(list_rect), |ui| {
            draw_history_list_panel(ui, HISTORY_LIST_WIDTH, content_h, state);
        });
        ui.add_space(layout::DRAWER_PAD);
    });
}

/// 对齐 `HostsViewer`：边框内 editor + StatusBar（`overflow: hidden` + 分区圆角）。
fn draw_history_viewer_panel(ui: &mut Ui, width: f32, height: f32, state: &mut HistoryState) {
    let t = theme::app(ui.ctx());
    egui::Frame::new()
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, t.separator))
        .corner_radius(HISTORY_PANEL_RADIUS)
        .inner_margin(egui::Margin::ZERO)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(width, height));
            ui.set_max_size(Vec2::new(width, height));

            let status = viewer_status(&state.viewer_text);
            let readonly_bg = t.editor_readonly_bg;

            pin_body_and_status_bar(
                ui,
                |ui| {
                    let body = ui.max_rect();
                    ui.painter()
                        .rect_filled(body, HISTORY_BODY_RADIUS, readonly_bg);
                    if state.items.is_empty() {
                        ui.scope_builder(
                            egui::UiBuilder::new().max_rect(body),
                            |ui| {
                                ui.centered_and_justified(|ui| {
                                    ui.label(
                                        RichText::new("暂无记录")
                                            .size(16.0)
                                            .color(t.weak_text),
                                    );
                                });
                            },
                        );
                    } else {
                        ui.scope_builder(
                            egui::UiBuilder::new().max_rect(body),
                            |ui| {
                                draw_readonly_hosts_viewer(ui, &mut state.viewer_text, false);
                            },
                        );
                    }
                },
                |ui| draw_status_bar_with_corners(ui, &status, HISTORY_STATUS_RADIUS),
            );
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
    let t = theme::app(ui.ctx());
    egui::Frame::new()
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0, t.separator))
        .corner_radius(HISTORY_PANEL_RADIUS)
        .inner_margin(egui::Margin::ZERO)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(width, height));
            ui.set_max_size(Vec2::new(width, height));

            pin_body_and_status_bar(
                ui,
                |ui| {
                    let body = ui.max_rect();
                    ui.painter()
                        .rect_filled(body, HISTORY_BODY_RADIUS, t.editor_bg);

                    let pad = HISTORY_LIST_INNER_PAD;
                    let inner_rect = body.shrink2(Vec2::splat(pad));

                    ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
                        if state.items.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new("暂无记录").color(t.weak_text));
                            });
                            return;
                        }

                        ScrollArea::vertical()
                            .id_salt("history_list")
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                ui.set_width(inner_rect.width());
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.set_width(inner_rect.width());
                                    let items: Vec<_> = state.items.iter().rev().cloned().collect();
                                    for item in items {
                                        if draw_history_item(
                                            ui,
                                            inner_rect.width(),
                                            &item,
                                            state.selected_id.as_deref(),
                                        ) {
                                            state.select(item.id);
                                        }
                                    }
                                    ui.add_space(4.0);
                                });
                            });
                    });
                },
                |ui| draw_panel_status_spacer_with_corners(ui, HISTORY_STATUS_RADIUS),
            );
        });
}

fn draw_history_item(
    ui: &mut Ui,
    width: f32,
    item: &ApplyHistoryItem,
    selected_id: Option<&str>,
) -> bool {
    let t = theme::app(ui.ctx());
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
            .rect_filled(rect, HISTORY_PANEL_RADIUS, t.tree_hover);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, HISTORY_PANEL_RADIUS, t.icon_hover_bg);
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
        t.text,
        16.0,
    );
    text_align::paint_galley_row_centered(
        ui,
        text_x,
        rect.top() + HISTORY_ITEM_PAD_Y + 8.0,
        time_galley,
        t.text,
    );

    let meta_galley = text_align::layout_vcentered_galley(
        ui,
        meta,
        ui_font_id(HISTORY_ITEM_META),
        t.weak_text,
        14.0,
    );
    text_align::paint_galley_row_centered(
        ui,
        text_x,
        rect.top() + HISTORY_ITEM_PAD_Y + 24.0,
        meta_galley,
        t.weak_text,
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
    let t = theme::app(ui.ctx());
    let mut action = FooterAction::None;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, layout::DRAWER_FOOTER_HEIGHT), Sense::hover());

    let row_top = rect.top() + (layout::DRAWER_FOOTER_HEIGHT - DRAWER_BTN_H) * 0.5;
    let half = (rect.width() - layout::DRAWER_PAD * 2.0) * 0.5;
    let left = egui::Rect::from_min_size(
        egui::pos2(rect.left() + layout::DRAWER_PAD, row_top),
        Vec2::new(half, DRAWER_BTN_H),
    );
    let right = egui::Rect::from_min_size(
        egui::pos2(rect.left() + layout::DRAWER_PAD + half, row_top),
        Vec2::new(half, DRAWER_BTN_H),
    );

    ui.scope_builder(egui::UiBuilder::new().max_rect(left), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("历史记录上限")
                    .size(14.0)
                    .color(t.text),
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

    ui.scope_builder(egui::UiBuilder::new().max_rect(right), |ui| {
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
    let t = theme::app(ui.ctx());
    let help_rect = ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover()).0;
    let help = ui.interact(help_rect, ui.id().with("history_help"), Sense::hover());
    if help.hovered() {
        ui.painter()
            .rect_filled(help_rect, t.corner_icon(), t.icon_hover_bg);
    }
    let help_galley = text_align::layout_vcentered_galley(
        ui,
        "?".to_string(),
        ui_font_id(14.0),
        t.weak_text,
        ICON_ROW_LINE_HEIGHT,
    );
    text_align::paint_galley_row_centered(
        ui,
        help_rect.center().x - help_galley.size().x * 0.5,
        help_rect.center().y,
        help_galley,
        t.weak_text,
    );
    help.on_hover_ui(|ui| {
        ui.set_max_width(260.0);
        ui.label("每次写入系统 Hosts 时保存一份快照；超出上限时删除最旧的记录。");
    });
}

fn format_timestamp(ms: i64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_else(|| ms.to_string())
}
