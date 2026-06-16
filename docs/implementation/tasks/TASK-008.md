# TASK-008: Dependency Declaration and Resolver

- **Related Requirements**: REQ-DEPS-001, REQ-DEPS-003
- **Milestone**: M2 — Dependency Management
- **Size**: L
- **Dependencies**: TASK-002

## Summary

Implement dependency declaration parsing in `Ordo.toml` and the PubGrub-based version resolver.

## Implementation Steps

1. Extend `Manifest` to parse `[dependencies]` and `[dev-dependencies]`:
   - All specifier forms: version string, path, git (tag/branch/rev), provider, optional, workspace ref
   - Validate that provider is explicit (error if ambiguous)
2. Add `pubgrub` crate dependency
3. Implement `Resolver` in `src/core/resolver.rs`:
   - Implement `pubgrub::DependencyProvider` trait
   - Map Ordo version requirements to PubGrub constraints
   - Handle `^` (default), `~`, `=`, `>=`, `>`, `<`, `<=` operators
   - Resolve transitive dependencies
   - Produce `ResolvedGraph`: ordered list of resolved packages with exact versions
4. Implement conflict error reporting:
   - Show the full conflict chain when resolution fails
   - Include source location in `Ordo.toml`
5. Define `Provider` trait in `src/backend/provider/mod.rs`:
   - `resolve()`, `fetch()`, `build_flags()`
   - Stub implementations for each provider
6. Write tests:
   - Simple resolution
   - Transitive resolution
   - Conflict detection and error messages

## Target Files

- `src/core/manifest.rs` (extend)
- `src/core/resolver.rs`
- `src/backend/provider/mod.rs`
