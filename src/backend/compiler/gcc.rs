use super::{CompileFlags, Compiler, LinkFlags};
use std::path::{Path, PathBuf};

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

        args.push(format!("-O{}", flags.opt_level));

        if flags.debug {
            args.push("-g".to_string());
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
    use crate::core::manifest::CppStandard;

    #[test]
    fn compile_args_basic() {
        let c = GccCompiler;
        let flags = CompileFlags {
            cpp_standard: Some(CppStandard::Cpp23),
            opt_level: 2,
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
