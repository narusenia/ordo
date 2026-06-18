use super::{ColorMode, StyleMode};
use crate::core::manifest::Manifest;

#[allow(dead_code)]
pub struct Context {
    pub style: StyleMode,
    pub verbose: u8,
    pub color: ColorMode,
}

impl Context {
    pub fn resolve(cli_style: StyleMode, cli_verbose: u8, cli_color: ColorMode) -> Self {
        let style = if cli_style != StyleMode::Default {
            cli_style
        } else {
            Self::style_from_manifest().unwrap_or(StyleMode::Default)
        };

        Self {
            style,
            verbose: cli_verbose,
            color: cli_color,
        }
    }

    #[cfg(test)]
    pub fn default_for_test() -> Self {
        Self {
            style: StyleMode::Default,
            verbose: 0,
            color: ColorMode::Auto,
        }
    }

    fn style_from_manifest() -> Option<StyleMode> {
        let cwd = std::env::current_dir().ok()?;
        let manifest_path = cwd.join("Ordo.toml");
        if !manifest_path.exists() {
            return None;
        }
        let manifest = Manifest::load(&manifest_path).ok()?;
        manifest.cli.and_then(|c| c.style)
    }
}
