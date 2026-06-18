use super::context::Context;
use crate::backend::provider::conan::ConanProvider;
use crate::backend::provider::git::expand_git_shorthand;
use crate::backend::provider::pkgconfig::PkgConfigProvider;
use crate::backend::provider::system::SystemProvider;
use crate::backend::provider::vcpkg::VcpkgProvider;
use crate::backend::provider::{Provider, ResolvedDep};
use miette::{IntoDiagnostic, Result, bail};
use promptuity::prompts::{Select, SelectOption};
use promptuity::themes::MinimalTheme;
use promptuity::{Promptuity, Term};
use std::path::Path;
use toml_edit::{DocumentMut, InlineTable, Item, Value};

const PROVIDERS: &[&str] = &["vcpkg", "pkg-config", "system", "conan", "path", "git"];

struct ParsedSpec {
    provider: Option<String>,
    name: String,
    version: Option<String>,
}

fn parse_spec(spec: &str) -> ParsedSpec {
    let (provider, rest) = if let Some((p, r)) = spec.split_once(':') {
        (Some(p.to_string()), r)
    } else {
        (None, spec)
    };

    let (name, version) = if let Some((n, v)) = rest.split_once('@') {
        (n.to_string(), Some(v.to_string()))
    } else {
        (rest.to_string(), None)
    };

    ParsedSpec {
        provider,
        name,
        version,
    }
}

fn prompt_provider() -> Result<String> {
    let mut term = Term::default();
    let mut theme = MinimalTheme::default();
    let mut p = Promptuity::new(&mut term, &mut theme);

    p.begin().into_diagnostic()?;

    let provider: String = p
        .prompt(
            Select::new(
                "Provider",
                PROVIDERS
                    .iter()
                    .map(|&name| SelectOption::new(name, name.to_string()))
                    .collect(),
            )
            .as_mut(),
        )
        .into_diagnostic()?;

    p.finish().into_diagnostic()?;

    Ok(provider)
}

pub fn run(
    spec: &str,
    provider_flag: Option<&str>,
    no_verify: bool,
    with: Option<&str>,
    ctx: &Context,
) -> Result<()> {
    let dir = std::env::current_dir().into_diagnostic()?;
    let parsed = parse_spec(spec);

    let provider = provider_flag
        .map(|s| s.to_string())
        .or(parsed.provider)
        .map_or_else(prompt_provider, Ok)?;

    if provider == "git" {
        let url = expand_git_shorthand(&parsed.name);
        let dep_name = git_repo_name(&parsed.name);
        return run_inner_git(&dir, &dep_name, &url, parsed.version.as_deref(), with, ctx);
    }

    if with.is_some() {
        bail!("`--with` is only valid for git dependencies");
    }

    run_inner(
        &dir,
        &provider,
        &parsed.name,
        parsed.version.as_deref(),
        !no_verify,
        ctx,
    )
}

fn git_repo_name(spec: &str) -> String {
    spec.rsplit('/')
        .next()
        .unwrap_or(spec)
        .trim_end_matches(".git")
        .to_string()
}

fn run_inner_git(
    dir: &Path,
    name: &str,
    url: &str,
    tag: Option<&str>,
    with: Option<&str>,
    ctx: &Context,
) -> Result<()> {
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

    let mut table = InlineTable::new();
    table.insert("git", url.into());
    if let Some(t) = tag {
        table.insert("tag", t.into());
    }
    if let Some(w) = with {
        table.insert("with", w.into());
    }
    deps.insert(name, Item::Value(Value::InlineTable(table)));

    std::fs::write(&manifest_path, doc.to_string()).into_diagnostic()?;

    let tag_str = tag.map(|t| format!(" @{t}")).unwrap_or_default();
    let with_str = with.map(|w| format!(" with {w}")).unwrap_or_default();
    ctx.style
        .success("Added", &format!("{name}{tag_str} (git){with_str}"));

    Ok(())
}

fn run_inner(
    dir: &Path,
    provider: &str,
    name: &str,
    version: Option<&str>,
    resolve: bool,
    ctx: &Context,
) -> Result<()> {
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

    let resolved_version = if resolve {
        verify_resolve(provider, name, version, ctx)?
    } else {
        version.map(|v| v.to_string())
    };

    let effective_version = resolved_version.as_deref().or(version);
    let value = build_dep_value(provider, effective_version)?;
    deps.insert(name, Item::Value(value));

    std::fs::write(&manifest_path, doc.to_string()).into_diagnostic()?;

    let version_str = effective_version
        .map(|v| format!(" v{v}"))
        .unwrap_or_default();
    ctx.style
        .success("Added", &format!("{name}{version_str} ({provider})"));

    Ok(())
}

fn verify_resolve(
    provider: &str,
    name: &str,
    version: Option<&str>,
    ctx: &Context,
) -> Result<Option<String>> {
    let sw = ctx
        .style
        .create_spinner_with_detail(&format!("Resolving {name} ({provider})…"));
    let on_progress = |msg: &str| {
        sw.set_detail(msg);
    };

    let result = match provider {
        "vcpkg" => {
            let p = VcpkgProvider::new();
            use crate::backend::provider::vcpkg::VcpkgPackageSpec;
            p.install_packages(&[VcpkgPackageSpec { name, version }], &on_progress)?;
            let root = p.vcpkg_root()?;
            let triplet = VcpkgProvider::host_triplet();
            let ver = p.query_version(&root, name, triplet);
            Ok(ResolvedDep {
                name: name.to_string(),
                version: ver,
                source: "vcpkg".to_string(),
                checksum: None,
            })
        }
        "conan" => {
            let p = ConanProvider::new();
            p.resolve_with_progress(name, version, &on_progress)
        }
        "pkg-config" => PkgConfigProvider.resolve(name, version),
        "system" => SystemProvider.resolve(name, version),
        _ => {
            sw.finish_success("", "");
            return Ok(None);
        }
    };

    match result {
        Ok(dep) => {
            sw.finish_success("Resolved", &format!("{name} v{} ({provider})", dep.version));
            let v = dep.version.clone();
            Ok(if v == "system" || v == "unknown" {
                None
            } else {
                Some(v)
            })
        }
        Err(e) => {
            sw.finish_error("Failed", &format!("{name} ({provider})"));
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
            let url = version.ok_or_else(|| {
                miette::miette!("git provider requires a URL as version (e.g. raylib@https://...)")
            })?;
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
    fn parse_spec_name_only() {
        let s = parse_spec("raylib");
        assert_eq!(s.name, "raylib");
        assert!(s.provider.is_none());
        assert!(s.version.is_none());
    }

    #[test]
    fn parse_spec_name_at_version() {
        let s = parse_spec("raylib@6.0");
        assert_eq!(s.name, "raylib");
        assert_eq!(s.version.as_deref(), Some("6.0"));
        assert!(s.provider.is_none());
    }

    #[test]
    fn parse_spec_provider_colon_name() {
        let s = parse_spec("vcpkg:raylib");
        assert_eq!(s.provider.as_deref(), Some("vcpkg"));
        assert_eq!(s.name, "raylib");
        assert!(s.version.is_none());
    }

    #[test]
    fn parse_spec_provider_colon_name_at_version() {
        let s = parse_spec("vcpkg:raylib@6");
        assert_eq!(s.provider.as_deref(), Some("vcpkg"));
        assert_eq!(s.name, "raylib");
        assert_eq!(s.version.as_deref(), Some("6"));
    }

    #[test]
    fn add_pkg_config_dep() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(tmp.path(), "pkg-config", "zlib", None, false, &ctx).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("[dependencies]"));
        assert!(content.contains("zlib"));
        assert!(content.contains("pkg-config"));
    }

    #[test]
    fn add_system_dep_with_version() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(tmp.path(), "system", "m", None, false, &ctx).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("provider = \"system\""));
    }

    #[test]
    fn add_vcpkg_dep_with_version() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(tmp.path(), "vcpkg", "fmt", Some("11"), false, &ctx).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("version = \"11\""));
        assert!(content.contains("provider = \"vcpkg\""));
    }

    #[test]
    fn add_duplicate_fails() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(tmp.path(), "pkg-config", "zlib", None, false, &ctx).unwrap();
        let result = run_inner(tmp.path(), "pkg-config", "zlib", None, false, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn add_unknown_provider_fails() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        let result = run_inner(tmp.path(), "npm", "lodash", None, false, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn add_creates_dependencies_section() {
        let tmp = TempDir::new().unwrap();
        let ctx = crate::cli::context::Context::default_for_test();
        std::fs::write(
            tmp.path().join("Ordo.toml"),
            r#"[package]
name = "test"
version = "0.1.0"
type = "executable"
"#,
        )
        .unwrap();

        run_inner(tmp.path(), "system", "pthread", None, false, &ctx).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("[dependencies]"));
        assert!(content.contains("pthread"));
    }

    #[test]
    fn parse_spec_git_shorthand() {
        let s = parse_spec("git:fmtlib/fmt@11.1.0");
        assert_eq!(s.provider.as_deref(), Some("git"));
        assert_eq!(s.name, "fmtlib/fmt");
        assert_eq!(s.version.as_deref(), Some("11.1.0"));
    }

    #[test]
    fn parse_spec_git_codeberg() {
        let s = parse_spec("git:codeberg.org/nxeu/ordo");
        assert_eq!(s.provider.as_deref(), Some("git"));
        assert_eq!(s.name, "codeberg.org/nxeu/ordo");
        assert!(s.version.is_none());
    }

    #[test]
    fn git_repo_name_from_path() {
        assert_eq!(git_repo_name("fmtlib/fmt"), "fmt");
        assert_eq!(git_repo_name("codeberg.org/nxeu/ordo"), "ordo");
        assert_eq!(git_repo_name("simple"), "simple");
    }

    #[test]
    fn add_git_dep_with_tag() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner_git(
            tmp.path(),
            "fmt",
            "https://github.com/fmtlib/fmt",
            Some("11.1.0"),
            None,
            &ctx,
        )
        .unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("git = \"https://github.com/fmtlib/fmt\""));
        assert!(content.contains("tag = \"11.1.0\""));
    }

    #[test]
    fn add_git_dep_without_tag() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner_git(
            tmp.path(),
            "fmt",
            "https://github.com/fmtlib/fmt",
            None,
            None,
            &ctx,
        )
        .unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("git = \"https://github.com/fmtlib/fmt\""));
        assert!(!content.contains("tag"));
    }
}
