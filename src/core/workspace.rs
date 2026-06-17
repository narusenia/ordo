#![allow(dead_code)]

use crate::core::manifest::{
    DependencySource, Language, Manifest, Toolchain, WorkspaceConfig,
};
use miette::{Result, bail};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Workspace {
    pub root_dir: PathBuf,
    pub root_manifest: Manifest,
    pub members: Vec<WorkspaceMember>,
}

#[derive(Debug)]
pub struct WorkspaceMember {
    pub name: String,
    pub dir: PathBuf,
    pub manifest: Manifest,
}

impl Workspace {
    pub fn load(root_dir: &Path) -> Result<Self> {
        let manifest_path = root_dir.join("Ordo.toml");
        let root_manifest = Manifest::load(&manifest_path)?;

        let ws_config = root_manifest.workspace.as_ref().ok_or_else(|| {
            miette::miette!("Ordo.toml at {} does not contain [workspace]", root_dir.display())
        })?;

        let member_dirs = discover_members(root_dir, ws_config)?;
        let mut members = Vec::with_capacity(member_dirs.len());

        for dir in &member_dirs {
            let member_manifest_path = dir.join("Ordo.toml");
            if !member_manifest_path.exists() {
                bail!(
                    "workspace member '{}' has no Ordo.toml",
                    dir.display()
                );
            }
            let mut manifest = Manifest::load(&member_manifest_path)?;
            let pkg = manifest.package.as_ref().ok_or_else(|| {
                miette::miette!(
                    "workspace member '{}' must have a [package] section",
                    dir.display()
                )
            })?;
            let name = pkg.name.clone();

            resolve_workspace_deps(&mut manifest, ws_config)?;

            members.push(WorkspaceMember {
                name,
                dir: dir.clone(),
                manifest,
            });
        }

        validate_unique_names(&members)?;

        Ok(Workspace {
            root_dir: root_dir.to_path_buf(),
            root_manifest,
            members,
        })
    }

    pub fn ws_config(&self) -> &WorkspaceConfig {
        self.root_manifest.workspace.as_ref().unwrap()
    }

    pub fn root_language(&self) -> &Language {
        &self.root_manifest.language
    }

    pub fn root_toolchain(&self) -> &Toolchain {
        &self.root_manifest.toolchain
    }

    pub fn find_member(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members.iter().find(|m| m.name == name)
    }

    pub fn member_names(&self) -> Vec<&str> {
        self.members.iter().map(|m| m.name.as_str()).collect()
    }

    pub fn build_dag(&self) -> Result<MemberDag> {
        MemberDag::build(&self.members)
    }
}

impl WorkspaceMember {
    pub fn effective_language(&self, root: &Language) -> EffectiveLanguage {
        EffectiveLanguage {
            c: self.manifest.language.c.or(root.c),
            cpp: self.manifest.language.cpp.or(root.cpp),
        }
    }

    pub fn effective_toolchain(&self, root: &Toolchain) -> EffectiveToolchain {
        EffectiveToolchain {
            compiler: self.manifest.toolchain.compiler.or(root.compiler),
            linker: self.manifest.toolchain.linker.or(root.linker),
        }
    }
}

#[derive(Debug)]
pub struct EffectiveLanguage {
    pub c: Option<crate::core::manifest::CStandard>,
    pub cpp: Option<crate::core::manifest::CppStandard>,
}

#[derive(Debug)]
pub struct EffectiveToolchain {
    pub compiler: Option<crate::core::manifest::CompilerKind>,
    pub linker: Option<crate::core::manifest::LinkerKind>,
}

// --- Member DAG ---

#[derive(Debug)]
pub struct MemberDag {
    pub order: Vec<String>,
    adjacency: HashMap<String, Vec<String>>,
}

impl MemberDag {
    fn build(members: &[WorkspaceMember]) -> Result<Self> {
        let member_names: BTreeSet<&str> = members.iter().map(|m| m.name.as_str()).collect();
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

        for member in members {
            let mut deps = Vec::new();
            for (_dep_name, spec) in &member.manifest.dependencies {
                if spec.source_kind() == DependencySource::Path {
                    if let Some(ref path) = spec.path {
                        let dep_dir = member.dir.join(path);
                        if let Ok(dep_manifest_path) = std::fs::canonicalize(dep_dir.join("Ordo.toml")) {
                            if let Ok(dep_manifest) = Manifest::load(&dep_manifest_path) {
                                if let Some(ref pkg) = dep_manifest.package {
                                    if member_names.contains(pkg.name.as_str()) {
                                        deps.push(pkg.name.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            adjacency.insert(member.name.clone(), deps);
        }

        let order = topological_sort(&adjacency)?;
        Ok(MemberDag { order, adjacency })
    }

    pub fn deps_of(&self, name: &str) -> &[String] {
        self.adjacency.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn transitive_deps(&self, name: &str) -> BTreeSet<String> {
        let mut visited = BTreeSet::new();
        self.collect_deps(name, &mut visited);
        visited.remove(name);
        visited
    }

    fn collect_deps(&self, name: &str, visited: &mut BTreeSet<String>) {
        if !visited.insert(name.to_string()) {
            return;
        }
        for dep in self.deps_of(name) {
            self.collect_deps(dep, visited);
        }
    }

    pub fn subset_order(&self, target: &str) -> Vec<String> {
        let needed = self.transitive_deps(target);
        self.order
            .iter()
            .filter(|n| needed.contains(n.as_str()) || *n == target)
            .cloned()
            .collect()
    }
}

fn topological_sort(adjacency: &HashMap<String, Vec<String>>) -> Result<Vec<String>> {
    // adjacency: node -> [deps it depends on]
    // Build reverse graph: node -> [nodes that depend on it]
    let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut dep_count: HashMap<&str, usize> = HashMap::new();

    for name in adjacency.keys() {
        reverse.entry(name.as_str()).or_default();
        dep_count.entry(name.as_str()).or_insert(0);
    }
    for (node, deps) in adjacency {
        *dep_count.entry(node.as_str()).or_insert(0) = deps.len();
        for dep in deps {
            reverse.entry(dep.as_str()).or_default().push(node.as_str());
        }
    }

    let mut queue: Vec<&str> = dep_count
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(&name, _)| name)
        .collect();
    queue.sort();

    let mut order = Vec::new();
    while let Some(node) = queue.pop() {
        order.push(node.to_string());
        if let Some(dependents) = reverse.get(node) {
            for dependent in dependents {
                if let Some(count) = dep_count.get_mut(dependent) {
                    *count -= 1;
                    if *count == 0 {
                        queue.push(dependent);
                        queue.sort();
                    }
                }
            }
        }
    }

    if order.len() != adjacency.len() {
        let remaining: Vec<&str> = adjacency
            .keys()
            .filter(|k| !order.contains(k))
            .map(|k| k.as_str())
            .collect();
        bail!(
            "circular dependency detected among workspace members: {}",
            remaining.join(", ")
        );
    }

    Ok(order)
}

// --- Discovery ---

fn discover_members(root_dir: &Path, config: &WorkspaceConfig) -> Result<Vec<PathBuf>> {
    let mut dirs = BTreeSet::new();

    for pattern in &config.members {
        let full_pattern = root_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        let matches = glob::glob(&pattern_str)
            .map_err(|e| miette::miette!("invalid glob pattern '{}': {}", pattern, e))?;

        for entry in matches {
            let path = entry.map_err(|e| miette::miette!("glob error: {}", e))?;
            if path.is_dir() && path.join("Ordo.toml").exists() {
                let canonical = std::fs::canonicalize(&path)
                    .map_err(|e| miette::miette!("failed to canonicalize {}: {}", path.display(), e))?;
                dirs.insert(canonical);
            }
        }
    }

    let root_canonical = std::fs::canonicalize(root_dir)
        .map_err(|e| miette::miette!("failed to canonicalize root: {}", e))?;
    dirs.remove(&root_canonical);

    let exclude_set = build_exclude_set(root_dir, &config.exclude)?;
    dirs.retain(|d| !exclude_set.contains(d));

    Ok(dirs.into_iter().collect())
}

fn build_exclude_set(root_dir: &Path, exclude: &[String]) -> Result<BTreeSet<PathBuf>> {
    let mut set = BTreeSet::new();
    for pattern in exclude {
        let full_pattern = root_dir.join(pattern);
        let pattern_str = full_pattern.to_string_lossy();
        if let Ok(matches) = glob::glob(&pattern_str) {
            for entry in matches.flatten() {
                if let Ok(canonical) = std::fs::canonicalize(&entry) {
                    set.insert(canonical);
                }
            }
        }
    }
    Ok(set)
}

// --- Workspace dependency resolution ---

fn resolve_workspace_deps(
    member: &mut Manifest,
    ws_config: &WorkspaceConfig,
) -> Result<()> {
    let ws_deps = &ws_config.dependencies;

    for (name, spec) in &mut member.dependencies {
        if spec.workspace {
            let ws_spec = ws_deps.get(name).ok_or_else(|| {
                miette::miette!(
                    "dependency '{}' uses `workspace = true` but is not declared in [workspace.dependencies]",
                    name
                )
            })?;
            *spec = ws_spec.clone();
        }
    }

    for (name, spec) in &mut member.dev_dependencies {
        if spec.workspace {
            let ws_spec = ws_deps.get(name).ok_or_else(|| {
                miette::miette!(
                    "dev-dependency '{}' uses `workspace = true` but is not declared in [workspace.dependencies]",
                    name
                )
            })?;
            *spec = ws_spec.clone();
        }
    }

    Ok(())
}

fn validate_unique_names(members: &[WorkspaceMember]) -> Result<()> {
    let mut seen: HashMap<&str, &Path> = HashMap::new();
    for member in members {
        if let Some(prev) = seen.get(member.name.as_str()) {
            bail!(
                "duplicate workspace member name '{}': found at '{}' and '{}'",
                member.name,
                prev.display(),
                member.dir.display()
            );
        }
        seen.insert(&member.name, &member.dir);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_workspace(root: &Path, workspace_toml: &str, members: &[(&str, &str)]) {
        fs::write(root.join("Ordo.toml"), workspace_toml).unwrap();
        for (path, content) in members {
            let dir = root.join(path);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("Ordo.toml"), content).unwrap();
        }
    }

    #[test]
    fn discover_basic_members() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["libs/*"]
            "#,
            &[
                ("libs/core", r#"
                    [package]
                    name = "core"
                    version = "0.1.0"
                    type = "static-library"
                "#),
                ("libs/utils", r#"
                    [package]
                    name = "utils"
                    version = "0.1.0"
                    type = "static-library"
                "#),
            ],
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.members.len(), 2);
        let names: Vec<&str> = ws.member_names();
        assert!(names.contains(&"core"));
        assert!(names.contains(&"utils"));
    }

    #[test]
    fn discover_with_exclude() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["libs/*"]
            exclude = ["libs/experimental"]
            "#,
            &[
                ("libs/core", r#"
                    [package]
                    name = "core"
                    version = "0.1.0"
                    type = "static-library"
                "#),
                ("libs/experimental", r#"
                    [package]
                    name = "experimental"
                    version = "0.1.0"
                    type = "static-library"
                "#),
            ],
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert_eq!(ws.members.len(), 1);
        assert_eq!(ws.members[0].name, "core");
    }

    #[test]
    fn workspace_dep_inheritance() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["apps/*"]

            [workspace.dependencies]
            fmt = "11"
            "#,
            &[
                ("apps/myapp", r#"
                    [package]
                    name = "myapp"
                    version = "0.1.0"
                    type = "executable"

                    [dependencies]
                    fmt = { workspace = true }
                "#),
            ],
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let member = &ws.members[0];
        let fmt = &member.manifest.dependencies["fmt"];
        assert_eq!(fmt.version.as_deref(), Some("11"));
        assert!(!fmt.workspace);
    }

    #[test]
    fn workspace_dep_not_declared_errors() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["apps/*"]
            "#,
            &[
                ("apps/myapp", r#"
                    [package]
                    name = "myapp"
                    version = "0.1.0"
                    type = "executable"

                    [dependencies]
                    fmt = { workspace = true }
                "#),
            ],
        );

        let err = Workspace::load(tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not declared in [workspace.dependencies]"), "got: {msg}");
    }

    #[test]
    fn toolchain_inheritance() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["libs/*"]

            [toolchain]
            compiler = "clang"
            linker = "lld"

            [language]
            cpp = "c++23"
            "#,
            &[
                ("libs/core", r#"
                    [package]
                    name = "core"
                    version = "0.1.0"
                    type = "static-library"
                "#),
                ("libs/special", r#"
                    [package]
                    name = "special"
                    version = "0.1.0"
                    type = "static-library"

                    [toolchain]
                    compiler = "gcc"
                "#),
            ],
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let root_lang = ws.root_language();
        let root_tc = ws.root_toolchain();

        let core = ws.find_member("core").unwrap();
        let core_tc = core.effective_toolchain(root_tc);
        assert_eq!(core_tc.compiler, Some(crate::core::manifest::CompilerKind::Clang));
        assert_eq!(core_tc.linker, Some(crate::core::manifest::LinkerKind::Lld));

        let special = ws.find_member("special").unwrap();
        let special_tc = special.effective_toolchain(root_tc);
        assert_eq!(special_tc.compiler, Some(crate::core::manifest::CompilerKind::Gcc));
        assert_eq!(special_tc.linker, Some(crate::core::manifest::LinkerKind::Lld));

        let core_lang = core.effective_language(root_lang);
        assert_eq!(core_lang.cpp, Some(crate::core::manifest::CppStandard::Cpp23));
    }

    #[test]
    fn duplicate_member_names_errors() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["libs/*", "extras/*"]
            "#,
            &[
                ("libs/mylib", r#"
                    [package]
                    name = "mylib"
                    version = "0.1.0"
                    type = "static-library"
                "#),
                ("extras/mylib", r#"
                    [package]
                    name = "mylib"
                    version = "0.2.0"
                    type = "static-library"
                "#),
            ],
        );

        let err = Workspace::load(tmp.path()).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("duplicate workspace member"), "got: {msg}");
    }

    #[test]
    fn dag_topological_order() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["libs/*", "apps/*"]
            "#,
            &[
                ("libs/core", r#"
                    [package]
                    name = "core"
                    version = "0.1.0"
                    type = "static-library"
                "#),
                ("libs/utils", r#"
                    [package]
                    name = "utils"
                    version = "0.1.0"
                    type = "static-library"

                    [dependencies]
                    core = { path = "../core" }
                "#),
                ("apps/myapp", r#"
                    [package]
                    name = "myapp"
                    version = "0.1.0"
                    type = "executable"

                    [dependencies]
                    utils = { path = "../../libs/utils" }
                "#),
            ],
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let dag = ws.build_dag().unwrap();

        let core_idx = dag.order.iter().position(|n| n == "core").unwrap();
        let utils_idx = dag.order.iter().position(|n| n == "utils").unwrap();
        let app_idx = dag.order.iter().position(|n| n == "myapp").unwrap();

        assert!(core_idx < utils_idx);
        assert!(utils_idx < app_idx);
    }

    #[test]
    fn dag_transitive_deps() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [workspace]
            members = ["libs/*", "apps/*"]
            "#,
            &[
                ("libs/core", r#"
                    [package]
                    name = "core"
                    version = "0.1.0"
                    type = "static-library"
                "#),
                ("libs/utils", r#"
                    [package]
                    name = "utils"
                    version = "0.1.0"
                    type = "static-library"

                    [dependencies]
                    core = { path = "../core" }
                "#),
                ("apps/myapp", r#"
                    [package]
                    name = "myapp"
                    version = "0.1.0"
                    type = "executable"

                    [dependencies]
                    utils = { path = "../../libs/utils" }
                "#),
            ],
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        let dag = ws.build_dag().unwrap();

        let deps = dag.transitive_deps("myapp");
        assert!(deps.contains("utils"));
        assert!(deps.contains("core"));
        assert!(!deps.contains("myapp"));

        let core_deps = dag.transitive_deps("core");
        assert!(core_deps.is_empty());
    }

    #[test]
    fn dag_cycle_detection() {
        let mut adjacency = HashMap::new();
        adjacency.insert("a".to_string(), vec!["b".to_string()]);
        adjacency.insert("b".to_string(), vec!["a".to_string()]);

        let err = topological_sort(&adjacency).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("circular"), "got: {msg}");
    }

    #[test]
    fn workspace_with_root_package() {
        let tmp = TempDir::new().unwrap();
        setup_workspace(
            tmp.path(),
            r#"
            [package]
            name = "root-app"
            version = "1.0.0"
            type = "executable"

            [workspace]
            members = ["libs/*"]
            "#,
            &[
                ("libs/core", r#"
                    [package]
                    name = "core"
                    version = "0.1.0"
                    type = "static-library"
                "#),
            ],
        );

        let ws = Workspace::load(tmp.path()).unwrap();
        assert!(!ws.root_manifest.is_virtual_workspace());
        assert_eq!(ws.root_manifest.package().name, "root-app");
        assert_eq!(ws.members.len(), 1);
    }
}
