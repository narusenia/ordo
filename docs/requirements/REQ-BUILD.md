# REQ-BUILD — Build System

## REQ-BUILD-001: Build Command

- **Priority**: Must
- **Status**: Draft
- **Description**: `ordo build` compiles the project. Default profile is `dev`; `--release` selects the release profile.
- **Acceptance Criteria**:
  - [ ] Parses `Ordo.toml` and generates `build.ninja` in `target/<profile>/build/`
  - [ ] Invokes Ninja to execute the build
  - [ ] `ordo build` uses the `dev` profile
  - [ ] `ordo build --release` uses the `release` profile
  - [ ] `ordo build --profile <name>` uses a custom profile
  - [ ] Build artifacts placed in `target/<profile>/` (e.g., `target/debug/myapp`)
  - [ ] Intermediate files (.o, .d, build.ninja) placed in `target/<profile>/build/`
  - [ ] Returns non-zero exit code on build failure
- **Dependencies**: REQ-PROJ-003, REQ-BUILD-003

## REQ-BUILD-002: Run Command

- **Priority**: Must
- **Status**: Draft
- **Description**: `ordo run` builds (if needed) and executes the project binary. Only valid for `executable` projects.
- **Acceptance Criteria**:
  - [ ] Triggers a build if artifacts are stale
  - [ ] Executes the built binary
  - [ ] Arguments after `--` are forwarded to the binary: `ordo run -- --verbose`
  - [ ] Errors clearly if `type` is not `executable`

## REQ-BUILD-003: Ninja Generation

- **Priority**: Must
- **Status**: Draft
- **Description**: Ordo generates `build.ninja` files directly from `Ordo.toml` without CMake in the pipeline.
- **Acceptance Criteria**:
  - [ ] Generates correct `build.ninja` for executable, static-library, and shared-library targets
  - [ ] Compiler rules use correct flags per compiler (Clang, GCC, MSVC)
  - [ ] Include paths, defines, and link flags are correctly propagated
  - [ ] Depfile rules are generated for incremental builds
  - [ ] Generated `build.ninja` is valid and can be executed standalone by Ninja

## REQ-BUILD-004: Build Profiles

- **Priority**: Must
- **Status**: Draft
- **Description**: Build profiles control compilation options. `dev` and `release` exist implicitly. Custom profiles use `inherits`.
- **Acceptance Criteria**:
  - [ ] `dev` and `release` profiles exist without explicit declaration in `Ordo.toml`
  - [ ] Supported profile options: `opt-level` (0/1/2/3/s/z), `debug`, `assertions`, `sanitize`, `lto` (false/thin/full), `strip`
  - [ ] Additional options: `pic`, `rtti`, `exceptions`, `warnings` (default/all/extra/error), `linker`, `static-runtime`, `coverage`, `split-debug`, `pch`, `unity`, `parallel`
  - [ ] Custom profiles with `inherits = "<base>"` inherit all unset values from the base
  - [ ] CLI overrides: `ordo build --profile <name>`
  - [ ] `dev` defaults: opt-level=0, debug=true, assertions=true
  - [ ] `release` defaults: opt-level=3, debug=false, lto=false, strip=true

## REQ-BUILD-005: Sanitizer Support

- **Priority**: Should
- **Status**: Draft
- **Description**: Sanitizers can be enabled per profile or via CLI flag.
- **Acceptance Criteria**:
  - [ ] Supported sanitizers: `address`, `undefined`, `thread`, `memory`
  - [ ] Configurable in `[profile.<name>] sanitize = [...]`
  - [ ] CLI override: `ordo build --sanitize=address`
  - [ ] Correct compiler flags generated for each compiler (Clang, GCC, MSVC where supported)

## REQ-BUILD-006: Incremental Build

- **Priority**: Must
- **Status**: Draft
- **Description**: Builds are incremental by default, using Ninja's dependency tracking.
- **Acceptance Criteria**:
  - [ ] Only recompiles changed source files and their dependents
  - [ ] Depfile (`.d`) generated per translation unit
  - [ ] Header changes trigger recompilation of affected sources

## REQ-BUILD-007: Unity Build

- **Priority**: Could
- **Status**: Draft
- **Description**: Unity (jumbo) builds can be enabled to reduce compilation time by merging translation units.
- **Acceptance Criteria**:
  - [ ] Enabled via `[build] unity = true` or `[profile.<name>] unity = true`
  - [ ] Generates a single combined source file that includes all sources
  - [ ] Falls back to normal build on failure

## REQ-BUILD-008: Precompiled Headers

- **Priority**: Could
- **Status**: Draft
- **Description**: Precompiled header support to reduce compilation time.
- **Acceptance Criteria**:
  - [ ] Configured via `[build] pch = "include/pch.hpp"` or per-profile
  - [ ] PCH compiled before all other sources
  - [ ] Correct flags per compiler (Clang `-include-pch`, GCC `-include`, MSVC `/Yu`)

## REQ-BUILD-009: Clean Command

- **Priority**: Must
- **Status**: Draft
- **Description**: `ordo clean` removes build artifacts.
- **Acceptance Criteria**:
  - [ ] `ordo clean` removes the `target/` directory
  - [ ] `ordo clean --cache` additionally clears external build cache (ccache/sccache)
  - [ ] Does not remove source files, `Ordo.toml`, or `Ordo.lock`

## REQ-BUILD-010: Check Command

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo check` performs syntax and type checking without producing final binaries, for fast feedback.
- **Acceptance Criteria**:
  - [ ] Invokes the compiler with `-fsyntax-only` (Clang/GCC) or equivalent
  - [ ] Faster than a full build
  - [ ] Reports errors in the same format as `ordo build`
