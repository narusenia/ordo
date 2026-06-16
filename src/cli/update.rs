use crate::core::lockfile::LockFile;
use crate::core::manifest::Manifest;
use crate::core::resolver::resolve_dependencies;
use crate::util::style;
use miette::{bail, Result};
use std::path::Path;

pub fn run(dir: &Path, name: Option<&str>) -> Result<()> {
    let manifest_path = dir.join("Ordo.toml");
    if !manifest_path.exists() {
        bail!("Ordo.toml not found in current directory");
    }

    let lock_path = dir.join("Ordo.lock");
    let manifest = Manifest::load(&manifest_path)?;

    if manifest.dependencies.is_empty() {
        style::success("Updated", "no dependencies to update");
        return Ok(());
    }

    if let Some(target) = name
        && !manifest.dependencies.contains_key(target)
    {
        bail!("dependency '{target}' not found in [dependencies]");
    }

    let old_lock = LockFile::load(&lock_path).ok();

    let resolved = resolve_dependencies(&manifest)?;
    let new_lock = LockFile::new(&resolved);

    let changes = diff_locks(old_lock.as_ref(), &new_lock, name);

    new_lock.save(&lock_path)?;

    if changes.is_empty() {
        style::success("Updated", "already up to date");
    } else {
        for change in &changes {
            match change {
                Change::Added { name, version } => {
                    style::success("Added", &format!("{name} v{version}"));
                }
                Change::Removed { name, version } => {
                    style::warn("Removed", &format!("{name} v{version}"));
                }
                Change::Updated {
                    name,
                    old_version,
                    new_version,
                } => {
                    style::success(
                        "Updated",
                        &format!("{name} v{old_version} → v{new_version}"),
                    );
                }
            }
        }
        style::success(
            "Finished",
            &format!(
                "{} package(s) changed",
                changes.len()
            ),
        );
    }

    Ok(())
}

enum Change {
    Added { name: String, version: String },
    Removed { name: String, version: String },
    Updated {
        name: String,
        old_version: String,
        new_version: String,
    },
}

fn diff_locks(old: Option<&LockFile>, new: &LockFile, filter: Option<&str>) -> Vec<Change> {
    let mut changes = Vec::new();

    let old_map: std::collections::BTreeMap<&str, &str> = old
        .map(|l| {
            l.packages
                .iter()
                .map(|p| (p.name.as_str(), p.version.as_str()))
                .collect()
        })
        .unwrap_or_default();

    let new_map: std::collections::BTreeMap<&str, &str> = new
        .packages
        .iter()
        .map(|p| (p.name.as_str(), p.version.as_str()))
        .collect();

    for (&name, &new_ver) in &new_map {
        if let Some(f) = filter
            && name != f
        {
            continue;
        }
        match old_map.get(name) {
            Some(&old_ver) if old_ver != new_ver => {
                changes.push(Change::Updated {
                    name: name.to_string(),
                    old_version: old_ver.to_string(),
                    new_version: new_ver.to_string(),
                });
            }
            None => {
                changes.push(Change::Added {
                    name: name.to_string(),
                    version: new_ver.to_string(),
                });
            }
            _ => {}
        }
    }

    if filter.is_none() {
        for (&name, &old_ver) in &old_map {
            if !new_map.contains_key(name) {
                changes.push(Change::Removed {
                    name: name.to_string(),
                    version: old_ver.to_string(),
                });
            }
        }
    }

    changes
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
    fn update_no_deps() {
        let tmp = project_with_deps(
            r#"[package]
name = "myapp"
version = "0.1.0"
type = "executable"
"#,
        );
        run(tmp.path(), None).unwrap();
    }

    #[test]
    fn update_creates_lock_file() {
        let tmp = project_with_deps(
            r#"[package]
name = "myapp"
version = "0.1.0"
type = "executable"

[dependencies]
fmt = { version = "11", provider = "vcpkg" }
"#,
        );
        run(tmp.path(), None).unwrap();
        assert!(tmp.path().join("Ordo.lock").exists());
    }

    #[test]
    fn update_specific_dep() {
        let tmp = project_with_deps(
            r#"[package]
name = "myapp"
version = "0.1.0"
type = "executable"

[dependencies]
fmt = { version = "11", provider = "vcpkg" }
"#,
        );
        run(tmp.path(), Some("fmt")).unwrap();
    }

    #[test]
    fn update_unknown_dep_fails() {
        let tmp = project_with_deps(
            r#"[package]
name = "myapp"
version = "0.1.0"
type = "executable"

[dependencies]
fmt = { version = "11", provider = "vcpkg" }
"#,
        );
        let result = run(tmp.path(), Some("nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn diff_locks_detects_added() {
        let new = LockFile {
            version: 1,
            packages: vec![crate::core::lockfile::LockedPackage {
                name: "fmt".to_string(),
                version: "11.0.0".to_string(),
                source: "vcpkg".to_string(),
                checksum: None,
            }],
        };
        let changes = diff_locks(None, &new, None);
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], Change::Added { name, .. } if name == "fmt"));
    }
}
