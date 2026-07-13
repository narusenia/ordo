use miette::{IntoDiagnostic, Result, bail};
use std::fs;
use std::path::Path;

use super::context::Context;

pub fn run(dir: &Path, ctx: &Context) -> Result<()> {
    let manifest_path = dir.join("Ordo.toml");
    if manifest_path.exists() {
        bail!("Ordo.toml already exists in this directory");
    }

    let package_type = detect_project_type(dir);
    let name = detect_project_name(dir);

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

    ctx.style
        .success("Created", &format!("Ordo.toml ({package_type} `{name}`)"));

    Ok(())
}

fn detect_project_type(dir: &Path) -> &'static str {
    let src = dir.join("src");
    if src.join("main.cpp").exists() || src.join("main.c").exists() || src.join("main.cc").exists()
    {
        "executable"
    } else {
        "static-library"
    }
}

fn detect_project_name(dir: &Path) -> String {
    dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use tempfile::TempDir;

    #[test]
    fn init_creates_manifest() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();

        run(tmp.path(), &ctx).unwrap();

        let manifest = fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(manifest.contains("version = \"0.1.0\""));
    }

    #[test]
    fn init_detects_executable() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/main.cpp"), "int main() {}").unwrap();

        run(tmp.path(), &ctx).unwrap();

        let manifest = fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(manifest.contains("type = \"executable\""));
    }

    #[test]
    fn init_detects_library() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        fs::write(tmp.path().join("src/lib.cpp"), "void foo() {}").unwrap();

        run(tmp.path(), &ctx).unwrap();

        let manifest = fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(manifest.contains("type = \"static-library\""));
    }

    #[test]
    fn init_fails_if_manifest_exists() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        fs::write(tmp.path().join("Ordo.toml"), "").unwrap();

        let result = run(tmp.path(), &ctx);
        assert!(result.is_err());
    }
}
