//! Arsenal — toolchain manager for Ordo. **Beta**: manages external tool binaries
//! (Ninja, clang-format).

mod platform;
mod registry;

use miette::{IntoDiagnostic, Result, bail};
use ordo_core::paths::OrdoPaths;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use registry::resolve_release;

/// Supported tools that Arsenal can manage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Ninja,
    ClangFormat,
}

impl Tool {
    /// Every tool Arsenal knows about.
    pub const ALL: [Tool; 2] = [Tool::Ninja, Tool::ClangFormat];

    /// Parse a tool name string into a Tool variant.
    pub fn parse(name: &str) -> Option<Tool> {
        match name.to_lowercase().as_str() {
            "ninja" => Some(Tool::Ninja),
            "clang-format" | "clang_format" | "clangformat" => Some(Tool::ClangFormat),
            _ => None,
        }
    }

    /// Display name for the tool.
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Ninja => "ninja",
            Tool::ClangFormat => "clang-format",
        }
    }

    /// Comma-separated list of tool names, for error messages.
    pub fn supported_names() -> String {
        Tool::ALL
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// An installed tool with its version and path to the binary.
#[derive(Debug, Clone)]
pub struct InstalledTool {
    pub tool: Tool,
    pub version: String,
    pub path: PathBuf,
}

/// Arsenal manages external tool binaries (download, install, resolve).
pub struct Arsenal {
    toolchains_dir: PathBuf,
}

impl Default for Arsenal {
    fn default() -> Self {
        Self::new()
    }
}

impl Arsenal {
    /// Create a new Arsenal instance using standard OrdoPaths.
    pub fn new() -> Self {
        let paths = OrdoPaths::resolve();
        Self {
            toolchains_dir: paths.toolchains_dir(),
        }
    }

    /// Create an Arsenal instance with a custom toolchains directory (for testing).
    #[cfg(test)]
    pub fn with_dir(toolchains_dir: PathBuf) -> Self {
        Self { toolchains_dir }
    }

    /// Install a tool. If version is None, installs the latest release.
    /// Calls `on_progress` with status messages during the process.
    pub fn install(
        &self,
        tool: Tool,
        version: Option<&str>,
        on_progress: &dyn Fn(&str),
    ) -> Result<InstalledTool> {
        match version {
            Some(v) => on_progress(&format!("Fetching {} v{} ...", tool.name(), v)),
            None => on_progress(&format!("Fetching latest release for {}...", tool.name())),
        }
        let release = resolve_release(tool, version)?;

        let version = release.version;
        let install_dir = self.tool_version_dir(tool, &version);

        // Already installed?
        let bin_path = install_dir.join(binary_name(tool));
        if bin_path.exists() {
            on_progress(&format!("{} v{} already installed", tool.name(), version));
            return Ok(InstalledTool {
                tool,
                version,
                path: bin_path,
            });
        }

        // Download
        on_progress(&format!("Downloading {} v{}...", tool.name(), version));
        let zip_bytes = download_asset(&release.url)?;

        if let Some(expected) = &release.sha256 {
            verify_sha256(&zip_bytes, expected)?;
        }

        // Extract
        on_progress(&format!("Extracting to {}...", install_dir.display()));
        fs::create_dir_all(&install_dir).into_diagnostic()?;
        extract_zip(&zip_bytes, &install_dir, tool)?;

        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o755);
            fs::set_permissions(&bin_path, perms).into_diagnostic()?;
        }

        if !bin_path.exists() {
            bail!(
                "extraction succeeded but binary not found at {}",
                bin_path.display()
            );
        }

        on_progress(&format!(
            "Installed {} v{} to {}",
            tool.name(),
            version,
            bin_path.display()
        ));

        Ok(InstalledTool {
            tool,
            version,
            path: bin_path,
        })
    }

    /// List all installed tools.
    pub fn list(&self) -> Vec<InstalledTool> {
        let mut installed = Vec::new();

        // For each known tool
        for tool in Tool::ALL {
            let tool_dir = self.toolchains_dir.join(tool.name());
            if !tool_dir.exists() {
                continue;
            }
            let Ok(entries) = fs::read_dir(&tool_dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let version = entry.file_name().to_string_lossy().to_string();
                let bin_path = path.join(binary_name(tool));
                if bin_path.exists() {
                    installed.push(InstalledTool {
                        tool,
                        version,
                        path: bin_path,
                    });
                }
            }
        }

        installed.sort_by(|a, b| a.version.cmp(&b.version));
        installed
    }

    /// Find the path to a specific tool, optionally matching a version requirement.
    /// Uses simple prefix matching: "1.12" matches "1.12.1", "1.12.0", etc.
    /// If no version specified, returns the latest installed version.
    pub fn which(&self, tool: Tool, version_req: Option<&str>) -> Option<PathBuf> {
        let installed = self.list();
        let matching: Vec<&InstalledTool> = installed
            .iter()
            .filter(|t| t.tool == tool)
            .filter(|t| match version_req {
                Some(req) => version_matches(&t.version, req),
                None => true,
            })
            .collect();

        // Return the latest matching version
        matching.last().map(|t| t.path.clone())
    }

    /// Remove a specific version of a tool.
    pub fn remove(&self, tool: Tool, version: &str) -> Result<()> {
        let dir = self.tool_version_dir(tool, version);
        if !dir.exists() {
            bail!("{} v{} is not installed", tool.name(), version);
        }
        fs::remove_dir_all(&dir).into_diagnostic()?;

        // Clean up empty parent directory
        let tool_dir = self.toolchains_dir.join(tool.name());
        if tool_dir.exists()
            && let Ok(entries) = fs::read_dir(&tool_dir)
            && entries.count() == 0
        {
            let _ = fs::remove_dir(&tool_dir);
        }

        Ok(())
    }

    /// Remove all installed tools. Returns the number of removed items.
    pub fn clean(&self) -> Result<usize> {
        let installed = self.list();
        let count = installed.len();

        if count == 0 {
            return Ok(0);
        }

        if self.toolchains_dir.exists() {
            fs::remove_dir_all(&self.toolchains_dir).into_diagnostic()?;
        }

        Ok(count)
    }

    /// Update a tool to the latest version.
    pub fn update(&self, tool: Tool, on_progress: &dyn Fn(&str)) -> Result<InstalledTool> {
        self.install(tool, None, on_progress)
    }

    /// Query the latest remote version for a tool.
    pub fn latest_remote_version(&self, tool: Tool) -> Result<String> {
        Ok(resolve_release(tool, None)?.version)
    }

    fn tool_version_dir(&self, tool: Tool, version: &str) -> PathBuf {
        self.toolchains_dir.join(tool.name()).join(version)
    }
}

/// Find installed binary matching requirement, or None.
/// Checks Arsenal first, then falls back to system PATH.
pub fn resolve_tool_path(tool: Tool, version_req: Option<&str>) -> Option<PathBuf> {
    let arsenal = Arsenal::new();
    if let Some(path) = arsenal.which(tool, version_req) {
        return Some(path);
    }

    // Fall back to system PATH. A pinned version has to hold there too —
    // silently using whatever is on PATH would make the pin meaningless.
    // With a pin, keep walking PATH: an older copy earlier in PATH should not
    // hide a matching one further along.
    let candidates = which_all_in_path(binary_name(tool));
    match version_req {
        None => candidates.into_iter().next(),
        Some(req) => candidates
            .into_iter()
            .find(|path| version_of(path).is_some_and(|v| version_matches(&v, req))),
    }
}

/// Ask a tool binary for its version by running `--version`.
/// Returns the first dotted-numeric token, which covers both bare output
/// ("1.13.2") and prefixed output ("clang-format version 23.1.1").
pub fn version_of(binary: &Path) -> Option<String> {
    let output = Command::new(binary).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    parse_version(&text)
}

fn parse_version(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|token| token.starts_with(|c: char| c.is_ascii_digit()))
        .map(|token| {
            token
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect::<String>()
        })
        .map(|v| v.trim_end_matches('.').to_string())
        .filter(|v| !v.is_empty())
}

fn which_all_in_path(name: &str) -> Vec<PathBuf> {
    which_all_in(name, &std::env::var("PATH").unwrap_or_default())
}

/// Every existing `name` across the directories of a PATH string, in order.
fn which_all_in(name: &str, path_var: &str) -> Vec<PathBuf> {
    let separator = if cfg!(windows) { ';' } else { ':' };

    path_var
        .split(separator)
        .map(|dir| Path::new(dir).join(name))
        .filter(|candidate| candidate.exists())
        .collect()
}

/// Simple prefix-based version matching.
/// "1.12" matches "1.12.1", "1.12.0", "1.12", etc.
/// Exact matches also work: "1.12.1" matches "1.12.1".
pub(crate) fn version_matches(installed: &str, requirement: &str) -> bool {
    if installed == requirement {
        return true;
    }
    // Prefix match with dot boundary
    installed.starts_with(requirement)
        && installed
            .as_bytes()
            .get(requirement.len())
            .is_none_or(|&b| b == b'.')
}

fn binary_name(tool: Tool) -> &'static str {
    match (tool, cfg!(windows)) {
        (Tool::Ninja, true) => "ninja.exe",
        (Tool::Ninja, false) => "ninja",
        (Tool::ClangFormat, true) => "clang-format.exe",
        (Tool::ClangFormat, false) => "clang-format",
    }
}

/// Verify a download against the digest published by the registry.
fn verify_sha256(bytes: &[u8], expected: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    if !actual.eq_ignore_ascii_case(expected) {
        bail!("download failed integrity check: expected sha256 {expected}, got {actual}");
    }

    Ok(())
}

/// Download an asset from a URL. Returns the raw bytes.
fn download_asset(url: &str) -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("ordo/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into_diagnostic()?;

    let response = client.get(url).send().into_diagnostic()?;

    if response.status() == reqwest::StatusCode::FORBIDDEN {
        bail!(
            "GitHub API rate limit exceeded. Try again later or set GITHUB_TOKEN environment variable."
        );
    }

    if !response.status().is_success() {
        bail!("failed to download {}: HTTP {}", url, response.status());
    }

    response.bytes().into_diagnostic().map(|b| b.to_vec())
}

/// Extract a ZIP archive to a destination directory.
/// For ninja, the ZIP contains just the binary at the root level.
fn extract_zip(zip_bytes: &[u8], dest_dir: &Path, tool: Tool) -> Result<()> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).into_diagnostic()?;

    let target_name = binary_name(tool);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).into_diagnostic()?;
        let name = file.name().to_string();

        // For ninja, we only extract the binary itself
        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&name);

        if file_name == target_name {
            let out_path = dest_dir.join(target_name);
            let mut out_file = fs::File::create(&out_path).into_diagnostic()?;
            std::io::copy(&mut file, &mut out_file).into_diagnostic()?;
            return Ok(());
        }
    }

    // If exact match not found, extract all files (fallback)
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor).into_diagnostic()?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).into_diagnostic()?;
        let name = file.name().to_string();
        if name.ends_with('/') {
            continue;
        }
        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&name);
        let out_path = dest_dir.join(file_name);
        let mut out_file = fs::File::create(&out_path).into_diagnostic()?;
        std::io::copy(&mut file, &mut out_file).into_diagnostic()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_parse_ninja() {
        assert_eq!(Tool::parse("ninja"), Some(Tool::Ninja));
        assert_eq!(Tool::parse("Ninja"), Some(Tool::Ninja));
        assert_eq!(Tool::parse("NINJA"), Some(Tool::Ninja));
    }

    #[test]
    fn tool_parse_unknown() {
        assert_eq!(Tool::parse("cmake"), None);
        assert_eq!(Tool::parse(""), None);
        assert_eq!(Tool::parse("unknown"), None);
    }

    #[test]
    fn tool_name() {
        assert_eq!(Tool::Ninja.name(), "ninja");
    }

    #[test]
    fn version_matches_exact() {
        assert!(version_matches("1.12.1", "1.12.1"));
        assert!(version_matches("1.12.0", "1.12.0"));
    }

    #[test]
    fn version_matches_prefix() {
        assert!(version_matches("1.12.1", "1.12"));
        assert!(version_matches("1.12.0", "1.12"));
        assert!(version_matches("1.12.1", "1"));
    }

    #[test]
    fn version_matches_no_false_positives() {
        // "1.1" should NOT match "1.12" (dot boundary check)
        assert!(!version_matches("1.12.1", "1.1"));
        assert!(!version_matches("2.0.0", "1"));
        assert!(!version_matches("1.12.1", "1.13"));
    }

    #[test]
    fn version_matches_full_version_as_prefix() {
        assert!(version_matches("1.12", "1.12"));
    }

    #[test]
    fn parse_version_bare_output() {
        assert_eq!(parse_version("1.13.2\n").as_deref(), Some("1.13.2"));
    }

    #[test]
    fn parse_version_prefixed_output() {
        assert_eq!(
            parse_version("clang-format version 23.1.1\n").as_deref(),
            Some("23.1.1")
        );
    }

    #[test]
    fn parse_version_strips_trailing_junk() {
        assert_eq!(parse_version("1.12.1.git\n").as_deref(), Some("1.12.1"));
        assert_eq!(
            parse_version("clang-format version 17.0.6-rc1\n").as_deref(),
            Some("17.0.6")
        );
    }

    #[test]
    fn parse_version_without_number() {
        assert_eq!(parse_version("unknown build\n"), None);
    }

    #[test]
    fn verify_sha256_accepts_matching_digest() {
        // sha256 of "ordo"
        let expected = "b6e1b1a4e5b0d3b8b9a0b2e6cbf1b0b6b58ba2a06ba5a3b6a1e4b7a07e2a0e1f";
        assert!(verify_sha256(b"ordo", expected).is_err());

        use sha2::{Digest, Sha256};
        let actual: String = Sha256::digest(b"ordo")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify_sha256(b"ordo", &actual).is_ok());
        assert!(verify_sha256(b"ordo", &actual.to_uppercase()).is_ok());
    }

    #[test]
    fn which_all_in_returns_every_match_in_order() {
        let tmp = tempfile::TempDir::new().unwrap();
        let first = tmp.path().join("first");
        let second = tmp.path().join("second");
        let empty = tmp.path().join("empty");
        for dir in [&first, &second, &empty] {
            fs::create_dir_all(dir).unwrap();
        }
        fs::write(first.join("tool"), b"fake").unwrap();
        fs::write(second.join("tool"), b"fake").unwrap();

        let separator = if cfg!(windows) { ";" } else { ":" };
        let path_var = [&empty, &first, &second]
            .map(|d| d.display().to_string())
            .join(separator);

        let found = which_all_in("tool", &path_var);
        assert_eq!(found, vec![first.join("tool"), second.join("tool")]);
    }

    #[test]
    fn tool_parse_clang_format() {
        assert_eq!(Tool::parse("clang-format"), Some(Tool::ClangFormat));
        assert_eq!(Tool::parse("clang_format"), Some(Tool::ClangFormat));
        assert_eq!(Tool::parse("Clang-Format"), Some(Tool::ClangFormat));
    }

    #[test]
    fn binary_name_clang_format() {
        let name = binary_name(Tool::ClangFormat);
        if cfg!(windows) {
            assert_eq!(name, "clang-format.exe");
        } else {
            assert_eq!(name, "clang-format");
        }
    }

    #[test]
    fn binary_name_ninja() {
        let name = binary_name(Tool::Ninja);
        if cfg!(windows) {
            assert_eq!(name, "ninja.exe");
        } else {
            assert_eq!(name, "ninja");
        }
    }

    #[test]
    fn list_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        let list = arsenal.list();
        assert!(list.is_empty());
    }

    #[test]
    fn which_returns_none_when_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        assert!(arsenal.which(Tool::Ninja, None).is_none());
        assert!(arsenal.which(Tool::Ninja, Some("1.12")).is_none());
    }

    #[test]
    fn list_and_which_find_installed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ninja_dir = tmp.path().join("ninja").join("1.12.1");
        fs::create_dir_all(&ninja_dir).unwrap();
        let bin_name = binary_name(Tool::Ninja);
        fs::write(ninja_dir.join(bin_name), b"fake-ninja").unwrap();

        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        let list = arsenal.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].tool, Tool::Ninja);
        assert_eq!(list[0].version, "1.12.1");

        // which with prefix match
        assert!(arsenal.which(Tool::Ninja, Some("1.12")).is_some());
        assert!(arsenal.which(Tool::Ninja, Some("1.12.1")).is_some());
        assert!(arsenal.which(Tool::Ninja, None).is_some());
        assert!(arsenal.which(Tool::Ninja, Some("1.13")).is_none());
    }

    #[test]
    fn remove_installed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ninja_dir = tmp.path().join("ninja").join("1.12.1");
        fs::create_dir_all(&ninja_dir).unwrap();
        let bin_name = binary_name(Tool::Ninja);
        fs::write(ninja_dir.join(bin_name), b"fake-ninja").unwrap();

        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        assert_eq!(arsenal.list().len(), 1);

        arsenal.remove(Tool::Ninja, "1.12.1").unwrap();
        assert_eq!(arsenal.list().len(), 0);
    }

    #[test]
    fn remove_not_installed() {
        let tmp = tempfile::TempDir::new().unwrap();
        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        let result = arsenal.remove(Tool::Ninja, "9.9.9");
        assert!(result.is_err());
    }

    #[test]
    fn clean_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        let count = arsenal.clean().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn clean_removes_all() {
        let tmp = tempfile::TempDir::new().unwrap();
        let bin_name = binary_name(Tool::Ninja);

        let dir1 = tmp.path().join("ninja").join("1.12.0");
        fs::create_dir_all(&dir1).unwrap();
        fs::write(dir1.join(bin_name), b"fake").unwrap();

        let dir2 = tmp.path().join("ninja").join("1.12.1");
        fs::create_dir_all(&dir2).unwrap();
        fs::write(dir2.join(bin_name), b"fake").unwrap();

        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        assert_eq!(arsenal.list().len(), 2);

        let count = arsenal.clean().unwrap();
        assert_eq!(count, 2);
        assert_eq!(arsenal.list().len(), 0);
    }

    #[test]
    fn list_reports_every_known_tool() {
        let tmp = tempfile::TempDir::new().unwrap();
        for tool in Tool::ALL {
            let dir = tmp.path().join(tool.name()).join("1.0.0");
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join(binary_name(tool)), b"fake").unwrap();
        }

        let arsenal = Arsenal::with_dir(tmp.path().to_path_buf());
        assert_eq!(arsenal.list().len(), Tool::ALL.len());
        for tool in Tool::ALL {
            assert!(arsenal.which(tool, None).is_some(), "{}", tool.name());
        }
    }
}
