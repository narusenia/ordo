# TASK-002: Manifest Parser and Config System

- **Related Requirements**: REQ-PROJ-003, REQ-PROJ-004, REQ-CLI-004
- **Milestone**: M1 — Foundation + MVP
- **Size**: L
- **Dependencies**: TASK-001

## Summary

Implement `Ordo.toml` parsing, validation, and the layered configuration merge system.

## Implementation Steps

1. Define `Manifest` struct in `src/core/manifest.rs` with serde derive:
   - `Package`, `Language`, `Toolchain` sub-structs
   - Validation logic: required fields, enum validation, SemVer format
2. Implement TOML parsing with span tracking for error reporting:
   - Use `toml::Spanned` or raw span tracking for `miette` integration
   - Parse errors show line/column with underline
3. Implement `ConfigMerger` in `src/core/config.rs`:
   - Load from: CLI args, environment vars, project Ordo.toml, .ordo/config.toml, workspace root, global config, defaults
   - Produce `ResolvedConfig` with origin tracking per field
   - Environment variable mapping: `ORDO_TOOLCHAIN_COMPILER` → `[toolchain] compiler`
4. Write tests:
   - Valid manifest parsing
   - Invalid manifest error messages with source spans
   - Config precedence ordering
   - Environment variable override

## Target Files

- `src/core/mod.rs`
- `src/core/manifest.rs`
- `src/core/config.rs`
