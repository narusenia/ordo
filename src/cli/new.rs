use crate::cli::ProjectLang;
use crate::util::style;
use miette::{IntoDiagnostic, Result, bail};
use promptuity::prompts::{Input, Select, SelectOption};
use promptuity::themes::MinimalTheme;
use promptuity::{Promptuity, Term};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run_interactive(base: &Path, no_git: bool, _ctx: &super::context::Context) -> Result<()> {
    let mut term = Term::default();
    let mut theme = MinimalTheme::default();
    let mut p = Promptuity::new(&mut term, &mut theme);

    p.with_intro("Create a new project")
        .begin()
        .into_diagnostic()?;

    let name: String = p
        .prompt(Input::new("Project name").with_placeholder("myapp"))
        .into_diagnostic()?;

    let lang: ProjectLang = p
        .prompt(
            Select::new(
                "Language",
                vec![
                    SelectOption::new("C++", ProjectLang::Cpp),
                    SelectOption::new("C", ProjectLang::C),
                ],
            )
            .as_mut(),
        )
        .into_diagnostic()?;

    let lib: bool = p
        .prompt(
            Select::new(
                "Type",
                vec![
                    SelectOption::new("executable", false),
                    SelectOption::new("static-library", true),
                ],
            )
            .as_mut(),
        )
        .into_diagnostic()?;

    p.finish().into_diagnostic()?;

    run(base, &name, lib, lang, no_git, _ctx)
}

pub fn run(
    base: &Path,
    name: &str,
    lib: bool,
    lang: ProjectLang,
    no_git: bool,
    _ctx: &super::context::Context,
) -> Result<()> {
    let project_dir = base.join(name);
    if project_dir.exists() {
        bail!("directory '{}' already exists", name);
    }

    fs::create_dir_all(&project_dir).into_diagnostic()?;

    if lib {
        create_library(&project_dir, name, lang)?;
    } else {
        create_executable(&project_dir, name, lang)?;
    }

    write_gitignore(&project_dir)?;

    if !no_git {
        git_init(&project_dir)?;
    }

    let kind = if lib { "library" } else { "executable" };
    let lang_label = match lang {
        ProjectLang::C => "C",
        ProjectLang::Cpp => "C++",
    };
    style::success("Created", &format!("{lang_label} {kind} project `{name}`"));

    let tree = build_tree(&project_dir, lib, lang);
    for line in &tree {
        style::tree_line(line);
    }

    Ok(())
}

fn build_tree(dir: &Path, lib: bool, lang: ProjectLang) -> Vec<String> {
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    let (src_ext, hdr_ext) = match lang {
        ProjectLang::C => ("c", "h"),
        ProjectLang::Cpp => ("cpp", "hpp"),
    };

    let mut lines = vec![format!("{name}/")];

    if lib {
        lines.push("├── Ordo.toml".to_string());
        lines.push("├── include/".to_string());
        lines.push(format!("│   └── {name}/"));
        lines.push(format!("│       └── {name}.{hdr_ext}"));
        lines.push("├── src/".to_string());
        lines.push(format!("│   └── {name}.{src_ext}"));
        lines.push("├── tests/".to_string());
        lines.push(format!("│   └── {name}_test.{src_ext}"));
        lines.push("└── .gitignore".to_string());
    } else {
        lines.push("├── Ordo.toml".to_string());
        lines.push("├── src/".to_string());
        lines.push(format!("│   └── main.{src_ext}"));
        lines.push("├── tests/".to_string());
        lines.push(format!("│   └── main_test.{src_ext}"));
        lines.push("└── .gitignore".to_string());
    }

    lines
}

fn create_executable(dir: &Path, name: &str, lang: ProjectLang) -> Result<()> {
    let (ext, src_content) = match lang {
        ProjectLang::C => (
            "c",
            r#"#include <stdio.h>

int main(void) {
    printf("Hello, world!\n");
    return 0;
}
"#
            .to_string(),
        ),
        ProjectLang::Cpp => (
            "cpp",
            r#"#include <iostream>

int main() {
    std::cout << "Hello, world!" << std::endl;
    return 0;
}
"#
            .to_string(),
        ),
    };

    write_manifest(dir, name, "executable", lang)?;

    fs::create_dir_all(dir.join("src")).into_diagnostic()?;
    fs::write(dir.join(format!("src/main.{ext}")), src_content).into_diagnostic()?;

    fs::create_dir_all(dir.join("tests")).into_diagnostic()?;
    fs::write(
        dir.join(format!("tests/main_test.{ext}")),
        match lang {
            ProjectLang::C => format!(
                r#"#include <assert.h>

int main(void) {{
    /* {name} tests */
    assert(1 + 1 == 2);
    return 0;
}}
"#
            ),
            ProjectLang::Cpp => format!(
                r#"#include <cassert>

int main() {{
    // {name} tests
    assert(1 + 1 == 2);
    return 0;
}}
"#
            ),
        },
    )
    .into_diagnostic()?;

    Ok(())
}

fn create_library(dir: &Path, name: &str, lang: ProjectLang) -> Result<()> {
    write_manifest(dir, name, "static-library", lang)?;

    let header_dir = dir.join(format!("include/{name}"));
    fs::create_dir_all(&header_dir).into_diagnostic()?;

    let guard = name.to_uppercase().replace('-', "_");

    match lang {
        ProjectLang::C => {
            let header_ext = "h";
            let src_ext = "c";

            fs::write(
                header_dir.join(format!("{name}.{header_ext}")),
                format!(
                    r#"#ifndef {guard}_H
#define {guard}_H

int {name}_version(void);

#endif /* {guard}_H */
"#
                ),
            )
            .into_diagnostic()?;

            fs::create_dir_all(dir.join("src")).into_diagnostic()?;
            fs::write(
                dir.join(format!("src/{name}.{src_ext}")),
                format!(
                    r#"#include "{name}/{name}.{header_ext}"

int {name}_version(void) {{
    return 1;
}}
"#
                ),
            )
            .into_diagnostic()?;

            fs::create_dir_all(dir.join("tests")).into_diagnostic()?;
            fs::write(
                dir.join(format!("tests/{name}_test.{src_ext}")),
                format!(
                    r#"#include <assert.h>
#include "{name}/{name}.{header_ext}"

int main(void) {{
    assert({name}_version() == 1);
    return 0;
}}
"#
                ),
            )
            .into_diagnostic()?;
        }
        ProjectLang::Cpp => {
            let header_ext = "hpp";
            let src_ext = "cpp";

            fs::write(
                header_dir.join(format!("{name}.{header_ext}")),
                format!(
                    r#"#ifndef {guard}_HPP
#define {guard}_HPP

namespace {name} {{

int version();

}} // namespace {name}

#endif // {guard}_HPP
"#
                ),
            )
            .into_diagnostic()?;

            fs::create_dir_all(dir.join("src")).into_diagnostic()?;
            fs::write(
                dir.join(format!("src/{name}.{src_ext}")),
                format!(
                    r#"#include "{name}/{name}.{header_ext}"

namespace {name} {{

int version() {{
    return 1;
}}

}} // namespace {name}
"#
                ),
            )
            .into_diagnostic()?;

            fs::create_dir_all(dir.join("tests")).into_diagnostic()?;
            fs::write(
                dir.join(format!("tests/{name}_test.{src_ext}")),
                format!(
                    r#"#include <cassert>
#include "{name}/{name}.{header_ext}"

int main() {{
    assert({name}::version() == 1);
    return 0;
}}
"#
                ),
            )
            .into_diagnostic()?;
        }
    }

    Ok(())
}

fn write_manifest(dir: &Path, name: &str, package_type: &str, lang: ProjectLang) -> Result<()> {
    let language_section = match lang {
        ProjectLang::C => "\n[language]\nc = \"c23\"\n",
        ProjectLang::Cpp => "\n[language]\ncpp = \"c++20\"\n",
    };

    fs::write(
        dir.join("Ordo.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
type = "{package_type}"
{language_section}"#
        ),
    )
    .into_diagnostic()
}

fn write_gitignore(dir: &Path) -> Result<()> {
    fs::write(dir.join(".gitignore"), "target/\n").into_diagnostic()
}

fn git_init(dir: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .into_diagnostic()?;

    if !status.success() {
        tracing::warn!("git init failed (exit {})", status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::context::Context;
    use tempfile::TempDir;

    #[test]
    fn new_cpp_executable() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        run(tmp.path(), "myapp", false, ProjectLang::Cpp, true, &ctx).unwrap();

        let dir = tmp.path().join("myapp");
        assert!(dir.join("Ordo.toml").exists());
        assert!(dir.join("src/main.cpp").exists());
        assert!(dir.join("tests/main_test.cpp").exists());

        let manifest = fs::read_to_string(dir.join("Ordo.toml")).unwrap();
        assert!(manifest.contains("type = \"executable\""));
        assert!(manifest.contains("cpp = \"c++20\""));
    }

    #[test]
    fn new_c_executable() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        run(tmp.path(), "myapp", false, ProjectLang::C, true, &ctx).unwrap();

        let dir = tmp.path().join("myapp");
        assert!(dir.join("src/main.c").exists());
        assert!(dir.join("tests/main_test.c").exists());
        assert!(!dir.join("src/main.cpp").exists());

        let manifest = fs::read_to_string(dir.join("Ordo.toml")).unwrap();
        assert!(manifest.contains("c = \"c23\""));
    }

    #[test]
    fn new_cpp_library() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        run(tmp.path(), "mylib", true, ProjectLang::Cpp, true, &ctx).unwrap();

        let dir = tmp.path().join("mylib");
        assert!(dir.join("include/mylib/mylib.hpp").exists());
        assert!(dir.join("src/mylib.cpp").exists());
        assert!(dir.join("tests/mylib_test.cpp").exists());

        let manifest = fs::read_to_string(dir.join("Ordo.toml")).unwrap();
        assert!(manifest.contains("type = \"static-library\""));
        assert!(manifest.contains("cpp = \"c++20\""));
    }

    #[test]
    fn new_c_library() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        run(tmp.path(), "mylib", true, ProjectLang::C, true, &ctx).unwrap();

        let dir = tmp.path().join("mylib");
        assert!(dir.join("include/mylib/mylib.h").exists());
        assert!(dir.join("src/mylib.c").exists());
        assert!(dir.join("tests/mylib_test.c").exists());

        let manifest = fs::read_to_string(dir.join("Ordo.toml")).unwrap();
        assert!(manifest.contains("c = \"c23\""));

        let header = fs::read_to_string(dir.join("include/mylib/mylib.h")).unwrap();
        assert!(header.contains("MYLIB_H"));
    }

    #[test]
    fn new_fails_if_directory_exists() {
        let tmp = TempDir::new().unwrap();
        let ctx = Context::default_for_test();
        fs::create_dir(tmp.path().join("existing")).unwrap();
        let result = run(tmp.path(), "existing", false, ProjectLang::Cpp, true, &ctx);
        assert!(result.is_err());
    }
}
