# TASK-005: Compiler Abstraction and Ninja Generator

- **Related Requirements**: REQ-BUILD-003, REQ-BUILD-006
- **Milestone**: M1 — Foundation + MVP
- **Size**: L
- **Dependencies**: TASK-002, TASK-003

## Summary

Implement the compiler abstraction trait and the core Ninja file generator.

## Implementation Steps

1. Define `Compiler` trait in `src/backend/compiler/mod.rs`:
   - `compile_cmd()`: generate compilation command for a single translation unit
   - `link_cmd()`: generate link command
   - `detect()`: find compiler on PATH, parse version
   - `syntax_only_flag()`: for `ordo check`
2. Implement `ClangCompiler` in `src/backend/compiler/clang.rs`:
   - Map Ordo options to Clang flags (-std=, -O, -g, -fsanitize=, etc.)
   - depfile generation (-MD -MF)
3. Implement `GccCompiler` in `src/backend/compiler/gcc.rs`:
   - Same mapping for GCC flags
4. Implement `MsvcCompiler` in `src/backend/compiler/msvc.rs`:
   - Map to MSVC flags (/std:, /O, /Zi, /fsanitize=, etc.)
5. Implement Ninja writer in `src/backend/ninja.rs`:
   - Write `build.ninja` with: rules (compile, link), build statements, depfile tracking
   - Generate `compile_commands.json` simultaneously
6. Implement compiler auto-detection: scan PATH for clang → gcc → cl.exe
7. Write tests:
   - Correct flag generation per compiler
   - Valid `build.ninja` output
   - Compiler detection logic

## Target Files

- `src/backend/mod.rs`
- `src/backend/compiler/mod.rs`
- `src/backend/compiler/clang.rs`
- `src/backend/compiler/gcc.rs`
- `src/backend/compiler/msvc.rs`
- `src/backend/ninja.rs`
