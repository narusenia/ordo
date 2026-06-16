# REQ-TOOL — Toolchain Management

## REQ-TOOL-001: Compiler Auto-Detection

- **Priority**: Must
- **Status**: Draft
- **Description**: When `[toolchain] compiler` is omitted, Ordo auto-detects a compiler from PATH with priority: Clang > GCC > MSVC.
- **Acceptance Criteria**:
  - [ ] Searches PATH for supported compilers
  - [ ] Priority order: clang/clang++ → gcc/g++ → cl.exe
  - [ ] Detects compiler version via `--version` or equivalent
  - [ ] Errors with actionable message if no supported compiler is found
  - [ ] `ordo doctor` displays detected compiler and version

## REQ-TOOL-002: Cross-Compilation

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo build --target <triple>` cross-compiles for the specified target using Clang/GCC-compatible target triples.
- **Acceptance Criteria**:
  - [ ] Target triples follow `<arch>-<os>-<abi>` format (e.g., `aarch64-linux-gnu`)
  - [ ] Per-target configuration via `[target.<triple>]` in `Ordo.toml` (compiler, sysroot, linker)
  - [ ] Falls back to global config (`~/.ordo/config.toml` or OS-appropriate path) for target settings
  - [ ] Cross-compiled artifacts placed in `target/<triple>/<profile>/`
  - [ ] Host builds (no `--target`) use `target/<profile>/` without triple in path

## REQ-TOOL-003: Toolchain Commands

- **Priority**: Could
- **Status**: Draft
- **Description**: CLI commands for inspecting and managing toolchains.
- **Acceptance Criteria**:
  - [ ] `ordo toolchain list` shows available compilers with versions and paths
  - [ ] `ordo toolchain install` (future) installs a compiler via Ordo-managed toolchains

## REQ-TOOL-004: Linker Selection

- **Priority**: Should
- **Status**: Draft
- **Description**: The linker can be explicitly selected per profile or globally.
- **Acceptance Criteria**:
  - [ ] `[toolchain] linker = "lld"` or per-profile `[profile.<name>] linker = "mold"`
  - [ ] Supported linkers: `lld`, `mold`, `gold`, `default` (system linker)
  - [ ] Correct flags per compiler: Clang (`-fuse-ld=lld`), GCC (`-fuse-ld=lld`), MSVC (link.exe)
  - [ ] Falls back to system default linker if not specified
