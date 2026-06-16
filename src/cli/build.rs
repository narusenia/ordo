use crate::backend::compiler::{self, CompileFlags, LinkFlags};
use crate::backend::ninja::NinjaGenerator;
use crate::backend::provider::pkgconfig::PkgConfigProvider;
use crate::backend::provider::system::SystemProvider;
use crate::backend::provider::vcpkg::VcpkgProvider;
use crate::backend::provider::{FetchedDep, Provider};
use crate::core::manifest::{CompilerKind, CppStandard, DependencySource, Manifest, PackageType, ProviderKind};
use crate::util::style;
use miette::{bail, IntoDiagnostic, Result};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

#[allow(dead_code)]
pub struct BuildOptions {
    pub release: bool,
    pub profile: Option<String>,
    pub jobs: Option<u32>,
    pub target: Option<String>,
    pub no_cache: bool,
    pub verbose: u8,
}

pub struct BuildResult {
    pub output_path: PathBuf,
    pub package_type: PackageType,
}

pub fn run(opts: &BuildOptions) -> Result<BuildResult> {
    let manifest_path = Path::new("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in current directory");
    }

    let manifest = Manifest::load(manifest_path)?;
    let profile_name = resolve_profile_name(opts);
    let build_dir = PathBuf::from(format!("target/{profile_name}/build"));
    let output_dir = PathBuf::from(format!("target/{profile_name}"));

    fs::create_dir_all(&build_dir).into_diagnostic()?;
    fs::create_dir_all(&output_dir).into_diagnostic()?;

    let compiler_kind = manifest
        .toolchain
        .compiler
        .unwrap_or_else(auto_detect_compiler);
    let compiler = compiler::create_compiler(compiler_kind);

    let sources = discover_sources(Path::new("src"))?;
    if sources.is_empty() {
        bail!("no source files found in src/");
    }

    let fetched_deps = fetch_dependencies(&manifest)?;
    let compile_flags = build_compile_flags(&manifest, opts, &fetched_deps);
    let link_flags = build_link_flags(&fetched_deps);

    let project_root = std::env::current_dir().into_diagnostic()?;

    let ninja_gen = NinjaGenerator::new(
        compiler.as_ref(),
        sources,
        project_root.join(&build_dir),
        project_root.clone(),
        manifest.package.name.clone(),
        manifest.package.package_type,
        compile_flags,
        link_flags,
    );

    let output = ninja_gen.generate();

    fs::write(build_dir.join("build.ninja"), &output.build_ninja).into_diagnostic()?;
    fs::write("compile_commands.json", &output.compile_commands).into_diagnostic()?;

    let start = Instant::now();
    invoke_ninja(&build_dir, opts.jobs, opts.verbose)?;
    let elapsed = start.elapsed();

    let output_path = resolve_output_path(
        &output_dir,
        &manifest.package.name,
        manifest.package.package_type,
    );

    let profile_desc = if profile_name == "release" {
        "optimized"
    } else {
        "unoptimized + debuginfo"
    };
    style::success(
        "Finished",
        &format!(
            "`{profile_name}` profile [{profile_desc}] in {:.2}s",
            elapsed.as_secs_f64()
        ),
    );
    style::meta(&format!("→ {}", output_path.display()));

    Ok(BuildResult {
        output_path,
        package_type: manifest.package.package_type,
    })
}

fn resolve_profile_name(opts: &BuildOptions) -> String {
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

fn fetch_dependencies(manifest: &Manifest) -> Result<Vec<FetchedDep>> {
    let mut fetched = Vec::new();

    for (name, spec) in &manifest.dependencies {
        let dep = match spec.source_kind() {
            DependencySource::Provider(ProviderKind::PkgConfig) => {
                let provider = PkgConfigProvider;
                let resolved = provider.resolve(name, spec.version.as_deref())?;
                style::success("Resolved", &format!("{name} v{} (pkg-config)", resolved.version));
                provider.fetch(&resolved)?
            }
            DependencySource::Provider(ProviderKind::System) => {
                let provider = SystemProvider;
                let resolved = provider.resolve(name, spec.version.as_deref())?;
                style::success("Resolved", &format!("{name} (system)"));
                provider.fetch(&resolved)?
            }
            DependencySource::Provider(ProviderKind::Vcpkg) => {
                let provider = VcpkgProvider::new();
                let resolved = provider.resolve(name, spec.version.as_deref())?;
                style::success("Resolved", &format!("{name} v{} (vcpkg)", resolved.version));
                provider.fetch(&resolved)?
            }
            _ => continue,
        };
        fetched.push(dep);
    }

    Ok(fetched)
}

fn build_compile_flags(manifest: &Manifest, opts: &BuildOptions, deps: &[FetchedDep]) -> CompileFlags {
    let (opt_level, debug) = if opts.release {
        (3, false)
    } else {
        (0, true)
    };

    let mut include_dirs = Vec::new();
    let include_path = PathBuf::from("include");
    if include_path.exists() {
        include_dirs.push(include_path);
    }

    for dep in deps {
        include_dirs.extend(dep.include_dirs.iter().cloned());
    }

    CompileFlags {
        cpp_standard: manifest.language.cpp.or(Some(CppStandard::Cpp20)),
        c_standard: manifest.language.c,
        opt_level,
        debug,
        defines: Vec::new(),
        include_dirs,
    }
}

fn build_link_flags(deps: &[FetchedDep]) -> LinkFlags {
    let mut lib_dirs = Vec::new();
    let mut libs = Vec::new();

    for dep in deps {
        lib_dirs.extend(dep.lib_dirs.iter().cloned());
        libs.extend(dep.libs.iter().cloned());
    }

    LinkFlags {
        lib_dirs,
        libs,
        linker: None,
    }
}

fn discover_sources(src_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    if !src_dir.exists() {
        return Ok(sources);
    }
    collect_sources(src_dir, &mut sources)?;
    sources.sort();
    Ok(sources)
}

fn collect_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if path.is_dir() {
            collect_sources(&path, out)?;
        } else if is_source_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("cpp" | "cc" | "cxx" | "c")
    )
}

fn invoke_ninja(build_dir: &Path, jobs: Option<u32>, _verbose: u8) -> Result<()> {
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

    let spinner = style::create_spinner("");
    let mut had_progress = false;

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = line.into_diagnostic()?;

        if let Some(parsed) = parse_ninja_progress(&line) {
            had_progress = true;

            let desc = clean_path(parsed.description.as_deref().unwrap_or(""));
            let progress = format!("[{}/{}]", parsed.current, parsed.total);

            let (active_verb, _done_verb) = if parsed.action.contains("Linking") {
                ("Linking", "Linked")
            } else if parsed.action.contains("Archiving") {
                ("Archiving", "Archived")
            } else {
                ("Compiling", "Compiled")
            };

            if parsed.current > 1 {
                let prev_msg = spinner.message();
                spinner.finish_and_clear();
                let prev_display = prev_msg
                    .replace("Compiling", "Compiled")
                    .replace("Linking", "Linked")
                    .replace("Archiving", "Archived");
                style::success("", &prev_display);
            }

            spinner.reset();
            spinner.enable_steady_tick(std::time::Duration::from_millis(80));
            spinner.set_message(format!("{active_verb} {desc} {progress}"));
        }
    }

    if had_progress {
        let final_msg = spinner.message();
        spinner.finish_and_clear();
        let done_msg = final_msg
            .replace("Compiling", "Compiled")
            .replace("Linking", "Linked")
            .replace("Archiving", "Archived");
        style::success("", &done_msg);
    } else {
        spinner.finish_and_clear();
    }

    let status = child.wait().into_diagnostic()?;

    let stderr_lines = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        for line in &stderr_lines {
            eprintln!("  {line}");
        }
        style::error("Build failed", "");
        bail!(
            "build failed (ninja exit code: {})",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

#[allow(dead_code)]
struct NinjaProgress {
    current: u32,
    total: u32,
    action: String,
    description: Option<String>,
    command: Option<String>,
}

fn parse_ninja_progress(line: &str) -> Option<NinjaProgress> {
    // Parse "[N/M] Action description" format
    let line = line.trim();
    if !line.starts_with('[') {
        return None;
    }

    let bracket_end = line.find(']')?;
    let counts = &line[1..bracket_end];
    let (current, total) = counts.split_once('/')?;
    let current: u32 = current.trim().parse().ok()?;
    let total: u32 = total.trim().parse().ok()?;

    let rest = line[bracket_end + 1..].trim().to_string();

    let (action, description) = if let Some(idx) = rest.find(' ') {
        let (a, d) = rest.split_at(idx);
        (a.to_string(), Some(d.trim().to_string()))
    } else {
        (rest, None)
    };

    Some(NinjaProgress {
        current,
        total,
        action,
        description,
        command: None,
    })
}

fn clean_path(path: &str) -> String {
    let path = path.trim();
    // Strip leading ../ sequences to show project-relative path
    let mut result = path;
    while let Some(rest) = result.strip_prefix("../") {
        result = rest;
    }
    // Also strip leading ./
    if let Some(rest) = result.strip_prefix("./") {
        result = rest;
    }
    result.to_string()
}

fn resolve_output_path(output_dir: &Path, name: &str, package_type: PackageType) -> PathBuf {
    match package_type {
        PackageType::Executable => output_dir.join(name),
        PackageType::StaticLibrary => output_dir.join(format!("lib{name}.a")),
        PackageType::SharedLibrary => output_dir.join(format!("lib{name}.so")),
    }
}
