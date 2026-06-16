#![allow(dead_code)]

use crate::core::manifest::{DependencySource, DependencySpec, Manifest};
use miette::{bail, Result};
use pubgrub::{OfflineDependencyProvider, Ranges, resolve};
use semver::Version;
use std::collections::BTreeMap;

type SemverRanges = Ranges<Version>;

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: Version,
    pub source: DependencySource,
}

pub fn resolve_dependencies(manifest: &Manifest) -> Result<Vec<ResolvedPackage>> {
    if manifest.dependencies.is_empty() {
        return Ok(Vec::new());
    }

    let mut provider = OfflineDependencyProvider::<String, SemverRanges>::new();
    let root_name = format!("{}@root", manifest.package.name);
    let root_version = parse_version(&manifest.package.version)?;

    let mut root_deps: Vec<(String, SemverRanges)> = Vec::new();
    let mut source_map: BTreeMap<String, DependencySource> = BTreeMap::new();

    for (name, spec) in &manifest.dependencies {
        let range = spec_to_range(name, spec)?;
        root_deps.push((name.clone(), range.clone()));
        source_map.insert(name.clone(), spec.source_kind());

        // For path/provider/system deps, register a single known version
        register_stub_package(&mut provider, name, spec)?;
    }

    provider.add_dependencies(root_name.clone(), root_version.clone(), root_deps);

    let solution = resolve(&provider, root_name.clone(), root_version).map_err(|e| {
        miette::miette!("dependency resolution failed:\n{e}")
    })?;

    let mut resolved = Vec::new();
    for (pkg, version) in solution {
        if pkg == root_name {
            continue;
        }
        let source = source_map
            .get(&pkg)
            .cloned()
            .unwrap_or(DependencySource::Unknown);
        resolved.push(ResolvedPackage {
            name: pkg,
            version,
            source,
        });
    }

    resolved.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(resolved)
}

fn spec_to_range(name: &str, spec: &DependencySpec) -> Result<SemverRanges> {
    match spec.source_kind() {
        DependencySource::Path | DependencySource::Workspace => {
            // Path/workspace deps match any version
            Ok(Ranges::full())
        }
        DependencySource::Git => {
            // Git deps: if version specified use it, otherwise match any
            match &spec.version {
                Some(v) => parse_version_req(v),
                None => Ok(Ranges::full()),
            }
        }
        DependencySource::Provider(_) | DependencySource::Registry => {
            match &spec.version {
                Some(v) => parse_version_req(v),
                None => Ok(Ranges::full()),
            }
        }
        DependencySource::Unknown => {
            bail!("dependency '{name}' has no source specified (add path, git, provider, or version)")
        }
    }
}

fn register_stub_package(
    provider: &mut OfflineDependencyProvider<String, SemverRanges>,
    name: &str,
    spec: &DependencySpec,
) -> Result<()> {
    let version = match &spec.version {
        Some(v) => {
            let trimmed = v.trim_start_matches(|c: char| !c.is_ascii_digit());
            parse_version(trimmed).unwrap_or_else(|_| Version::new(0, 0, 0))
        }
        None => Version::new(0, 0, 0),
    };

    // Register this package with no transitive dependencies for now
    let no_deps: Vec<(String, SemverRanges)> = Vec::new();
    provider.add_dependencies(name.to_string(), version, no_deps);

    Ok(())
}

fn parse_version(s: &str) -> Result<Version> {
    // Normalize: "11" → "11.0.0", "1.2" → "1.2.0"
    let normalized = normalize_version(s);
    Version::parse(&normalized)
        .map_err(|e| miette::miette!("invalid version '{s}': {e}"))
}

fn parse_version_req(req: &str) -> Result<SemverRanges> {
    let req = req.trim();

    // Detect operator prefix
    if let Some(rest) = req.strip_prefix(">=") {
        let v = parse_version(rest.trim())?;
        return Ok(Ranges::higher_than(v));
    }
    if let Some(rest) = req.strip_prefix("<=") {
        let v = parse_version(rest.trim())?;
        return Ok(Ranges::strictly_lower_than(v.clone()).union(&Ranges::singleton(v)));
    }
    if let Some(rest) = req.strip_prefix('>') {
        let v = parse_version(rest.trim())?;
        return Ok(Ranges::strictly_higher_than(v));
    }
    if let Some(rest) = req.strip_prefix('<') {
        let v = parse_version(rest.trim())?;
        return Ok(Ranges::strictly_lower_than(v));
    }
    if let Some(rest) = req.strip_prefix('~') {
        let v = parse_version(rest.trim())?;
        let upper = Version::new(v.major, v.minor + 1, 0);
        return Ok(Ranges::between(v, upper));
    }
    if let Some(rest) = req.strip_prefix('=') {
        let v = parse_version(rest.trim())?;
        return Ok(Ranges::singleton(v));
    }

    // Wildcard
    if req == "*" {
        return Ok(Ranges::full());
    }

    // Strip optional ^ prefix
    let version_str = req.strip_prefix('^').unwrap_or(req);
    let v = parse_version(version_str.trim())?;

    // ^ (caret) semantics: compatible with version
    let upper = if v.major > 0 {
        Version::new(v.major + 1, 0, 0)
    } else if v.minor > 0 {
        Version::new(0, v.minor + 1, 0)
    } else {
        Version::new(0, 0, v.patch + 1)
    };

    Ok(Ranges::between(v, upper))
}

fn normalize_version(s: &str) -> String {
    let parts: Vec<&str> = s.split('.').collect();
    match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manifest::Manifest;
    use std::path::PathBuf;

    fn parse_manifest(content: &str) -> Manifest {
        Manifest::parse(content, &PathBuf::from("Ordo.toml")).unwrap()
    }

    #[test]
    fn resolve_no_dependencies() {
        let m = parse_manifest(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"
            "#,
        );
        let resolved = resolve_dependencies(&m).unwrap();
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_single_path_dep() {
        let m = parse_manifest(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            core = { path = "../core" }
            "#,
        );
        let resolved = resolve_dependencies(&m).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "core");
        assert_eq!(resolved[0].source, DependencySource::Path);
    }

    #[test]
    fn resolve_version_dep() {
        let m = parse_manifest(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            fmt = "11"
            "#,
        );
        let resolved = resolve_dependencies(&m).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "fmt");
        assert_eq!(resolved[0].version, Version::new(11, 0, 0));
    }

    #[test]
    fn resolve_multiple_deps() {
        let m = parse_manifest(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            fmt = "11"
            core = { path = "../core" }
            spdlog = { version = "1.14", provider = "vcpkg" }
            "#,
        );
        let resolved = resolve_dependencies(&m).unwrap();
        assert_eq!(resolved.len(), 3);
        // Sorted by name
        assert_eq!(resolved[0].name, "core");
        assert_eq!(resolved[1].name, "fmt");
        assert_eq!(resolved[2].name, "spdlog");
    }

    #[test]
    fn parse_caret_default() {
        let range = parse_version_req("1.2.3").unwrap();
        assert!(range.contains(&Version::new(1, 2, 3)));
        assert!(range.contains(&Version::new(1, 9, 0)));
        assert!(!range.contains(&Version::new(2, 0, 0)));
        assert!(!range.contains(&Version::new(1, 2, 2)));
    }

    #[test]
    fn parse_caret_explicit() {
        let range = parse_version_req("^1.2").unwrap();
        assert!(range.contains(&Version::new(1, 2, 0)));
        assert!(range.contains(&Version::new(1, 9, 9)));
        assert!(!range.contains(&Version::new(2, 0, 0)));
    }

    #[test]
    fn parse_tilde() {
        let range = parse_version_req("~1.2.3").unwrap();
        assert!(range.contains(&Version::new(1, 2, 3)));
        assert!(range.contains(&Version::new(1, 2, 9)));
        assert!(!range.contains(&Version::new(1, 3, 0)));
    }

    #[test]
    fn parse_exact() {
        let range = parse_version_req("=1.2.3").unwrap();
        assert!(range.contains(&Version::new(1, 2, 3)));
        assert!(!range.contains(&Version::new(1, 2, 4)));
    }

    #[test]
    fn parse_gte() {
        let range = parse_version_req(">=1.0.0").unwrap();
        assert!(range.contains(&Version::new(1, 0, 0)));
        assert!(range.contains(&Version::new(99, 0, 0)));
        assert!(!range.contains(&Version::new(0, 9, 9)));
    }

    #[test]
    fn parse_gt() {
        let range = parse_version_req(">1.0.0").unwrap();
        assert!(!range.contains(&Version::new(1, 0, 0)));
        assert!(range.contains(&Version::new(1, 0, 1)));
    }

    #[test]
    fn parse_lt() {
        let range = parse_version_req("<2.0.0").unwrap();
        assert!(range.contains(&Version::new(1, 9, 9)));
        assert!(!range.contains(&Version::new(2, 0, 0)));
    }

    #[test]
    fn parse_lte() {
        let range = parse_version_req("<=2.0.0").unwrap();
        assert!(range.contains(&Version::new(2, 0, 0)));
        assert!(!range.contains(&Version::new(2, 0, 1)));
    }

    #[test]
    fn normalize_single_number() {
        assert_eq!(normalize_version("11"), "11.0.0");
    }

    #[test]
    fn normalize_two_numbers() {
        assert_eq!(normalize_version("1.2"), "1.2.0");
    }

    #[test]
    fn normalize_three_numbers() {
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
    }

    #[test]
    fn parse_wildcard() {
        let range = parse_version_req("*").unwrap();
        assert!(range.contains(&Version::new(0, 0, 0)));
        assert!(range.contains(&Version::new(99, 99, 99)));
    }

    #[test]
    fn resolve_wildcard_dep() {
        let m = parse_manifest(
            r#"
            [package]
            name = "myapp"
            version = "0.1.0"
            type = "executable"

            [dependencies]
            hoge = "*"
            "#,
        );
        let resolved = resolve_dependencies(&m).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "hoge");
    }

    #[test]
    fn caret_zero_major() {
        let range = parse_version_req("^0.2.3").unwrap();
        assert!(range.contains(&Version::new(0, 2, 3)));
        assert!(range.contains(&Version::new(0, 2, 9)));
        assert!(!range.contains(&Version::new(0, 3, 0)));
    }

    #[test]
    fn caret_zero_minor() {
        let range = parse_version_req("^0.0.3").unwrap();
        assert!(range.contains(&Version::new(0, 0, 3)));
        assert!(!range.contains(&Version::new(0, 0, 4)));
    }
}
