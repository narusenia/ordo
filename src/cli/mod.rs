pub mod add;
pub mod build;
pub mod check;
pub mod clean;
pub mod context;
pub mod fmt;
pub mod init;
pub mod lint;
pub mod new;
pub mod run;
pub mod run_script;
pub mod test;
pub mod tree;
pub mod update;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "ordo",
    version,
    about = "A modern project orchestrator for C and C++"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Control colored output
    #[arg(long, global = true, default_value = "auto", env = "ORDO_COLOR")]
    pub color: ColorMode,

    /// Verbose output (-v for commands, -vv for debug)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Output style
    #[arg(long, global = true, default_value = "default", env = "ORDO_CLI_STYLE")]
    pub style: StyleMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StyleMode {
    #[default]
    Default,
    Minimal,
    #[value(name = "cargo-like")]
    CargoLike,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProjectLang {
    C,
    #[default]
    Cpp,
}

#[derive(Subcommand)]
pub enum Command {
    /// Create a new project
    New {
        /// Project name (interactive if omitted)
        name: Option<String>,

        /// Create a library project
        #[arg(long)]
        lib: bool,

        /// Project language
        #[arg(long)]
        lang: Option<ProjectLang>,

        /// Skip git initialization
        #[arg(long)]
        no_git: bool,
    },

    /// Initialize ordo in an existing directory
    Init,

    /// Build the project
    Build {
        /// Build with release profile
        #[arg(long)]
        release: bool,

        /// Build with a named profile
        #[arg(long, conflicts_with = "release")]
        profile: Option<String>,

        /// Number of parallel jobs
        #[arg(short, long)]
        jobs: Option<u32>,

        /// Target triple for cross-compilation
        #[arg(long)]
        target: Option<String>,

        /// Disable build cache
        #[arg(long)]
        no_cache: bool,

        /// Enabled features (comma-separated)
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,

        /// Disable default features
        #[arg(long)]
        no_default_features: bool,

        /// Enable all features
        #[arg(long)]
        all_features: bool,

        /// Error if Ordo.lock is out of sync
        #[arg(long)]
        locked: bool,

        /// Disallow network access
        #[arg(long)]
        frozen: bool,

        /// Build a specific workspace member
        #[arg(short, long)]
        package: Option<String>,
    },

    /// Build and run the project
    Run {
        /// Arguments to pass to the binary
        #[arg(last = true)]
        args: Vec<String>,

        /// Build with release profile
        #[arg(long)]
        release: bool,

        /// Run a specific workspace member
        #[arg(short, long)]
        package: Option<String>,

        /// Enabled features (comma-separated)
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,

        /// Disable default features
        #[arg(long)]
        no_default_features: bool,

        /// Enable all features
        #[arg(long)]
        all_features: bool,
    },

    /// Run tests
    Test {
        /// Filter tests by name
        #[arg(long)]
        filter: Option<String>,

        /// Number of parallel test jobs
        #[arg(short, long)]
        jobs: Option<u32>,

        /// Build with release profile
        #[arg(long)]
        release: bool,

        /// Build with a named profile
        #[arg(long, conflicts_with = "release")]
        profile: Option<String>,

        /// Enabled features (comma-separated)
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,

        /// Disable default features
        #[arg(long)]
        no_default_features: bool,

        /// Enable all features
        #[arg(long)]
        all_features: bool,

        /// Test a specific workspace member
        #[arg(short, long)]
        package: Option<String>,
    },

    /// Syntax check without producing binaries
    Check,

    /// Remove build artifacts
    Clean {
        /// Also clear external build cache
        #[arg(long)]
        cache: bool,

        /// Clean a specific workspace member
        #[arg(short, long)]
        package: Option<String>,
    },

    /// Format source code
    Fmt {
        /// Check formatting without modifying files
        #[arg(long)]
        check: bool,
    },

    /// Run linter
    Lint {
        /// Apply auto-fixes
        #[arg(long)]
        fix: bool,
    },

    /// Watch for changes and re-run a command
    Watch {
        #[command(subcommand)]
        command: WatchCommand,

        /// Send desktop notification on result
        #[arg(long)]
        notify: bool,
    },

    /// Add a dependency (e.g. `ordo add raylib@6 -P vcpkg`, `ordo add vcpkg:raylib@6`, `ordo add fmt glfw raylib`)
    Add {
        /// Package specs: name, name@version, or provider:name@version (multiple allowed)
        #[arg(required = true)]
        specs: Vec<String>,

        /// Provider (pkg-config, system, vcpkg, conan, path, git). Applies to all specs.
        #[arg(short = 'P', long)]
        provider: Option<String>,

        /// Skip provider verification (don't check if the package exists)
        #[arg(long)]
        no_verify: bool,

        /// Lua build script path (git dependencies only)
        #[arg(long)]
        with: Option<String>,

        /// Real package name when using a different local name (single spec only)
        #[arg(long)]
        alias: Option<String>,

        /// Override library name(s) for linking, comma-separated (single spec only)
        #[arg(long, value_delimiter = ',')]
        link_name: Option<Vec<String>>,
    },

    /// Update dependencies
    Update {
        /// Update a specific dependency only
        name: Option<String>,
    },

    /// Show dependency tree
    Tree {
        /// Show tree for a specific workspace member
        #[arg(short, long)]
        package: Option<String>,
    },

    /// Install the project to system
    Install {
        /// Installation prefix
        #[arg(long)]
        prefix: Option<PathBuf>,
    },

    /// Create a distributable archive
    Package,

    /// Publish to the Ordo registry
    Publish,

    /// Import from external project formats
    Import {
        #[command(subcommand)]
        source: ImportSource,
    },

    /// Generate configuration files
    Generate {
        #[command(subcommand)]
        target: GenerateTarget,
    },

    /// Manage toolchains
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommand,
    },

    /// Run CI pipeline
    Ci {
        /// Continue on failure
        #[arg(long)]
        keep_going: bool,
    },

    /// Check development environment
    Doctor,

    /// Show resolved configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },

    /// Run a user-defined script
    RunScript {
        /// Script name
        name: String,
    },

    /// Manage ordo itself
    #[command(name = "self")]
    SelfCmd {
        #[command(subcommand)]
        command: SelfCommand,
    },
}

#[derive(Subcommand)]
pub enum WatchCommand {
    /// Watch and rebuild
    Build,
    /// Watch and re-run tests
    Test,
    /// Watch, rebuild, and re-run
    Run,
}

#[derive(Subcommand)]
pub enum ImportSource {
    /// Import from CMakeLists.txt
    Cmake,
}

#[derive(Subcommand)]
pub enum GenerateTarget {
    /// Generate CMakeLists.txt
    Cmake,
    /// Generate CMakePresets.json
    Presets,
    /// Generate VSCode configuration
    Vscode,
    /// Generate CLion configuration
    Clion,
    /// Generate .clangd configuration
    Clangd,
    /// Generate GitHub Actions workflow
    GithubActions,
    /// Generate GitLab CI configuration
    GitlabCi,
}

#[derive(Subcommand)]
pub enum ToolchainCommand {
    /// List available toolchains
    List,
    /// Install a toolchain
    Install,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Show resolved configuration
    Show {
        /// Show the source of each value
        #[arg(long)]
        origin: bool,
    },
}

#[derive(Subcommand)]
pub enum SelfCommand {
    /// Update ordo to the latest version
    Update {
        /// Install a specific version
        #[arg(long)]
        version: Option<String>,
    },
}
