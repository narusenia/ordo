mod backend;
mod cli;
mod core;
mod error;
mod util;

use clap::Parser;
use cli::{Cli, ColorMode, Command};
use miette::Result;
use tracing_subscriber::EnvFilter;
use util::paths::OrdoPaths;

fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.verbose);
    init_color(cli.color);

    let _paths = OrdoPaths::resolve();

    match cli.command {
        Command::New { name, lib, no_git } => {
            cli::new::run(&name, lib, no_git)?;
        }
        Command::Init => {
            cli::init::run()?;
        }
        Command::Build {
            release,
            profile,
            jobs,
            target,
            no_cache,
            ..
        } => {
            let opts = cli::build::BuildOptions {
                release,
                profile,
                jobs,
                target,
                no_cache,
            };
            cli::build::run(&opts)?;
        }
        Command::Run { args, release } => {
            cli::run::run(&args, release)?;
        }
        Command::Test { .. } => {
            eprintln!("ordo test: not yet implemented");
        }
        Command::Check => {
            eprintln!("ordo check: not yet implemented");
        }
        Command::Clean { cache } => {
            cli::clean::run(cache)?;
        }
        Command::Fmt { .. } => {
            eprintln!("ordo fmt: not yet implemented");
        }
        Command::Lint { .. } => {
            eprintln!("ordo lint: not yet implemented");
        }
        Command::Watch { .. } => {
            eprintln!("ordo watch: not yet implemented");
        }
        Command::Add { .. } => {
            eprintln!("ordo add: not yet implemented");
        }
        Command::Update { .. } => {
            eprintln!("ordo update: not yet implemented");
        }
        Command::Tree => {
            eprintln!("ordo tree: not yet implemented");
        }
        Command::Install { .. } => {
            eprintln!("ordo install: not yet implemented");
        }
        Command::Package => {
            eprintln!("ordo package: not yet implemented");
        }
        Command::Publish => {
            eprintln!("ordo publish: not yet implemented");
        }
        Command::Import { .. } => {
            eprintln!("ordo import: not yet implemented");
        }
        Command::Generate { .. } => {
            eprintln!("ordo generate: not yet implemented");
        }
        Command::Toolchain { .. } => {
            eprintln!("ordo toolchain: not yet implemented");
        }
        Command::Ci { .. } => {
            eprintln!("ordo ci: not yet implemented");
        }
        Command::Doctor => {
            eprintln!("ordo doctor: not yet implemented");
        }
        Command::Config { .. } => {
            eprintln!("ordo config: not yet implemented");
        }
        Command::RunScript { .. } => {
            eprintln!("ordo run-script: not yet implemented");
        }
        Command::SelfCmd { .. } => {
            eprintln!("ordo self: not yet implemented");
        }
    }

    Ok(())
}

fn init_color(mode: ColorMode) {
    match mode {
        ColorMode::Always => owo_colors::set_override(true),
        ColorMode::Never => owo_colors::set_override(false),
        ColorMode::Auto => {} // owo-colors auto-detects TTY
    }
}

fn init_tracing(verbosity: u8) {
    let filter = match verbosity {
        0 => "ordo=warn",
        1 => "ordo=info",
        2 => "ordo=debug",
        _ => "ordo=trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .without_time()
        .init();
}
