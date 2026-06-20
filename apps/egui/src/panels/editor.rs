//! hosts 编辑器（对齐 SwitchHosts `HostsEditor` + CodeMirror `hosts_cm.ts`）。

use switch_hosts_core::hosts_edit::{
    is_hosts_comment_line, is_valid_hosts_line, parse_line_segments, toggle_comment_by_line,
    toggle_comment_by_selection, TokenKind,
};
use switch_hosts_core::manifest_edit::is_editor_read_only;
use switch_hosts_core::storage::manifest::{find_node, Manifest};
use eframe::egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use eframe::egui::widgets::text_edit::TextEditOutput;
use eframe::egui::{self, Color32, FontId, Id, RichText, Sense, Stroke, TextStyle, Ui, Vec2};

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

    /// 按最大行号位数计算 gutter 宽度（对齐 CodeMirror 自动 gutter）。
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

/// 最长行的像素宽度（用于横向滚动，不换行）。
fn editor_content_width(ui: &Ui, text: &str, metrics: &EditorMetrics) -> f32 {
    const RIGHT_MARGIN: f32 = 8.0;
    let t = theme::app(ui.ctx());
    let mut max_w = 0.0f32;
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let galley = ui.fonts(|fonts| fonts.layout_job(build_syntax_job(line, metrics, &t)));
        max_w = max_w.max(galley.size().x);
    }
    max_w + RIGHT_MARGIN
}

fn layout_hosts_galley(ui: &Ui, text: &str, metrics: &EditorMetrics) -> std::sync::Arc<egui::Galley> {
    let t = theme::app(ui.ctx());
    ui.fonts(|fonts| fonts.layout_job(build_syntax_job(text, metrics, &t)))
}

fn galley_content_height(galley: &egui::Galley) -> f32 {
    EDITOR_PAD_Y * 2.0 + galley.size().y
}

fn editor_line_count_estimate(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.chars().filter(|&c| c == '\n').count() + 1
    }
}

fn draw_editor_body(
    ui: &mut Ui,
    metrics: &EditorMetrics,
    text: &mut String,
    viewport: Vec2,
    bg: Color32,
    read_only: bool,
    editor_id: Id,
    gutter_line: &mut Option<usize>,
    pending_selection: &mut Option<(usize, usize)>,
) {
    let gutter_w = metrics.gutter_width(ui, editor_line_count_estimate(text));
    let code_viewport_w = (viewport.x - gutter_w).max(0.0);
    let code_min_w = editor_content_width(ui, text, metrics).max(code_viewport_w);

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .id_salt(editor_id.with("vscroll"))
        .show(ui, |ui| {
            let galley = layout_hosts_galley(ui, text, metrics);
            let line_count = galley.rows.len().max(1);
            let gutter_w = metrics.gutter_width(ui, line_count);
            let content_h = galley_content_height(&galley);
            let total_w = gutter_w + code_min_w;

            let (body_rect, _) =
                ui.allocate_exact_size(Vec2::new(total_w, content_h), Sense::hover());

            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(body_rect), |ui| {
                ui.horizontal_top(|ui| {
                    ui.set_min_size(Vec2::new(total_w, content_h));
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let gutter_w = metrics.gutter_width(ui, line_count);
                    ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                            ui.cursor().min,
                            Vec2::new(gutter_w, content_h),
                        )),
                        |ui| {
                            draw_gutter(
                                ui,
                                metrics,
                                &galley,
                                content_h,
                                bg,
                                read_only,
                                gutter_line,
                            );
                        },
                    );

                    egui::ScrollArea::horizontal()
                        .auto_shrink([false; 2])
                        .id_salt(editor_id.with("hscroll"))
                        .show(ui, |ui| {
                            ui.set_min_width(code_min_w);
                            ui.set_min_height(content_h);
                            draw_code_area(
                                ui,
                                metrics,
                                text,
                                read_only,
                                line_count,
                                content_h,
                                editor_id,
                                *gutter_line,
                                pending_selection,
                            );
                        });
                });
            });
        });
}

/// 只读 hosts 预览（对齐 `HostsViewer`）。
pub fn draw_readonly_hosts_viewer(ui: &mut Ui, text: &mut String) {
    let t = theme::app(ui.ctx());
    let bg = t.editor_readonly_bg;
    let full_rect = ui.max_rect();
    ui.painter().rect_filled(full_rect, 0.0, bg);

    let editor_id = Id::new("hosts_readonly_viewer");

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full_rect), |ui| {
        egui::Frame::new()
            .fill(bg)
            .stroke(Stroke::NONE)
            .inner_margin(0.0)
            .show(ui, |ui| {
                let metrics = EditorMetrics::from_ui(ui);
                let inner = ui.available_size();
                let mut gutter_line = None;
                let mut no_selection = None;
                draw_editor_body(
                    ui,
                    &metrics,
                    text,
                    inner,
                    bg,
                    true,
                    editor_id,
                    &mut gutter_line,
                    &mut no_selection,
                );
            });
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
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full_rect), |ui| {
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

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full_rect), |ui| {
        egui::Frame::new()
            .fill(bg)
            .stroke(Stroke::NONE)
            .inner_margin(0.0)
            .show(ui, |ui| {
                let metrics = EditorMetrics::from_ui(ui);
                let inner = ui.available_size();
                let mut gutter_line = None;
                draw_editor_body(
                    ui,
                    &metrics,
                    text,
                    inner,
                    bg,
                    read_only,
                    editor_id,
                    &mut gutter_line,
                    pending_selection,
                );
            });
    });
}

fn draw_gutter(
    ui: &mut Ui,
    metrics: &EditorMetrics,
    galley: &egui::Galley,
    content_height: f32,
    bg: Color32,
    read_only: bool,
    gutter_line: &mut Option<usize>,
) {
    let line_count = galley.rows.len().max(1);
    let gutter_width = metrics.gutter_width(ui, line_count);
    let (gutter_rect, _) = ui.allocate_exact_size(
        Vec2::new(gutter_width, content_height),
        Sense::hover(),
    );
    ui.painter().rect_filled(gutter_rect, 0.0, bg);

    for (line_idx, row) in galley.rows.iter().enumerate() {
        let line_top = gutter_rect.top() + EDITOR_PAD_Y + row.rect.top();
        let line_rect = egui::Rect::from_min_max(
            egui::pos2(gutter_rect.left(), line_top),
            egui::pos2(gutter_rect.right(), line_top + row.rect.height()),
        );
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
            let t = theme::app(ui.ctx());
            let line_center_y = gutter_rect.top() + EDITOR_PAD_Y + row.rect.center().y;
            ui.painter().text(
                egui::pos2(gutter_rect.right() - EDITOR_GUTTER_PAD_RIGHT, line_center_y),
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

fn draw_code_area(
    ui: &mut Ui,
    metrics: &EditorMetrics,
    text: &mut String,
    read_only: bool,
    line_count: usize,
    content_height: f32,
    editor_id: Id,
    gutter_line: Option<usize>,
    pending_selection: &mut Option<(usize, usize)>,
) {
    if read_only {
        let mut display = text.clone();
        draw_text_edit(
            ui,
            metrics,
            &mut display,
            true,
            line_count,
            content_height,
            editor_id,
            gutter_line,
            pending_selection,
        );
        return;
    }

    draw_text_edit(
        ui,
        metrics,
        text,
        false,
        line_count,
        content_height,
        editor_id,
        gutter_line,
        pending_selection,
    );
}

fn draw_text_edit(
    ui: &mut Ui,
    metrics: &EditorMetrics,
    text: &mut String,
    read_only: bool,
    line_count: usize,
    content_height: f32,
    editor_id: Id,
    gutter_line: Option<usize>,
    pending_selection: &mut Option<(usize, usize)>,
) {
    let content_w = editor_content_width(ui, text, metrics).max(ui.available_width());

    let mut layouter = hosts_syntax_layouter;
    let mut edit = egui::TextEdit::multiline(text)
        .id(editor_id)
        .code_editor()
        .font(TextStyle::Monospace)
        .frame(false)
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

    if read_only {
        edit = edit.interactive(false);
    }

    let output = ui
        .allocate_ui_with_layout(
            Vec2::new(content_w, content_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| edit.show(ui),
        )
        .inner;
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

    if output.response.has_focus() && !read_only {
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
                // egui 编辑器保持光标在当前行；不用 CodeMirror 的 moveToNextLine 行为。
                toggle_comment_by_selection(&snapshot, cursor.0, cursor.1, false),
            );
        }
    }
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
            index: result.selection_start,
            prefer_next_row: false,
        },
        secondary: CCursor {
            index: result.selection_end,
            prefer_next_row: false,
        },
    }));
    state.store(ui.ctx(), output.response.id);
    output.response.request_focus();
}

fn cursor_char_range(range: &egui::text::CursorRange) -> (usize, usize) {
    let [min, max] = range.sorted_cursors();
    (min.ccursor.index, max.ccursor.index)
}

fn hosts_syntax_layouter(ui: &Ui, text: &str, _wrap_width: f32) -> std::sync::Arc<egui::Galley> {
    let metrics = EditorMetrics::from_ui(ui);
    let t = theme::app(ui.ctx());
    ui.fonts(|fonts| fonts.layout_job(build_syntax_job(text, &metrics, &t)))
}

fn build_syntax_job(text: &str, metrics: &EditorMetrics, t: &theme::AppTheme) -> LayoutJob {
    let font_id = metrics.font_id.clone();
    let mut job = LayoutJob::default();

    for line in text.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);

        if body.is_empty() {
            if line.ends_with('\n') {
                job.append("\n", 0.0, line_format(font_id.clone(), t.editor_text));
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
