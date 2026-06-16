# REQ-WORK — Workspace

## REQ-WORK-001: Workspace Declaration

- **Priority**: Should
- **Status**: Draft
- **Description**: A root `Ordo.toml` with `[workspace]` defines a workspace containing multiple member packages.
- **Acceptance Criteria**:
  - [ ] `[workspace] members = [...]` lists member directories
  - [ ] Glob patterns supported: `"libs/*"`, `"apps/*"`
  - [ ] `exclude = [...]` excludes specific members matching a glob
  - [ ] Root `Ordo.toml` can contain both `[workspace]` and `[package]` simultaneously
  - [ ] A workspace-only root (no `[package]`) is valid

## REQ-WORK-002: Shared Dependencies

- **Priority**: Should
- **Status**: Draft
- **Description**: `[workspace.dependencies]` declares shared dependency versions; members reference them with `{ workspace = true }`.
- **Acceptance Criteria**:
  - [ ] Dependencies declared in `[workspace.dependencies]` with full specifier (version, provider, etc.)
  - [ ] Members use `fmt = { workspace = true }` to inherit the workspace declaration
  - [ ] Members cannot override the version of a workspace dependency
  - [ ] Members can add dependencies not in the workspace section independently

## REQ-WORK-003: Shared Toolchain

- **Priority**: Should
- **Status**: Draft
- **Description**: Workspace-level `[toolchain]` and `[language]` settings apply to all members, with per-member override.
- **Acceptance Criteria**:
  - [ ] Workspace root `[toolchain]` and `[language]` are inherited by all members
  - [ ] Members can override any toolchain/language setting in their own `Ordo.toml`
  - [ ] Override applies only to that member; does not affect siblings

## REQ-WORK-004: Workspace Build

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo build` at the workspace root builds all members respecting inter-member dependencies.
- **Acceptance Criteria**:
  - [ ] Inter-member dependency DAG is computed via `[dependencies] member = { path = "..." }`
  - [ ] All members compiled into a single `build.ninja` for optimal Ninja scheduling
  - [ ] Single `target/` directory at the workspace root
  - [ ] `ordo build -p <member>` builds a specific member and its dependencies only

## REQ-WORK-005: Workspace Commands

- **Priority**: Should
- **Status**: Draft
- **Description**: Workspace-level commands operate across all members.
- **Acceptance Criteria**:
  - [ ] `ordo test` at root runs tests for all members
  - [ ] `ordo fmt` / `ordo lint` at root processes all members
  - [ ] `-p <member>` flag scopes commands to a specific member
  - [ ] `ordo tree` shows the combined dependency tree across all members
