use crate::Tool;
use crate::platform::{github_asset_name, wheel_tags};
use crate::version_matches;
use miette::{IntoDiagnostic, Result, bail};
use serde::Deserialize;
use std::collections::BTreeMap;

const GITHUB_API_BASE: &str = "https://api.github.com/repos";
const PYPI_API_BASE: &str = "https://pypi.org/pypi";

/// A release resolved down to one downloadable artifact for the current platform.
pub struct ResolvedRelease {
    pub version: String,
    pub url: String,
    /// Expected digest of the download, when the registry publishes one.
    pub sha256: Option<String>,
}

/// Resolve a tool version to a concrete download for this platform.
///
/// Ninja comes from GitHub releases; the LLVM tools come from PyPI wheels,
/// which are the only prebuilt single-file distribution that covers every
/// platform Ordo ships binaries for.
pub fn resolve_release(tool: Tool, version_req: Option<&str>) -> Result<ResolvedRelease> {
    match tool {
        Tool::Ninja => github_release(tool, "ninja-build/ninja", version_req),
        Tool::ClangFormat => pypi_release("clang-format", version_req),
    }
}

// ---- GitHub releases ----

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<AssetInfo>,
}

#[derive(Debug, Deserialize)]
struct AssetInfo {
    name: String,
    browser_download_url: String,
}

fn github_release(tool: Tool, repo: &str, version_req: Option<&str>) -> Result<ResolvedRelease> {
    let url = match version_req {
        Some(v) => {
            let tag = v.strip_prefix('v').unwrap_or(v);
            format!("{GITHUB_API_BASE}/{repo}/releases/tags/v{tag}")
        }
        None => format!("{GITHUB_API_BASE}/{repo}/releases/latest"),
    };

    let release = fetch_github_release(&url)?;
    let asset_name = github_asset_name(tool)?;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .ok_or_else(|| {
            miette::miette!(
                "no matching asset '{}' found in release {}",
                asset_name,
                release.tag_name
            )
        })?;

    Ok(ResolvedRelease {
        version: release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name)
            .to_string(),
        url: asset.browser_download_url.clone(),
        sha256: None,
    })
}

fn fetch_github_release(url: &str) -> Result<ReleaseInfo> {
    let mut builder = client_builder();

    // Use GITHUB_TOKEN for authentication if available (avoids rate limits)
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
            headers.insert(AUTHORIZATION, val);
        }
        builder = builder.default_headers(headers);
    }

    let client = builder.build().into_diagnostic()?;
    let response = client.get(url).send().into_diagnostic()?;

    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        bail!(
            "GitHub API rate limit exceeded. Try again later or set GITHUB_TOKEN environment variable."
        );
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        bail!("release not found: {url}");
    }
    if !status.is_success() {
        bail!("GitHub API request failed: HTTP {status}");
    }

    response.json::<ReleaseInfo>().into_diagnostic()
}

// ---- PyPI wheels ----

#[derive(Debug, Deserialize)]
struct PyPiIndex {
    info: PyPiInfo,
    releases: BTreeMap<String, Vec<PyPiFile>>,
}

#[derive(Debug, Deserialize)]
struct PyPiInfo {
    version: String,
}

#[derive(Debug, Deserialize)]
struct PyPiFile {
    filename: String,
    url: String,
    digests: PyPiDigests,
    #[serde(default)]
    yanked: bool,
}

#[derive(Debug, Deserialize)]
struct PyPiDigests {
    sha256: Option<String>,
}

fn pypi_release(package: &str, version_req: Option<&str>) -> Result<ResolvedRelease> {
    let client = client_builder().build().into_diagnostic()?;
    let url = format!("{PYPI_API_BASE}/{package}/json");
    let response = client.get(&url).send().into_diagnostic()?;

    let status = response.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        bail!("package not found on PyPI: {package}");
    }
    if !status.is_success() {
        bail!("PyPI request failed: HTTP {status}");
    }

    let index: PyPiIndex = response.json().into_diagnostic()?;
    let version = select_version(&index, version_req)?;
    let files = index
        .releases
        .get(&version)
        .ok_or_else(|| miette::miette!("no files published for {package} {version}"))?;
    let file = select_wheel(files)?;

    Ok(ResolvedRelease {
        version,
        url: file.url.clone(),
        sha256: file.digests.sha256.clone(),
    })
}

fn select_version(index: &PyPiIndex, version_req: Option<&str>) -> Result<String> {
    let Some(req) = version_req else {
        return Ok(index.info.version.clone());
    };

    let mut candidates: Vec<&String> = index
        .releases
        .iter()
        .filter(|(v, files)| version_matches(v, req) && files.iter().any(|f| !f.yanked))
        .map(|(v, _)| v)
        .collect();
    candidates.sort_by_key(|v| version_key(v));

    candidates
        .last()
        .map(|v| (*v).clone())
        .ok_or_else(|| miette::miette!("no release matching version '{req}' found on PyPI"))
}

fn select_wheel(files: &[PyPiFile]) -> Result<&PyPiFile> {
    for tags in wheel_tags()? {
        let found = files.iter().find(|f| {
            !f.yanked && f.filename.ends_with(".whl") && tags.iter().all(|t| f.filename.contains(t))
        });
        if let Some(file) = found {
            return Ok(file);
        }
    }

    bail!(
        "no wheel published for this platform (os={}, arch={})",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// Sort key for dotted numeric versions. Non-numeric segments sort as 0,
/// which is good enough to pick the newest of a set of release versions.
fn version_key(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| {
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(0)
        })
        .collect()
}

fn client_builder() -> reqwest::blocking::ClientBuilder {
    reqwest::blocking::Client::builder().user_agent(format!("ordo/{}", env!("CARGO_PKG_VERSION")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wheel(filename: &str) -> PyPiFile {
        PyPiFile {
            filename: filename.to_string(),
            url: format!("https://example.invalid/{filename}"),
            digests: PyPiDigests {
                sha256: Some("deadbeef".to_string()),
            },
            yanked: false,
        }
    }

    #[test]
    fn version_key_orders_numerically() {
        assert!(version_key("23.1.1") > version_key("23.1.0"));
        assert!(version_key("23.1.1") > version_key("9.1.1"));
        assert!(version_key("20.1.0") < version_key("20.10.0"));
    }

    #[test]
    fn select_wheel_picks_current_platform() {
        let files = vec![
            wheel("clang_format-23.1.1-py2.py3-none-macosx_11_0_arm64.whl"),
            wheel("clang_format-23.1.1-py2.py3-none-macosx_10_9_x86_64.whl"),
            wheel("clang_format-23.1.1-py2.py3-none-manylinux_2_28_aarch64.whl"),
            wheel("clang_format-23.1.1-py2.py3-none-manylinux_2_28_x86_64.whl"),
            wheel("clang_format-23.1.1-py2.py3-none-musllinux_1_2_x86_64.whl"),
            wheel("clang_format-23.1.1-py2.py3-none-win_amd64.whl"),
            wheel("clang_format-23.1.1-py2.py3-none-win_arm64.whl"),
        ];

        let picked = select_wheel(&files).expect("current platform should have a wheel");
        let tags = wheel_tags().unwrap()[0];
        for tag in tags {
            assert!(
                picked.filename.contains(tag),
                "{} should contain {tag}",
                picked.filename
            );
        }
    }

    #[test]
    fn select_wheel_skips_yanked() {
        let mut files = vec![wheel("clang_format-23.1.1-py2.py3-none-any.whl")];
        for tag_set in wheel_tags().unwrap() {
            let mut name = "clang_format-23.1.1-py2.py3-none".to_string();
            for tag in *tag_set {
                name.push('-');
                name.push_str(tag);
            }
            name.push_str(".whl");
            let mut f = wheel(&name);
            f.yanked = true;
            files.push(f);
        }

        assert!(select_wheel(&files).is_err());
    }

    #[test]
    fn select_wheel_rejects_sdist_only() {
        let files = vec![wheel("clang_format-23.1.1.tar.gz")];
        assert!(select_wheel(&files).is_err());
    }
}
