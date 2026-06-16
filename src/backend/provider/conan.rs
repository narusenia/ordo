use super::{FetchedDep, Provider, ResolvedDep};
use miette::{bail, IntoDiagnostic, Result};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct ConanProvider {
    runner: Box<dyn CommandRunner>,
    output_dir_override: Option<PathBuf>,
}

impl ConanProvider {
    pub fn new() -> Self {
        Self {
            runner: Box::new(RealCommandRunner),
            output_dir_override: None,
        }
    }

    #[cfg(test)]
    fn with_runner_and_output(runner: Box<dyn CommandRunner>, output_dir: PathBuf) -> Self {
        Self {
            runner,
            output_dir_override: Some(output_dir),
        }
    }

    fn detect_conan(&self) -> Result<()> {
        let output = match self.runner.run("conan", &["--version"], None) {
            Ok(o) => o,
            Err(_) => bail!(
                "conan: command not found\n  \
                 help: install Conan 2.x — https://conan.io/downloads"
            ),
        };
        if !output.status.success() {
            bail!(
                "conan not found or not working\n  \
                 help: install Conan 2.x — https://conan.io/downloads"
            );
        }
        self.ensure_default_profile()?;
        Ok(())
    }

    fn ensure_default_profile(&self) -> Result<()> {
        let output = self.runner.run(
            "conan",
            &["profile", "path", "default"],
            None,
        )?;
        if !output.status.success() {
            let detect = self.runner.run(
                "conan",
                &["profile", "detect"],
                None,
            )?;
            if !detect.status.success() {
                bail!(
                    "conan: failed to create default profile\n  \
                     help: run `conan profile detect` manually"
                );
            }
        }
        Ok(())
    }

    fn conan_output_dir(&self) -> PathBuf {
        if let Some(dir) = &self.output_dir_override {
            return dir.clone();
        }
        PathBuf::from("target").join("conan")
    }

    fn install_package(
        &self,
        name: &str,
        version: Option<&str>,
        output_dir: &Path,
    ) -> Result<()> {
        let tmp_dir = tempfile::tempdir().into_diagnostic()?;
        let conanfile = build_conanfile(name, version);
        std::fs::write(tmp_dir.path().join("conanfile.txt"), &conanfile).into_diagnostic()?;

        std::fs::create_dir_all(output_dir).into_diagnostic()?;

        let output = self.runner.run(
            "conan",
            &[
                "install",
                &tmp_dir.path().display().to_string(),
                "--output-folder",
                &output_dir.display().to_string(),
                "--generator",
                "PkgConfigDeps",
                "--build=missing",
            ],
            None,
        )?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") || stderr.contains("ERROR: Package") {
                bail!(
                    "conan: package '{name}' not found\n  \
                     help: run `conan search '{name}'` to check available packages"
                );
            }
            bail!(
                "conan install failed for '{name}':\n{stderr}\n  \
                 help: check conan output for details"
            );
        }

        Ok(())
    }

    fn parse_pc_files(&self, output_dir: &Path, name: &str) -> Result<FetchedDep> {
        let mut include_dirs = Vec::new();
        let mut lib_dirs = Vec::new();
        let mut libs = Vec::new();
        let mut frameworks = Vec::new();

        let pc_files = find_pc_files(output_dir, name);
        if pc_files.is_empty() {
            bail!(
                "conan: no .pc files found for '{name}' in {}\n  \
                 help: the package may not support PkgConfigDeps generator",
                output_dir.display()
            );
        }

        for pc_path in &pc_files {
            let content = std::fs::read_to_string(pc_path).into_diagnostic()?;
            parse_pc_content(&content, &mut include_dirs, &mut lib_dirs, &mut libs, &mut frameworks);
        }

        include_dirs.sort();
        include_dirs.dedup();
        lib_dirs.sort();
        lib_dirs.dedup();
        libs.sort();
        libs.dedup();
        frameworks.sort();
        frameworks.dedup();

        Ok(FetchedDep {
            name: name.to_string(),
            include_dirs,
            lib_dirs,
            libs,
            frameworks,
        })
    }

    fn query_version(&self, output_dir: &Path, name: &str) -> String {
        let pc_files = find_pc_files(output_dir, name);
        for pc_path in &pc_files {
            if let Ok(content) = std::fs::read_to_string(pc_path) {
                for line in content.lines() {
                    if let Some(ver) = line.strip_prefix("Version:") {
                        let ver = ver.trim();
                        if !ver.is_empty() {
                            return ver.to_string();
                        }
                    }
                }
            }
        }
        "unknown".to_string()
    }
}

impl Provider for ConanProvider {
    fn name(&self) -> &str {
        "conan"
    }

    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep> {
        self.detect_conan()?;
        let output_dir = self.conan_output_dir();
        self.install_package(name, version, &output_dir)?;

        let resolved_version = self.query_version(&output_dir, name);

        Ok(ResolvedDep {
            name: name.to_string(),
            version: resolved_version,
            source: "conan".to_string(),
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        let output_dir = self.conan_output_dir();
        self.parse_pc_files(&output_dir, &dep.name)
    }
}

fn build_conanfile(name: &str, version: Option<&str>) -> String {
    let ref_str = match version {
        Some(v) => format!("{name}/{v}"),
        None => format!("{name}/[*]"),
    };
    format!(
        "[requires]\n{ref_str}\n\n[generators]\nPkgConfigDeps\n"
    )
}

fn find_pc_files(dir: &Path, name: &str) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("pc")
            && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
            && (stem == name || stem.starts_with(&format!("{name}-")))
        {
            files.push(path);
        }
    }
    files
}

fn parse_pc_content(
    content: &str,
    include_dirs: &mut Vec<PathBuf>,
    lib_dirs: &mut Vec<PathBuf>,
    libs: &mut Vec<String>,
    frameworks: &mut Vec<String>,
) {
    for line in content.lines() {
        let line = line.trim();
        if let Some(cflags) = line.strip_prefix("Cflags:") {
            for token in cflags.split_whitespace() {
                if let Some(dir) = token.strip_prefix("-I") {
                    let dir = dir.trim_matches('"');
                    if !dir.contains("${") {
                        include_dirs.push(PathBuf::from(dir));
                    }
                }
            }
        } else if let Some(libs_line) = line.strip_prefix("Libs:") {
            let tokens: Vec<&str> = libs_line.split_whitespace().collect();
            let mut i = 0;
            while i < tokens.len() {
                let token = tokens[i];
                if let Some(dir) = token.strip_prefix("-L") {
                    let dir = dir.trim_matches('"');
                    if !dir.contains("${") {
                        lib_dirs.push(PathBuf::from(dir));
                    }
                } else if let Some(name) = token.strip_prefix("-l") {
                    libs.push(name.to_string());
                } else if token == "-framework"
                    && let Some(&fw) = tokens.get(i + 1)
                {
                    frameworks.push(fw.to_string());
                    i += 2;
                    continue;
                }
                i += 1;
            }
        }
    }
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

        fn on(&self, key: &str, output: Output) {
            self.responses
                .lock()
                .unwrap()
                .insert(key.to_string(), output);
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
            if let Some(output) = responses.get(program) {
                Ok(output.clone())
            } else {
                Ok(MockRunner::success_output(""))
            }
        }
    }

    #[test]
    fn build_conanfile_with_version() {
        let cf = build_conanfile("spdlog", Some("1.14.1"));
        assert!(cf.contains("spdlog/1.14.1"));
        assert!(cf.contains("[requires]"));
    }

    #[test]
    fn build_conanfile_without_version() {
        let cf = build_conanfile("spdlog", None);
        assert!(cf.contains("spdlog/[*]"));
    }

    #[test]
    fn find_pc_files_matches() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("spdlog.pc"), "Name: spdlog\n").unwrap();
        std::fs::write(tmp.path().join("spdlog-header.pc"), "Name: spdlog\n").unwrap();
        std::fs::write(tmp.path().join("fmt.pc"), "Name: fmt\n").unwrap();

        let files = find_pc_files(tmp.path(), "spdlog");
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn find_pc_files_no_match() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("fmt.pc"), "Name: fmt\n").unwrap();

        let files = find_pc_files(tmp.path(), "spdlog");
        assert!(files.is_empty());
    }

    #[test]
    fn parse_pc_content_extracts_flags() {
        let content = r#"
Name: spdlog
Version: 1.14.1
Cflags: -I/opt/conan/include
Libs: -L/opt/conan/lib -lspdlog
"#;
        let mut inc = Vec::new();
        let mut lib_dirs = Vec::new();
        let mut libs = Vec::new();
        let mut fws = Vec::new();
        parse_pc_content(content, &mut inc, &mut lib_dirs, &mut libs, &mut fws);

        assert_eq!(inc, vec![PathBuf::from("/opt/conan/include")]);
        assert_eq!(lib_dirs, vec![PathBuf::from("/opt/conan/lib")]);
        assert_eq!(libs, vec!["spdlog"]);
        assert!(fws.is_empty());
    }

    #[test]
    fn parse_pc_content_with_frameworks() {
        let content = "Libs: -lglfw3 -framework Cocoa -framework IOKit\n";
        let mut inc = Vec::new();
        let mut lib_dirs = Vec::new();
        let mut libs = Vec::new();
        let mut fws = Vec::new();
        parse_pc_content(content, &mut inc, &mut lib_dirs, &mut libs, &mut fws);

        assert_eq!(libs, vec!["glfw3"]);
        assert_eq!(fws, vec!["Cocoa", "IOKit"]);
    }

    #[test]
    fn parse_pc_content_skips_variable_refs() {
        let content = "Cflags: -I${prefix}/include -I/real/path\nLibs: -L${libdir} -L/real/lib\n";
        let mut inc = Vec::new();
        let mut lib_dirs = Vec::new();
        let mut libs = Vec::new();
        let mut fws = Vec::new();
        parse_pc_content(content, &mut inc, &mut lib_dirs, &mut libs, &mut fws);

        assert_eq!(inc, vec![PathBuf::from("/real/path")]);
        assert_eq!(lib_dirs, vec![PathBuf::from("/real/lib")]);
    }

    #[test]
    fn query_version_from_pc() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("spdlog.pc"),
            "Name: spdlog\nVersion: 1.14.1\n",
        )
        .unwrap();

        let provider = ConanProvider::with_runner_and_output(
            Box::new(MockRunner::new()),
            tmp.path().to_path_buf(),
        );
        assert_eq!(provider.query_version(tmp.path(), "spdlog"), "1.14.1");
    }

    #[test]
    fn resolve_installs_and_parses() {
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path().to_path_buf();

        std::fs::write(
            output_dir.join("spdlog.pc"),
            "Name: spdlog\nVersion: 1.14.1\nCflags: -I/include\nLibs: -L/lib -lspdlog\n",
        )
        .unwrap();

        let runner = MockRunner::new();
        runner.on("conan", MockRunner::success_output("Conan 2.x\n"));

        let provider = ConanProvider::with_runner_and_output(Box::new(runner), output_dir);

        let resolved = provider.resolve("spdlog", Some("1.14.1")).unwrap();
        assert_eq!(resolved.name, "spdlog");
        assert_eq!(resolved.version, "1.14.1");
        assert_eq!(resolved.source, "conan");

        let fetched = provider.fetch(&resolved).unwrap();
        assert_eq!(fetched.libs, vec!["spdlog"]);
    }

    #[test]
    fn parse_pc_files_error_when_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let provider = ConanProvider::new();
        let err = provider.parse_pc_files(tmp.path(), "missing").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("no .pc files found"), "got: {msg}");
    }
}
