# TASK-026: Cross-Compilation and Toolchain Commands

- **Related Requirements**: REQ-TOOL-001, REQ-TOOL-002, REQ-TOOL-004
- **Milestone**: M5 — Advanced Build
- **Size**: M
- **Dependencies**: TASK-005, TASK-006

## Summary

Implement `--target <triple>` cross-compilation and `ordo toolchain list`.

## Implementation Steps

1. Implement target triple parsing:
   - Parse `<arch>-<os>-<abi>` format
   - Validate known architectures, OS, and ABI combinations
2. Implement `[target.<triple>]` config loading:
   - Project Ordo.toml → global config.toml fallback
   - Extract: compiler, sysroot, linker per target
3. Extend compiler abstraction for cross-compilation:
   - Inject `--target <triple>` for Clang
   - Inject `--sysroot=<path>` when configured
   - Select appropriate linker
4. Extend Ninja generator:
   - Cross-compiled artifacts in `target/<triple>/<profile>/`
5. Implement `ordo toolchain list` in `src/cli/toolchain.rs`:
   - Scan PATH for all supported compilers
   - Display: name, version, path
   - Mark which compiler is currently selected
6. Write tests:
   - Target triple parsing
   - Config resolution for cross targets
   - Correct artifact placement

## Target Files

- `src/cli/toolchain.rs`
- `src/backend/compiler/mod.rs` (extend)
- `src/backend/ninja.rs` (extend)
