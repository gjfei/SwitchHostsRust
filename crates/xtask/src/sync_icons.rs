use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::util::workspace_root;

pub fn sync_icons() -> Result<()> {
    let root = workspace_root();
    let default_src = PathBuf::from("/Users/jarven/Desktop/project/self/SwitchHosts/src-tauri/icons");
    let src = std::env::var("SWITCHHOSTS_ICONS")
        .map(PathBuf::from)
        .unwrap_or(default_src);
    let dest = root.join("crates/ui-assets/app-icons");

    if !src.is_dir() {
        bail!(
            "找不到 SwitchHosts 图标目录: {}\n可设置环境变量 SWITCHHOSTS_ICONS 指向 src-tauri/icons",
            src.display()
        );
    }

    fs::create_dir_all(&dest)?;
    let mut count = 0usize;
    for entry in fs::read_dir(&src).with_context(|| format!("read {}", src.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().context("file name")?;
            fs::copy(&path, dest.join(file_name))?;
            count += 1;
        }
    }

    eprintln!("==> 已同步 {count} 个文件到 {}", dest.display());
    Ok(())
}
