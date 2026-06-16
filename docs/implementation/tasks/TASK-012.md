# TASK-012: Conan Provider (Active) ✅

- **Related Requirements**: REQ-DEPS-006
- **Milestone**: M2 — Dependency Management
- **Size**: M
- **Dependencies**: TASK-008

## Summary

Implement the Conan active provider that auto-installs packages.

## Implementation Steps

1. Implement `ConanProvider` in `src/backend/provider/conan.rs`:
   - Detect Conan installation on PATH
   - Generate `conanfile.txt` internally
   - Execute `conan install` with appropriate generator (e.g., `PkgConfigDeps` or `CMakeDeps`)
   - Parse generator output for include paths and link flags
   - Map to `BuildFlags`
2. Handle Conan profiles:
   - Map Ordo profile (debug/release) to Conan settings (`build_type`)
   - Map compiler settings
3. Pin resolved version in `Ordo.lock`
4. Error handling:
   - Conan not installed → error with install instructions
   - Package not found → error with `conan search` suggestion
5. Write tests with mocked Conan subprocess

## Target Files

- `src/backend/provider/conan.rs`
