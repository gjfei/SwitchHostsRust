//! 从 URL 导入备份（对齐 `ImportFromUrl.tsx`）。

use eframe::egui::{self, Color32, RichText, Sense, Stroke, Vec2};
use egui::Context;

use crate::panels::drawer::drawer_text_button;
use crate::theme::{self, layout};

const MODAL_MAX_WIDTH: f32 = 400.0;
const MODAL_MIN_WIDTH: f32 = 320.0;
const MODAL_SHADOW: egui::epaint::Shadow = egui::epaint::Shadow {
    offset: [0, 8],
    blur: 24,
    spread: 0,
    color: Color32::from_black_alpha(40),
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ImportFromUrlResult {
    #[default]
    None,
    Cancelled,
    Confirmed(String),
}

#[derive(Debug, Clone, Default)]
pub struct ImportFromUrlState {
    pub open: bool,
    pub url: String,
}

impl ImportFromUrlState {
    pub fn open_modal(&mut self) {
        self.open = true;
        self.url.clear();
    }
}

fn import_url_valid(url: &str) -> bool {
    let rest = if let Some(r) = url.strip_prefix("http://") {
        Some(r)
    } else if let Some(r) = url.strip_prefix("https://") {
        Some(r)
    } else if let Some(r) = url.strip_prefix("HTTP://") {
        Some(r)
    } else if let Some(r) = url.strip_prefix("HTTPS://") {
        Some(r)
    } else {
        None
    };
    rest.is_some_and(|r| {
        r.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    })
}

pub fn draw_import_from_url_modal(
    ctx: &Context,
    state: &mut ImportFromUrlState,
) -> ImportFromUrlResult {
    if !state.open {
        return ImportFromUrlResult::None;
    }

    let t = theme::app(ctx);
    let mut result = ImportFromUrlResult::None;
    let screen = ctx.input(|i| i.content_rect());
    let ok_enabled = import_url_valid(&state.url);

    let backdrop_id = egui::Id::new("import_from_url_backdrop");
    egui::Area::new(backdrop_id)
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.set_min_size(screen.size());
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(100));
            if ui
                .interact(screen, backdrop_id, Sense::click())
                .clicked()
            {
                state.open = false;
                result = ImportFromUrlResult::Cancelled;
            }
        });

    let modal_width = MODAL_MAX_WIDTH.min(screen.width() - 32.0).max(MODAL_MIN_WIDTH);

    egui::Area::new(egui::Id::new("import_from_url_modal"))
        .order(egui::Order::Tooltip)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(modal_width, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    egui::Frame {
                        fill: t.editor_bg,
                        corner_radius: t.corner_drawer(),
                        shadow: MODAL_SHADOW,
                        inner_margin: egui::Margin::same(layout::DRAWER_PAD as i8),
                        ..Default::default()
                    }
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("从 URL 导入")
                                .size(16.0)
                                .strong()
                                .color(t.text),
                        );
                        ui.add_space(8.0);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut state.url)
                                .hint_text("http:// or https://")
                                .desired_width(f32::INFINITY)
                                .margin(egui::Margin::symmetric(8, 6)),
                        );
                        if state.open && ui.memory(|m| m.focused().is_none()) {
                            response.request_focus();
                        }
                        if response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && ok_enabled
                        {
                            state.open = false;
                            result = ImportFromUrlResult::Confirmed(state.url.clone());
                        }

                        ui.add_space(16.0);
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                ui.add_enabled_ui(ok_enabled, |ui| {
                                    if drawer_text_button(
                                        ui,
                                        "确定",
                                        t.accent,
                                        Stroke::NONE,
                                        t.text_selected,
                                        true,
                                    )
                                    .clicked()
                                    {
                                        state.open = false;
                                        result =
                                            ImportFromUrlResult::Confirmed(state.url.clone());
                                    }
                                });
                                ui.add_space(12.0);
                                if drawer_text_button(
                                    ui,
                                    "取消",
                                    t.editor_bg,
                                    Stroke::new(1.0, t.input_border),
                                    t.text,
                                    true,
                                )
                                .clicked()
                                {
                                    state.open = false;
                                    result = ImportFromUrlResult::Cancelled;
                                }
                            },
                        );
                    });
                },
            );
        });

    result
}
