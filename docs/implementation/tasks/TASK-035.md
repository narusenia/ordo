# TASK-035: CI Command and CI Template Generation

- **Related Requirements**: REQ-SEC-005 (no build scripts context), REQ-PROJ-005 (CI steps)
- **Milestone**: M6 — Packaging & Ecosystem
- **Size**: M
- **Dependencies**: TASK-006, TASK-020, TASK-021, TASK-022

## Summary

Implement `ordo ci` and `ordo generate github-actions` / `ordo generate gitlab-ci`.

## Implementation Steps

1. Implement `ordo ci` in `src/cli/ci.rs`:
   - Read `[ci] steps` from Ordo.toml (or use defaults)
   - Default steps: `fmt --check`, `lint`, `build`, `test`, `build --release`
   - Execute each step sequentially
   - Implicitly apply `--locked`
   - Default: stop on first failure
   - `--keep-going`: run all steps, report all failures at end
   - Report summary: passed/failed steps with timing
2. Implement `ordo generate github-actions`:
   - Generate `.github/workflows/ci.yml`
   - Matrix: configurable OS (ubuntu-latest, macos-latest, windows-latest)
   - Steps: install Ordo, `ordo ci`
   - Cache `target/` directory
3. Implement `ordo generate gitlab-ci`:
   - Generate `.gitlab-ci.yml`
   - Similar structure to GitHub Actions variant
4. `--force` flag to overwrite existing CI files
5. Write tests

## Target Files

- `src/cli/ci.rs`
- `src/cli/generate.rs` (extend)
