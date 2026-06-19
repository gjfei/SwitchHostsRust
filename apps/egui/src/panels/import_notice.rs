//! 操作结果提示弹窗。

use eframe::egui::Context;

use crate::panels::drawer::{draw_confirm_modal, ConfirmModalResult};

fn draw_message_modal(ctx: &Context, id: &str, title: &str, message: &str) -> bool {
    matches!(
        draw_confirm_modal(ctx, id, title, message, "确定", false),
        ConfirmModalResult::Confirmed | ConfirmModalResult::Cancelled
    )
}

pub fn draw_import_error_modal(ctx: &Context, message: &str) -> bool {
    draw_message_modal(ctx, "import_error", "导入失败", message)
}

pub fn draw_apply_error_modal(ctx: &Context, message: &str) -> bool {
    draw_message_modal(ctx, "apply_error", "写入 hosts 失败", message)
}
