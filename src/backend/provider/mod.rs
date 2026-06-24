#![allow(dead_code)]

pub mod brew;
pub mod conan;
pub mod git;
pub mod nix;
pub mod pacman;
pub mod pkgconfig;
pub mod system;
pub mod vcpkg;

use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedDep {
    pub name: String,
    pub include_dirs: Vec<PathBuf>,
    pub lib_dirs: Vec<PathBuf>,
    pub libs: Vec<String>,
    pub frameworks: Vec<String>,
}

pub trait Provider {
    fn name(&self) -> &str;
    fn resolve(&self, name: &str, version: Option<&str>) -> Result<ResolvedDep>;
    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep>;
}

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output>;

    fn run_streaming(
        &self,
        program: &str,
        args: &[&str],
        cwd: Option<&Path>,
        on_line: &dyn Fn(&str),
    ) -> Result<Output> {
        let _ = on_line;
        self.run(program, args, cwd)
    }
}

pub(crate) struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &str, args: &[&str], cwd: Option<&Path>) -> Result<Output> {
        let mut cmd = Command::new(program);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.output().into_diagnostic()
    }

    fn run_streaming(
        &self,
        program: &str,
        args: &[&str],
        cwd: Option<&Path>,
        on_line: &dyn Fn(&str),
    ) -> Result<Output> {
        use std::io::{BufRead, BufReader};
        use std::process::Stdio;

        let mut cmd = Command::new(program);
        cmd.args(args);
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().into_diagnostic()?;

        let stderr = child.stderr.take().unwrap();
        let stderr_handle = std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            reader.lines().map_while(Result::ok).collect::<Vec<_>>()
        });

        let stdout = child.stdout.take().unwrap();
        let mut stdout_lines = Vec::new();
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if !line.trim().is_empty() {
                on_line(line.trim());
            }
            stdout_lines.push(line);
        }

        let status = child.wait().into_diagnostic()?;
        let stderr_lines = stderr_handle.join().unwrap_or_default();
        for line in &stderr_lines {
            if !line.trim().is_empty() {
                on_line(line.trim());
            }
        }

        Ok(Output {
            status,
            stdout: stdout_lines.join("\n").into_bytes(),
            stderr: stderr_lines.join("\n").into_bytes(),
        })
    }
}
