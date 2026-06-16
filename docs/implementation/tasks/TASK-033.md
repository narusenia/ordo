# TASK-033: IDE Generation and CMake Compatibility Commands

- **Related Requirements**: REQ-IDE-002, REQ-IDE-003, REQ-IDE-004, REQ-IDE-005
- **Milestone**: M6 — Packaging & Ecosystem
- **Size**: M
- **Dependencies**: TASK-005, TASK-006

## Summary

Implement `ordo generate` subcommands and `ordo import cmake`.

## Implementation Steps

1. Implement `ordo generate cmake` in `src/cli/generate.rs`:
   - Generate `CMakeLists.txt` from `Ordo.toml`
   - Map: package → project(), type → add_executable/add_library, deps → find_package/target_link_libraries
   - Include profile settings as CMake options
2. Implement `ordo generate presets`:
   - Generate `CMakePresets.json` with configure/build presets matching Ordo profiles
3. Implement `ordo generate vscode`:
   - Generate `.vscode/c_cpp_properties.json` (include paths, defines, compiler)
   - Generate `.vscode/tasks.json` (ordo build, test tasks)
   - `--force` to overwrite existing files
4. Implement `ordo generate clion`:
   - Leverage CMake output + presets for CLion support
5. Implement `ordo generate clangd`:
   - Generate `.clangd` configuration file
6. Implement `ordo import cmake` in `src/cli/import.rs`:
   - Parse `CMakeLists.txt` for common patterns:
     - `project()`, `add_executable()`, `add_library()`, `target_link_libraries()`
     - `target_include_directories()`, `find_package()`, `set(CMAKE_CXX_STANDARD ...)`
   - Generate `Ordo.toml` from parsed information
   - Unconvertible constructs → `# TODO: manual conversion needed` comments in output
   - Print migration summary
7. Write tests for each generate variant

## Target Files

- `src/cli/generate.rs`
- `src/cli/import.rs`
