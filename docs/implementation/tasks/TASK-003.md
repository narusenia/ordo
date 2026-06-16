# TASK-003: Error System

- **Related Requirements**: REQ-CLI-002, REQ-CLI-003
- **Milestone**: M1 — Foundation + MVP
- **Size**: M
- **Dependencies**: TASK-001

## Summary

Implement the structured error system with error codes, source-span display, and help hints.

## Implementation Steps

1. Define error types in `src/error/mod.rs`:
   - `OrdoError` enum: `Config`, `Deps`, `Build`, `Toolchain`, `Test`
   - Each variant implements `miette::Diagnostic` with error code, source span, help text
2. Define error codes in `src/error/codes.rs`:
   - `E00xx`: Config/manifest errors
   - `E01xx`: Dependency resolution errors
   - `E02xx`: Build errors
   - `E03xx`: Toolchain errors
   - `E04xx`: Test errors
3. Implement `miette` integration:
   - `Ordo.toml` errors display the TOML source with line/column highlighting
   - `= help:` lines with actionable suggestions
4. Implement color control:
   - `--color never/always/auto`
   - `ORDO_COLOR=0` environment variable
   - Detect TTY for auto mode
5. Write tests for error display formatting

## Target Files

- `src/error/mod.rs`
- `src/error/codes.rs`
