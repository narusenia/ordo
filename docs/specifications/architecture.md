# Ordo — Architecture Specification

## Overview

Ordo is a monolithic CLI binary written in Rust. It follows a layered architecture separating user-facing CLI handling from core logic and backend integrations.

## High-Level Architecture

```
┌─────────────────────────────────────────────┐
│                   CLI Layer                  │
│              (clap derive macros)            │
├─────────────────────────────────────────────┤
│                Command Layer                 │
│   new │ build │ run │ test │ fmt │ ...       │
├─────────────────────────────────────────────┤
│                 Core Engine                  │
│  ┌──────────┐ ┌──────────┐ ┌─────────────┐  │
│  │ Manifest │ │ Resolver │ │   Builder    │  │
│  │ Parser   │ │ (pubgrub)│ │(ninja gen)   │  │
│  ├──────────┤ ├──────────┤ ├─────────────┤  │
│  │ Config   │ │ Module   │ │   Tester     │  │
│  │ Merger   │ │ Scanner  │ │              │  │
│  └──────────┘ └──────────┘ └─────────────┘  │
├─────────────────────────────────────────────┤
│               Backend Layer                  │
│  ┌───────┐ ┌───────┐ ┌────────┐ ┌────────┐  │
│  │ vcpkg │ │ conan │ │pkg-conf│ │registry│  │
│  ├───────┤ ├───────┤ ├────────┤ ├────────┤  │
│  │  git  │ │ ninja │ │compiler│ │ cache  │  │
│  └───────┘ └───────┘ └────────┘ └────────┘  │
└─────────────────────────────────────────────┘
```

## Layer Responsibilities

### CLI Layer

- **Crate**: `clap` with derive macros
- **Responsibility**: Parse command-line arguments, dispatch to command handlers
- **Design**: Each subcommand is a struct with `#[derive(Parser)]`. Multi-level commands (e.g., `ordo generate cmake`) use clap's subcommand nesting.

### Command Layer

- **Responsibility**: Orchestrate core engine calls for each CLI command
- **Design**: Thin layer. Each command function loads config, invokes core engine methods, handles errors, and produces user-facing output.
- **Error handling**: Commands catch `Result` from core, format via `miette`, and set exit codes.

### Core Engine

#### Manifest Parser
- Parses `Ordo.toml` using `toml` + `serde`
- Validates schema, produces strongly-typed `Manifest` struct
- Error reporting with source spans (line/column) via `miette`

#### Config Merger
- Implements the 6-level precedence chain: CLI → env → project → local → workspace → global → defaults
- Produces a fully resolved `ResolvedConfig` struct
- Tracks origin of each value for `ordo config show --origin`

#### Resolver (Dependency Resolution)
- PubGrub-based SAT solver for SemVer resolution
- Inputs: `[dependencies]` + `[dev-dependencies]` + transitive deps
- Outputs: resolved dependency graph + `Ordo.lock` entries
- Delegates to Provider backends for package metadata retrieval

#### Builder (Ninja Generator)
- Generates `build.ninja` from resolved manifest + dependencies
- Handles: compiler rules, link rules, profile flags, feature defines
- Workspace mode: single `build.ninja` for all members
- Also generates `compile_commands.json`

#### Module Scanner
- Scans C++ sources for `import`/`export module` declarations
- Builds module dependency DAG
- Generates BMI build rules in `build.ninja`
- Fallback: invokes `clang-scan-deps` for complex cases

#### Tester
- Discovers test sources, builds test binaries
- Detects test framework from includes
- Executes tests in parallel, collects results

### Backend Layer

Each backend is a trait implementation:

#### Provider Trait
```rust
trait Provider {
    fn resolve(&self, name: &str, version: &VersionReq) -> Result<ResolvedDep>;
    fn fetch(&self, dep: &ResolvedDep) -> Result<FetchedDep>;
    fn build_flags(&self, dep: &FetchedDep) -> Result<BuildFlags>;
}
```

Implementations: `VcpkgProvider`, `ConanProvider`, `PkgConfigProvider`, `SystemProvider`, `GitProvider`, `RegistryProvider`

#### Compiler Abstraction
```rust
trait Compiler {
    fn compile_cmd(&self, src: &Path, obj: &Path, flags: &CompileFlags) -> Command;
    fn link_cmd(&self, objs: &[Path], out: &Path, flags: &LinkFlags) -> Command;
    fn syntax_only_flag(&self) -> &str;
    fn module_flags(&self, bmi: &Path) -> Vec<String>;
}
```

Implementations: `ClangCompiler`, `GccCompiler`, `MsvcCompiler`

#### Cache Integration
- Wraps compiler invocations with `sccache`/`ccache` prefix
- Auto-detected or configured via `[cache]`

## Async Architecture

- **Runtime**: `tokio` for IO-bound operations
- **Async operations**: Git clone, registry HTTP requests, vcpkg/conan subprocess execution, file watching
- **CPU-bound operations**: `rayon` for parallel source scanning, module dependency analysis
- **Build execution**: Ninja is invoked as a subprocess (inherently parallel)

## Directory Layout (Ordo source)

```
src/
├── main.rs                 # Entry point, CLI setup
├── cli/
│   ├── mod.rs              # clap definitions
│   ├── new.rs              # ordo new
│   ├── build.rs            # ordo build
│   ├── run.rs              # ordo run
│   └── ...                 # one file per command
├── core/
│   ├── manifest.rs         # Ordo.toml parsing + validation
│   ├── config.rs           # Config merging + precedence
│   ├── resolver.rs         # Dependency resolution (pubgrub)
│   ├── lockfile.rs         # Ordo.lock read/write
│   ├── builder.rs          # Ninja generation
│   ├── modules.rs          # Module scanning + DAG
│   ├── tester.rs           # Test discovery + execution
│   ├── features.rs         # Feature resolution
│   └── workspace.rs        # Workspace member discovery + DAG
├── backend/
│   ├── provider/
│   │   ├── mod.rs          # Provider trait
│   │   ├── vcpkg.rs
│   │   ├── conan.rs
│   │   ├── pkgconfig.rs
│   │   ├── system.rs
│   │   ├── git.rs
│   │   └── registry.rs
│   ├── compiler/
│   │   ├── mod.rs          # Compiler trait
│   │   ├── clang.rs
│   │   ├── gcc.rs
│   │   └── msvc.rs
│   ├── ninja.rs            # build.ninja writer
│   └── cache.rs            # ccache/sccache integration
├── error/
│   ├── mod.rs              # Error types, error codes
│   └── codes.rs            # E00xx - E04xx definitions
└── util/
    ├── paths.rs            # OS-native path resolution (dirs)
    ├── semver.rs            # SemVer utilities
    └── hash.rs             # SHA-256 utilities
```

## Key Design Decisions

1. **Single binary**: No daemon, no background service. Each `ordo` invocation is self-contained.
2. **No build scripts**: Deliberate omission for security. External tool integration via `[scripts]`.
3. **Ninja as the only build executor**: Ordo generates, Ninja executes. No custom build scheduler.
4. **Single build.ninja for workspaces**: Maximizes Ninja's parallel scheduling efficiency.
5. **Compiler abstraction via traits**: New compiler support = new trait impl, no changes to core logic.
6. **Provider abstraction via traits**: New dependency source = new trait impl.

## Error Architecture

```
OrdoError
├── ConfigError (E00xx)     # Ordo.toml parse/validation
├── DepsError (E01xx)       # Dependency resolution/fetch
├── BuildError (E02xx)      # Compilation/linking
├── ToolchainError (E03xx)  # Compiler/linker detection
└── TestError (E04xx)       # Test execution
```

All errors implement `miette::Diagnostic` for rich source-span display.

## Crate Dependencies

| Crate | Purpose | Layer |
|-------|---------|-------|
| `clap` (derive) | CLI argument parsing | CLI |
| `toml` + `serde` | Ordo.toml parsing | Core |
| `miette` | Error display with source spans | Error |
| `pubgrub` | SemVer dependency resolution | Core |
| `tokio` | Async runtime for IO | Backend |
| `rayon` | CPU-parallel scanning | Core |
| `reqwest` | HTTP client for registry | Backend |
| `gix` | Pure-Rust git operations | Backend |
| `notify` | Filesystem watching | CLI |
| `dirs` | OS-native path resolution | Util |
| `owo-colors` | Terminal color output | CLI |
| `tracing` | Structured logging | All |
