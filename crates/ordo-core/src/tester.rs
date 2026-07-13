use crate::manifest::{PackageType, TestFramework};
use miette::{IntoDiagnostic, Result};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct TestTarget {
    pub name: String,
    pub source: PathBuf,
    pub framework: TestFramework,
}

pub fn discover_tests(
    project_root: &Path,
    test_src_override: Option<&str>,
    framework_override: Option<TestFramework>,
) -> Result<Vec<TestTarget>> {
    let test_dir = match test_src_override {
        Some(dir) => project_root.join(dir),
        None => {
            let tests = project_root.join("tests");
            let test = project_root.join("test");
            if tests.exists() {
                tests
            } else if test.exists() {
                test
            } else {
                return Ok(Vec::new());
            }
        }
    };

    if !test_dir.exists() {
        return Ok(Vec::new());
    }

    let mut sources = Vec::new();
    collect_test_sources(&test_dir, &mut sources)?;
    sources.sort();

    let mut result = Vec::new();
    for source in sources {
        let name = source
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let framework = match framework_override {
            Some(fw) => fw,
            None => detect_framework(&source)?,
        };

        result.push(TestTarget {
            name,
            source,
            framework,
        });
    }

    Ok(result)
}

fn collect_test_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if path.is_dir() {
            collect_test_sources(&path, out)?;
        } else if is_test_source(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_test_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("cpp" | "cc" | "cxx" | "c")
    )
}

fn detect_framework(source: &Path) -> Result<TestFramework> {
    let content = fs::read_to_string(source).into_diagnostic()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("#include") {
            continue;
        }
        if trimmed.contains("gtest/gtest.h") || trimmed.contains("gmock/gmock.h") {
            return Ok(TestFramework::Gtest);
        }
        if trimmed.contains("catch2/catch.hpp")
            || trimmed.contains("catch2/catch_all.hpp")
            || trimmed.contains("catch2/catch_test_macros.hpp")
        {
            return Ok(TestFramework::Catch2);
        }
        if trimmed.contains("doctest/doctest.h") || trimmed.contains("doctest.h") {
            return Ok(TestFramework::Doctest);
        }
    }
    Ok(TestFramework::Plain)
}

pub struct TestLibrary {
    pub sources: Vec<PathBuf>,
    pub lib_name: String,
}

pub fn extract_test_library(
    project_root: &Path,
    pkg_name: &str,
    package_type: PackageType,
) -> Result<Option<TestLibrary>> {
    let src_dir = project_root.join("src");
    if !src_dir.exists() {
        return Ok(None);
    }

    match package_type {
        PackageType::Executable => {
            let mut sources = Vec::new();
            collect_non_main_sources(&src_dir, &mut sources)?;
            if sources.is_empty() {
                return Ok(None);
            }
            sources.sort();
            Ok(Some(TestLibrary {
                sources,
                lib_name: format!("{pkg_name}_testlib"),
            }))
        }
        PackageType::StaticLibrary | PackageType::SharedLibrary => Ok(None),
    }
}

fn collect_non_main_sources(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).into_diagnostic()? {
        let entry = entry.into_diagnostic()?;
        let path = entry.path();
        if path.is_dir() {
            collect_non_main_sources(&path, out)?;
        } else if is_test_source(&path) && !is_main_file(&path) {
            out.push(path);
        }
    }
    Ok(())
}

fn is_main_file(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    stem == "main"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn detect_plain_framework() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("test_basic.cpp");
        fs::write(&src, "#include <cassert>\nint main() { return 0; }").unwrap();
        assert_eq!(detect_framework(&src).unwrap(), TestFramework::Plain);
    }

    #[test]
    fn detect_gtest_framework() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("test_foo.cpp");
        fs::write(
            &src,
            "#include <gtest/gtest.h>\nTEST(Foo, Bar) { EXPECT_TRUE(true); }",
        )
        .unwrap();
        assert_eq!(detect_framework(&src).unwrap(), TestFramework::Gtest);
    }

    #[test]
    fn detect_catch2_framework() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("test_foo.cpp");
        fs::write(
            &src,
            "#include <catch2/catch_test_macros.hpp>\nTEST_CASE(\"foo\") {}",
        )
        .unwrap();
        assert_eq!(detect_framework(&src).unwrap(), TestFramework::Catch2);
    }

    #[test]
    fn detect_doctest_framework() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("test_foo.cpp");
        fs::write(&src, "#include <doctest/doctest.h>\nTEST_CASE(\"foo\") {}").unwrap();
        assert_eq!(detect_framework(&src).unwrap(), TestFramework::Doctest);
    }

    #[test]
    fn discover_tests_in_tests_dir() {
        let dir = tempfile::tempdir().unwrap();
        let tests_dir = dir.path().join("tests");
        fs::create_dir_all(&tests_dir).unwrap();
        fs::write(
            tests_dir.join("test_a.cpp"),
            "#include <cassert>\nint main() { return 0; }",
        )
        .unwrap();
        fs::write(
            tests_dir.join("test_b.cpp"),
            "#include <cassert>\nint main() { return 0; }",
        )
        .unwrap();

        let targets = discover_tests(dir.path(), None, None).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "test_a");
        assert_eq!(targets[1].name, "test_b");
    }

    #[test]
    fn discover_no_tests_dir() {
        let dir = tempfile::tempdir().unwrap();
        let targets = discover_tests(dir.path(), None, None).unwrap();
        assert!(targets.is_empty());
    }

    #[test]
    fn extract_test_library_skips_main() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("main.cpp"), "int main() {}").unwrap();
        fs::write(src.join("util.cpp"), "int util() { return 1; }").unwrap();

        let lib = extract_test_library(dir.path(), "myapp", PackageType::Executable)
            .unwrap()
            .unwrap();
        assert_eq!(lib.lib_name, "myapp_testlib");
        assert_eq!(lib.sources.len(), 1);
        assert!(lib.sources[0].ends_with("util.cpp"));
    }

    #[test]
    fn extract_test_library_none_for_lib_project() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("lib.cpp"), "int foo() { return 1; }").unwrap();

        let lib = extract_test_library(dir.path(), "mylib", PackageType::StaticLibrary).unwrap();
        assert!(lib.is_none());
    }
}
