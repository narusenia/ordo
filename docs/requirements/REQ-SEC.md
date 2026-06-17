# REQ-SEC — Security & Supply Chain

## REQ-SEC-001: Lock File Integrity

- **Priority**: Must
- **Status**: Draft
- **Description**: `Ordo.lock` records SHA-256 hashes for all dependencies and verifies them on build.
- **Acceptance Criteria**:
  - [ ] Every dependency entry in `Ordo.lock` includes a SHA-256 hash of its source archive or content
  - [ ] Hash verification on every build; mismatch → hard error
  - [ ] Git dependencies pinned to exact commit hash in the lock file (even if declared with tag/branch)
  - [ ] Provider (vcpkg/conan) dependencies also hash-checked in `Ordo.lock`

## REQ-SEC-002: CI Integrity Flags

- **Priority**: Must
- **Status**: Draft
- **Description**: `--locked` and `--frozen` flags enforce lock file integrity in CI environments.
- **Acceptance Criteria**:
  - [ ] `--locked`: error if `Ordo.lock` and `Ordo.toml` are out of sync (lock needs update)
  - [ ] `--frozen`: error on any network access (fully offline build from lock + cache)
  - [ ] `ordo ci` implicitly applies `--locked`

## REQ-SEC-003: Registry Security

- **Priority**: Could
- **Status**: Draft
- **Description**: The Ordo Registry enforces security best practices.
- **Acceptance Criteria**:
  - [ ] HTTPS required for all registry communication
  - [ ] API token authentication for publish operations
  - [ ] Published packages cannot be deleted, only yanked
  - [ ] Package signing (future): cryptographic signature verification
  - [ ] 2FA for registry accounts (future)

## REQ-SEC-004: Audit Command

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo audit` checks dependencies against a known vulnerability database (future feature).
- **Acceptance Criteria**:
  - [ ] Placeholder command reserved for future implementation
  - [ ] Planned: check `Ordo.lock` entries against a vulnerability advisory database
  - [ ] Clear message when invoked before implementation

## REQ-SEC-005: No Implicit Build Scripts

- **Priority**: Must
- **Status**: Draft
- **Description**: Ordo does not support implicit build scripts (programmatic build logic executed automatically during build). This eliminates a major supply chain attack vector. The only exception is Lua build scripts for git dependencies, which require explicit user opt-in via the `with` field (see REQ-LUA).
- **Acceptance Criteria**:
  - [ ] No `build.rs` equivalent; no `[build-dependencies]`
  - [ ] No pre-build or post-build hooks that execute automatically
  - [ ] `[scripts]` are user-invoked only (never triggered implicitly by build)
  - [ ] Git dependency Lua scripts execute only when explicitly declared via `with` in `Ordo.toml`
  - [ ] Lua scripts run in a sandboxed environment (REQ-LUA-007)
  - [ ] Documented as a deliberate security decision
