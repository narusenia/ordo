#![allow(dead_code)]

use std::path::{Path, PathBuf};

pub struct SandboxScope {
    pub src_dir: PathBuf,
    pub out_dir: PathBuf,
}

impl SandboxScope {
    pub fn new(src_dir: PathBuf, out_dir: PathBuf) -> Self {
        Self { src_dir, out_dir }
    }

    pub fn validate_path(&self, path: &Path) -> Result<(), String> {
        let canonical = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.src_dir.join(path)
        };

        let normalized = normalize_path(&canonical);

        let src_norm = normalize_path(&self.src_dir);
        let out_norm = normalize_path(&self.out_dir);

        if normalized.starts_with(&src_norm) || normalized.starts_with(&out_norm) {
            Ok(())
        } else {
            Err(format!(
                "path '{}' is outside allowed directories (src: {}, out: {})",
                path.display(),
                self.src_dir.display(),
                self.out_dir.display()
            ))
        }
    }

    pub fn resolve_path(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.src_dir.join(p)
        }
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            other => result.push(other),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn valid_paths_within_src() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out).unwrap();

        let scope = SandboxScope::new(src.clone(), out.clone());
        assert!(scope.validate_path(&src.join("file.txt")).is_ok());
        assert!(scope.validate_path(&out.join("lib")).is_ok());
    }

    #[test]
    fn relative_path_resolved_to_src() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out).unwrap();

        let scope = SandboxScope::new(src.clone(), out);
        assert!(scope.validate_path(Path::new("subdir/file.txt")).is_ok());
    }

    #[test]
    fn path_traversal_rejected() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out).unwrap();

        let scope = SandboxScope::new(src, out);
        assert!(
            scope
                .validate_path(Path::new("../../../etc/passwd"))
                .is_err()
        );
    }

    #[test]
    fn outside_path_rejected() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out).unwrap();

        let scope = SandboxScope::new(src, out);
        assert!(scope.validate_path(Path::new("/tmp/evil")).is_err());
    }
}
