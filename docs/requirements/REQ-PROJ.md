# REQ-PROJ — Project Management

## REQ-PROJ-001: Project Creation

- **Priority**: Must
- **Status**: Draft
- **Description**: `ordo new <name>` creates a new project directory with scaffolded files. Supports two project types: executable (default) and library (`--lib`).
- **Acceptance Criteria**:
  - [ ] `ordo new myapp` generates: `Ordo.toml`, `.gitignore`, `src/main.cpp`, `tests/main_test.cpp`
  - [ ] `ordo new mylib --lib` generates: `Ordo.toml`, `.gitignore`, `include/mylib/mylib.hpp`, `src/mylib.cpp`, `tests/mylib_test.cpp`
  - [ ] `Ordo.toml` is populated with correct `[package]` metadata and sensible defaults
  - [ ] `git init` is executed automatically; `--no-git` skips it
  - [ ] `.gitignore` includes `target/`

## REQ-PROJ-002: Project Initialization

- **Priority**: Must
- **Status**: Draft
- **Description**: `ordo init` initializes an Ordo project in an existing directory by generating `Ordo.toml` without modifying existing source files.
- **Acceptance Criteria**:
  - [ ] Generates `Ordo.toml` in the current directory
  - [ ] Does not create, move, or delete any source files
  - [ ] Detects existing source layout and infers `type` (executable if `main.cpp` exists, library otherwise)
  - [ ] Errors if `Ordo.toml` already exists

## REQ-PROJ-003: Ordo.toml Schema — Package Metadata

- **Priority**: Must
- **Status**: Draft
- **Description**: `Ordo.toml` supports `[package]` section with project metadata fields.
- **Acceptance Criteria**:
  - [ ] Required fields: `name`, `version`, `type`
  - [ ] `type` accepts: `executable`, `static-library`, `shared-library`
  - [ ] Optional fields: `license`, `description`, `authors` (array), `repository`
  - [ ] `version` follows SemVer format
  - [ ] Invalid values produce clear error messages with source location

## REQ-PROJ-004: Ordo.toml Schema — Language Configuration

- **Priority**: Must
- **Status**: Draft
- **Description**: `[language]` section specifies C and C++ standard versions.
- **Acceptance Criteria**:
  - [ ] `cpp` field accepts standard identifiers: `c++17`, `c++20`, `c++23`, `c++26`
  - [ ] `c` field accepts: `c11`, `c17`, `c23`
  - [ ] Default: `cpp = "c++20"` when omitted
  - [ ] Values map to correct compiler flags (`-std=c++20`, `/std:c++20`, etc.)

## REQ-PROJ-005: Ordo.toml Schema — Scripts

- **Priority**: Should
- **Status**: Draft
- **Description**: `[scripts]` section defines named shell commands that can be executed via `ordo run-script <name>`.
- **Acceptance Criteria**:
  - [ ] Scripts are key-value pairs: `name = "shell command"`
  - [ ] `ordo run-script <name>` executes the associated command in the project root
  - [ ] Unknown script name produces a clear error listing available scripts
  - [ ] Scripts are NOT automatically invoked as build hooks (no pre/post-build)
- **Dependencies**: None

## REQ-PROJ-006: Ordo.toml Schema — Features

- **Priority**: Should
- **Status**: Draft
- **Description**: `[features]` section defines conditional compilation flags following the Cargo feature model.
- **Acceptance Criteria**:
  - [ ] `default` key lists features enabled by default
  - [ ] Each feature maps to a list of other features or `dep:<name>` for optional dependencies
  - [ ] Enabled features inject compiler defines: `-D<PREFIX><FEATURE_NAME>=1`
  - [ ] Default prefix is `ORDO_FEATURE_`; customizable via `[features.config] prefix = "..."`
  - [ ] CLI flags: `--features <list>`, `--no-default-features`, `--all-features`
  - [ ] Feature interdependencies are resolved transitively
- **Dependencies**: REQ-DEPS-001
