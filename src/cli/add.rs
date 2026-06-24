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
    specs: &[String],
    provider_flag: Option<&str>,
    no_verify: bool,
    with: Option<&str>,
    alias: Option<&str>,
    link_name: Option<&[String]>,
    ctx: &Context,
) -> Result<()> {
    if specs.len() > 1 {
        if alias.is_some() {
            bail!("`--alias` can only be used with a single package spec");
        }
        if link_name.is_some() {
            bail!("`--link-name` can only be used with a single package spec");
        }
        if with.is_some() {
            bail!("`--with` can only be used with a single package spec");
        }
    }

    let dir = std::env::current_dir().into_diagnostic()?;
    let mut succeeded = 0u32;
    let mut failed: Vec<(String, String)> = Vec::new();

    for spec in specs {
        let parsed = parse_spec(spec);

        let provider = match provider_flag
            .map(|s| s.to_string())
            .or(parsed.provider.clone())
        {
            Some(p) => p,
            None => {
                if specs.len() > 1 {
                    failed.push((
                        parsed.name.clone(),
                        "no provider specified (use -P or provider:name syntax)".to_string(),
                    ));
                    continue;
                }
                prompt_provider()?
            }
        };

        let result = if provider == "git" {
            let url = expand_git_shorthand(&parsed.name);
            let dep_name = git_repo_name(&parsed.name);
            run_inner_git(&dir, &dep_name, &url, parsed.version.as_deref(), with, ctx)
        } else {
            if with.is_some() {
                failed.push((
                    parsed.name.clone(),
                    "`--with` is only valid for git dependencies".to_string(),
                ));
                continue;
            }
            run_inner(
                &dir,
                &provider,
                &parsed.name,
                parsed.version.as_deref(),
                !no_verify,
                alias,
                link_name,
                ctx,
            )
        };

        match result {
            Ok(()) => succeeded += 1,
            Err(e) => failed.push((parsed.name.clone(), format!("{e}"))),
        }
    }

    if specs.len() > 1 {
        let total = specs.len() as u32;
        if failed.is_empty() {
            ctx.style
                .success("Added", &format!("{succeeded}/{total} packages"));
        } else {
            for (name, reason) in &failed {
                ctx.style.error("Failed", &format!("{name}: {reason}"));
            }
            bail!("Added {succeeded}/{total} packages");
        }
    } else if !failed.is_empty() {
        let (_, reason) = &failed[0];
        bail!("{reason}");
    }

    Ok(())
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

#[allow(clippy::too_many_arguments)]
fn run_inner(
    dir: &Path,
    provider: &str,
    name: &str,
    version: Option<&str>,
    resolve: bool,
    alias: Option<&str>,
    link_name: Option<&[String]>,
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

    let resolve_name = alias.unwrap_or(name);
    let resolved_version = if resolve {
        verify_resolve(provider, resolve_name, version, ctx)?
    } else {
        version.map(|v| v.to_string())
    };

    let effective_version = resolved_version.as_deref().or(version);
    let value = build_dep_value(provider, effective_version, alias, link_name)?;
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

fn build_dep_value(
    provider: &str,
    version: Option<&str>,
    alias: Option<&str>,
    link_name: Option<&[String]>,
) -> Result<Value> {
    let mut table = match provider {
        "pkg-config" | "system" | "vcpkg" | "conan" => {
            let mut table = InlineTable::new();
            if let Some(v) = version {
                table.insert("version", v.into());
            }
            table.insert("provider", provider.into());
            table
        }
        "path" => {
            let path = version.unwrap_or(".");
            let mut table = InlineTable::new();
            table.insert("path", path.into());
            table
        }
        "git" => {
            let url = version.ok_or_else(|| {
                miette::miette!("git provider requires a URL as version (e.g. raylib@https://...)")
            })?;
            let mut table = InlineTable::new();
            table.insert("git", url.into());
            table
        }
        _ => bail!(
            "unknown provider '{provider}'\n  \
             valid providers: pkg-config, system, vcpkg, conan, path, git"
        ),
    };

    if let Some(a) = alias {
        table.insert("alias", a.into());
    }
    if let Some(names) = link_name {
        if names.len() == 1 {
            table.insert("link-name", names[0].as_str().into());
        } else {
            let arr: toml_edit::Array = names.iter().map(|s| s.as_str()).collect();
            table.insert("link-name", Value::Array(arr));
        }
    }

    Ok(Value::InlineTable(table))
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
        run_inner(
            tmp.path(),
            "pkg-config",
            "zlib",
            None,
            false,
            None,
            None,
            &ctx,
        )
        .unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("[dependencies]"));
        assert!(content.contains("zlib"));
        assert!(content.contains("pkg-config"));
    }

    #[test]
    fn add_system_dep_with_version() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(tmp.path(), "system", "m", None, false, None, None, &ctx).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("provider = \"system\""));
    }

    #[test]
    fn add_vcpkg_dep_with_version() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(
            tmp.path(),
            "vcpkg",
            "fmt",
            Some("11"),
            false,
            None,
            None,
            &ctx,
        )
        .unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("version = \"11\""));
        assert!(content.contains("provider = \"vcpkg\""));
    }

    #[test]
    fn add_duplicate_fails() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(
            tmp.path(),
            "pkg-config",
            "zlib",
            None,
            false,
            None,
            None,
            &ctx,
        )
        .unwrap();
        let result = run_inner(
            tmp.path(),
            "pkg-config",
            "zlib",
            None,
            false,
            None,
            None,
            &ctx,
        );
        assert!(result.is_err());
    }

    #[test]
    fn add_unknown_provider_fails() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        let result = run_inner(tmp.path(), "npm", "lodash", None, false, None, None, &ctx);
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

        run_inner(
            tmp.path(),
            "system",
            "pthread",
            None,
            false,
            None,
            None,
            &ctx,
        )
        .unwrap();

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

    #[test]
    fn add_with_alias() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        run_inner(
            tmp.path(),
            "pkg-config",
            "my-ssl",
            None,
            false,
            Some("openssl"),
            None,
            &ctx,
        )
        .unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("my-ssl"));
        assert!(content.contains("alias = \"openssl\""));
    }

    #[test]
    fn add_with_link_name_single() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        let link_names = vec!["ssl".to_string()];
        run_inner(
            tmp.path(),
            "pkg-config",
            "openssl",
            None,
            false,
            None,
            Some(&link_names),
            &ctx,
        )
        .unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("link-name = \"ssl\""));
    }

    #[test]
    fn add_with_link_name_multiple() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        let link_names = vec!["ssl".to_string(), "crypto".to_string()];
        run_inner(
            tmp.path(),
            "pkg-config",
            "openssl",
            None,
            false,
            None,
            Some(&link_names),
            &ctx,
        )
        .unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("link-name = [\"ssl\", \"crypto\"]"));
    }

    fn run_multi(tmp: &TempDir, specs: &[&str], provider: Option<&str>) -> Result<()> {
        let ctx = crate::cli::context::Context::default_for_test();
        let specs: Vec<String> = specs.iter().map(|s| s.to_string()).collect();
        std::env::set_current_dir(tmp.path()).unwrap();
        run(&specs, provider, true, None, None, None, &ctx)
    }

    #[test]
    fn add_multiple_packages() {
        let tmp = setup_project();
        run_multi(&tmp, &["zlib", "m", "pthread"], Some("system")).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("zlib"));
        assert!(content.contains("\nm = ") || content.contains("\nm="));
        assert!(content.contains("pthread"));
    }

    #[test]
    fn add_multiple_with_inline_provider() {
        let tmp = setup_project();
        run_multi(&tmp, &["system:zlib", "system:m"], None).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("zlib"));
        assert!(content.contains("\nm = ") || content.contains("\nm="));
    }

    #[test]
    fn add_multiple_rejects_alias() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        let specs = vec!["zlib".to_string(), "m".to_string()];
        std::env::set_current_dir(tmp.path()).unwrap();
        let result = run(
            &specs,
            Some("system"),
            true,
            None,
            Some("my-alias"),
            None,
            &ctx,
        );
        assert!(result.is_err());
    }

    #[test]
    fn add_multiple_rejects_link_name() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        let specs = vec!["zlib".to_string(), "m".to_string()];
        let link_names = vec!["z".to_string()];
        std::env::set_current_dir(tmp.path()).unwrap();
        let result = run(
            &specs,
            Some("system"),
            true,
            None,
            None,
            Some(&link_names),
            &ctx,
        );
        assert!(result.is_err());
    }

    #[test]
    fn add_multiple_partial_failure() {
        let tmp = setup_project();
        let ctx = crate::cli::context::Context::default_for_test();
        // First add zlib so the second call hits a duplicate
        run_inner(tmp.path(), "system", "zlib", None, false, None, None, &ctx).unwrap();

        let specs = vec!["zlib".to_string(), "m".to_string()];
        std::env::set_current_dir(tmp.path()).unwrap();
        let result = run(&specs, Some("system"), true, None, None, None, &ctx);
        // Should fail because zlib is a duplicate
        assert!(result.is_err());

        // But m should have been added successfully
        let content = std::fs::read_to_string(tmp.path().join("Ordo.toml")).unwrap();
        assert!(content.contains("\nm = ") || content.contains("\nm="));
    }

    #[test]
    fn add_multiple_no_provider_fails_gracefully() {
        let tmp = setup_project();
        // Without -P and without inline provider, multi-add should fail per-package
        run_multi(&tmp, &["zlib", "m"], None).unwrap_err();
    }
}
