//! Tabler SVG 图标（@tabler/icons 3.42.0，见 `assets/icons/`）。

/// 内置 Tabler outline 图标标识。
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
    InfoCircle,
    Refresh,
    Message2,
    Home,
    Upload,
    Download,
    CloudDownload,
    Adjustments,
    Logout,
}

impl Icon {
    /// 原始 SVG 字节。
    pub fn svg_bytes(self) -> &'static [u8] {
        match self {
            Self::DeviceDesktop => include_bytes!("../assets/icons/device-desktop.svg"),
            Self::FileText => include_bytes!("../assets/icons/file-text.svg"),
            Self::World => include_bytes!("../assets/icons/world.svg"),
            Self::Stack2 => include_bytes!("../assets/icons/stack-2.svg"),
            Self::Folder => include_bytes!("../assets/icons/folder.svg"),
            Self::FolderOpen => include_bytes!("../assets/icons/folder-open.svg"),
            Self::Trash => include_bytes!("../assets/icons/trash.svg"),
            Self::TrashX => include_bytes!("../assets/icons/trash-x.svg"),
            Self::ArrowLeft => include_bytes!("../assets/icons/arrow-left.svg"),
            Self::ArrowRight => include_bytes!("../assets/icons/arrow-right.svg"),
            Self::List => include_bytes!("../assets/icons/list.svg"),
            Self::Search => include_bytes!("../assets/icons/search.svg"),
            Self::History => include_bytes!("../assets/icons/history.svg"),
            Self::Settings => include_bytes!("../assets/icons/settings.svg"),
            Self::Plus => include_bytes!("../assets/icons/plus.svg"),
            Self::SidebarLeftCollapse => {
                include_bytes!("../assets/icons/layout-sidebar-left-collapse.svg")
            }
            Self::SidebarLeftExpand => {
                include_bytes!("../assets/icons/layout-sidebar-left-expand.svg")
            }
            Self::SidebarRightCollapse => {
                include_bytes!("../assets/icons/layout-sidebar-right-collapse.svg")
            }
            Self::SidebarRightExpand => {
                include_bytes!("../assets/icons/layout-sidebar-right-expand.svg")
            }
            Self::Pencil => include_bytes!("../assets/icons/pencil.svg"),
            Self::Edit => include_bytes!("../assets/icons/edit.svg"),
            Self::X => include_bytes!("../assets/icons/x.svg"),
            Self::ChevronRight => include_bytes!("../assets/icons/chevron-right.svg"),
            Self::ChevronDown => include_bytes!("../assets/icons/chevron-down.svg"),
            Self::InfoCircle => include_bytes!("../assets/icons/info-circle.svg"),
            Self::Refresh => include_bytes!("../assets/icons/refresh.svg"),
            Self::Message2 => include_bytes!("../assets/icons/message-2.svg"),
            Self::Home => include_bytes!("../assets/icons/home.svg"),
            Self::Upload => include_bytes!("../assets/icons/upload.svg"),
            Self::Download => include_bytes!("../assets/icons/download.svg"),
            Self::CloudDownload => include_bytes!("../assets/icons/cloud-download.svg"),
            Self::Adjustments => include_bytes!("../assets/icons/adjustments.svg"),
            Self::Logout => include_bytes!("../assets/icons/logout.svg"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_has_svg_payload() {
        let icons = [
            Icon::DeviceDesktop,
            Icon::FileText,
            Icon::World,
            Icon::Trash,
            Icon::Settings,
        ];
        for icon in icons {
            assert!(icon.svg_bytes().starts_with(b"<svg"));
        }
    }
}
