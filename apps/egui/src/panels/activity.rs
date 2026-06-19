use eframe::egui;

pub fn draw_activity_bar(ctx: &egui::Context, view_trash: &mut bool) {
    egui::SidePanel::left("activity_bar")
        .exact_width(48.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if ui.selectable_label(!*view_trash, "H").clicked() {
                    *view_trash = false;
                }
                if ui.selectable_label(*view_trash, "T").clicked() {
                    *view_trash = true;
                }
            });
        });
}
