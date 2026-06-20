//! egui 字体：消费 [`ui_assets::fonts`] 中的普惠体字节。

use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};
use ui_assets::fonts;

pub const UI_FONT_KEY: &str = "alibaba_puhuiti";
pub const UI_FONT_FAMILY: &str = "alibaba_puhuiti";

pub const CJK_FONT_KEY: &str = UI_FONT_KEY;
pub const CJK_FONT_FAMILY: &str = UI_FONT_FAMILY;

pub fn setup_fonts(ctx: &egui::Context) {
    let Some(bytes) = fonts::load_puhuiti_regular_bytes() else {
        tracing::warn!("未加载阿里巴巴普惠体，界面可能无法正常显示中文");
        return;
    };
    let mut data = FontData::from_owned(bytes);
    data.index = 0;

    let mut defs = FontDefinitions::default();
    defs.font_data.insert(UI_FONT_KEY.to_owned(), Arc::new(data));

    let stack = vec![UI_FONT_KEY.to_owned()];
    defs.families
        .insert(FontFamily::Name(UI_FONT_FAMILY.into()), stack.clone());
    defs.families.insert(FontFamily::Proportional, stack.clone());
    defs.families.insert(FontFamily::Monospace, stack);

    ctx.set_fonts(defs);
}

pub fn setup_cjk_fonts(ctx: &egui::Context) {
    setup_fonts(ctx);
}

pub fn ui_font_id(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(UI_FONT_FAMILY.into()))
}

pub fn cjk_font_id(size: f32) -> FontId {
    ui_font_id(size)
}
