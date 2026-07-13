#![allow(dead_code)]

pub mod clang;
pub mod gcc;
pub mod msvc;

use crate::core::manifest::{
    CStandard, CompilerKind, CppStandard, LinkerKind, LtoMode, OptLevel, Sanitizer, WarningLevel,
};

pub fn san_flag(s: &Sanitizer) -> &'static str {
    match s {
        Sanitizer::Address => "address",
        Sanitizer::Undefined => "undefined",
        Sanitizer::Thread => "thread",
        Sanitizer::Memory => "memory",
    }
}
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CompileFlags {
    pub cpp_standard: Option<CppStandard>,
    pub c_standard: Option<CStandard>,
    pub opt_level: OptLevel,
    pub debug: bool,
    pub assertions: bool,
    pub sanitize: Vec<Sanitizer>,
    pub pic: bool,
    pub rtti: bool,
    pub exceptions: bool,
    pub warnings: WarningLevel,
    pub coverage: bool,
    pub split_debug: bool,
    pub defines: Vec<String>,
    pub include_dirs: Vec<PathBuf>,
}

impl Default for CompileFlags {
    fn default() -> Self {
        Self {
            cpp_standard: Some(CppStandard::Cpp20),
            c_standard: None,
            opt_level: OptLevel::O0,
            debug: true,
            assertions: true,
            sanitize: Vec::new(),
            pic: false,
            rtti: true,
            exceptions: true,
            warnings: WarningLevel::All,
            coverage: false,
            split_debug: false,
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
    pub lto: LtoMode,
    pub strip: bool,
    pub static_runtime: bool,
    pub sanitize: Vec<Sanitizer>,
    pub coverage: bool,
}

pub trait Compiler {
    fn name(&self) -> &str;
    fn c_executable(&self) -> &str;
    fn cpp_executable(&self) -> &str;
    fn compile_args(
        &self,
        src: &Path,
        obj: &Path,
        depfile: &Path,
        flags: &CompileFlags,
    ) -> Vec<String>;
    fn link_args(&self, objects: &[PathBuf], output: &Path, flags: &LinkFlags) -> Vec<String>;
    fn syntax_only_flag(&self) -> &str;
    fn is_msvc(&self) -> bool {
        false
    }
}

pub fn executable_extension() -> &'static str {
    if cfg!(windows) { ".exe" } else { "" }
}

pub fn static_lib_extension() -> &'static str {
    if cfg!(windows) { ".lib" } else { ".a" }
}

pub fn static_lib_prefix() -> &'static str {
    if cfg!(windows) { "" } else { "lib" }
}

pub fn shared_lib_extension() -> &'static str {
    if cfg!(target_os = "macos") {
        ".dylib"
    } else if cfg!(windows) {
        ".dll"
    } else {
        ".so"
    }
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
    if kind == CompilerKind::Msvc {
        return try_detect_msvc(exe);
    }

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

fn try_detect_msvc(exe: &str) -> Option<DetectedCompiler> {
    // cl.exe prints version info to stderr when run with no source file.
    // It returns non-zero, but that's expected.
    let output = std::process::Command::new(exe)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.contains("Microsoft") && !stderr.contains("cl") {
        return None;
    }

    let version = parse_version(&stderr).unwrap_or_else(|| "unknown".to_string());
    let path = which(exe)?;

    Some(DetectedCompiler {
        kind: CompilerKind::Msvc,
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
    #[cfg(windows)]
    let cmd = "where";
    #[cfg(not(windows))]
    let cmd = "which";

    let output = std::process::Command::new(cmd).arg(exe).output().ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // `where` on Windows can return multiple lines; take the first
        let path = stdout.lines().next()?.trim().to_string();
        if path.is_empty() {
            return None;
        }
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

    #[test]
    fn parse_version_msvc() {
        let output = "Microsoft (R) C/C++ Optimizing Compiler Version 19.38.33133 for x64";
        assert_eq!(parse_version(output), Some("19.38.33133".to_string()));
    }

    #[test]
    fn which_finds_existing_binary() {
        // `sh` exists on all Unix systems; `cmd` on Windows
        #[cfg(not(windows))]
        let exe = "sh";
        #[cfg(windows)]
        let exe = "cmd";
        assert!(which(exe).is_some());
    }

    #[test]
    fn which_returns_none_for_nonexistent() {
        assert!(which("__ordo_nonexistent_binary_12345__").is_none());
    }
}
