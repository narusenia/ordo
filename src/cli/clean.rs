use super::context::Context;
use crate::core::manifest::Manifest;
use miette::{IntoDiagnostic, Result};
use std::fs;
use std::path::Path;

pub fn run(cache: bool, package: Option<&str>, ctx: &Context) -> Result<()> {
    let cwd = std::env::current_dir().into_diagnostic()?;
    let manifest_path = cwd.join("Ordo.toml");

    if let Some(pkg_name) = package {
        return clean_package(&cwd, pkg_name, cache, ctx);
    }

    let target = cwd.join("target");
    if target.exists() {
        let size = dir_size(&target);
        fs::remove_dir_all(&target).into_diagnostic()?;
        let size_str = format_size(size);
        ctx.style
            .success("Removed", &format!("target/ ({size_str} freed)"));
    } else {
        ctx.style.skip("Nothing to clean", "target/");
    }

    if manifest_path.exists() {
        let lock_path = cwd.join("Ordo.lock");
        if lock_path.exists() {
            fs::remove_file(&lock_path).into_diagnostic()?;
            ctx.style.success("Removed", "Ordo.lock");
        }

        if let Ok(manifest) = Manifest::load(&manifest_path)
            && manifest.is_workspace()
        {
            clean_workspace_member_artifacts(&cwd, ctx);
        }
    }

    if cache {
        clear_external_cache(ctx);
    }

    Ok(())
}

fn clean_package(root: &Path, pkg_name: &str, cache: bool, ctx: &Context) -> Result<()> {
    use crate::core::workspace::Workspace;
    use miette::bail;

    let manifest_path = root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", root.display());
    }
    let manifest = Manifest::load(&manifest_path)?;
    if !manifest.is_workspace() {
        bail!("--package can only be used in a workspace");
    }

    let ws = Workspace::load(root)?;
    let member = ws.find_member(pkg_name).ok_or_else(|| {
        let available = ws.member_names().join(", ");
        miette::miette!(
            "package '{}' not found in workspace; available: {}",
            pkg_name,
            available
        )
    })?;

    let target = root.join("target");
    let mut cleaned = 0u64;
    for profile in &["debug", "release"] {
        let pkg_dir = target.join(profile).join(pkg_name);
        if pkg_dir.exists() {
            cleaned += dir_size(&pkg_dir);
            fs::remove_dir_all(&pkg_dir).into_diagnostic()?;
        }
    }

    let member_cc = root.join(&member.dir).join("compile_commands.json");
    if member_cc.exists() {
        let _ = fs::remove_file(&member_cc);
    }

    if cleaned > 0 {
        ctx.style.success(
            "Removed",
            &format!("{pkg_name} artifacts ({} freed)", format_size(cleaned)),
        );
    } else {
        ctx.style.skip("Nothing to clean", pkg_name);
    }

    if cache {
        clear_external_cache(ctx);
    }

    Ok(())
}

fn clean_workspace_member_artifacts(root: &Path, ctx: &Context) {
    use crate::core::workspace::Workspace;

    let Ok(ws) = Workspace::load(root) else {
        return;
    };

    let mut cleaned = 0u64;
    for member in &ws.members {
        let member_target = member.dir.join("target");
        if member_target.exists() {
            cleaned += dir_size(&member_target);
            let _ = fs::remove_dir_all(&member_target);
        }
        let member_lock = member.dir.join("Ordo.lock");
        if member_lock.exists() {
            let _ = fs::remove_file(&member_lock);
        }
        let member_cc = member.dir.join("compile_commands.json");
        if member_cc.exists() {
            let _ = fs::remove_file(&member_cc);
        }
    }

    if cleaned > 0 {
        ctx.style.success(
            "Removed",
            &format!("member artifacts ({} freed)", format_size(cleaned)),
        );
    }
}

fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(meta) = p.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn clear_external_cache(ctx: &Context) {
    if let Ok(status) = std::process::Command::new("sccache")
        .arg("--stop-server")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        && status.success()
    {
        ctx.style.success("Stopped", "sccache server");
        return;
    }

    if let Ok(status) = std::process::Command::new("ccache")
        .arg("-C")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        && status.success()
    {
        ctx.style.success("Cleared", "ccache");
    }
}
