//! Tabler SVG 图标（来自 SwitchHosts 依赖的 @tabler/icons@3.42.0，见 `assets/icons/`）。

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui::load::{Bytes, SizeHint};
use eframe::egui::load::SizedTexture;
use eframe::egui::{self, pos2, Color32, ColorImage, Context, Id, Rect, Ui, Vec2};
use serde_json::Value;
use switch_hosts_core::manifest_edit::HostsNodeKind;

fn icon_cache_id() -> Id {
    Id::new("swh_icon_cache")
}

/// 按 (icon, 像素高度, tint) 缓存已着色纹理。
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

fn icon_svg_bytes(icon: Icon) -> &'static [u8] {
    match icon.source() {
        egui::ImageSource::Bytes { bytes, .. } => match bytes {
            Bytes::Static(b) => b,
            Bytes::Shared(_) => {
                debug_assert!(false, "embedded icons must be static bytes");
                &[]
            }
        },
        _ => {
            debug_assert!(false, "embedded icons must be static bytes");
            &[]
        }
    }
}

fn load_silhouette(icon: Icon, px: u32) -> Arc<ColorImage> {
    let mut image =
        egui_extras::image::load_svg_bytes_with_size(icon_svg_bytes(icon), Some(SizeHint::Height(px)))
            .unwrap_or_else(|e| panic!("failed to rasterize icon {icon:?}: {e}"));
    white_silhouette(&mut image);
    Arc::new(image)
}

fn colored_icon_texture(ctx: &Context, icon: Icon, size: f32, tint: Color32) -> Option<SizedTexture> {
    let px = (size * ctx.pixels_per_point()).round().max(1.0) as u32;
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
        d.get_temp_mut_or_insert_with(icon_cache_id(), IconCache::default)
            .textures
            .insert(key, handle.clone());
    });

    Some(SizedTexture::new(handle.id(), Vec2::splat(size)))
}

/// 内置 Tabler outline 图标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    DeviceDesktop,
    FileText,
    World,
    Stack2,
    Folder,
    FolderOpen,
    Trash,
    TrashX,
    ArrowLeft,
    ArrowRight,
    List,
    Search,
    History,
    Settings,
    Plus,
    SidebarLeftCollapse,
    SidebarLeftExpand,
    SidebarRightCollapse,
    SidebarRightExpand,
    Pencil,
    Edit,
    X,
    ChevronRight,
    ChevronDown,
}

impl Icon {
    /// 与 SwitchHosts `ItemIcon` 一致默认 16px。
    pub const DEFAULT_SIZE: f32 = 16.0;

    pub fn source(self) -> egui::ImageSource<'static> {
        match self {
            Self::DeviceDesktop => egui::include_image!("../assets/icons/device-desktop.svg"),
            Self::FileText => egui::include_image!("../assets/icons/file-text.svg"),
            Self::World => egui::include_image!("../assets/icons/world.svg"),
            Self::Stack2 => egui::include_image!("../assets/icons/stack-2.svg"),
            Self::Folder => egui::include_image!("../assets/icons/folder.svg"),
            Self::FolderOpen => egui::include_image!("../assets/icons/folder-open.svg"),
            Self::Trash => egui::include_image!("../assets/icons/trash.svg"),
            Self::TrashX => egui::include_image!("../assets/icons/trash-x.svg"),
            Self::ArrowLeft => egui::include_image!("../assets/icons/arrow-left.svg"),
            Self::ArrowRight => egui::include_image!("../assets/icons/arrow-right.svg"),
            Self::List => egui::include_image!("../assets/icons/list.svg"),
            Self::Search => egui::include_image!("../assets/icons/search.svg"),
            Self::History => egui::include_image!("../assets/icons/history.svg"),
            Self::Settings => egui::include_image!("../assets/icons/settings.svg"),
            Self::Plus => egui::include_image!("../assets/icons/plus.svg"),
            Self::SidebarLeftCollapse => {
                egui::include_image!("../assets/icons/layout-sidebar-left-collapse.svg")
            }
            Self::SidebarLeftExpand => {
                egui::include_image!("../assets/icons/layout-sidebar-left-expand.svg")
            }
            Self::SidebarRightCollapse => {
                egui::include_image!("../assets/icons/layout-sidebar-right-collapse.svg")
            }
            Self::SidebarRightExpand => {
                egui::include_image!("../assets/icons/layout-sidebar-right-expand.svg")
            }
            Self::Pencil => egui::include_image!("../assets/icons/pencil.svg"),
            Self::Edit => egui::include_image!("../assets/icons/edit.svg"),
            Self::X => egui::include_image!("../assets/icons/x.svg"),
            Self::ChevronRight => egui::include_image!("../assets/icons/chevron-right.svg"),
            Self::ChevronDown => egui::include_image!("../assets/icons/chevron-down.svg"),
        }
    }
}

/// 在布局中显示图标（不可点击）。
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

/// 可点击图标按钮（对齐 Mantine filled / 带边框样式）。
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

/// 无边框图标按钮：默认透明，hover 时显示背景（对齐 TopBar `ActionIcon variant="subtle"`）。
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

/// 在指定矩形内绘制图标（用于树行等自定义布局）。
pub fn paint_icon(ui: &Ui, icon: Icon, center: egui::Pos2, size: f32, tint: Color32) {
    let Some(tex) = colored_icon_texture(ui.ctx(), icon, size, tint) else {
        return;
    };
    let rect = Rect::from_center_size(center, Vec2::splat(size));
    ui.painter()
        .image(tex.id, rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

/// 根据 hosts 类型解析图标。
pub fn kind_icon(kind: HostsNodeKind) -> Icon {
    match kind {
        HostsNodeKind::Local => Icon::FileText,
        HostsNodeKind::Remote => Icon::World,
        HostsNodeKind::Group => Icon::Stack2,
        HostsNodeKind::Folder => Icon::Folder,
    }
}

/// 根据 manifest 节点解析图标（对齐 `ItemIcon.tsx`）。
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

/// 安装 SVG/图片加载器（在 `App::new` 中调用一次）。
pub fn install_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    let _ = ctx;
}
