# Ordo — Implementation Plan

## Overview

The implementation is organized into 6 milestones, progressing from a minimal working tool to the full-featured orchestrator. Each milestone produces a usable increment.

## Milestones

| Milestone | Title | Tasks | Progress | Key Deliverable |
|-----------|-------|-------|----------|-----------------|
| M1 | Foundation + MVP | TASK-001 ~ 007 | 7/7 | `ordo new`, `ordo build`, `ordo run` work for a single executable project |
| M2 | Dependency Management | TASK-008 ~ 014 | 7/7 | All provider backends, lock file, `ordo add/update/tree` |
| M2.5 | Lua Build Scripts | TASK-037 ~ 039 | 0/3 | Lua scripting for git deps, sandbox, caching |
| M3 | Workspace & Profiles | TASK-015 ~ 019 | 2/5 | Workspace support, full profile options, features |
| M4 | Testing & Quality | TASK-020 ~ 024 | 0/5 | `ordo test/fmt/lint/check`, compile_commands.json |
| M5 | Advanced Build | TASK-025 ~ 029 | 0/5 | C++ modules, cross-compilation, cache, watch mode |
| M6 | Packaging & Ecosystem | TASK-030 ~ 036 | 0/7 | Install, package, registry, IDE generation, CI, doctor, self update |

## Progress Tracker

### M1: Foundation + MVP

| Task | Title | Status | Size |
|------|-------|--------|------|
| TASK-001 | Project Skeleton and CLI Framework | Done | M |
| TASK-002 | Manifest Parser and Config System | Done | L |
| TASK-003 | Error System | Done | M |
| TASK-004 | Project Scaffolding (new / init) | Done | M |
| TASK-005 | Compiler Abstraction and Ninja Generator | Done | L |
| TASK-006 | Build Command | Done | M |
| TASK-007 | Run and Clean Commands | Done | S |

### M2: Dependency Management

| Task | Title | Status | Size |
|------|-------|--------|------|
| TASK-008 | Dependency Declaration and Resolver | Done | L |
| TASK-009 | Lock File | Done | M |
| TASK-010 | Passive Providers (pkg-config, system) | Done | M |
| TASK-011 | vcpkg Provider (Active) | Done | L |
| TASK-012 | Conan Provider (Active) | Done | M |
| TASK-013 | Git Provider | Done | L |
| TASK-014 | Dependency CLI Commands (add, update, tree) | Done | M |

### M2.5: Lua Build Scripts

| Task | Title | Status | Size |
|------|-------|--------|------|
| TASK-037 | Lua Runtime Integration and Sandbox | Todo | L |
| TASK-038 | Lua Script API (exec, file helpers, context) | Todo | M |
| TASK-039 | Git Provider Lua Integration and Caching | Todo | M |

### M3: Workspace & Profiles

| Task | Title | Status | Size |
|------|-------|--------|------|
| TASK-015 | Workspace Discovery and Configuration | Done | L |
| TASK-016 | Workspace Build Integration | Done | L |
| TASK-017 | Full Build Profile Support | Todo | M |
| TASK-018 | Feature Flags | Todo | M |
| TASK-019 | Dev Dependencies | Todo | S |

### M4: Testing & Quality

| Task | Title | Status | Size |
|------|-------|--------|------|
| TASK-020 | Test Framework | Todo | L |
| TASK-021 | Format Command | Todo | S |
| TASK-022 | Lint Command and compile_commands.json | Todo | M |
| TASK-023 | Check Command | Todo | S |
| TASK-024 | Scripts Command | Todo | S |

### M5: Advanced Build

| Task | Title | Status | Size |
|------|-------|--------|------|
| TASK-025 | C++ Modules Support | Todo | L |
| TASK-026 | Cross-Compilation and Toolchain Commands | Todo | M |
| TASK-027 | Unity Build and Precompiled Headers | Todo | M |
| TASK-028 | Watch Mode and Cache Integration | Todo | M |
| TASK-029 | Security — Integrity Verification and CI Flags | Todo | M |

### M6: Packaging & Ecosystem

| Task | Title | Status | Size |
|------|-------|--------|------|
| TASK-030 | Install Command with pkg-config/CMake Config | Todo | M |
| TASK-031 | Package Command | Todo | S |
| TASK-032 | Ordo Registry (Client + Server Foundation) | Todo | L |
| TASK-033 | IDE Generation and CMake Compatibility | Todo | M |
| TASK-034 | Doctor, Config Show, and Self Update | Todo | M |
| TASK-035 | CI Command and CI Template Generation | Todo | M |
| TASK-036 | Toolchain Install (Future Foundation) | Todo | S |

## Implementation Order

```
M1: Foundation + MVP
  TASK-001 → TASK-002 → TASK-003 → TASK-004 → TASK-005 → TASK-006 → TASK-007
                                      ↓
M2: Dependency Management
  TASK-008 → TASK-009 → TASK-010 → TASK-011 → TASK-012 → TASK-013 → TASK-014
                                                             ↓
M2.5: Lua Build Scripts
  TASK-037 → TASK-038 → TASK-039
                            ↓
M3: Workspace & Profiles
  TASK-015 → TASK-016 → TASK-017 → TASK-018 → TASK-019
                                                  ↓
M4: Testing & Quality
  TASK-020 → TASK-021 → TASK-022 → TASK-023 → TASK-024
                                                  ↓
M5: Advanced Build
  TASK-025 → TASK-026 → TASK-027 → TASK-028 → TASK-029
                                                  ↓
M6: Packaging & Ecosystem
  TASK-030 → TASK-031 → TASK-032 → TASK-033 → TASK-034 → TASK-035 → TASK-036
```

## Traceability Matrix

| Task | Status | Requirements |
|------|--------|-------------|
| TASK-001 | Done | REQ-CLI-001 (partial), REQ-CLI-005 |
| TASK-002 | Done | REQ-PROJ-003, REQ-PROJ-004, REQ-CLI-004 |
| TASK-003 | Done | REQ-CLI-002, REQ-CLI-003 |
| TASK-004 | Done | REQ-PROJ-001, REQ-PROJ-002 |
| TASK-005 | Done | REQ-BUILD-003, REQ-BUILD-006 |
| TASK-006 | Done | REQ-BUILD-001, REQ-BUILD-004 (partial: dev/release only) |
| TASK-007 | Done | REQ-BUILD-002, REQ-BUILD-009 |
| TASK-008 | Done | REQ-DEPS-001, REQ-DEPS-003 |
| TASK-009 | Done | REQ-DEPS-004 |
| TASK-010 | Done | REQ-DEPS-007, REQ-DEPS-008 |
| TASK-011 | Done | REQ-DEPS-005 |
| TASK-012 | Todo | REQ-DEPS-006 |
| TASK-013 | Todo | REQ-DEPS-001 (git) |
| TASK-014 | Todo | REQ-DEPS-009 |
| TASK-015 | Todo | REQ-WORK-001, REQ-WORK-002, REQ-WORK-003 |
| TASK-016 | Todo | REQ-WORK-004, REQ-WORK-005 |
| TASK-017 | Todo | REQ-BUILD-004 (full), REQ-BUILD-005 |
| TASK-018 | Todo | REQ-PROJ-006 |
| TASK-019 | Todo | REQ-DEPS-002 |
| TASK-020 | Todo | REQ-TEST-001, REQ-TEST-002, REQ-TEST-003, REQ-TEST-004 |
| TASK-021 | Todo | REQ-QUAL-001 |
| TASK-022 | Todo | REQ-QUAL-002, REQ-QUAL-004 |
| TASK-023 | Todo | REQ-BUILD-010 |
| TASK-024 | Todo | REQ-PROJ-005 |
| TASK-025 | Todo | REQ-MOD-001, REQ-MOD-002, REQ-MOD-003, REQ-MOD-004 |
| TASK-026 | Todo | REQ-TOOL-001, REQ-TOOL-002, REQ-TOOL-004 |
| TASK-027 | Todo | REQ-BUILD-007, REQ-BUILD-008 |
| TASK-028 | Todo | REQ-CLI-006 |
| TASK-029 | Todo | REQ-SEC-001, REQ-SEC-002 |
| TASK-030 | Todo | REQ-PKG-001, REQ-PKG-004, REQ-PKG-005 |
| TASK-031 | Todo | REQ-PKG-002 |
| TASK-032 | Todo | REQ-PKG-006, REQ-PKG-003 |
| TASK-033 | Todo | REQ-IDE-002, REQ-IDE-003, REQ-IDE-004, REQ-IDE-005 |
| TASK-034 | Todo | REQ-CLI-007, REQ-CLI-008 |
| TASK-035 | Todo | REQ-SEC-005, REQ-PROJ-005 (CI steps) |
| TASK-036 | Todo | REQ-TOOL-003 |

| TASK-037 | Todo | REQ-LUA-002, REQ-LUA-007 |
| TASK-038 | Todo | REQ-LUA-003, REQ-LUA-004, REQ-LUA-005, REQ-LUA-006 |
| TASK-039 | Todo | REQ-LUA-001, REQ-LUA-008 |

## Untraced Requirements (Future / Placeholder)

| Requirement | Status |
|-------------|--------|
| REQ-QUAL-003 (ordo analyze) | Future — placeholder command only |
| REQ-SEC-003 (registry security) | Addressed partially in TASK-032 |
| REQ-SEC-004 (ordo audit) | Future — placeholder command only |
| REQ-TEST-005 (ordo bench) | Future |
| REQ-LUA-009 (script sharing via registry) | Future — depends on REQ-PKG-006 |
| REQ-LUA-010 (general-purpose Lua scripts) | Future — extends REQ-PROJ-005 |
