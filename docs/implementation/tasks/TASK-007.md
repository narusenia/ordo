# TASK-007: Run and Clean Commands

- **Related Requirements**: REQ-BUILD-002, REQ-BUILD-009
- **Milestone**: M1 — Foundation + MVP
- **Size**: S
- **Dependencies**: TASK-006

## Summary

Implement `ordo run` (build + execute) and `ordo clean` (remove target/).

## Implementation Steps

1. Implement `ordo run` in `src/cli/run.rs`:
   - Trigger build (reuse build pipeline from TASK-006)
   - Locate the built binary in `target/<profile>/`
   - Execute binary as a child process
   - Forward arguments after `--` to the binary
   - Error if project type is not `executable`
   - Forward binary's exit code as Ordo's exit code
2. Implement `ordo clean` in `src/cli/clean.rs`:
   - Delete `target/` directory
   - `--cache` flag: additionally clear external cache (invoke `ccache -C` or `sccache --stop-server`)
   - Do not delete `Ordo.toml`, `Ordo.lock`, or source files
3. Write integration tests:
   - `ordo run` produces expected stdout from the built program
   - `ordo run -- args` passes arguments correctly
   - `ordo clean` removes target/ and subsequent build is from scratch

## Target Files

- `src/cli/run.rs`
- `src/cli/clean.rs`
