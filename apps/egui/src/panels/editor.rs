use switch_hosts_core::hosts_edit::{parse_line_segments, toggle_line_comment, TokenKind};
use eframe::egui;

pub fn draw_editor(ui: &mut egui::Ui, text: &mut String, selected_id: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(format!(
            "编辑器{}",
            selected_id
                .map(|id| format!(" — {id}"))
                .unwrap_or_default()
        ));
        if ui.button("切换行注释").on_hover_text("对首行切换 # 注释").clicked() {
            if let Some(first) = text.lines().next() {
                let toggled = toggle_line_comment(first);
                if let Some(rest) = text.strip_prefix(first) {
                    *text = format!("{toggled}{rest}");
                } else {
                    *text = toggled;
                }
            }
        }
    });

    ui.add(
        egui::TextEdit::multiline(text)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(20),
    );

    ui.separator();
    ui.label("语法预览（前 5 行）");
    for line in text.lines().take(5) {
        ui.horizontal(|ui| {
            for seg in parse_line_segments(line) {
                let color = match seg.kind {
                    TokenKind::Ip => egui::Color32::from_rgb(0, 128, 255),
                    TokenKind::Hostname => egui::Color32::from_rgb(0, 160, 80),
                    TokenKind::Comment => egui::Color32::GRAY,
                    TokenKind::Plain => egui::Color32::WHITE,
                };
                ui.colored_label(color, &seg.text);
            }
        });
    }
}
