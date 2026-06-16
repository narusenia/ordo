# TASK-010: Passive Providers (pkg-config, system)

- **Related Requirements**: REQ-DEPS-007, REQ-DEPS-008
- **Milestone**: M2 — Dependency Management
- **Size**: M
- **Dependencies**: TASK-008

## Summary

Implement the pkg-config and system provider backends.

## Implementation Steps

1. Implement `PkgConfigProvider` in `src/backend/provider/pkgconfig.rs`:
   - Execute `pkg-config --cflags --libs <name>`
   - Parse output into include paths and link flags
   - Handle versioned queries: `pkg-config --atleast-version=<ver> <name>`
   - Error with clear message if package not found
2. Implement `SystemProvider` in `src/backend/provider/system.rs`:
   - Generate `-l<name>` linker flag
   - No include path injection (uses compiler defaults)
   - Errors manifest at link time; provide upfront check where feasible
3. Integrate both providers into the build pipeline:
   - Provider output → `BuildFlags` struct → injected into `build.ninja`
4. Write tests:
   - Mock pkg-config output for unit tests
   - Integration test with a real system library (e.g., `zlib`)

## Target Files

- `src/backend/provider/pkgconfig.rs`
- `src/backend/provider/system.rs`
