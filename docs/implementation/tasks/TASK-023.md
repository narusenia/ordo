# TASK-023: Check Command

- **Related Requirements**: REQ-BUILD-010
- **Milestone**: M4 — Testing & Quality
- **Size**: S
- **Dependencies**: TASK-005

## Summary

Implement `ordo check` — syntax/type checking without producing binaries.

## Implementation Steps

1. Implement `src/cli/check.rs`:
   - Generate `build.ninja` with compiler's syntax-only flag
   - Clang/GCC: `-fsyntax-only`
   - MSVC: `/Zs`
   - Invoke Ninja → only compile checks, no linking
   - Report errors in same format as `ordo build`
2. Should be noticeably faster than `ordo build` (no linking, no object file output)
3. Write tests comparing `check` vs `build` behavior on error cases

## Target Files

- `src/cli/check.rs`
- `src/backend/ninja.rs` (add syntax-only rule variant)
