pub mod config;
pub mod error;
pub mod lockfile;
pub mod manifest;
pub mod paths;
pub mod resolver;
pub mod tester;
pub mod workspace;

/// CLI output style mode.
///
/// Defined in `ordo-core` so that `manifest::CliConfig` can reference it
/// without introducing a circular dependency on the CLI crate.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StyleMode {
    #[default]
    Default,
    Minimal,
    #[value(name = "cargo-like")]
    CargoLike,
}
