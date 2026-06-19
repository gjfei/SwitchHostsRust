//! 中文字体：从系统字体加载 CJK 字形，避免界面中文显示为方块。

use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

const CJK_FONT_KEY: &str = "switch_hosts_cjk";

/// 在 egui 默认字体后追加 CJK 回退字体（拉丁文仍用内置字体）。
pub fn setup_cjk_fonts(ctx: &egui::Context) {
    let Some(bytes) = load_cjk_font_bytes() else {
        tracing::warn!("未找到系统中文字体，界面中文可能无法正常显示");
        return;
    };

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        CJK_FONT_KEY.to_owned(),
        Arc::new(FontData::from_owned(bytes)),
    );

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.push(CJK_FONT_KEY.to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push(CJK_FONT_KEY.to_owned());
    }

    ctx.set_fonts(fonts);
}

/// 按平台尝试读取常见系统 CJK 字体。
fn load_cjk_font_bytes() -> Option<Vec<u8>> {
    for path in cjk_font_candidates() {
        if path.exists() {
            if let Ok(bytes) = std::fs::read(&path) {
                tracing::info!("已加载 CJK 字体: {}", path.display());
                return Some(bytes);
            }
        }
    }
    None
}

fn cjk_font_candidates() -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return vec![
            "/System/Library/Fonts/PingFang.ttc".into(),
            "/System/Library/Fonts/STHeiti Light.ttc".into(),
            "/System/Library/Fonts/Hiragino Sans GB.ttc".into(),
            "/Library/Fonts/Arial Unicode.ttf".into(),
        ];
    }
    #[cfg(target_os = "windows")]
    {
        return vec![
            "C:\\Windows\\Fonts\\msyh.ttc".into(),
            "C:\\Windows\\Fonts\\msyhbd.ttc".into(),
            "C:\\Windows\\Fonts\\simhei.ttf".into(),
            "C:\\Windows\\Fonts\\simsun.ttc".into(),
        ];
    }
    #[cfg(target_os = "linux")]
    {
        return vec![
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".into(),
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc".into(),
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc".into(),
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc".into(),
        ];
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_has_pingfang_or_fallback() {
        #[cfg(target_os = "macos")]
        {
            let candidates = cjk_font_candidates();
            assert!(candidates.iter().any(|p| p.exists()));
        }
    }

    #[test]
    fn font_candidates_non_empty_on_desktop() {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        assert!(!cjk_font_candidates().is_empty());
    }
}
