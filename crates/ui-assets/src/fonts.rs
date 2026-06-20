//! 阿里巴巴普惠体 3.0 Regular（编译期嵌入 + 可选运行时路径）。

use std::path::PathBuf;

pub const PUHUITI_REGULAR_FILE: &str = "AlibabaPuHuiTi-3-55-Regular.ttf";

const EMBEDDED: &[u8] = include_bytes!("../assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf");

/// 编译期嵌入的普惠体 Regular 字节。
pub fn puhuiti_regular_bytes() -> &'static [u8] {
    EMBEDDED
}

/// 按优先级读取字体：嵌入 → bundle Resources → 系统安装路径。
pub fn load_puhuiti_regular_bytes() -> Option<Vec<u8>> {
    if EMBEDDED.len() >= 1024 {
        return Some(EMBEDDED.to_vec());
    }
    for path in puhuiti_regular_path_candidates() {
        if let Ok(bytes) = std::fs::read(&path) {
            if bytes.len() >= 1024 {
                return Some(bytes);
            }
        }
    }
    None
}

pub fn puhuiti_regular_path_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(resources) = exe
            .parent()
            .and_then(|macos| macos.parent())
            .map(|contents| contents.join("Resources/fonts").join(PUHUITI_REGULAR_FILE))
        {
            paths.push(resources);
        }
    }

    paths.extend(installed_puhuiti_paths());

    #[cfg(debug_assertions)]
    {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        paths.push(manifest.join("assets/fonts").join(PUHUITI_REGULAR_FILE));
        paths.push(
            PathBuf::from("crates/ui-assets/assets/fonts").join(PUHUITI_REGULAR_FILE),
        );
        paths.push(PathBuf::from("assets/fonts").join(PUHUITI_REGULAR_FILE));
    }

    paths
}

fn installed_puhuiti_paths() -> Vec<PathBuf> {
    let name = PUHUITI_REGULAR_FILE;
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return vec![
                PathBuf::from(home).join("Library/Fonts").join(name),
                PathBuf::from("/Library/Fonts").join(name),
            ];
        }
    }
    #[cfg(target_os = "windows")]
    {
        return vec![PathBuf::from(r"C:\Windows\Fonts").join(name)];
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return vec![
                PathBuf::from("/usr/share/fonts/opentype/alibaba-puhuiti").join(name),
                PathBuf::from("/usr/share/fonts/truetype/alibaba-puhuiti").join(name),
                PathBuf::from(home)
                    .join(".local/share/fonts")
                    .join(name),
            ];
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = name;
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_puhuiti_is_present() {
        assert!(puhuiti_regular_bytes().len() > 1_000_000);
    }

    #[test]
    fn load_puhuiti_from_embedded() {
        assert!(load_puhuiti_regular_bytes().is_some());
    }

    #[test]
    fn path_candidates_non_empty_on_desktop() {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        assert!(!puhuiti_regular_path_candidates().is_empty());
    }
}
