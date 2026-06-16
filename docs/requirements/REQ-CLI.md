# REQ-CLI — CLI, UX & Configuration

## REQ-CLI-001: Command Structure

- **Priority**: Must
- **Status**: Draft
- **Description**: Ordo uses a multi-level subcommand structure.
- **Acceptance Criteria**:
  - [ ] Top-level commands: `new`, `init`, `build`, `run`, `test`, `bench`, `clean`, `check`, `watch`, `fmt`, `lint`, `analyze`, `add`, `update`, `tree`, `install`, `package`, `publish`, `import`, `generate`, `toolchain`, `ci`, `doctor`, `config`, `run-script`, `self`
  - [ ] Multi-level: `ordo import cmake`, `ordo generate cmake`, `ordo self update`, `ordo toolchain list`, `ordo config show`
  - [ ] `--help` on every command and subcommand
  - [ ] Unknown subcommands suggest closest match (did-you-mean)

## REQ-CLI-002: Error Display

- **Priority**: Must
- **Status**: Draft
- **Description**: Errors use structured error codes and display source locations for `Ordo.toml` issues.
- **Acceptance Criteria**:
  - [ ] Error code format: `E00xx` (config), `E01xx` (deps), `E02xx` (build), `E03xx` (toolchain), `E04xx` (test)
  - [ ] `Ordo.toml` errors show line number, column, and underline (via `miette` or `ariadne`)
  - [ ] `= help:` lines suggest next actions where possible
  - [ ] Compiler errors passed through unmodified
  - [ ] English-only initially

## REQ-CLI-003: Color and Verbosity

- **Priority**: Must
- **Status**: Draft
- **Description**: Output supports color and multiple verbosity levels.
- **Acceptance Criteria**:
  - [ ] Color enabled by default; disable with `--color never` or `ORDO_COLOR=0`
  - [ ] `-v`: show executed commands (compiler invocations, ninja commands)
  - [ ] `-vv`: detailed debug logging (via `tracing`)
  - [ ] Default: only show progress, warnings, and errors

## REQ-CLI-004: Configuration System

- **Priority**: Must
- **Status**: Draft
- **Description**: Layered configuration with well-defined precedence.
- **Acceptance Criteria**:
  - [ ] Precedence (high to low): CLI flags → environment variables → project `Ordo.toml` → `.ordo/config.toml` (project-local, gitignored) → workspace root `Ordo.toml` → global config → built-in defaults
  - [ ] Environment variables: `ORDO_` prefix, e.g., `ORDO_TOOLCHAIN_COMPILER=clang`
  - [ ] `ordo config show` displays the fully resolved configuration
  - [ ] `ordo config show --origin` shows which source each value came from

## REQ-CLI-005: Global Configuration Paths

- **Priority**: Must
- **Status**: Draft
- **Description**: OS-native paths for global configuration and cache.
- **Acceptance Criteria**:
  - [ ] Linux: `$XDG_CONFIG_HOME/ordo/` (config), `$XDG_CACHE_HOME/ordo/` (cache)
  - [ ] macOS: `~/Library/Application Support/ordo/` (config), `~/Library/Caches/ordo/` (cache)
  - [ ] Windows: `%APPDATA%\ordo\` (config), `%LOCALAPPDATA%\ordo\cache\` (cache)
  - [ ] `ORDO_HOME` overrides all paths to a single directory
  - [ ] Path resolution via `dirs` crate

## REQ-CLI-006: Watch Mode

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo watch <command>` re-executes a command on file changes.
- **Acceptance Criteria**:
  - [ ] `ordo watch build`, `ordo watch test`, `ordo watch run`
  - [ ] File watching via `notify` crate
  - [ ] Default watched: `src/`, `include/`, `tests/`, `Ordo.toml`; excludes `target/`, `.git/`
  - [ ] Custom watch paths via `[watch] include = [...], exclude = [...]`
  - [ ] Debounce: 300ms after last change before triggering
  - [ ] `ordo watch run` kills previous process before restart
  - [ ] `--notify` flag for desktop notifications on build success/failure
  - [ ] Future: `ordo watch -- <arbitrary command>`

## REQ-CLI-007: Doctor Command

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo doctor` reports on the development environment.
- **Acceptance Criteria**:
  - [ ] Lists: Ordo version, detected compiler(s) + versions, Ninja, cache tool, vcpkg, conan, pkg-config, clang-format, clang-tidy
  - [ ] Shows check/cross mark for each tool's availability
  - [ ] Indicates which tools are required vs optional

## REQ-CLI-008: Self Update

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo self update` updates Ordo to the latest version.
- **Acceptance Criteria**:
  - [ ] Downloads the latest release binary from the configured release source
  - [ ] `ordo self update --version <ver>` updates to a specific version
  - [ ] Update check: once per 24 hours, prints a one-line notice to stderr if a newer version exists
  - [ ] `ORDO_NO_UPDATE_CHECK=1` disables the update check
  - [ ] Release source is not hardcoded to a single platform (Codeberg/GitHub agnostic)
