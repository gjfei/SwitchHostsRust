//! Workspace tasks for `cargo package-macos` / `cargo package-dmg` aliases.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("xtask crate lives in crates/xtask")
        .to_path_buf()
}

fn run_package_macos(with_dmg: bool) -> ExitCode {
    let script = workspace_root().join("scripts/package-macos.sh");
    let mut cmd = Command::new("bash");
    cmd.arg(script);
    if with_dmg {
        cmd.arg("--dmg");
    }
    match cmd.status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(_) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("error: failed to run package-macos.sh: {e}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        Some("package-dmg") => run_package_macos(true),
        Some("package-macos") => run_package_macos(false),
        Some(other) => {
            eprintln!("error: unknown xtask `{other}`");
            eprintln!("usage: cargo package-macos | cargo package-dmg");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("usage: cargo package-macos | cargo package-dmg");
            ExitCode::FAILURE
        }
    }
}
