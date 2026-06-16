# REQ-QUAL — Code Quality

## REQ-QUAL-001: Format Command

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo fmt` formats source code using clang-format.
- **Acceptance Criteria**:
  - [ ] Default behavior: rewrites files in-place
  - [ ] `--check` flag: reports formatting differences without modifying files (CI mode)
  - [ ] Uses `.clang-format` from project root if present
  - [ ] Falls back to Ordo's built-in sensible defaults if no `.clang-format` exists
  - [ ] Processes all C/C++ sources in `src/`, `include/`, `tests/`
  - [ ] Configurable via `[fmt] tool = "clang-format"`, `style = ".clang-format"`
  - [ ] Returns non-zero exit code when `--check` finds differences

## REQ-QUAL-002: Lint Command

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo lint` runs clang-tidy for static analysis and linting.
- **Acceptance Criteria**:
  - [ ] Uses `compile_commands.json` (auto-generated; see REQ-IDE-001)
  - [ ] Generates `compile_commands.json` on-demand if not present (no prior `ordo build` required)
  - [ ] Uses `.clang-tidy` from project root if present; Ordo built-in defaults otherwise
  - [ ] `--fix` flag: applies clang-tidy auto-fixes
  - [ ] Reports diagnostics in clang-tidy's native format
  - [ ] Configurable via `[lint] tool = "clang-tidy"`, `config = ".clang-tidy"`

## REQ-QUAL-003: Analyze Command

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo analyze` runs deeper static analysis (future feature).
- **Acceptance Criteria**:
  - [ ] Placeholder command reserved for future implementation
  - [ ] Planned backends: clang-tidy deep checks, cppcheck
  - [ ] Clear message when invoked before implementation: "Not yet implemented"

## REQ-QUAL-004: compile_commands.json Generation

- **Priority**: Must
- **Status**: Draft
- **Description**: `compile_commands.json` is auto-generated at the project root for tooling compatibility.
- **Acceptance Criteria**:
  - [ ] Generated during `ordo build` at the project root (not inside `target/`)
  - [ ] Generated on-demand by `ordo lint` if not present
  - [ ] Contains correct entries for all translation units with flags, includes, and defines
  - [ ] Compatible with clangd, VSCode, Neovim, CLion
  - [ ] Updated on each build to reflect current configuration
