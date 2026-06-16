# Ordo

> A modern project orchestrator for C and C++

Cargo-like developer experience for native development. Ordo unifies build, dependency management, and developer tooling into a single CLI.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/narusenia/ordo/main/install.sh | sh
```

Or download a binary directly from [Releases](https://github.com/narusenia/ordo/releases).

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `ordo-linux-x86_64` |
| Linux aarch64 | `ordo-linux-aarch64` |
| macOS x86_64 | `ordo-macos-x86_64` |
| macOS aarch64 | `ordo-macos-aarch64` |
| Windows x86_64 | `ordo-windows-x86_64.exe` |

### Requirements

- [Ninja](https://ninja-build.org/) build system
- A C/C++ compiler (Clang, GCC, or MSVC)

## Quick Start

```sh
ordo new myapp
cd myapp
ordo add vcpkg:fmt@11.2.0
ordo build
ordo run
```

## Features

### Project Scaffolding

```sh
ordo new                    # Interactive — prompts for name, language, type
ordo new myapp              # C++ executable (default)
ordo new mylib --lib        # C++ static library
ordo new myapp --lang c     # C project
ordo init                   # Initialize in existing directory
```

### Build

```sh
ordo build                  # Debug build
ordo build --release        # Release build
ordo run                    # Build and run
ordo clean                  # Remove build artifacts
```

Ordo generates Ninja build files directly — no CMake in the pipeline. Automatic compiler detection, C/C++ source separation, and rich progress display with streaming output.

### Dependencies

Five provider backends, unified under one CLI:

```toml
# Ordo.toml
[dependencies]
raylib = { version = "6.0", provider = "vcpkg" }
sdl = { version = "3.4.8", provider = "conan" }
openssl = { provider = "pkg-config" }
m = { provider = "system" }
fmt = { git = "https://github.com/fmtlib/fmt", tag = "11.1.0" }
```

#### Adding dependencies

```sh
ordo add vcpkg:raylib@6.0       # vcpkg with pinned version
ordo add vcpkg:fmt@>=11         # vcpkg with minimum version
ordo add conan:sdl@3.4.8        # Conan
ordo add system:m                # System library
ordo add git:fmtlib/fmt@11.1.0  # Git (GitHub shorthand)
ordo add git:codeberg.org/user/repo  # Git (custom host)
ordo add raylib                  # Interactive provider selection
```

Version operators:
- `@11.2.0` or `@=11.2.0` — pin to exact version
- `@>=11` — minimum version
- `@^11` — compatible version (>=11.0.0)
- No version — latest

#### Dependency tree

```sh
ordo tree
```
```
myapp v0.1.0
├── raylib v6.0 (vcpkg)
│   libs: glfw3, nanosvg, nanosvgrast, raylib
│   frameworks: Cocoa, CoreFoundation, IOKit
│   include: /Users/.../vcpkg/installed/arm64-osx/include
├── sdl v3.4.8 (conan)
│   libs: SDL3
│   frameworks: AVFoundation, CoreHaptics, Cocoa, ...
└── m (system)
    libs: m
```

#### Update

```sh
ordo update                 # Re-resolve all dependencies
ordo update fmt             # Re-resolve a specific dependency
```

### Provider Details

| Provider | Auto-install | Version pinning | Platforms |
|----------|-------------|-----------------|-----------|
| **vcpkg** | Yes (auto-bootstrap) | `overrides` for pin, `version>=` for range | All |
| **Conan** | Requires `conan` CLI | Via Conan version ranges | All |
| **pkg-config** | No (system packages) | `--atleast-version` | Linux, macOS |
| **system** | No | N/A | All |
| **git** | Yes (clone + cache) | Tag, branch, or rev | All |

### Configuration

```toml
[package]
name = "myapp"
version = "0.1.0"
type = "executable"          # executable, static-library, shared-library

[language]
cpp = "c++20"                # c++17, c++20, c++23, c++26
# or
c = "c23"                    # c11, c17, c23

[toolchain]
compiler = "clang"           # clang, gcc, msvc
linker = "lld"               # lld, mold, gold, default
```

### CLI Output

Ordo follows Cargo's output conventions with rich terminal UI:

- Colored status verbs (`Compiling`, `Linking`, `Resolved`, etc.)
- Spinners with real-time streaming detail for long operations
- Interactive prompts for `ordo new` and `ordo add` (via promptuity)
- `compile_commands.json` auto-generated for IDE integration

## Philosophy

> Don't replace the ecosystem. Orchestrate it.

Modern C/C++ development requires juggling Ninja, vcpkg, Conan, pkg-config, and more. Ordo provides a unified interface over these tools while preserving compatibility with existing ecosystems.

## Roadmap

Features planned for future releases:

- **Workspaces** — multi-project builds with shared dependencies
- **Build profiles** — custom optimization, sanitizer, and LTO settings
- **Feature flags** — conditional compilation and optional dependencies
- **Testing** — `ordo test` with GoogleTest/Catch2/doctest auto-detection
- **Code quality** — `ordo fmt` (clang-format), `ordo lint` (clang-tidy)
- **C++ modules** — C++20 module dependency scanning and BMI management
- **Cross-compilation** — `ordo build --target aarch64-linux-gnu`
- **Build cache** — sccache/ccache integration
- **Watch mode** — `ordo watch build/test/run`
- **IDE generation** — VSCode, CLion, clangd configs
- **Packaging** — `ordo install`, `ordo package`, `ordo publish`
- **Git dep build integration** — build Ordo.toml/CMakeLists.txt sub-projects

See `docs/implementation/plan.md` for the full task breakdown.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
