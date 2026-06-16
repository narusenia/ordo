# TASK-013: Git Provider

- **Related Requirements**: REQ-DEPS-001 (git dependencies)
- **Milestone**: M2 — Dependency Management
- **Size**: L
- **Dependencies**: TASK-008, TASK-009

## Summary

Implement git dependency fetching: clone, checkout, build.

## Implementation Steps

1. Implement `GitProvider` in `src/backend/provider/git.rs`:
   - Clone repository using `gix` (pure Rust git)
   - Resolve tag/branch/rev to exact commit hash
   - Cache cloned repos in `<cache_dir>/git/`
   - Checkout specified ref
2. Implement build detection for git dependencies:
   - If `Ordo.toml` exists → build with Ordo (recursive)
   - If `CMakeLists.txt` exists → build with cmake + ninja
   - Neither → error with clear message
3. Extract build results:
   - Include paths from the dependency's `include/` or as declared
   - Library paths from the dependency's build output
   - Map to `BuildFlags`
4. Lock file integration:
   - Record `git+<url>#<commit_hash>` in `Ordo.lock`
   - On subsequent builds, verify cached clone matches locked commit
5. Implement shallow clone where possible for performance
6. Add `tokio` async for clone operations
7. Write tests:
   - Clone from local bare repo (for test isolation)
   - CMakeLists.txt fallback build
   - Lock file pinning

## Target Files

- `src/backend/provider/git.rs`
