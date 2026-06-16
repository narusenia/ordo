use miette::{bail, IntoDiagnostic, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(name: &str, lib: bool, no_git: bool) -> Result<()> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        bail!("directory '{}' already exists", name);
    }

    fs::create_dir_all(project_dir).into_diagnostic()?;

    if lib {
        create_library(project_dir, name)?;
    } else {
        create_executable(project_dir, name)?;
    }

    write_gitignore(project_dir)?;

    if !no_git {
        git_init(project_dir)?;
    }

    let kind = if lib { "library" } else { "executable" };
    eprintln!("Created {kind} project '{name}'");

    Ok(())
}

fn create_executable(dir: &Path, name: &str) -> Result<()> {
    write_manifest(dir, name, "executable")?;

    fs::create_dir_all(dir.join("src")).into_diagnostic()?;
    fs::write(
        dir.join("src/main.cpp"),
        r#"#include <iostream>

int main() {
    std::cout << "Hello, world!" << std::endl;
    return 0;
}
"#,
    )
    .into_diagnostic()?;

    fs::create_dir_all(dir.join("tests")).into_diagnostic()?;
    fs::write(
        dir.join("tests/main_test.cpp"),
        format!(
            r#"#include <cassert>

int main() {{
    // {name} tests
    assert(1 + 1 == 2);
    return 0;
}}
"#
        ),
    )
    .into_diagnostic()?;

    Ok(())
}

fn create_library(dir: &Path, name: &str) -> Result<()> {
    write_manifest(dir, name, "static-library")?;

    let header_dir = dir.join(format!("include/{name}"));
    fs::create_dir_all(&header_dir).into_diagnostic()?;

    let guard = name.to_uppercase().replace('-', "_");
    fs::write(
        header_dir.join(format!("{name}.hpp")),
        format!(
            r#"#ifndef {guard}_HPP
#define {guard}_HPP

namespace {name} {{

int version();

}} // namespace {name}

#endif // {guard}_HPP
"#
        ),
    )
    .into_diagnostic()?;

    fs::create_dir_all(dir.join("src")).into_diagnostic()?;
    fs::write(
        dir.join(format!("src/{name}.cpp")),
        format!(
            r#"#include "{name}/{name}.hpp"

namespace {name} {{

int version() {{
    return 1;
}}

}} // namespace {name}
"#
        ),
    )
    .into_diagnostic()?;

    fs::create_dir_all(dir.join("tests")).into_diagnostic()?;
    fs::write(
        dir.join(format!("tests/{name}_test.cpp")),
        format!(
            r#"#include <cassert>
#include "{name}/{name}.hpp"

int main() {{
    assert({name}::version() == 1);
    return 0;
}}
"#
        ),
    )
    .into_diagnostic()?;

    Ok(())
}

fn write_manifest(dir: &Path, name: &str, package_type: &str) -> Result<()> {
    fs::write(
        dir.join("Ordo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
type = "{package_type}"
"#
        ),
    )
    .into_diagnostic()
}

fn write_gitignore(dir: &Path) -> Result<()> {
    fs::write(dir.join(".gitignore"), "target/\n").into_diagnostic()
}

fn git_init(dir: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .into_diagnostic()?;

    if !status.success() {
        tracing::warn!("git init failed (exit {})", status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn in_temp<F: FnOnce(&Path)>(f: F) {
        let tmp = TempDir::new().unwrap();
        f(tmp.path());
    }

    #[test]
    fn new_executable_creates_expected_files() {
        in_temp(|tmp| {
            let name = "myapp";
            let dir = tmp.join(name);

            // run inside temp so project_dir is relative
            std::env::set_current_dir(tmp).unwrap();
            run(name, false, true).unwrap();

            assert!(dir.join("Ordo.toml").exists());
            assert!(dir.join("src/main.cpp").exists());
            assert!(dir.join("tests/main_test.cpp").exists());
            assert!(dir.join(".gitignore").exists());

            let manifest = fs::read_to_string(dir.join("Ordo.toml")).unwrap();
            assert!(manifest.contains("name = \"myapp\""));
            assert!(manifest.contains("type = \"executable\""));
        });
    }

    #[test]
    fn new_library_creates_expected_files() {
        in_temp(|tmp| {
            let name = "mylib";
            let dir = tmp.join(name);

            std::env::set_current_dir(tmp).unwrap();
            run(name, true, true).unwrap();

            assert!(dir.join("Ordo.toml").exists());
            assert!(dir.join("include/mylib/mylib.hpp").exists());
            assert!(dir.join("src/mylib.cpp").exists());
            assert!(dir.join("tests/mylib_test.cpp").exists());

            let manifest = fs::read_to_string(dir.join("Ordo.toml")).unwrap();
            assert!(manifest.contains("type = \"static-library\""));

            let header = fs::read_to_string(dir.join("include/mylib/mylib.hpp")).unwrap();
            assert!(header.contains("MYLIB_HPP"));
            assert!(header.contains("namespace mylib"));
        });
    }

    #[test]
    fn new_fails_if_directory_exists() {
        in_temp(|tmp| {
            let name = "existing";
            fs::create_dir(tmp.join(name)).unwrap();

            std::env::set_current_dir(tmp).unwrap();
            let result = run(name, false, true);
            assert!(result.is_err());
        });
    }
}
