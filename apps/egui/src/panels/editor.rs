use switch_hosts_core::hosts_edit::{parse_line_segments, TokenKind};
use eframe::egui;

pub fn draw_editor(ui: &mut egui::Ui, text: &mut String, selected_id: Option<&str>) {
    ui.label(format!(
        "Editor{}",
        selected_id
            .map(|id| format!(" — {id}"))
            .unwrap_or_default()
    ));
    ui.add(
        egui::TextEdit::multiline(text)
            .font(egui::TextStyle::Monospace)
            .desired_width(f32::INFINITY)
            .desired_rows(20),
    );

    ui.separator();
    ui.label("Preview (first 5 lines):");
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
