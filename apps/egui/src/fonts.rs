//! 中文字体：从系统字体加载 CJK 字形，避免界面中文显示为方块。

use std::path::Path;
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};

pub const CJK_FONT_KEY: &str = "switch_hosts_cjk";
pub const CJK_FONT_FAMILY: &str = "switch_hosts_cjk";

/// 在 egui 默认字体后追加 CJK 回退字体（拉丁文仍用内置字体）。
pub fn setup_cjk_fonts(ctx: &egui::Context) {
    let Some(data) = load_cjk_font_data() else {
        tracing::warn!("未找到系统中文字体，界面中文可能无法正常显示");
        return;
    };

    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(CJK_FONT_KEY.to_owned(), Arc::new(data));

    // 拉丁字体优先，CJK 作回退：保证 TopBar / 列表 / 状态栏等默认 UI 的垂直度量一致；
    // 中文缺字仍回退到 CJK。SegmentedControl 等紧凑区域显式用 `cjk_font_id()`。
    // 勿把 emoji 字体放回栈内——会对部分汉字（如「远」）返回占位 glyph。
    let proportional = vec!["Ubuntu-Light".to_owned(), CJK_FONT_KEY.to_owned()];
    let monospace = vec!["Hack".to_owned(), CJK_FONT_KEY.to_owned()];

    fonts
        .families
        .insert(FontFamily::Name(CJK_FONT_FAMILY.into()), vec![CJK_FONT_KEY.to_owned()]);
    fonts.families.insert(FontFamily::Proportional, proportional);
    fonts.families.insert(FontFamily::Monospace, monospace);

    ctx.set_fonts(fonts);
}

/// 强制走 CJK 字体（单一 font family，Latin + CJK 同一套度量）。
pub fn cjk_font_id(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(CJK_FONT_FAMILY.into()))
}

/// 界面可见文字（含中英混排如 `Hosts 类型`、`添加 hosts`）统一用此字体，
/// 避免 Proportional 栈内 Ubuntu / CJK 逐字切换导致基线不齐。
pub fn ui_font_id(size: f32) -> FontId {
    cjk_font_id(size)
}

fn load_cjk_font_data() -> Option<FontData> {
    for path in cjk_font_candidates() {
        if !path.exists() {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let mut data = FontData::from_owned(bytes);
        data.index = cjk_font_index(&path);
        tracing::info!(
            "已加载 CJK 字体: {} (index {})",
            path.display(),
            data.index
        );
        return Some(data);
    }
    None
}

/// TTC 需指定 face index；PingFang SC Regular 在 index 6。
fn cjk_font_index(path: &Path) -> u32 {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name == "pingfang.ttc" {
        6
    } else {
        0
    }
}

fn cjk_font_candidates() -> Vec<std::path::PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return vec![
            "/System/Library/Fonts/Hiragino Sans GB.ttc".into(),
            "/System/Library/Fonts/STHeiti Light.ttc".into(),
            "/System/Library/Fonts/PingFang.ttc".into(),
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

    #[test]
    fn cjk_font_bytes_load_on_desktop() {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        assert!(load_cjk_font_data().is_some());
    }
}
