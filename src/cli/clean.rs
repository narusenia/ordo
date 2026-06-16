use crate::util::style;
use miette::{IntoDiagnostic, Result};
use std::fs;
use std::path::Path;

pub fn run(cache: bool) -> Result<()> {
    let target = Path::new("target");
    if target.exists() {
        fs::remove_dir_all(target).into_diagnostic()?;
        style::status("Removed", "target/");
    } else {
        style::status_warn("Skipped", "target/ does not exist");
    }

    if cache {
        clear_external_cache();
    }

    Ok(())
}

fn clear_external_cache() {
    if let Ok(status) = std::process::Command::new("sccache")
        .arg("--stop-server")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        && status.success()
    {
        eprintln!("Stopped sccache server");
        return;
    }

    if let Ok(status) = std::process::Command::new("ccache")
        .arg("-C")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        && status.success()
    {
        eprintln!("Cleared ccache");
    }
}
