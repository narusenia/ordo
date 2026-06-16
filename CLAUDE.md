# Ordo

A Rust-based build and project management tool for C and C++ with a Cargo-like developer experience.

## Project Structure

```
docs/
├── requirements/       # Requirements (REQ-{SCOPE}-{SEQ})
├── specifications/     # Architecture, data model
└── implementation/     # Plan, milestones, tasks (TASK-{SEQ})
```

## Tech Stack

- Language: Rust (edition 2021)
- Key crates: clap (CLI), toml + serde (config), miette (errors), indicatif + console (rich CLI output — progress bars, spinners, styled status lines), promptuity (interactive prompts — Select, Input, Confirm; use MinimalTheme), notify (watch), tokio (async IO), rayon (parallel CPU), gix (git), pubgrub (dependency resolution), reqwest (HTTP), dirs (OS paths), owo-colors (color), tracing (logging)
- Build backend: Ninja (direct generation, no CMake in pipeline)

## CLI Output Style

- Use `indicatif` for progress bars and spinners (build progress, dependency fetching)
- Use `console` for styled terminal output (bold, colored status lines)
- Follow Cargo's output style: `Compiling myapp v0.1.0`, `Finished dev [debug] target(s) in 1.23s`
- Status verbs are right-aligned and colored green (success) or red (error)

## Workflow

Follow these steps for every implementation task:

### 0. Confirm Scope

- Clarify with the user exactly what will be done in this session.
- Resolve any ambiguity before writing code.

### 1. Branching Strategy

- Decide whether a feature branch is needed.
  - **No branch needed** → commit to `main` directly (confirm with user before pushing).
  - **Branch needed** → create a descriptively named branch like `feat/generate-ninja-file` or `fix/compiler-detection-on-windows`. Do NOT use phase numbers or task IDs in branch names (no `phase1`, `TASK-001`, `m1-mvp`, etc.).

### 2. Implementation

- Before coding, ask the user about all unknowns — leave zero ambiguity.
- Write an implementation plan, then execute it.
- Commit in logical units. Each commit should be a self-contained, coherent change.
- Commit messages: **one line, concise, with Conventional Commits prefix**. Format: `<type>: <short description>`
  - Prefixes: `feat:`, `fix:`, `docs:`, `test:`, `ci:`, `chore:`, `refactor:`, `perf:`
  - Example: `feat: add Ninja rule generation for Clang compile commands`
  - Avoid vague descriptions ("update", "cleanup", "review feedback") or AI tool names.

### 3. Self-Review

- After implementation, review the diff for correctness, missed edge cases, and style.
- Fix any issues found before moving on.

### 4. Document Updates

- Check whether changes affect `docs/` (requirements, specifications, implementation plan).
- If so, update the relevant documents or ask the user.

### 5. Verify

- If CI commands or check commands exist (`cargo check`, `cargo test`, `cargo clippy`, etc.), run them and confirm everything passes.

### 6. Pull Request

- Create a PR with a clear title and description.
- PR body should include a summary of changes and a test plan.

## Conventions

- Error codes: `E00xx` (config), `E01xx` (deps), `E02xx` (build), `E03xx` (toolchain), `E04xx` (test)
- Build artifacts: `target/{debug,release}/` — Cargo-style layout
- Config precedence: CLI > env (`ORDO_` prefix) > project Ordo.toml > .ordo/config.toml > workspace > global > defaults
- OS paths: XDG (Linux), ~/Library (macOS), %APPDATA% (Windows), `ORDO_HOME` override
- License: MIT / Apache-2.0 dual
