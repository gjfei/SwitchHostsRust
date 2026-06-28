use std::process::ExitStatus;

use anyhow::{Result, bail};

use crate::apps;
use crate::util::{ensure_cargo_watch, run_cargo, workspace_root};

pub fn dev(app: &str, extra_args: &[String]) -> Result<ExitStatus> {
    dev_run(&apps::resolve(app)?.package, extra_args)
}

pub fn dev_watch(app: &str, extra_args: &[String]) -> Result<()> {
    dev_watch_pkg(&apps::resolve(app)?.package, extra_args)
}

fn dev_run(package: &str, extra_args: &[String]) -> Result<ExitStatus> {
    let root = workspace_root();
    let mut args = vec!["run", "-p", package];
    for arg in extra_args {
        args.push(arg.as_str());
    }
    run_cargo(&root, &args)
}

fn dev_watch_pkg(package: &str, extra_args: &[String]) -> Result<()> {
    ensure_cargo_watch()?;
    let root = workspace_root();
    let watch_cmd = format!("run -p {package}");
    let mut args = vec!["watch", "-x", watch_cmd.as_str()];
    for arg in extra_args {
        args.push(arg.as_str());
    }
    let status = run_cargo(&root, &args)?;
    if !status.success() {
        bail!("cargo watch failed");
    }
    Ok(())
}
