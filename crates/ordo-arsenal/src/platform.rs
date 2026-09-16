use crate::Tool;
use miette::{Result, bail};
use std::path::Path;

/// Map (os, arch) to the expected GitHub release asset name.
pub fn github_asset_name(tool: Tool) -> Result<&'static str> {
    match tool {
        Tool::Ninja => ninja_asset_name(),
        Tool::ClangFormat => bail!("{} is not distributed via GitHub releases", tool.name()),
    }
}

fn ninja_asset_name() -> Result<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("macos", "aarch64") => Ok("ninja-mac.zip"),
        ("macos", "x86_64") => Ok("ninja-mac.zip"),
        ("linux", "x86_64") => Ok("ninja-linux.zip"),
        ("linux", "aarch64") => Ok("ninja-linux-aarch64.zip"),
        ("windows", _) => Ok("ninja-win.zip"),
        _ => bail!(
            "unsupported platform: os={}, arch={} — ninja has no prebuilt binary for this target",
            os,
            arch
        ),
    }
}

/// Substring sets identifying the wheels that run on this platform, in
/// preference order. A wheel matches when its filename contains every
/// substring in one set.
pub fn wheel_tags() -> Result<&'static [&'static [&'static str]]> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    Ok(match (os, arch) {
        ("macos", "aarch64") => &[&["macosx", "arm64"]],
        ("macos", "x86_64") => &[&["macosx", "x86_64"]],
        ("linux", "x86_64") => {
            if is_musl() {
                &[&["musllinux", "x86_64"]]
            } else {
                &[&["manylinux", "x86_64"]]
            }
        }
        ("linux", "aarch64") => {
            if is_musl() {
                &[&["musllinux", "aarch64"]]
            } else {
                &[&["manylinux", "aarch64"]]
            }
        }
        ("windows", "x86_64") => &[&["win_amd64"]],
        ("windows", "aarch64") => &[&["win_arm64"]],
        _ => bail!(
            "unsupported platform: os={}, arch={} — no prebuilt wheel for this target",
            os,
            arch
        ),
    })
}

/// Detect a musl libc system by the presence of its dynamic loader.
/// glibc wheels do not run there, and musl wheels do not run on glibc.
// ponytail: loader-path probe, swap for a real libc query if a distro shows up that lies about it
fn is_musl() -> bool {
    ["/lib/ld-musl-x86_64.so.1", "/lib/ld-musl-aarch64.so.1"]
        .iter()
        .any(|p| Path::new(p).exists())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_has_ninja_asset() {
        let result = github_asset_name(Tool::Ninja);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.ends_with(".zip"));
        assert!(name.starts_with("ninja-"));
    }

    #[test]
    fn clang_format_has_no_github_asset() {
        assert!(github_asset_name(Tool::ClangFormat).is_err());
    }

    #[test]
    fn current_platform_has_wheel_tags() {
        let tags = wheel_tags().expect("supported platform");
        assert!(!tags.is_empty());
        assert!(tags.iter().all(|set| !set.is_empty()));
    }

    #[test]
    fn linux_wheel_tags_pick_one_libc() {
        // Whichever libc this machine has, manylinux and musllinux never mix.
        if std::env::consts::OS != "linux" {
            return;
        }
        let tags = wheel_tags().unwrap();
        let flat: Vec<&str> = tags.iter().flat_map(|s| s.iter().copied()).collect();
        assert!(!(flat.contains(&"manylinux") && flat.contains(&"musllinux")));
    }
}
