# TASK-022: Lint Command and compile_commands.json

- **Related Requirements**: REQ-QUAL-002, REQ-QUAL-004
- **Milestone**: M4 — Testing & Quality
- **Size**: M
- **Dependencies**: TASK-006

## Summary

Implement `ordo lint` using clang-tidy, with on-demand compile_commands.json generation.

## Implementation Steps

1. Ensure `compile_commands.json` generation in the build pipeline:
   - Generated at project root during `ordo build`
   - Implement standalone generation for `ordo lint` when no prior build exists
2. Implement `src/cli/lint.rs`:
   - Discover source files
   - Invoke `clang-tidy` with `-p .` (compile_commands.json at root)
   - `--fix` flag: pass `--fix` to clang-tidy
   - Report clang-tidy output as-is (passthrough)
   - Return non-zero exit code on warnings/errors
3. Config resolution:
   - Use `.clang-tidy` if present
   - If absent, Ordo built-in defaults
4. Read `[lint]` config for tool and config overrides
5. Error if clang-tidy is not found on PATH
6. Write tests

## Target Files

- `src/cli/lint.rs`
- `src/backend/ninja.rs` (ensure compile_commands.json generation)
