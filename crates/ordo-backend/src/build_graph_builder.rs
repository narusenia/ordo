use crate::compiler::{
    self, CompileFlags, Compiler, LinkFlags, executable_extension, shared_lib_extension,
    static_lib_extension, static_lib_prefix,
};
use ordo_core::build_graph::{BuildGraph, BuildTask, LinkKind, LinkTask};
use ordo_core::manifest::PackageType;
use std::path::{Path, PathBuf};

/// Builds a `BuildGraph` from compiler, source list, and build configuration.
///
/// This replaces the logic that was previously embedded in `NinjaGenerator`
/// for computing object paths, depfiles, compile commands, and link commands.
pub struct BuildGraphBuilder<'a> {
    compiler: &'a dyn Compiler,
    sources: Vec<PathBuf>,
    build_dir: PathBuf,
    project_root: PathBuf,
    output_name: String,
    package_type: PackageType,
    compile_flags: CompileFlags,
    link_flags: LinkFlags,
    system_include_dirs: Vec<PathBuf>,
}

impl<'a> BuildGraphBuilder<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        compiler: &'a dyn Compiler,
        sources: Vec<PathBuf>,
        build_dir: PathBuf,
        project_root: PathBuf,
        output_name: String,
        package_type: PackageType,
        compile_flags: CompileFlags,
        link_flags: LinkFlags,
    ) -> Self {
        let system_include_dirs = compiler::query_system_includes(compiler.cpp_executable());
        Self {
            compiler,
            sources,
            build_dir,
            project_root,
            output_name,
            package_type,
            compile_flags,
            link_flags,
            system_include_dirs,
        }
    }

    pub fn build(self) -> BuildGraph {
        let obj_ext = if self.compiler.is_msvc() { "obj" } else { "o" };

        let mut tasks = Vec::with_capacity(self.sources.len());
        let mut object_paths = Vec::with_capacity(self.sources.len());

        for src in &self.sources {
            let abs_src = self.project_root.join(src);
            let stem = src
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let obj = PathBuf::from(format!("{stem}.{obj_ext}"));
            let depfile = PathBuf::from(format!("{stem}.d"));
            let cpp = is_cpp_source(src);

            let flags = self.compile_flags_for(cpp);
            let mut args = self.compiler.compile_args(&abs_src, &obj, &depfile, &flags);

            // Prepend compiler executable
            let exe = if cpp {
                self.compiler.cpp_executable()
            } else {
                self.compiler.c_executable()
            };
            let mut command = vec![exe.to_string()];
            command.append(&mut args);

            // Add system include dirs
            for sys_inc in &self.system_include_dirs {
                command.push("-isystem".to_string());
                command.push(sys_inc.display().to_string());
            }

            object_paths.push(obj.clone());

            tasks.push(BuildTask {
                source: src.clone(),
                object: obj,
                depfile,
                command,
                is_cpp: cpp,
            });
        }

        let link = self.build_link_task(&object_paths);

        BuildGraph {
            tasks,
            links: vec![link],
            project_root: self.project_root,
            build_dir: self.build_dir,
        }
    }

    fn build_link_task(&self, objects: &[PathBuf]) -> LinkTask {
        let output = self.compute_output_path();
        let kind = match self.package_type {
            PackageType::Executable => LinkKind::Executable,
            PackageType::StaticLibrary => LinkKind::StaticLibrary,
            PackageType::SharedLibrary => LinkKind::SharedLibrary,
        };

        let command = match self.package_type {
            PackageType::StaticLibrary => {
                if self.compiler.is_msvc() {
                    vec![
                        "lib.exe".to_string(),
                        "/nologo".to_string(),
                        format!("/OUT:{}", output.display()),
                        objects
                            .iter()
                            .map(|o| o.display().to_string())
                            .collect::<Vec<_>>()
                            .join(" "),
                    ]
                } else {
                    let mut cmd = vec![
                        "ar".to_string(),
                        "rcs".to_string(),
                        output.display().to_string(),
                    ];
                    for obj in objects {
                        cmd.push(obj.display().to_string());
                    }
                    cmd
                }
            }
            PackageType::Executable | PackageType::SharedLibrary => {
                let has_cpp = self.sources.iter().any(|s| is_cpp_source(s));
                let exe = if self.compiler.is_msvc() {
                    "link.exe"
                } else if has_cpp {
                    self.compiler.cpp_executable()
                } else {
                    self.compiler.c_executable()
                };

                let args = self.compiler.link_args(objects, &output, &self.link_flags);
                let mut cmd = vec![exe.to_string()];
                cmd.extend(args);

                // For MSVC shared library, insert /DLL
                if self.package_type == PackageType::SharedLibrary && self.compiler.is_msvc() {
                    // MSVC link_args doesn't add /DLL; handled by ninja rule.
                    // For BuildGraph commands we insert it.
                    cmd.insert(1, "/nologo".to_string());
                    cmd.insert(2, "/DLL".to_string());
                } else if self.compiler.is_msvc() {
                    cmd.insert(1, "/nologo".to_string());
                }

                if self.package_type == PackageType::SharedLibrary && !self.compiler.is_msvc() {
                    // Insert -shared right after the executable
                    cmd.insert(1, "-shared".to_string());
                }

                cmd
            }
        };

        LinkTask {
            objects: objects.to_vec(),
            output,
            command,
            kind,
        }
    }

    fn compute_output_path(&self) -> PathBuf {
        let output_dir = self.build_dir.parent().unwrap_or(Path::new("."));
        match self.package_type {
            PackageType::Executable => {
                output_dir.join(format!("{}{}", self.output_name, executable_extension()))
            }
            PackageType::StaticLibrary => output_dir.join(format!(
                "{}{}{}",
                static_lib_prefix(),
                self.output_name,
                static_lib_extension()
            )),
            PackageType::SharedLibrary => output_dir.join(format!(
                "{}{}{}",
                static_lib_prefix(),
                self.output_name,
                shared_lib_extension()
            )),
        }
    }

    fn compile_flags_for(&self, cpp: bool) -> CompileFlags {
        let mut flags = self.compile_flags.clone();
        if cpp {
            flags.c_standard = None;
        } else {
            flags.cpp_standard = None;
        }
        flags
    }
}

fn is_cpp_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("cpp" | "cc" | "cxx" | "C")
    )
}
