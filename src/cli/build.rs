use crate::backend::compiler::{self, CompileFlags, LinkFlags};
use crate::backend::ninja::NinjaGenerator;
use crate::core::manifest::{CompilerKind, CppStandard, Manifest, PackageType};
use crate::util::style;
use miette::{bail, IntoDiagnostic, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[allow(dead_code)]
pub struct BuildOptions {
    pub release: bool,
    pub profile: Option<String>,
    pub jobs: Option<u32>,
    pub target: Option<String>,
    pub no_cache: bool,
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

    let compile_flags = build_compile_flags(&manifest, opts);
    let link_flags = LinkFlags::default();

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

    style::status(
        "Compiling",
        &format!("{} v{} ({})", manifest.package.name, manifest.package.version, profile_name),
    );

    let start = Instant::now();
    invoke_ninja(&build_dir, opts.jobs)?;
    let elapsed = start.elapsed();

    let output_path = resolve_output_path(&output_dir, &manifest.package.name, manifest.package.package_type);

    style::status(
        "Finished",
        &format!("`{profile_name}` profile [{}] target(s) in {:.2}s",
            if profile_name == "release" { "optimized" } else { "unoptimized + debuginfo" },
            elapsed.as_secs_f64(),
        ),
    );

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

fn build_compile_flags(manifest: &Manifest, opts: &BuildOptions) -> CompileFlags {
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

    CompileFlags {
        cpp_standard: manifest.language.cpp.or(Some(CppStandard::Cpp20)),
        c_standard: manifest.language.c,
        opt_level,
        debug,
        defines: Vec::new(),
        include_dirs,
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

fn invoke_ninja(build_dir: &Path, jobs: Option<u32>) -> Result<()> {
    let mut cmd = Command::new("ninja");
    cmd.arg("-C").arg(build_dir);

    if let Some(j) = jobs {
        cmd.arg(format!("-j{j}"));
    }

    tracing::info!("running: ninja -C {}", build_dir.display());

    let status = cmd
        .status()
        .into_diagnostic()
        .map_err(|_| miette::miette!("failed to execute ninja — is it installed?"))?;

    if !status.success() {
        style::status_error("Error", "build failed");
        bail!("build failed (ninja exit code: {})", status.code().unwrap_or(-1));
    }

    Ok(())
}

fn resolve_output_path(output_dir: &Path, name: &str, package_type: PackageType) -> PathBuf {
    match package_type {
        PackageType::Executable => output_dir.join(name),
        PackageType::StaticLibrary => output_dir.join(format!("lib{name}.a")),
        PackageType::SharedLibrary => output_dir.join(format!("lib{name}.so")),
    }
}
