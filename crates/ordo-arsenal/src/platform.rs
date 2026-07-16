use crate::Tool;
use miette::{Result, bail};

/// Map (os, arch) to the expected GitHub release asset name.
pub fn platform_asset_name(tool: Tool) -> Result<&'static str> {
    match tool {
        Tool::Ninja => ninja_asset_name(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_platform_has_asset() {
        let result = platform_asset_name(Tool::Ninja);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.ends_with(".zip"));
        assert!(name.starts_with("ninja-"));
    }
}
