# TASK-036: Toolchain Install (Future Foundation)

- **Related Requirements**: REQ-TOOL-003
- **Milestone**: M6 — Packaging & Ecosystem
- **Size**: S
- **Dependencies**: TASK-026

## Summary

Lay the groundwork for `ordo toolchain install` — Ordo-managed compiler installations.

## Implementation Steps

1. Implement `ordo toolchain install` stub in `src/cli/toolchain.rs`:
   - Planned: download and manage compiler installations under Ordo's cache directory
   - For now: print "not yet implemented" with a description of planned functionality
2. Define the toolchain storage layout:
   - `<cache_dir>/toolchains/<compiler>-<version>/`
   - Metadata file tracking installed toolchains
3. Document the planned design in code comments for future implementation
4. Ensure `ordo toolchain list` works alongside future install

## Target Files

- `src/cli/toolchain.rs` (extend)
