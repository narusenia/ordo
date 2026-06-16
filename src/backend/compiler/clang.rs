use super::{CompileFlags, Compiler, LinkFlags};
use std::path::{Path, PathBuf};

pub struct ClangCompiler;

impl Compiler for ClangCompiler {
    fn name(&self) -> &str {
        "clang"
    }

    fn executable(&self) -> &str {
        "clang++"
    }

    fn compile_args(&self, src: &Path, obj: &Path, depfile: &Path, flags: &CompileFlags) -> Vec<String> {
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

        // Depfile for incremental builds
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
        let c = ClangCompiler;
        let flags = CompileFlags {
            cpp_standard: Some(CppStandard::Cpp20),
            opt_level: 0,
            debug: true,
            ..CompileFlags::default()
        };

        let args = c.compile_args(
            Path::new("src/main.cpp"),
            Path::new("build/main.o"),
            Path::new("build/main.d"),
            &flags,
        );

        assert!(args.contains(&"-c".to_string()));
        assert!(args.contains(&"-std=c++20".to_string()));
        assert!(args.contains(&"-O0".to_string()));
        assert!(args.contains(&"-g".to_string()));
        assert!(args.contains(&"-MD".to_string()));
        assert!(args.contains(&"-MF".to_string()));
        assert!(args.contains(&"build/main.d".to_string()));
        assert!(args.contains(&"src/main.cpp".to_string()));
    }

    #[test]
    fn compile_args_with_includes_and_defines() {
        let c = ClangCompiler;
        let flags = CompileFlags {
            defines: vec!["FOO=1".to_string(), "BAR".to_string()],
            include_dirs: vec![PathBuf::from("include"), PathBuf::from("vendor/fmt/include")],
            ..CompileFlags::default()
        };

        let args = c.compile_args(
            Path::new("src/main.cpp"),
            Path::new("build/main.o"),
            Path::new("build/main.d"),
            &flags,
        );

        assert!(args.contains(&"-DFOO=1".to_string()));
        assert!(args.contains(&"-DBAR".to_string()));
        assert!(args.contains(&"-Iinclude".to_string()));
        assert!(args.contains(&"-Ivendor/fmt/include".to_string()));
    }

    #[test]
    fn link_args_basic() {
        let c = ClangCompiler;
        let objects = vec![PathBuf::from("build/main.o"), PathBuf::from("build/util.o")];
        let flags = LinkFlags::default();

        let args = c.link_args(&objects, Path::new("myapp"), &flags);

        assert!(args.contains(&"-o".to_string()));
        assert!(args.contains(&"myapp".to_string()));
        assert!(args.contains(&"build/main.o".to_string()));
        assert!(args.contains(&"build/util.o".to_string()));
    }

    #[test]
    fn link_args_with_linker() {
        let c = ClangCompiler;
        let flags = LinkFlags {
            linker: Some(crate::core::manifest::LinkerKind::Lld),
            libs: vec!["fmt".to_string()],
            lib_dirs: vec![PathBuf::from("/usr/local/lib")],
        };

        let args = c.link_args(&[PathBuf::from("build/main.o")], Path::new("myapp"), &flags);

        assert!(args.contains(&"-fuse-ld=lld".to_string()));
        assert!(args.contains(&"-lfmt".to_string()));
        assert!(args.contains(&"-L/usr/local/lib".to_string()));
    }
}
