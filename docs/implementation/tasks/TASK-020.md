# TASK-020: Test Framework

- **Related Requirements**: REQ-TEST-001, REQ-TEST-002, REQ-TEST-003, REQ-TEST-004
- **Milestone**: M4 — Testing & Quality
- **Size**: L
- **Dependencies**: TASK-006

## Summary

Implement `ordo test` — discover, build, and run tests with framework auto-detection.

## Implementation Steps

1. Implement test discovery in `src/core/tester.rs`:
   - Scan `[test] src` directory for source files
   - One binary per test file
2. Implement framework detection:
   - Scan `#include` directives: `gtest/gtest.h` → GoogleTest, `catch2/catch.hpp` → Catch2, `doctest/doctest.h` → doctest
   - `"plain"` mode: no framework, compile and run directly
   - Explicit override via `[test] framework`
3. Implement test library extraction:
   - For executable projects: compile all `src/` files except `main.cpp` into a static library
   - Test binaries link against this library + test framework
   - Library projects: test binaries link against the project library
4. Extend Ninja generator with test build rules:
   - Separate test binary rules per test file
   - Link correct framework libraries
5. Implement test runner in `src/cli/test.rs`:
   - Build all test binaries
   - Execute in parallel (configurable `--jobs`)
   - `--filter <pattern>`: select test binaries by name, forward to framework filter
   - Collect pass/fail results, report summary
   - Non-zero exit code if any test fails
6. Write integration tests:
   - Plain test (no framework)
   - GoogleTest auto-detection (if available)

## Target Files

- `src/core/tester.rs`
- `src/cli/test.rs`
- `src/backend/ninja.rs` (extend)
