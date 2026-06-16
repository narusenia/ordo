# TASK-001: Project Skeleton and CLI Framework

- **Related Requirements**: REQ-CLI-001 (partial), REQ-CLI-005
- **Milestone**: M1 — Foundation + MVP
- **Size**: M
- **Dependencies**: None

## Summary

Initialize the Rust project with `cargo init`, set up the CLI framework with clap, and implement the global configuration path resolution.

## Implementation Steps

1. `cargo init --name ordo` with edition 2021
2. Add core dependencies to `Cargo.toml`: `clap` (derive), `toml`, `serde`, `miette`, `owo-colors`, `tracing`, `tracing-subscriber`, `dirs`
3. Define the top-level CLI structure in `src/cli/mod.rs` using clap derive:
   - Subcommands: `New`, `Init`, `Build`, `Run`, `Clean` (MVP subset)
   - Global flags: `--color`, `-v`/`-vv`, `--version`
4. Implement OS-native path resolution in `src/util/paths.rs`:
   - Config dir, cache dir, credentials path
   - `ORDO_HOME` override
   - Platform-specific defaults using `dirs` crate
5. Implement `tracing` setup with verbosity levels
6. Stub out each command handler to print "not yet implemented"
7. Write unit tests for path resolution across platforms

## Target Files

- `Cargo.toml`
- `src/main.rs`
- `src/cli/mod.rs`
- `src/util/paths.rs`
- `src/util/mod.rs`
