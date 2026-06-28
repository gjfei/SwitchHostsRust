use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use cargo_packager::PackageFormat;

use crate::packager_config::{self, AppSpec};
use crate::util::{cargo_target_dir, resolve_path, run_cargo, workspace_root};

pub struct PackageOptions {
    pub release: bool,
    /// 仅构建 .app，不生成 .dmg（macOS）
    pub app_only: bool,
    /// 默认全部 enabled app（仅 app/ 下）；可指定 `egui-app` 等
    pub apps: Vec<String>,
    /// 默认 `dist/`；开发 `.app` 可写入 `target/packager/debug` 避免覆盖发布产物
    pub out_dir: Option<PathBuf>,
}

enum TargetPlatform {
    MacOs,
    #[allow(dead_code)]
    Windows,
}

pub fn package_macos(opts: PackageOptions) -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = opts;
        bail!("package-macos 仅支持 macOS");
    }

    #[cfg(target_os = "macos")]
    package_platform(opts, TargetPlatform::MacOs)
}

pub fn package_windows(opts: PackageOptions) -> Result<()> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = opts;
        bail!("package-windows 仅支持 Windows");
    }

    #[cfg(target_os = "windows")]
    package_platform(opts, TargetPlatform::Windows)
}

fn package_platform(opts: PackageOptions, platform: TargetPlatform) -> Result<()> {
    let root = workspace_root();
    let manifest = packager_config::load_manifest(&root)?;
    let apps = packager_config::select_apps(&manifest, &opts.apps)?;
    let target_dir = cargo_target_dir(&root)?;
    let profile = if opts.release { "release" } else { "debug" };
    let make_dmg = matches!(platform, TargetPlatform::MacOs) && !opts.app_only;
    let app_only = opts.app_only;
    let out_dir = opts
        .out_dir
        .map(|p| resolve_path(&root, p))
        .unwrap_or_else(|| resolve_path(&root, manifest.out_dir.clone()));
    // 发布 DMG：.app 仅作中间产物，写入 target/packager/，dist/ 只留 .dmg
    let packager_out_dir = if make_dmg {
        root.join("target/packager").join(profile)
    } else {
        out_dir.clone()
    };

    build_apps(&root, opts.release, &apps)?;

    if make_dmg {
        std::fs::create_dir_all(&packager_out_dir).with_context(|| {
            format!("create staging dir {}", packager_out_dir.display())
        })?;
    }

    eprintln!(
        "==> cargo-packager ({profile}, {:?}) — {} app(s)",
        platform_label(&platform),
        apps.len()
    );

    let mut all_outputs = Vec::new();
    for app in apps {
        let formats = match platform {
            TargetPlatform::MacOs => packager_config::resolve_mac_packager_formats(app_only)?,
            TargetPlatform::Windows => packager_config::resolve_windows_packager_formats(&manifest)?,
        };
        eprintln!("==> {} ({})", app.product_name, app.name);
        let config = app.to_packager_config(
            &manifest,
            &root,
            &target_dir,
            profile,
            &packager_out_dir,
            &formats,
        )?;
        let outputs =
            cargo_packager::package(&config).with_context(|| format!("pack `{}`", app.name))?;
        for output in &outputs {
            for path in &output.paths {
                eprintln!("  {} ({:?})", path.display(), output.format);
                if matches!(platform, TargetPlatform::MacOs)
                    && output.format == PackageFormat::App
                {
                    verify_app_icon(path)?;
                    adhoc_sign(path);
                    if make_dmg && packager_config::wants_dmg(&manifest, app, app_only) {
                        let dmg = out_dir.join(format!("{}.dmg", app.product_name));
                        let volicon = path.join("Contents/Resources/icon.icns");
                        let dmg_section = manifest.dmg.as_ref();
                        let background = dmg_section
                            .map(|d| d.resolved_background(&root))
                            .unwrap_or_else(|| {
                                root.join("crates/xtask/assets/dmg/background.png")
                            });
                        crate::dmg::create_styled_dmg(&crate::dmg::CreateDmgOptions {
                            app_bundle: path,
                            output_dmg: &dmg,
                            volume_name: &app.product_name,
                            volicon: &volicon,
                            background: &background,
                        })?;
                        eprintln!("  {} (Dmg)", dmg.display());
                        all_outputs.push(cargo_packager::PackageOutput::new(
                            PackageFormat::Dmg,
                            vec![dmg],
                        ));
                        remove_app_bundle(path)?;
                    }
                }
            }
        }
        if !make_dmg {
            all_outputs.extend(outputs);
        }
    }

    print_summary(&all_outputs, platform);
    Ok(())
}

fn platform_label(platform: &TargetPlatform) -> &'static str {
    match platform {
        TargetPlatform::MacOs => "macos",
        TargetPlatform::Windows => "windows",
    }
}

pub fn open_app_bundle(release: bool, package: &str) -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (release, package);
        bail!("run-app-macos 仅支持 macOS");
    }

    #[cfg(target_os = "macos")]
    {
        let out_dir = PathBuf::from("target/packager").join(if release {
            "release"
        } else {
            "debug"
        });
        package_macos(PackageOptions {
            release,
            app_only: true,
            apps: vec![package.into()],
            out_dir: Some(out_dir.clone()),
        })?;

        let manifest = packager_config::load_manifest(&workspace_root())?;
        let app_spec = manifest
            .apps
            .iter()
            .find(|a| a.name == package)
            .with_context(|| format!("Packager.toml 缺少 {package}"))?;
        let app = workspace_root()
            .join(out_dir)
            .join(format!("{}.app", app_spec.product_name));

        if !app.is_dir() {
            bail!("未找到 {}", app.display());
        }

        eprintln!("==> open {}", app.display());
        Command::new("open")
            .arg(&app)
            .status()
            .context("open failed")?;
        Ok(())
    }
}

fn build_apps(root: &Path, release: bool, apps: &[&AppSpec]) -> Result<()> {
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    for app in apps {
        args.extend(["-p", &app.cargo_package, "--bin", &app.bin]);
    }
    eprintln!("==> cargo {}", args.join(" "));
    let status = run_cargo(root, &args)?;
    if !status.success() {
        bail!("cargo build failed");
    }
    Ok(())
}

fn print_summary(outputs: &[cargo_packager::PackageOutput], platform: TargetPlatform) {
    let mut paths: Vec<_> = outputs.iter().flat_map(|o| o.paths.iter()).collect();
    paths.sort_by_key(|p| p.display().to_string());
    paths.dedup_by_key(|p| p.display().to_string());

    if paths.is_empty() {
        return;
    }

    eprintln!();
    eprintln!("==> 产物:");
    for path in &paths {
        eprintln!("  {}", path.display());
    }

    if matches!(platform, TargetPlatform::MacOs) {
        if let Some(dmg) = outputs
            .iter()
            .find(|o| o.format == PackageFormat::Dmg)
            .and_then(|o| o.paths.first())
        {
            eprintln!();
            eprintln!("安装：打开 {}", dmg.display());
        } else if let Some(app) = outputs
            .iter()
            .find(|o| o.format == PackageFormat::App)
            .and_then(|o| o.paths.first())
        {
            eprintln!();
            eprintln!("运行 GUI:");
            eprintln!("  open \"{}\"", app.display());
        }
    }
}

fn remove_app_bundle(app: &Path) -> Result<()> {
    if app.is_dir() {
        std::fs::remove_dir_all(app)
            .with_context(|| format!("remove intermediate {}", app.display()))?;
    }
    Ok(())
}

fn verify_app_icon(app: &Path) -> Result<()> {
    let icon = app.join("Contents/Resources/icon.icns");
    if icon.is_file() {
        Ok(())
    } else {
        bail!(
            "未找到应用图标 {}（检查 Packager.toml icons 路径）",
            icon.display()
        )
    }
}

#[cfg(target_os = "macos")]
fn adhoc_sign(app: &Path) {
    let Ok(status) = Command::new("codesign")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
    else {
        return;
    };
    if !status.success() {
        return;
    }

    eprintln!("    ad-hoc 签名 {}", app.display());
    match Command::new("codesign")
        .args(["--force", "--deep", "--sign", "-"])
        .arg(app)
        .output()
    {
        Ok(output) if output.status.success() => {}
        Ok(_) | Err(_) => eprintln!("    warning: codesign 失败，可手动运行或忽略"),
    }
}
