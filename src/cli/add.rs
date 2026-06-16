use crate::backend::provider::pkgconfig::PkgConfigProvider;
use crate::backend::provider::system::SystemProvider;
use crate::backend::provider::vcpkg::VcpkgProvider;
use crate::backend::provider::Provider;
use crate::util::style;
use miette::{bail, IntoDiagnostic, Result};
use std::path::Path;
use toml_edit::{DocumentMut, InlineTable, Item, Value};

pub fn run(provider: &str, name: &str, version: Option<&str>) -> Result<()> {
    let dir = std::env::current_dir().into_diagnostic()?;
    run_inner(&dir, provider, name, version, true)
}

fn run_inner(dir: &Path, provider: &str, name: &str, version: Option<&str>, resolve: bool) -> Result<()> {
    let manifest_path = dir.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in current directory");
    }

    let content = std::fs::read_to_string(&manifest_path).into_diagnostic()?;
    let mut doc: DocumentMut = content.parse().into_diagnostic()?;

    if !doc.contains_table("dependencies") {
        doc["dependencies"] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    let deps = doc["dependencies"].as_table_mut().unwrap();

    if deps.contains_key(name) {
        bail!("dependency '{name}' already exists in [dependencies]");
    }

    if resolve {
        verify_resolve(provider, name, version)?;
    }

    let value = build_dep_value(provider, version)?;
    deps.insert(name, Item::Value(value));

    std::fs::write(&manifest_path, doc.to_string()).into_diagnostic()?;

    let version_str = version.map(|v| format!(" v{v}")).unwrap_or_default();
    style::success("Added", &format!("{name}{version_str} ({provider})"));

    Ok(())
}

fn verify_resolve(provider: &str, name: &str, version: Option<&str>) -> Result<()> {
    let p: Box<dyn Provider> = match provider {
        "pkg-config" => Box::new(PkgConfigProvider),
        "system" => Box::new(SystemProvider),
        "vcpkg" => Box::new(VcpkgProvider::new()),
        _ => return Ok(()),
    };

    let spinner = style::create_spinner(&format!("Resolving {name} ({provider})…"));

    match p.resolve(name, version) {
        Ok(dep) => {
            style::finish_spinner_success(
                &spinner,
                "Resolved",
                &format!("{name} v{} ({provider})", dep.version),
            );
            Ok(())
        }
        Err(e) => {
            style::finish_spinner_error(&spinner, "Failed", &format!("{name} ({provider})"));
            Err(e)
        }
    }
}

fn build_dep_value(provider: &str, version: Option<&str>) -> Result<Value> {
    match provider {
        "pkg-config" | "system" | "vcpkg" | "conan" => {
            let mut table = InlineTable::new();
            if let Some(v) = version {
                table.insert("version", v.into());
            }
            table.insert("provider", provider.into());
            Ok(Value::InlineTable(table))
        }
        "path" => {
            let path = version.unwrap_or(".");
            let mut table = InlineTable::new();
            table.insert("path", path.into());
            Ok(Value::InlineTable(table))
        }
        "git" => {
            let url = version.ok_or_else(|| miette::miette!("git provider requires --version <url>"))?;
            let mut table = InlineTable::new();
            table.insert("git", url.into());
            Ok(Value::InlineTable(table))
        }
        _ => bail!(
            "unknown provider '{provider}'\n  \
             valid providers: pkg-config, system, vcpkg, conan, path, git"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_project() -> TempDir {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Ordo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"
type = "executable"
"#,
        )
        .unwrap();
        tmp
    }

    #[test]
    fn add_pkg_config_dep() {
        let tmp = setup_project();
        run_inner(tmp.path(), "pkg-config", "zlib", None, false).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("[dependencies]"));
        assert!(content.contains("zlib"));
        assert!(content.contains("pkg-config"));
    }

    #[test]
    fn add_system_dep_with_version() {
        let tmp = setup_project();
        run_inner(tmp.path(), "system", "m", None, false).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("provider = \"system\""));
    }

    #[test]
    fn add_vcpkg_dep_with_version() {
        let tmp = setup_project();
        run_inner(tmp.path(), "vcpkg", "fmt", Some("11"), false).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("version = \"11\""));
        assert!(content.contains("provider = \"vcpkg\""));
    }

    #[test]
    fn add_duplicate_fails() {
        let tmp = setup_project();
        run_inner(tmp.path(), "pkg-config", "zlib", None, false).unwrap();
        let result = run_inner(tmp.path(), "pkg-config", "zlib", None, false);
        assert!(result.is_err());
    }

    #[test]
    fn add_unknown_provider_fails() {
        let tmp = setup_project();
        let result = run_inner(tmp.path(), "npm", "lodash", None, false);
        assert!(result.is_err());
    }

    #[test]
    fn add_creates_dependencies_section() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Ordo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"
type = "executable"
"#,
        )
        .unwrap();

        run_inner(tmp.path(), "system", "pthread", None, false).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("[dependencies]"));
        assert!(content.contains("pthread"));
    }
}
