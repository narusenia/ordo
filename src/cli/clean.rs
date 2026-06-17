use crate::core::manifest::Manifest;
use crate::util::style;
use miette::{IntoDiagnostic, Result};
use std::fs;
use std::path::Path;

pub fn run(cache: bool) -> Result<()> {
    let cwd = std::env::current_dir().into_diagnostic()?;
    let manifest_path = cwd.join("Ordo.toml");

    let target = cwd.join("target");
    if target.exists() {
        let size = dir_size(&target);
        fs::remove_dir_all(&target).into_diagnostic()?;
        let size_str = format_size(size);
        style::success("Removed", &format!("target/ ({size_str} freed)"));
    } else {
        style::skip("Nothing to clean", "target/");
    }

    if manifest_path.exists() {
        let lock_path = cwd.join("Ordo.lock");
        if lock_path.exists() {
            fs::remove_file(&lock_path).into_diagnostic()?;
            style::success("Removed", "Ordo.lock");
        }

        if let Ok(manifest) = Manifest::load(&manifest_path)
            && manifest.is_workspace()
        {
            clean_workspace_member_artifacts(&cwd);
        }
    }

    if cache {
        clear_external_cache();
    }

    Ok(())
}

fn clean_workspace_member_artifacts(root: &Path) {
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
        style::success(
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

fn clear_external_cache() {
    if let Ok(status) = std::process::Command::new("sccache")
        .arg("--stop-server")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        && status.success()
    {
        style::success("Stopped", "sccache server");
        return;
    }

    if let Ok(status) = std::process::Command::new("ccache")
        .arg("-C")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        && status.success()
    {
        style::success("Cleared", "ccache");
    }
}
