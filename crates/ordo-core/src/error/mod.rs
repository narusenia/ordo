#![allow(dead_code)]

pub mod codes;

use miette::Diagnostic;
use thiserror::Error;

use crate::manifest::ManifestError;

#[derive(Debug, Error, Diagnostic)]
pub enum OrdoError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Config(#[from] ManifestError),

    #[error("{message}")]
    #[diagnostic(code("{code}"))]
    Deps {
        code: &'static str,
        message: String,
        #[help]
        help: Option<String>,
    },

    #[error("{message}")]
    #[diagnostic(code("{code}"))]
    Build {
        code: &'static str,
        message: String,
        #[help]
        help: Option<String>,
    },

    #[error("{message}")]
    #[diagnostic(code("{code}"))]
    Toolchain {
        code: &'static str,
        message: String,
        #[help]
        help: Option<String>,
    },

    #[error("{message}")]
    #[diagnostic(code("{code}"))]
    Test {
        code: &'static str,
        message: String,
        #[help]
        help: Option<String>,
    },
}

impl OrdoError {
    pub fn toolchain(code: &'static str, message: impl Into<String>) -> Self {
        Self::Toolchain {
            code,
            message: message.into(),
            help: None,
        }
    }

    pub fn toolchain_with_help(
        code: &'static str,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self::Toolchain {
            code,
            message: message.into(),
            help: Some(help.into()),
        }
    }

    pub fn build(code: &'static str, message: impl Into<String>) -> Self {
        Self::Build {
            code,
            message: message.into(),
            help: None,
        }
    }

    pub fn build_with_help(
        code: &'static str,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self::Build {
            code,
            message: message.into(),
            help: Some(help.into()),
        }
    }

    pub fn deps(code: &'static str, message: impl Into<String>) -> Self {
        Self::Deps {
            code,
            message: message.into(),
            help: None,
        }
    }

    pub fn deps_with_help(
        code: &'static str,
        message: impl Into<String>,
        help: impl Into<String>,
    ) -> Self {
        Self::Deps {
            code,
            message: message.into(),
            help: Some(help.into()),
        }
    }

    pub fn test_err(code: &'static str, message: impl Into<String>) -> Self {
        Self::Test {
            code,
            message: message.into(),
            help: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::codes;

    #[test]
    fn toolchain_error_display() {
        let err = OrdoError::toolchain(codes::E0300, "no compiler found on PATH");
        assert_eq!(err.to_string(), "no compiler found on PATH");
        let diag: &dyn Diagnostic = &err;
        assert!(diag.code().is_some());
    }

    #[test]
    fn toolchain_with_help_display() {
        let err = OrdoError::toolchain_with_help(
            codes::E0300,
            "no compiler found",
            "install clang or gcc",
        );
        let diag: &dyn Diagnostic = &err;
        assert!(diag.help().is_some());
        assert!(diag.help().unwrap().to_string().contains("install clang"));
    }

    #[test]
    fn build_error_display() {
        let err = OrdoError::build(codes::E0200, "ninja not found");
        assert_eq!(err.to_string(), "ninja not found");
    }

    #[test]
    fn deps_error_display() {
        let err = OrdoError::deps_with_help(
            codes::E0102,
            "provider not specified for 'fmt'",
            "add provider = \"vcpkg\" to the dependency",
        );
        let diag: &dyn Diagnostic = &err;
        assert!(diag.help().is_some());
    }

    #[test]
    fn test_error_display() {
        let err = OrdoError::test_err(codes::E0402, "no tests found in tests/");
        assert_eq!(err.to_string(), "no tests found in tests/");
    }

    #[test]
    fn manifest_error_converts_to_ordo_error() {
        let manifest_err = ManifestError::ValidationError {
            message: "name must not be empty".to_string(),
            help: Some("set a valid name".to_string()),
        };
        let ordo_err: OrdoError = manifest_err.into();
        assert!(matches!(ordo_err, OrdoError::Config(_)));
        assert!(ordo_err.to_string().contains("name must not be empty"));
    }
}
