mod backend;
mod cli;
mod core;
mod error;
mod util;

use clap::Parser;
use cli::context::Context;
use cli::{Cli, ColorMode, Command, ProjectLang};
use miette::{IntoDiagnostic, Result};
use tracing_subscriber::EnvFilter;
use util::paths::OrdoPaths;

fn main() -> Result<()> {
    let cli = Cli::parse();

    init_tracing(cli.verbose);
    init_color(cli.color);

    let _paths = OrdoPaths::resolve();
    let ctx = Context::resolve(cli.style, cli.verbose, cli.color);

    match cli.command {
        Command::New {
            name,
            lib,
            lang,
            no_git,
        } => {
            let cwd = std::env::current_dir().into_diagnostic()?;
            match name {
                Some(name) => {
                    let lang = lang.unwrap_or(ProjectLang::Cpp);
                    cli::new::run(&cwd, &name, lib, lang, no_git, &ctx)?;
                }
                None => {
                    cli::new::run_interactive(&cwd, no_git, &ctx)?;
                }
            }
        }
        Command::Init => {
            let cwd = std::env::current_dir().into_diagnostic()?;
            cli::init::run(&cwd, &ctx)?;
        }
        Command::Build {
            release,
            profile,
            jobs,
            target,
            no_cache,
            features,
            no_default_features,
            all_features,
            locked,
            frozen,
            package,
        } => {
            let opts = cli::build::BuildOptions {
                release,
                profile,
                jobs,
                target,
                no_cache,
                features,
                no_default_features,
                all_features,
                locked,
                frozen,
                verbose: cli.verbose,
                package,
            };
            cli::build::run(&opts, &ctx)?;
        }
        Command::Run {
            args,
            release,
            package,
            features,
            no_default_features,
            all_features,
        } => {
            cli::run::run(
                &args,
                release,
                package.as_deref(),
                &features,
                no_default_features,
                all_features,
                &ctx,
            )?;
        }
        Command::Test {
            filter,
            jobs,
            release,
            profile,
            features,
            no_default_features,
            all_features,
            package,
        } => {
            let opts = cli::test::TestOptions {
                filter,
                jobs,
                release,
                profile,
                features,
                no_default_features,
                all_features,
                package,
                verbose: cli.verbose,
            };
            cli::test::run(&opts, &ctx)?;
        }
        Command::Check => {
            cli::check::run(&ctx)?;
        }
        Command::Clean { cache, .. } => {
            cli::clean::run(cache, &ctx)?;
        }
        Command::Fmt { check } => {
            cli::fmt::run(check, &ctx)?;
        }
        Command::Lint { fix } => {
            cli::lint::run(fix, &ctx)?;
        }
        Command::Watch { .. } => {
            eprintln!("ordo watch: not yet implemented");
        }
        Command::Add {
            specs,
            provider,
            no_verify,
            with,
            alias,
            link_name,
        } => {
            cli::add::run(
                &specs,
                provider.as_deref(),
                no_verify,
                with.as_deref(),
                alias.as_deref(),
                link_name.as_deref(),
                &ctx,
            )?;
        }
        Command::Update { name } => {
            let cwd = std::env::current_dir().into_diagnostic()?;
            cli::update::run(&cwd, name.as_deref(), &ctx)?;
        }
        Command::Tree { .. } => {
            let cwd = std::env::current_dir().into_diagnostic()?;
            cli::tree::run(&cwd, &ctx)?;
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
        Command::RunScript { name } => {
            cli::run_script::run(&name, &ctx)?;
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
