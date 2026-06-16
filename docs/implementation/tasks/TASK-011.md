# TASK-011: vcpkg Provider (Active)

- **Related Requirements**: REQ-DEPS-005
- **Milestone**: M2 — Dependency Management
- **Size**: L
- **Dependencies**: TASK-008

## Summary

Implement the vcpkg active provider that auto-installs packages.

## Implementation Steps

1. Implement `VcpkgProvider` in `src/backend/provider/vcpkg.rs`:
   - Locate vcpkg: `VCPKG_ROOT` env → Ordo-managed instance at cache dir
   - If Ordo-managed: bootstrap vcpkg on first use (clone + bootstrap script)
   - Generate `vcpkg.json` manifest internally (not written to project root)
   - Execute `vcpkg install` with the manifest
   - Parse installed package: extract include dirs from `installed/<triplet>/include/`, lib dirs from `installed/<triplet>/lib/`
   - Map to `BuildFlags`
2. Handle vcpkg triplet selection:
   - Map Ordo target triple → vcpkg triplet (e.g., `x64-linux`, `arm64-osx`)
   - Default to host triplet when no cross-compilation
3. Handle version constraints:
   - vcpkg's version database → select compatible version
   - Pin exact version in `Ordo.lock`
4. Error handling:
   - vcpkg not found + cannot bootstrap → error with install instructions
   - Package not found in vcpkg → error suggesting `vcpkg search`
5. Write tests with mocked vcpkg subprocess

## Target Files

- `src/backend/provider/vcpkg.rs`
