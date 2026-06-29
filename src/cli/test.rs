use super::context::Context;
use crate::backend::compiler::{self, CompileFlags, LinkFlags};
use crate::backend::ninja::{TestBinarySpec, TestNinjaGenerator};
use crate::core::manifest::{CompilerKind, CppStandard, Manifest, PackageType, TestFramework};
use crate::core::tester;
use crate::core::workspace::Workspace;
use miette::{IntoDiagnostic, Result, bail};
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

    let compiler_kind = manifest
        .toolchain
        .compiler
        .unwrap_or_else(auto_detect_compiler);
    let compiler = compiler::create_compiler(compiler_kind);

    let compile_flags = build_test_compile_flags(&manifest, &profile_name, opts);
    let link_flags = build_test_link_flags(&manifest, &profile_name);

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
        fs::canonicalize(project_root).into_diagnostic()?,
        compile_flags,
        link_flags,
        test_lib
            .as_ref()
            .map(|l| l.sources.clone())
            .unwrap_or_default(),
        test_lib.as_ref().map(|l| l.lib_name.clone()),
        project_lib_path,
        test_specs,
    );

    let output = ninja_gen.generate();
    fs::write(test_build_dir.join("build.ninja"), &output.build_ninja).into_diagnostic()?;

    ctx.style.header(&format!("Testing {}", pkg.name));

    let start = Instant::now();
    invoke_ninja(&test_build_dir, opts.jobs, ctx)?;

    let results = run_test_binaries(
        &output.test_binaries,
        opts.filter.as_deref(),
        &filtered,
        ctx,
    )?;
    let elapsed = start.elapsed();

    let passed = results.iter().filter(|r| r.passed).count();
    let failed = results.iter().filter(|r| !r.passed).count();

    for r in &results {
        if !r.passed {
            ctx.style.error("FAIL", &r.name);
            if !r.output.is_empty() {
                for line in r.output.lines() {
                    eprintln!("  {line}");
                }
            }
        }
    }

    if failed > 0 {
        ctx.style.error(
            "Result",
            &format!(
                "{passed} passed, {failed} failed in {:.2}s",
                elapsed.as_secs_f64()
            ),
        );
        bail!("test failed: {failed} of {} tests failed", results.len());
    }

    ctx.style.success(
        "Result",
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
            ctx.style
                .success("PASS", &format!("{name} ({:.2}s)", duration.as_secs_f64()));
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

fn build_test_compile_flags(
    manifest: &Manifest,
    profile_name: &str,
    opts: &TestOptions,
) -> CompileFlags {
    use crate::core::manifest::ResolvedFeatures;

    let profile = manifest.resolve_profile(profile_name).unwrap_or_else(|_| {
        if opts.release {
            crate::core::manifest::Profile::release_defaults()
        } else {
            crate::core::manifest::Profile::dev_defaults()
        }
    });

    let mut include_dirs = Vec::new();
    let include_path = std::env::current_dir()
        .ok()
        .map(|cwd| cwd.join("include"))
        .unwrap_or_default();
    if include_path.exists()
        && let Ok(canonical) = fs::canonicalize(&include_path)
    {
        include_dirs.push(canonical);
    }

    let has_cpp = manifest.language.cpp.is_some() || manifest.language.c.is_none();
    let feature_defines = ResolvedFeatures::resolve(
        manifest,
        &opts.features,
        opts.no_default_features,
        opts.all_features,
    )
    .map(|r| r.defines)
    .unwrap_or_default();

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
        defines: feature_defines,
        include_dirs,
    }
}

fn build_test_link_flags(manifest: &Manifest, profile_name: &str) -> LinkFlags {
    let profile = manifest
        .resolve_profile(profile_name)
        .unwrap_or_else(|_| crate::core::manifest::Profile::dev_defaults());

    let linker = profile.linker.as_deref().and_then(|l| match l {
        "lld" => Some(crate::core::manifest::LinkerKind::Lld),
        "mold" => Some(crate::core::manifest::LinkerKind::Mold),
        "gold" => Some(crate::core::manifest::LinkerKind::Gold),
        _ => None,
    });

    LinkFlags {
        lib_dirs: Vec::new(),
        libs: Vec::new(),
        frameworks: Vec::new(),
        linker,
        lto: profile.lto,
        strip: profile.strip,
        static_runtime: profile.static_runtime,
        sanitize: profile.sanitize,
        coverage: profile.coverage,
    }
}

fn invoke_ninja(build_dir: &Path, jobs: Option<u32>, ctx: &Context) -> Result<()> {
    let mut cmd = Command::new("ninja");
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
