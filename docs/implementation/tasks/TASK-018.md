# TASK-018: Feature Flags

- **Related Requirements**: REQ-PROJ-006
- **Milestone**: M3 — Workspace & Profiles
- **Size**: M
- **Dependencies**: TASK-008

## Summary

Implement Cargo-style feature flags with conditional compilation and optional dependencies.

## Implementation Steps

1. Extend manifest parsing for `[features]` section:
   - `default` list
   - Feature → dependency mapping (`dep:<name>`)
   - Feature → feature dependencies
   - `[features.config] prefix`
2. Implement feature resolution in `src/core/features.rs`:
   - Resolve enabled features transitively
   - Handle `--features`, `--no-default-features`, `--all-features` CLI flags
   - Determine which optional dependencies are activated
3. Integrate with build pipeline:
   - For each enabled feature, inject `-D<PREFIX><NAME>=1` into compiler flags
   - Activate optional dependencies in the resolver
4. Integrate with dependency resolution:
   - Only resolve `optional = true` dependencies when their feature is enabled
   - `dep:<name>` in a feature list enables the optional dependency
5. Write tests:
   - Default features
   - Feature interdependency
   - Optional dependency activation
   - Custom prefix

## Target Files

- `src/core/features.rs`
- `src/core/manifest.rs` (extend)
- `src/core/resolver.rs` (extend)
