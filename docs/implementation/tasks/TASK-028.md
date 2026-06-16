# TASK-028: Watch Mode and Cache Integration

- **Related Requirements**: REQ-CLI-006, cache from REQ-BUILD
- **Milestone**: M5 — Advanced Build
- **Size**: M
- **Dependencies**: TASK-006

## Summary

Implement `ordo watch` and build cache (ccache/sccache) integration.

## Implementation Steps

1. Add `notify` crate dependency
2. Implement `src/cli/watch.rs`:
   - `ordo watch build`, `ordo watch test`, `ordo watch run`
   - Watch: `src/`, `include/`, `tests/`, `Ordo.toml`
   - Exclude: `target/`, `.git/`
   - Custom include/exclude from `[watch]` config
   - Debounce: 300ms after last filesystem event
   - `ordo watch run`: kill previous process → rebuild → relaunch
   - `--notify` flag: desktop notification on build result (via `notify-rust` crate or similar)
3. Implement cache integration in `src/backend/cache.rs`:
   - Auto-detect: check PATH for sccache → ccache
   - `[cache] tool = "auto"|"ccache"|"sccache"|"none"`
   - Wrap compiler commands in build.ninja: `sccache clang++` or `ccache clang++`
   - `--no-cache` CLI flag: skip cache wrapper
4. Write tests:
   - Cache wrapper in generated build.ninja
   - Watch debounce logic (unit test)

## Target Files

- `src/cli/watch.rs`
- `src/backend/cache.rs`
- `src/backend/ninja.rs` (extend for cache wrapping)
