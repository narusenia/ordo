use super::context::Context;
use crate::backend::compiler::{self, CompileFlags, LinkFlags};
use crate::backend::ninja::NinjaGenerator;
use crate::backend::provider::conan::ConanProvider;
use crate::backend::provider::git::{GitDepSpec, GitProvider};
use crate::backend::provider::pkgconfig::PkgConfigProvider;
use crate::backend::provider::system::SystemProvider;
use crate::backend::provider::vcpkg::{VcpkgPackageSpec, VcpkgProvider};
use crate::backend::provider::{FetchedDep, Provider, ResolvedDep};
use crate::core::lockfile::LockFile;
use crate::core::manifest::{
    CompilerKind, CppStandard, DependencySource, Manifest, PackageType, ProviderKind,
};
use crate::core::resolver::resolve_dependencies_with_features;
use crate::core::workspace::Workspace;
use miette::{IntoDiagnostic, Result, bail};
use std::collections::HashSet;
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
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub all_features: bool,
    pub locked: bool,
    pub frozen: bool,
    pub verbose: u8,
    pub package: Option<String>,
}

pub struct BuildResult {
    pub output_path: PathBuf,
    pub package_type: PackageType,
}

pub struct BuildContext {
    pub project_root: PathBuf,
    pub workspace_root: Option<PathBuf>,
    pub target_dir: PathBuf,
    pub profile_name: String,
    pub release: bool,
    pub jobs: Option<u32>,
    pub verbose: u8,
    pub no_cache: bool,
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub all_features: bool,
    pub locked: bool,
    pub frozen: bool,
    pub building: HashSet<PathBuf>,
    pub built_deps: std::collections::HashMap<PathBuf, FetchedDep>,
}

pub fn run(opts: &BuildOptions, ctx: &Context) -> Result<BuildResult> {
    let project_root = std::env::current_dir().into_diagnostic()?;
    let canonical = fs::canonicalize(&project_root).into_diagnostic()?;
    let profile_name = resolve_profile_name(opts);

    let manifest_path = project_root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", project_root.display());
    }

    let manifest = Manifest::load(&manifest_path)?;

    if manifest.is_workspace() {
        return run_workspace_build(opts, &project_root, &profile_name, ctx);
    }

    let target_dir = project_root.join("target");
    let mut bctx = BuildContext {
        project_root,
        workspace_root: None,
        target_dir,
        profile_name,
        release: opts.release,
        jobs: opts.jobs,
        verbose: opts.verbose,
        no_cache: opts.no_cache,
        features: opts.features.clone(),
        no_default_features: opts.no_default_features,
        all_features: opts.all_features,
        locked: opts.locked,
        frozen: opts.frozen,
        building: HashSet::from([canonical]),
        built_deps: std::collections::HashMap::new(),
    };

    build_project(&mut bctx, ctx)
}

fn run_workspace_build(
    opts: &BuildOptions,
    root_dir: &Path,
    profile_name: &str,
    ctx: &Context,
) -> Result<BuildResult> {
    let ws = Workspace::load(root_dir)?;
    let dag = ws.build_dag()?;

    let members_to_build: Vec<String> = if let Some(ref target) = opts.package {
        if ws.find_member(target).is_none() {
            let available = ws.member_names().join(", ");
            bail!(
                "package '{}' not found in workspace; available members: {}",
                target,
                available
            );
        }
        dag.subset_order(target)
    } else {
        if ws.root_manifest.is_virtual_workspace() {
            dag.order.clone()
        } else {
            let mut order = dag.order.clone();
            order.push("__root__".to_string());
            order
        }
    };

    let ws_target_dir = root_dir.join("target");
    fs::create_dir_all(&ws_target_dir).into_diagnostic()?;

    let mut last_result = None;
    let mut built_deps = std::collections::HashMap::new();

    let ws_root = root_dir.to_path_buf();

    for member_name in &members_to_build {
        if member_name == "__root__" {
            let canonical = fs::canonicalize(root_dir).into_diagnostic()?;
            let mut bctx = BuildContext {
                project_root: root_dir.to_path_buf(),
                workspace_root: Some(ws_root.clone()),
                target_dir: ws_target_dir.clone(),
                profile_name: profile_name.to_string(),
                release: opts.release,
                jobs: opts.jobs,
                verbose: opts.verbose,
                no_cache: opts.no_cache,
                features: opts.features.clone(),
                no_default_features: opts.no_default_features,
                all_features: opts.all_features,
                locked: opts.locked,
                frozen: opts.frozen,
                building: HashSet::from([canonical.clone()]),
                built_deps: std::mem::take(&mut built_deps),
            };
            last_result = Some(build_project(&mut bctx, ctx)?);
            built_deps = std::mem::take(&mut bctx.built_deps);
            if let Some(dep) = collect_member_as_fetched_dep(root_dir, &ws_target_dir, profile_name)
            {
                built_deps.insert(canonical, dep);
            }
            continue;
        }

        let member = ws.find_member(member_name).unwrap();
        let member_dir = &member.dir;
        let canonical = fs::canonicalize(member_dir).into_diagnostic()?;

        ctx.style.header(member_name);

        let mut bctx = BuildContext {
            project_root: member_dir.clone(),
            workspace_root: Some(ws_root.clone()),
            target_dir: ws_target_dir.clone(),
            profile_name: profile_name.to_string(),
            release: opts.release,
            jobs: opts.jobs,
            verbose: opts.verbose,
            no_cache: opts.no_cache,
            features: opts.features.clone(),
            no_default_features: opts.no_default_features,
            all_features: opts.all_features,
            locked: opts.locked,
            frozen: opts.frozen,
            building: HashSet::from([canonical.clone()]),
            built_deps: std::mem::take(&mut built_deps),
        };

        last_result = Some(build_project(&mut bctx, ctx)?);
        built_deps = std::mem::take(&mut bctx.built_deps);

        if let Some(dep) = collect_member_as_fetched_dep(member_dir, &ws_target_dir, profile_name) {
            built_deps.insert(canonical, dep);
        }
    }

    last_result.ok_or_else(|| miette::miette!("no members to build"))
}

fn build_project(ctx: &mut BuildContext, ui: &Context) -> Result<BuildResult> {
    let manifest_path = ctx.project_root.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in {}", ctx.project_root.display());
    }

    let mut manifest = Manifest::load(&manifest_path)?;

    if manifest.package.is_none() {
        bail!(
            "cannot build '{}': no [package] section (workspace-only root?)",
            ctx.project_root.display()
        );
    }

    if let Some(ref ws_root) = ctx.workspace_root {
        resolve_workspace_deps_for_member(&mut manifest, ws_root)?;
    }

    let project_root = fs::canonicalize(&ctx.project_root).into_diagnostic()?;
    let pkg_name = manifest.package().name.clone();

    let (build_dir, output_dir) = if ctx.workspace_root.is_some() {
        let base = ctx.target_dir.join(&ctx.profile_name).join(&pkg_name);
        (base.join("build"), base)
    } else {
        let base = ctx.target_dir.join(&ctx.profile_name);
        (base.join("build"), base)
    };

    fs::create_dir_all(&build_dir).into_diagnostic()?;
    fs::create_dir_all(&output_dir).into_diagnostic()?;

    let compiler_kind = manifest
        .toolchain
        .compiler
        .unwrap_or_else(auto_detect_compiler);
    let compiler = compiler::create_compiler(compiler_kind);

    let src_dir = ctx.project_root.join("src");
    let sources = discover_sources(&src_dir)?;
    if sources.is_empty() {
        bail!("no source files found in {}", src_dir.display());
    }

    let fetched_deps = resolve_and_fetch(&manifest, ctx, ui)?;
    let compile_flags = build_compile_flags(&manifest, ctx, &fetched_deps);
    let link_flags = build_link_flags(&manifest, ctx, &fetched_deps);

    let ninja_gen = NinjaGenerator::new(
        compiler.as_ref(),
        sources,
        build_dir.clone(),
        project_root.clone(),
        manifest.package().name.clone(),
        manifest.package().package_type,
        compile_flags,
        link_flags,
    );

    let output = ninja_gen.generate();

    fs::write(build_dir.join("build.ninja"), &output.build_ninja).into_diagnostic()?;
    fs::write(
        project_root.join("compile_commands.json"),
        &output.compile_commands,
    )
    .into_diagnostic()?;

    let start = Instant::now();
    invoke_ninja(&build_dir, ctx.jobs, ctx.verbose, ui)?;
    let elapsed = start.elapsed();

    let output_path = resolve_output_path(
        &output_dir,
        &manifest.package().name,
        manifest.package().package_type,
    );

    let profile = manifest
        .resolve_profile(&ctx.profile_name)
        .unwrap_or_else(|_| {
            if ctx.release {
                crate::core::manifest::Profile::release_defaults()
            } else {
                crate::core::manifest::Profile::dev_defaults()
            }
        });
    let profile_desc = profile.display_desc();
    ui.style.success(
        "Finished",
        &format!(
            "`{}` profile [{profile_desc}] in {:.2}s",
            ctx.profile_name,
            elapsed.as_secs_f64()
        ),
    );
    ui.style.meta(&format!("→ {}", output_path.display()));

    Ok(BuildResult {
        output_path,
        package_type: manifest.package().package_type,
    })
}

fn resolve_feature_defines(manifest: &Manifest, ctx: &BuildContext) -> Vec<String> {
    use crate::core::manifest::ResolvedFeatures;
    ResolvedFeatures::resolve(
        manifest,
        &ctx.features,
        ctx.no_default_features,
        ctx.all_features,
    )
    .map(|r| r.defines)
    .unwrap_or_default()
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

const DEP_CACHE_FILE: &str = ".dep-cache.json";

fn resolve_and_fetch(
    manifest: &Manifest,
    ctx: &mut BuildContext,
    ui: &Context,
) -> Result<Vec<FetchedDep>> {
    if manifest.dependencies.is_empty() {
        return Ok(Vec::new());
    }

    let lock_root = ctx.workspace_root.as_deref().unwrap_or(&ctx.project_root);
    let lock_path = lock_root.join("Ordo.lock");

    let pkg_name = manifest
        .package
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("root");
    let cache_dir = if ctx.workspace_root.is_some() {
        ctx.target_dir.join(&ctx.profile_name).join(pkg_name)
    } else {
        ctx.target_dir.clone()
    };
    let cache_path = cache_dir.join(DEP_CACHE_FILE);

    let existing_lock = LockFile::load(&lock_path).ok();

    let activated_deps = {
        use crate::core::manifest::ResolvedFeatures;
        ResolvedFeatures::resolve(
            manifest,
            &ctx.features,
            ctx.no_default_features,
            ctx.all_features,
        )
        .ok()
        .map(|r| r.activated_deps)
    };
    let resolved = resolve_dependencies_with_features(
        manifest,
        existing_lock.as_ref(),
        activated_deps.as_ref(),
    )?;

    let is_fresh = existing_lock
        .as_ref()
        .is_some_and(|lock| lock.is_fresh(&resolved));

    if ctx.locked && !is_fresh {
        bail!(
            "Ordo.lock is out of date; run `ordo update` to regenerate it, \
             or remove --locked to allow automatic updates"
        );
    }

    if is_fresh && !ctx.no_cache {
        if let Ok(cached) = load_dep_cache(&cache_path) {
            ui.style.success(
                "Resolved",
                &format!("{} dependencies (cached)", cached.len()),
            );
            return Ok(cached);
        }
        if ctx.frozen {
            bail!(
                "dependency cache missing and --frozen disallows network access; \
                 run `ordo build` without --frozen first"
            );
        }
    } else if ctx.frozen {
        bail!(
            "Ordo.lock is out of date and --frozen disallows network access; \
             run `ordo update` and `ordo build` without --frozen first"
        );
    }

    let fetch_result = fetch_dependencies(manifest, ctx, ui)?;

    let mut lock = LockFile::load(&lock_path).unwrap_or(LockFile {
        version: 1,
        packages: Vec::new(),
    });
    lock.merge(&resolved);

    for pkg in &mut lock.packages {
        if let Some(ver) = fetch_result.resolved_versions.get(&pkg.name) {
            pkg.version = ver.clone();
        }
        if let Some(cksum) = fetch_result.checksums.get(&pkg.name) {
            pkg.checksum = Some(cksum.clone());
        }
    }

    lock.save(&lock_path)?;
    save_dep_cache(&cache_path, &fetch_result.deps)?;

    Ok(fetch_result.deps)
}

fn load_dep_cache(path: &Path) -> Result<Vec<FetchedDep>> {
    let content = fs::read_to_string(path).into_diagnostic()?;
    serde_json::from_str(&content).into_diagnostic()
}

fn resolve_workspace_deps_for_member(manifest: &mut Manifest, ws_root: &Path) -> Result<()> {
    let ws_manifest_path = ws_root.join("Ordo.toml");
    if !ws_manifest_path.exists() {
        return Ok(());
    }
    let ws_manifest = Manifest::load(&ws_manifest_path)?;
    let ws_config = match ws_manifest.workspace {
        Some(ref ws) => ws,
        None => return Ok(()),
    };

    for (name, spec) in &mut manifest.dependencies {
        if spec.workspace {
            let ws_spec = ws_config.dependencies.get(name).ok_or_else(|| {
                miette::miette!(
                    "dependency '{}' uses `workspace = true` but is not in [workspace.dependencies]",
                    name
                )
            })?;
            *spec = ws_spec.clone();
        }
    }

    for (name, spec) in &mut manifest.dev_dependencies {
        if spec.workspace {
            let ws_spec = ws_config.dependencies.get(name).ok_or_else(|| {
                miette::miette!(
                    "dev-dependency '{}' uses `workspace = true` but is not in [workspace.dependencies]",
                    name
                )
            })?;
            *spec = ws_spec.clone();
        }
    }

    Ok(())
}

fn save_dep_cache(path: &Path, deps: &[FetchedDep]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).into_diagnostic()?;
    }
    let content = serde_json::to_string_pretty(deps).into_diagnostic()?;
    fs::write(path, content).into_diagnostic()
}

struct FetchResult {
    deps: Vec<FetchedDep>,
    resolved_versions: std::collections::HashMap<String, String>,
    checksums: std::collections::HashMap<String, String>,
}

fn fetch_dependencies(
    manifest: &Manifest,
    ctx: &mut BuildContext,
    ui: &Context,
) -> Result<FetchResult> {
    let mut fetched = Vec::new();
    let mut resolved_versions = std::collections::HashMap::new();
    let mut checksums = std::collections::HashMap::new();

    // Pass 1: batch-install all vcpkg deps in one manifest
    let vcpkg_deps: Vec<VcpkgPackageSpec<'_>> = manifest
        .dependencies
        .iter()
        .filter(|(_, spec)| spec.source_kind() == DependencySource::Provider(ProviderKind::Vcpkg))
        .map(|(name, spec)| VcpkgPackageSpec {
            name: spec.package_name(name),
            version: spec.version.as_deref(),
        })
        .collect();

    if !vcpkg_deps.is_empty() {
        let sw = ui.style.create_spinner_with_detail(&format!(
            "Installing {} vcpkg package(s)…",
            vcpkg_deps.len()
        ));
        let vcpkg = VcpkgProvider::new();
        let on_progress = |msg: &str| {
            sw.set_detail(msg);
        };
        match vcpkg.install_packages(&vcpkg_deps, &on_progress) {
            Ok(()) => {
                let names: Vec<&str> = vcpkg_deps.iter().map(|p| p.name).collect();
                sw.finish_success("Installed", &format!("{} (vcpkg)", names.join(", ")));
            }
            Err(e) => {
                sw.finish_error("Failed", "vcpkg install");
                return Err(e);
            }
        }
    }

    // Pass 2: resolve + fetch each dependency
    for (name, spec) in &manifest.dependencies {
        let pkg_name = spec.package_name(name);
        let mut dep = match spec.source_kind() {
            DependencySource::Provider(ProviderKind::PkgConfig) => {
                let spinner = ui
                    .style
                    .create_spinner(&format!("Resolving {name} (pkg-config)…"));
                let provider = PkgConfigProvider;
                match provider.resolve(pkg_name, spec.version.as_deref()) {
                    Ok(resolved) => {
                        ui.style.finish_spinner_success(
                            &spinner,
                            "Resolved",
                            &format!("{name} v{} (pkg-config)", resolved.version),
                        );
                        resolved_versions.insert(name.clone(), resolved.version.clone());
                        provider.fetch(&resolved)?
                    }
                    Err(e) => {
                        ui.style.finish_spinner_error(
                            &spinner,
                            "Failed",
                            &format!("{name} (pkg-config)"),
                        );
                        return Err(e);
                    }
                }
            }
            DependencySource::Provider(ProviderKind::System) => {
                let spinner = ui
                    .style
                    .create_spinner(&format!("Resolving {name} (system)…"));
                let provider = SystemProvider;
                match provider.resolve(pkg_name, spec.version.as_deref()) {
                    Ok(resolved) => {
                        ui.style.finish_spinner_success(
                            &spinner,
                            "Resolved",
                            &format!("{name} (system)"),
                        );
                        resolved_versions.insert(name.clone(), resolved.version.clone());
                        provider.fetch(&resolved)?
                    }
                    Err(e) => {
                        ui.style.finish_spinner_error(
                            &spinner,
                            "Failed",
                            &format!("{name} (system)"),
                        );
                        return Err(e);
                    }
                }
            }
            DependencySource::Provider(ProviderKind::Vcpkg) => {
                let spinner = ui
                    .style
                    .create_spinner(&format!("Resolving {name} (vcpkg)…"));
                let provider = VcpkgProvider::new();
                let root = provider.vcpkg_root()?;
                let triplet = VcpkgProvider::host_triplet();
                let version = provider.query_version(&root, pkg_name, triplet);
                let resolved = ResolvedDep {
                    name: pkg_name.to_string(),
                    version: version.clone(),
                    source: "vcpkg".to_string(),
                    checksum: None,
                };
                match provider.fetch(&resolved) {
                    Ok(dep) => {
                        ui.style.finish_spinner_success(
                            &spinner,
                            "Resolved",
                            &format!("{name} v{version} (vcpkg)"),
                        );
                        resolved_versions.insert(name.clone(), version);
                        dep
                    }
                    Err(e) => {
                        ui.style.finish_spinner_error(
                            &spinner,
                            "Failed",
                            &format!("{name} (vcpkg)"),
                        );
                        return Err(e);
                    }
                }
            }
            DependencySource::Provider(ProviderKind::Conan) => {
                let sw = ui
                    .style
                    .create_spinner_with_detail(&format!("Resolving {name} (conan)…"));
                let provider = ConanProvider::new();
                let on_progress = |msg: &str| {
                    sw.set_detail(msg);
                };
                match provider.resolve_with_progress(
                    pkg_name,
                    spec.version.as_deref(),
                    &on_progress,
                ) {
                    Ok(resolved) => {
                        sw.finish_success(
                            "Resolved",
                            &format!("{name} v{} (conan)", resolved.version),
                        );
                        resolved_versions.insert(name.clone(), resolved.version.clone());
                        provider.fetch(&resolved)?
                    }
                    Err(e) => {
                        sw.finish_error("Failed", &format!("{name} (conan)"));
                        return Err(e);
                    }
                }
            }
            DependencySource::Git => {
                let git_url = spec.git.as_deref().unwrap();
                let git_spec = GitDepSpec::from_dep(
                    git_url,
                    spec.tag.as_deref(),
                    spec.branch.as_deref(),
                    spec.rev.as_deref(),
                );
                let sw = ui
                    .style
                    .create_spinner_with_detail(&format!("Fetching {name} (git)…"));
                let provider = GitProvider::new();
                let on_progress = |msg: &str| {
                    sw.set_detail(msg);
                };
                match provider.resolve_git(name, &git_spec, &on_progress) {
                    Ok(resolved) => {
                        sw.finish_success("Fetched", &format!("{name} {} (git)", resolved.version));
                        resolved_versions.insert(name.clone(), resolved.version.clone());
                        if let Some(ref cksum) = resolved.checksum {
                            checksums.insert(name.clone(), cksum.clone());
                        }
                        let script = spec.with.as_ref().map(std::path::Path::new);
                        let script_root =
                            ctx.workspace_root.as_deref().unwrap_or(&ctx.project_root);
                        if script.is_some() {
                            let bw = ui
                                .style
                                .create_spinner_with_detail(&format!("Building {name} (lua)…"));
                            let on_lua_progress = |msg: &str| {
                                bw.set_detail(msg);
                            };
                            match provider.fetch_git_with_script(
                                name,
                                &git_spec,
                                script,
                                Some(script_root),
                                &on_lua_progress,
                            ) {
                                Ok(dep) => {
                                    bw.finish_success("Built", &format!("{name} (lua)"));
                                    dep
                                }
                                Err(e) => {
                                    bw.finish_error("Failed", &format!("{name} (lua)"));
                                    return Err(e);
                                }
                            }
                        } else {
                            provider.fetch_git_with_script(
                                name,
                                &git_spec,
                                None,
                                None,
                                &on_progress,
                            )?
                        }
                    }
                    Err(e) => {
                        sw.finish_error("Failed", &format!("{name} (git)"));
                        return Err(e);
                    }
                }
            }
            DependencySource::Path => {
                let rel_path = spec.path.as_ref().unwrap();
                let dep_dir = ctx.project_root.join(rel_path);
                if let Ok(dep_manifest) = Manifest::load(&dep_dir.join("Ordo.toml"))
                    && let Some(ref pkg) = dep_manifest.package
                {
                    resolved_versions.insert(name.clone(), pkg.version.clone());
                }
                fetch_path_dep(name, &dep_dir, ctx, ui)?
            }
            _ => continue,
        };
        if let Some(ref link_names) = spec.link_name {
            dep.libs = link_names.clone();
        }
        fetched.push(dep);
    }

    Ok(FetchResult {
        deps: fetched,
        resolved_versions,
        checksums,
    })
}

fn build_compile_flags(
    manifest: &Manifest,
    ctx: &BuildContext,
    deps: &[FetchedDep],
) -> CompileFlags {
    let profile = manifest
        .resolve_profile(&ctx.profile_name)
        .unwrap_or_else(|_| {
            if ctx.release {
                crate::core::manifest::Profile::release_defaults()
            } else {
                crate::core::manifest::Profile::dev_defaults()
            }
        });

    let mut include_dirs = Vec::new();
    let include_path = ctx.project_root.join("include");
    if include_path.exists() {
        if let Ok(canonical) = fs::canonicalize(&include_path) {
            include_dirs.push(canonical);
        } else {
            include_dirs.push(include_path);
        }
    }

    for dep in deps {
        include_dirs.extend(dep.include_dirs.iter().cloned());
    }

    let has_cpp = manifest.language.cpp.is_some() || manifest.language.c.is_none();
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
        defines: resolve_feature_defines(manifest, ctx),
        include_dirs,
    }
}

fn fetch_path_dep(
    name: &str,
    dep_dir: &Path,
    ctx: &mut BuildContext,
    ui: &Context,
) -> Result<FetchedDep> {
    if !dep_dir.exists() {
        bail!(
            "path dependency '{}' points to '{}' which does not exist",
            name,
            dep_dir.display()
        );
    }

    let dep_manifest_path = dep_dir.join("Ordo.toml");
    if !dep_manifest_path.exists() {
        bail!(
            "path dependency '{}' at '{}' has no Ordo.toml",
            name,
            dep_dir.display()
        );
    }

    let canonical_dep = fs::canonicalize(dep_dir).into_diagnostic()?;

    if let Some(cached) = ctx.built_deps.get(&canonical_dep) {
        return Ok(cached.clone());
    }

    if ctx.building.contains(&canonical_dep) {
        bail!(
            "circular path dependency detected: '{}' at '{}'",
            name,
            canonical_dep.display()
        );
    }

    let include_dir = dep_dir.join("include");
    let mut include_dirs = if include_dir.exists() {
        vec![fs::canonicalize(&include_dir).into_diagnostic()?]
    } else {
        vec![fs::canonicalize(dep_dir).into_diagnostic()?]
    };

    let src_dir = dep_dir.join("src");
    let has_sources = src_dir.exists() && !discover_sources(&src_dir)?.is_empty();

    if !has_sources {
        ui.style
            .success("Resolved", &format!("{name} (path, header-only)"));
        return Ok(FetchedDep {
            name: name.to_string(),
            include_dirs,
            lib_dirs: Vec::new(),
            libs: Vec::new(),
            frameworks: Vec::new(),
        });
    }

    let parent_root = ctx.project_root.clone();
    ctx.project_root = dep_dir.to_path_buf();
    ctx.building.insert(canonical_dep.clone());

    let build_result = build_project(ctx, ui);

    ctx.project_root = parent_root;
    ctx.building.remove(&canonical_dep);

    build_result?;

    let dep_manifest = Manifest::load(&dep_dir.join("Ordo.toml"))?;
    let dep_pkg_name = dep_manifest
        .package
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or(name);

    let dep_output_dir = if ctx.workspace_root.is_some() {
        ctx.target_dir.join(&ctx.profile_name).join(dep_pkg_name)
    } else {
        dep_dir.join(format!("target/{}", ctx.profile_name))
    };
    let (mut lib_dirs, mut libs) = scan_library_artifacts(&dep_output_dir);
    let mut frameworks = Vec::new();

    let dep_cache_path = dep_output_dir.join(DEP_CACHE_FILE);
    if let Ok(transitive) = load_dep_cache(&dep_cache_path) {
        for t in &transitive {
            include_dirs.extend(t.include_dirs.iter().cloned());
            lib_dirs.extend(t.lib_dirs.iter().cloned());
            libs.extend(t.libs.iter().cloned());
            frameworks.extend(t.frameworks.iter().cloned());
        }
    }

    ui.style.success("Built", &format!("{name} (path)"));

    let dep = FetchedDep {
        name: name.to_string(),
        include_dirs,
        lib_dirs,
        libs,
        frameworks,
    };

    ctx.built_deps.insert(canonical_dep, dep.clone());

    Ok(dep)
}

fn collect_member_as_fetched_dep(
    member_dir: &Path,
    target_dir: &Path,
    profile_name: &str,
) -> Option<FetchedDep> {
    let manifest = Manifest::load(&member_dir.join("Ordo.toml")).ok()?;
    let pkg = manifest.package.as_ref()?;
    let pkg_name = &pkg.name;

    let include_dir = member_dir.join("include");
    let mut include_dirs = if include_dir.exists() {
        vec![fs::canonicalize(&include_dir).ok()?]
    } else {
        vec![fs::canonicalize(member_dir).ok()?]
    };

    let output_dir = target_dir.join(profile_name).join(pkg_name);
    let (mut lib_dirs, mut libs) = scan_library_artifacts(&output_dir);
    let mut frameworks = Vec::new();

    let dep_cache_path = output_dir.join(DEP_CACHE_FILE);
    if let Ok(transitive) = load_dep_cache(&dep_cache_path) {
        for t in &transitive {
            include_dirs.extend(t.include_dirs.iter().cloned());
            lib_dirs.extend(t.lib_dirs.iter().cloned());
            libs.extend(t.libs.iter().cloned());
            frameworks.extend(t.frameworks.iter().cloned());
        }
    }

    Some(FetchedDep {
        name: pkg_name.clone(),
        include_dirs,
        lib_dirs,
        libs,
        frameworks,
    })
}

fn scan_library_artifacts(output_dir: &Path) -> (Vec<PathBuf>, Vec<String>) {
    if !output_dir.exists() {
        return (Vec::new(), Vec::new());
    }

    let Ok(entries) = fs::read_dir(output_dir) else {
        return (Vec::new(), Vec::new());
    };

    let mut libs = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        match ext {
            "a" | "so" | "dylib" | "lib" => {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let lib_name = stem.strip_prefix("lib").unwrap_or(stem);
                    libs.push(lib_name.to_string());
                }
            }
            _ => {}
        }
    }

    libs.sort();
    libs.dedup();

    let lib_dirs = if libs.is_empty() {
        Vec::new()
    } else {
        vec![output_dir.to_path_buf()]
    };

    (lib_dirs, libs)
}

fn build_link_flags(manifest: &Manifest, ctx: &BuildContext, deps: &[FetchedDep]) -> LinkFlags {
    let profile = manifest
        .resolve_profile(&ctx.profile_name)
        .unwrap_or_else(|_| {
            if ctx.release {
                crate::core::manifest::Profile::release_defaults()
            } else {
                crate::core::manifest::Profile::dev_defaults()
            }
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

    let linker = profile.linker.as_deref().and_then(|l| match l {
        "lld" => Some(crate::core::manifest::LinkerKind::Lld),
        "mold" => Some(crate::core::manifest::LinkerKind::Mold),
        "gold" => Some(crate::core::manifest::LinkerKind::Gold),
        _ => None,
    });

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

fn invoke_ninja(build_dir: &Path, jobs: Option<u32>, _verbose: u8, ui: &Context) -> Result<()> {
    use crate::util::style::BuildStep;

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

    let spinner = ui.style.create_spinner("");
    let mut had_progress = false;
    let mut output_lines: Vec<String> = Vec::new();
    let mut prev_step: Option<BuildStep> = None;

    let reader = BufReader::new(stdout);
    for line in reader.lines() {
        let line = line.into_diagnostic()?;

        if let Some(parsed) = parse_ninja_progress(&line) {
            had_progress = true;

            let file = clean_path(parsed.description.as_deref().unwrap_or(""));

            let (active_verb, done_verb) = if parsed.action.contains("Linking") {
                ("Linking", "Linked")
            } else if parsed.action.contains("Archiving") {
                ("Archiving", "Archived")
            } else {
                ("Compiling", "Compiled")
            };

            if parsed.current > 1
                && let Some(ref prev) = prev_step
            {
                spinner.finish_and_clear();
                ui.style.finish_build_step(prev);
            }

            let step = BuildStep {
                action: active_verb.to_string(),
                file: file.clone(),
                current: parsed.current,
                total: parsed.total,
                done_verb,
            };

            ui.style.display_build_step(&step, &spinner);
            prev_step = Some(step);
        } else if !line.trim().is_empty() {
            output_lines.push(line);
        }
    }

    spinner.finish_and_clear();

    let status = child.wait().into_diagnostic()?;

    if had_progress && let Some(ref prev) = prev_step {
        if status.success() {
            ui.style.finish_build_step(prev);
        } else {
            ui.style.finish_build_failed(prev);
        }
    }

    let stderr_lines = stderr_handle.join().unwrap_or_default();
    if !status.success() {
        for line in &output_lines {
            eprintln!("  {line}");
        }
        for line in &stderr_lines {
            eprintln!("  {line}");
        }
        ui.style.error("Build failed", "");
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn scan_library_artifacts_finds_libs() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("libfoo.a"), b"").unwrap();
        fs::write(tmp.path().join("libbar.so"), b"").unwrap();
        fs::write(tmp.path().join("libbaz.dylib"), b"").unwrap();
        fs::write(tmp.path().join("qux.lib"), b"").unwrap();
        fs::write(tmp.path().join("README.md"), b"").unwrap();

        let (lib_dirs, libs) = scan_library_artifacts(tmp.path());
        assert_eq!(lib_dirs.len(), 1);
        assert_eq!(lib_dirs[0], tmp.path());
        assert_eq!(libs, vec!["bar", "baz", "foo", "qux"]);
    }

    #[test]
    fn scan_library_artifacts_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let (lib_dirs, libs) = scan_library_artifacts(tmp.path());
        assert!(lib_dirs.is_empty());
        assert!(libs.is_empty());
    }

    #[test]
    fn scan_library_artifacts_nonexistent_dir() {
        let (lib_dirs, libs) = scan_library_artifacts(Path::new("/nonexistent"));
        assert!(lib_dirs.is_empty());
        assert!(libs.is_empty());
    }

    #[test]
    fn fetch_path_dep_missing_dir() {
        let ui = Context::default_for_test();
        let mut ctx = BuildContext {
            project_root: PathBuf::from("/tmp"),
            workspace_root: None,
            target_dir: PathBuf::from("/tmp/target"),
            profile_name: "debug".to_string(),
            release: false,
            jobs: None,
            verbose: 0,
            no_cache: false,
            features: Vec::new(),
            no_default_features: false,
            all_features: false,
            locked: false,
            frozen: false,
            building: HashSet::new(),
            built_deps: std::collections::HashMap::new(),
        };
        let result = fetch_path_dep("mylib", Path::new("/nonexistent/mylib"), &mut ctx, &ui);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("does not exist"), "got: {msg}");
    }

    #[test]
    fn fetch_path_dep_missing_manifest() {
        let ui = Context::default_for_test();
        let tmp = TempDir::new().unwrap();
        let mut ctx = BuildContext {
            project_root: tmp.path().to_path_buf(),
            workspace_root: None,
            target_dir: tmp.path().join("target"),
            profile_name: "debug".to_string(),
            release: false,
            jobs: None,
            verbose: 0,
            no_cache: false,
            features: Vec::new(),
            no_default_features: false,
            all_features: false,
            locked: false,
            frozen: false,
            building: HashSet::new(),
            built_deps: std::collections::HashMap::new(),
        };
        let result = fetch_path_dep("mylib", tmp.path(), &mut ctx, &ui);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("no Ordo.toml"), "got: {msg}");
    }

    #[test]
    fn fetch_path_dep_circular_dependency() {
        let tmp = TempDir::new().unwrap();
        let dep_dir = tmp.path().join("mylib");
        fs::create_dir_all(&dep_dir).unwrap();
        fs::write(
            dep_dir.join("Ordo.toml"),
            r#"[package]
name = "mylib"
version = "0.1.0"
type = "static-library"
"#,
        )
        .unwrap();

        let canonical = fs::canonicalize(&dep_dir).unwrap();
        let mut ctx = BuildContext {
            project_root: tmp.path().to_path_buf(),
            workspace_root: None,
            target_dir: tmp.path().join("target"),
            profile_name: "debug".to_string(),
            release: false,
            jobs: None,
            verbose: 0,
            no_cache: false,
            features: Vec::new(),
            no_default_features: false,
            all_features: false,
            locked: false,
            frozen: false,
            building: HashSet::from([canonical]),
            built_deps: std::collections::HashMap::new(),
        };
        let ui = Context::default_for_test();
        let result = fetch_path_dep("mylib", &dep_dir, &mut ctx, &ui);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("circular"), "got: {msg}");
    }

    #[test]
    fn fetch_path_dep_header_only() {
        let tmp = TempDir::new().unwrap();
        let dep_dir = tmp.path().join("myheader");
        let include_dir = dep_dir.join("include");
        fs::create_dir_all(&include_dir).unwrap();
        fs::write(include_dir.join("myheader.h"), "// header").unwrap();
        fs::write(
            dep_dir.join("Ordo.toml"),
            r#"[package]
name = "myheader"
version = "0.1.0"
type = "static-library"
"#,
        )
        .unwrap();

        let mut ctx = BuildContext {
            project_root: tmp.path().to_path_buf(),
            workspace_root: None,
            target_dir: tmp.path().join("target"),
            profile_name: "debug".to_string(),
            release: false,
            jobs: None,
            verbose: 0,
            no_cache: false,
            features: Vec::new(),
            no_default_features: false,
            all_features: false,
            locked: false,
            frozen: false,
            building: HashSet::new(),
            built_deps: std::collections::HashMap::new(),
        };
        let ui = Context::default_for_test();
        let dep = fetch_path_dep("myheader", &dep_dir, &mut ctx, &ui).unwrap();
        assert_eq!(dep.name, "myheader");
        assert_eq!(dep.include_dirs.len(), 1);
        assert!(dep.include_dirs[0].ends_with("include"));
        assert!(dep.lib_dirs.is_empty());
        assert!(dep.libs.is_empty());
    }

    #[test]
    fn dep_cache_round_trip() {
        let tmp = TempDir::new().unwrap();
        let cache_path = tmp.path().join("target/.dep-cache.json");

        let deps = vec![
            FetchedDep {
                name: "fmt".to_string(),
                include_dirs: vec![PathBuf::from("/usr/include/fmt")],
                lib_dirs: vec![PathBuf::from("/usr/lib")],
                libs: vec!["fmt".to_string()],
                frameworks: Vec::new(),
            },
            FetchedDep {
                name: "zlib".to_string(),
                include_dirs: Vec::new(),
                lib_dirs: Vec::new(),
                libs: vec!["z".to_string()],
                frameworks: Vec::new(),
            },
        ];

        save_dep_cache(&cache_path, &deps).unwrap();
        let loaded = load_dep_cache(&cache_path).unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].name, "fmt");
        assert_eq!(loaded[0].libs, vec!["fmt"]);
        assert_eq!(loaded[1].name, "zlib");
    }

    #[test]
    fn dep_cache_missing_returns_error() {
        let result = load_dep_cache(Path::new("/nonexistent/.dep-cache.json"));
        assert!(result.is_err());
    }
}
