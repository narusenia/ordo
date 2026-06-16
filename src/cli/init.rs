use crate::util::style;
use miette::{bail, IntoDiagnostic, Result};
use std::fs;
use std::path::Path;

pub fn run() -> Result<()> {
    let manifest_path = Path::new("Ordo.toml");
    if manifest_path.exists() {
        bail!("Ordo.toml already exists in this directory");
    }

    let package_type = detect_project_type();
    let name = detect_project_name();

    fs::write(
        manifest_path,
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
type = "{package_type}"
"#
        ),
    )
    .into_diagnostic()?;

    style::success("Initialized", &format!("ordo project `{name}` ({package_type})"));

    Ok(())
}

fn detect_project_type() -> &'static str {
    let candidates = ["src/main.cpp", "src/main.c", "main.cpp", "main.c"];
    for c in &candidates {
        if Path::new(c).exists() {
            return "executable";
        }
    }
    "static-library"
}

fn detect_project_name() -> String {
    std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "unnamed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_creates_manifest() {
        let tmp = TempDir::new().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        run().unwrap();

        let manifest = fs::read_to_string("Ordo.toml").unwrap();
        assert!(manifest.contains("version = \"0.1.0\""));
    }

    #[test]
    fn init_detects_executable() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.cpp"), "int main() {}").unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        run().unwrap();

        let manifest = fs::read_to_string("Ordo.toml").unwrap();
        assert!(manifest.contains("type = \"executable\""));
    }

    #[test]
    fn init_detects_library() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/lib.cpp"), "void foo() {}").unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        run().unwrap();

        let manifest = fs::read_to_string("Ordo.toml").unwrap();
        assert!(manifest.contains("type = \"static-library\""));
    }

    #[test]
    fn init_fails_if_manifest_exists() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Ordo.toml"), "").unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let result = run();
        assert!(result.is_err());
    }
}
