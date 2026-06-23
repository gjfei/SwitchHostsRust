use std::process::ExitStatus;

use anyhow::{Result, bail};

use crate::util::{ensure_cargo_watch, run_cargo, workspace_root};

pub fn dev_gui(extra_args: &[String]) -> Result<ExitStatus> {
    let root = workspace_root();
    let mut args = vec!["run", "-p", "egui-app"];
    for arg in extra_args {
        args.push(arg.as_str());
    }
    run_cargo(&root, &args)
}

pub fn dev_gui_watch(extra_args: &[String]) -> Result<()> {
    ensure_cargo_watch()?;
    let root = workspace_root();
    let mut args = vec!["watch", "-x", "run -p egui-app"];
    for arg in extra_args {
        args.push(arg.as_str());
    }
    let status = run_cargo(&root, &args)?;
    if !status.success() {
        bail!("cargo watch failed");
    }
    Ok(())
}
