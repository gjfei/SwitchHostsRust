use std::process::Command;

use super::error::ApplyError;

pub fn run_after_apply(cmd: &str) -> Result<(), ApplyError> {
    if cmd.trim().is_empty() {
        return Ok(());
    }
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", cmd]).status()
    } else {
        Command::new("sh").args(["-c", cmd]).status()
    }
    .map_err(ApplyError::Io)?;
    if !status.success() {
        return Err(ApplyError::Elevation(format!(
            "post-apply command exited with {status}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_cmd_ok() {
        run_after_apply("").unwrap();
    }
}
