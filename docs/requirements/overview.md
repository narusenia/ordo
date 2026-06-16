# Ordo — Requirements Overview

## Project Summary

Ordo is a Rust-based build and project management tool for C and C++ that provides a Cargo-like developer experience. It unifies build systems, dependency management, toolchains, testing, packaging, and developer tooling into a single cohesive workflow.

Ordo does not replace the C/C++ ecosystem — it orchestrates it.

## Scope

### In Scope

- Project scaffolding and management (Ordo.toml)
- Build system with direct Ninja generation
- Dependency management with multiple provider backends
- C++ modules (C++20/23/26) as a first-class feature
- Workspace/monorepo support
- Toolchain management and cross-compilation
- Testing with framework auto-detection
- Code quality tooling (format, lint, analyze)
- Packaging, installation, and a package registry
- IDE integration and CMake compatibility
- CI/CD integration
- Build caching (local and future remote)
- Security and supply chain integrity

### Out of Scope

- Replacing compilers, linkers, or Ninja itself
- Build scripts / programmatic build logic (by design)
- Plugin system (by design; extensibility via `[scripts]` and PATH)
- GUI / visual tooling
- Language support beyond C and C++

## Glossary

| Term | Definition |
|------|-----------|
| **Ordo.toml** | The project manifest file, analogous to Cargo.toml |
| **Provider** | A backend that supplies dependencies (vcpkg, conan, pkg-config, system, git, registry) |
| **Active Provider** | A provider where Ordo auto-installs packages (vcpkg, conan) |
| **Passive Provider** | A provider where Ordo only detects pre-installed packages (pkg-config, system) |
| **Profile** | A named set of build configuration options (e.g., dev, release) |
| **Workspace** | A collection of related packages managed together in a monorepo |
| **Target Triple** | A Clang/GCC-compatible platform identifier (e.g., `aarch64-linux-gnu`) |
| **BMI** | Binary Module Interface — compiler-generated module metadata |
| **Feature** | A conditional compilation flag that controls optional functionality |
| **Ordo.lock** | A lockfile recording exact resolved dependency versions and hashes |
| **Ordo Registry** | The self-hosted package registry for Ordo packages |

## Requirements Summary

| Scope | ID Range | Title | Count |
|-------|----------|-------|-------|
| PROJ | REQ-PROJ-001 ~ 006 | Project Management | 6 |
| BUILD | REQ-BUILD-001 ~ 010 | Build System | 10 |
| DEPS | REQ-DEPS-001 ~ 009 | Dependency Management | 9 |
| MOD | REQ-MOD-001 ~ 004 | C++ Modules | 4 |
| WORK | REQ-WORK-001 ~ 005 | Workspace | 5 |
| TOOL | REQ-TOOL-001 ~ 004 | Toolchain Management | 4 |
| TEST | REQ-TEST-001 ~ 005 | Testing | 5 |
| QUAL | REQ-QUAL-001 ~ 004 | Code Quality | 4 |
| PKG | REQ-PKG-001 ~ 006 | Packaging & Registry | 6 |
| IDE | REQ-IDE-001 ~ 005 | IDE & CMake Integration | 5 |
| CLI | REQ-CLI-001 ~ 008 | CLI, UX & Configuration | 8 |
| SEC | REQ-SEC-001 ~ 005 | Security & Supply Chain | 5 |
| **Total** | | | **71** |

## Priority Distribution

| Priority | Count | Description |
|----------|-------|-------------|
| Must | 38 | Core functionality required for a usable tool |
| Should | 22 | Important features expected by users |
| Could | 11 | Valuable additions, deferrable without impact |
