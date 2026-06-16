use super::{CompileFlags, Compiler, LinkFlags};
use crate::core::manifest::CppStandard;
use std::path::{Path, PathBuf};

pub struct MsvcCompiler;

impl Compiler for MsvcCompiler {
    fn name(&self) -> &str {
        "msvc"
    }

    fn c_executable(&self) -> &str {
        "cl"
    }

    fn cpp_executable(&self) -> &str {
        "cl"
    }

    fn compile_args(&self, src: &Path, obj: &Path, depfile: &Path, flags: &CompileFlags) -> Vec<String> {
        let mut args = vec!["/c".to_string(), "/nologo".to_string()];

        if let Some(std) = flags.cpp_standard {
            let flag = match std {
                CppStandard::Cpp17 => "/std:c++17",
                CppStandard::Cpp20 => "/std:c++20",
                CppStandard::Cpp23 | CppStandard::Cpp26 => "/std:c++latest",
            };
            args.push(flag.to_string());
        }

        match flags.opt_level {
            0 => args.push("/Od".to_string()),
            1 => args.push("/O1".to_string()),
            _ => args.push("/O2".to_string()),
        }

        if flags.debug {
            args.push("/Zi".to_string());
        }

        for def in &flags.defines {
            args.push(format!("/D{def}"));
        }

        for inc in &flags.include_dirs {
            args.push(format!("/I{}", inc.display()));
        }

        // MSVC depfile via /showIncludes (parsed by Ninja's msvc_deps_prefix)
        args.push("/showIncludes".to_string());

        args.push(format!("/Fo{}", obj.display()));
        args.push(src.display().to_string());

        // depfile path is tracked but MSVC uses /showIncludes instead of -MF
        let _ = depfile;

        args
    }

    fn link_args(&self, objects: &[PathBuf], output: &Path, flags: &LinkFlags) -> Vec<String> {
        let mut args = vec!["/nologo".to_string()];

        args.push(format!("/OUT:{}", output.display()));

        for dir in &flags.lib_dirs {
            args.push(format!("/LIBPATH:{}", dir.display()));
        }

        for obj in objects {
            args.push(obj.display().to_string());
        }

        for lib in &flags.libs {
            args.push(format!("{lib}.lib"));
        }

        args
    }

    fn syntax_only_flag(&self) -> &str {
        "/Zs"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_args_basic() {
        let c = MsvcCompiler;
        let flags = CompileFlags {
            cpp_standard: Some(CppStandard::Cpp20),
            opt_level: 0,
            debug: true,
            ..CompileFlags::default()
        };

        let args = c.compile_args(
            Path::new("src\\main.cpp"),
            Path::new("build\\main.obj"),
            Path::new("build\\main.d"),
            &flags,
        );

        assert!(args.contains(&"/c".to_string()));
        assert!(args.contains(&"/std:c++20".to_string()));
        assert!(args.contains(&"/Od".to_string()));
        assert!(args.contains(&"/Zi".to_string()));
        assert!(args.contains(&"/showIncludes".to_string()));
    }

    #[test]
    fn compile_args_release() {
        let c = MsvcCompiler;
        let flags = CompileFlags {
            cpp_standard: Some(CppStandard::Cpp20),
            opt_level: 3,
            debug: false,
            ..CompileFlags::default()
        };

        let args = c.compile_args(
            Path::new("src\\main.cpp"),
            Path::new("build\\main.obj"),
            Path::new("build\\main.d"),
            &flags,
        );

        assert!(args.contains(&"/O2".to_string()));
        assert!(!args.contains(&"/Zi".to_string()));
    }

    #[test]
    fn link_args_basic() {
        let c = MsvcCompiler;
        let flags = LinkFlags {
            libs: vec!["user32".to_string()],
            ..LinkFlags::default()
        };

        let args = c.link_args(
            &[PathBuf::from("build\\main.obj")],
            Path::new("myapp.exe"),
            &flags,
        );

        assert!(args.contains(&"/nologo".to_string()));
        assert!(args.contains(&"/OUT:myapp.exe".to_string()));
        assert!(args.contains(&"user32.lib".to_string()));
    }
}
