# TASK-015: Workspace Discovery and Configuration

- **Related Requirements**: REQ-WORK-001, REQ-WORK-002, REQ-WORK-003
- **Milestone**: M3 — Workspace & Profiles
- **Size**: L
- **Dependencies**: TASK-002

## Summary

Implement workspace member discovery, shared dependency resolution, and toolchain inheritance.

## Implementation Steps

1. Extend `Manifest` to parse `[workspace]` section:
   - `members` with glob pattern expansion
   - `exclude` patterns
   - `[workspace.dependencies]`
   - Allow `[workspace]` + `[package]` co-existence
2. Implement workspace discovery in `src/core/workspace.rs`:
   - Expand globs against filesystem
   - Apply exclude filters
   - Load each member's `Ordo.toml`
   - Validate member references (`{ workspace = true }` matches workspace.dependencies)
3. Implement toolchain/language inheritance:
   - Workspace root settings as defaults
   - Member overrides take precedence
4. Implement shared dependency resolution:
   - Members using `{ workspace = true }` inherit exact specifier
   - Members cannot override workspace dependency version
   - Merged dependency graph across all members
5. Build inter-member dependency DAG from path dependencies
6. Write tests:
   - Glob expansion
   - Exclude filtering
   - Workspace dependency inheritance
   - Toolchain override

## Target Files

- `src/core/manifest.rs` (extend)
- `src/core/workspace.rs`
