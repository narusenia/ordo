# TASK-009: Lock File

- **Related Requirements**: REQ-DEPS-004
- **Milestone**: M2 — Dependency Management
- **Size**: M
- **Dependencies**: TASK-008

## Summary

Implement `Ordo.lock` generation, reading, verification, and update logic.

## Implementation Steps

1. Define lock file schema in `src/core/lockfile.rs`:
   - `version` field (start at 1)
   - `[[package]]` entries: name, version, source, checksum
   - Source format: `registry+<url>`, `git+<url>#<commit>`, `vcpkg`, `conan`, `pkg-config`, `system`, `path+<rel>`
2. Implement lock file writer:
   - Serialize resolved graph to TOML
   - Compute SHA-256 hash for each dependency source
   - Pin git dependencies to exact commit hash
3. Implement lock file reader:
   - Parse existing `Ordo.lock`
   - Compare against current resolution → detect staleness
4. Integrate with build pipeline:
   - If `Ordo.lock` exists and is fresh → use locked versions
   - If `Ordo.lock` is stale → re-resolve and update
   - If `Ordo.lock` does not exist → resolve and create
5. Implement `ordo update`:
   - Re-resolve all dependencies within SemVer constraints
   - `ordo update <name>`: re-resolve a single dependency
6. Hash verification:
   - On build, verify downloaded content matches lock file hash
   - Mismatch → hard error
7. Write tests for lock file round-trip and staleness detection

## Target Files

- `src/core/lockfile.rs`
- `src/util/hash.rs`
