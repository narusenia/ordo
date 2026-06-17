# REQ-LUA — Lua Build Scripts for Git Dependencies

## REQ-LUA-001: Lua Script Declaration

- **Priority**: Must
- **Status**: Draft
- **Description**: Git dependencies can specify a Lua build script via the `with` field. The script is executed after git clone/checkout to build the dependency and extract include/library paths. Only user-specified scripts are executed; no implicit script execution.
- **Acceptance Criteria**:
  - [ ] `with` field in dependency spec: `fmt = { git = "https://...", tag = "11.1.0", with = "scripts/fmt.lua" }`
  - [ ] `with` path is relative to the project root
  - [ ] File placement is unrestricted; `scripts/` directory is recommended by convention
  - [ ] `ordo add git:fmt/fmt --with scripts/fmt.lua` sets the `with` field in `Ordo.toml`
  - [ ] `with` is only valid on git dependencies; error on other dependency types
  - [ ] Missing script file produces a clear error with the expected path

## REQ-LUA-002: Lua Runtime

- **Priority**: Must
- **Status**: Draft
- **Description**: Lua scripts run in an embedded Lua 5.4 runtime via the `mlua` crate with sandbox and serde integration.
- **Acceptance Criteria**:
  - [ ] `mlua` crate with `lua54` and `serialize` features
  - [ ] `Lua::sandbox()` enabled — global table is read-only
  - [ ] Lua standard libraries `io`, `os`, `loadfile`, `dofile` are removed
  - [ ] Only ordo-provided API functions are available to scripts
  - [ ] Script return value is deserialized to a Rust struct via `mlua::serde`

## REQ-LUA-003: Script API — Context Variables

- **Priority**: Must
- **Status**: Draft
- **Description**: Ordo injects context variables into the Lua environment before script execution.
- **Acceptance Criteria**:
  - [ ] `src` — absolute path to the cloned source directory
  - [ ] `out` — absolute path to the build output directory (in global cache)
  - [ ] `target.os` — `"linux"`, `"macos"`, or `"windows"`
  - [ ] `target.arch` — `"x86_64"` or `"aarch64"`
  - [ ] `profile` — `"debug"` or `"release"`
  - [ ] `compiler.cc` — C compiler path
  - [ ] `compiler.cxx` — C++ compiler path
  - [ ] `compiler.id` — `"clang"`, `"gcc"`, or `"msvc"`

## REQ-LUA-004: Script API — exec()

- **Priority**: Must
- **Status**: Draft
- **Description**: `exec(command, args, opts)` executes an external command from within Lua scripts.
- **Acceptance Criteria**:
  - [ ] `exec("cmake", {"-B", "build"})` executes `cmake -B build` in the source directory
  - [ ] Non-zero exit code raises a Lua error by default (script stops)
  - [ ] `exec("cmd", args, {ignore_errors = true})` suppresses failure; returns `false, exit_code`
  - [ ] On success, returns `true, 0`
  - [ ] Command output is streamed to the terminal in real-time (consistent with vcpkg/conan output)
  - [ ] Verbose output (`-v`) shows the full command being executed

## REQ-LUA-005: Script API — File Operation Helpers

- **Priority**: Must
- **Status**: Draft
- **Description**: Cross-platform file operation helpers that are scoped to the source and output directories.
- **Acceptance Criteria**:
  - [ ] `copy(src_path, dst_path)` — copies a file or directory
  - [ ] `mkdir(path)` — creates a directory (including parents)
  - [ ] `glob(pattern)` — returns a list of paths matching the glob pattern
  - [ ] All path arguments are resolved relative to `src` (source dir)
  - [ ] Write operations outside `src` and `out` directories produce a Lua error
  - [ ] Read operations outside `src` and `out` directories produce a Lua error

## REQ-LUA-006: Script Return Value

- **Priority**: Must
- **Status**: Draft
- **Description**: Scripts return a Lua table with build output paths. Ordo uses these to generate Ninja build flags.
- **Acceptance Criteria**:
  - [ ] Script must end with `return { include_dirs = {...}, lib_dirs = {...}, libs = {...} }`
  - [ ] `include_dirs` — list of absolute paths to include directories
  - [ ] `lib_dirs` — list of absolute paths to library directories
  - [ ] `libs` — list of library names (without `lib` prefix or file extension)
  - [ ] Missing or malformed return value produces a clear error
  - [ ] Empty lists are valid (e.g., header-only libraries may have empty `libs`)

## REQ-LUA-007: Sandbox — Directory Scope

- **Priority**: Must
- **Status**: Draft
- **Description**: File operation helpers are restricted to the source and output directories. Network access is disabled by default.
- **Acceptance Criteria**:
  - [ ] `copy`, `mkdir`, `glob` only operate within `src` and `out` directories
  - [ ] Path traversal attempts (e.g., `../../../etc/passwd`) are caught and produce an error
  - [ ] No network access by default from Lua scripts
  - [ ] `--dangerously-allow-network-access` CLI flag permits network access during script execution
  - [ ] Ordo.toml does not persist network access permission (CLI flag only, per-invocation)

## REQ-LUA-008: Execution Timing and Caching

- **Priority**: Must
- **Status**: Draft
- **Description**: Lua scripts execute during `ordo add` and results are cached in `Ordo.lock`. Subsequent builds use cached paths.
- **Acceptance Criteria**:
  - [ ] `ordo add git:user/repo --with script.lua` clones, executes the script, and records results in `Ordo.lock`
  - [ ] `ordo build` uses `Ordo.lock` paths without re-executing the script
  - [ ] `ordo update <name>` re-executes the script if the git commit hash changed
  - [ ] Script file SHA-256 hash is recorded in `Ordo.lock`
  - [ ] Script hash change triggers automatic re-execution on next `ordo build` or `ordo update`
  - [ ] `ordo update <name> --rebuild` forces script re-execution regardless of hash state
  - [ ] Build output directory: `~/.ordo/cache/git-builds/<slug>-<commit>/`

## REQ-LUA-009: Future — Script Sharing via Registry

- **Priority**: Could
- **Status**: Draft
- **Description**: Future capability to share and reuse Lua build scripts via the Ordo Registry. Not in initial scope.
- **Acceptance Criteria**:
  - [ ] Placeholder requirement for future design
  - [ ] Initial API design should not preclude registry-based script distribution
  - [ ] Possible syntax: `with = "registry:fmt"` to reference a registry-hosted script
- **Dependencies**: REQ-PKG-006 (Ordo Registry)

## REQ-LUA-010: Future — General-Purpose Lua Scripts

- **Priority**: Could
- **Status**: Draft
- **Description**: Future extension of `[scripts]` section (REQ-PROJ-005) to support Lua scripts in addition to shell commands.
- **Acceptance Criteria**:
  - [ ] Placeholder requirement for future design
  - [ ] API design should keep Lua runtime and helpers reusable beyond git dependency builds
  - [ ] Possible syntax: `prebuild = { lua = "scripts/generate.lua" }`
- **Dependencies**: REQ-PROJ-005
