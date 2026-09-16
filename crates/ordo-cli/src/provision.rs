use super::context::Context;
use miette::{Result, bail};
use ordo_arsenal::Tool;
use std::path::PathBuf;

/// Locate a tool, offering to install it through Arsenal when it is missing.
///
/// The prompt is skipped (and the install proceeds) when `CI` or `ORDO_YES`
/// is set, so unattended runs are not blocked on a question.
pub(crate) fn resolve_or_provision(
    tool: Tool,
    version_req: Option<&str>,
    ui: &Context,
) -> Result<PathBuf> {
    if let Some(path) = ordo_arsenal::resolve_tool_path(tool, version_req) {
        return Ok(path);
    }

    ui.style.warn("Warning", &missing_reason(tool, version_req));

    if !confirm(tool) {
        bail!("{}", install_hint(tool));
    }

    let arsenal = ordo_arsenal::Arsenal::new();
    let installed = arsenal.install(tool, version_req, &|msg| {
        ui.style.success("Arsenal", msg);
    })?;

    ui.style.success(
        "Installed",
        &format!("{} v{}", tool.name(), installed.version),
    );

    Ok(installed.path)
}

fn confirm(tool: Tool) -> bool {
    if std::env::var("ORDO_YES").is_ok() || std::env::var("CI").is_ok() {
        return true;
    }

    use promptuity::prompts::Confirm;
    use promptuity::themes::MinimalTheme;
    use promptuity::{Promptuity, Term};

    let mut term = Term::default();
    let mut theme = MinimalTheme::default();
    let mut p = Promptuity::new(&mut term, &mut theme);
    p.prompt(Confirm::new(format!("Install {} via Arsenal?", tool.name())).with_default(true))
        .unwrap_or(false)
}

fn missing_reason(tool: Tool, version_req: Option<&str>) -> String {
    let what = match tool {
        Tool::Ninja => "required for the Ninja build engine",
        Tool::ClangFormat => "required for ordo fmt",
    };
    match version_req {
        Some(req) => format!("{} v{req} not found — {what}", tool.name()),
        None => format!("{} not found — {what}", tool.name()),
    }
}

fn install_hint(tool: Tool) -> String {
    let alternative = match tool {
        Tool::Ninja => " or switch to `[build] engine = \"faber\"`",
        Tool::ClangFormat => " or point `[fmt] tool` at your own binary",
    };
    format!(
        "{} not found — install it with `ordo toolchain install {}`{alternative}",
        tool.name(),
        tool.name()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_reason_mentions_the_pin() {
        let msg = missing_reason(Tool::ClangFormat, Some("23"));
        assert!(msg.contains("clang-format v23"), "{msg}");
    }

    #[test]
    fn install_hint_names_the_tool_command() {
        assert!(install_hint(Tool::Ninja).contains("ordo toolchain install ninja"));
        assert!(install_hint(Tool::ClangFormat).contains("ordo toolchain install clang-format"));
    }
}
