# TASK-013: Git Provider ✅

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

## Done (Phase 1)

- `GitProvider`: clone (bare) → fetch → checkout to worktree via system `git`
- Cache at `<cache_dir>/git/`, slug-based directory naming
- Resolve tag/branch/rev to commit hash, record `git+<url>#<short_hash>`
- `ordo add git:user/repo@tag` shorthand (GitHub default, `git:host.com/user/repo` for others)
- `ordo build` integration: clone → checkout → scan `include/` for headers
- 12 tests (mocked subprocess)

## Deferred (Phase 2)

- **Build integration for git deps**: currently only header-only libraries work (include path extraction). Building Ordo.toml or CMakeLists.txt sub-projects requires:
  - Ordo.toml deps: recursive NinjaGenerator invocation (feasible, NinjaGenerator is already modular)
  - CMakeLists.txt deps: cmake shell-out + install manifest parsing (medium complexity)
- **gix (pure Rust git)**: currently uses system `git` CLI. Migrating to gix removes the runtime dependency.
- **Lock file pinning**: record exact commit hash in Ordo.lock and verify on rebuild
- **Async clone**: tokio-based parallel fetching for multiple git deps

## Target Files

- `src/backend/provider/git.rs`
