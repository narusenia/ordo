#![allow(dead_code)]

use miette::{Diagnostic, NamedSource, SourceSpan};
use serde::Deserialize;
use std::fmt;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub package: Option<Package>,
    pub workspace: Option<WorkspaceConfig>,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub toolchain: Toolchain,
    #[serde(default)]
    pub cli: Option<CliConfig>,
    #[serde(default)]
    pub features: FeatureConfig,
    #[serde(default)]
    pub profile: std::collections::BTreeMap<String, ProfileConfig>,
    #[serde(default)]
    pub dependencies: std::collections::BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub dev_dependencies: std::collections::BTreeMap<String, DependencySpec>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct WorkspaceConfig {
    pub members: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub dependencies: std::collections::BTreeMap<String, DependencySpec>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(rename = "type")]
    pub package_type: PackageType,
    pub license: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageType {
    Executable,
    StaticLibrary,
    SharedLibrary,
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Executable => write!(f, "executable"),
            Self::StaticLibrary => write!(f, "static-library"),
            Self::SharedLibrary => write!(f, "shared-library"),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Language {
    pub c: Option<CStandard>,
    pub cpp: Option<CppStandard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum CStandard {
    #[serde(rename = "c11")]
    C11,
    #[serde(rename = "c17")]
    C17,
    #[serde(rename = "c23")]
    C23,
}

impl CStandard {
    pub fn as_flag(&self) -> &'static str {
        match self {
            Self::C11 => "c11",
            Self::C17 => "c17",
            Self::C23 => "c23",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum CppStandard {
    #[serde(rename = "c++17")]
    Cpp17,
    #[serde(rename = "c++20")]
    Cpp20,
    #[serde(rename = "c++23")]
    Cpp23,
    #[serde(rename = "c++26")]
    Cpp26,
}

impl CppStandard {
    pub fn as_flag(&self) -> &'static str {
        match self {
            Self::Cpp17 => "c++17",
            Self::Cpp20 => "c++20",
            Self::Cpp23 => "c++23",
            Self::Cpp26 => "c++26",
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CliConfig {
    pub style: Option<crate::cli::StyleMode>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Toolchain {
    pub compiler: Option<CompilerKind>,
    pub linker: Option<LinkerKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompilerKind {
    Clang,
    Gcc,
    Msvc,
    ClangCl,
}

impl fmt::Display for CompilerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Clang => write!(f, "clang"),
            Self::Gcc => write!(f, "gcc"),
            Self::Msvc => write!(f, "msvc"),
            Self::ClangCl => write!(f, "clang-cl"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LinkerKind {
    Lld,
    Mold,
    Gold,
    Default,
}

impl fmt::Display for LinkerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lld => write!(f, "lld"),
            Self::Mold => write!(f, "mold"),
            Self::Gold => write!(f, "gold"),
            Self::Default => write!(f, "default"),
        }
    }
}

// --- Feature configuration ---

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FeatureConfig {
    #[serde(default)]
    pub default: Vec<String>,
    #[serde(default, flatten)]
    pub features: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub config: Option<FeaturePrefixConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeaturePrefixConfig {
    #[serde(default = "FeaturePrefixConfig::default_prefix")]
    pub prefix: String,
}

impl FeaturePrefixConfig {
    fn default_prefix() -> String {
        "ORDO_FEATURE_".to_string()
    }
}

impl FeatureConfig {
    pub fn prefix(&self) -> &str {
        self.config
            .as_ref()
            .map(|c| c.prefix.as_str())
            .unwrap_or("ORDO_FEATURE_")
    }

    pub fn is_empty(&self) -> bool {
        self.default.is_empty() && self.features.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedFeatures {
    pub enabled: std::collections::BTreeSet<String>,
    pub activated_deps: std::collections::BTreeSet<String>,
    pub defines: Vec<String>,
}

impl ResolvedFeatures {
    pub fn resolve(
        manifest: &Manifest,
        cli_features: &[String],
        no_default_features: bool,
        all_features: bool,
    ) -> Result<Self, ManifestError> {
        let feature_map = &manifest.features;
        let mut enabled = std::collections::BTreeSet::new();
        let mut activated_deps = std::collections::BTreeSet::new();

        if all_features {
            for name in feature_map.features.keys() {
                enabled.insert(name.clone());
            }
            if !feature_map.default.is_empty() {
                for f in &feature_map.default {
                    enabled.insert(f.clone());
                }
            }
        } else {
            if !no_default_features {
                for f in &feature_map.default {
                    enabled.insert(f.clone());
                }
            }
            for f in cli_features {
                enabled.insert(f.clone());
            }
        }

        let mut queue: Vec<String> = enabled.iter().cloned().collect();
        let mut visited = std::collections::HashSet::new();

        while let Some(feat) = queue.pop() {
            if !visited.insert(feat.clone()) {
                continue;
            }

            if let Some(deps) = feature_map.features.get(&feat) {
                for dep in deps {
                    if let Some(dep_name) = dep.strip_prefix("dep:") {
                        activated_deps.insert(dep_name.to_string());
                    } else {
                        if !enabled.contains(dep) {
                            enabled.insert(dep.clone());
                            queue.push(dep.clone());
                        }
                    }
                }
            } else if !feature_map.default.is_empty() || !feature_map.features.is_empty() {
                return Err(ManifestError::ValidationError {
                    message: format!("unknown feature '{feat}'"),
                    help: Some(format!(
                        "available features: {}",
                        feature_map
                            .features
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                });
            }
        }

        let prefix = feature_map.prefix();
        let defines = enabled
            .iter()
            .map(|f| {
                let upper = f.to_uppercase().replace('-', "_");
                format!("{prefix}{upper}=1")
            })
            .collect();

        Ok(Self {
            enabled,
            activated_deps,
            defines,
        })
    }
}

// --- Profile configuration ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
    Os,
    Oz,
}

impl fmt::Display for OptLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_flag())
    }
}

impl OptLevel {
    pub fn as_flag(&self) -> &str {
        match self {
            Self::O0 => "0",
            Self::O1 => "1",
            Self::O2 => "2",
            Self::O3 => "3",
            Self::Os => "s",
            Self::Oz => "z",
        }
    }
}

impl<'de> Deserialize<'de> for OptLevel {
    fn deserialize<D>(deserializer: D) -> Result<OptLevel, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;

        impl<'de> serde::de::Visitor<'de> for V {
            type Value = OptLevel;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("0, 1, 2, 3, \"s\", or \"z\"")
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<OptLevel, E> {
                match v {
                    0 => Ok(OptLevel::O0),
                    1 => Ok(OptLevel::O1),
                    2 => Ok(OptLevel::O2),
                    3 => Ok(OptLevel::O3),
                    _ => Err(E::custom(format!("invalid opt-level: {v}"))),
                }
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<OptLevel, E> {
                if v >= 0 {
                    self.visit_u64(v as u64)
                } else {
                    Err(E::custom(format!("invalid opt-level: {v}")))
                }
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<OptLevel, E> {
                match v {
                    "0" => Ok(OptLevel::O0),
                    "1" => Ok(OptLevel::O1),
                    "2" => Ok(OptLevel::O2),
                    "3" => Ok(OptLevel::O3),
                    "s" => Ok(OptLevel::Os),
                    "z" => Ok(OptLevel::Oz),
                    _ => Err(E::custom(format!("invalid opt-level: \"{v}\""))),
                }
            }
        }

        deserializer.deserialize_any(V)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LtoMode {
    #[default]
    Off,
    Thin,
    Full,
}

impl<'de> Deserialize<'de> for LtoMode {
    fn deserialize<D>(deserializer: D) -> Result<LtoMode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V;

        impl<'de> serde::de::Visitor<'de> for V {
            type Value = LtoMode;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("false, \"thin\", or \"full\"")
            }

            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<LtoMode, E> {
                if v {
                    Ok(LtoMode::Full)
                } else {
                    Ok(LtoMode::Off)
                }
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<LtoMode, E> {
                match v {
                    "false" | "off" => Ok(LtoMode::Off),
                    "thin" => Ok(LtoMode::Thin),
                    "full" | "true" => Ok(LtoMode::Full),
                    _ => Err(E::custom(format!("invalid lto mode: \"{v}\""))),
                }
            }
        }

        deserializer.deserialize_any(V)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WarningLevel {
    Default,
    All,
    Extra,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Sanitizer {
    Address,
    Undefined,
    Thread,
    Memory,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProfileConfig {
    pub inherits: Option<String>,
    pub opt_level: Option<OptLevel>,
    pub debug: Option<bool>,
    pub assertions: Option<bool>,
    pub sanitize: Option<Vec<Sanitizer>>,
    pub lto: Option<LtoMode>,
    pub strip: Option<bool>,
    pub pic: Option<bool>,
    pub rtti: Option<bool>,
    pub exceptions: Option<bool>,
    pub warnings: Option<WarningLevel>,
    pub linker: Option<String>,
    pub static_runtime: Option<bool>,
    pub coverage: Option<bool>,
    pub split_debug: Option<bool>,
    pub pch: Option<String>,
    pub unity: Option<bool>,
    pub parallel: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub opt_level: OptLevel,
    pub debug: bool,
    pub assertions: bool,
    pub sanitize: Vec<Sanitizer>,
    pub lto: LtoMode,
    pub strip: bool,
    pub pic: bool,
    pub rtti: bool,
    pub exceptions: bool,
    pub warnings: WarningLevel,
    pub linker: Option<String>,
    pub static_runtime: bool,
    pub coverage: bool,
    pub split_debug: bool,
    pub pch: Option<String>,
    pub unity: Option<bool>,
    pub parallel: Option<u32>,
}

impl Profile {
    pub fn dev_defaults() -> Self {
        Self {
            opt_level: OptLevel::O0,
            debug: true,
            assertions: true,
            sanitize: Vec::new(),
            lto: LtoMode::Off,
            strip: false,
            pic: false,
            rtti: true,
            exceptions: true,
            warnings: WarningLevel::All,
            linker: None,
            static_runtime: false,
            coverage: false,
            split_debug: false,
            pch: None,
            unity: None,
            parallel: None,
        }
    }

    pub fn release_defaults() -> Self {
        Self {
            opt_level: OptLevel::O3,
            debug: false,
            assertions: false,
            sanitize: Vec::new(),
            lto: LtoMode::Off,
            strip: true,
            pic: false,
            rtti: true,
            exceptions: true,
            warnings: WarningLevel::All,
            linker: None,
            static_runtime: false,
            coverage: false,
            split_debug: false,
            pch: None,
            unity: None,
            parallel: None,
        }
    }

    pub fn display_desc(&self) -> String {
        let mut parts = Vec::new();

        match self.opt_level {
            OptLevel::O0 => parts.push("unoptimized"),
            OptLevel::O1 => parts.push("basic optimization"),
            OptLevel::O2 => parts.push("optimized"),
            OptLevel::O3 => parts.push("max optimization"),
            OptLevel::Os => parts.push("size-optimized"),
            OptLevel::Oz => parts.push("min-size"),
        }

        if self.debug {
            parts.push("debuginfo");
        }

        if self.lto != LtoMode::Off {
            parts.push("lto");
        }

        parts.join(" + ")
    }

    fn merge_from(&mut self, config: &ProfileConfig) {
        if let Some(v) = config.opt_level {
            self.opt_level = v;
        }
        if let Some(v) = config.debug {
            self.debug = v;
        }
        if let Some(v) = config.assertions {
            self.assertions = v;
        }
        if let Some(ref v) = config.sanitize {
            self.sanitize = v.clone();
        }
        if let Some(v) = config.lto {
            self.lto = v;
        }
        if let Some(v) = config.strip {
            self.strip = v;
        }
        if let Some(v) = config.pic {
            self.pic = v;
        }
        if let Some(v) = config.rtti {
            self.rtti = v;
        }
        if let Some(v) = config.exceptions {
            self.exceptions = v;
        }
        if let Some(v) = config.warnings {
            self.warnings = v;
        }
        if config.linker.is_some() {
            self.linker.clone_from(&config.linker);
        }
        if let Some(v) = config.static_runtime {
            self.static_runtime = v;
        }
        if let Some(v) = config.coverage {
            self.coverage = v;
        }
        if let Some(v) = config.split_debug {
            self.split_debug = v;
        }
        if config.pch.is_some() {
            self.pch.clone_from(&config.pch);
        }
        if config.unity.is_some() {
            self.unity = config.unity;
        }
        if config.parallel.is_some() {
            self.parallel = config.parallel;
        }
    }
}

// --- Dependency specification ---

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencySpec {
    pub version: Option<String>,
    pub path: Option<String>,
    pub git: Option<String>,
    pub tag: Option<String>,
    pub branch: Option<String>,
    pub rev: Option<String>,
    pub provider: Option<ProviderKind>,
    pub optional: bool,
    pub workspace: bool,
    pub features: Vec<String>,
    pub with: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Vcpkg,
    Conan,
    PkgConfig,
    System,
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vcpkg => write!(f, "vcpkg"),
            Self::Conan => write!(f, "conan"),
            Self::PkgConfig => write!(f, "pkg-config"),
            Self::System => write!(f, "system"),
        }
    }
}

impl<'de> Deserialize<'de> for DependencySpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Version(String),
            Table(DependencyTable),
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct DependencyTable {
            version: Option<String>,
            path: Option<String>,
            git: Option<String>,
            tag: Option<String>,
            branch: Option<String>,
            rev: Option<String>,
            provider: Option<ProviderKind>,
            #[serde(default)]
            optional: bool,
            #[serde(default)]
            workspace: bool,
            #[serde(default)]
            features: Vec<String>,
            with: Option<String>,
        }

        match Raw::deserialize(deserializer)? {
            Raw::Version(v) => Ok(DependencySpec {
                version: Some(v),
                path: None,
                git: None,
                tag: None,
                branch: None,
                rev: None,
                provider: None,
                optional: false,
                workspace: false,
                features: Vec::new(),
                with: None,
            }),
            Raw::Table(t) => Ok(DependencySpec {
                version: t.version,
                path: t.path,
                git: t.git,
                tag: t.tag,
                branch: t.branch,
                rev: t.rev,
                provider: t.provider,
                optional: t.optional,
                workspace: t.workspace,
                features: t.features,
                with: t.with,
            }),
        }
    }
}

impl DependencySpec {
    pub fn source_kind(&self) -> DependencySource {
        if self.workspace {
            DependencySource::Workspace
        } else if self.path.is_some() {
            DependencySource::Path
        } else if self.git.is_some() {
            DependencySource::Git
        } else if let Some(provider) = self.provider {
            DependencySource::Provider(provider)
        } else if self.version.is_some() {
            DependencySource::Registry
        } else {
            DependencySource::Unknown
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    Path,
    Git,
    Registry,
    Provider(ProviderKind),
    Workspace,
    Unknown,
}

// --- Errors ---

#[derive(Debug, Error, Diagnostic)]
#[allow(clippy::enum_variant_names)]
pub enum ManifestError {
    #[error("failed to read Ordo.toml")]
    #[diagnostic(code(E0001))]
    ReadError(#[from] std::io::Error),

    #[error("{message}")]
    #[diagnostic(code(E0002))]
    ParseError {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("here")]
        span: Option<SourceSpan>,
    },

    #[error("{message}")]
    #[diagnostic(code(E0003))]
    ValidationError {
        message: String,
        #[help]
        help: Option<String>,
    },
}

impl Manifest {
    pub fn package(&self) -> &Package {
        self.package
            .as_ref()
            .expect("called package() on workspace-only manifest")
    }

    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    pub fn is_virtual_workspace(&self) -> bool {
        self.workspace.is_some() && self.package.is_none()
    }

    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content, path)
    }

    pub fn parse(content: &str, path: &Path) -> Result<Self, ManifestError> {
        let manifest: Manifest =
            toml::from_str(content).map_err(|e| Self::toml_error(e, content, path))?;
        manifest.validate()?;
        Ok(manifest)
    }

    fn toml_error(e: toml::de::Error, content: &str, path: &Path) -> ManifestError {
        let span = e.span().map(|r| SourceSpan::new(r.start.into(), r.len()));
        ManifestError::ParseError {
            message: e.message().to_string(),
            src: NamedSource::new(path.display().to_string(), content.to_string()),
            span,
        }
    }

    fn validate(&self) -> Result<(), ManifestError> {
        if self.package.is_none() && self.workspace.is_none() {
            return Err(ManifestError::ValidationError {
                message: "Ordo.toml must contain [package] or [workspace] (or both)".to_string(),
                help: Some("add a [package] or [workspace] section".to_string()),
            });
        }

        if let Some(ref pkg) = self.package {
            if pkg.name.is_empty() {
                return Err(ManifestError::ValidationError {
                    message: "package name must not be empty".to_string(),
                    help: Some("set [package] name to a valid identifier".to_string()),
                });
            }
            Self::validate_semver(&pkg.version)?;
        }

        if let Some(ref ws) = self.workspace
            && ws.members.is_empty()
        {
            return Err(ManifestError::ValidationError {
                message: "workspace must have at least one member".to_string(),
                help: Some(
                    "add member paths to [workspace] members = [\"libs/*\", \"apps/*\"]"
                        .to_string(),
                ),
            });
        }

        for (name, spec) in &self.dependencies {
            if spec.with.is_some() && spec.git.is_none() {
                return Err(ManifestError::ValidationError {
                    message: format!(
                        "dependency '{name}': `with` is only valid on git dependencies"
                    ),
                    help: Some("add a `git = \"...\"` field or remove `with`".to_string()),
                });
            }
        }

        Ok(())
    }

    pub fn resolve_profile(&self, name: &str) -> Result<Profile, ManifestError> {
        let canonical = match name {
            "debug" => "dev",
            other => other,
        };

        let mut chain: Vec<&ProfileConfig> = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut current = canonical.to_string();

        loop {
            if !visited.insert(current.clone()) {
                return Err(ManifestError::ValidationError {
                    message: format!("circular profile inheritance: '{current}'"),
                    help: Some("check 'inherits' fields for cycles".to_string()),
                });
            }

            if let Some(config) = self.profile.get(&current) {
                chain.push(config);
                if let Some(ref inherits) = config.inherits {
                    current = inherits.clone();
                    continue;
                }
            }
            break;
        }

        let mut base = match current.as_str() {
            "dev" => Profile::dev_defaults(),
            "release" => Profile::release_defaults(),
            "bench" => {
                let mut p = Profile::release_defaults();
                p.debug = true;
                p
            }
            other => {
                if chain.is_empty() {
                    return Err(ManifestError::ValidationError {
                        message: format!("unknown profile '{name}'"),
                        help: Some(
                            "use 'dev', 'release', 'bench', or define [profile.<name>] with 'inherits'"
                                .to_string(),
                        ),
                    });
                }
                if self.profile.contains_key(other) {
                    return Err(ManifestError::ValidationError {
                        message: format!("profile '{other}' must specify 'inherits'"),
                        help: Some("add inherits = \"dev\" or inherits = \"release\"".to_string()),
                    });
                }
                return Err(ManifestError::ValidationError {
                    message: format!("profile '{other}' referenced by 'inherits' is not defined"),
                    help: Some(format!(
                        "define [profile.{other}] or use a built-in name (dev, release, bench)"
                    )),
                });
            }
        };

        for config in chain.iter().rev() {
            base.merge_from(config);
        }

        Ok(base)
    }

    fn validate_semver(version: &str) -> Result<(), ManifestError> {
        let parts: Vec<&str> = version.split('.').collect();
        let valid = match parts.len() {
            1..=3 => parts.iter().all(|p| p.parse::<u64>().is_ok()),
            _ => false,
        };

        if !valid {
            return Err(ManifestError::ValidationError {
                message: format!("invalid version '{version}': expected SemVer (e.g., 0.1.0)"),
                help: Some("use MAJOR.MINOR.PATCH format".to_string()),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn parse(content: &str) -> Result<Manifest, ManifestError> {
        Manifest::parse(content, &PathBuf::from("Ordo.toml"))
    }

    #[test]
    fn minimal_executable() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        assert_eq!(m.package().name, "myapp");
        assert_eq!(m.package().version, "0.1.0");
        assert_eq!(m.package().package_type, PackageType::Executable);
        assert_eq!(m.language.cpp, None);
        assert!(m.toolchain.compiler.is_none());
    }

    #[test]
    fn full_manifest() {
        let m = parse(
            r#"
            [package]
            name = "mylib"
            version = "1.2.3"
            type = "static-library"
            license = "MIT"
            description = "A library"
            authors = ["Alice", "Bob"]
            repository = "https://example.com"

            [language]
            c = "c23"
            cpp = "c++23"

            [toolchain]
            compiler = "clang"
            linker = "lld"
            "#,
        )
        .unwrap();

        assert_eq!(m.package().package_type, PackageType::StaticLibrary);
        assert_eq!(m.package().license.as_deref(), Some("MIT"));
        assert_eq!(m.package().authors.len(), 2);
        assert_eq!(m.language.c, Some(CStandard::C23));
        assert_eq!(m.language.cpp, Some(CppStandard::Cpp23));
        assert_eq!(m.toolchain.compiler, Some(CompilerKind::Clang));
        assert_eq!(m.toolchain.linker, Some(LinkerKind::Lld));
    }

    #[test]
    fn shared_library() {
        let m = parse(
            r#"
            [package]
            name = "myso"
            version = "0.1.0"
            type = "shared-library"
            "#,
        )
        .unwrap();
        assert_eq!(m.package().package_type, PackageType::SharedLibrary);
    }

    #[test]
    fn invalid_version() {
        let err = parse(
            r#"
            [package]
            name = "myapp"
            version = "not-a-version"
            type = "executable"
            "#,
        )
        .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("invalid version"), "got: {msg}");
    }

    #[test]
    fn empty_name() {
        let err = parse(
            r#"
            [package]
            name = ""
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("name must not be empty"), "got: {msg}");
    }

    #[test]
    fn missing_package_and_workspace() {
        let err = parse("").unwrap_err();
        assert!(matches!(err, ManifestError::ValidationError { .. }));
        let msg = err.to_string();
        assert!(msg.contains("[package] or [workspace]"), "got: {msg}");
    }

    #[test]
    fn invalid_type() {
        let err = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "invalid-type"
            "#,
        )
        .unwrap_err();

        assert!(matches!(err, ManifestError::ParseError { .. }));
    }

    #[test]
    fn invalid_compiler() {
        let err = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [toolchain]
            compiler = "turbo-c"
            "#,
        )
        .unwrap_err();

        assert!(matches!(err, ManifestError::ParseError { .. }));
    }

    #[test]
    fn language_defaults_to_none() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        assert_eq!(m.language.cpp, None);
        assert_eq!(m.language.c, None);
    }

    #[test]
    fn all_cpp_standards() {
        for (input, expected) in [
            ("c++17", CppStandard::Cpp17),
            ("c++20", CppStandard::Cpp20),
            ("c++23", CppStandard::Cpp23),
            ("c++26", CppStandard::Cpp26),
        ] {
            let m = parse(&format!(
                r#"
                [package]
                name = "myapp"
                version = "0.1.0"
                type = "executable"

                [language]
                cpp = "{input}"
                "#
            ))
            .unwrap();

            assert_eq!(m.language.cpp, Some(expected));
        }
    }

    #[test]
    fn all_c_standards() {
        for (input, expected) in [
            ("c11", CStandard::C11),
            ("c17", CStandard::C17),
            ("c23", CStandard::C23),
        ] {
            let m = parse(&format!(
                r#"
                [package]
                name = "myapp"
                version = "0.1.0"
                type = "executable"

                [language]
                c = "{input}"
                "#
            ))
            .unwrap();

            assert_eq!(m.language.c, Some(expected));
        }
    }

    #[test]
    fn all_compiler_kinds() {
        for (input, expected) in [
            ("clang", CompilerKind::Clang),
            ("gcc", CompilerKind::Gcc),
            ("msvc", CompilerKind::Msvc),
            ("clang-cl", CompilerKind::ClangCl),
        ] {
            let m = parse(&format!(
                r#"
                [package]
                name = "myapp"
                version = "0.1.0"
                type = "executable"

                [toolchain]
                compiler = "{input}"
                "#
            ))
            .unwrap();

            assert_eq!(m.toolchain.compiler, Some(expected));
        }
    }

    #[test]
    fn all_linker_kinds() {
        for (input, expected) in [
            ("lld", LinkerKind::Lld),
            ("mold", LinkerKind::Mold),
            ("gold", LinkerKind::Gold),
            ("default", LinkerKind::Default),
        ] {
            let m = parse(&format!(
                r#"
                [package]
                name = "myapp"
                version = "0.1.0"
                type = "executable"

                [toolchain]
                linker = "{input}"
                "#
            ))
            .unwrap();

            assert_eq!(m.toolchain.linker, Some(expected));
        }
    }

    #[test]
    fn as_flag_methods() {
        assert_eq!(CStandard::C11.as_flag(), "c11");
        assert_eq!(CStandard::C17.as_flag(), "c17");
        assert_eq!(CStandard::C23.as_flag(), "c23");
        assert_eq!(CppStandard::Cpp17.as_flag(), "c++17");
        assert_eq!(CppStandard::Cpp20.as_flag(), "c++20");
        assert_eq!(CppStandard::Cpp23.as_flag(), "c++23");
        assert_eq!(CppStandard::Cpp26.as_flag(), "c++26");
    }

    // --- Dependency parsing tests ---

    #[test]
    fn dep_version_short_form() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            fmt = "11"
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["fmt"];
        assert_eq!(dep.version.as_deref(), Some("11"));
        assert_eq!(dep.source_kind(), DependencySource::Registry);
    }

    #[test]
    fn dep_version_long_form() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            fmt = { version = "11.1" }
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["fmt"];
        assert_eq!(dep.version.as_deref(), Some("11.1"));
        assert_eq!(dep.source_kind(), DependencySource::Registry);
    }

    #[test]
    fn dep_path() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            core = { path = "../core" }
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["core"];
        assert_eq!(dep.path.as_deref(), Some("../core"));
        assert_eq!(dep.source_kind(), DependencySource::Path);
    }

    #[test]
    fn dep_git_with_tag() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            fmt = { git = "https://github.com/fmtlib/fmt", tag = "11.1.0" }
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["fmt"];
        assert_eq!(dep.git.as_deref(), Some("https://github.com/fmtlib/fmt"));
        assert_eq!(dep.tag.as_deref(), Some("11.1.0"));
        assert_eq!(dep.source_kind(), DependencySource::Git);
    }

    #[test]
    fn dep_provider_vcpkg() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            spdlog = { version = "1.14", provider = "vcpkg" }
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["spdlog"];
        assert_eq!(dep.provider, Some(ProviderKind::Vcpkg));
        assert_eq!(dep.version.as_deref(), Some("1.14"));
        assert_eq!(
            dep.source_kind(),
            DependencySource::Provider(ProviderKind::Vcpkg)
        );
    }

    #[test]
    fn dep_provider_pkg_config() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            openssl = { provider = "pkg-config" }
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["openssl"];
        assert_eq!(dep.provider, Some(ProviderKind::PkgConfig));
    }

    #[test]
    fn dep_provider_system() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            zlib = { provider = "system" }
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["zlib"];
        assert_eq!(dep.provider, Some(ProviderKind::System));
        assert_eq!(
            dep.source_kind(),
            DependencySource::Provider(ProviderKind::System)
        );
    }

    #[test]
    fn dep_optional() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            qt = { provider = "vcpkg", optional = true }
            "#,
        )
        .unwrap();

        assert!(m.dependencies["qt"].optional);
    }

    #[test]
    fn dep_workspace_ref() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            fmt = { workspace = true }
            "#,
        )
        .unwrap();

        let dep = &m.dependencies["fmt"];
        assert!(dep.workspace);
        assert_eq!(dep.source_kind(), DependencySource::Workspace);
    }

    #[test]
    fn dep_with_features() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            spdlog = { version = "1.14", provider = "vcpkg", features = ["async"] }
            "#,
        )
        .unwrap();

        assert_eq!(m.dependencies["spdlog"].features, vec!["async"]);
    }

    #[test]
    fn dev_dependencies() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dev-dependencies]
            gtest = { provider = "vcpkg" }
            "#,
        )
        .unwrap();

        assert!(m.dev_dependencies.contains_key("gtest"));
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn mixed_dependency_forms() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            fmt = "11"
            core = { path = "../core" }
            spdlog = { version = "1.14", provider = "vcpkg" }
            openssl = { provider = "pkg-config" }
            zlib = { provider = "system" }
            "#,
        )
        .unwrap();

        assert_eq!(m.dependencies.len(), 5);
        assert_eq!(
            m.dependencies["fmt"].source_kind(),
            DependencySource::Registry
        );
        assert_eq!(m.dependencies["core"].source_kind(), DependencySource::Path);
        assert_eq!(
            m.dependencies["spdlog"].source_kind(),
            DependencySource::Provider(ProviderKind::Vcpkg)
        );
        assert_eq!(
            m.dependencies["openssl"].source_kind(),
            DependencySource::Provider(ProviderKind::PkgConfig)
        );
        assert_eq!(
            m.dependencies["zlib"].source_kind(),
            DependencySource::Provider(ProviderKind::System)
        );
    }

    #[test]
    fn no_dependencies_is_valid() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        assert!(m.dependencies.is_empty());
        assert!(m.dev_dependencies.is_empty());
    }

    // --- Workspace parsing tests ---

    #[test]
    fn workspace_only() {
        let m = parse(
            r#"
            [workspace]
            members = ["libs/*", "apps/*"]
            "#,
        )
        .unwrap();

        assert!(m.package.is_none());
        assert!(m.is_workspace());
        assert!(m.is_virtual_workspace());
        let ws = m.workspace.as_ref().unwrap();
        assert_eq!(ws.members, vec!["libs/*", "apps/*"]);
        assert!(ws.exclude.is_empty());
    }

    #[test]
    fn workspace_with_exclude() {
        let m = parse(
            r#"
            [workspace]
            members = ["libs/*"]
            exclude = ["libs/experimental"]
            "#,
        )
        .unwrap();

        let ws = m.workspace.as_ref().unwrap();
        assert_eq!(ws.exclude, vec!["libs/experimental"]);
    }

    #[test]
    fn workspace_with_package() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [workspace]
            members = ["libs/*"]
            "#,
        )
        .unwrap();

        assert!(m.package.is_some());
        assert!(m.is_workspace());
        assert!(!m.is_virtual_workspace());
        assert_eq!(m.package().name, "myapp");
    }

    #[test]
    fn workspace_shared_dependencies() {
        let m = parse(
            r#"
            [workspace]
            members = ["apps/*"]

            [workspace.dependencies]
            fmt = "11"
            spdlog = { version = "1.14", provider = "vcpkg" }
            "#,
        )
        .unwrap();

        let ws = m.workspace.as_ref().unwrap();
        assert_eq!(ws.dependencies.len(), 2);
        assert_eq!(ws.dependencies["fmt"].version.as_deref(), Some("11"));
        assert_eq!(
            ws.dependencies["spdlog"].provider,
            Some(ProviderKind::Vcpkg)
        );
    }

    #[test]
    fn workspace_empty_members_invalid() {
        let err = parse(
            r#"
            [workspace]
            members = []
            "#,
        )
        .unwrap_err();

        let msg = err.to_string();
        assert!(msg.contains("at least one member"), "got: {msg}");
    }

    // --- Profile tests ---

    #[test]
    fn profile_dev_defaults() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("dev").unwrap();
        assert_eq!(p.opt_level, OptLevel::O0);
        assert!(p.debug);
        assert!(p.assertions);
        assert!(!p.strip);
        assert!(p.rtti);
        assert!(p.exceptions);
        assert_eq!(p.warnings, WarningLevel::All);
        assert_eq!(p.lto, LtoMode::Off);
    }

    #[test]
    fn profile_debug_alias() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        assert_eq!(
            m.resolve_profile("debug").unwrap(),
            m.resolve_profile("dev").unwrap()
        );
    }

    #[test]
    fn profile_release_defaults() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("release").unwrap();
        assert_eq!(p.opt_level, OptLevel::O3);
        assert!(!p.debug);
        assert!(!p.assertions);
        assert!(p.strip);
    }

    #[test]
    fn profile_bench_defaults() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("bench").unwrap();
        assert_eq!(p.opt_level, OptLevel::O3);
        assert!(p.debug);
        assert!(p.strip);
    }

    #[test]
    fn profile_user_override() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.dev]
            opt-level = 2
            sanitize = ["address", "undefined"]
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("dev").unwrap();
        assert_eq!(p.opt_level, OptLevel::O2);
        assert_eq!(p.sanitize, vec![Sanitizer::Address, Sanitizer::Undefined]);
        assert!(p.debug);
    }

    #[test]
    fn profile_release_override() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.release]
            lto = "thin"
            opt-level = "s"
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("release").unwrap();
        assert_eq!(p.lto, LtoMode::Thin);
        assert_eq!(p.opt_level, OptLevel::Os);
        assert!(!p.debug);
    }

    #[test]
    fn profile_custom_inherits() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.profiling]
            inherits = "release"
            debug = true
            strip = false
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("profiling").unwrap();
        assert_eq!(p.opt_level, OptLevel::O3);
        assert!(p.debug);
        assert!(!p.strip);
    }

    #[test]
    fn profile_inherits_chain() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.release]
            lto = "thin"

            [profile.dist]
            inherits = "release"
            lto = "full"
            strip = false
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("dist").unwrap();
        assert_eq!(p.lto, LtoMode::Full);
        assert!(!p.strip);
        assert_eq!(p.opt_level, OptLevel::O3);
    }

    #[test]
    fn profile_circular_inherits() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.a]
            inherits = "b"

            [profile.b]
            inherits = "a"
            "#,
        )
        .unwrap();

        let err = m.resolve_profile("a").unwrap_err();
        assert!(err.to_string().contains("circular"), "got: {err}");
    }

    #[test]
    fn profile_custom_no_inherits() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.custom]
            opt-level = 1
            "#,
        )
        .unwrap();

        let err = m.resolve_profile("custom").unwrap_err();
        assert!(err.to_string().contains("inherits"), "got: {err}");
    }

    #[test]
    fn profile_unknown() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        let err = m.resolve_profile("nonexistent").unwrap_err();
        assert!(err.to_string().contains("unknown"), "got: {err}");
    }

    #[test]
    fn profile_lto_bool_false() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.dev]
            lto = false
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("dev").unwrap();
        assert_eq!(p.lto, LtoMode::Off);
    }

    #[test]
    fn profile_lto_bool_true() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.dev]
            lto = true
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("dev").unwrap();
        assert_eq!(p.lto, LtoMode::Full);
    }

    #[test]
    fn profile_all_fields() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [profile.dev]
            opt-level = "z"
            debug = false
            assertions = false
            sanitize = ["thread"]
            lto = "thin"
            strip = true
            pic = true
            rtti = false
            exceptions = false
            warnings = "error"
            linker = "mold"
            static-runtime = true
            coverage = true
            split-debug = true
            pch = "pch/all.h"
            unity = true
            parallel = 8
            "#,
        )
        .unwrap();

        let p = m.resolve_profile("dev").unwrap();
        assert_eq!(p.opt_level, OptLevel::Oz);
        assert!(!p.debug);
        assert!(!p.assertions);
        assert_eq!(p.sanitize, vec![Sanitizer::Thread]);
        assert_eq!(p.lto, LtoMode::Thin);
        assert!(p.strip);
        assert!(p.pic);
        assert!(!p.rtti);
        assert!(!p.exceptions);
        assert_eq!(p.warnings, WarningLevel::Error);
        assert_eq!(p.linker.as_deref(), Some("mold"));
        assert!(p.static_runtime);
        assert!(p.coverage);
        assert!(p.split_debug);
        assert_eq!(p.pch.as_deref(), Some("pch/all.h"));
        assert_eq!(p.unity, Some(true));
        assert_eq!(p.parallel, Some(8));
    }

    // --- Feature tests ---

    #[test]
    fn feature_parsing_basic() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = ["logging"]
            logging = []
            gui = ["dep:qt", "logging"]
            simd = []
            "#,
        )
        .unwrap();

        assert_eq!(m.features.default, vec!["logging"]);
        assert!(m.features.features.contains_key("logging"));
        assert!(m.features.features.contains_key("gui"));
        assert!(m.features.features.contains_key("simd"));
        assert_eq!(m.features.features["gui"], vec!["dep:qt", "logging"]);
    }

    #[test]
    fn feature_resolve_defaults() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = ["logging"]
            logging = []
            gui = []
            "#,
        )
        .unwrap();

        let resolved = ResolvedFeatures::resolve(&m, &[], false, false).unwrap();
        assert!(resolved.enabled.contains("logging"));
        assert!(!resolved.enabled.contains("gui"));
    }

    #[test]
    fn feature_resolve_no_default() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = ["logging"]
            logging = []
            "#,
        )
        .unwrap();

        let resolved = ResolvedFeatures::resolve(&m, &[], true, false).unwrap();
        assert!(resolved.enabled.is_empty());
    }

    #[test]
    fn feature_resolve_all() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = ["logging"]
            logging = []
            gui = []
            simd = []
            "#,
        )
        .unwrap();

        let resolved = ResolvedFeatures::resolve(&m, &[], false, true).unwrap();
        assert!(resolved.enabled.contains("logging"));
        assert!(resolved.enabled.contains("gui"));
        assert!(resolved.enabled.contains("simd"));
    }

    #[test]
    fn feature_resolve_cli_features() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = []
            logging = []
            gui = []
            "#,
        )
        .unwrap();

        let resolved =
            ResolvedFeatures::resolve(&m, &["gui".to_string()], false, false).unwrap();
        assert!(resolved.enabled.contains("gui"));
        assert!(!resolved.enabled.contains("logging"));
    }

    #[test]
    fn feature_resolve_transitive() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = []
            logging = []
            gui = ["logging"]
            "#,
        )
        .unwrap();

        let resolved =
            ResolvedFeatures::resolve(&m, &["gui".to_string()], false, false).unwrap();
        assert!(resolved.enabled.contains("gui"));
        assert!(resolved.enabled.contains("logging"));
    }

    #[test]
    fn feature_resolve_dep_activation() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = []
            gui = ["dep:qt", "logging"]
            logging = []

            [dependencies]
            qt = { provider = "vcpkg", optional = true }
            "#,
        )
        .unwrap();

        let resolved =
            ResolvedFeatures::resolve(&m, &["gui".to_string()], false, false).unwrap();
        assert!(resolved.activated_deps.contains("qt"));
        assert!(resolved.enabled.contains("logging"));
    }

    #[test]
    fn feature_resolve_defines() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = ["logging"]
            logging = []
            "#,
        )
        .unwrap();

        let resolved = ResolvedFeatures::resolve(&m, &[], false, false).unwrap();
        assert!(resolved.defines.contains(&"ORDO_FEATURE_LOGGING=1".to_string()));
    }

    #[test]
    fn feature_custom_prefix() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = ["logging"]
            logging = []

            [features.config]
            prefix = "MYAPP_"
            "#,
        )
        .unwrap();

        let resolved = ResolvedFeatures::resolve(&m, &[], false, false).unwrap();
        assert!(resolved.defines.contains(&"MYAPP_LOGGING=1".to_string()));
    }

    #[test]
    fn feature_unknown_errors() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [features]
            default = []
            logging = []
            "#,
        )
        .unwrap();

        let err =
            ResolvedFeatures::resolve(&m, &["nonexistent".to_string()], false, false).unwrap_err();
        assert!(err.to_string().contains("unknown feature"), "got: {err}");
    }

    #[test]
    fn feature_no_features_section() {
        let m = parse(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        )
        .unwrap();

        let resolved = ResolvedFeatures::resolve(&m, &[], false, false).unwrap();
        assert!(resolved.enabled.is_empty());
        assert!(resolved.defines.is_empty());
    }
}
