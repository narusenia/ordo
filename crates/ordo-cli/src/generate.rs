use super::context::Context;
use crate::GenerateTarget;
use miette::{IntoDiagnostic, Result, bail};
use ordo_core::manifest::Manifest;
use std::fs;
use std::path::Path;

pub fn run(target: &GenerateTarget, ctx: &Context) -> Result<()> {
    match target {
        GenerateTarget::ClangFormat => clang_format(ctx),
        _ => {
            eprintln!("ordo generate: not yet implemented for this target");
            Ok(())
        }
    }
}

/// Write the style `ordo fmt` would otherwise pass to clang-format into the
/// project, so editors and other tools pick up the same rules.
fn clang_format(ctx: &Context) -> Result<()> {
    let project_root = std::env::current_dir().into_diagnostic()?;
    let path = project_root.join(".clang-format");
    if path.exists() {
        bail!(
            "{} already exists — edit it instead, or delete it first",
            path.display()
        );
    }

    fs::write(&path, style_content(&project_root)?).into_diagnostic()?;
    ctx.style
        .success("Generated", &display_name(&path, &project_root));

    Ok(())
}

fn style_content(project_root: &Path) -> Result<String> {
    let manifest_path = project_root.join("Ordo.toml");
    let style = if manifest_path.exists() {
        Manifest::load(&manifest_path)?.fmt.style
    } else {
        None
    };

    Ok(style.unwrap_or_else(|| crate::fmt::DEFAULT_STYLE.to_string()))
}

fn display_name(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
