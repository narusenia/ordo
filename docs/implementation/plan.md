# Ordo — Implementation Plan

## Overview

The implementation is organized into 6 milestones, progressing from a minimal working tool to the full-featured orchestrator. Each milestone produces a usable increment.

## Milestones

| Milestone | Title | Tasks | Key Deliverable |
|-----------|-------|-------|-----------------|
| M1 | Foundation + MVP | TASK-001 ~ 007 | `ordo new`, `ordo build`, `ordo run` work for a single executable project |
| M2 | Dependency Management | TASK-008 ~ 014 | All provider backends, lock file, `ordo add/update/tree` |
| M3 | Workspace & Profiles | TASK-015 ~ 019 | Workspace support, full profile options, features |
| M4 | Testing & Quality | TASK-020 ~ 024 | `ordo test/fmt/lint/check`, compile_commands.json |
| M5 | Advanced Build | TASK-025 ~ 029 | C++ modules, cross-compilation, cache, watch mode |
| M6 | Packaging & Ecosystem | TASK-030 ~ 036 | Install, package, registry, IDE generation, CI, doctor, self update |

## Implementation Order

```
M1: Foundation + MVP
  TASK-001 → TASK-002 → TASK-003 → TASK-004 → TASK-005 → TASK-006 → TASK-007
                                      ↓
M2: Dependency Management
  TASK-008 → TASK-009 → TASK-010 → TASK-011 → TASK-012 → TASK-013 → TASK-014
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

| Task | Requirements |
|------|-------------|
| TASK-001 | REQ-CLI-001 (partial), REQ-CLI-005 |
| TASK-002 | REQ-PROJ-003, REQ-PROJ-004, REQ-CLI-004 |
| TASK-003 | REQ-CLI-002, REQ-CLI-003 |
| TASK-004 | REQ-PROJ-001, REQ-PROJ-002 |
| TASK-005 | REQ-BUILD-003, REQ-BUILD-006 |
| TASK-006 | REQ-BUILD-001, REQ-BUILD-004 (partial: dev/release only) |
| TASK-007 | REQ-BUILD-002, REQ-BUILD-009 |
| TASK-008 | REQ-DEPS-001, REQ-DEPS-003 |
| TASK-009 | REQ-DEPS-004 |
| TASK-010 | REQ-DEPS-007, REQ-DEPS-008 |
| TASK-011 | REQ-DEPS-005 |
| TASK-012 | REQ-DEPS-006 |
| TASK-013 | REQ-DEPS-001 (git) |
| TASK-014 | REQ-DEPS-009 |
| TASK-015 | REQ-WORK-001, REQ-WORK-002, REQ-WORK-003 |
| TASK-016 | REQ-WORK-004, REQ-WORK-005 |
| TASK-017 | REQ-BUILD-004 (full), REQ-BUILD-005 |
| TASK-018 | REQ-PROJ-006 |
| TASK-019 | REQ-DEPS-002 |
| TASK-020 | REQ-TEST-001, REQ-TEST-002, REQ-TEST-003, REQ-TEST-004 |
| TASK-021 | REQ-QUAL-001 |
| TASK-022 | REQ-QUAL-002, REQ-QUAL-004 |
| TASK-023 | REQ-BUILD-010 |
| TASK-024 | REQ-PROJ-005 |
| TASK-025 | REQ-MOD-001, REQ-MOD-002, REQ-MOD-003, REQ-MOD-004 |
| TASK-026 | REQ-TOOL-001, REQ-TOOL-002, REQ-TOOL-004 |
| TASK-027 | REQ-BUILD-007, REQ-BUILD-008 |
| TASK-028 | REQ-CLI-006 |
| TASK-029 | REQ-SEC-001, REQ-SEC-002 |
| TASK-030 | REQ-PKG-001, REQ-PKG-004, REQ-PKG-005 |
| TASK-031 | REQ-PKG-002 |
| TASK-032 | REQ-PKG-006, REQ-PKG-003 |
| TASK-033 | REQ-IDE-002, REQ-IDE-003, REQ-IDE-004, REQ-IDE-005 |
| TASK-034 | REQ-CLI-007, REQ-CLI-008 |
| TASK-035 | REQ-SEC-005, REQ-PROJ-005 (CI steps) |
| TASK-036 | REQ-TOOL-003 |

## Untraced Requirements (Future / Placeholder)

| Requirement | Status |
|-------------|--------|
| REQ-QUAL-003 (ordo analyze) | Future — placeholder command only |
| REQ-SEC-003 (registry security) | Addressed partially in TASK-032 |
| REQ-SEC-004 (ordo audit) | Future — placeholder command only |
| REQ-TEST-005 (ordo bench) | Future |
