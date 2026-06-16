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

    let deps: Vec<_> = manifest.dependencies.iter().collect();
    let last_idx = deps.len() - 1;

    for (i, (name, spec)) in deps.iter().enumerate() {
        let is_last = i == last_idx;
        let prefix = if is_last { "└── " } else { "├── " };

        let version_str = spec.version.as_deref().unwrap_or("*");
        let source_str = match spec.source_kind() {
            DependencySource::Provider(ProviderKind::Vcpkg) => " (vcpkg)",
            DependencySource::Provider(ProviderKind::Conan) => " (conan)",
            DependencySource::Provider(ProviderKind::PkgConfig) => " (pkg-config)",
            DependencySource::Provider(ProviderKind::System) => " (system)",
            DependencySource::Git => " (git)",
            DependencySource::Path => " (path)",
            DependencySource::Registry => "",
            _ => "",
        };

        let optional_str = if spec.optional { " [optional]" } else { "" };

        eprintln!("{prefix}{name} v{version_str}{source_str}{optional_str}");
    }

    Ok(())
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
