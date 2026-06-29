use super::{FetchedDep, Provider, ResolvedDep};
use miette::{IntoDiagnostic, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct BrewProvider;

impl Provider for BrewProvider {
    fn name(&self) -> &str {
        "brew"
    }

    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep> {
        let output = Command::new("brew")
            .args(["info", "--json=v2", name])
            .output()
            .into_diagnostic()?;

        if !output.status.success() {
            bail!(
                "brew: package '{name}' not found\n  \
                 help: install with: brew install {name}"
            );
        }

        let json: serde_json::Value = serde_json::from_slice(&output.stdout).into_diagnostic()?;

        let resolved_version = json["formulae"]
            .as_array()
            .and_then(|f| f.first())
            .and_then(|f| f["versions"]["stable"].as_str())
            .unwrap_or("unknown")
            .to_string();

        if let Some(req) = version
            && !resolved_version.starts_with(req)
            && resolved_version != req
        {
            bail!(
                "brew: package '{name}' version {resolved_version} does not satisfy >= {req}\n  \
                 help: brew upgrade {name}"
            );
        }

        let installed = json["formulae"]
            .as_array()
            .and_then(|f| f.first())
            .and_then(|f| f["installed"].as_array())
            .map(|i| !i.is_empty())
            .unwrap_or(false);

        if !installed {
            bail!(
                "brew: package '{name}' is not installed\n  \
                 help: install with: brew install {name}"
            );
        }

        Ok(ResolvedDep {
            name: name.to_string(),
            version: resolved_version,
            source: "brew".to_string(),
            checksum: None,
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        let prefix_output = Command::new("brew")
            .args(["--prefix", &dep.name])
            .output()
            .into_diagnostic()?;

        let prefix = String::from_utf8_lossy(&prefix_output.stdout)
            .trim()
            .to_string();

        if prefix.is_empty() {
            return Ok(FetchedDep {
                name: dep.name.clone(),
                include_dirs: Vec::new(),
                lib_dirs: Vec::new(),
                libs: vec![dep.name.clone()],
                frameworks: Vec::new(),
                defines: Vec::new(),
            });
        }

        let prefix_path = PathBuf::from(&prefix);
        let include_dir = prefix_path.join("include");
        let lib_dir = prefix_path.join("lib");

        let mut include_dirs = Vec::new();
        if include_dir.exists() {
            include_dirs.push(include_dir);
        }

        let mut lib_dirs = Vec::new();
        if lib_dir.exists() {
            lib_dirs.push(lib_dir);
        }

        let libs = find_lib_names(&prefix_path);

        Ok(FetchedDep {
            name: dep.name.clone(),
            include_dirs,
            lib_dirs,
            libs,
            frameworks: Vec::new(),
            defines: Vec::new(),
        })
    }
}

fn find_lib_names(prefix: &Path) -> Vec<String> {
    let lib_dir = prefix.join("lib");
    if !lib_dir.exists() {
        return vec![];
    }

    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&lib_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name();
            let fname = fname.to_string_lossy();
            // Match lib<name>.a or lib<name>.dylib or lib<name>.so
            if let Some(rest) = fname.strip_prefix("lib")
                && let Some(name) = rest
                    .strip_suffix(".a")
                    .or_else(|| rest.strip_suffix(".dylib"))
                    .or_else(|| rest.strip_suffix(".so"))
                && !name.is_empty()
                && !names.contains(&name.to_string())
            {
                names.push(name.to_string());
            }
        }
    }

    if names.is_empty() {
        names.push(prefix.file_name().unwrap().to_string_lossy().to_string());
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn find_lib_names_from_dir() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("lib");
        std::fs::create_dir(&lib_dir).unwrap();
        std::fs::write(lib_dir.join("libssl.a"), "").unwrap();
        std::fs::write(lib_dir.join("libcrypto.dylib"), "").unwrap();
        std::fs::write(lib_dir.join("libz.so"), "").unwrap();

        let names = find_lib_names(&tmp.path().to_path_buf());
        assert!(names.contains(&"ssl".to_string()));
        assert!(names.contains(&"crypto".to_string()));
        assert!(names.contains(&"z".to_string()));
    }

    #[test]
    fn find_lib_names_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let lib_dir = tmp.path().join("lib");
        std::fs::create_dir(&lib_dir).unwrap();

        let names = find_lib_names(&tmp.path().to_path_buf());
        assert_eq!(names.len(), 1); // falls back to dir name
    }
}
