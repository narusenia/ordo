# TASK-014: Dependency CLI Commands (add, update, tree)

- **Related Requirements**: REQ-DEPS-009
- **Milestone**: M2 — Dependency Management
- **Size**: M
- **Dependencies**: TASK-008, TASK-009

## Summary

Implement `ordo add`, `ordo update`, and `ordo tree` commands.

## Implementation Steps

1. Implement `ordo add <name>` in `src/cli/add.rs`:
   - Parse name and optional version/provider from CLI
   - Modify `Ordo.toml` programmatically (preserve formatting where possible, using `toml_edit` crate)
   - Trigger resolution to validate the addition
   - Update `Ordo.lock`
2. Implement `ordo update` in `src/cli/update.rs`:
   - Re-resolve all dependencies within SemVer constraints
   - `ordo update <name>`: re-resolve only the named dependency
   - Rewrite `Ordo.lock`
   - Report changes (old version → new version)
3. Implement `ordo tree` in `src/cli/tree.rs`:
   - Print dependency tree with indentation
   - Show: name, version, provider/source
   - Mark direct vs transitive dependencies
   - Detect and display duplicate versions
4. Add `toml_edit` to dependencies for `Ordo.toml` modification
5. Write integration tests for each command

## Target Files

- `src/cli/add.rs`
- `src/cli/update.rs`
- `src/cli/tree.rs`
