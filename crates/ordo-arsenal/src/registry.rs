use crate::Tool;
use miette::{IntoDiagnostic, Result, bail};
use serde::Deserialize;

const GITHUB_API_BASE: &str = "https://api.github.com/repos";

/// Minimal representation of a GitHub release.
#[derive(Debug, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub assets: Vec<AssetInfo>,
}

/// Minimal representation of a GitHub release asset.
#[derive(Debug, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub browser_download_url: String,
}

fn repo_path(tool: Tool) -> &'static str {
    match tool {
        Tool::Ninja => "ninja-build/ninja",
    }
}

fn build_client() -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder()
        .user_agent(format!("ordo/{}", env!("CARGO_PKG_VERSION")));

    // Use GITHUB_TOKEN for authentication if available (avoids rate limits)
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
        let mut headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
            headers.insert(AUTHORIZATION, val);
        }
        builder = builder.default_headers(headers);
    }

    builder.build().into_diagnostic()
}

/// Fetch the latest release from GitHub.
pub fn fetch_latest_release(tool: Tool) -> Result<ReleaseInfo> {
    let url = format!("{}/{}/releases/latest", GITHUB_API_BASE, repo_path(tool));
    fetch_release(&url)
}

/// Fetch a specific release by tag from GitHub.
pub fn fetch_release_by_tag(tool: Tool, tag: &str) -> Result<ReleaseInfo> {
    let url = format!(
        "{}/{}/releases/tags/{}",
        GITHUB_API_BASE,
        repo_path(tool),
        tag
    );
    fetch_release(&url)
}

fn fetch_release(url: &str) -> Result<ReleaseInfo> {
    let client = build_client()?;
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
