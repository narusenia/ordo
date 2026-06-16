# TASK-017: Full Build Profile Support

- **Related Requirements**: REQ-BUILD-004 (full), REQ-BUILD-005
- **Milestone**: M3 — Workspace & Profiles
- **Size**: M
- **Dependencies**: TASK-006

## Summary

Extend build profiles beyond dev/release to include all options and custom profiles with `inherits`.

## Implementation Steps

1. Extend profile parsing to all fields:
   - Code generation: `pic`, `rtti`, `exceptions`, `warnings`
   - Linking: `linker`, `static-runtime`
   - Debug/analysis: `coverage`, `split-debug`
   - Performance: `pch`, `unity`, `parallel`
2. Implement `inherits` resolution:
   - Custom profile inherits all unset values from base
   - Chain inheritance (custom → release → defaults)
   - Detect circular inheritance → error
3. Implement `bench` profile:
   - Implicit: inherits `release`, debug=true
4. Map all profile options to compiler/linker flags per compiler:
   - Clang, GCC, MSVC flag mappings for each option
5. Implement `ordo build --profile <name>`
6. Write tests:
   - Inheritance chain resolution
   - Flag generation for each profile option
   - Custom profile with overrides

## Target Files

- `src/core/manifest.rs` (extend profile parsing)
- `src/backend/compiler/clang.rs` (extend flag mapping)
- `src/backend/compiler/gcc.rs` (extend)
- `src/backend/compiler/msvc.rs` (extend)
