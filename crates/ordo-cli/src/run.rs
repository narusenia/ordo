use super::context::Context;
use crate::build::{BuildOptions, BuildResult};
use miette::{IntoDiagnostic, Result, bail};
use ordo_core::manifest::{Manifest, PackageType};
use ordo_core::workspace::Workspace;
use std::process::Command;

#[allow(clippy::too_many_arguments)]
pub fn run(
    args: &[String],
    release: bool,
    package: Option<&str>,
    features: &[String],
    no_default_features: bool,
    all_features: bool,
    ctx: &Context,
) -> Result<()> {
    let cwd = std::env::current_dir().into_diagnostic()?;
    let manifest_path = cwd.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in current directory");
    }

    let manifest = Manifest::load(&manifest_path)?;

    if manifest.is_workspace() {
        return run_workspace(
            args,
            release,
            package,
            features,
            no_default_features,
            all_features,
            &cwd,
            ctx,
        );
    }

    if package.is_some() {
        bail!("-p/--package is only valid in a workspace");
    }

    run_single(
        args,
        release,
        None,
        features,
        no_default_features,
        all_features,
        ctx,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_workspace(
    args: &[String],
    release: bool,
    package: Option<&str>,
    features: &[String],
    no_default_features: bool,
    all_features: bool,
    root_dir: &std::path::Path,
    ctx: &Context,
) -> Result<()> {
    let ws = Workspace::load(root_dir)?;

    let target_name = match package {
        Some(name) => {
            if ws.find_member(name).is_none() {
                let available = ws.member_names().join(", ");
                bail!(
                    "package '{}' not found in workspace; available members: {}",
                    name,
                    available
                );
            }
            name.to_string()
        }
        None => {
            let executables: Vec<&str> = ws
                .members
                .iter()
                .filter(|m| {
                    m.manifest
                        .package
                        .as_ref()
                        .is_some_and(|p| p.package_type == PackageType::Executable)
                })
                .map(|m| m.name.as_str())
                .collect();

            match executables.len() {
                0 => bail!("no executable members in workspace; use -p to specify a member"),
                1 => executables[0].to_string(),
                _ => bail!(
                    "multiple executable members in workspace: {}; use -p to specify one",
                    executables.join(", ")
                ),
            }
        }
    };

    let member = ws.find_member(&target_name).unwrap();
    if member.manifest.package().package_type != PackageType::Executable {
        bail!(
            "cannot run '{}': it is a {}, not an executable",
            target_name,
            member.manifest.package().package_type
        );
    }

    run_single(
        args,
        release,
        Some(&target_name),
        features,
        no_default_features,
        all_features,
        ctx,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_single(
    args: &[String],
    release: bool,
    package: Option<&str>,
    features: &[String],
    no_default_features: bool,
    all_features: bool,
    ctx: &Context,
) -> Result<()> {
    let build_opts = BuildOptions {
        release,
        profile: None,
        jobs: None,
        target: None,
        no_cache: false,
        features: features.to_vec(),
        no_default_features,
        all_features,
        locked: false,
        frozen: false,
        verbose: 0,
        package: package.map(|s| s.to_string()),
    };

    let BuildResult {
        output_path,
        package_type,
    } = crate::build::run(&build_opts, ctx)?;

    if package_type != PackageType::Executable {
        bail!(
            "cannot run a {} project — only executables can be run",
            package_type
        );
    }

    if !output_path.exists() {
        bail!("built binary not found at {}", output_path.display());
    }

    ctx.style
        .run_icon("Running", &format!("{}", output_path.display()));

    let status = Command::new(&output_path)
        .args(args)
        .status()
        .into_diagnostic()?;

    std::process::exit(status.code().unwrap_or(1));
}
