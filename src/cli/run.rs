use crate::cli::build::{BuildOptions, BuildResult};
use crate::core::manifest::PackageType;
use crate::util::style;
use miette::{IntoDiagnostic, Result, bail};
use std::process::Command;

pub fn run(args: &[String], release: bool) -> Result<()> {
    let build_opts = BuildOptions {
        release,
        profile: None,
        jobs: None,
        target: None,
        no_cache: false,
        locked: false,
        frozen: false,
        verbose: 0,
        package: None,
    };

    let BuildResult {
        output_path,
        package_type,
    } = crate::cli::build::run(&build_opts)?;

    if package_type != PackageType::Executable {
        bail!(
            "cannot run a {} project — only executables can be run",
            package_type
        );
    }

    if !output_path.exists() {
        bail!("built binary not found at {}", output_path.display());
    }

    style::run_icon("Running", &format!("{}", output_path.display()));

    let status = Command::new(&output_path)
        .args(args)
        .status()
        .into_diagnostic()?;

    std::process::exit(status.code().unwrap_or(1));
}
