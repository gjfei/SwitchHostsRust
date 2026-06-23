use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::util::workspace_root;

const FONT_NAME: &str = "AlibabaPuHuiTi-3-55-Regular.ttf";

pub fn sync_fonts() -> Result<()> {
    let root = workspace_root();
    let dest_dir = root.join("crates/ui-assets/assets/fonts");
    let dest_file = dest_dir.join(FONT_NAME);
    fs::create_dir_all(&dest_dir)?;

    if let Ok(env_path) = std::env::var("SWITCHHOSTS_FONT") {
        let path = PathBuf::from(&env_path);
        if path.is_file() {
            fs::copy(&path, &dest_file).context("copy SWITCHHOSTS_FONT")?;
            eprintln!("==> 已从 SWITCHHOSTS_FONT 复制到 {}", dest_file.display());
            return Ok(());
        }
    }

    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .context("HOME environment variable")?;
    for zip in [
        home.join("Downloads/AlibabaPuHuiTi-3-55-Regular.zip"),
        home.join(format!("Downloads/{}.zip", FONT_NAME.trim_end_matches(".ttf"))),
    ] {
        if zip.is_file() {
            extract_font_from_zip(&zip, &dest_file)?;
            eprintln!("==> 已从 {} 解压到 {}", zip.display(), dest_file.display());
            return Ok(());
        }
    }

    for src in [
        home.join(format!("Library/Fonts/{FONT_NAME}")),
        PathBuf::from(format!("/Library/Fonts/{FONT_NAME}")),
    ] {
        if src.is_file() {
            fs::copy(&src, &dest_file).context("copy system font")?;
            eprintln!("==> 已从 {} 复制到 {}", src.display(), dest_file.display());
            return Ok(());
        }
    }

    bail!(
        "找不到 {FONT_NAME}\n\
         请从 https://fonts.alibabagroup.com/ 下载 Regular，或设置:\n\
         SWITCHHOSTS_FONT=/path/to/{FONT_NAME} cargo sync-fonts"
    )
}

fn extract_font_from_zip(zip_path: &Path, dest: &Path) -> Result<()> {
    let tmp = tempfile::tempdir().context("create temp dir")?;
    let status = Command::new("unzip")
        .args(["-oj", zip_path.to_str().unwrap(), &format!("*/{FONT_NAME}")])
        .arg("-d")
        .arg(tmp.path())
        .status()
        .context("run unzip")?;
    let extracted = tmp.path().join(FONT_NAME);
    if status.success() && extracted.is_file() {
        fs::copy(&extracted, dest)?;
        return Ok(());
    }

    let status = Command::new("unzip")
        .args(["-oj", zip_path.to_str().unwrap(), FONT_NAME])
        .arg("-d")
        .arg(tmp.path())
        .status()
        .context("run unzip (flat)")?;
    if status.success() && extracted.is_file() {
        fs::copy(&extracted, dest)?;
        Ok(())
    } else {
        bail!("无法从 {} 解压 {}", zip_path.display(), FONT_NAME)
    }
}
