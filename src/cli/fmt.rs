use super::context::Context;
use crate::core::manifest::Manifest;
use crate::core::workspace::Workspace;
use miette::{IntoDiagnostic, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_STYLE: &str = "BasedOnStyle: LLVM
IndentWidth: 4
ColumnLimit: 100
";

pub fn run(check: bool, package: Option<&str>, ctx: &Context) -> Result<()> {
    let project_root = std::env::current_dir().into_diagnostic()?;
    let manifest_path = project_root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", project_root.display());
    }

    let manifest = Manifest::load(&manifest_path)?;

    if manifest.is_workspace() {
        return run_workspace_fmt(check, package, &project_root, ctx);
    }

    run_single_fmt(check, &project_root, &manifest, ctx)
}

fn run_workspace_fmt(
    check: bool,
    package: Option<&str>,
    root_dir: &Path,
    ctx: &Context,
) -> Result<()> {
    let ws = Workspace::load(root_dir)?;

    let members: Vec<String> = if let Some(target) = package {
        if ws.find_member(target).is_none() {
            let available = ws.member_names().join(", ");
            bail!(
                "package '{}' not found in workspace; available members: {}",
                target,
                available
            );
        }
        vec![target.to_string()]
    } else {
        ws.member_names().into_iter().map(String::from).collect()
    };

    let mut total_fail = 0u32;
    for name in &members {
        let member = ws.find_member(name).unwrap();
        let member_dir = root_dir.join(&member.dir);
        let member_manifest_path = member_dir.join("Ordo.toml");
        let member_manifest = Manifest::load(&member_manifest_path)?;
        if let Err(e) = run_single_fmt(check, &member_dir, &member_manifest, ctx) {
            ctx.style.error("Failed", &format!("{name}: {e}"));
            total_fail += 1;
        }
    }

    if total_fail > 0 {
        bail!("formatting failed for {total_fail} member(s)");
    }

    Ok(())
}

fn run_single_fmt(
    check: bool,
    project_root: &Path,
    manifest: &Manifest,
    ctx: &Context,
) -> Result<()> {
    let fmt_config = &manifest.fmt;
    let tool = resolve_tool(fmt_config.tool.as_deref())?;

    let sources = discover_formattable_sources(project_root)?;
    if sources.is_empty() {
        return Ok(());
    }

    let _style_file = ensure_style_file(project_root, fmt_config.style.as_deref())?;

    let pkg_name = manifest
        .package
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("project");

    let spinner = ctx.style.create_spinner(&format!(
        "{} {} ({} files)...",
        if check { "Checking" } else { "Formatting" },
        pkg_name,
        sources.len()
    ));

    let result = if check {
        run_check_inner(&tool, &sources)
    } else {
        run_format_inner(&tool, &sources)
    };

    spinner.finish_and_clear();

    match result {
        Ok(()) => {
            let verb = if check { "Check passed" } else { "Formatted" };
            ctx.style
                .success(verb, &format!("{pkg_name} ({} files)", sources.len()));
            Ok(())
        }
        Err(e) => {
            let verb = if check {
                "Check failed"
            } else {
                "Format failed"
            };
            ctx.style.error(verb, pkg_name);
            Err(e)
        }
    }
}

fn run_format_inner(tool: &str, sources: &[PathBuf]) -> Result<()> {
    let mut cmd = Command::new(tool);
    cmd.arg("-i");
    for src in sources {
        cmd.arg(src);
    }

    let output = cmd.output().into_diagnostic()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            eprintln!("  {line}");
        }
        bail!("formatting failed");
    }

    Ok(())
}

fn run_check_inner(tool: &str, sources: &[PathBuf]) -> Result<()> {
    let mut cmd = Command::new(tool);
    cmd.arg("--dry-run").arg("--Werror");
    for src in sources {
        cmd.arg(src);
    }

    let output = cmd.output().into_diagnostic()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            if line.contains("warning:") || line.contains("error:") {
                eprintln!("  {line}");
            }
        }
        bail!("formatting check failed");
    }

    Ok(())
}

fn ensure_style_file(project_root: &Path, style_override: Option<&str>) -> Result<Option<PathBuf>> {
    let style_path = project_root.join(".clang-format");
    if style_path.exists() {
        return Ok(None);
    }

    let content = style_override.unwrap_or(DEFAULT_STYLE);
    fs::write(&style_path, content).into_diagnostic()?;
    Ok(Some(style_path))
}

fn discover_formattable_sources(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    for dir_name in &["src", "include", "tests", "test"] {
        let dir = project_root.join(dir_name);
        if dir.exists() {
            collect_sources(&dir, &mut sources)?;
        }
    }
    sources.sort();
    Ok(sources)
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if path.is_dir() {
            collect_sources(&path, out)?;
        } else if is_formattable(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_formattable(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("cpp" | "cc" | "cxx" | "c" | "h" | "hpp" | "hxx")
    )
}

fn resolve_tool(configured: Option<&str>) -> Result<String> {
    if let Some(tool) = configured {
        if Command::new(tool).arg("--version").output().is_ok() {
            return Ok(tool.to_string());
        }
        bail!("{tool} not found on PATH");
    }

    if Command::new("clang-format")
        .arg("--version")
        .output()
        .is_ok()
    {
        return Ok("clang-format".to_string());
    }

    if cfg!(target_os = "macos")
        && let Ok(output) = Command::new("xcrun")
            .args(["--find", "clang-format"])
            .output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    bail!("clang-format not found — install it or set [fmt] tool in Ordo.toml")
}
