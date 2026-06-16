# TASK-019: Dev Dependencies

- **Related Requirements**: REQ-DEPS-002
- **Milestone**: M3 — Workspace & Profiles
- **Size**: S
- **Dependencies**: TASK-008, TASK-020

## Summary

Implement `[dev-dependencies]` support — dependencies only included in test and benchmark builds.

## Implementation Steps

1. Extend manifest parsing for `[dev-dependencies]`
2. Extend resolver to include dev-deps only when building tests/benchmarks
3. Extend Ninja generator:
   - Test binaries link against dev-dependencies
   - Release builds exclude dev-dependencies
4. Extend `ordo tree` to distinguish dev-dependencies
5. Write tests

## Target Files

- `src/core/manifest.rs` (extend)
- `src/core/resolver.rs` (extend)
- `src/backend/ninja.rs` (extend)
