# TASK-034: Doctor, Config Show, and Self Update

- **Related Requirements**: REQ-CLI-007, REQ-CLI-008
- **Milestone**: M6 — Packaging & Ecosystem
- **Size**: M
- **Dependencies**: TASK-001

## Summary

Implement diagnostic and self-management commands.

## Implementation Steps

1. Implement `ordo doctor` in `src/cli/doctor.rs`:
   - Detect and display: Ordo version, compiler(s), Ninja, sccache/ccache, vcpkg, conan, pkg-config, clang-format, clang-tidy
   - For each tool: name, version, path, check/cross mark
   - Categorize: required (compiler, ninja) vs optional (cache, providers, quality tools)
2. Implement `ordo config show` in `src/cli/config.rs`:
   - Display fully resolved configuration
   - `--origin` flag: annotate each value with its source (CLI, env, project, workspace, global, default)
3. Implement `ordo self update` in `src/cli/self_cmd.rs`:
   - Download latest release binary from configured release URL
   - Replace current binary
   - `--version <ver>`: install specific version
   - Platform-agnostic release source (configurable URL)
4. Implement update check:
   - On any command, check release API once per 24 hours
   - Cache last check timestamp in cache dir
   - Print one-line notice to stderr if newer version exists
   - `ORDO_NO_UPDATE_CHECK=1` disables
5. Write tests for doctor output parsing and config show

## Target Files

- `src/cli/doctor.rs`
- `src/cli/config.rs`
- `src/cli/self_cmd.rs`
