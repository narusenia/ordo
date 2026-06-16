# TASK-006: Build Command

- **Related Requirements**: REQ-BUILD-001, REQ-BUILD-004 (partial: dev/release)
- **Milestone**: M1 — Foundation + MVP
- **Size**: M
- **Dependencies**: TASK-005

## Summary

Implement `ordo build` — the core command that generates `build.ninja` and invokes Ninja.

## Implementation Steps

1. Implement build pipeline in `src/cli/build.rs`:
   - Load and validate `Ordo.toml`
   - Resolve configuration (merge layers)
   - Detect compiler
   - Discover source files in `src/`
   - Generate `build.ninja` in `target/<profile>/build/`
   - Generate `compile_commands.json` at project root
   - Invoke `ninja -C target/<profile>/build/`
   - Place final binary/library in `target/<profile>/`
2. Implement profile resolution (dev/release only for MVP):
   - `ordo build` → dev profile
   - `ordo build --release` → release profile
   - Apply profile options to compiler flags
3. Implement `--jobs <n>` → pass to `ninja -j <n>`
4. Handle Ninja process output:
   - Stream stdout/stderr to terminal
   - Compiler errors passed through unmodified
   - Set exit code based on Ninja result
5. Write integration tests:
   - Build a simple hello-world project
   - Verify binary is created at correct path
   - Verify incremental build (second build is faster)

## Target Files

- `src/cli/build.rs`
