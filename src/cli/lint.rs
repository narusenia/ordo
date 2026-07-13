use super::context::Context;
use crate::core::manifest::Manifest;
use crate::core::workspace::Workspace;
use miette::{IntoDiagnostic, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_CHECKS: &str = "\
Checks: >
  -*,
  bugprone-*,
  clang-analyzer-*,
  misc-*,
  modernize-*,
  performance-*,
  readability-*
";

pub fn run(fix: bool, package: Option<&str>, ctx: &Context) -> Result<()> {
    let project_root = std::env::current_dir().into_diagnostic()?;
    let manifest_path = project_root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", project_root.display());
    }

    let manifest = Manifest::load(&manifest_path)?;

    if manifest.is_workspace() {
        return run_workspace_lint(fix, package, &project_root, ctx);
    }

    run_single_lint(fix, &project_root, &manifest, ctx)
}

fn run_workspace_lint(
    fix: bool,
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
        if let Err(e) = run_single_lint(fix, &member_dir, &member_manifest, ctx) {
            ctx.style.error("Failed", &format!("{name}: {e}"));
            total_fail += 1;
        }
    }

    if total_fail > 0 {
        bail!("lint failed for {total_fail} member(s)");
    }

    Ok(())
}

fn run_single_lint(
    fix: bool,
    project_root: &Path,
    manifest: &Manifest,
    ctx: &Context,
) -> Result<()> {
    let lint_config = &manifest.lint;
    let tool = resolve_tool(lint_config.tool.as_deref())?;

    let compile_db = project_root.join("compile_commands.json");
    if !compile_db.exists() {
        ctx.style.warn(
            "Warning",
            "compile_commands.json not found, running build first",
        );
        run_build(project_root)?;
        if !compile_db.exists() {
            bail!("compile_commands.json still not found after build");
        }
    }

    let _config_file = ensure_config_file(project_root, lint_config.config.as_deref())?;

    let sources = discover_lintable_sources(project_root)?;
    if sources.is_empty() {
        return Ok(());
    }

    let pkg_name = manifest
        .package
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("project");

    let spinner = ctx.style.create_spinner(&format!(
        "{} {} ({} files)...",
        if fix { "Fixing" } else { "Linting" },
        pkg_name,
        sources.len()
    ));

    let mut cmd = Command::new(&tool);
    cmd.arg("-p").arg(project_root);
    if fix {
        cmd.arg("--fix").arg("--fix-errors");
    }
    for arg in &lint_config.extra_args {
        cmd.arg(arg);
    }
    for src in &sources {
        cmd.arg(src);
    }

    let output = cmd.output().into_diagnostic()?;
    spinner.finish_and_clear();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    let has_errors = combined.lines().any(|l| l.contains("error:"));
    let has_warnings = combined
        .lines()
        .any(|l| l.contains("warning:") && !l.contains("warnings generated"));

    if !stdout.is_empty() {
        for line in stdout.lines() {
            eprintln!("  {line}");
        }
    }
    if !stderr.is_empty() {
        for line in stderr.lines() {
            if !line.contains("warnings generated") && !line.trim().is_empty() {
                eprintln!("  {line}");
            }
        }
    }

    if has_errors {
        let verb = if fix { "Fix failed" } else { "Lint failed" };
        ctx.style.error(verb, pkg_name);
        bail!("lint found errors");
    }

    if has_warnings {
        let verb = if fix { "Fixed" } else { "Checked" };
        ctx.style.warn(
            verb,
            &format!("{pkg_name} ({} files, warnings emitted)", sources.len()),
        );
        return Ok(());
    }

    let verb = if fix { "Fixed" } else { "Checked" };
    ctx.style
        .success(verb, &format!("{pkg_name} ({} files)", sources.len()));
    Ok(())
}

fn run_build(project_root: &Path) -> Result<()> {
    let status = Command::new("ordo")
        .arg("build")
        .current_dir(project_root)
        .status()
        .into_diagnostic()?;
    if !status.success() {
        bail!("build failed — lint requires a successful build for compile_commands.json");
    }
    Ok(())
}

fn ensure_config_file(
    project_root: &Path,
    config_override: Option<&str>,
) -> Result<Option<PathBuf>> {
    let config_path = project_root.join(".clang-tidy");
    if config_path.exists() {
        return Ok(None);
    }

    let content = config_override.unwrap_or(DEFAULT_CHECKS);
    fs::write(&config_path, content).into_diagnostic()?;
    Ok(Some(config_path))
}

fn discover_lintable_sources(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    for dir_name in &["src", "include"] {
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
        } else if is_lintable(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_lintable(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("cpp" | "cc" | "cxx" | "c")
    )
}

fn resolve_tool(configured: Option<&str>) -> Result<String> {
    if let Some(tool) = configured {
        if Command::new(tool).arg("--version").output().is_ok() {
            return Ok(tool.to_string());
        }
        bail!("{tool} not found on PATH");
    }

    if Command::new("clang-tidy").arg("--version").output().is_ok() {
        return Ok("clang-tidy".to_string());
    }

    if cfg!(target_os = "macos")
        && let Ok(output) = Command::new("xcrun")
            .args(["--find", "clang-tidy"])
            .output()
        && output.status.success()
    {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() {
            return Ok(path);
        }
    }

    bail!("clang-tidy not found — install it or set [lint] tool in Ordo.toml")
}
