use super::{FetchedDep, Provider, ResolvedDep};
use miette::{IntoDiagnostic, Result, bail};
use std::path::PathBuf;
use std::process::Command;

pub struct PkgConfigProvider;

impl Provider for PkgConfigProvider {
    fn name(&self) -> &str {
        "pkg-config"
    }

    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep> {
        if let Some(ver) = version {
            let status = Command::new("pkg-config")
                .args(["--atleast-version", ver, name])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .into_diagnostic()?;

            if !status.success() {
                bail!(
                    "pkg-config: package '{name}' version >= {ver} not found\n  \
                     help: install it via your system package manager"
                );
            }
        } else {
            let status = Command::new("pkg-config")
                .args(["--exists", name])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .into_diagnostic()?;

            if !status.success() {
                bail!(
                    "pkg-config: package '{name}' not found\n  \
                     help: install it via your system package manager"
                );
            }
        }

        let version_output = Command::new("pkg-config")
            .args(["--modversion", name])
            .output()
            .into_diagnostic()?;

        let resolved_version = String::from_utf8_lossy(&version_output.stdout)
            .trim()
            .to_string();

        Ok(ResolvedDep {
            name: name.to_string(),
            version: resolved_version,
            source: "pkg-config".to_string(),
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        let cflags_output = Command::new("pkg-config")
            .args(["--cflags", &dep.name])
            .output()
            .into_diagnostic()?;

        let libs_output = Command::new("pkg-config")
            .args(["--libs", &dep.name])
            .output()
            .into_diagnostic()?;

        let cflags = String::from_utf8_lossy(&cflags_output.stdout);
        let libs = String::from_utf8_lossy(&libs_output.stdout);

        let include_dirs = parse_include_dirs(&cflags);
        let (lib_dirs, lib_names) = parse_libs(&libs);

        let frameworks = parse_frameworks(&libs);

        Ok(FetchedDep {
            name: dep.name.clone(),
            include_dirs,
            lib_dirs,
            libs: lib_names,
            frameworks,
        })
    }
}

fn parse_include_dirs(cflags: &str) -> Vec<PathBuf> {
    cflags
        .split_whitespace()
        .filter_map(|flag| flag.strip_prefix("-I").map(PathBuf::from))
        .collect()
}

fn parse_frameworks(libs: &str) -> Vec<String> {
    let tokens: Vec<&str> = libs.split_whitespace().collect();
    let mut frameworks = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == "-framework"
            && let Some(&name) = tokens.get(i + 1)
        {
            frameworks.push(name.to_string());
            i += 2;
            continue;
        }
        i += 1;
    }
    frameworks
}

fn parse_libs(libs: &str) -> (Vec<PathBuf>, Vec<String>) {
    let mut lib_dirs = Vec::new();
    let mut lib_names = Vec::new();

    for flag in libs.split_whitespace() {
        if let Some(dir) = flag.strip_prefix("-L") {
            lib_dirs.push(PathBuf::from(dir));
        } else if let Some(name) = flag.strip_prefix("-l") {
            lib_names.push(name.to_string());
        }
    }

    (lib_dirs, lib_names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_include_dirs_basic() {
        let dirs = parse_include_dirs("-I/usr/include/openssl -I/opt/local/include");
        assert_eq!(
            dirs,
            vec![
                PathBuf::from("/usr/include/openssl"),
                PathBuf::from("/opt/local/include"),
            ]
        );
    }

    #[test]
    fn parse_include_dirs_empty() {
        let dirs = parse_include_dirs("");
        assert!(dirs.is_empty());
    }

    #[test]
    fn parse_include_dirs_mixed_flags() {
        let dirs = parse_include_dirs("-I/usr/include -DFOO=1 -Wall");
        assert_eq!(dirs, vec![PathBuf::from("/usr/include")]);
    }

    #[test]
    fn parse_libs_basic() {
        let (dirs, names) = parse_libs("-L/usr/lib -lssl -lcrypto");
        assert_eq!(dirs, vec![PathBuf::from("/usr/lib")]);
        assert_eq!(names, vec!["ssl", "crypto"]);
    }

    #[test]
    fn parse_libs_no_dir() {
        let (dirs, names) = parse_libs("-lz");
        assert!(dirs.is_empty());
        assert_eq!(names, vec!["z"]);
    }

    #[test]
    fn parse_libs_empty() {
        let (dirs, names) = parse_libs("");
        assert!(dirs.is_empty());
        assert!(names.is_empty());
    }

    #[test]
    fn parse_frameworks_from_libs_line() {
        let fws = parse_frameworks(
            "-L/usr/lib -lglfw3 -framework Cocoa -framework IOKit -framework CoreFoundation",
        );
        assert_eq!(fws, vec!["Cocoa", "IOKit", "CoreFoundation"]);
    }

    #[test]
    fn parse_frameworks_empty() {
        let fws = parse_frameworks("-L/usr/lib -lz");
        assert!(fws.is_empty());
    }

    #[test]
    fn parse_frameworks_no_input() {
        let fws = parse_frameworks("");
        assert!(fws.is_empty());
    }
}
