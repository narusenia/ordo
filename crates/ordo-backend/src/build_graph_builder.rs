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

pub struct TestBinarySpec {
    pub name: String,
    pub test_source: PathBuf,
    pub framework_libs: Vec<String>,
    pub framework_lib_dirs: Vec<PathBuf>,
    pub framework_include_dirs: Vec<PathBuf>,
}

pub struct TestBuildOutput {
    pub graph: BuildGraph,
    pub test_binaries: Vec<(String, PathBuf)>,
}

pub struct TestBuildGraphBuilder<'a> {
    compiler: &'a dyn Compiler,
    build_dir: PathBuf,
    project_root: PathBuf,
    compile_flags: CompileFlags,
    link_flags: LinkFlags,
    lib_sources: Vec<PathBuf>,
    lib_name: Option<String>,
    project_lib_path: Option<PathBuf>,
    tests: Vec<TestBinarySpec>,
    system_include_dirs: Vec<PathBuf>,
}

impl<'a> TestBuildGraphBuilder<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        compiler: &'a dyn Compiler,
        build_dir: PathBuf,
        project_root: PathBuf,
        compile_flags: CompileFlags,
        link_flags: LinkFlags,
        lib_sources: Vec<PathBuf>,
        lib_name: Option<String>,
        project_lib_path: Option<PathBuf>,
        tests: Vec<TestBinarySpec>,
    ) -> Self {
        let system_include_dirs = compiler::query_system_includes(compiler.cpp_executable());
        Self {
            compiler,
            build_dir,
            project_root,
            compile_flags,
            link_flags,
            lib_sources,
            lib_name,
            project_lib_path,
            tests,
            system_include_dirs,
        }
    }

    pub fn build(self) -> TestBuildOutput {
        let obj_ext = if self.compiler.is_msvc() { "obj" } else { "o" };
        let mut tasks = Vec::new();
        let mut links = Vec::new();
        let mut test_binaries = Vec::new();

        // Compile library sources
        let mut lib_objects = Vec::new();
        for src in &self.lib_sources {
            let abs_src = self.project_root.join(src);
            let stem = src.file_stem().unwrap_or_default().to_string_lossy();
            let obj = PathBuf::from(format!("lib_{stem}.{obj_ext}"));
            let depfile = PathBuf::from(format!("lib_{stem}.d"));
            let cpp = is_cpp_source(src);
            let flags = self.compile_flags_for(cpp, &[]);

            let exe = if cpp {
                self.compiler.cpp_executable()
            } else {
                self.compiler.c_executable()
            };
            let mut args = self.compiler.compile_args(&abs_src, &obj, &depfile, &flags);
            let mut command = vec![exe.to_string()];
            command.append(&mut args);
            self.append_system_includes(&mut command);

            lib_objects.push(obj.clone());
            tasks.push(BuildTask {
                source: src.clone(),
                object: obj,
                depfile,
                command,
                is_cpp: cpp,
            });
        }

        // Archive library if we have lib sources
        let lib_archive = if !lib_objects.is_empty() {
            if let Some(ref lib_name) = self.lib_name {
                let lib_ext = if self.compiler.is_msvc() { "lib" } else { "a" };
                let lib_prefix = if self.compiler.is_msvc() { "" } else { "lib" };
                let lib_file = PathBuf::from(format!("{lib_prefix}{lib_name}.{lib_ext}"));

                let command = if self.compiler.is_msvc() {
                    let mut cmd = vec![
                        "lib.exe".to_string(),
                        "/nologo".to_string(),
                        format!("/OUT:{}", lib_file.display()),
                    ];
                    for obj in &lib_objects {
                        cmd.push(obj.display().to_string());
                    }
                    cmd
                } else {
                    let mut cmd = vec![
                        "ar".to_string(),
                        "rcs".to_string(),
                        lib_file.display().to_string(),
                    ];
                    for obj in &lib_objects {
                        cmd.push(obj.display().to_string());
                    }
                    cmd
                };

                links.push(LinkTask {
                    objects: lib_objects,
                    output: lib_file.clone(),
                    command,
                    kind: LinkKind::StaticLibrary,
                });
                Some(lib_file)
            } else {
                None
            }
        } else {
            None
        };

        let exe_ext = executable_extension();
        let output_dir = self.build_dir.parent().unwrap_or(Path::new("."));

        // Build each test binary
        for test in &self.tests {
            let abs_src = self.project_root.join(&test.test_source);
            let obj = PathBuf::from(format!("test_{}.{obj_ext}", test.name));
            let depfile = PathBuf::from(format!("test_{}.d", test.name));
            let cpp = is_cpp_source(&test.test_source);
            let flags = self.compile_flags_for(cpp, &test.framework_include_dirs);

            let exe = if cpp {
                self.compiler.cpp_executable()
            } else {
                self.compiler.c_executable()
            };
            let mut args = self.compiler.compile_args(&abs_src, &obj, &depfile, &flags);
            let mut command = vec![exe.to_string()];
            command.append(&mut args);
            self.append_system_includes(&mut command);

            tasks.push(BuildTask {
                source: test.test_source.clone(),
                object: obj.clone(),
                depfile,
                command,
                is_cpp: cpp,
            });

            // Link test binary
            let bin_path = output_dir.join(format!("{}{exe_ext}", test.name));

            let mut link_objects = vec![obj];
            if let Some(ref lib_file) = lib_archive {
                link_objects.push(lib_file.clone());
            }
            if let Some(ref project_lib) = self.project_lib_path {
                link_objects.push(project_lib.clone());
            }

            let has_cpp_test = cpp || self.lib_sources.iter().any(|s| is_cpp_source(s));
            let link_exe = if self.compiler.is_msvc() {
                "link.exe"
            } else if has_cpp_test {
                self.compiler.cpp_executable()
            } else {
                self.compiler.c_executable()
            };

            let mut combined_link_flags = self.link_flags.clone();
            combined_link_flags
                .lib_dirs
                .extend(test.framework_lib_dirs.iter().cloned());
            combined_link_flags
                .libs
                .extend(test.framework_libs.iter().cloned());

            let link_args = self
                .compiler
                .link_args(&link_objects, &bin_path, &combined_link_flags);
            let mut link_command = vec![link_exe.to_string()];
            if self.compiler.is_msvc() {
                link_command.push("/nologo".to_string());
            }
            link_command.extend(link_args);

            links.push(LinkTask {
                objects: link_objects,
                output: bin_path.clone(),
                command: link_command,
                kind: LinkKind::Executable,
            });

            test_binaries.push((test.name.clone(), bin_path));
        }

        TestBuildOutput {
            graph: BuildGraph {
                tasks,
                links,
                project_root: self.project_root,
                build_dir: self.build_dir,
            },
            test_binaries,
        }
    }

    fn compile_flags_for(&self, cpp: bool, extra_include_dirs: &[PathBuf]) -> CompileFlags {
        let mut flags = self.compile_flags.clone();
        if cpp {
            flags.c_standard = None;
        } else {
            flags.cpp_standard = None;
        }
        for inc in extra_include_dirs {
            flags.include_dirs.push(inc.clone());
        }
        flags
    }

    fn append_system_includes(&self, command: &mut Vec<String>) {
        for sys_inc in &self.system_include_dirs {
            command.push("-isystem".to_string());
            command.push(sys_inc.display().to_string());
        }
    }
}
