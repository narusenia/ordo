use super::{FetchedDep, Provider, ResolvedDep};
use miette::{IntoDiagnostic, Result, bail};
use std::process::Command;

pub struct PacmanProvider;

impl Provider for PacmanProvider {
    fn name(&self) -> &str {
        "pacman"
    }

    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep> {
        let output = Command::new("pacman")
            .args(["-Qi", name])
            .output()
            .into_diagnostic()?;

        if !output.status.success() {
            bail!(
                "pacman: package '{name}' is not installed\n  \
                 help: install with: sudo pacman -S {name}"
            );
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let resolved_version = stdout
            .lines()
            .find(|l| l.starts_with("Version"))
            .and_then(|l| l.split_once(':'))
            .map(|(_, v)| v.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if let Some(req) = version {
            let base_ver = resolved_version
                .split('-')
                .next()
                .unwrap_or(&resolved_version);
            if !base_ver.starts_with(req) && base_ver != req {
                bail!(
                    "pacman: package '{name}' version {resolved_version} does not satisfy >= {req}\n  \
                     help: sudo pacman -Syu {name}"
                );
            }
        }

        Ok(ResolvedDep {
            name: name.to_string(),
            version: resolved_version,
            source: "pacman".to_string(),
            checksum: None,
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        // pacman packages use standard system paths
        Ok(FetchedDep {
            name: dep.name.clone(),
            include_dirs: Vec::new(),
            lib_dirs: Vec::new(),
            libs: vec![dep.name.clone()],
            frameworks: Vec::new(),
            defines: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pacman_provider_name() {
        assert_eq!(PacmanProvider.name(), "pacman");
    }

    #[test]
    fn fetch_returns_lib_name() {
        let dep = ResolvedDep {
            name: "zlib".to_string(),
            version: "1.3.1".to_string(),
            source: "pacman".to_string(),
            checksum: None,
        };
        let fetched = PacmanProvider.fetch(&dep).unwrap();
        assert_eq!(fetched.libs, vec!["zlib"]);
        assert!(fetched.include_dirs.is_empty());
    }
}
