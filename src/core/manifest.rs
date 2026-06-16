#![allow(dead_code)]

use miette::{Diagnostic, NamedSource, SourceSpan};
use serde::Deserialize;
use std::fmt;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub toolchain: Toolchain,
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
        if self.package.name.is_empty() {
            return Err(ManifestError::ValidationError {
                message: "package name must not be empty".to_string(),
                help: Some("set [package] name to a valid identifier".to_string()),
            });
        }

        Self::validate_semver(&self.package.version)?;

        Ok(())
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

        assert_eq!(m.package.name, "myapp");
        assert_eq!(m.package.version, "0.1.0");
        assert_eq!(m.package.package_type, PackageType::Executable);
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

        assert_eq!(m.package.package_type, PackageType::StaticLibrary);
        assert_eq!(m.package.license.as_deref(), Some("MIT"));
        assert_eq!(m.package.authors.len(), 2);
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
        assert_eq!(m.package.package_type, PackageType::SharedLibrary);
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
    fn missing_package_section() {
        let err = parse("").unwrap_err();
        assert!(matches!(err, ManifestError::ParseError { .. }));
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
}
