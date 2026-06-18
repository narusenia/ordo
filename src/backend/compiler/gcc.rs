use super::{CompileFlags, Compiler, LtoMode, LinkFlags};
use crate::core::manifest::{Sanitizer, WarningLevel};
use std::path::{Path, PathBuf};

fn san_flag(s: &Sanitizer) -> &'static str {
    match s {
        Sanitizer::Address => "address",
        Sanitizer::Undefined => "undefined",
        Sanitizer::Thread => "thread",
        Sanitizer::Memory => "memory",
    }
}

pub struct GccCompiler;

impl Compiler for GccCompiler {
    fn name(&self) -> &str {
        "gcc"
    }

    fn c_executable(&self) -> &str {
        "gcc"
    }

    fn cpp_executable(&self) -> &str {
        "g++"
    }

    fn compile_args(
        &self,
        src: &Path,
        obj: &Path,
        depfile: &Path,
        flags: &CompileFlags,
    ) -> Vec<String> {
        let mut args = vec!["-c".to_string()];

        if let Some(std) = flags.cpp_standard {
            args.push(format!("-std={}", std.as_flag()));
        } else if let Some(std) = flags.c_standard {
            args.push(format!("-std={}", std.as_flag()));
        }

        args.push(format!("-O{}", flags.opt_level.as_flag()));

        if flags.debug {
            args.push("-g".to_string());
        }

        if flags.split_debug {
            args.push("-gsplit-dwarf".to_string());
        }

        if !flags.assertions {
            args.push("-DNDEBUG".to_string());
        }

        for san in &flags.sanitize {
            args.push(format!("-fsanitize={}", san_flag(san)));
        }

        if flags.pic {
            args.push("-fPIC".to_string());
        }

        if flags.cpp_standard.is_some() {
            if !flags.rtti {
                args.push("-fno-rtti".to_string());
            }
            if !flags.exceptions {
                args.push("-fno-exceptions".to_string());
            }
        }

        match flags.warnings {
            WarningLevel::Default => {}
            WarningLevel::All => args.push("-Wall".to_string()),
            WarningLevel::Extra => {
                args.push("-Wall".to_string());
                args.push("-Wextra".to_string());
            }
            WarningLevel::Error => {
                args.push("-Wall".to_string());
                args.push("-Wextra".to_string());
                args.push("-Werror".to_string());
            }
        }

        if flags.coverage {
            args.push("--coverage".to_string());
        }

        for def in &flags.defines {
            args.push(format!("-D{def}"));
        }

        for inc in &flags.include_dirs {
            args.push(format!("-I{}", inc.display()));
        }

        args.push("-MD".to_string());
        args.push("-MF".to_string());
        args.push(depfile.display().to_string());

        args.push("-o".to_string());
        args.push(obj.display().to_string());
        args.push(src.display().to_string());

        args
    }

    fn link_args(&self, objects: &[PathBuf], output: &Path, flags: &LinkFlags) -> Vec<String> {
        let mut args = Vec::new();

        if let Some(linker) = flags.linker {
            args.push(format!("-fuse-ld={linker}"));
        }

        match flags.lto {
            LtoMode::Off => {}
            LtoMode::Thin => args.push("-flto=auto".to_string()),
            LtoMode::Full => args.push("-flto".to_string()),
        }

        if flags.strip {
            args.push("-s".to_string());
        }

        if flags.static_runtime {
            args.push("-static".to_string());
        }

        for san in &flags.sanitize {
            args.push(format!("-fsanitize={}", san_flag(san)));
        }

        if flags.coverage {
            args.push("--coverage".to_string());
        }

        for dir in &flags.lib_dirs {
            args.push(format!("-L{}", dir.display()));
        }

        args.push("-o".to_string());
        args.push(output.display().to_string());

        for obj in objects {
            args.push(obj.display().to_string());
        }

        for lib in &flags.libs {
            args.push(format!("-l{lib}"));
        }

        for fw in &flags.frameworks {
            args.push("-framework".to_string());
            args.push(fw.clone());
        }

        args
    }

    fn syntax_only_flag(&self) -> &str {
        "-fsyntax-only"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::{CppStandard, OptLevel};

    #[test]
    fn compile_args_basic() {
        let c = GccCompiler;
        let flags = CompileFlags {
            cpp_standard: Some(CppStandard::Cpp23),
            opt_level: OptLevel::O2,
            debug: false,
            ..CompileFlags::default()
        };

        let args = c.compile_args(
            Path::new("src/main.cpp"),
            Path::new("build/main.o"),
            Path::new("build/main.d"),
            &flags,
        );

        assert!(args.contains(&"-std=c++23".to_string()));
        assert!(args.contains(&"-O2".to_string()));
        assert!(!args.contains(&"-g".to_string()));
    }

    #[test]
    fn link_args_basic() {
        let c = GccCompiler;
        let args = c.link_args(
            &[PathBuf::from("build/main.o")],
            Path::new("myapp"),
            &LinkFlags::default(),
        );

        assert!(args.contains(&"-o".to_string()));
        assert!(args.contains(&"myapp".to_string()));
    }
}
