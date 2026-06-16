#![allow(dead_code)]

pub mod clang;
pub mod gcc;
pub mod msvc;

use crate::core::manifest::{CompilerKind, CppStandard, CStandard, LinkerKind};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CompileFlags {
    pub cpp_standard: Option<CppStandard>,
    pub c_standard: Option<CStandard>,
    pub opt_level: u8,
    pub debug: bool,
    pub defines: Vec<String>,
    pub include_dirs: Vec<PathBuf>,
}

impl Default for CompileFlags {
    fn default() -> Self {
        Self {
            cpp_standard: Some(CppStandard::Cpp20),
            c_standard: None,
            opt_level: 0,
            debug: true,
            defines: Vec::new(),
            include_dirs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LinkFlags {
    pub lib_dirs: Vec<PathBuf>,
    pub libs: Vec<String>,
    pub frameworks: Vec<String>,
    pub linker: Option<LinkerKind>,
}

pub trait Compiler {
    fn name(&self) -> &str;
    fn executable(&self) -> &str;
    fn compile_args(&self, src: &Path, obj: &Path, depfile: &Path, flags: &CompileFlags) -> Vec<String>;
    fn link_args(&self, objects: &[PathBuf], output: &Path, flags: &LinkFlags) -> Vec<String>;
    fn syntax_only_flag(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct DetectedCompiler {
    pub kind: CompilerKind,
    pub path: PathBuf,
    pub version: String,
}

pub fn detect_compiler() -> Option<DetectedCompiler> {
    // Priority: clang → gcc → msvc
    if let Some(c) = try_detect("clang++", CompilerKind::Clang) {
        return Some(c);
    }
    if let Some(c) = try_detect("g++", CompilerKind::Gcc) {
        return Some(c);
    }
    if let Some(c) = try_detect("cl", CompilerKind::Msvc) {
        return Some(c);
    }
    None
}

pub fn detect_specific(kind: CompilerKind) -> Option<DetectedCompiler> {
    let exe = match kind {
        CompilerKind::Clang => "clang++",
        CompilerKind::Gcc => "g++",
        CompilerKind::Msvc => "cl",
        CompilerKind::ClangCl => "clang-cl",
    };
    try_detect(exe, kind)
}

fn try_detect(exe: &str, kind: CompilerKind) -> Option<DetectedCompiler> {
    let output = std::process::Command::new(exe)
        .arg("--version")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = parse_version(&stdout).unwrap_or_else(|| "unknown".to_string());

    let path = which(exe)?;

    Some(DetectedCompiler {
        kind,
        path,
        version,
    })
}

fn parse_version(output: &str) -> Option<String> {
    for word in output.split_whitespace() {
        // Strip trailing non-version chars, then take only digits and dots
        let candidate = word.split('-').next().unwrap_or(word);
        let parts: Vec<&str> = candidate.split('.').collect();
        if parts.len() >= 2 && parts.iter().all(|p| p.parse::<u32>().is_ok()) {
            return Some(candidate.to_string());
        }
    }
    None
}

fn which(exe: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("which")
        .arg(exe)
        .output()
        .ok()?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Some(PathBuf::from(path))
    } else {
        None
    }
}

pub fn create_compiler(kind: CompilerKind) -> Box<dyn Compiler> {
    match kind {
        CompilerKind::Clang | CompilerKind::ClangCl => Box::new(clang::ClangCompiler),
        CompilerKind::Gcc => Box::new(gcc::GccCompiler),
        CompilerKind::Msvc => Box::new(msvc::MsvcCompiler),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_clang() {
        let output = "Homebrew clang version 18.1.8\nTarget: arm64-apple-darwin24.0.0";
        assert_eq!(parse_version(output), Some("18.1.8".to_string()));
    }

    #[test]
    fn parse_version_gcc() {
        let output = "g++ (Ubuntu 13.2.0-23ubuntu4) 13.2.0";
        assert_eq!(parse_version(output), Some("13.2.0".to_string()));
    }

    #[test]
    fn parse_version_none() {
        assert_eq!(parse_version("no version here"), None);
    }

    #[test]
    fn create_compiler_returns_correct_type() {
        let c = create_compiler(CompilerKind::Clang);
        assert_eq!(c.name(), "clang");

        let c = create_compiler(CompilerKind::Gcc);
        assert_eq!(c.name(), "gcc");

        let c = create_compiler(CompilerKind::Msvc);
        assert_eq!(c.name(), "msvc");
    }
}
