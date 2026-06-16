# TASK-004: Project Scaffolding (new / init)

- **Related Requirements**: REQ-PROJ-001, REQ-PROJ-002
- **Milestone**: M1 — Foundation + MVP
- **Size**: M
- **Dependencies**: TASK-002

## Summary

Implement `ordo new` and `ordo init` commands for project creation and initialization.

## Implementation Steps

1. Implement `ordo new <name>` in `src/cli/new.rs`:
   - Create project directory
   - Generate `Ordo.toml` with package metadata (name, version=0.1.0, type)
   - `ordo new <name>`: type=executable → `src/main.cpp` + `tests/main_test.cpp`
   - `ordo new <name> --lib`: type=static-library → `include/<name>/<name>.hpp` + `src/<name>.cpp` + `tests/<name>_test.cpp`
   - Generate `.gitignore` with `target/`
   - Run `git init` (skip with `--no-git`)
2. Implement `ordo init` in `src/cli/init.rs`:
   - Generate `Ordo.toml` in current directory
   - Detect existing layout: `main.cpp` → executable, otherwise → static-library
   - Error if `Ordo.toml` already exists
   - Do not create/modify source files
3. Template content:
   - `main.cpp`: minimal hello-world
   - Library header: include guard, namespace, placeholder function
   - Test file: minimal test depending on `[test] framework` default
4. Write integration tests: verify generated file structure and content

## Target Files

- `src/cli/new.rs`
- `src/cli/init.rs`
