# REQ-IDE — IDE & CMake Integration

## REQ-IDE-001: compile_commands.json

- **Priority**: Must
- **Status**: Draft
- **Description**: Auto-generated at the project root for clangd and IDE compatibility. (See also REQ-QUAL-004.)
- **Acceptance Criteria**:
  - [ ] Generated directly at the project root (no symlinks)
  - [ ] Updated on every build
  - [ ] Cross-platform: no symlink dependency (Windows compatible)

## REQ-IDE-002: Generate VSCode Configuration

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo generate vscode` creates VSCode-specific configuration files.
- **Acceptance Criteria**:
  - [ ] Generates `.vscode/c_cpp_properties.json` with include paths, defines, compiler path
  - [ ] Generates `.vscode/tasks.json` with `ordo build`, `ordo test` tasks
  - [ ] Does not overwrite existing files without `--force`

## REQ-IDE-003: Generate CLion Configuration

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo generate clion` produces configuration compatible with CLion.
- **Acceptance Criteria**:
  - [ ] Leverages `ordo generate cmake` output + CMakePresets for CLion native support
  - [ ] CLion can open the project with proper indexing via the generated CMake files

## REQ-IDE-004: Generate clangd Configuration

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo generate clangd` creates a `.clangd` configuration file.
- **Acceptance Criteria**:
  - [ ] Generates `.clangd` with compile flags database path and project-specific settings
  - [ ] Does not overwrite existing `.clangd` without `--force`

## REQ-IDE-005: CMake Compatibility Layer

- **Priority**: Should
- **Status**: Draft
- **Description**: Bidirectional (one-way each) CMake interoperability.
- **Acceptance Criteria**:
  - [ ] `ordo import cmake`: parses `CMakeLists.txt` and generates `Ordo.toml` (best-effort)
    - Supports: `project()`, `add_executable()`, `add_library()`, `target_link_libraries()`, `target_include_directories()`, `find_package()`, `set(CMAKE_CXX_STANDARD ...)`
    - Unsupported constructs → `# TODO: manual conversion needed` comments
    - Documented as a migration aid, not a complete converter
  - [ ] `ordo generate cmake`: generates `CMakeLists.txt` from `Ordo.toml`
  - [ ] `ordo generate presets`: generates `CMakePresets.json`
  - [ ] No bidirectional sync — each command is a one-way, one-time generation
