#![allow(dead_code)]

use super::sandbox::SandboxScope;
use miette::{IntoDiagnostic, Result};
use mlua::prelude::*;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct LuaBuildResult {
    #[serde(default)]
    pub include_dirs: Vec<String>,
    #[serde(default)]
    pub lib_dirs: Vec<String>,
    #[serde(default)]
    pub libs: Vec<String>,
}

pub struct LuaContext {
    pub src_dir: PathBuf,
    pub out_dir: PathBuf,
    pub target_os: String,
    pub target_arch: String,
    pub profile: String,
    pub compiler_cc: String,
    pub compiler_cxx: String,
    pub compiler_id: String,
}

pub struct LuaRunner;

impl LuaRunner {
    pub fn execute(script_path: &Path, ctx: &LuaContext) -> Result<LuaBuildResult> {
        let script_content = std::fs::read_to_string(script_path).map_err(|e| {
            miette::miette!(
                "failed to read Lua script '{}': {}",
                script_path.display(),
                e
            )
        })?;

        let lua = Lua::new();

        Self::remove_dangerous_globals(&lua)?;
        Self::inject_context(&lua, ctx)?;

        let sandbox = Arc::new(SandboxScope::new(ctx.src_dir.clone(), ctx.out_dir.clone()));
        Self::register_exec(&lua, &ctx.src_dir)?;
        Self::register_file_helpers(&lua, sandbox)?;

        let result: LuaValue = lua
            .load(&script_content)
            .set_name(script_path.display().to_string())
            .eval()
            .map_err(|e| miette::miette!("Lua script error: {e}"))?;

        let build_result: LuaBuildResult = lua
            .from_value(result)
            .map_err(|e| {
                miette::miette!(
                    "Lua script must return {{ include_dirs = {{...}}, lib_dirs = {{...}}, libs = {{...}} }}: {e}"
                )
            })?;

        Ok(build_result)
    }

    fn remove_dangerous_globals(lua: &Lua) -> Result<()> {
        let globals = lua.globals();
        for name in &["io", "os", "loadfile", "dofile"] {
            globals
                .set(*name, LuaNil)
                .map_err(|e| miette::miette!("failed to remove '{name}': {e}"))?;
        }
        Ok(())
    }

    fn inject_context(lua: &Lua, ctx: &LuaContext) -> Result<()> {
        let globals = lua.globals();

        globals
            .set("src", ctx.src_dir.display().to_string())
            .into_diagnostic()?;
        globals
            .set("out", ctx.out_dir.display().to_string())
            .into_diagnostic()?;
        globals
            .set("profile", ctx.profile.as_str())
            .into_diagnostic()?;

        let target = lua.create_table().into_diagnostic()?;
        target.set("os", ctx.target_os.as_str()).into_diagnostic()?;
        target
            .set("arch", ctx.target_arch.as_str())
            .into_diagnostic()?;
        globals.set("target", target).into_diagnostic()?;

        let compiler = lua.create_table().into_diagnostic()?;
        compiler
            .set("cc", ctx.compiler_cc.as_str())
            .into_diagnostic()?;
        compiler
            .set("cxx", ctx.compiler_cxx.as_str())
            .into_diagnostic()?;
        compiler
            .set("id", ctx.compiler_id.as_str())
            .into_diagnostic()?;
        globals.set("compiler", compiler).into_diagnostic()?;

        Ok(())
    }

    fn register_exec(lua: &Lua, src_dir: &Path) -> Result<()> {
        let src_dir = src_dir.to_path_buf();
        let exec_fn = lua
            .create_function(
                move |_, (command, args, opts): (String, Option<LuaTable>, Option<LuaTable>)| {
                    let mut cmd_args: Vec<String> = Vec::new();
                    if let Some(args_table) = args {
                        for pair in args_table.sequence_values::<String>() {
                            cmd_args.push(pair.map_err(LuaError::external)?);
                        }
                    }

                    let ignore_errors = opts
                        .as_ref()
                        .and_then(|o| o.get::<bool>("ignore_errors").ok())
                        .unwrap_or(false);

                    let output = std::process::Command::new(&command)
                        .args(&cmd_args)
                        .current_dir(&src_dir)
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .output()
                        .map_err(|e| LuaError::external(format!("exec '{command}': {e}")))?;

                    let code = output.status.code().unwrap_or(-1);

                    if !output.status.success() && !ignore_errors {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let mut detail = String::new();
                        if !stdout.is_empty() {
                            detail.push_str(&stdout);
                        }
                        if !stderr.is_empty() {
                            detail.push_str(&stderr);
                        }
                        return Err(LuaError::external(format!(
                            "command '{command}' failed with exit code {code}\n{detail}"
                        )));
                    }

                    Ok((output.status.success(), code))
                },
            )
            .into_diagnostic()?;

        lua.globals().set("exec", exec_fn).into_diagnostic()?;
        Ok(())
    }

    fn register_file_helpers(lua: &Lua, sandbox: Arc<SandboxScope>) -> Result<()> {
        // copy(src_path, dst_path)
        let sb = sandbox.clone();
        let copy_fn = lua
            .create_function(move |_, (src_path, dst_path): (String, String)| {
                let src = sb.resolve_path(&src_path);
                let dst = sb.resolve_path(&dst_path);
                sb.validate_path(&src).map_err(LuaError::external)?;
                sb.validate_path(&dst).map_err(LuaError::external)?;

                if src.is_dir() {
                    copy_dir_recursive(&src, &dst).map_err(LuaError::external)?;
                } else {
                    if let Some(parent) = dst.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|e| LuaError::external(format!("mkdir for copy: {e}")))?;
                    }
                    std::fs::copy(&src, &dst).map_err(|e| {
                        LuaError::external(format!(
                            "copy '{}' -> '{}': {e}",
                            src.display(),
                            dst.display()
                        ))
                    })?;
                }
                Ok(())
            })
            .into_diagnostic()?;

        // mkdir(path)
        let sb = sandbox.clone();
        let mkdir_fn = lua
            .create_function(move |_, path: String| {
                let full_path = sb.resolve_path(&path);
                sb.validate_path(&full_path).map_err(LuaError::external)?;
                std::fs::create_dir_all(&full_path).map_err(|e| {
                    LuaError::external(format!("mkdir '{}': {e}", full_path.display()))
                })?;
                Ok(())
            })
            .into_diagnostic()?;

        // glob(pattern)
        let sb = sandbox.clone();
        let glob_fn = lua
            .create_function(move |lua, pattern: String| {
                let full_pattern = sb.resolve_path(&pattern);
                sb.validate_path(&full_pattern)
                    .map_err(LuaError::external)?;

                let matches = glob::glob(&full_pattern.to_string_lossy())
                    .map_err(|e| LuaError::external(format!("invalid glob: {e}")))?;

                let result = lua.create_table()?;
                for (idx, entry) in matches.flatten().enumerate() {
                    result.set(idx + 1, entry.display().to_string())?;
                }
                Ok(result)
            })
            .into_diagnostic()?;

        let globals = lua.globals();
        globals.set("copy", copy_fn).into_diagnostic()?;
        globals.set("mkdir", mkdir_fn).into_diagnostic()?;
        globals.set("glob", glob_fn).into_diagnostic()?;

        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::result::Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("mkdir '{}': {e}", dst.display()))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("read_dir '{}': {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("read entry: {e}"))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "copy '{}' -> '{}': {e}",
                    src_path.display(),
                    dst_path.display()
                )
            })?;
        }
    }
    Ok(())
}

pub fn compute_script_hash(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let content = std::fs::read(path)
        .map_err(|e| miette::miette!("failed to read script '{}': {e}", path.display()))?;
    let hash = Sha256::digest(&content);
    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    Ok(format!("sha256:{hex}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_context(tmp: &TempDir) -> LuaContext {
        let src = tmp.path().join("src");
        let out = tmp.path().join("out");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&out).unwrap();

        LuaContext {
            src_dir: src,
            out_dir: out,
            target_os: "macos".to_string(),
            target_arch: "aarch64".to_string(),
            profile: "debug".to_string(),
            compiler_cc: "clang".to_string(),
            compiler_cxx: "clang++".to_string(),
            compiler_id: "clang".to_string(),
        }
    }

    #[test]
    fn basic_return_value() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            return {
                include_dirs = { src .. "/include" },
                lib_dirs = { out .. "/lib" },
                libs = { "mylib" }
            }
            "#,
        )
        .unwrap();

        let result = LuaRunner::execute(&script, &ctx).unwrap();
        assert_eq!(result.include_dirs.len(), 1);
        assert!(result.include_dirs[0].ends_with("/src/include"));
        assert_eq!(result.libs, vec!["mylib"]);
    }

    #[test]
    fn empty_return_value() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"return { include_dirs = { src .. "/include" }, lib_dirs = {}, libs = {} }"#,
        )
        .unwrap();

        let result = LuaRunner::execute(&script, &ctx).unwrap();
        assert!(result.libs.is_empty());
    }

    #[test]
    fn context_variables_available() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            assert(target.os == "macos")
            assert(target.arch == "aarch64")
            assert(profile == "debug")
            assert(compiler.id == "clang")
            assert(compiler.cc == "clang")
            assert(compiler.cxx == "clang++")
            return { include_dirs = {}, lib_dirs = {}, libs = {} }
            "#,
        )
        .unwrap();

        LuaRunner::execute(&script, &ctx).unwrap();
    }

    #[test]
    fn dangerous_globals_removed() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            assert(io == nil, "io should be nil")
            assert(os == nil, "os should be nil")
            assert(loadfile == nil, "loadfile should be nil")
            assert(dofile == nil, "dofile should be nil")
            return { include_dirs = {}, lib_dirs = {}, libs = {} }
            "#,
        )
        .unwrap();

        LuaRunner::execute(&script, &ctx).unwrap();
    }

    #[test]
    fn invalid_return_value_errors() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(&script, r#"return "not a table""#).unwrap();

        let err = LuaRunner::execute(&script, &ctx).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("must return"), "got: {msg}");
    }

    #[test]
    fn exec_function_works() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            local ok, code = exec("echo", {"hello"})
            assert(ok == true)
            assert(code == 0)
            return { include_dirs = {}, lib_dirs = {}, libs = {} }
            "#,
        )
        .unwrap();

        LuaRunner::execute(&script, &ctx).unwrap();
    }

    #[test]
    fn exec_failure_raises_error() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            exec("false", {})
            return { include_dirs = {}, lib_dirs = {}, libs = {} }
            "#,
        )
        .unwrap();

        let err = LuaRunner::execute(&script, &ctx).unwrap_err();
        assert!(err.to_string().contains("failed"));
    }

    #[test]
    fn exec_ignore_errors() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            local ok, code = exec("false", {}, { ignore_errors = true })
            assert(ok == false)
            assert(code ~= 0)
            return { include_dirs = {}, lib_dirs = {}, libs = {} }
            "#,
        )
        .unwrap();

        LuaRunner::execute(&script, &ctx).unwrap();
    }

    #[test]
    fn mkdir_and_copy() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        std::fs::write(ctx.src_dir.join("hello.txt"), "hello").unwrap();

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            mkdir(out .. "/include")
            copy("hello.txt", out .. "/include/hello.txt")
            return { include_dirs = { out .. "/include" }, lib_dirs = {}, libs = {} }
            "#,
        )
        .unwrap();

        let result = LuaRunner::execute(&script, &ctx).unwrap();
        assert_eq!(result.include_dirs.len(), 1);
        assert!(ctx.out_dir.join("include/hello.txt").exists());
    }

    #[test]
    fn glob_function() {
        let tmp = TempDir::new().unwrap();
        let ctx = test_context(&tmp);

        std::fs::write(ctx.src_dir.join("a.h"), "").unwrap();
        std::fs::write(ctx.src_dir.join("b.h"), "").unwrap();
        std::fs::write(ctx.src_dir.join("c.cpp"), "").unwrap();

        let script = tmp.path().join("build.lua");
        std::fs::write(
            &script,
            r#"
            local headers = glob(src .. "/*.h")
            assert(#headers == 2, "expected 2 headers, got " .. #headers)
            return { include_dirs = {}, lib_dirs = {}, libs = {} }
            "#,
        )
        .unwrap();

        LuaRunner::execute(&script, &ctx).unwrap();
    }

    #[test]
    fn script_hash() {
        let tmp = TempDir::new().unwrap();
        let script = tmp.path().join("test.lua");
        std::fs::write(&script, "return {}").unwrap();

        let hash = compute_script_hash(&script).unwrap();
        assert!(hash.starts_with("sha256:"));

        let hash2 = compute_script_hash(&script).unwrap();
        assert_eq!(hash, hash2);
    }
}
