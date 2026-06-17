use super::{CommandRunner, FetchedDep, Provider, RealCommandRunner, ResolvedDep};
use crate::util::paths::OrdoPaths;
use miette::{IntoDiagnostic, Result, bail};
use std::path::{Path, PathBuf};

pub struct GitProvider {
    runner: Box<dyn CommandRunner>,
    cache_dir_override: Option<PathBuf>,
}

impl GitProvider {
    pub fn new() -> Self {
        Self {
            runner: Box::new(RealCommandRunner),
            cache_dir_override: None,
        }
    }

    #[cfg(test)]
    fn with_runner_and_cache(runner: Box<dyn CommandRunner>, cache_dir: PathBuf) -> Self {
        Self {
            runner,
            cache_dir_override: Some(cache_dir),
        }
    }

    fn git_cache_dir(&self) -> PathBuf {
        if let Some(dir) = &self.cache_dir_override {
            return dir.clone();
        }
        OrdoPaths::resolve().git_cache()
    }

    fn repo_cache_path(&self, url: &str) -> PathBuf {
        let slug = url_to_slug(url);
        self.git_cache_dir().join(slug)
    }

    fn clone_or_fetch(&self, url: &str, on_progress: &dyn Fn(&str)) -> Result<PathBuf> {
        let cache_path = self.repo_cache_path(url);

        if cache_path.join(".git").exists() || cache_path.join("HEAD").exists() {
            on_progress("Fetching updates…");
            self.runner.run_streaming(
                "git",
                &[
                    "-C",
                    &cache_path.display().to_string(),
                    "fetch",
                    "--all",
                    "--prune",
                    "--progress",
                ],
                None,
                on_progress,
            )?;
            return Ok(cache_path);
        }

        std::fs::create_dir_all(cache_path.parent().unwrap_or(Path::new("."))).into_diagnostic()?;

        on_progress(&format!("Cloning {url}…"));
        let output = self.runner.run_streaming(
            "git",
            &[
                "clone",
                "--bare",
                "--progress",
                url,
                &cache_path.display().to_string(),
            ],
            None,
            on_progress,
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "git: failed to clone '{url}':\n{stderr}\n  \
                 help: check the URL and your network connection"
            );
        }

        Ok(cache_path)
    }

    fn checkout_to_worktree(&self, bare_path: &Path, git_ref: &str, dest: &Path) -> Result<()> {
        if dest.exists() {
            std::fs::remove_dir_all(dest).into_diagnostic()?;
        }
        std::fs::create_dir_all(dest).into_diagnostic()?;

        let output = self.runner.run(
            "git",
            &[
                "clone",
                "--shared",
                "--branch",
                git_ref,
                "--depth",
                "1",
                &bare_path.display().to_string(),
                &dest.display().to_string(),
            ],
            None,
        )?;

        if !output.status.success() {
            let output_rev = self.runner.run(
                "git",
                &[
                    "--git-dir",
                    &bare_path.display().to_string(),
                    "rev-parse",
                    "--verify",
                    git_ref,
                ],
                None,
            )?;

            if !output_rev.status.success() {
                bail!(
                    "git: ref '{git_ref}' not found in repository\n  \
                     help: check available tags/branches"
                );
            }

            if dest.exists() {
                std::fs::remove_dir_all(dest).into_diagnostic()?;
            }
            std::fs::create_dir_all(dest).into_diagnostic()?;

            self.runner.run(
                "git",
                &[
                    "clone",
                    "--shared",
                    &bare_path.display().to_string(),
                    &dest.display().to_string(),
                ],
                None,
            )?;

            self.runner.run(
                "git",
                &["-C", &dest.display().to_string(), "checkout", git_ref],
                None,
            )?;
        }

        Ok(())
    }

    fn resolve_commit_hash(&self, bare_path: &Path, git_ref: &str) -> Result<String> {
        let output = self.runner.run(
            "git",
            &[
                "--git-dir",
                &bare_path.display().to_string(),
                "rev-parse",
                git_ref,
            ],
            None,
        )?;

        let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if hash.is_empty() {
            bail!("git: could not resolve ref '{git_ref}'");
        }
        Ok(hash)
    }

    fn scan_checkout(&self, checkout_dir: &Path, name: &str) -> Result<FetchedDep> {
        let include_dir = checkout_dir.join("include");

        let include_dirs = if include_dir.exists() {
            vec![include_dir]
        } else {
            vec![checkout_dir.to_path_buf()]
        };

        Ok(FetchedDep {
            name: name.to_string(),
            include_dirs,
            lib_dirs: Vec::new(),
            libs: Vec::new(),
            frameworks: Vec::new(),
        })
    }
}

pub struct GitDepSpec {
    pub url: String,
    pub git_ref: String,
}

impl GitDepSpec {
    pub fn from_dep(
        git_url: &str,
        tag: Option<&str>,
        branch: Option<&str>,
        rev: Option<&str>,
    ) -> Self {
        let git_ref = tag.or(branch).or(rev).unwrap_or("HEAD").to_string();
        Self {
            url: git_url.to_string(),
            git_ref,
        }
    }
}

impl Provider for GitProvider {
    fn name(&self) -> &str {
        "git"
    }

    fn resolve(&self, _name: &str, _version: Option<&str>) -> Result<ResolvedDep> {
        bail!("git provider requires resolve_git() — use resolve_git directly");
    }

    fn fetch(&self, _dep: &ResolvedDep) -> Result<FetchedDep> {
        bail!("git provider requires fetch_git() — use fetch_git directly");
    }
}

impl GitProvider {
    pub fn resolve_git(
        &self,
        name: &str,
        spec: &GitDepSpec,
        on_progress: &dyn Fn(&str),
    ) -> Result<ResolvedDep> {
        let bare_path = self.clone_or_fetch(&spec.url, on_progress)?;
        let commit = self.resolve_commit_hash(&bare_path, &spec.git_ref)?;

        let short_hash = if commit.len() >= 7 {
            &commit[..7]
        } else {
            &commit
        };

        Ok(ResolvedDep {
            name: name.to_string(),
            version: format!("{}#{short_hash}", spec.git_ref),
            source: format!("git+{}", spec.url),
        })
    }

    pub fn fetch_git(
        &self,
        name: &str,
        spec: &GitDepSpec,
        on_progress: &dyn Fn(&str),
    ) -> Result<FetchedDep> {
        let bare_path = self.clone_or_fetch(&spec.url, on_progress)?;

        let checkout_dir = self
            .git_cache_dir()
            .join("checkouts")
            .join(url_to_slug(&spec.url))
            .join(&spec.git_ref);

        self.checkout_to_worktree(&bare_path, &spec.git_ref, &checkout_dir)?;
        self.scan_checkout(&checkout_dir, name)
    }
}

fn url_to_slug(url: &str) -> String {
    url.trim_end_matches('/')
        .trim_end_matches(".git")
        .replace("://", "-")
        .replace(['/', '.', ':'], "-")
}

pub fn expand_git_shorthand(spec: &str) -> String {
    if spec.contains("://") || spec.starts_with("git@") {
        return spec.to_string();
    }
    if spec.contains('.') {
        return format!("https://{spec}");
    }
    format!("https://github.com/{spec}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::process::Output;
    use std::sync::{Arc, Mutex};

    struct MockRunner {
        calls: Arc<Mutex<Vec<(String, Vec<String>)>>>,
        responses: Arc<Mutex<HashMap<String, Output>>>,
    }

    impl MockRunner {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn on_args_containing(&self, keyword: &str, output: Output) {
            self.responses
                .lock()
                .unwrap()
                .insert(keyword.to_string(), output);
        }

        fn success_output(stdout: &str) -> Output {
            Output {
                status: std::process::ExitStatus::default(),
                stdout: stdout.as_bytes().to_vec(),
                stderr: Vec::new(),
            }
        }
    }

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, args: &[&str], _cwd: Option<&Path>) -> Result<Output> {
            let args_vec: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            let args_str = args_vec.join(" ");
            self.calls
                .lock()
                .unwrap()
                .push((program.to_string(), args_vec));

            let responses = self.responses.lock().unwrap();
            for (keyword, output) in responses.iter() {
                if args_str.contains(keyword) {
                    return Ok(output.clone());
                }
            }
            Ok(MockRunner::success_output(""))
        }
    }

    #[test]
    fn url_to_slug_github() {
        assert_eq!(
            url_to_slug("https://github.com/fmtlib/fmt"),
            "https-github-com-fmtlib-fmt"
        );
    }

    #[test]
    fn url_to_slug_strips_git_suffix() {
        assert_eq!(
            url_to_slug("https://github.com/fmtlib/fmt.git"),
            "https-github-com-fmtlib-fmt"
        );
    }

    #[test]
    fn expand_shorthand_github() {
        assert_eq!(
            expand_git_shorthand("fmtlib/fmt"),
            "https://github.com/fmtlib/fmt"
        );
    }

    #[test]
    fn expand_shorthand_custom_host() {
        assert_eq!(
            expand_git_shorthand("codeberg.org/nxeu/ordo"),
            "https://codeberg.org/nxeu/ordo"
        );
    }

    #[test]
    fn expand_shorthand_full_url() {
        let url = "https://github.com/fmtlib/fmt";
        assert_eq!(expand_git_shorthand(url), url);
    }

    #[test]
    fn expand_shorthand_ssh() {
        let url = "git@github.com:fmtlib/fmt.git";
        assert_eq!(expand_git_shorthand(url), url);
    }

    #[test]
    fn git_dep_spec_tag() {
        let spec =
            GitDepSpec::from_dep("https://github.com/fmtlib/fmt", Some("11.1.0"), None, None);
        assert_eq!(spec.git_ref, "11.1.0");
    }

    #[test]
    fn git_dep_spec_branch() {
        let spec = GitDepSpec::from_dep("https://github.com/fmtlib/fmt", None, Some("main"), None);
        assert_eq!(spec.git_ref, "main");
    }

    #[test]
    fn git_dep_spec_defaults_to_head() {
        let spec = GitDepSpec::from_dep("https://github.com/fmtlib/fmt", None, None, None);
        assert_eq!(spec.git_ref, "HEAD");
    }

    #[test]
    fn resolve_git_clones_and_resolves() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().to_path_buf();

        let bare_dir = cache_dir.join("https-github-com-fmtlib-fmt");
        std::fs::create_dir_all(bare_dir.join(".git")).unwrap();

        let runner = MockRunner::new();
        runner.on_args_containing("rev-parse", MockRunner::success_output("abc1234def5678\n"));
        runner.on_args_containing("fetch", MockRunner::success_output(""));

        let provider = GitProvider::with_runner_and_cache(Box::new(runner), cache_dir);
        let spec =
            GitDepSpec::from_dep("https://github.com/fmtlib/fmt", Some("11.1.0"), None, None);

        let resolved = provider.resolve_git("fmt", &spec, &|_| {}).unwrap();
        assert_eq!(resolved.name, "fmt");
        assert!(resolved.version.contains("11.1.0#abc1234"));
        assert!(resolved.source.starts_with("git+"));
    }

    #[test]
    fn scan_checkout_with_include_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        std::fs::create_dir_all(dir.join("include")).unwrap();

        let provider = GitProvider::new();
        let fetched = provider.scan_checkout(dir, "fmt").unwrap();
        assert_eq!(fetched.name, "fmt");
        assert_eq!(fetched.include_dirs, vec![dir.join("include")]);
    }

    #[test]
    fn scan_checkout_fallback_to_root() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let provider = GitProvider::new();
        let fetched = provider.scan_checkout(dir, "fmt").unwrap();
        assert_eq!(fetched.include_dirs, vec![dir.to_path_buf()]);
    }
}
