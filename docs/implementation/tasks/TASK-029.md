# TASK-029: Security — Integrity Verification and CI Flags

- **Related Requirements**: REQ-SEC-001, REQ-SEC-002
- **Milestone**: M5 — Advanced Build
- **Size**: M
- **Dependencies**: TASK-009

## Summary

Implement hash verification for dependencies and `--locked`/`--frozen` CI flags.

## Implementation Steps

1. Implement SHA-256 verification in build pipeline:
   - On dependency fetch: compute hash of downloaded content
   - Compare against `Ordo.lock` checksum → mismatch = hard error
   - Provider dependencies (vcpkg/conan): hash the installed package contents
2. Implement `--locked` flag:
   - Compare `Ordo.lock` against `Ordo.toml` for consistency
   - If lock file needs update → error (not auto-update)
   - Ensure deterministic builds from lock file
3. Implement `--frozen` flag:
   - Disable all network access during build
   - Use only cached/local dependencies
   - Error on any operation that would require network
4. Integrate `--locked` with `ordo ci`:
   - `ordo ci` implicitly applies `--locked`
5. Write tests:
   - Hash mismatch detection
   - `--locked` with stale lock file
   - `--frozen` with missing cache

## Target Files

- `src/util/hash.rs` (extend)
- `src/core/lockfile.rs` (extend)
- `src/cli/build.rs` (extend)
