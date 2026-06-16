# REQ-PKG — Packaging & Registry

## REQ-PKG-001: Install Command

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo install` builds and installs the project to a system path.
- **Acceptance Criteria**:
  - [ ] Default install prefix: `/usr/local/` (Linux/macOS), configurable via `--prefix`
  - [ ] Installs binaries to `<prefix>/bin/`
  - [ ] For libraries: installs headers to `<prefix>/include/`, libraries to `<prefix>/lib/`
  - [ ] Auto-generates `.pc` (pkg-config) file for library projects
  - [ ] Auto-generates CMake package config (`FooConfig.cmake`, `FooConfigVersion.cmake`)
  - [ ] Install targets configurable via `[install]` section in `Ordo.toml`

## REQ-PKG-002: Package Command

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo package` creates a distributable archive of the built project.
- **Acceptance Criteria**:
  - [ ] Produces `.tar.gz` (Linux/macOS) and `.zip` (Windows) archives
  - [ ] Archive contains: binaries, headers (for libraries), LICENSE, README if present
  - [ ] Archive placed in `target/package/`
  - [ ] OS-specific package formats (deb, rpm) are future scope

## REQ-PKG-003: Publish Command

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo publish` publishes a package to the Ordo Registry (future feature).
- **Acceptance Criteria**:
  - [ ] Authenticates via API token from credentials file
  - [ ] Validates package metadata before upload
  - [ ] Uploads source archive to the registry
  - [ ] Rejects duplicate version numbers
  - [ ] `ordo yank --version <ver>` marks a published version as yanked (not deleted)
- **Dependencies**: REQ-PKG-006

## REQ-PKG-004: pkg-config Generation

- **Priority**: Should
- **Status**: Draft
- **Description**: Library projects auto-generate a `.pc` file on install.
- **Acceptance Criteria**:
  - [ ] Generated `.pc` contains: Name, Description, Version, Cflags, Libs
  - [ ] Installed to `<prefix>/lib/pkgconfig/`
  - [ ] Values derived from `[package]` and `[install]` sections

## REQ-PKG-005: CMake Config Generation

- **Priority**: Should
- **Status**: Draft
- **Description**: Library projects auto-generate CMake find-package config files on install.
- **Acceptance Criteria**:
  - [ ] Generates `<Name>Config.cmake` and `<Name>ConfigVersion.cmake`
  - [ ] Installed to `<prefix>/lib/cmake/<Name>/`
  - [ ] Consumers can use `find_package(<Name>)` in CMake projects

## REQ-PKG-006: Ordo Registry

- **Priority**: Could
- **Status**: Draft
- **Description**: A self-hosted package registry with scoped namespaces.
- **Acceptance Criteria**:
  - [ ] Scoped package names: `@org/package` (e.g., `@nxeu/core`)
  - [ ] Flat names also allowed (no scope required)
  - [ ] Git-based index for offline search capability
  - [ ] Source archive storage (`.tar.gz`)
  - [ ] SemVer required for all published packages
  - [ ] REST API for search, metadata, publish, yank
  - [ ] API token authentication; credentials stored in OS-appropriate config path
  - [ ] Yank support (mark as deprecated, no deletion)
