# Ordo

> A modern project orchestrator for C and C++
>
> Cargo-like developer experience for native development.

Ordo is a Rust-based build and project management tool that unifies build systems, dependency management, toolchains, testing, packaging, and developer tooling into a single workflow.

## Philosophy

> Don't replace the ecosystem. Orchestrate it.

Modern C/C++ development requires juggling CMake, Ninja, vcpkg, Conan, pkg-config, clangd, ccache, clang-format, clang-tidy, and more. Ordo provides a unified interface over these tools while preserving compatibility with existing ecosystems.

## Quick Start

```bash
ordo new myapp
cd myapp
ordo build
ordo run
```

## Features

### Project Management

```bash
ordo new myapp          # Create executable project
ordo new mylib --lib    # Create library project
ordo init               # Initialize in existing directory
```

### Build

```bash
ordo build              # Debug build
ordo build --release    # Release build
ordo run                # Build and run
ordo check              # Syntax check only (fast)
ordo clean              # Remove build artifacts
```

### Dependencies

```toml
# Ordo.toml
[dependencies]
core = { path = "../core" }
fmt = { git = "https://github.com/fmtlib/fmt", tag = "11.1.0" }
spdlog = { version = "1.14", provider = "vcpkg" }
openssl = { provider = "pkg-config" }
zlib = { provider = "system" }
```

```bash
ordo add fmt            # Add dependency
ordo update             # Update lock file
ordo tree               # Show dependency tree
```

Supported providers: **vcpkg**, **Conan**, **pkg-config**, **system**, **git**, **Ordo Registry**

### Workspaces

```toml
[workspace]
members = ["apps/*", "libs/*"]

[workspace.dependencies]
fmt = "11"
```

### Testing

```bash
ordo test               # Run all tests
ordo test --filter name # Filter tests
ordo test --jobs 4      # Parallel execution
```

Auto-detects GoogleTest, Catch2, and doctest.

### Code Quality

```bash
ordo fmt                # Format (clang-format)
ordo fmt --check        # Check formatting (CI)
ordo lint               # Lint (clang-tidy)
ordo lint --fix         # Auto-fix lint issues
```

### C++ Modules

First-class support for C++20 modules:

```toml
[modules]
enabled = true
import-std = true
```

Automatic module dependency scanning and BMI management across Clang, GCC, and MSVC.

### Cross-Compilation

```bash
ordo build --target aarch64-linux-gnu
```

```toml
[target.aarch64-linux-gnu]
compiler = "clang"
sysroot = "/usr/aarch64-linux-gnu"
```

### Build Profiles

```toml
[profile.dev]
opt-level = 0
debug = true
sanitize = ["address", "undefined"]

[profile.release]
opt-level = 3
lto = "thin"
strip = true

[profile.custom]
inherits = "release"
opt-level = "s"
```

### Feature Flags

```toml
[features]
default = ["logging"]
logging = []
gui = ["dep:qt"]

[dependencies]
qt = { provider = "vcpkg", optional = true }
```

```bash
ordo build --features gui
```

### Watch Mode

```bash
ordo watch build
ordo watch test
ordo watch run
```

### IDE Integration

```bash
ordo generate vscode
ordo generate clion
ordo generate clangd
```

`compile_commands.json` is auto-generated at the project root.

### CMake Compatibility

```bash
ordo import cmake       # CMakeLists.txt -> Ordo.toml (migration aid)
ordo generate cmake     # Ordo.toml -> CMakeLists.txt
ordo generate presets   # Generate CMakePresets.json
```

### CI

```bash
ordo ci                 # Run full CI pipeline
ordo generate github-actions
ordo generate gitlab-ci
```

### Packaging

```bash
ordo install            # Install to system (with pkg-config + CMake config)
ordo package            # Create distributable archive
ordo publish            # Publish to Ordo Registry
```

### Diagnostics

```bash
ordo doctor             # Check development environment
ordo config show        # Show resolved configuration
ordo config show --origin  # Show where each value comes from
```

## Build Backend

Ordo generates Ninja build files directly — no CMake in the build pipeline. This enables full control over C++ modules, dependency scanning, and build optimization while leveraging Ninja's battle-tested incremental build and parallelism.

## Configuration

Project configuration lives in `Ordo.toml`:

```toml
[package]
name = "myapp"
version = "0.1.0"
type = "executable"

[language]
cpp = "c++20"

[toolchain]
compiler = "clang"
linker = "lld"

[cache]
tool = "auto"  # sccache > ccache > none
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
