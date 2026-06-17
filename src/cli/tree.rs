use crate::backend::provider::conan::ConanProvider;
use crate::backend::provider::pkgconfig::PkgConfigProvider;
use crate::backend::provider::system::SystemProvider;
use crate::backend::provider::vcpkg::{VcpkgPackageSpec, VcpkgProvider};
use crate::backend::provider::{FetchedDep, Provider};
use crate::core::manifest::{DependencySource, Manifest, ProviderKind};
use crate::util::style;
use miette::{Result, bail};
use std::path::Path;

pub fn run(dir: &Path) -> Result<()> {
    let manifest_path = dir.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in current directory");
    }

    let manifest = Manifest::load(&manifest_path)?;

    if manifest.is_workspace() {
        return run_workspace_tree(dir, &manifest);
    }

    let pkg = manifest.package();
    eprintln!("{} v{}", pkg.name, pkg.version);

    if manifest.dependencies.is_empty() {
        style::meta("no dependencies");
        return Ok(());
    }

    let spinner = style::create_spinner("Fetching dependency info…");
    let fetched = fetch_all_for_tree(&manifest, dir);
    spinner.finish_and_clear();

    let deps: Vec<_> = manifest.dependencies.iter().collect();
    let last_idx = deps.len() - 1;

    for (i, (name, spec)) in deps.iter().enumerate() {
        let is_last = i == last_idx;
        let prefix = if is_last { "└── " } else { "├── " };
        let cont = if is_last { "    " } else { "│   " };

        let version_str = spec.version.as_deref().unwrap_or("*");
        let source_str = match spec.source_kind() {
            DependencySource::Provider(ProviderKind::Vcpkg) => " (vcpkg)",
            DependencySource::Provider(ProviderKind::Conan) => " (conan)",
            DependencySource::Provider(ProviderKind::PkgConfig) => " (pkg-config)",
            DependencySource::Provider(ProviderKind::System) => " (system)",
            DependencySource::Git => " (git)",
            DependencySource::Path => " (path)",
            _ => "",
        };

        let optional_str = if spec.optional { " [optional]" } else { "" };
        eprintln!("{prefix}{name} v{version_str}{source_str}{optional_str}");

        if let Some(dep) = fetched.get(name.as_str()) {
            if !dep.libs.is_empty() {
                style::tree_detail(cont, &format!("libs: {}", dep.libs.join(", ")));
            }
            if !dep.frameworks.is_empty() {
                style::tree_detail(cont, &format!("frameworks: {}", dep.frameworks.join(", ")));
            }
            if !dep.include_dirs.is_empty() {
                let dirs: Vec<String> = dep
                    .include_dirs
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect();
                style::tree_detail(cont, &format!("include: {}", dirs.join(", ")));
            }
        }
    }

    Ok(())
}

fn run_workspace_tree(dir: &Path, manifest: &Manifest) -> Result<()> {
    use crate::core::workspace::Workspace;

    let ws = Workspace::load(dir)?;
    let dag = ws.build_dag()?;

    if let Some(ref pkg) = manifest.package {
        eprintln!("{} v{} (workspace)", pkg.name, pkg.version);
    } else {
        eprintln!("(workspace)");
    }

    let members = &dag.order;
    let last_idx = members.len().saturating_sub(1);

    for (i, name) in members.iter().enumerate() {
        let is_last = i == last_idx;
        let prefix = if is_last { "└── " } else { "├── " };
        let cont = if is_last { "    " } else { "│   " };

        let member = ws.find_member(name).unwrap();
        let pkg = member.manifest.package();
        eprintln!("{prefix}{} v{}", pkg.name, pkg.version);

        let deps: Vec<String> = dag.deps_of(name).to_vec();
        if !deps.is_empty() {
            style::tree_detail(cont, &format!("deps: {}", deps.join(", ")));
        }

        if !member.manifest.dependencies.is_empty() {
            let ext_deps: Vec<&str> = member
                .manifest
                .dependencies
                .iter()
                .filter(|(_, spec)| spec.source_kind() != DependencySource::Path)
                .map(|(n, _)| n.as_str())
                .collect();
            if !ext_deps.is_empty() {
                style::tree_detail(cont, &format!("ext: {}", ext_deps.join(", ")));
            }
        }
    }

    Ok(())
}

fn fetch_all_for_tree(
    manifest: &Manifest,
    dir: &Path,
) -> std::collections::HashMap<String, FetchedDep> {
    let mut result = std::collections::HashMap::new();

    // Batch vcpkg install first
    let vcpkg_deps: Vec<VcpkgPackageSpec<'_>> = manifest
        .dependencies
        .iter()
        .filter(|(_, spec)| spec.source_kind() == DependencySource::Provider(ProviderKind::Vcpkg))
        .map(|(name, spec)| VcpkgPackageSpec {
            name: name.as_str(),
            version: spec.version.as_deref(),
        })
        .collect();

    if !vcpkg_deps.is_empty() {
        let vcpkg = VcpkgProvider::new();
        let _ = vcpkg.install_packages(&vcpkg_deps, &|_| {});
    }

    for (name, spec) in &manifest.dependencies {
        let fetched = match spec.source_kind() {
            DependencySource::Provider(ProviderKind::Vcpkg) => {
                let p = VcpkgProvider::new();
                p.resolve(name, spec.version.as_deref())
                    .and_then(|r| p.fetch(&r))
                    .ok()
            }
            DependencySource::Provider(ProviderKind::Conan) => {
                let p = ConanProvider::new();
                p.resolve(name, spec.version.as_deref())
                    .and_then(|r| p.fetch(&r))
                    .ok()
            }
            DependencySource::Provider(ProviderKind::PkgConfig) => {
                let p = PkgConfigProvider;
                p.resolve(name, spec.version.as_deref())
                    .and_then(|r| p.fetch(&r))
                    .ok()
            }
            DependencySource::Provider(ProviderKind::System) => {
                let p = SystemProvider;
                p.resolve(name, spec.version.as_deref())
                    .and_then(|r| p.fetch(&r))
                    .ok()
            }
            DependencySource::Path => {
                let dep_dir = dir.join(spec.path.as_ref().unwrap());
                let include_dir = dep_dir.join("include");
                let include_dirs = if include_dir.exists() {
                    vec![include_dir]
                } else if dep_dir.exists() {
                    vec![dep_dir]
                } else {
                    Vec::new()
                };
                Some(FetchedDep {
                    name: name.clone(),
                    include_dirs,
                    lib_dirs: Vec::new(),
                    libs: Vec::new(),
                    frameworks: Vec::new(),
                })
            }
            _ => None,
        };

        if let Some(dep) = fetched {
            result.insert(name.clone(), dep);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn project_with_deps(toml: &str) -> TempDir {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Ordo.toml"), toml).unwrap();
        tmp
    }

    #[test]
    fn tree_no_deps() {
        let tmp = project_with_deps(
            r#"[package]
name = "myapp"
version = "0.1.0"
type = "executable"
"#,
        );
        run(tmp.path()).unwrap();
    }

    #[test]
    fn tree_with_deps() {
        let tmp = project_with_deps(
            r#"[package]
name = "myapp"
version = "0.1.0"
type = "executable"

[dependencies]
fmt = { version = "11", provider = "vcpkg" }
zlib = { provider = "system" }
"#,
        );
        run(tmp.path()).unwrap();
    }

    #[test]
    fn tree_missing_manifest() {
        let tmp = TempDir::new().unwrap();
        let result = run(tmp.path());
        assert!(result.is_err());
    }
}
