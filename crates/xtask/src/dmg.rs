//! macOS DMG：`.app` +「应用程序」链接 + 卷标图标 + 背景（无需 GUI / AppleScript）。
//!
//! 先创建空卷再挂载写入，避免 `hdiutil create -srcfolder` 在含 `/Applications`
//! 符号链接或部分系统目录时返回「操作不被允许」。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

pub struct CreateDmgOptions<'a> {
    pub app_bundle: &'a Path,
    pub output_dmg: &'a Path,
    pub volume_name: &'a str,
    pub volicon: &'a Path,
    pub background: &'a Path,
}

pub fn create_styled_dmg(opts: &CreateDmgOptions<'_>) -> Result<()> {
    let app_name = opts
        .app_bundle
        .file_name()
        .and_then(|n| n.to_str())
        .context("app bundle name")?;

    if !opts.background.is_file() {
        bail!(
            "DMG 背景图不存在: {}（检查 Packager.toml [dmg].background）",
            opts.background.display()
        );
    }
    if !opts.volicon.is_file() {
        bail!(
            "DMG 卷标图标不存在: {}（检查 app 内 icon.icns）",
            opts.volicon.display()
        );
    }

    if opts.output_dmg.is_file() {
        fs::remove_file(opts.output_dmg)
            .with_context(|| format!("remove old {}", opts.output_dmg.display()))?;
    }

    let dmg_name = opts
        .output_dmg
        .file_name()
        .and_then(|n| n.to_str())
        .context("dmg file name")?;
    let dmg_temp = opts
        .output_dmg
        .with_file_name(format!("rw.{dmg_name}"));
    if dmg_temp.is_file() {
        fs::remove_file(&dmg_temp)?;
    }

    if let Some(parent) = opts.output_dmg.parent() {
        fs::create_dir_all(parent)?;
    }

    let size_mb = estimate_size_mb(opts.app_bundle)?;

    eprintln!("    创建 DMG 卷（{}，约 {size_mb} MB）…", opts.volume_name);
    let create = Command::new("hdiutil")
        .args([
            "create",
            "-size",
            &format!("{size_mb}m"),
            "-volname",
            opts.volume_name,
            "-fs",
            "HFS+",
            "-ov",
        ])
        .arg(&dmg_temp)
        .output()
        .context("hdiutil create")?;
    if !create.status.success() {
        bail!(
            "hdiutil create failed: {}",
            String::from_utf8_lossy(&create.stderr)
        );
    }

    let mount_dir = format!("/Volumes/{}", opts.volume_name);
    detach_mount_if_present(&mount_dir)?;

    eprintln!("    挂载并写入内容…");
    let attach = Command::new("hdiutil")
        .args(["attach", "-readwrite", "-noverify", "-noautoopen"])
        .arg(&dmg_temp)
        .output()
        .context("hdiutil attach")?;
    if !attach.status.success() {
        let _ = fs::remove_file(&dmg_temp);
        bail!(
            "hdiutil attach failed: {}",
            String::from_utf8_lossy(&attach.stderr)
        );
    }
    let stdout = String::from_utf8_lossy(&attach.stdout);
    let dev_name = stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .find(|s| s.starts_with("/dev/"))
        .context("parse hdiutil attach device")?
        .to_string();

    let mount_path = PathBuf::from(&mount_dir);
    let bg_name = opts
        .background
        .file_name()
        .and_then(|n| n.to_str())
        .context("background file name")?;

    let result = (|| -> Result<()> {
        copy_dir_all(opts.app_bundle, &mount_path.join(app_name))?;

        if !mount_path.join("Applications").exists() {
            std::os::unix::fs::symlink("/Applications", mount_path.join("Applications"))?;
        }

        let bg_dir = mount_path.join(".background");
        fs::create_dir_all(&bg_dir)?;
        fs::copy(opts.background, bg_dir.join(bg_name))?;

        fs::copy(opts.volicon, mount_path.join(".VolumeIcon.icns"))?;
        let volicon = mount_path.join(".VolumeIcon.icns");
        let _ = run_setfile(&["-c", "icnC", volicon.to_str().unwrap()]);
        let _ = run_setfile(&["-a", "C", mount_path.to_str().unwrap()]);

        let _ = fs::remove_dir_all(mount_path.join(".fseventsd"));
        Ok(())
    })();

    if let Err(err) = result {
        let _ = hdiutil_detach(&dev_name);
        let _ = fs::remove_file(&dmg_temp);
        return Err(err);
    }

    hdiutil_detach(&dev_name)?;

    eprintln!("    压缩 DMG…");
    let convert = Command::new("hdiutil")
        .args([
            "convert",
            dmg_temp.to_str().unwrap(),
            "-format",
            "UDZO",
            "-imagekey",
            "zlib-level=9",
            "-ov",
            "-o",
            opts.output_dmg.to_str().unwrap(),
        ])
        .output()
        .context("hdiutil convert")?;
    fs::remove_file(&dmg_temp).ok();
    if !convert.status.success() {
        bail!(
            "hdiutil convert failed: {}",
            String::from_utf8_lossy(&convert.stderr)
        );
    }

    Ok(())
}

fn estimate_size_mb(app_bundle: &Path) -> Result<u32> {
    let output = Command::new("du")
        .args(["-sk", app_bundle.to_str().context("app path")?])
        .output()
        .context("du")?;
    if !output.status.success() {
        return Ok(256);
    }
    let kb: u32 = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    // du -sk → KB；加 64 MB 余量给背景、元数据
    Ok((kb / 1024).max(64) + 64)
}

fn detach_mount_if_present(mount_dir: &str) -> Result<()> {
    if Path::new(mount_dir).is_dir() {
        let _ = Command::new("hdiutil").args(["detach", mount_dir]).status();
    }
    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    if dst.exists() {
        fs::remove_dir_all(dst)?;
    }
    let output = Command::new("ditto").arg(src).arg(dst).output()?;
    if output.status.success() {
        Ok(())
    } else {
        bail!(
            "ditto copy failed: {} -> {}: {}",
            src.display(),
            dst.display(),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

fn run_setfile(args: &[&str]) -> Result<()> {
    match Command::new("SetFile").args(args).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(_) => bail!("SetFile failed: {:?}", args),
        Err(_) => {
            eprintln!("    warning: 未找到 SetFile，跳过 custom icon 属性");
            Ok(())
        }
    }
}

fn hdiutil_detach(dev_name: &str) -> Result<()> {
    for attempt in 0..3 {
        let status = Command::new("hdiutil").args(["detach", dev_name]).status();
        match status {
            Ok(s) if s.success() => return Ok(()),
            Ok(s) if s.code() == Some(16) && attempt < 2 => {
                std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
            }
            Ok(_) => bail!("hdiutil detach failed for {dev_name}"),
            Err(e) => return Err(e).context("hdiutil detach"),
        }
    }
    bail!("hdiutil detach failed for {dev_name}")
}
