use super::build::{DEP_CACHE_FILE, load_dep_cache};
use super::context::Context;
use miette::{IntoDiagnostic, Result, bail};
use ordo_backend::build_graph_builder::{
    TestBinarySpec as BuildGraphTestSpec, TestBuildGraphBuilder,
};
use ordo_backend::compiler::{self, CompileFlags, LinkFlags};
use ordo_backend::ninja::{TestBinarySpec, TestNinjaGenerator};
use ordo_backend::provider::FetchedDep;
use ordo_core::manifest::{CompilerKind, CppStandard, Manifest, PackageType, TestFramework};
use ordo_core::tester;
use ordo_core::workspace::Workspace;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

pub struct TestOptions {
    pub filter: Option<String>,
    pub jobs: Option<u32>,
    pub release: bool,
    pub profile: Option<String>,
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub all_features: bool,
    pub package: Option<String>,
    #[allow(dead_code)]
    pub verbose: u8,
}

struct TestResult {
    name: String,
    passed: bool,
    output: String,
}

pub fn run(opts: &TestOptions, ctx: &Context) -> Result<()> {
    let project_root = std::env::current_dir().into_diagnostic()?;
    let manifest_path = project_root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", project_root.display());
    }

    let manifest = Manifest::load(&manifest_path)?;

    if manifest.is_workspace() {
        return run_workspace_tests(opts, &project_root, ctx);
    }

    run_single_tests(opts, &project_root, None, ctx)
}

fn run_workspace_tests(opts: &TestOptions, root_dir: &Path, ctx: &Context) -> Result<()> {
    let ws = Workspace::load(root_dir)?;

    let members: Vec<String> = if let Some(ref target) = opts.package {
        if ws.find_member(target).is_none() {
            let available = ws.member_names().join(", ");
            bail!(
                "package '{}' not found in workspace; available members: {}",
                target,
                available
            );
        }
        vec![target.clone()]
    } else {
        ws.member_names().into_iter().map(String::from).collect()
    };

    let mut total_pass = 0u32;
    let mut total_fail = 0u32;

    for name in &members {
        let member = ws.find_member(name).unwrap();
        let member_dir = root_dir.join(&member.dir);
        match run_single_tests(opts, &member_dir, Some(root_dir), ctx) {
            Ok(()) => total_pass += 1,
            Err(e) => {
                ctx.style.error("Failed", &format!("{name}: {e}"));
                total_fail += 1;
            }
        }
    }

    if total_fail > 0 {
        bail!(
            "test failed: {} member(s) passed, {} failed",
            total_pass,
            total_fail
        );
    }

    Ok(())
}

fn run_single_tests(
    opts: &TestOptions,
    project_root: &Path,
    workspace_root: Option<&Path>,
    ctx: &Context,
) -> Result<()> {
    let manifest_path = project_root.join("Ordo.toml");
    let manifest = Manifest::load(&manifest_path)?;

    let pkg = manifest.package.as_ref().ok_or_else(|| {
        miette::miette!(
            "cannot test '{}': no [package] section",
            project_root.display()
        )
    })?;

    let test_config = &manifest.test;
    let test_targets = tester::discover_tests(
        project_root,
        test_config.src.as_deref(),
        test_config.framework,
    )?;

    if test_targets.is_empty() {
        ctx.style
            .warn("Warning", &format!("no tests found for `{}`", pkg.name));
        return Ok(());
    }

    let filtered: Vec<_> = if let Some(ref filter) = opts.filter {
        test_targets
            .into_iter()
            .filter(|t| t.name.contains(filter.as_str()))
            .collect()
    } else {
        test_targets
    };

    if filtered.is_empty() {
        ctx.style.warn(
            "Warning",
            &format!(
                "no tests match filter '{}'",
                opts.filter.as_deref().unwrap_or("")
            ),
        );
        return Ok(());
    }

    let profile_name = resolve_profile_name(opts);
    let target_dir = workspace_root.unwrap_or(project_root).join("target");
    let profile_dir = if workspace_root.is_some() {
        target_dir.join(&profile_name).join(&pkg.name)
    } else {
        target_dir.join(&profile_name)
    };
    let test_build_dir = profile_dir.join("test-build");
    fs::create_dir_all(&test_build_dir).into_diagnostic()?;
    fs::create_dir_all(&profile_dir).into_diagnostic()?;

    let _lock = crate::lock::BuildLock::acquire(&test_build_dir, ctx)?;

    let compiler_kind = manifest
        .toolchain
        .compiler
        .unwrap_or_else(auto_detect_compiler);
    let compiler = compiler::create_compiler(compiler_kind);

    let fetched_deps = load_cached_deps(&profile_dir);
    let compile_flags =
        build_test_compile_flags(&manifest, project_root, &profile_name, opts, &fetched_deps);
    let link_flags = build_test_link_flags(&manifest, &profile_name, &fetched_deps);

    let test_lib = tester::extract_test_library(project_root, &pkg.name, pkg.package_type)?;

    let project_lib_path = match pkg.package_type {
        PackageType::StaticLibrary => {
            let lib_path = profile_dir.join(format!("lib{}.a", pkg.name));
            if lib_path.exists() {
                Some(lib_path)
            } else {
                None
            }
        }
        PackageType::SharedLibrary => {
            let lib_path = profile_dir.join(format!("lib{}.so", pkg.name));
            if lib_path.exists() {
                Some(lib_path)
            } else {
                None
            }
        }
        PackageType::Executable => None,
    };

    let canonical_root = fs::canonicalize(project_root).into_diagnostic()?;
    let lib_sources = test_lib
        .as_ref()
        .map(|l| l.sources.clone())
        .unwrap_or_default();
    let lib_name_opt = test_lib.as_ref().map(|l| l.lib_name.clone());

    let engine = manifest.build.engine.unwrap_or_default();

    ctx.style.header(&format!("Testing {}", pkg.name));
    let start = Instant::now();

    let test_binaries: Vec<(String, PathBuf)> = match engine {
        ordo_core::manifest::BuildEngine::Ninja => {
            let test_specs: Vec<TestBinarySpec> = filtered
                .iter()
                .map(|t| {
                    let (fw_libs, fw_lib_dirs, fw_include_dirs) = framework_link_info(t.framework);
                    TestBinarySpec {
                        name: t.name.clone(),
                        test_source: t.source.clone(),
                        framework_libs: fw_libs,
                        framework_lib_dirs: fw_lib_dirs,
                        framework_include_dirs: fw_include_dirs,
                    }
                })
                .collect();

            let ninja_gen = TestNinjaGenerator::new(
                compiler.as_ref(),
                test_build_dir.clone(),
                canonical_root,
                compile_flags,
                link_flags,
                lib_sources,
                lib_name_opt,
                project_lib_path,
                test_specs,
            );

            let output = ninja_gen.generate();
            fs::write(test_build_dir.join("build.ninja"), &output.build_ninja).into_diagnostic()?;
            invoke_ninja(
                &test_build_dir,
                opts.jobs,
                manifest.toolchain.ninja.as_deref(),
                ctx,
            )?;
            output.test_binaries
        }
        ordo_core::manifest::BuildEngine::Faber => {
            use ordo_faber::FaberEvent;

            let test_specs: Vec<BuildGraphTestSpec> = filtered
                .iter()
                .map(|t| {
                    let (fw_libs, fw_lib_dirs, fw_include_dirs) = framework_link_info(t.framework);
                    BuildGraphTestSpec {
                        name: t.name.clone(),
                        test_source: t.source.clone(),
                        framework_libs: fw_libs,
                        framework_lib_dirs: fw_lib_dirs,
                        framework_include_dirs: fw_include_dirs,
                    }
                })
                .collect();

            let test_output = TestBuildGraphBuilder::new(
                compiler.as_ref(),
                test_build_dir.clone(),
                canonical_root,
                compile_flags,
                link_flags,
                lib_sources,
                lib_name_opt,
                project_lib_path,
                test_specs,
            )
            .build();

            ctx.style.warn("Note", "Faber build engine is beta");
            let faber = ordo_faber::FaberEngine::new(opts.jobs, None);
            let result = faber.execute(&test_output.graph, opts.verbose, &|event| match event {
                FaberEvent::Compiled {
                    file,
                    current,
                    total,
                } => {
                    ctx.style
                        .success("Compiled", &format!("{file} [{current}/{total}]"));
                }
                FaberEvent::CompileFailed { file, stderr } => {
                    ctx.style.error("Failed", &file);
                    if !stderr.is_empty() {
                        for line in stderr.lines() {
                            eprintln!("  {line}");
                        }
                    }
                }
                FaberEvent::Linking { file } => {
                    ctx.style.success("Linking", &file);
                }
                FaberEvent::Linked { file } => {
                    ctx.style.success("Linked", &file);
                }
                FaberEvent::LinkFailed { stderr } => {
                    ctx.style.error("Failed", "linking");
                    if !stderr.is_empty() {
                        for line in stderr.lines() {
                            eprintln!("  {line}");
                        }
                    }
                }
            })?;

            if !result.success {
                let err_count = result.errors.len();
                bail!("test build failed with {err_count} error(s)");
            }

            test_output.test_binaries
        }
    };

    let results = run_test_binaries(&test_binaries, opts.filter.as_deref(), &filtered, ctx)?;
    let elapsed = start.elapsed();

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();

    for r in &results {
        if !r.passed {
            ctx.style.error("Failed", &r.name);
            if !r.output.is_empty() {
                for line in r.output.lines() {
                    eprintln!("  {line}");
                }
            }
        }
    }

    ctx.style.summary_bar();

    if failed > 0 {
        ctx.style.error(
            "Tests",
            &format!(
                "{passed} passed, {failed} failed in {:.2}s",
                elapsed.as_secs_f64()
            ),
        );
        bail!("test failed: {failed} of {} tests failed", results.len());
    }

    ctx.style.success(
        "Tests",
        &format!("{passed} passed in {:.2}s", elapsed.as_secs_f64()),
    );

    Ok(())
}

fn run_test_binaries(
    binaries: &[(String, PathBuf)],
    framework_filter: Option<&str>,
    targets: &[tester::TestTarget],
    ctx: &Context,
) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    for (name, bin_path) in binaries {
        if !bin_path.exists() {
            results.push(TestResult {
                name: name.clone(),
                passed: false,
                output: format!("test binary not found: {}", bin_path.display()),
            });
            continue;
        }

        let framework = targets
            .iter()
            .find(|t| t.name == *name)
            .map(|t| t.framework)
            .unwrap_or(TestFramework::Plain);

        let mut cmd = Command::new(bin_path);
        if let Some(filter) = framework_filter {
            match framework {
                TestFramework::Gtest => {
                    cmd.arg(format!("--gtest_filter=*{filter}*"));
                }
                TestFramework::Catch2 => {
                    cmd.arg(filter);
                }
                TestFramework::Doctest => {
                    cmd.arg(format!("--test-case=*{filter}*"));
                }
                TestFramework::Plain => {}
            }
        }

        ctx.style.run_icon("Running", name);

        let start = Instant::now();
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .into_diagnostic()?;
        let duration = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let mut combined = String::new();
        if !stdout.is_empty() {
            combined.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&stderr);
        }

        let passed = output.status.success();
        if passed {
            ctx.style.success(
                "Passed",
                &format!("{name} ({:.2}s)", duration.as_secs_f64()),
            );
        }

        results.push(TestResult {
            name: name.clone(),
            passed,
            output: combined,
        });
    }

    Ok(results)
}

fn framework_link_info(framework: TestFramework) -> (Vec<String>, Vec<PathBuf>, Vec<PathBuf>) {
    match framework {
        TestFramework::Gtest => (
            vec![
                "gtest".to_string(),
                "gtest_main".to_string(),
                "pthread".to_string(),
            ],
            Vec::new(),
            Vec::new(),
        ),
        TestFramework::Catch2 => (
            vec!["Catch2Main".to_string(), "Catch2".to_string()],
            Vec::new(),
            Vec::new(),
        ),
        TestFramework::Doctest => (Vec::new(), Vec::new(), Vec::new()),
        TestFramework::Plain => (Vec::new(), Vec::new(), Vec::new()),
    }
}

fn resolve_profile_name(opts: &TestOptions) -> String {
    if let Some(ref profile) = opts.profile {
        profile.clone()
    } else if opts.release {
        "release".to_string()
    } else {
        "debug".to_string()
    }
}

fn auto_detect_compiler() -> CompilerKind {
    compiler::detect_compiler()
        .map(|c| c.kind)
        .unwrap_or(CompilerKind::Clang)
}

fn load_cached_deps(profile_dir: &Path) -> Vec<FetchedDep> {
    let cache_path = profile_dir.join(DEP_CACHE_FILE);
    load_dep_cache(&cache_path).unwrap_or_default()
}

fn build_test_compile_flags(
    manifest: &Manifest,
    project_root: &Path,
    profile_name: &str,
    opts: &TestOptions,
    deps: &[FetchedDep],
) -> CompileFlags {
    use ordo_core::manifest::ResolvedFeatures;

    let profile = manifest.resolve_profile(profile_name).unwrap_or_else(|_| {
        if opts.release {
            ordo_core::manifest::Profile::release_defaults()
        } else {
            ordo_core::manifest::Profile::dev_defaults()
        }
    });

    let mut include_dirs = Vec::new();
    let include_path = project_root.join("include");
    if include_path.exists()
        && let Ok(canonical) = fs::canonicalize(&include_path)
    {
        include_dirs.push(canonical);
    }

    for dep in deps {
        include_dirs.extend(dep.include_dirs.iter().cloned());
    }

    let has_cpp = manifest.language.cpp.is_some() || manifest.language.c.is_none();
    let mut defines = ResolvedFeatures::resolve(
        manifest,
        &opts.features,
        opts.no_default_features,
        opts.all_features,
    )
    .map(|r| r.defines)
    .unwrap_or_default();

    for dep in deps {
        defines.extend(dep.defines.iter().cloned());
    }

    CompileFlags {
        cpp_standard: if has_cpp {
            manifest.language.cpp.or(Some(CppStandard::Cpp20))
        } else {
            None
        },
        c_standard: manifest.language.c,
        opt_level: profile.opt_level,
        debug: profile.debug,
        assertions: profile.assertions,
        sanitize: profile.sanitize.clone(),
        pic: profile.pic,
        rtti: profile.rtti,
        exceptions: profile.exceptions,
        warnings: profile.warnings,
        coverage: profile.coverage,
        split_debug: profile.split_debug,
        defines,
        include_dirs,
    }
}

fn build_test_link_flags(
    manifest: &Manifest,
    profile_name: &str,
    deps: &[FetchedDep],
) -> LinkFlags {
    let profile = manifest
        .resolve_profile(profile_name)
        .unwrap_or_else(|_| ordo_core::manifest::Profile::dev_defaults());

    let linker = profile.linker.as_deref().and_then(|l| match l {
        "lld" => Some(ordo_core::manifest::LinkerKind::Lld),
        "mold" => Some(ordo_core::manifest::LinkerKind::Mold),
        "gold" => Some(ordo_core::manifest::LinkerKind::Gold),
        _ => None,
    });

    let mut lib_dirs = Vec::new();
    let mut libs = Vec::new();
    let mut frameworks = Vec::new();

    for dep in deps {
        lib_dirs.extend(dep.lib_dirs.iter().cloned());
        libs.extend(dep.libs.iter().cloned());
        frameworks.extend(dep.frameworks.iter().cloned());
    }

    frameworks.sort();
    frameworks.dedup();

    LinkFlags {
        lib_dirs,
        libs,
        frameworks,
        linker,
        lto: profile.lto,
        strip: profile.strip,
        static_runtime: profile.static_runtime,
        sanitize: profile.sanitize,
        coverage: profile.coverage,
    }
}

fn resolve_ninja_bin(version_req: Option<&str>) -> PathBuf {
    ordo_arsenal::resolve_tool_path(ordo_arsenal::Tool::Ninja, version_req)
        .unwrap_or_else(|| PathBuf::from("ninja"))
}

fn invoke_ninja(
    build_dir: &Path,
    jobs: Option<u32>,
    manifest_ninja_version: Option<&str>,
    ctx: &Context,
) -> Result<()> {
    let ninja_bin = resolve_ninja_bin(manifest_ninja_version);
    let mut cmd = Command::new(&ninja_bin);
    cmd.arg("-C").arg(build_dir);

    if let Some(j) = jobs {
        cmd.arg(format!("-j{j}"));
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .into_diagnostic()
        .map_err(|_| miette::miette!("failed to execute ninja — is it installed?"))?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stderr_handle = std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        reader.lines().map_while(Result::ok).collect::<Vec<_>>()
    });

    let spinner = ctx.style.create_spinner("Building tests...");
    let reader = BufReader::new(stdout);
    let mut output_lines = Vec::new();
    for line in reader.lines() {
        let line = line.into_diagnostic()?;
        if !line.trim().is_empty() {
            output_lines.push(line);
        }
    }

    spinner.finish_and_clear();
    let status = child.wait().into_diagnostic()?;

    let stderr_lines = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        for line in &output_lines {
            eprintln!("  {line}");
        }
        for line in &stderr_lines {
            eprintln!("  {line}");
        }
        ctx.style.error("Build failed", "test compilation failed");
        bail!(
            "test build failed (ninja exit code: {})",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
