# TASK-030: Install Command with pkg-config/CMake Config Generation

- **Related Requirements**: REQ-PKG-001, REQ-PKG-004, REQ-PKG-005
- **Milestone**: M6 — Packaging & Ecosystem
- **Size**: M
- **Dependencies**: TASK-006

## Summary

Implement `ordo install` with automatic generation of pkg-config and CMake find-package files.

## Implementation Steps

1. Implement `src/cli/install.rs`:
   - Build with release profile (or specified profile)
   - Install binaries to `<prefix>/bin/`
   - Install headers to `<prefix>/include/` (from `[install] headers` patterns)
   - Install libraries to `<prefix>/lib/`
   - Default prefix: `/usr/local/` (configurable via `--prefix`)
2. Implement pkg-config `.pc` generation:
   - Template: Name, Description, Version from `[package]`
   - Cflags: `-I${includedir}`
   - Libs: `-L${libdir} -l<name>`
   - Install to `<prefix>/lib/pkgconfig/`
3. Implement CMake config generation:
   - `<Name>Config.cmake`: define imported target
   - `<Name>ConfigVersion.cmake`: version compatibility check
   - Install to `<prefix>/lib/cmake/<Name>/`
4. Only generate pkg-config/CMake config for library projects (not executables)
5. Write tests

## Target Files

- `src/cli/install.rs`
