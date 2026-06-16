# REQ-DEPS — Dependency Management

## REQ-DEPS-001: Dependency Declaration

- **Priority**: Must
- **Status**: Draft
- **Description**: Dependencies are declared in `[dependencies]` with explicit source/provider specification. Ambiguous dependencies (no provider) produce an error.
- **Acceptance Criteria**:
  - [ ] Path dependencies: `core = { path = "../core" }`
  - [ ] Git dependencies: `fmt = { git = "https://...", tag = "11.1.0" }`
  - [ ] Git dependencies accept `tag`, `branch`, or `rev` specifiers
  - [ ] Registry dependencies: `fmt = "11"` or `fmt = { version = "11" }`
  - [ ] Provider dependencies: `fmt = { provider = "vcpkg" }`, with optional `version`
  - [ ] Supported providers: `vcpkg`, `conan`, `pkg-config`, `system`
  - [ ] Missing provider for a name present in multiple sources → error with guidance
  - [ ] Optional dependencies: `qt = { provider = "vcpkg", optional = true }`

## REQ-DEPS-002: Dev Dependencies

- **Priority**: Should
- **Status**: Draft
- **Description**: `[dev-dependencies]` declares test-only and benchmark-only dependencies, not included in release builds.
- **Acceptance Criteria**:
  - [ ] Dev dependencies are linked into test and bench binaries only
  - [ ] Dev dependencies do not appear in `ordo tree` output for the release target
  - [ ] Same declaration syntax as `[dependencies]`

## REQ-DEPS-003: SemVer Resolution

- **Priority**: Must
- **Status**: Draft
- **Description**: Version requirements use SemVer with `^` as the default operator. Resolution uses the PubGrub algorithm. Only SemVer is supported as the versioning scheme. Non-SemVer versioning (CalVer, etc.) must use explicit operators (`=`, `>=`, `>`, `<`, `<=`) instead of `^` or `~`.
- **Acceptance Criteria**:
  - [ ] `"1.2"` is equivalent to `"^1.2"` (>=1.2.0, <2.0.0)
  - [ ] Supports operators: `^`, `~`, `=`, `>=`, `>`, `<`, `<=`
  - [ ] Transitive dependencies resolved without conflicts
  - [ ] Conflicting requirements produce clear error messages listing the conflict chain
  - [ ] Resolution result is deterministic for the same input
  - [ ] Only SemVer is supported; non-SemVer versions work with exact/range operators

## REQ-DEPS-004: Lock File

- **Priority**: Must
- **Status**: Draft
- **Description**: `Ordo.lock` records the exact resolved versions and integrity hashes of all dependencies.
- **Acceptance Criteria**:
  - [ ] Generated/updated on `ordo build` or `ordo update`
  - [ ] Records: package name, version, source, SHA-256 hash
  - [ ] Git dependencies pinned to exact commit hash
  - [ ] `ordo update` re-resolves versions within SemVer constraints and updates the lock
  - [ ] `ordo update <name>` updates a single dependency
  - [ ] Recommended: commit `Ordo.lock` for executables, gitignore for libraries

## REQ-DEPS-005: Provider — vcpkg (Active)

- **Priority**: Should
- **Status**: Draft
- **Description**: vcpkg integration auto-installs packages.
- **Acceptance Criteria**:
  - [ ] Uses `VCPKG_ROOT` if set; otherwise Ordo-managed vcpkg under `~/.ordo/` (or OS-appropriate cache dir)
  - [ ] Generates a vcpkg manifest (`vcpkg.json`) internally
  - [ ] Executes `vcpkg install` automatically
  - [ ] Extracts include paths and link flags from vcpkg installed tree
  - [ ] Errors clearly if vcpkg binary is not found and cannot be bootstrapped

## REQ-DEPS-006: Provider — conan (Active)

- **Priority**: Should
- **Status**: Draft
- **Description**: Conan integration auto-installs packages.
- **Acceptance Criteria**:
  - [ ] Generates `conanfile.txt` internally
  - [ ] Executes `conan install` automatically
  - [ ] Extracts include paths and link flags from Conan generators
  - [ ] Uses user's Conan installation; errors if not found

## REQ-DEPS-007: Provider — pkg-config (Passive)

- **Priority**: Must
- **Status**: Draft
- **Description**: pkg-config integration detects pre-installed system libraries.
- **Acceptance Criteria**:
  - [ ] Queries `pkg-config --cflags --libs <name>`
  - [ ] Passes results to compiler/linker flags in `build.ninja`
  - [ ] Errors if the package is not found via pkg-config

## REQ-DEPS-008: Provider — system (Passive)

- **Priority**: Must
- **Status**: Draft
- **Description**: System provider links against libraries in the compiler's default search paths.
- **Acceptance Criteria**:
  - [ ] Adds `-l<name>` to linker flags
  - [ ] Relies on the compiler's default include and library search paths
  - [ ] Errors at link time if the library is not found

## REQ-DEPS-009: Dependency Commands

- **Priority**: Should
- **Status**: Draft
- **Description**: CLI commands for managing dependencies.
- **Acceptance Criteria**:
  - [ ] `ordo add <name>` adds a dependency to `Ordo.toml` (prompts for provider/version)
  - [ ] `ordo update` re-resolves all dependencies
  - [ ] `ordo update <name>` re-resolves a single dependency
  - [ ] `ordo tree` prints the resolved dependency tree
  - [ ] `ordo tree` shows version, provider, and optional direct/transitive distinction
