#![allow(dead_code)]

use crate::core::manifest::{CompilerKind, CppStandard, LinkerKind, Manifest};
use crate::util::paths::OrdoPaths;
use std::env;
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Default,
    Global,
    Workspace,
    ProjectLocal,
    Project,
    Env,
    Cli,
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Global => write!(f, "global config"),
            Self::Workspace => write!(f, "workspace"),
            Self::ProjectLocal => write!(f, ".ordo/config.toml"),
            Self::Project => write!(f, "Ordo.toml"),
            Self::Env => write!(f, "environment variable"),
            Self::Cli => write!(f, "CLI flag"),
        }
    }
}

#[derive(Debug)]
pub struct Tracked<T> {
    pub value: T,
    pub origin: Origin,
}

impl<T> Tracked<T> {
    fn new(value: T, origin: Origin) -> Self {
        Self { value, origin }
    }

    fn default(value: T) -> Self {
        Self::new(value, Origin::Default)
    }
}

#[derive(Debug)]
pub struct ResolvedConfig {
    pub compiler: Tracked<Option<CompilerKind>>,
    pub linker: Tracked<Option<LinkerKind>>,
    pub cpp_standard: Tracked<CppStandard>,
}

impl ResolvedConfig {
    pub fn resolve(manifest: &Manifest, paths: &OrdoPaths, cli: &CliOverrides) -> Self {
        Self::resolve_with(manifest, paths, &EnvVars::from_env(), cli)
    }

    pub fn resolve_with(
        manifest: &Manifest,
        paths: &OrdoPaths,
        env_vars: &EnvVars,
        cli: &CliOverrides,
    ) -> Self {
        let mut config = Self {
            compiler: Tracked::default(None),
            linker: Tracked::default(None),
            cpp_standard: Tracked::default(CppStandard::Cpp20),
        };

        config.apply_global(paths);
        config.apply_project(manifest);
        config.apply_project_local(manifest);
        config.apply_env_vars(env_vars);
        config.apply_cli(cli);

        config
    }

    fn apply_global(&mut self, paths: &OrdoPaths) {
        let path = paths.global_config_file();
        if let Some(manifest) = load_toml_if_exists(&path) {
            if let Some(c) = manifest.toolchain.compiler {
                self.compiler = Tracked::new(Some(c), Origin::Global);
            }
            if let Some(l) = manifest.toolchain.linker {
                self.linker = Tracked::new(Some(l), Origin::Global);
            }
            if let Some(cpp) = manifest.language.cpp {
                self.cpp_standard = Tracked::new(cpp, Origin::Global);
            }
        }
    }

    fn apply_project(&mut self, manifest: &Manifest) {
        if let Some(c) = manifest.toolchain.compiler {
            self.compiler = Tracked::new(Some(c), Origin::Project);
        }
        if let Some(l) = manifest.toolchain.linker {
            self.linker = Tracked::new(Some(l), Origin::Project);
        }
        if let Some(cpp) = manifest.language.cpp {
            self.cpp_standard = Tracked::new(cpp, Origin::Project);
        }
    }

    fn apply_project_local(&mut self, _manifest: &Manifest) {
        // .ordo/config.toml — loaded relative to project root
        // Deferred until we have project root discovery
    }

    fn apply_env(&mut self) {
        self.apply_env_vars(&EnvVars::from_env());
    }

    fn apply_env_vars(&mut self, vars: &EnvVars) {
        if let Some(ref val) = vars.compiler
            && let Some(c) = parse_compiler(val)
        {
            self.compiler = Tracked::new(Some(c), Origin::Env);
        }
        if let Some(ref val) = vars.linker
            && let Some(l) = parse_linker(val)
        {
            self.linker = Tracked::new(Some(l), Origin::Env);
        }
        if let Some(ref val) = vars.cpp
            && let Some(cpp) = parse_cpp_standard(val)
        {
            self.cpp_standard = Tracked::new(cpp, Origin::Env);
        }
    }

    fn apply_cli(&mut self, cli: &CliOverrides) {
        if let Some(c) = cli.compiler {
            self.compiler = Tracked::new(Some(c), Origin::Cli);
        }
        if let Some(l) = cli.linker {
            self.linker = Tracked::new(Some(l), Origin::Cli);
        }
    }
}

#[derive(Debug, Default)]
pub struct CliOverrides {
    pub compiler: Option<CompilerKind>,
    pub linker: Option<LinkerKind>,
}

#[derive(Debug, Default)]
pub struct EnvVars {
    pub compiler: Option<String>,
    pub linker: Option<String>,
    pub cpp: Option<String>,
}

impl EnvVars {
    pub fn from_env() -> Self {
        Self {
            compiler: env::var("ORDO_TOOLCHAIN_COMPILER").ok(),
            linker: env::var("ORDO_TOOLCHAIN_LINKER").ok(),
            cpp: env::var("ORDO_LANGUAGE_CPP").ok(),
        }
    }
}

fn load_toml_if_exists(path: &Path) -> Option<Manifest> {
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

fn parse_compiler(s: &str) -> Option<CompilerKind> {
    match s {
        "clang" => Some(CompilerKind::Clang),
        "gcc" => Some(CompilerKind::Gcc),
        "msvc" => Some(CompilerKind::Msvc),
        "clang-cl" => Some(CompilerKind::ClangCl),
        _ => None,
    }
}

fn parse_linker(s: &str) -> Option<LinkerKind> {
    match s {
        "lld" => Some(LinkerKind::Lld),
        "mold" => Some(LinkerKind::Mold),
        "gold" => Some(LinkerKind::Gold),
        "default" => Some(LinkerKind::Default),
        _ => None,
    }
}

fn parse_cpp_standard(s: &str) -> Option<CppStandard> {
    match s {
        "c++17" => Some(CppStandard::Cpp17),
        "c++20" => Some(CppStandard::Cpp20),
        "c++23" => Some(CppStandard::Cpp23),
        "c++26" => Some(CppStandard::Cpp26),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::Manifest;
    use std::path::PathBuf;

    fn minimal_manifest() -> Manifest {
        Manifest::parse(
            r#"
            [package]
            name = "test"
            version = "0.1.0"
            type = "executable"
            "#,
            &PathBuf::from("Ordo.toml"),
        )
        .unwrap()
    }

    fn manifest_with_toolchain() -> Manifest {
        Manifest::parse(
            r#"
            [package]
            name = "test"
            version = "0.1.0"
            type = "executable"

            [toolchain]
            compiler = "gcc"
            linker = "mold"

            [language]
            cpp = "c++23"
            "#,
            &PathBuf::from("Ordo.toml"),
        )
        .unwrap()
    }

    fn test_paths() -> OrdoPaths {
        OrdoPaths {
            config_dir: PathBuf::from("/nonexistent/ordo"),
            cache_dir: PathBuf::from("/nonexistent/ordo/cache"),
            credentials_file: PathBuf::from("/nonexistent/ordo/credentials.toml"),
        }
    }

    fn no_env() -> EnvVars {
        EnvVars::default()
    }

    fn no_cli() -> CliOverrides {
        CliOverrides::default()
    }

    #[test]
    fn defaults_when_nothing_set() {
        let config =
            ResolvedConfig::resolve_with(&minimal_manifest(), &test_paths(), &no_env(), &no_cli());

        assert_eq!(config.compiler.value, None);
        assert_eq!(config.compiler.origin, Origin::Default);
        assert_eq!(config.cpp_standard.value, CppStandard::Cpp20);
        assert_eq!(config.cpp_standard.origin, Origin::Default);
    }

    #[test]
    fn project_overrides_default() {
        let config = ResolvedConfig::resolve_with(
            &manifest_with_toolchain(),
            &test_paths(),
            &no_env(),
            &no_cli(),
        );

        assert_eq!(config.compiler.value, Some(CompilerKind::Gcc));
        assert_eq!(config.compiler.origin, Origin::Project);
        assert_eq!(config.linker.value, Some(LinkerKind::Mold));
        assert_eq!(config.linker.origin, Origin::Project);
        assert_eq!(config.cpp_standard.value, CppStandard::Cpp23);
        assert_eq!(config.cpp_standard.origin, Origin::Project);
    }

    #[test]
    fn env_overrides_project() {
        let env = EnvVars {
            compiler: Some("clang".to_string()),
            ..EnvVars::default()
        };
        let config = ResolvedConfig::resolve_with(
            &manifest_with_toolchain(),
            &test_paths(),
            &env,
            &no_cli(),
        );

        assert_eq!(config.compiler.value, Some(CompilerKind::Clang));
        assert_eq!(config.compiler.origin, Origin::Env);
        assert_eq!(config.linker.value, Some(LinkerKind::Mold));
        assert_eq!(config.linker.origin, Origin::Project);
    }

    #[test]
    fn cli_overrides_env() {
        let env = EnvVars {
            compiler: Some("clang".to_string()),
            ..EnvVars::default()
        };
        let cli = CliOverrides {
            compiler: Some(CompilerKind::Msvc),
            linker: None,
        };
        let config =
            ResolvedConfig::resolve_with(&manifest_with_toolchain(), &test_paths(), &env, &cli);

        assert_eq!(config.compiler.value, Some(CompilerKind::Msvc));
        assert_eq!(config.compiler.origin, Origin::Cli);
    }

    #[test]
    fn env_cpp_standard_override() {
        let env = EnvVars {
            cpp: Some("c++26".to_string()),
            ..EnvVars::default()
        };
        let config =
            ResolvedConfig::resolve_with(&minimal_manifest(), &test_paths(), &env, &no_cli());

        assert_eq!(config.cpp_standard.value, CppStandard::Cpp26);
        assert_eq!(config.cpp_standard.origin, Origin::Env);
    }

    #[test]
    fn invalid_env_values_ignored() {
        let env = EnvVars {
            compiler: Some("turbo-c".to_string()),
            ..EnvVars::default()
        };
        let config =
            ResolvedConfig::resolve_with(&minimal_manifest(), &test_paths(), &env, &no_cli());

        assert_eq!(config.compiler.value, None);
        assert_eq!(config.compiler.origin, Origin::Default);
    }

    #[test]
    fn origin_display() {
        assert_eq!(Origin::Default.to_string(), "default");
        assert_eq!(Origin::Cli.to_string(), "CLI flag");
        assert_eq!(Origin::Env.to_string(), "environment variable");
        assert_eq!(Origin::Project.to_string(), "Ordo.toml");
    }
}
