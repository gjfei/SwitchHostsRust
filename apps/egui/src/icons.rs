//! Tabler SVG 图标 — egui 绘制层，资源来自 [`ui_assets::icons`]。

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::load::SizeHint;
use eframe::egui::load::SizedTexture;
use eframe::egui::{self, pos2, Color32, ColorImage, Context, Id, Rect, Ui, Vec2};
use serde_json::Value;
use switch_hosts_core::manifest_edit::HostsNodeKind;
pub use ui_assets::icons::Icon;

fn icon_cache_id() -> Id {
    Id::new("swh_icon_cache")
}

const ICON_CACHE_PX: [u32; 4] = [16, 20, 24, 32];
const MAX_ICON_TEXTURES: usize = 96;

fn quantize_icon_px(px: u32) -> u32 {
    ICON_CACHE_PX
        .into_iter()
        .find(|size| px <= *size)
        .unwrap_or(32)
}

#[derive(Clone, Default)]
struct IconCache {
    silhouettes: HashMap<(Icon, u32), Arc<ColorImage>>,
    textures: HashMap<(Icon, u32, [u8; 4]), egui::TextureHandle>,
}

fn white_silhouette(image: &mut ColorImage) {
    for px in &mut image.pixels {
        if px.a() > 0 {
            *px = Color32::from_rgba_unmultiplied(255, 255, 255, px.a());
        }
    }
}

fn colorize_silhouette(src: &ColorImage, tint: Color32) -> ColorImage {
    ColorImage {
        size: src.size,
        pixels: src
            .pixels
            .iter()
            .map(|&px| {
                if px.a() == 0 {
                    Color32::TRANSPARENT
                } else {
                    let a = px.a();
                    Color32::from_rgba_unmultiplied(
                        (tint.r() as u32 * a as u32 / 255) as u8,
                        (tint.g() as u32 * a as u32 / 255) as u8,
                        (tint.b() as u32 * a as u32 / 255) as u8,
                        a,
                    )
                }
            })
            .collect(),
    }
}

fn load_silhouette(icon: Icon, px: u32) -> Arc<ColorImage> {
    let mut image = egui_extras::image::load_svg_bytes_with_size(
        icon.svg_bytes(),
        Some(SizeHint::Height(px)),
    )
    .unwrap_or_else(|e| panic!("failed to rasterize icon {icon:?}: {e}"));
    white_silhouette(&mut image);
    Arc::new(image)
}

fn colored_icon_texture(ctx: &Context, icon: Icon, size: f32, tint: Color32) -> Option<SizedTexture> {
    let px = quantize_icon_px((size * ctx.pixels_per_point()).round().max(1.0) as u32);
    let key = (icon, px, [tint.r(), tint.g(), tint.b(), tint.a()]);

    if let Some(handle) = ctx.data(|d| {
        d.get_temp::<IconCache>(icon_cache_id())
            .and_then(|cache| cache.textures.get(&key).cloned())
    }) {
        return Some(SizedTexture::new(handle.id(), Vec2::splat(size)));
    }

    let silhouette = ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_insert_with(icon_cache_id(), IconCache::default);
        cache
            .silhouettes
            .entry((icon, px))
            .or_insert_with(|| load_silhouette(icon, px))
            .clone()
    });

    let colored = colorize_silhouette(&silhouette, tint);
    let name = format!(
        "swh_icon/{icon:?}/{px}/{}/{}/{}/{}",
        tint.r(),
        tint.g(),
        tint.b(),
        tint.a()
    );
    let handle = ctx.load_texture(name, colored, egui::TextureOptions::LINEAR);

    ctx.data_mut(|d| {
        let cache = d.get_temp_mut_or_insert_with(icon_cache_id(), IconCache::default);
        if cache.textures.len() >= MAX_ICON_TEXTURES {
            cache.textures.clear();
        }
        cache.textures.insert(key, handle.clone());
    });

    Some(SizedTexture::new(handle.id(), Vec2::splat(size)))
}

/// 与 SwitchHosts `ItemIcon` 一致默认 16px。
pub const DEFAULT_ICON_SIZE: f32 = 16.0;

pub fn icon(ui: &mut Ui, icon: Icon, size: f32, tint: Color32) -> egui::Response {
    if let Some(tex) = colored_icon_texture(ui.ctx(), icon, size, tint) {
        ui.add(
            egui::Image::from_texture(tex)
                .fit_to_exact_size(Vec2::splat(size))
                .sense(egui::Sense::hover()),
        )
    } else {
        ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover())
            .1
    }
}

pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, tint: Color32) -> egui::Response {
    const MIN_HIT: f32 = 28.0;
    if let Some(tex) = colored_icon_texture(ui.ctx(), icon, size, tint) {
        ui.add(
            egui::Button::image(egui::Image::from_texture(tex))
                .small()
                .min_size(Vec2::splat(size.max(MIN_HIT))),
        )
    } else {
        ui.allocate_exact_size(Vec2::splat(size.max(MIN_HIT)), egui::Sense::click())
            .1
    }
}

pub fn subtle_icon_button(
    ui: &mut Ui,
    icon: Icon,
    size: f32,
    tint: Color32,
    hover_bg: Color32,
    hit: f32,
    radius: f32,
) -> egui::Response {
    let hit = hit.max(size);
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(hit), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        if response.hovered() {
            ui.painter().rect_filled(rect, radius, hover_bg);
        }
        paint_icon(ui, icon, rect.center(), size, tint);
    }
    response
}

pub fn paint_icon(ui: &Ui, icon: Icon, center: egui::Pos2, size: f32, tint: Color32) {
    let Some(tex) = colored_icon_texture(ui.ctx(), icon, size, tint) else {
        return;
    };
    let rect = Rect::from_center_size(center, Vec2::splat(size));
    ui.painter().image(
        tex.id,
        rect,
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

pub fn kind_icon(kind: HostsNodeKind) -> Icon {
    match kind {
        HostsNodeKind::Local => Icon::FileText,
        HostsNodeKind::Remote => Icon::World,
        HostsNodeKind::Group => Icon::Stack2,
        HostsNodeKind::Folder => Icon::Folder,
    }
}

pub fn node_icon(node: &Value, collapsed: bool) -> Icon {
    if node.get("isSys").and_then(|v| v.as_bool()).unwrap_or(false)
        || node.get("is_sys").and_then(|v| v.as_bool()).unwrap_or(false)
    {
        return Icon::DeviceDesktop;
    }
    match node.get("type").and_then(|v| v.as_str()) {
        Some("remote") => Icon::World,
        Some("group") => Icon::Stack2,
        Some("folder") => {
            if collapsed {
                Icon::Folder
            } else {
                Icon::FolderOpen
            }
        }
        _ => Icon::FileText,
    }
}
