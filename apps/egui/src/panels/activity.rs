use eframe::egui;

/// 左侧活动栏：Hosts / 回收站切换。
pub fn draw_activity_bar(ctx: &egui::Context, view_trash: &mut bool) {
    egui::SidePanel::left("activity_bar")
        .exact_width(52.0)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                if ui
                    .selectable_label(!*view_trash, "Hosts")
                    .on_hover_text("方案列表")
                    .clicked()
                {
                    *view_trash = false;
                }
                ui.add_space(8.0);
                if ui
                    .selectable_label(*view_trash, "回收")
                    .on_hover_text("回收站")
                    .clicked()
                {
                    *view_trash = true;
                }
            });
        });
}
