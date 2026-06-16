# TASK-031: Package Command

- **Related Requirements**: REQ-PKG-002
- **Milestone**: M6 — Packaging & Ecosystem
- **Size**: S
- **Dependencies**: TASK-006

## Summary

Implement `ordo package` — create distributable archives.

## Implementation Steps

1. Implement `src/cli/package.rs`:
   - Build with release profile
   - Collect: binaries, headers (for libraries), LICENSE, README
   - Create `.tar.gz` archive (Linux/macOS default)
   - Create `.zip` archive (Windows default, or `--format zip`)
   - Output to `target/package/<name>-<version>-<target>.<ext>`
2. Archive naming: `<name>-<version>-<target-triple>.<ext>`
3. Verify archive contents are correct
4. Write tests

## Target Files

- `src/cli/package.rs`
