//! hosts 编辑器（对齐 SwitchHosts `HostsEditor` + CodeMirror `hosts_cm.ts`）。

use switch_hosts_core::hosts_edit::{
    is_hosts_comment_line, is_valid_hosts_line, parse_line_segments, toggle_comment_by_line,
    toggle_comment_by_selection, TokenKind,
};
use switch_hosts_core::manifest_edit::is_editor_read_only;
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use eframe::egui::widgets::text_edit::TextEditOutput;
use eframe::egui::{self, Color32, FontId, Id, RichText, Sense, TextStyle, Ui, Vec2};

use crate::panels::status_bar::{draw_status_bar, editor_status, pin_body_and_status_bar};
use crate::theme::{self, layout};

const EDITOR_WIDGET_ID: &str = "hosts_code_editor";
/// 对齐 CodeMirror `.cm-content { padding: 8px 0 }`
const EDITOR_PAD_Y: f32 = 8.0;
/// 对齐 `.cm-lineNumbers .cm-gutterElement { padding: 0 6px 0 8px }`
const EDITOR_GUTTER_PAD_LEFT: f32 = 8.0;
const EDITOR_GUTTER_PAD_RIGHT: f32 = 6.0;

struct EditorMetrics {
    font_id: FontId,
}

impl EditorMetrics {
    fn from_ui(ui: &Ui) -> Self {
        let font_id = ui
            .style()
            .text_styles
            .get(&TextStyle::Monospace)
            .cloned()
            .unwrap_or_else(|| FontId::monospace(layout::EDITOR_FONT_SIZE));
        Self { font_id }
    }

    fn gutter_width(&self, ui: &Ui, line_count: usize) -> f32 {
        let t = theme::app(ui.ctx());
        let widest = format!("{}", line_count.max(1));
        let text_w = ui
            .painter()
            .layout_no_wrap(widest, self.font_id.clone(), t.editor_line_number)
            .size()
            .x;
        text_w + EDITOR_GUTTER_PAD_LEFT + EDITOR_GUTTER_PAD_RIGHT
    }
}

/// 编辑器布局度量（gutter 固定，仅代码区滚动）。
struct EditorLayout {
    gutter_w: f32,
    code_vp_w: f32,
    code_content_w: f32,
    content_h: f32,
    line_count: usize,
    viewport_h: f32,
}

impl EditorLayout {
    fn compute(
        ui: &Ui,
        metrics: &EditorMetrics,
        text: &str,
        galley: &egui::Galley,
        viewport: Vec2,
    ) -> Self {
        let line_count = galley.rows.len().max(1);
        let gutter_w = metrics.gutter_width(ui, line_count);
        let code_vp_w = (viewport.x - gutter_w).max(0.0);
        let code_content_w = editor_content_width(ui, text, metrics);
        let content_h = galley_content_height(galley);
        Self {
            gutter_w,
            code_vp_w,
            code_content_w,
            content_h,
            line_count,
            viewport_h: viewport.y,
        }
    }
}

fn editor_content_width(ui: &Ui, text: &str, metrics: &EditorMetrics) -> f32 {
    const RIGHT_MARGIN: f32 = 8.0;
    let t = theme::app(ui.ctx());
    let mut max_w = 0.0f32;
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let galley = ui.fonts_mut(|fonts| fonts.layout_job(build_syntax_job(line, metrics, &t)));
        max_w = max_w.max(galley.size().x);
    }
    max_w + RIGHT_MARGIN
}

fn layout_hosts_galley(ui: &Ui, text: &str, metrics: &EditorMetrics) -> std::sync::Arc<egui::Galley> {
    let t = theme::app(ui.ctx());
    ui.fonts_mut(|fonts| fonts.layout_job(build_syntax_job(text, metrics, &t)))
}

fn galley_content_height(galley: &egui::Galley) -> f32 {
    EDITOR_PAD_Y * 2.0 + galley.size().y
}

fn draw_editor_body(
    ui: &mut Ui,
    metrics: &EditorMetrics,
    text: &mut String,
    bg: Color32,
    read_only: bool,
    editor_id: Id,
    gutter_line: &mut Option<usize>,
    pending_selection: &mut Option<(usize, usize)>,
) {
    let origin = ui.max_rect().min;
    let viewport = ui.max_rect().size();

    let galley = layout_hosts_galley(ui, text, metrics);
    let layout = EditorLayout::compute(ui, metrics, text, &galley, viewport);
    let scroll_h = ui.max_rect().height().min(layout.viewport_h);

    // 代码区：单个 ScrollArea 包裹 code_editor
    let code_rect = egui::Rect::from_min_size(
        origin + egui::vec2(layout.gutter_w, 0.0),
        egui::vec2(layout.code_vp_w, scroll_h),
    );
    let text_output = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(code_rect)
                .id(editor_id.with("code")),
            |ui| {
                ui.style_mut().always_scroll_the_only_direction = true;
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .max_width(layout.code_vp_w)
                    .max_height(scroll_h)
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                    .id_salt("scroll")
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(
                            layout.code_content_w,
                            layout.content_h,
                        ));
                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(egui::Rect::from_min_size(
                                    ui.cursor().min,
                                    egui::vec2(layout.code_content_w, layout.content_h),
                                ))
                                .id(editor_id.with("code_wide")),
                            |ui| {
                                ui.set_width(layout.code_content_w);
                                draw_text_edit(
                                    ui,
                                    text,
                                    read_only,
                                    layout.line_count,
                                    editor_id,
                                    *gutter_line,
                                    pending_selection,
                                )
                            },
                        )
                        .inner
                    })
                    .inner
            },
        )
        .inner;

    // 行号区：按 TextEdit 实际绘制位置对齐（galley_pos 已含滚动偏移）
    let gutter_rect =
        egui::Rect::from_min_size(origin, egui::vec2(layout.gutter_w, scroll_h));
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(gutter_rect)
            .id(editor_id.with("gutter")),
        |ui| {
            draw_gutter_viewport(
                ui,
                metrics,
                &text_output.galley,
                text_output.galley_pos,
                text_output.text_clip_rect,
                bg,
                read_only,
                gutter_line,
            );
        },
    );

}

/// 行号 gutter：与 TextEdit 的 galley 绘制原点对齐。
fn draw_gutter_viewport(
    ui: &mut Ui,
    metrics: &EditorMetrics,
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    text_clip_rect: egui::Rect,
    bg: Color32,
    read_only: bool,
    gutter_line: &mut Option<usize>,
) {
    let gutter_rect = ui.max_rect();
    let painter = ui.painter().with_clip_rect(gutter_rect);
    painter.rect_filled(gutter_rect, 0.0, bg);

    let row_height = ui.fonts_mut(|fonts| fonts.row_height(&metrics.font_id));
    // galley_pos 为屏幕坐标；与 text_clip_rect 取齐，避免空内容时 galley 行高为 0 导致偏上。
    let text_base_y = text_clip_rect.min.y;
    let paint_origin_y = galley_pos.y - galley.rect.top();
    let line_base_y = if (paint_origin_y - text_base_y).abs() <= 1.0 {
        text_base_y
    } else {
        paint_origin_y
    };

    let t = theme::app(ui.ctx());
    let line_count = galley.rows.len().max(1);
    for line_idx in 0..line_count {
        let (line_top, line_h) = if let Some(row) = galley.rows.get(line_idx) {
            (
                line_base_y + row.rect().top(),
                row.rect().height().max(row_height),
            )
        } else {
            (
                line_base_y + line_idx as f32 * row_height,
                row_height,
            )
        };
        let line_rect = egui::Rect::from_min_max(
            egui::pos2(gutter_rect.left(), line_top),
            egui::pos2(gutter_rect.right(), line_top + line_h),
        );
        if line_rect.bottom() < gutter_rect.top() || line_rect.top() > gutter_rect.bottom() {
            continue;
        }

        let response = ui.interact(
            line_rect,
            ui.id().with("gutter").with(line_idx),
            if read_only {
                Sense::hover()
            } else {
                Sense::click()
            },
        );
        if ui.is_rect_visible(line_rect) {
            painter.text(
                egui::pos2(
                    gutter_rect.right() - EDITOR_GUTTER_PAD_RIGHT,
                    line_rect.center().y,
                ),
                egui::Align2::RIGHT_CENTER,
                format!("{}", line_idx + 1),
                metrics.font_id.clone(),
                t.editor_line_number,
            );
        }
        if !read_only && response.clicked() {
            *gutter_line = Some(line_idx);
        }
    }
}

/// 只读 hosts 预览（对齐 `HostsViewer`）。
pub fn draw_readonly_hosts_viewer(ui: &mut Ui, text: &mut String, paint_background: bool) {
    let t = theme::app(ui.ctx());
    let bg = t.editor_readonly_bg;
    let full_rect = ui.max_rect();
    if paint_background {
        ui.painter().rect_filled(full_rect, 0.0, bg);
    }

    let editor_id = Id::new("hosts_readonly_viewer");

    ui.scope_builder(egui::UiBuilder::new().max_rect(full_rect), |ui| {
        let metrics = EditorMetrics::from_ui(ui);
        let mut gutter_line = None;
        let mut no_selection = None;
        draw_editor_body(
            ui,
            &metrics,
            text,
            bg,
            true,
            editor_id,
            &mut gutter_line,
            &mut no_selection,
        );
    });
}

/// 编辑器 + 底部状态栏（对齐 `HostsEditor` `.root` 一体布局）。
pub fn draw_editor_with_status_bar(
    ui: &mut Ui,
    text: &mut String,
    manifest: &Manifest,
    selected_id: Option<&str>,
    editor_revision: u64,
    pending_selection: &mut Option<(usize, usize)>,
) {
    let area = ui.max_rect();
    ui.set_min_size(area.size());
    ui.set_max_size(area.size());

    let status = editor_status(manifest, selected_id, text);
    pin_body_and_status_bar(
        ui,
        |ui| draw_editor_panel(ui, text, manifest, selected_id, editor_revision, pending_selection),
        |ui| draw_status_bar(ui, &status),
    );
}

pub fn draw_editor_panel(
    ui: &mut Ui,
    text: &mut String,
    manifest: &Manifest,
    selected_id: Option<&str>,
    editor_revision: u64,
    pending_selection: &mut Option<(usize, usize)>,
) {
    let node = selected_id.and_then(|id| find_node(&manifest.root, id));
    let read_only = is_editor_read_only(selected_id, node.as_ref());
    let t = theme::app(ui.ctx());
    let bg = if read_only {
        t.editor_readonly_bg
    } else {
        t.editor_bg
    };

    let full_rect = ui.max_rect();
    ui.painter().rect_filled(full_rect, 0.0, bg);

    if selected_id.is_none() {
        ui.scope_builder(egui::UiBuilder::new().max_rect(full_rect), |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("系统 Hosts 或方案")
                        .color(theme::app(ui.ctx()).weak_text),
                );
            });
        });
        return;
    }

    let editor_id = Id::new(EDITOR_WIDGET_ID)
        .with(selected_id.unwrap_or("none"))
        .with(editor_revision);

    ui.scope_builder(egui::UiBuilder::new().max_rect(full_rect), |ui| {
        let metrics = EditorMetrics::from_ui(ui);
        let mut gutter_line = None;
        draw_editor_body(
            ui,
            &metrics,
            text,
            bg,
            read_only,
            editor_id,
            &mut gutter_line,
            pending_selection,
        );
    });
}

fn draw_text_edit(
    ui: &mut Ui,
    text: &mut String,
    read_only: bool,
    line_count: usize,
    editor_id: Id,
    gutter_line: Option<usize>,
    pending_selection: &mut Option<(usize, usize)>,
) -> TextEditOutput {
    let output = if read_only {
        let mut display = text.clone();
        show_text_edit_widget(ui, &mut display, false, line_count, editor_id)
    } else {
        show_text_edit_widget(ui, text, true, line_count, editor_id)
    };

    if read_only {
        return output;
    }

    let cursor = output
        .cursor_range
        .as_ref()
        .map(cursor_char_range)
        .unwrap_or((0, 0));

    if let Some((start, end)) = pending_selection.take() {
        let end = end.min(text.len());
        let start = start.min(end);
        let mut state = output.state.clone();
        state.cursor.set_char_range(Some(CCursorRange {
            primary: CCursor {
                index: end,
                prefer_next_row: false,
            },
            secondary: CCursor {
                index: start,
                prefer_next_row: false,
            },
            h_pos: None,
        }));
        state.store(ui.ctx(), output.response.id);
        output.response.request_focus();
    }

    if let Some(line_idx) = gutter_line {
        let snapshot = text.clone();
        apply_comment_toggle(
            ui,
            text,
            &output,
            toggle_comment_by_line(&snapshot, line_idx, cursor.0, cursor.1),
        );
    }

    if output.response.has_focus() {
        let shortcut = ui.input(|i| {
            i.key_pressed(egui::Key::Slash)
                && (i.modifiers.command || i.modifiers.ctrl)
        });
        if shortcut {
            let snapshot = text.clone();
            apply_comment_toggle(
                ui,
                text,
                &output,
                toggle_comment_by_selection(&snapshot, cursor.0, cursor.1, false),
            );
        }
    }

    output
}

fn show_text_edit_widget(
    ui: &mut Ui,
    text: &mut String,
    interactive: bool,
    line_count: usize,
    editor_id: Id,
) -> TextEditOutput {
    let mut layouter = hosts_syntax_layouter;
    let mut edit = egui::TextEdit::multiline(text)
        .id(editor_id)
        .code_editor()
        .font(TextStyle::Monospace)
        .frame(egui::Frame::NONE)
        .desired_width(f32::INFINITY)
        .desired_rows(line_count)
        .vertical_align(egui::Align::TOP)
        .horizontal_align(egui::Align::LEFT)
        .margin(egui::Margin {
            left: 0,
            right: 8,
            top: EDITOR_PAD_Y as i8,
            bottom: EDITOR_PAD_Y as i8,
        })
        .layouter(&mut layouter);

    if !interactive {
        edit = edit.interactive(false);
    }

    edit.show(ui)
}

fn apply_comment_toggle(
    ui: &Ui,
    text: &mut String,
    output: &TextEditOutput,
    result: switch_hosts_core::hosts_edit::CommentToggleResult,
) {
    if !result.changed {
        return;
    }
    *text = result.content;
    let mut state = output.state.clone();
    state.cursor.set_char_range(Some(CCursorRange {
        primary: CCursor {
            index: result.selection_end,
            prefer_next_row: false,
        },
        secondary: CCursor {
            index: result.selection_start,
            prefer_next_row: false,
        },
        h_pos: None,
    }));
    state.store(ui.ctx(), output.response.id);
    output.response.request_focus();
}

fn cursor_char_range(range: &egui::text::CCursorRange) -> (usize, usize) {
    let [min, max] = range.sorted_cursors();
    (min.index, max.index)
}

fn hosts_syntax_layouter(
    ui: &Ui,
    text: &dyn egui::TextBuffer,
    _wrap_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let metrics = EditorMetrics::from_ui(ui);
    let t = theme::app(ui.ctx());
    ui.fonts_mut(|fonts| fonts.layout_job(build_syntax_job(text.as_str(), &metrics, &t)))
}

fn build_syntax_job(text: &str, metrics: &EditorMetrics, t: &theme::AppTheme) -> LayoutJob {
    let font_id = metrics.font_id.clone();

    if text.is_empty() {
        let mut job = LayoutJob::simple(String::new(), font_id, t.editor_text, f32::INFINITY);
        job.wrap.max_width = f32::INFINITY;
        return job;
    }

    let mut job = LayoutJob::default();

    for line in text.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);

        if body.is_empty() {
            if line.ends_with('\n') {
                job.append("\n", 0.0, line_format(font_id.clone(), t.editor_text));
            } else {
                // 空文档或末尾无换行的空行：须占位一节，否则 galley 行高为 0，行号会偏上。
                job.append("", 0.0, line_format(font_id.clone(), t.editor_text));
            }
            continue;
        }

        if is_hosts_comment_line(body) {
            append_line_text(&mut job, body, t.editor_comment, &font_id);
        } else if !is_valid_hosts_line(body) {
            append_line_text(&mut job, body, t.editor_error, &font_id);
        } else {
            for (i, seg) in parse_line_segments(body).iter().enumerate() {
                if i > 0 {
                    job.append(" ", 0.0, line_format(font_id.clone(), t.editor_text));
                }
                let (color, seg_font) = token_style(seg.kind, &font_id, t);
                job.append(&seg.text, 0.0, line_format(seg_font, color));
            }
        }

        if line.ends_with('\n') {
            job.append("\n", 0.0, line_format(font_id.clone(), t.editor_text));
        }
    }

    job.wrap.max_width = f32::INFINITY;
    job
}

fn line_format(font_id: FontId, color: Color32) -> TextFormat {
    TextFormat::simple(font_id, color)
}

fn append_line_text(
    job: &mut LayoutJob,
    text: &str,
    color: Color32,
    font_id: &FontId,
) {
    job.append(text, 0.0, line_format(font_id.clone(), color));
}

fn token_style(kind: TokenKind, base: &FontId, t: &theme::AppTheme) -> (Color32, FontId) {
    match kind {
        TokenKind::Comment => (t.editor_comment, base.clone()),
        TokenKind::Ip => (t.editor_ip, base.clone()),
        TokenKind::Hostname => (t.editor_text, base.clone()),
        TokenKind::Error => (t.editor_error, base.clone()),
        TokenKind::Plain => (t.editor_text, base.clone()),
    }
}
