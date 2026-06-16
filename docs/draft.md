# Ordo

> A modern project orchestrator for C and C++
>
> Cargo-like developer experience for native development.

Ordo is a Rust-based build and project management tool for C and C++ that unifies build systems, dependency management, toolchains, testing, packaging, and developer tooling into a single workflow.

---

# Vision

Modern C++ development often requires combining multiple tools:

- CMake
- Ninja
- vcpkg
- Conan
- pkg-config
- clangd
- ccache / sccache
- clang-format
- clang-tidy

Ordo aims to provide a unified interface over these tools while preserving compatibility with existing ecosystems.

```bash
ordo new app
ordo build
ordo run
ordo test
ordo fmt
ordo lint
```

The goal is not to replace the ecosystem, but to orchestrate it.

---

# Core Principles

## Cargo-like UX

Simple and discoverable commands.

```bash
ordo new app
ordo build
ordo run
ordo test
```

No need to learn multiple build systems to start a project.

---

## Ecosystem Compatibility

Ordo integrates with existing tools instead of replacing them.

Supported providers:

- vcpkg
- Conan
- pkg-config
- System packages

Future support:

- Ordo Registry

---

## Toolchain-First Design

Toolchains are first-class citizens.

```toml
[toolchain]
compiler = "clang"
version = "18"

[target]
platform = "linux"
arch = "x86_64"
```

Supported compilers:

- Clang
- GCC
- MSVC
- clang-cl

---

## Native Monorepo Support

Workspace management similar to Cargo.

```toml
[workspace]
members = [
    "apps/editor",
    "libs/core",
    "libs/render"
]
```

---

## Modern C++ Support

Designed with C++20/23/26 in mind.

Features:

- Modules
- import std
- Dependency scanning
- BMI management
- Compiler abstraction

---

# Features

## Project Management

### Create Projects

```bash
ordo new app
ordo new lib
ordo init
```

### Workspaces

```toml
[workspace]
members = [
    "apps/editor",
    "libs/core"
]
```

### Monorepos

Shared:

- dependencies
- toolchains
- build configuration

---

# Build System

## Build

```bash
ordo build
ordo build --release
```

## Run

```bash
ordo run
```

## Test

```bash
ordo test
```

## Watch Mode

```bash
ordo watch build
ordo watch test
```

## Clean

```bash
ordo clean
```

## compile_commands.json

Generated automatically.

Compatible with:

- clangd
- VSCode
- Neovim
- CLion

---

# Dependency Management

## Local Dependencies

```toml
[dependencies]
core = { path = "../core" }
```

## Git Dependencies

```toml
[dependencies]
fmt = { git = "https://github.com/fmtlib/fmt" }
```

## Registry Dependencies

```toml
[dependencies]
fmt = "11"
```

---

## vcpkg Integration

```toml
fmt = { provider = "vcpkg" }
```

---

## Conan Integration

```toml
fmt = { provider = "conan" }
```

---

## pkg-config Integration

```toml
openssl = { provider = "pkg-config" }
```

---

## System Libraries

```toml
zlib = { provider = "system" }
```

---

# Toolchain Management

## Language Standards

```toml
[language]
c = "c23"
cpp = "c++26"
```

---

## Cross Compilation

```bash
ordo build --target aarch64-linux-gnu
```

---

## Compiler Selection

```toml
[toolchain]
compiler = "clang"
```

Supported:

- clang
- gcc
- msvc
- clang-cl

---

# Build Profiles

Cargo-inspired build profiles.

```toml
[profile.dev]
debug = true

[profile.release]
opt-level = 3
lto = true
strip = true
```

---

## Sanitizers

```toml
sanitize = [
    "address",
    "undefined"
]
```

Supported:

- AddressSanitizer
- UBSan
- ThreadSanitizer
- MemorySanitizer

---

# Build Performance

## Incremental Build

Enabled by default.

---

## Unity Build

```toml
unity = true
```

---

## Precompiled Headers

```toml
pch = "include/pch.hpp"
```

---

## Cache Integration

Supported:

- ccache
- sccache

Future:

- Remote cache

---

# C++ Modules

A first-class feature.

## Module Dependency Scanning

Automatic.

## BMI Management

Automatic.

## Compiler Compatibility Layer

Supports:

- Clang
- GCC
- MSVC

## import std

Supported.

---

# Testing

## Test Execution

```bash
ordo test
```

## Framework Detection

Automatically detects:

- GoogleTest
- Catch2
- doctest

---

# Code Quality

## Formatting

```bash
ordo fmt
```

Powered by:

- clang-format

---

## Linting

```bash
ordo lint
```

Powered by:

- clang-tidy

---

## Static Analysis

```bash
ordo analyze
```

Future support.

---

# Packaging

## Install

```bash
ordo install
```

## Package

```bash
ordo package
```

## Publish

```bash
ordo publish
```

Future support.

---

# C Library Support

## pkg-config Generation

Generate:

```ini
foo.pc
```

---

## Install Rules

Automatic installation layout generation.

---

## C ABI Packaging

Future support:

- ABI validation
- symbol visibility
- shared library versioning

---

# IDE Integration

## clangd

Automatic support via:

```text
compile_commands.json
```

## VSCode

Configuration generation.

## CLion

Native support.

## Neovim

Works out-of-the-box with clangd.

---

# CMake Compatibility

## Import Existing Projects

```bash
ordo import-cmake
```

---

## Export CMake

```bash
ordo export-cmake
```

---

## Generate Presets

```bash
ordo generate presets
```

---

## FetchContent Migration

Potential future support.

---

# Future Roadmap

## Ordo Registry

A package registry for C and C++.

```toml
fmt = "11.1"
```

---

## Ordo.lock

Deterministic dependency resolution.

```text
Ordo.lock
```

---

## Semantic Version Resolver

Cargo-style dependency resolution.

---

## Binary Distribution

```bash
ordo install ripgrep
```

---

## Remote Build Cache

Shared cache across machines and CI.

---

## CI Integration

```bash
ordo ci
```

Potential future feature.

---

# Positioning

Ordo is not merely a build system.

It is a complete development orchestrator for C and C++.

It provides:

- Cargo-like workflows
- Unified dependency management
- Toolchain management
- Modern C++ support
- Ecosystem compatibility
- Monorepo support
- IDE integration

while remaining compatible with:

- CMake
- Ninja
- vcpkg
- Conan
- pkg-config

---

# Philosophy

> Don't replace the ecosystem.
>
> Orchestrate it.

Ordo exists to bring order to modern C and C++ development.
