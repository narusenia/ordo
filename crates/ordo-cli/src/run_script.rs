use super::context::Context;
use miette::{IntoDiagnostic, Result, bail};
use ordo_core::manifest::Manifest;
use std::process::Command;

pub fn run(name: &str, ctx: &Context) -> Result<()> {
    let project_root = std::env::current_dir().into_diagnostic()?;
    let manifest_path = project_root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", project_root.display());
    }

    let manifest = Manifest::load(&manifest_path)?;

    if manifest.scripts.is_empty() {
        bail!("no [scripts] section defined in Ordo.toml");
    }

    let script = manifest.scripts.get(name).ok_or_else(|| {
        let available: Vec<&str> = manifest.scripts.keys().map(|s| s.as_str()).collect();
        miette::miette!(
            "unknown script '{}'; available: {}",
            name,
            available.join(", ")
        )
    })?;

    ctx.style.run_icon("Running", &format!("script `{name}`"));

    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };
    let shell_flag = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };

    let status = Command::new(shell)
        .arg(shell_flag)
        .arg(script)
        .current_dir(&project_root)
        .status()
        .into_diagnostic()?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        bail!("script '{name}' exited with code {code}");
    }

    Ok(())
}
