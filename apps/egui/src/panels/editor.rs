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

use crate::theme::{
    EDITOR_BG, EDITOR_COMMENT, EDITOR_ERROR, EDITOR_FONT_SIZE, EDITOR_IP, EDITOR_LINE_NUMBER,
    EDITOR_READONLY_BG, EDITOR_TEXT,
};

const EDITOR_WIDGET_ID: &str = "hosts_code_editor";
/// 对齐 CodeMirror `.cm-content { padding: 8px 0 }`
const EDITOR_PAD_Y: f32 = 8.0;
/// 对齐 `.cm-lineNumbers .cm-gutterElement { padding: 0 6px 0 8px }`
const EDITOR_GUTTER_PAD_LEFT: f32 = 8.0;
const EDITOR_GUTTER_PAD_RIGHT: f32 = 6.0;

struct EditorMetrics {
    font_id: FontId,
    row_height: f32,
}

impl EditorMetrics {
    fn from_ui(ui: &Ui) -> Self {
        let font_id = FontId::monospace(EDITOR_FONT_SIZE);
        let row_height = ui.fonts(|fonts| fonts.row_height(&font_id));
        Self { font_id, row_height }
    }

    fn content_height(&self, line_count: usize) -> f32 {
        EDITOR_PAD_Y * 2.0 + line_count as f32 * self.row_height
    }

    /// 按最大行号位数计算 gutter 宽度（对齐 CodeMirror 自动 gutter）。
    fn gutter_width(&self, ui: &Ui, line_count: usize) -> f32 {
        let widest = format!("{}", line_count.max(1));
        let text_w = ui
            .painter()
            .layout_no_wrap(widest, self.font_id.clone(), EDITOR_LINE_NUMBER)
            .size()
            .x;
        text_w + EDITOR_GUTTER_PAD_LEFT + EDITOR_GUTTER_PAD_RIGHT
    }
}

/// 只读 hosts 预览（对齐 `HostsViewer`）。
pub fn draw_readonly_hosts_viewer(ui: &mut Ui, text: &mut String) {
    let bg = EDITOR_READONLY_BG;
    let full_rect = ui.max_rect();
    ui.painter().rect_filled(full_rect, 0.0, bg);

    let line_count = text.lines().count().max(1);
    let editor_id = Id::new("hosts_readonly_viewer");

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full_rect), |ui| {
        egui::Frame::new()
            .fill(bg)
            .stroke(Stroke::NONE)
            .inner_margin(0.0)
            .show(ui, |ui| {
                let metrics = EditorMetrics::from_ui(ui);
                let inner = ui.available_size();
                let content_h = metrics.content_height(line_count);
                let body_h = inner.y.max(content_h);

                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(inner.x, body_h));
                        ui.horizontal_top(|ui| {
                            ui.set_min_height(body_h);
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let mut gutter_line = None;
                            draw_gutter(
                                ui,
                                &metrics,
                                line_count,
                                body_h,
                                bg,
                                true,
                                &mut gutter_line,
                            );
                            draw_code_area(
                                ui,
                                &metrics,
                                text,
                                true,
                                line_count,
                                body_h,
                                editor_id,
                                None,
                            );
                        });
                    });
            });
    });
}

pub fn draw_editor_panel(
    ui: &mut Ui,
    text: &mut String,
    manifest: &Manifest,
    selected_id: Option<&str>,
) {
    let node = selected_id.and_then(|id| find_node(&manifest.root, id));
    let read_only = is_editor_read_only(selected_id, node.as_ref());
    let bg = if read_only {
        EDITOR_READONLY_BG
    } else {
        EDITOR_BG
    };

    let full_rect = ui.max_rect();
    ui.painter().rect_filled(full_rect, 0.0, bg);

    if selected_id.is_none() {
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full_rect), |ui| {
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new("系统 Hosts 或方案")
                        .color(Color32::from_rgb(140, 140, 150)),
                );
            });
        });
        return;
    }

    let line_count = text.lines().count().max(1);
    let editor_id = Id::new(EDITOR_WIDGET_ID);

    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(full_rect), |ui| {
        egui::Frame::new()
            .fill(bg)
            .stroke(Stroke::NONE)
            .inner_margin(0.0)
            .show(ui, |ui| {
                let metrics = EditorMetrics::from_ui(ui);
                let inner = ui.available_size();
                let content_h = metrics.content_height(line_count);
                let body_h = inner.y.max(content_h);

                let mut gutter_line: Option<usize> = None;

                egui::ScrollArea::both()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.set_min_size(Vec2::new(inner.x, body_h));
                        ui.horizontal_top(|ui| {
                            ui.set_min_height(body_h);
                            ui.spacing_mut().item_spacing.x = 0.0;
                            draw_gutter(
                                ui,
                                &metrics,
                                line_count,
                                body_h,
                                bg,
                                read_only,
                                &mut gutter_line,
                            );
                            draw_code_area(
                                ui,
                                &metrics,
                                text,
                                read_only,
                                line_count,
                                body_h,
                                editor_id,
                                gutter_line,
                            );
                        });
                    });
            });
    });
}

fn draw_gutter(
    ui: &mut Ui,
    metrics: &EditorMetrics,
    line_count: usize,
    body_height: f32,
    bg: Color32,
    read_only: bool,
    gutter_line: &mut Option<usize>,
) {
    let gutter_width = metrics.gutter_width(ui, line_count);
    let gutter_height = body_height.max(metrics.content_height(line_count));
    let (gutter_rect, _) = ui.allocate_exact_size(
        Vec2::new(gutter_width, gutter_height),
        Sense::hover(),
    );
    ui.painter().rect_filled(gutter_rect, 0.0, bg);

    for line_idx in 0..line_count {
        let line_top = gutter_rect.top() + EDITOR_PAD_Y + line_idx as f32 * metrics.row_height;
        let line_rect = egui::Rect::from_min_max(
            egui::pos2(gutter_rect.left(), line_top),
            egui::pos2(gutter_rect.right(), line_top + metrics.row_height),
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
            ui.painter().text(
                egui::pos2(gutter_rect.right() - EDITOR_GUTTER_PAD_RIGHT, line_rect.center().y),
                egui::Align2::RIGHT_CENTER,
                format!("{}", line_idx + 1),
                metrics.font_id.clone(),
                EDITOR_LINE_NUMBER,
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
    body_height: f32,
    editor_id: Id,
    gutter_line: Option<usize>,
) {
    let viewport_rows =
        ((body_height - EDITOR_PAD_Y * 2.0) / metrics.row_height).floor() as usize;
    let edit_rows = line_count.max(viewport_rows.max(1));

    ui.set_min_width(ui.available_width());
    ui.set_min_height(body_height);

    let mut layouter = hosts_syntax_layouter;
    let mut edit = egui::TextEdit::multiline(text)
        .id(editor_id)
        .code_editor()
        .font(TextStyle::Monospace)
        .frame(false)
        .desired_width(f32::INFINITY)
        .desired_rows(edit_rows)
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

    let output = edit.show(ui);
    let cursor = output
        .cursor_range
        .as_ref()
        .map(cursor_char_range)
        .unwrap_or((0, 0));

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

fn hosts_syntax_layouter(ui: &Ui, text: &str, wrap_width: f32) -> std::sync::Arc<egui::Galley> {
    let metrics = EditorMetrics::from_ui(ui);
    ui.fonts(|fonts| fonts.layout_job(build_syntax_job(text, wrap_width, &metrics)))
}

fn build_syntax_job(text: &str, wrap_width: f32, metrics: &EditorMetrics) -> LayoutJob {
    let font_id = metrics.font_id.clone();
    let row_height = metrics.row_height;
    let mut job = LayoutJob::default();

    for line in text.split_inclusive('\n') {
        let body = line.strip_suffix('\n').unwrap_or(line);

        if body.is_empty() {
            if line.ends_with('\n') {
                job.append(
                    "\n",
                    0.0,
                    line_format(font_id.clone(), EDITOR_TEXT, row_height),
                );
            }
            continue;
        }

        if is_hosts_comment_line(body) {
            append_line_text(&mut job, body, EDITOR_COMMENT, &font_id, row_height);
        } else if !is_valid_hosts_line(body) {
            append_line_text(&mut job, body, EDITOR_ERROR, &font_id, row_height);
        } else {
            for (i, seg) in parse_line_segments(body).iter().enumerate() {
                if i > 0 {
                    job.append(
                        " ",
                        0.0,
                        line_format(font_id.clone(), EDITOR_TEXT, row_height),
                    );
                }
                let (color, seg_font) = token_style(seg.kind, &font_id);
                job.append(
                    &seg.text,
                    0.0,
                    line_format(seg_font, color, row_height),
                );
            }
        }

        if line.ends_with('\n') {
            job.append(
                "\n",
                0.0,
                line_format(font_id.clone(), EDITOR_TEXT, row_height),
            );
        }
    }

    job.wrap.max_width = wrap_width;
    job
}

fn line_format(font_id: FontId, color: Color32, row_height: f32) -> TextFormat {
    let mut fmt = TextFormat::simple(font_id, color);
    fmt.line_height = Some(row_height);
    fmt
}

fn append_line_text(
    job: &mut LayoutJob,
    text: &str,
    color: Color32,
    font_id: &FontId,
    row_height: f32,
) {
    job.append(
        text,
        0.0,
        line_format(font_id.clone(), color, row_height),
    );
}

fn token_style(kind: TokenKind, base: &FontId) -> (Color32, FontId) {
    match kind {
        TokenKind::Comment => (EDITOR_COMMENT, base.clone()),
        TokenKind::Ip => (EDITOR_IP, base.clone()),
        TokenKind::Hostname => (EDITOR_TEXT, base.clone()),
        TokenKind::Error => (EDITOR_ERROR, base.clone()),
        TokenKind::Plain => (EDITOR_TEXT, base.clone()),
    }
}
