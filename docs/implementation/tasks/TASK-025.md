# TASK-025: C++ Modules Support

- **Related Requirements**: REQ-MOD-001, REQ-MOD-002, REQ-MOD-003, REQ-MOD-004
- **Milestone**: M5 — Advanced Build
- **Size**: L
- **Dependencies**: TASK-005, TASK-006

## Summary

Implement C++ modules as a first-class feature: dependency scanning, BMI management, and import std.

## Implementation Steps

1. Implement module scanner in `src/core/modules.rs`:
   - Rust-based parser to extract `export module`, `import`, `module` declarations from C++ source
   - Handle: named modules, module partitions, header units (future)
   - Build module dependency DAG
   - Detect circular dependencies → error
2. Implement fallback scanner:
   - Invoke `clang-scan-deps` when available
   - Parse its JSON output
   - Use as fallback when self-built parser is insufficient
3. Implement compiler version checking:
   - When `[modules] enabled = true`, verify compiler meets minimum: Clang 18+, GCC 14+, MSVC 17.5+
   - Error with clear message if below minimum
4. Extend Ninja generator for modules:
   - BMI generation rules per compiler
   - Clang: `-fmodule-file=`, `--precompile`
   - GCC: `-fmodules-ts`
   - MSVC: `/module:interface`, `/module:output`
   - Module dependency edges in build.ninja (BMI A must exist before compiling B that imports A)
   - BMIs stored in `target/<profile>/build/modules/`
5. Implement `import std` support:
   - Detect `[modules] import-std = true`
   - Generate std module BMI using compiler-specific mechanism
   - Must be built before any user module that uses `import std;`
6. Write tests:
   - Simple module project
   - Module dependency chain
   - import std (with compatible compiler)
   - Compiler version rejection

## Target Files

- `src/core/modules.rs`
- `src/backend/ninja.rs` (extend)
- `src/backend/compiler/clang.rs` (extend)
- `src/backend/compiler/gcc.rs` (extend)
- `src/backend/compiler/msvc.rs` (extend)
