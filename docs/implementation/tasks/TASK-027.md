# TASK-027: Unity Build and Precompiled Headers

- **Related Requirements**: REQ-BUILD-007, REQ-BUILD-008
- **Milestone**: M5 — Advanced Build
- **Size**: M
- **Dependencies**: TASK-005

## Summary

Implement unity build and precompiled header support.

## Implementation Steps

1. Implement unity build in Ninja generator:
   - When `unity = true`: generate a combined `.cpp` file that `#include`s all source files
   - Compile the unity file instead of individual sources
   - Fallback to normal build on compilation failure (optional)
   - Per-profile override support
2. Implement precompiled headers:
   - When `pch = "include/pch.hpp"`: compile PCH before all other sources
   - Clang: `-x c++-header` to compile, `-include-pch` to use
   - GCC: compile to `.gch`, `-include` to use
   - MSVC: `/Yc` to create, `/Yu` to use
   - PCH stored in `target/<profile>/build/`
   - Add PCH as a dependency for all source compilations in build.ninja
3. Write tests:
   - Unity build produces correct binary
   - PCH accelerates build (timing test optional)
   - Per-profile PCH override

## Target Files

- `src/backend/ninja.rs` (extend)
- `src/backend/compiler/clang.rs` (extend)
- `src/backend/compiler/gcc.rs` (extend)
- `src/backend/compiler/msvc.rs` (extend)
