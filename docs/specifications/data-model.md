# Ordo — Data Model Specification

## Ordo.toml Full Schema

This is the canonical reference for the `Ordo.toml` manifest format.

### [package]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | string | Yes | — | Package name (kebab-case recommended) |
| `version` | string | Yes | — | SemVer version (e.g., `"0.1.0"`) |
| `type` | enum | Yes | — | `"executable"`, `"static-library"`, `"shared-library"` |
| `license` | string | No | — | SPDX license identifier |
| `description` | string | No | — | One-line description |
| `authors` | string[] | No | — | List of authors |
| `repository` | string | No | — | Repository URL |

### [language]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `c` | string | No | — | C standard: `"c11"`, `"c17"`, `"c23"` |
| `cpp` | string | No | `"c++20"` | C++ standard: `"c++17"`, `"c++20"`, `"c++23"`, `"c++26"` |

### [toolchain]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `compiler` | string | No | auto-detect | `"clang"`, `"gcc"`, `"msvc"`, `"clang-cl"` |
| `linker` | string | No | compiler default | `"lld"`, `"mold"`, `"gold"`, `"default"` |

### [modules]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `enabled` | bool | No | `false` | Enable C++ modules support |
| `import-std` | bool | No | `false` | Enable `import std;` support |

### [dependencies] / [dev-dependencies]

Dependency specifiers:

```toml
# Registry (short form)
fmt = "11"

# Registry (long form)
fmt = { version = "11" }

# Path
core = { path = "../core" }

# Git
fmt = { git = "https://github.com/fmtlib/fmt", tag = "11.1.0" }
fmt = { git = "https://...", branch = "main" }
fmt = { git = "https://...", rev = "abc123" }

# Provider
fmt = { provider = "vcpkg" }
fmt = { version = "11", provider = "vcpkg" }
openssl = { provider = "pkg-config" }
zlib = { provider = "system" }

# Optional dependency (for features)
qt = { provider = "vcpkg", optional = true }

# Workspace reference
fmt = { workspace = true }

# With features
spdlog = { version = "1.14", provider = "vcpkg", features = ["async"] }
```

### [features]

```toml
[features]
default = ["logging"]
logging = []
gui = ["dep:qt", "logging"]    # enables optional dep + another feature
simd = []

[features.config]
prefix = "MYAPP_"              # default: "ORDO_FEATURE_"
```

### [build]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `jobs` | int | No | `0` (auto) | Parallel compilation jobs (`-j` for Ninja) |
| `pch` | string | No | — | Path to precompiled header source |
| `unity` | bool | No | `false` | Enable unity build |

### [test]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `framework` | string | No | `"auto"` | `"auto"`, `"googletest"`, `"catch2"`, `"doctest"`, `"plain"` |
| `src` | string | No | `"tests/"` | Test source directory |

### [fmt]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `tool` | string | No | `"clang-format"` | Formatting tool |
| `style` | string | No | `".clang-format"` | Style config file path |

### [lint]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `tool` | string | No | `"clang-tidy"` | Linting tool |
| `config` | string | No | `".clang-tidy"` | Config file path |

### [cache]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `tool` | string | No | `"auto"` | `"auto"`, `"ccache"`, `"sccache"`, `"none"` |

### [watch]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `include` | string[] | No | `[]` | Additional directories to watch |
| `exclude` | string[] | No | `[]` | Directories to exclude from watching |

### [install]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `binaries` | string[] | No | inferred | Binary names to install |
| `headers` | string[] | No | inferred | Header glob patterns |
| `libraries` | string[] | No | inferred | Library files to install |

### [scripts]

```toml
[scripts]
prebuild = "python generate_version.py"
deploy = "rsync -av target/release/ server:/opt/"
```

### [ci]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `steps` | string[] | No | see below | CI pipeline steps |

Default steps:
```toml
steps = ["fmt --check", "lint", "build", "test", "build --release"]
```

### [profile.<name>]

| Field | Type | Default (dev) | Default (release) | Description |
|-------|------|---------------|-------------------|-------------|
| `inherits` | string | — | — | Base profile to inherit from |
| `opt-level` | 0/1/2/3/s/z | `0` | `3` | Optimization level |
| `debug` | bool | `true` | `false` | Debug info |
| `assertions` | bool | `true` | `false`* | Enable assertions |
| `sanitize` | string[] | `[]` | `[]` | Sanitizers: address, undefined, thread, memory |
| `lto` | false/thin/full | `false` | `false` | Link-time optimization |
| `strip` | bool | `false` | `true` | Strip symbols |
| `pic` | bool | `false` | `false` | Position-independent code |
| `rtti` | bool | `true` | `true` | C++ RTTI |
| `exceptions` | bool | `true` | `true` | C++ exceptions |
| `warnings` | string | `"all"` | `"all"` | default/all/extra/error |
| `linker` | string | — | — | Override linker for this profile |
| `static-runtime` | bool | `false` | `false` | Static C/C++ runtime |
| `coverage` | bool | `false` | `false` | Code coverage instrumentation |
| `split-debug` | bool | `false` | `false` | Separate debug info file |
| `pch` | string | — | — | Per-profile PCH override |
| `unity` | bool | — | — | Per-profile unity build override |
| `parallel` | int | — | — | Per-profile job count override |

### [target.<triple>]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `compiler` | string | No | from [toolchain] | Compiler for this target |
| `sysroot` | string | No | — | Sysroot path |
| `linker` | string | No | from [toolchain] | Linker for this target |

### [workspace]

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `members` | string[] | Yes | — | Member paths (glob supported) |
| `exclude` | string[] | No | `[]` | Excluded member paths |

### [workspace.dependencies]

Same format as `[dependencies]`, shared across workspace members.

---

## Ordo.lock Schema

```toml
# Auto-generated by Ordo. Do not edit manually.
version = 1

[[package]]
name = "fmt"
version = "11.1.0"
source = "registry+https://registry.ordo.dev"
checksum = "sha256:abcdef1234567890..."

[[package]]
name = "spdlog"
version = "1.14.1"
source = "vcpkg"
checksum = "sha256:..."

[[package]]
name = "mylib"
version = "0.3.0"
source = "git+https://github.com/user/mylib#a1b2c3d4e5f6"
checksum = "sha256:..."

[[package]]
name = "core"
version = "0.1.0"
source = "path+../core"
```

Fields per `[[package]]`:
- `name`: Package name
- `version`: Resolved version
- `source`: Source URI with type prefix (`registry+`, `git+`, `vcpkg`, `conan`, `pkg-config`, `system`, `path+`)
- `checksum`: `sha256:<hex>` (omitted for path dependencies)

---

## Global Config Schema (~/.ordo/config.toml equivalent)

```toml
[defaults]
compiler = "clang"
cpp = "c++20"
linker = "lld"

[target.aarch64-linux-gnu]
compiler = "clang"
sysroot = "/opt/aarch64-sysroot"
linker = "lld"

[cache]
tool = "sccache"

[registries.ordo]
index = "https://registry.ordo.dev"

# Additional registries
[registries.private]
index = "https://registry.internal.company.com"
```

---

## Credentials Schema

Stored at the OS-appropriate config path (e.g., `~/.ordo/credentials.toml`).

```toml
[registries.ordo]
token = "ordo_xxxxxxxxxxxxxxxxxxxx"

[registries.private]
token = "..."
```
