use crate::backend::provider::conan::ConanProvider;
use crate::backend::provider::pkgconfig::PkgConfigProvider;
use crate::backend::provider::system::SystemProvider;
use crate::backend::provider::vcpkg::{VcpkgPackageSpec, VcpkgProvider};
use crate::backend::provider::{FetchedDep, Provider};
use crate::core::manifest::{DependencySource, Manifest, ProviderKind};
use crate::util::style;
use miette::{bail, Result};
use std::path::Path;

pub fn run(dir: &Path) -> Result<()> {
    let manifest_path = dir.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in current directory");
    }

    let manifest = Manifest::load(&manifest_path)?;

    eprintln!(
        "{} v{}",
        manifest.package.name, manifest.package.version
    );

    if manifest.dependencies.is_empty() {
        style::meta("no dependencies");
        return Ok(());
    }

    let fetched = fetch_all_for_tree(&manifest);

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
                style::tree_line(&format!("{cont}libs: {}", dep.libs.join(", ")));
            }
            if !dep.frameworks.is_empty() {
                style::tree_line(&format!("{cont}frameworks: {}", dep.frameworks.join(", ")));
            }
            if !dep.include_dirs.is_empty() {
                let dirs: Vec<String> = dep.include_dirs.iter().map(|p| p.display().to_string()).collect();
                style::tree_line(&format!("{cont}include: {}", dirs.join(", ")));
            }
        }
    }

    Ok(())
}

fn fetch_all_for_tree(manifest: &Manifest) -> std::collections::HashMap<String, FetchedDep> {
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
