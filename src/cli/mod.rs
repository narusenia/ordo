pub mod add;
pub mod build;
pub mod clean;
pub mod init;
pub mod new;
pub mod run;
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
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
    },

    /// Build and run the project
    Run {
        /// Arguments to pass to the binary
        #[arg(last = true)]
        args: Vec<String>,

        /// Build with release profile
        #[arg(long)]
        release: bool,
    },

    /// Run tests
    Test {
        /// Filter tests by name
        #[arg(long)]
        filter: Option<String>,

        /// Number of parallel test jobs
        #[arg(short, long)]
        jobs: Option<u32>,
    },

    /// Syntax check without producing binaries
    Check,

    /// Remove build artifacts
    Clean {
        /// Also clear external build cache
        #[arg(long)]
        cache: bool,
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

    /// Add a dependency (e.g. `ordo add raylib@6 -p vcpkg`, `ordo add vcpkg:raylib@6`)
    Add {
        /// Package spec: name, name@version, or provider:name@version
        spec: String,

        /// Provider (pkg-config, system, vcpkg, conan, path, git). Interactive if omitted.
        #[arg(short, long)]
        provider: Option<String>,
    },

    /// Update dependencies
    Update {
        /// Update a specific dependency only
        name: Option<String>,
    },

    /// Show dependency tree
    Tree,

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
