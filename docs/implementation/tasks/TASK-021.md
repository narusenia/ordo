# TASK-021: Format Command

- **Related Requirements**: REQ-QUAL-001
- **Milestone**: M4 — Testing & Quality
- **Size**: S
- **Dependencies**: TASK-001

## Summary

Implement `ordo fmt` using clang-format.

## Implementation Steps

1. Implement `src/cli/fmt.rs`:
   - Discover C/C++ source files in `src/`, `include/`, `tests/`
   - Invoke `clang-format -i` on each file (default: rewrite)
   - `--check` flag: invoke `clang-format --dry-run --Werror`, report differences
   - Return non-zero exit code on `--check` failure
2. Config resolution:
   - Use `.clang-format` if present in project root
   - If absent, write Ordo's built-in default style to a temp file and pass to clang-format
3. Read `[fmt]` config for tool and style overrides
4. Error if clang-format is not found on PATH
5. Write tests

## Target Files

- `src/cli/fmt.rs`
