use super::context::Context;
use crate::core::manifest::Manifest;
use miette::{IntoDiagnostic, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEFAULT_STYLE: &str = "BasedOnStyle: LLVM
IndentWidth: 4
ColumnLimit: 100
";

pub fn run(check: bool, ctx: &Context) -> Result<()> {
    let project_root = std::env::current_dir().into_diagnostic()?;
    let manifest_path = project_root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", project_root.display());
    }

    let manifest = Manifest::load(&manifest_path)?;
    let fmt_config = &manifest.fmt;

    let tool = resolve_tool(fmt_config.tool.as_deref())?;

    let sources = discover_formattable_sources(&project_root)?;
    if sources.is_empty() {
        ctx.style.warn("Warning", "no source files found to format");
        return Ok(());
    }

    let _style_file = ensure_style_file(&project_root, fmt_config.style.as_deref())?;

    if check {
        run_check(&tool, &sources, ctx)
    } else {
        run_format(&tool, &sources, ctx)
    }
}

fn run_format(tool: &str, sources: &[PathBuf], ctx: &Context) -> Result<()> {
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

    ctx.style
        .success("Formatted", &format!("{} files", sources.len()));
    Ok(())
}

fn run_check(tool: &str, sources: &[PathBuf], ctx: &Context) -> Result<()> {
    let mut cmd = Command::new(tool);
    cmd.arg("--dry-run").arg("--Werror");
    for src in sources {
        cmd.arg(src);
    }

    let output = cmd.output().into_diagnostic()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut diff_count = 0u32;
        for line in stderr.lines() {
            if line.contains("warning:") || line.contains("error:") {
                diff_count += 1;
                eprintln!("  {line}");
            }
        }
        if diff_count == 0 {
            eprintln!("  {stderr}");
        }
        ctx.style.error(
            "Check failed",
            &format!("{diff_count} file(s) need formatting"),
        );
        bail!("formatting check failed");
    }

    ctx.style
        .success("Check passed", &format!("{} files", sources.len()));
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
