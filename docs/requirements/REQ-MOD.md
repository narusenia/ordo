# REQ-MOD — C++ Modules

## REQ-MOD-001: Module Toggle

- **Priority**: Should
- **Status**: Draft
- **Description**: C++ modules are opt-in via `[modules] enabled = true`. When enabled, Ordo enforces compiler version minimums.
- **Acceptance Criteria**:
  - [ ] Default: `enabled = false`
  - [ ] When enabled, checks compiler version: Clang >= 18, GCC >= 14, MSVC >= 17.5
  - [ ] Error with clear message if compiler does not meet minimum version
  - [ ] `import-std = false` by default; opt-in separately

## REQ-MOD-002: Module Dependency Scanning

- **Priority**: Should
- **Status**: Draft
- **Description**: Ordo scans source files for `import` and `export module` declarations to build a module dependency DAG.
- **Acceptance Criteria**:
  - [ ] Primary scanner: self-built Rust parser extracting `import`/`export module` statements
  - [ ] Fallback scanner: compiler-based (`clang-scan-deps`) when self-built parser is insufficient
  - [ ] DAG determines compilation order for BMI generation
  - [ ] Circular module dependencies produce a clear error

## REQ-MOD-003: BMI Management

- **Priority**: Should
- **Status**: Draft
- **Description**: Binary Module Interface files are generated and managed automatically.
- **Acceptance Criteria**:
  - [ ] BMI generation order derived from the module dependency DAG
  - [ ] BMIs stored in `target/<profile>/build/`
  - [ ] Correct compiler flags per compiler: Clang (`-fmodule-file=`), GCC (`-fmodules-ts`), MSVC (`/module:interface`)
  - [ ] BMI invalidation on source change triggers recompilation of dependents

## REQ-MOD-004: import std

- **Priority**: Could
- **Status**: Draft
- **Description**: Support for `import std;` when `[modules] import-std = true`.
- **Acceptance Criteria**:
  - [ ] Detects compiler support for `import std`
  - [ ] Generates the std module BMI using compiler-specific mechanisms
  - [ ] Works with Clang 18+, GCC 14+, MSVC 17.5+
  - [ ] Clear error if the compiler version does not support `import std`
- **Dependencies**: REQ-MOD-001
