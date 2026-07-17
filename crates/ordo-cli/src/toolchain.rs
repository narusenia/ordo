use super::context::Context;
use crate::ToolchainCommand;
use miette::{Result, bail};
use ordo_arsenal::{Arsenal, Tool};
use ordo_core::manifest::Manifest;

pub fn run(command: &ToolchainCommand, ctx: &Context) -> Result<()> {
    match command {
        ToolchainCommand::Install { tool, version } => run_install(tool, version.as_deref(), ctx),
        ToolchainCommand::List => run_list(ctx),
        ToolchainCommand::Remove { tool, version } => run_remove(tool, version, ctx),
        ToolchainCommand::Which { tool } => run_which(tool, ctx),
        ToolchainCommand::Clean => run_clean(ctx),
        ToolchainCommand::Update { tool } => run_update(tool.as_deref(), ctx),
    }
}

fn parse_tool(name: &str) -> Result<Tool> {
    Tool::parse(name)
        .ok_or_else(|| miette::miette!("unknown tool '{}'. Supported tools: ninja", name))
}

fn run_install(tool_name: &str, version: Option<&str>, ctx: &Context) -> Result<()> {
    let tool = parse_tool(tool_name)?;
    let arsenal = Arsenal::new();

    let sw = ctx.style.create_spinner_with_detail(&format!(
        "Installing {}{}...",
        tool.name(),
        version.map(|v| format!(" v{v}")).unwrap_or_default()
    ));
    let on_progress = |msg: &str| {
        sw.set_detail(msg);
    };

    match arsenal.install(tool, version, &on_progress) {
        Ok(installed) => {
            sw.finish_success(
                "Installed",
                &format!(
                    "{} v{} ({})",
                    tool.name(),
                    installed.version,
                    installed.path.display()
                ),
            );
            Ok(())
        }
        Err(e) => {
            sw.finish_error("Failed", &format!("{} install", tool.name()));
            Err(e)
        }
    }
}

fn run_list(ctx: &Context) -> Result<()> {
    let arsenal = Arsenal::new();
    let installed = arsenal.list();

    if installed.is_empty() {
        ctx.style.warn("Info", "No tools installed via Arsenal");
        return Ok(());
    }

    ctx.style.header("Installed tools");
    for item in &installed {
        ctx.style.success(
            item.tool.name(),
            &format!("v{} ({})", item.version, item.path.display()),
        );
    }

    Ok(())
}

fn run_remove(tool_name: &str, version: &str, ctx: &Context) -> Result<()> {
    let tool = parse_tool(tool_name)?;
    let arsenal = Arsenal::new();
    arsenal.remove(tool, version)?;
    ctx.style
        .success("Removed", &format!("{} v{}", tool.name(), version));
    Ok(())
}

fn run_which(tool_name: &str, ctx: &Context) -> Result<()> {
    let tool = parse_tool(tool_name)?;

    // Check Ordo.toml for version pin
    let version_req = load_manifest_tool_version(tool);

    let arsenal = Arsenal::new();
    if let Some(path) = arsenal.which(tool, version_req.as_deref()) {
        ctx.style.success(tool.name(), &path.display().to_string());
        return Ok(());
    }

    // Fall back to system PATH
    if let Some(path) = ordo_arsenal::resolve_tool_path(tool, version_req.as_deref()) {
        ctx.style
            .success(tool.name(), &format!("{} (system)", path.display()));
        return Ok(());
    }

    bail!(
        "{} not found (not installed via Arsenal and not on PATH)",
        tool.name()
    );
}

fn run_clean(ctx: &Context) -> Result<()> {
    let arsenal = Arsenal::new();
    let count = arsenal.clean()?;
    if count == 0 {
        ctx.style.warn("Info", "Nothing to clean");
    } else {
        ctx.style
            .success("Cleaned", &format!("{count} tool(s) removed"));
    }
    Ok(())
}

fn run_update(tool_name: Option<&str>, ctx: &Context) -> Result<()> {
    let tools: Vec<Tool> = match tool_name {
        Some(name) => vec![parse_tool(name)?],
        None => vec![Tool::Ninja],
    };

    let arsenal = Arsenal::new();

    for tool in tools {
        let sw = ctx
            .style
            .create_spinner_with_detail(&format!("Updating {}...", tool.name()));
        let on_progress = |msg: &str| {
            sw.set_detail(msg);
        };

        match arsenal.update(tool, &on_progress) {
            Ok(installed) => {
                sw.finish_success(
                    "Updated",
                    &format!("{} v{}", tool.name(), installed.version),
                );
            }
            Err(e) => {
                sw.finish_error("Failed", &format!("{} update", tool.name()));
                return Err(e);
            }
        }
    }

    Ok(())
}

/// Load the tool version pin from Ordo.toml if present.
fn load_manifest_tool_version(tool: Tool) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let manifest_path = cwd.join("Ordo.toml");
    if !manifest_path.exists() {
        return None;
    }
    let manifest = Manifest::load(&manifest_path).ok()?;
    match tool {
        Tool::Ninja => manifest.toolchain.ninja,
    }
}
