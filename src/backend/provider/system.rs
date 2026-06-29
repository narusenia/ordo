use super::{FetchedDep, Provider, ResolvedDep};
use miette::Result;

pub struct SystemProvider;

impl Provider for SystemProvider {
    fn name(&self) -> &str {
        "system"
    }

    fn resolve(&self, name: &str, _version: Option<&str>) -> Result<ResolvedDep> {
        // System provider assumes the library is available in default search paths.
        // Actual availability is verified at link time.
        Ok(ResolvedDep {
            name: name.to_string(),
            version: "system".to_string(),
            source: "system".to_string(),
            checksum: None,
        })
    }

    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep> {
        // No include dirs — uses compiler defaults.
        // Only adds -l<name> via the libs field.
        Ok(FetchedDep {
            name: dep.name.clone(),
            include_dirs: Vec::new(),
            lib_dirs: Vec::new(),
            libs: vec![dep.name.clone()],
            frameworks: Vec::new(),
            defines: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_returns_system_source() {
        let provider = SystemProvider;
        let dep = provider.resolve("z", None).unwrap();
        assert_eq!(dep.name, "z");
        assert_eq!(dep.source, "system");
    }

    #[test]
    fn fetch_produces_link_flag() {
        let provider = SystemProvider;
        let dep = provider.resolve("z", None).unwrap();
        let fetched = provider.fetch(&dep).unwrap();
        assert_eq!(fetched.libs, vec!["z"]);
        assert!(fetched.include_dirs.is_empty());
        assert!(fetched.lib_dirs.is_empty());
    }
}
