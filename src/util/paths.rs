#![allow(dead_code)]

use std::env;
use std::path::PathBuf;

pub struct OrdoPaths {
    pub config_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub credentials_file: PathBuf,
}

impl OrdoPaths {
    pub fn resolve() -> Self {
        if let Ok(home) = env::var("ORDO_HOME") {
            let home = PathBuf::from(home);
            return Self {
                credentials_file: home.join("credentials.toml"),
                config_dir: home.clone(),
                cache_dir: home,
            };
        }

        let config_dir = dirs::config_dir()
            .expect("could not determine config directory")
            .join("ordo");

        let cache_dir = dirs::cache_dir()
            .expect("could not determine cache directory")
            .join("ordo");

        let credentials_file = config_dir.join("credentials.toml");

        Self {
            config_dir,
            cache_dir,
            credentials_file,
        }
    }

    pub fn global_config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn registry_cache(&self) -> PathBuf {
        self.cache_dir.join("registry")
    }

    pub fn git_cache(&self) -> PathBuf {
        self.cache_dir.join("git")
    }

    pub fn toolchains_dir(&self) -> PathBuf {
        self.cache_dir.join("toolchains")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordo_home_override() {
        let _guard = TempEnvVar::set("ORDO_HOME", "/tmp/ordo-test");
        let paths = OrdoPaths::resolve();
        assert_eq!(paths.config_dir, PathBuf::from("/tmp/ordo-test"));
        assert_eq!(paths.cache_dir, PathBuf::from("/tmp/ordo-test"));
        assert_eq!(
            paths.credentials_file,
            PathBuf::from("/tmp/ordo-test/credentials.toml")
        );
    }

    #[test]
    fn default_paths_are_under_ordo_subdir() {
        let _guard = TempEnvVar::remove("ORDO_HOME");
        let paths = OrdoPaths::resolve();
        assert!(paths.config_dir.ends_with("ordo"));
        assert!(paths.cache_dir.ends_with("ordo"));
    }

    #[test]
    fn subpaths_derive_from_base() {
        let _guard = TempEnvVar::set("ORDO_HOME", "/tmp/ordo-test");
        let paths = OrdoPaths::resolve();
        assert_eq!(
            paths.global_config_file(),
            PathBuf::from("/tmp/ordo-test/config.toml")
        );
        assert_eq!(
            paths.registry_cache(),
            PathBuf::from("/tmp/ordo-test/registry")
        );
        assert_eq!(paths.git_cache(), PathBuf::from("/tmp/ordo-test/git"));
        assert_eq!(
            paths.toolchains_dir(),
            PathBuf::from("/tmp/ordo-test/toolchains")
        );
    }

    struct TempEnvVar {
        key: &'static str,
        prev: Option<String>,
    }

    impl TempEnvVar {
        fn set(key: &'static str, val: &str) -> Self {
            let prev = env::var(key).ok();
            // SAFETY: tests run single-threaded via `cargo test -- --test-threads=1`
            // or are isolated by unique env var names.
            unsafe { env::set_var(key, val) };
            Self { key, prev }
        }

        fn remove(key: &'static str) -> Self {
            let prev = env::var(key).ok();
            unsafe { env::remove_var(key) };
            Self { key, prev }
        }
    }

    impl Drop for TempEnvVar {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { env::set_var(self.key, v) },
                None => unsafe { env::remove_var(self.key) },
            }
        }
    }
}
