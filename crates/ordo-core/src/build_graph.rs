use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BuildGraph {
    pub tasks: Vec<BuildTask>,
    pub links: Vec<LinkTask>,
    pub project_root: PathBuf,
    pub build_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BuildTask {
    pub source: PathBuf,
    pub object: PathBuf,
    pub depfile: PathBuf,
    pub command: Vec<String>,
    pub is_cpp: bool,
}

#[derive(Debug, Clone)]
pub struct LinkTask {
    pub objects: Vec<PathBuf>,
    pub output: PathBuf,
    pub command: Vec<String>,
    pub kind: LinkKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Executable,
    StaticLibrary,
    SharedLibrary,
}

impl BuildGraph {
    pub fn compile_commands_json(&self) -> String {
        let entries: Vec<CompileCommandEntry> = self
            .tasks
            .iter()
            .map(|task| CompileCommandEntry {
                directory: self.project_root.display().to_string(),
                file: task.source.display().to_string(),
                arguments: task.command.clone(),
            })
            .collect();

        serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".to_string())
    }
}

#[derive(serde::Serialize)]
struct CompileCommandEntry {
    directory: String,
    file: String,
    arguments: Vec<String>,
}
