use super::{FetchedDep, Provider, ResolvedDep};
use miette::{IntoDiagnostic, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct NugetProvider;

impl Provider for NugetProvider {
    fn name(&self) -> &str {
        "nuget"
    }

    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep> {
        let status = Command::new("nuget")
            .arg("help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if status.is_err() || !status.unwrap().success() {
            bail!(
                "nuget: nuget CLI is not installed\n  \
                 help: install from https://www.nuget.org/downloads"
            );
        }

        Ok(ResolvedDep {
            name: name.to_string(),
            version: version.unwrap_or("latest").to_string(),
            source: "nuget".to_string(),
            checksum: None,
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        let cwd = std::env::current_dir().into_diagnostic()?;
        let packages_dir = cwd.join("packages");

        let mut args = vec!["install", &dep.name, "-OutputDirectory", "packages"];
        let version_str;
        if dep.version != "latest" {
            version_str = dep.version.clone();
            args.extend_from_slice(&["-Version", &version_str]);
        }

        let output = Command::new("nuget")
            .args(&args)
            .output()
            .into_diagnostic()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("nuget: failed to install '{}'\n  {stderr}", dep.name);
        }

        let (include_dirs, lib_dirs, libs) = find_native_paths(&packages_dir, &dep.name);

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

fn find_native_paths(packages_dir: &Path, name: &str) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<String>) {
    let mut include_dirs = Vec::new();
    let mut lib_dirs = Vec::new();
    let mut libs = Vec::new();

    if !packages_dir.exists() {
        return (include_dirs, lib_dirs, libs);
    }

    // NuGet packages are installed as <name>.<version>/
    if let Ok(entries) = std::fs::read_dir(packages_dir) {
        for entry in entries.flatten() {
            let dir_name = entry.file_name();
            let dir_name = dir_name.to_string_lossy();
            if dir_name.starts_with(name)
                || dir_name.to_lowercase().starts_with(&name.to_lowercase())
            {
                let pkg_dir = entry.path();

                // Check for native include dirs
                let build_native = pkg_dir.join("build").join("native").join("include");
                if build_native.exists() {
                    include_dirs.push(build_native);
                }
                let include = pkg_dir.join("include");
                if include.exists() {
                    include_dirs.push(include);
                }

                // Check for native lib dirs (x64)
                for lib_subdir in &[
                    "build/native/lib/x64",
                    "lib/native/x64",
                    "runtimes/win-x64/native",
                ] {
                    let lib_path = pkg_dir.join(lib_subdir);
                    if lib_path.exists() {
                        lib_dirs.push(lib_path.clone());
                        scan_lib_names(&lib_path, &mut libs);
                    }
                }
            }
        }
    }

    if libs.is_empty() {
        libs.push(name.to_string());
    }

    (include_dirs, lib_dirs, libs)
}

fn scan_lib_names(dir: &Path, libs: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name();
            let fname = fname.to_string_lossy();
            if let Some(name) = fname.strip_suffix(".lib")
                && !name.is_empty()
                && !libs.contains(&name.to_string())
            {
                libs.push(name.to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn nuget_provider_name() {
        assert_eq!(NugetProvider.name(), "nuget");
    }

    #[test]
    fn find_native_paths_empty() {
        let tmp = TempDir::new().unwrap();
        let (inc, lib, libs) = find_native_paths(tmp.path(), "openssl");
        assert!(inc.is_empty());
        assert!(lib.is_empty());
        assert_eq!(libs, vec!["openssl"]);
    }

    #[test]
    fn scan_lib_names_finds_libs() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("ssl.lib"), "").unwrap();
        std::fs::write(tmp.path().join("crypto.lib"), "").unwrap();
        std::fs::write(tmp.path().join("readme.txt"), "").unwrap();

        let mut libs = Vec::new();
        scan_lib_names(tmp.path(), &mut libs);
        assert!(libs.contains(&"ssl".to_string()));
        assert!(libs.contains(&"crypto".to_string()));
        assert_eq!(libs.len(), 2);
    }
}
