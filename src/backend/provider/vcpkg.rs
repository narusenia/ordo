use super::{FetchedDep, Provider, ResolvedDep};
use crate::util::paths::OrdoPaths;
use miette::{bail, IntoDiagnostic, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct VcpkgProvider {
    runner: Box<dyn CommandRunner>,
    root_override: Option<PathBuf>,
}

impl VcpkgProvider {
    pub fn new() -> Self {
        Self {
            runner: Box::new(RealCommandRunner),
            root_override: None,
        }
    }

    #[cfg(test)]
    fn with_runner_and_root(runner: Box<dyn CommandRunner>, root: PathBuf) -> Self {
        Self {
            runner,
            root_override: Some(root),
        }
    }

    pub fn vcpkg_root(&self) -> Result<PathBuf> {
        if let Some(root) = &self.root_override {
            return Ok(root.clone());
        }

        if let Ok(root) = std::env::var("VCPKG_ROOT") {
            let root = PathBuf::from(root);
            if root.join("vcpkg").exists() || root.join("vcpkg.exe").exists() {
                return Ok(root);
            }
            bail!(
                "VCPKG_ROOT is set to '{}' but vcpkg binary not found there\n  \
                 help: run the bootstrap script inside that directory, or unset VCPKG_ROOT to let Ordo manage vcpkg",
                root.display()
            );
        }

        let managed_root = OrdoPaths::resolve().cache_dir.join("vcpkg");
        if managed_root.join("vcpkg").exists() || managed_root.join("vcpkg.exe").exists() {
            return Ok(managed_root);
        }

        self.bootstrap(&managed_root)?;
        Ok(managed_root)
    }

    fn bootstrap(&self, dest: &Path) -> Result<()> {
        std::fs::create_dir_all(dest).into_diagnostic()?;

        self.runner.run(
            "git",
            &[
                "clone",
                "--depth",
                "1",
                "https://github.com/microsoft/vcpkg.git",
                &dest.display().to_string(),
            ],
            None,
        )?;

        let script = if cfg!(windows) {
            "bootstrap-vcpkg.bat"
        } else {
            "bootstrap-vcpkg.sh"
        };

        self.runner.run(&dest.join(script).display().to_string(), &[], Some(dest))?;
        Ok(())
    }

    fn vcpkg_exe(&self, root: &Path) -> PathBuf {
        if cfg!(windows) {
            root.join("vcpkg.exe")
        } else {
            root.join("vcpkg")
        }
    }

    pub fn host_triplet() -> &'static str {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("linux", "x86_64") => "x64-linux",
            ("linux", "aarch64") => "arm64-linux",
            ("macos", "x86_64") => "x64-osx",
            ("macos", "aarch64") => "arm64-osx",
            ("windows", "x86_64") => "x64-windows",
            ("windows", "aarch64") => "arm64-windows",
            _ => "x64-linux",
        }
    }

    pub fn install_packages(
        &self,
        packages: &[(&str, Option<&str>)],
    ) -> Result<()> {
        if packages.is_empty() {
            return Ok(());
        }

        let root = self.vcpkg_root()?;
        let triplet = Self::host_triplet();

        let tmp_dir = tempfile::tempdir().into_diagnostic()?;
        let manifest = build_vcpkg_manifest_multi(packages);
        std::fs::write(tmp_dir.path().join("vcpkg.json"), &manifest).into_diagnostic()?;

        let vcpkg = self.vcpkg_exe(&root);
        let output = self.runner.run(
            &vcpkg.display().to_string(),
            &[
                "install",
                "--x-manifest-root",
                &tmp_dir.path().display().to_string(),
                &format!("--triplet={triplet}"),
                &format!("--x-install-root={}", root.join("installed").display()),
            ],
            Some(&root),
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = if stderr.trim().is_empty() { &stdout } else { &stderr };
            let names: Vec<&str> = packages.iter().map(|(n, _)| *n).collect();
            bail!(
                "vcpkg install failed for [{}]:\n{}\n  \
                 help: check vcpkg logs for details",
                names.join(", "),
                detail.trim()
            );
        }

        Ok(())
    }

    fn parse_installed(
        &self,
        root: &Path,
        name: &str,
        triplet: &str,
    ) -> Result<FetchedDep> {
        let installed = root.join("installed").join(triplet);
        let include_dir = installed.join("include");
        let lib_dir = installed.join("lib");

        let include_dirs = if include_dir.exists() {
            vec![include_dir]
        } else {
            Vec::new()
        };

        let lib_dirs = if lib_dir.exists() {
            vec![lib_dir.clone()]
        } else {
            Vec::new()
        };

        let libs = if lib_dir.exists() {
            scan_lib_names(&lib_dir)
        } else {
            Vec::new()
        };

        let frameworks = scan_pc_frameworks(&installed);

        if include_dirs.is_empty() && libs.is_empty() {
            bail!(
                "vcpkg: package '{name}' installed but no headers or libraries found at {}\n  \
                 help: the package may use a different name — check `vcpkg list`",
                installed.display()
            );
        }

        Ok(FetchedDep {
            name: name.to_string(),
            include_dirs,
            lib_dirs,
            libs,
            frameworks,
        })
    }

    pub fn query_version(&self, root: &Path, name: &str, triplet: &str) -> String {
        let vcpkg = self.vcpkg_exe(root);
        let output = self.runner.run(
            &vcpkg.display().to_string(),
            &["list", &format!("{name}:{triplet}")],
            Some(root),
        );

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with(name) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return parts[1].to_string();
                    }
                }
            }
        }

        "unknown".to_string()
    }
}

impl Provider for VcpkgProvider {
    fn name(&self) -> &str {
        "vcpkg"
    }

    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep> {
        self.install_packages(&[(name, version)])?;

        let root = self.vcpkg_root()?;
        let triplet = Self::host_triplet();
        let resolved_version = self.query_version(&root, name, triplet);

        Ok(ResolvedDep {
            name: name.to_string(),
            version: resolved_version,
            source: "vcpkg".to_string(),
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        let root = self.vcpkg_root()?;
        let triplet = Self::host_triplet();
        self.parse_installed(&root, &dep.name, triplet)
    }
}

fn build_vcpkg_manifest_multi(packages: &[(&str, Option<&str>)]) -> String {
    let deps: Vec<String> = packages
        .iter()
        .map(|(name, version)| {
            let version_constraint = version
                .map(|v| format!(",\n      \"version>=\": \"{v}\""))
                .unwrap_or_default();
            format!("    {{\n      \"name\": \"{name}\"{version_constraint}\n    }}")
        })
        .collect();

    format!(
        "{{\n  \"name\": \"ordo-deps\",\n  \"version\": \"0.0.0\",\n  \"dependencies\": [\n{}\n  ]\n}}",
        deps.join(",\n")
    )
}

fn scan_lib_names(lib_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(lib_dir) else {
        return Vec::new();
    };

    let mut names = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };

        match ext {
            "a" | "so" | "dylib" | "lib" => {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let lib_name = stem.strip_prefix("lib").unwrap_or(stem);
                    names.push(lib_name.to_string());
                }
            }
            _ => {}
        }
    }

    names.sort();
    names.dedup();
    names
}

fn scan_pc_frameworks(installed_dir: &Path) -> Vec<String> {
    let pc_dir = installed_dir.join("lib").join("pkgconfig");
    let Ok(entries) = std::fs::read_dir(&pc_dir) else {
        return Vec::new();
    };

    let mut frameworks = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("pc") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in content.lines() {
            let line = line.trim();
            if let Some(libs) = line.strip_prefix("Libs:") {
                let tokens: Vec<&str> = libs.split_whitespace().collect();
                let mut i = 0;
                while i < tokens.len() {
                    if tokens[i] == "-framework"
                        && let Some(&name) = tokens.get(i + 1)
                    {
                        frameworks.push(name.to_string());
                        i += 2;
                        continue;
                    }
                    i += 1;
                }
            }
        }
    }

    frameworks.sort();
    frameworks.dedup();
    frameworks
}

// --- Command runner abstraction for testing ---

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output>;
}

struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output().into_diagnostic()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct MockCall {
        program: String,
        args: Vec<String>,
    }

    struct MockRunner {
        calls: Arc<Mutex<Vec<MockCall>>>,
        responses: Arc<Mutex<HashMap<String, Output>>>,
    }

    impl MockRunner {
        fn new() -> Self {
            Self {
                calls: Arc::new(Mutex::new(Vec::new())),
                responses: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn on(&self, program: &str, output: Output) {
            self.responses
                .lock()
                .unwrap()
                .insert(program.to_string(), output);
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
            self.calls.lock().unwrap().push(MockCall {
                program: program.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
            });

            let responses = self.responses.lock().unwrap();
            // Match by program basename for vcpkg binary paths
            let key = Path::new(program)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(program);

            if let Some(output) = responses.get(key) {
                Ok(output.clone())
            } else if let Some(output) = responses.get(program) {
                Ok(output.clone())
            } else {
                Ok(MockRunner::success_output(""))
            }
        }
    }

    #[test]
    fn host_triplet_is_valid() {
        let triplet = VcpkgProvider::host_triplet();
        assert!(
            ["x64-linux", "arm64-linux", "x64-osx", "arm64-osx", "x64-windows", "arm64-windows"]
                .contains(&triplet)
        );
    }

    #[test]
    fn build_vcpkg_manifest_without_version() {
        let manifest = build_vcpkg_manifest_multi(&[("spdlog", None)]);
        assert!(manifest.contains("\"name\": \"spdlog\""));
        assert!(!manifest.contains("version>="));
    }

    #[test]
    fn build_vcpkg_manifest_with_version() {
        let manifest = build_vcpkg_manifest_multi(&[("spdlog", Some("1.14"))]);
        assert!(manifest.contains("\"name\": \"spdlog\""));
        assert!(manifest.contains("\"version>=\": \"1.14\""));
    }

    #[test]
    fn build_vcpkg_manifest_multi_packages() {
        let manifest = build_vcpkg_manifest_multi(&[
            ("fmt", Some("11")),
            ("raylib", None),
        ]);
        assert!(manifest.contains("\"name\": \"fmt\""));
        assert!(manifest.contains("\"name\": \"raylib\""));
        assert!(manifest.contains("\"version>=\": \"11\""));
    }

    #[test]
    fn scan_lib_names_finds_libs() {
        let tmp = tempfile::tempdir().unwrap();
        let lib_dir = tmp.path();
        std::fs::write(lib_dir.join("libfoo.a"), b"").unwrap();
        std::fs::write(lib_dir.join("libbar.so"), b"").unwrap();
        std::fs::write(lib_dir.join("baz.lib"), b"").unwrap();
        std::fs::write(lib_dir.join("README.txt"), b"").unwrap();

        let names = scan_lib_names(lib_dir);
        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"bar".to_string()));
        assert!(names.contains(&"baz".to_string()));
        assert!(!names.contains(&"README".to_string()));
    }

    #[test]
    fn scan_lib_names_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let names = scan_lib_names(tmp.path());
        assert!(names.is_empty());
    }

    #[test]
    fn scan_lib_names_nonexistent_dir() {
        let names = scan_lib_names(Path::new("/nonexistent/path"));
        assert!(names.is_empty());
    }

    #[test]
    fn resolve_calls_install_and_list() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let triplet = VcpkgProvider::host_triplet();
        let include_dir = root.join("installed").join(triplet).join("include");
        let lib_dir = root.join("installed").join(triplet).join("lib");
        std::fs::create_dir_all(&include_dir).unwrap();
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(lib_dir.join("libspdlog.a"), b"").unwrap();

        let runner = MockRunner::new();
        runner.on("vcpkg", MockRunner::success_output("spdlog:arm64-osx 1.14.1\n"));

        let provider = VcpkgProvider::with_runner_and_root(Box::new(runner), root);

        let resolved = provider.resolve("spdlog", Some("1.14")).unwrap();
        assert_eq!(resolved.name, "spdlog");
        assert_eq!(resolved.source, "vcpkg");

        let fetched = provider.fetch(&resolved).unwrap();
        assert_eq!(fetched.name, "spdlog");
        assert!(fetched.include_dirs.contains(&include_dir));
        assert!(fetched.libs.contains(&"spdlog".to_string()));
    }

    #[test]
    fn resolve_without_version() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let triplet = VcpkgProvider::host_triplet();
        let include_dir = root.join("installed").join(triplet).join("include");
        let lib_dir = root.join("installed").join(triplet).join("lib");
        std::fs::create_dir_all(&include_dir).unwrap();
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(lib_dir.join("libz.a"), b"").unwrap();

        let runner = MockRunner::new();
        runner.on("vcpkg", MockRunner::success_output("zlib:arm64-osx 1.3.1\n"));

        let provider = VcpkgProvider::with_runner_and_root(Box::new(runner), root);

        let resolved = provider.resolve("zlib", None).unwrap();
        assert_eq!(resolved.name, "zlib");
    }

    #[test]
    fn parse_installed_finds_libs() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let triplet = VcpkgProvider::host_triplet();
        let include_dir = root.join("installed").join(triplet).join("include");
        let lib_dir = root.join("installed").join(triplet).join("lib");
        std::fs::create_dir_all(&include_dir).unwrap();
        std::fs::create_dir_all(&lib_dir).unwrap();
        std::fs::write(lib_dir.join("libfoo.a"), b"").unwrap();

        let provider = VcpkgProvider::new();
        let fetched = provider.parse_installed(root, "foo", triplet).unwrap();
        assert_eq!(fetched.name, "foo");
        assert!(fetched.libs.contains(&"foo".to_string()));
        assert!(fetched.include_dirs.contains(&include_dir));
    }

    #[test]
    fn parse_installed_empty_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let triplet = VcpkgProvider::host_triplet();
        std::fs::create_dir_all(root.join("installed").join(triplet)).unwrap();

        let provider = VcpkgProvider::new();
        let err = provider.parse_installed(root, "missing", triplet).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no headers or libraries found"), "got: {msg}");
    }

    #[test]
    fn bootstrap_calls_git_clone_then_script() {
        let tmp = tempfile::tempdir().unwrap();
        let dest = tmp.path().join("vcpkg");

        let runner = MockRunner::new();
        let provider = VcpkgProvider::with_runner_and_root(Box::new(runner), dest.clone());

        let _ = provider.bootstrap(&dest);
    }

    #[test]
    fn scan_pc_frameworks_extracts_frameworks() {
        let tmp = tempfile::tempdir().unwrap();
        let installed = tmp.path();
        let pc_dir = installed.join("lib").join("pkgconfig");
        std::fs::create_dir_all(&pc_dir).unwrap();
        std::fs::write(
            pc_dir.join("glfw3.pc"),
            "Name: GLFW\nLibs: -L${libdir} -lglfw3 -framework Cocoa -framework IOKit\n",
        )
        .unwrap();

        let fws = scan_pc_frameworks(installed);
        assert_eq!(fws, vec!["Cocoa", "IOKit"]);
    }

    #[test]
    fn scan_pc_frameworks_empty_without_pc_files() {
        let tmp = tempfile::tempdir().unwrap();
        let fws = scan_pc_frameworks(tmp.path());
        assert!(fws.is_empty());
    }

    #[test]
    fn parse_installed_includes_frameworks() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let triplet = VcpkgProvider::host_triplet();
        let installed = root.join("installed").join(triplet);
        let include_dir = installed.join("include");
        let lib_dir = installed.join("lib");
        let pc_dir = lib_dir.join("pkgconfig");
        std::fs::create_dir_all(&include_dir).unwrap();
        std::fs::create_dir_all(&pc_dir).unwrap();
        std::fs::write(lib_dir.join("libglfw3.a"), b"").unwrap();
        std::fs::write(
            pc_dir.join("glfw3.pc"),
            "Name: GLFW\nLibs: -lglfw3 -framework Cocoa -framework CoreFoundation\n",
        )
        .unwrap();

        let provider = VcpkgProvider::new();
        let fetched = provider.parse_installed(root, "glfw3", triplet).unwrap();
        assert_eq!(fetched.frameworks, vec!["Cocoa", "CoreFoundation"]);
    }
}
