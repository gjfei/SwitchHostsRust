//! 查找 / 替换抽屉（对齐 `SwitchHosts/src/renderer/pages/find.tsx` + 共享 SideDrawer 壳层）。

use switch_hosts_core::find::{
    adjusted_replace_range, byte_to_char_index, find_in_manifest, flatten_find_items,
    replace_all_in_manifest, replace_one, FindMatchRow, FindSearchOptions, ReplaceOneArgs,
};
use switch_hosts_core::storage::config::AppConfig;
use switch_hosts_core::storage::entries;
use switch_hosts_core::storage::manifest::Manifest;
use switch_hosts_core::storage::paths::AppPaths;
use eframe::egui::{self, Color32, CornerRadius, FontId, Id, Sense, Stroke, Ui, Vec2};

use crate::fonts::ui_font_id;
use crate::icons::{self, Icon};
use crate::panels::drawer::{
    backdrop_dismiss_clicked, drawer_panel_frame, drawer_text_button, draw_drawer_header,
    paint_side_drawer_backdrop, side_drawer_geometry,
};
use crate::panels::widgets::ellipsize_text;
use crate::text_align;
use crate::theme::{self, layout};

const DEBOUNCE_SECS: f64 = 0.5;
const INPUT_ROW_H: f32 = 36.0;
const INPUT_PAD_X: f32 = layout::DRAWER_PAD;
const INPUT_PAD_RIGHT: f32 = 12.0;
const CHECKBOX_ROW_H: f32 = 36.0;
const RESULT_ROW_HEIGHT: f32 = 29.0;
const RESULT_LIST_PAD_Y: f32 = 5.0;
const RESULT_ROW_INSET: f32 = 8.0;
const RESULT_ROW_PAD_LEFT: f32 = 8.0;
const RESULT_COL_MIN: f32 = 60.0;
const RESULT_COL_GAP: f32 = 4.0;
const RESULT_SCROLLBAR_OFFSET: f32 = 12.0;
const NAV_BTN: f32 = 36.0;
const FIND_STATUS_LINE_HEIGHT: f32 = 12.0;

#[derive(Debug, Default)]
pub struct FindReplaceState {
    pub open: bool,
    open_last_frame: bool,
    pub query: String,
    pub replace_with: String,
    pub rows: Vec<FindMatchRow>,
    pub current_idx: Option<usize>,
    debounced_query: String,
    debounced_options: FindSearchOptions,
    query_changed_at: Option<f64>,
    search_seq: u64,
    applied_search_seq: u64,
    pub error: Option<String>,
    is_replacing: bool,
    scroll_to_current: bool,
    query_focused: bool,
}

#[derive(Debug, Default)]
pub enum FindReplaceAction {
    #[default]
    None,
    ContentChanged(Vec<String>),
    JumpToMatch {
        entry_id: String,
        start_char: usize,
        end_char: usize,
    },
}

impl FindReplaceState {
    pub fn open_drawer(&mut self) {
        self.open = true;
        self.scroll_to_current = true;
        self.query_focused = true;
    }

    fn close(&mut self) {
        self.open = false;
        self.rows.clear();
        self.rows.shrink_to_fit();
        self.error = None;
        self.is_replacing = false;
    }
}

fn find_drawer_width(ctx: &egui::Context) -> f32 {
    let screen = ctx.input(|i| i.screen_rect());
    let inset = screen.shrink2(Vec2::splat(layout::DRAWER_OFFSET));
    inset.width().clamp(layout::DRAWER_WIDTH, 720.0)
}

pub fn draw_find_replace_drawer(
    ctx: &egui::Context,
    state: &mut FindReplaceState,
    config: &mut AppConfig,
    manifest: &Manifest,
    paths: &AppPaths,
) -> Option<FindReplaceAction> {
    if !state.open {
        state.open_last_frame = false;
        return None;
    }

    if !state.open_last_frame {
        state.query_focused = true;
    }

    let mut action = None;
    let options = FindSearchOptions {
        is_regexp: config.find_is_regexp,
        ignore_case: config.find_is_ignore_case,
    };

    tick_debounced_search(ctx, state, manifest, paths, &options);

    let options_stale = state.debounced_options.is_regexp != options.is_regexp
        || state.debounced_options.ignore_case != options.ignore_case;
    let pending_search = state.query != state.debounced_query || options_stale;
    let results_current = state.applied_search_seq == state.search_seq
        && state.debounced_query == state.query
        && !options_stale;
    let action_busy = state.is_replacing || pending_search;
    let can_replace_all = state
        .rows
        .iter()
        .any(|r| !r.is_readonly && !r.is_disabled);

    let allow_backdrop_dismiss = state.open_last_frame;
    let width = find_drawer_width(ctx);
    let geom = side_drawer_geometry(ctx, width);

    paint_side_drawer_backdrop(ctx, "find_backdrop", geom.backdrop_rect);
    if allow_backdrop_dismiss
        && backdrop_dismiss_clicked(ctx, geom.backdrop_rect, geom.drawer_rect, true)
    {
        state.close();
        state.open_last_frame = false;
        return None;
    }

    egui::Area::new(Id::new("find_drawer"))
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
                        if draw_drawer_header(ui, Icon::Search, "查找并替换", "find_close") {
                            state.close();
                        }

                        let body_h = ui.available_height() - layout::DRAWER_FOOTER_HEIGHT;
                        let body_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(geom.drawer_rect.width(), body_h.max(0.0)),
                        );
                        ui.painter().rect_filled(body_rect, 0.0, theme::app(ctx).editor_bg);
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            draw_find_body(
                                ui,
                                ctx,
                                state,
                                config,
                                paths,
                                state.is_replacing,
                                action_busy,
                                &mut action,
                                results_current,
                                pending_search,
                                can_replace_all,
                                manifest,
                                &options,
                            );
                        });

                        let footer_rect = egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(geom.drawer_rect.width(), layout::DRAWER_FOOTER_HEIGHT),
                        );
                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(footer_rect), |ui| {
                            draw_find_footer(
                                ui,
                                state,
                                results_current,
                                pending_search,
                                can_replace_all,
                                action_busy,
                                &mut action,
                                manifest,
                                paths,
                                config,
                                &options,
                            );
                        });
                    });
                });
        });

    state.open_last_frame = state.open;
    action
}

fn draw_find_body(
    ui: &mut Ui,
    ctx: &egui::Context,
    state: &mut FindReplaceState,
    config: &mut AppConfig,
    paths: &AppPaths,
    input_disabled: bool,
    action_busy: bool,
    action: &mut Option<FindReplaceAction>,
    _results_current: bool,
    _pending_search: bool,
    _can_replace_all: bool,
    _manifest: &Manifest,
    _options: &FindSearchOptions,
) {
    let flushed_id = ui.id().with("find_flushed");
    draw_flushed_input(
        ui,
        flushed_id.with("kw"),
        "keywords",
        &mut state.query,
        input_disabled,
        state.query_focused,
        || state.query_changed_at = Some(ctx.input(|i| i.time)),
    );
    if state.query_focused {
        state.query_focused = false;
    }
    draw_flushed_input(
        ui,
        flushed_id.with("replace"),
        "replace to",
        &mut state.replace_with,
        input_disabled,
        false,
        || {},
    );

    draw_checkbox_row(ui, config, state, action_busy, ctx);

    let t = theme::app(ui.ctx());
    let list_h = ui.available_height().max(80.0);
    let col_widths = result_column_widths(ui, config);
    egui::Frame::new()
        .fill(t.editor_bg)
        .inner_margin(0.0)
        .show(ui, |ui| {
            ui.set_min_height(list_h);
            draw_result_header(ui, col_widths);
            draw_result_list(
                ui,
                state,
                col_widths,
                ui.available_height().max(60.0),
                action,
                action_busy,
                paths,
            );
        });
}

fn draw_find_footer(
    ui: &mut Ui,
    state: &mut FindReplaceState,
    results_current: bool,
    pending_search: bool,
    can_replace_all: bool,
    busy: bool,
    action: &mut Option<FindReplaceAction>,
    manifest: &Manifest,
    paths: &AppPaths,
    config: &AppConfig,
    options: &FindSearchOptions,
) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, layout::DRAWER_FOOTER_HEIGHT), Sense::hover());

    ui.allocate_new_ui(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.set_width(rect.width());
            draw_status_bar(
                ui,
                state,
                results_current,
                pending_search,
                can_replace_all,
                busy,
                action,
                manifest,
                paths,
                config,
                options,
            );
        },
    );
}

fn tick_debounced_search(
    ctx: &egui::Context,
    state: &mut FindReplaceState,
    manifest: &Manifest,
    paths: &AppPaths,
    options: &FindSearchOptions,
) {
    let now = ctx.input(|i| i.time);
    let options_changed = state.debounced_options.is_regexp != options.is_regexp
        || state.debounced_options.ignore_case != options.ignore_case;

    if state.query != state.debounced_query || options_changed {
        if state.query_changed_at.is_none() {
            state.query_changed_at = Some(now);
        }
        let elapsed = now - state.query_changed_at.unwrap_or(now);
        if elapsed >= DEBOUNCE_SECS {
            let keyword = state.query.clone();
            run_search(state, manifest, paths, &keyword, options);
            state.debounced_query = keyword;
            state.debounced_options = options.clone();
            state.query_changed_at = None;
        } else {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                DEBOUNCE_SECS - elapsed,
            ));
        }
    } else {
        state.query_changed_at = None;
    }
}

fn run_search(
    state: &mut FindReplaceState,
    manifest: &Manifest,
    paths: &AppPaths,
    keyword: &str,
    options: &FindSearchOptions,
) {
    state.search_seq = state.search_seq.wrapping_add(1);
    let seq = state.search_seq;
    state.error = None;

    if keyword.is_empty() {
        state.rows.clear();
        state.current_idx = None;
        state.applied_search_seq = seq;
        return;
    }

    match find_in_manifest(manifest, paths, keyword, options) {
        Ok(items) => {
            if state.search_seq != seq {
                return;
            }
            state.rows = flatten_find_items(&items);
            state.current_idx = if state.rows.is_empty() {
                None
            } else {
                Some(0)
            };
            state.scroll_to_current = true;
            state.applied_search_seq = seq;
        }
        Err(err) => {
            if state.search_seq != seq {
                return;
            }
            state.rows.clear();
            state.current_idx = None;
            state.error = Some(err.to_string());
            state.applied_search_seq = seq;
        }
    }
}

fn draw_flushed_input(
    ui: &mut Ui,
    id: egui::Id,
    placeholder: &str,
    value: &mut String,
    disabled: bool,
    request_focus: bool,
    on_change: impl FnOnce(),
) {
    let t = theme::app(ui.ctx());
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, INPUT_ROW_H), Sense::hover());

    let input_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + INPUT_PAD_X, rect.top()),
        egui::pos2(rect.right() - INPUT_PAD_RIGHT, rect.bottom()),
    );
    ui.painter()
        .hline(rect.x_range(), rect.bottom(), Stroke::new(1.0, t.separator));

    let mut edit = egui::TextEdit::singleline(value)
        .id(id)
        .hint_text(placeholder)
        .frame(false)
        .margin(egui::Margin::ZERO);
    if disabled {
        edit = edit.interactive(false);
    }
    if request_focus {
        ui.memory_mut(|m| m.request_focus(id));
    }

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(input_rect), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_height(input_rect.height());
            if ui.add(edit).changed() {
                on_change();
            }
        });
    });
}

fn draw_checkbox_row(
    ui: &mut Ui,
    config: &mut AppConfig,
    state: &mut FindReplaceState,
    busy: bool,
    ctx: &egui::Context,
) {
    ui.allocate_ui_with_layout(
        Vec2::new(ui.available_width(), CHECKBOX_ROW_H),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.add_space(layout::DRAWER_PAD);
            let mut is_regexp = config.find_is_regexp;
            let mut ignore_case = config.find_is_ignore_case;
            ui.spacing_mut().item_spacing.x = 16.0;
            if ui
                .add_enabled(!busy, egui::Checkbox::new(&mut is_regexp, "正则表达式"))
                .changed()
            {
                config.find_is_regexp = is_regexp;
                state.query_changed_at = Some(ctx.input(|i| i.time));
            }
            if ui
                .add_enabled(!busy, egui::Checkbox::new(&mut ignore_case, "忽略大小写"))
                .changed()
            {
                config.find_is_ignore_case = ignore_case;
                state.query_changed_at = Some(ctx.input(|i| i.time));
            }
        },
    );
}

fn result_column_widths(ui: &Ui, config: &AppConfig) -> [f32; 3] {
    let track = (ui.available_width()
        - RESULT_ROW_INSET * 2.0
        - RESULT_SCROLLBAR_OFFSET
        - RESULT_ROW_PAD_LEFT)
        .max(RESULT_COL_MIN * 3.0);
    if config.find_result_column_widths.len() == 3 {
        let mut w = [
            config.find_result_column_widths[0] as f32,
            config.find_result_column_widths[1] as f32,
            config.find_result_column_widths[2] as f32,
        ];
        for width in &mut w {
            *width = width.max(RESULT_COL_MIN);
        }
        w[2] = (track - w[0] - w[1] - RESULT_COL_GAP * 2.0).max(RESULT_COL_MIN);
        return w;
    }
    let title = (track * 0.2).max(RESULT_COL_MIN);
    let line = RESULT_COL_MIN;
    let m = (track - title - line - RESULT_COL_GAP * 2.0).max(RESULT_COL_MIN);
    [m, title, line]
}

fn draw_result_header(ui: &mut Ui, widths: [f32; 3]) {
    let t = theme::app(ui.ctx());
    let h = RESULT_ROW_HEIGHT;
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    ui.painter()
        .hline(rect.x_range(), rect.top(), Stroke::new(1.0, t.separator));

    let mut x = rect.left() + RESULT_ROW_INSET + RESULT_ROW_PAD_LEFT;
    for (label, col_w) in [("匹配", widths[0]), ("标题", widths[1]), ("行", widths[2])] {
        ui.painter().text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            ui_font_id(layout::TREE_FONT_SIZE),
            t.weak_text,
        );
        x += col_w + RESULT_COL_GAP;
    }
}

fn draw_result_list(
    ui: &mut Ui,
    state: &mut FindReplaceState,
    widths: [f32; 3],
    list_h: f32,
    action: &mut Option<FindReplaceAction>,
    busy: bool,
    paths: &AppPaths,
) {
    let row_outer_w =
        RESULT_ROW_PAD_LEFT + widths[0] + widths[1] + widths[2] + RESULT_COL_GAP * 2.0;

    let scroll_to = if state.scroll_to_current {
        state.scroll_to_current = false;
        state.current_idx
    } else {
        None
    };

    egui::ScrollArea::both()
        .max_height(list_h)
        .auto_shrink([false; 2])
        .show(ui, |ui| {
            ui.add_space(RESULT_LIST_PAD_Y);
            ui.set_min_width(row_outer_w + RESULT_ROW_INSET * 2.0);
            for (idx, row) in state.rows.iter().enumerate() {
                if scroll_to == Some(idx) {
                    ui.scroll_to_rect(
                        egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(row_outer_w, RESULT_ROW_HEIGHT),
                        ),
                        Some(egui::Align::Center),
                    );
                }
                ui.horizontal(|ui| {
                    ui.add_space(RESULT_ROW_INSET);
                    let row_response = draw_result_row(
                        ui,
                        row,
                        widths,
                        row_outer_w,
                        state.current_idx == Some(idx),
                        busy,
                    );
                    if row_response.clicked() && !busy {
                        state.current_idx = Some(idx);
                    }
                    if row_response.double_clicked() && !busy {
                        if let Some(jump) = jump_action_for_row(paths, row) {
                            *action = Some(jump);
                        }
                    }
                });
            }
            ui.add_space(RESULT_LIST_PAD_Y);
        });
}

fn draw_result_row(
    ui: &mut Ui,
    row: &FindMatchRow,
    widths: [f32; 3],
    outer_w: f32,
    selected: bool,
    busy: bool,
) -> egui::Response {
    let t = theme::app(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(outer_w, RESULT_ROW_HEIGHT), Sense::click());

    if ui.is_rect_visible(rect) {
        let radius = t.corner_panel();
        if selected {
            ui.painter().rect_filled(rect, radius, t.accent);
        } else if response.hovered() && !busy {
            ui.painter().rect_filled(rect, radius, t.icon_hover_bg);
        }
        if !selected {
            ui.painter().hline(
                rect.x_range(),
                rect.bottom(),
                Stroke::new(1.0, t.separator),
            );
        }

        let text_color = if selected {
            t.text_selected
        } else if row.is_disabled || row.is_readonly {
            t.weak_text
        } else {
            t.text
        };
        let line_color = if selected {
            t.text_selected
        } else {
            t.weak_text
        };

        let mut x = rect.left() + RESULT_ROW_PAD_LEFT;
        let cy = rect.center().y;

        paint_match_cell(
            ui,
            egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + widths[0], rect.bottom()),
            ),
            row,
            selected,
            text_color,
            &t,
        );
        x += widths[0] + RESULT_COL_GAP;

        let icon = match row.item_type.as_str() {
            "remote" => Icon::World,
            _ => Icon::FileText,
        };
        icons::paint_icon(ui, icon, egui::pos2(x + 8.0, cy), 16.0, text_color);
        let title =
            ellipsize_text(ui, &row.item_title, ui_font_id(layout::TREE_FONT_SIZE), widths[1] - 24.0);
        ui.painter().text(
            egui::pos2(x + 20.0, cy),
            egui::Align2::LEFT_CENTER,
            title,
            ui_font_id(layout::TREE_FONT_SIZE),
            text_color,
        );
        x += widths[1] + RESULT_COL_GAP;

        ui.painter().text(
            egui::pos2(x, cy),
            egui::Align2::LEFT_CENTER,
            row.line.to_string(),
            ui_font_id(layout::TREE_FONT_SIZE),
            line_color,
        );

        if row.is_disabled {
            ui.painter().line_segment(
                [egui::pos2(rect.left(), cy), egui::pos2(rect.right(), cy)],
                Stroke::new(1.0, text_color),
            );
        }
    }
    response
}

fn paint_match_cell(
    ui: &Ui,
    rect: egui::Rect,
    row: &FindMatchRow,
    selected: bool,
    text_color: Color32,
    t: &theme::AppTheme,
) {
    let mono = FontId::monospace(layout::TREE_FONT_SIZE);
    let painter = ui.painter();
    let mut x = rect.left();
    let cy = rect.center().y;
    let max_x = rect.right();

    if row.is_readonly {
        let tag = "只读";
        painter.text(
            egui::pos2(x, cy),
            egui::Align2::LEFT_CENTER,
            tag,
            ui_font_id(10.0),
            if selected {
                t.text_selected
            } else {
                t.weak_text
            },
        );
        x += ui
            .fonts(|f| f.layout_no_wrap(tag.to_owned(), ui_font_id(10.0), text_color))
            .size()
            .x
            + 8.0;
    }

    for (text, is_match) in [
        (row.before.as_str(), false),
        (row.match_text.as_str(), true),
        (row.after.as_str(), false),
    ] {
        if text.is_empty() {
            continue;
        }
        // 对齐 `.highlight { color: var(--swh-font-color) }`：选中行匹配词仍用深色字 + 黄底。
        let segment_color = if is_match && selected {
            t.text
        } else {
            text_color
        };
        let galley = ui.fonts(|f| {
            f.layout_no_wrap(text.to_owned(), mono.clone(), segment_color)
        });
        let gw = galley.size().x;
        if x + gw > max_x {
            let room = (max_x - x).max(0.0);
            if room <= 0.0 {
                break;
            }
            let clipped = ellipsize_text(ui, text, mono.clone(), room);
            let cg = ui
                .fonts(|f| f.layout_no_wrap(clipped, mono.clone(), segment_color));
            if is_match {
                let r = egui::Rect::from_min_size(
                    egui::pos2(x, cy - layout::TREE_FONT_SIZE * 0.55),
                    Vec2::new(cg.size().x, layout::TREE_FONT_SIZE * 1.1),
                );
                painter.rect_filled(r, 0.0, t.find_highlight_bg);
            }
            painter.galley(egui::pos2(x, cy - cg.size().y * 0.5), cg, segment_color);
            break;
        }
        if is_match {
            let r = egui::Rect::from_min_size(
                egui::pos2(x, cy - layout::TREE_FONT_SIZE * 0.55),
                Vec2::new(gw, layout::TREE_FONT_SIZE * 1.1),
            );
            painter.rect_filled(r, 0.0, t.find_highlight_bg);
        }
        painter.galley(
            egui::pos2(x, cy - galley.size().y * 0.5),
            galley,
            segment_color,
        );
        x += gw;
    }
}

fn draw_status_bar(
    ui: &mut Ui,
    state: &mut FindReplaceState,
    results_current: bool,
    pending_search: bool,
    can_replace_all: bool,
    busy: bool,
    action: &mut Option<FindReplaceAction>,
    manifest: &Manifest,
    paths: &AppPaths,
    config: &AppConfig,
    options: &FindSearchOptions,
) {
    let t = theme::app(ui.ctx());
    ui.set_width(ui.max_rect().width());
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.add_space(layout::DRAWER_PAD);

        if busy && pending_search {
            ui.allocate_ui_with_layout(
                Vec2::new(16.0, NAV_BTN),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add(egui::Spinner::new().size(16.0));
                },
            );
            ui.add_space(8.0);
        }

        let count = state.rows.len();
        let (status, color) = if let Some(err) = &state.error {
            (err.clone(), t.find_error)
        } else if pending_search {
            ("搜索中…".to_string(), t.weak_text)
        } else if count == 1 {
            ("1 项匹配".to_string(), t.weak_text)
        } else {
            (format!("{count} 项匹配"), t.weak_text)
        };
        let status_galley = text_align::layout_vcentered_galley(
            ui,
            status,
            ui_font_id(12.0),
            color,
            FIND_STATUS_LINE_HEIGHT,
        );
        let status_w = status_galley.size().x;
        let (status_rect, _) =
            ui.allocate_exact_size(Vec2::new(status_w, NAV_BTN), Sense::hover());
        text_align::paint_galley_row_centered(
            ui,
            status_rect.left(),
            status_rect.center().y,
            status_galley,
            color,
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(layout::DRAWER_PAD);
            let action_disabled = busy || !results_current || state.is_replacing;
            let has_rows = !state.rows.is_empty();
            let cur = state.current_idx.unwrap_or(0);
            let next_enabled = !action_disabled && has_rows && cur + 1 < state.rows.len();
            let prev_enabled = !action_disabled && has_rows && cur > 0;

            let can_replace_one = state
                .current_idx
                .and_then(|i| state.rows.get(i))
                .is_some_and(|r| !r.is_readonly && !r.is_disabled);

            if drawer_text_button(
                ui,
                "替换",
                t.accent,
                Stroke::NONE,
                t.text_selected,
                can_replace_one && !action_disabled,
            )
            .clicked()
            {
                if let Some(ids) = do_replace_one(state, manifest, paths, options) {
                    *action = Some(FindReplaceAction::ContentChanged(ids));
                    let _ = config.save(&paths.config_file);
                }
            }
            ui.add_space(8.0);
            if drawer_text_button(
                ui,
                "替换所有",
                t.editor_bg,
                Stroke::new(1.0, t.input_border),
                t.text,
                can_replace_all && !action_disabled,
            )
            .clicked()
            {
                let keyword = state.debounced_query.clone();
                if let Some(ids) = do_replace_all(state, manifest, paths, &keyword, options) {
                    *action = Some(FindReplaceAction::ContentChanged(ids));
                    let _ = config.save(&paths.config_file);
                }
            }

            ui.add_space(12.0);

            ui.allocate_ui_with_layout(
                Vec2::new(NAV_BTN * 2.0, NAV_BTN),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    if nav_btn(ui, Icon::ArrowLeft, prev_enabled).clicked() && prev_enabled {
                        state.current_idx = Some(cur.saturating_sub(1));
                        state.scroll_to_current = true;
                    }
                    if nav_btn(ui, Icon::ArrowRight, next_enabled).clicked() && next_enabled {
                        state.current_idx = Some((cur + 1).min(state.rows.len() - 1));
                        state.scroll_to_current = true;
                    }
                },
            );
        });
    });
}

fn nav_btn(ui: &mut Ui, icon: Icon, enabled: bool) -> egui::Response {
    let t = theme::app(ui.ctx());
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(NAV_BTN), Sense::click());
    if !enabled {
        response = ui.interact(rect, response.id, Sense::hover());
    }
    let stroke_color = if enabled {
        t.input_border
    } else {
        Color32::from_rgb(230, 230, 235)
    };
    let icon_color = if enabled {
        if response.hovered() {
            t.text
        } else {
            t.nav_icon_inactive_tint
        }
    } else {
        Color32::from_rgb(180, 180, 190)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        t.editor_bg,
        Stroke::new(1.0, stroke_color),
        egui::StrokeKind::Inside,
    );
    icons::paint_icon(ui, icon, rect.center(), 18.0, icon_color);
    response
}

fn do_replace_all(
    state: &mut FindReplaceState,
    manifest: &Manifest,
    paths: &AppPaths,
    keyword: &str,
    options: &FindSearchOptions,
) -> Option<Vec<String>> {
    state.is_replacing = true;
    let result = replace_all_in_manifest(manifest, paths, keyword, options, &state.replace_with);
    state.is_replacing = false;

    match result {
        Ok(outcome) if outcome.replaced_count > 0 => {
            let changed: std::collections::HashSet<_> =
                outcome.item_ids.iter().cloned().collect();
            for row in &mut state.rows {
                if !row.is_readonly && changed.contains(&row.item_id) {
                    row.is_disabled = true;
                    row.replace_to = Some(state.replace_with.clone());
                }
            }
            Some(outcome.item_ids)
        }
        Ok(_) => {
            state.error = Some("搜索结果已过期，请重新搜索。".into());
            run_search(state, manifest, paths, keyword, options);
            None
        }
        Err(err) => {
            state.error = Some(err.to_string());
            None
        }
    }
}

fn do_replace_one(
    state: &mut FindReplaceState,
    manifest: &Manifest,
    paths: &AppPaths,
    options: &FindSearchOptions,
) -> Option<Vec<String>> {
    let idx = state.current_idx?;
    let row = state.rows.get(idx)?.clone();
    if row.is_readonly || row.is_disabled {
        return None;
    }
    let (start, end) = adjusted_replace_range(&state.rows, idx)?;

    state.is_replacing = true;
    let replaced = replace_one(
        manifest,
        paths,
        &ReplaceOneArgs {
            item_id: row.item_id.clone(),
            start_byte: start,
            end_byte: end,
            expected: row.match_text.clone(),
            replace_to: state.replace_with.clone(),
        },
    );
    state.is_replacing = false;

    match replaced {
        Ok(true) => {
            if let Some(r) = state.rows.get_mut(idx) {
                r.is_disabled = true;
                r.replace_to = Some(state.replace_with.clone());
            }
            if idx + 1 < state.rows.len() {
                state.current_idx = Some(idx + 1);
                state.scroll_to_current = true;
            }
            Some(vec![row.item_id])
        }
        Ok(false) => {
            state.error = Some("搜索结果已过期，请重新搜索。".into());
            let keyword = state.debounced_query.clone();
            run_search(state, manifest, paths, &keyword, options);
            None
        }
        Err(err) => {
            state.error = Some(err.to_string());
            None
        }
    }
}

fn jump_action_for_row(paths: &AppPaths, row: &FindMatchRow) -> Option<FindReplaceAction> {
    let content = entries::read_entry(&paths.entries_dir, &row.item_id).ok()?;
    Some(FindReplaceAction::JumpToMatch {
        entry_id: row.item_id.clone(),
        start_char: byte_to_char_index(&content, row.start_byte),
        end_char: byte_to_char_index(&content, row.end_byte),
    })
}
