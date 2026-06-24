use super::{FetchedDep, Provider, ResolvedDep};
use miette::{Result, bail};
use std::path::PathBuf;
use std::process::Command;

pub struct NixProvider;

impl Provider for NixProvider {
    fn name(&self) -> &str {
        "nix"
    }

    fn resolve(&self, name: &str, _version: Option<&str>) -> Result<ResolvedDep> {
        // Try `nix profile list` first (modern nix)
        if let Ok(output) = Command::new("nix").args(["profile", "list"]).output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.lines().any(|line| line.contains(name)) {
                let version = extract_nix_version(&stdout, name);
                return Ok(ResolvedDep {
                    name: name.to_string(),
                    version,
                    source: "nix".to_string(),
                    checksum: None,
                });
            }
        }

        // Fallback: `nix-env -q` (legacy)
        if let Ok(output) = Command::new("nix-env").args(["-q", name]).output()
            && output.status.success()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let version = stdout
                .lines()
                .find(|l| l.contains(name))
                .and_then(|l| l.strip_prefix(name))
                .map(|v| v.trim_start_matches('-').to_string())
                .unwrap_or_else(|| "unknown".to_string());

            return Ok(ResolvedDep {
                name: name.to_string(),
                version,
                source: "nix".to_string(),
                checksum: None,
            });
        }

        bail!(
            "nix: package '{name}' not found\n  \
             help: install with: nix profile install nixpkgs#{name}"
        );
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        let mut include_dirs = Vec::new();
        let mut lib_dirs = Vec::new();

        // Try to find the package path via `nix eval`
        if let Ok(output) = Command::new("nix")
            .args(["eval", "--raw", &format!("nixpkgs#{}", dep.name)])
            .output()
            && output.status.success()
        {
            let store_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let store = PathBuf::from(&store_path);
            let inc = store.join("include");
            let lib = store.join("lib");
            if inc.exists() {
                include_dirs.push(inc);
            }
            if lib.exists() {
                lib_dirs.push(lib);
            }
        }

        Ok(FetchedDep {
            name: dep.name.clone(),
            include_dirs,
            lib_dirs,
            libs: vec![dep.name.clone()],
            frameworks: Vec::new(),
        })
    }
}

fn extract_nix_version(profile_output: &str, name: &str) -> String {
    for line in profile_output.lines() {
        if !line.contains(name) {
            continue;
        }
        for part in line.split_whitespace() {
            if part.contains(name)
                && let Some(rest) = part.rsplit('/').next()
            {
                let without_hash = rest.split_once('-').map(|(_, r)| r).unwrap_or(rest);
                if let Some(ver) = without_hash.strip_prefix(name) {
                    let ver = ver.trim_start_matches('-');
                    if !ver.is_empty() {
                        return ver.to_string();
                    }
                }
            }
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_version_from_store_path() {
        let output = "0 - - /nix/store/abc123-openssl-3.1.4";
        let version = extract_nix_version(output, "openssl");
        assert_eq!(version, "3.1.4");
    }

    #[test]
    fn extract_version_not_found() {
        let output = "0 - - /nix/store/abc123-zlib-1.3";
        let version = extract_nix_version(output, "openssl");
        assert_eq!(version, "unknown");
    }
}
