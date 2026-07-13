use super::{FetchedDep, Provider, ResolvedDep};
use miette::{IntoDiagnostic, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct ClibProvider;

impl Provider for ClibProvider {
    fn name(&self) -> &str {
        "clib"
    }

    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep> {
        let status = Command::new("clib")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        if status.is_err() || !status.unwrap().success() {
            bail!(
                "clib: clib is not installed\n  \
                 help: install from https://github.com/clibs/clib"
            );
        }

        Ok(ResolvedDep {
            name: name.to_string(),
            version: version.unwrap_or("latest").to_string(),
            source: "clib".to_string(),
            checksum: None,
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        let cwd = std::env::current_dir().into_diagnostic()?;
        let deps_dir = cwd.join("deps");

        let spec = if dep.version != "latest" {
            format!("{}@{}", dep.name, dep.version)
        } else {
            dep.name.clone()
        };

        let output = Command::new("clib")
            .args(["install", &spec, "-o", "deps"])
            .output()
            .into_diagnostic()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("clib: failed to install '{}'\n  {stderr}", dep.name);
        }

        let pkg_name = dep.name.rsplit('/').next().unwrap_or(&dep.name);
        let pkg_dir = deps_dir.join(pkg_name);

        let mut include_dirs = Vec::new();
        if pkg_dir.exists() {
            include_dirs.push(pkg_dir.clone());
        }
        let include_subdir = pkg_dir.join("include");
        if include_subdir.exists() {
            include_dirs.push(include_subdir);
        }

        Ok(FetchedDep {
            name: dep.name.clone(),
            include_dirs,
            lib_dirs: Vec::new(),
            libs: Vec::new(),
            frameworks: Vec::new(),
            defines: Vec::new(),
        })
    }
}

fn find_source_files(dir: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension()
                && (ext == "c" || ext == "cpp" || ext == "cc")
            {
                sources.push(path);
            }
        }
    }
    sources
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn find_source_files_basic() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("foo.c"), "").unwrap();
        std::fs::write(tmp.path().join("bar.cpp"), "").unwrap();
        std::fs::write(tmp.path().join("baz.h"), "").unwrap();

        let sources = find_source_files(tmp.path());
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn clib_provider_name() {
        assert_eq!(ClibProvider.name(), "clib");
    }
}
