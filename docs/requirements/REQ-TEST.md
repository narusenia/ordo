# REQ-TEST — Testing

## REQ-TEST-001: Test Execution

- **Priority**: Must
- **Status**: Draft
- **Description**: `ordo test` builds and runs test binaries from the `tests/` directory.
- **Acceptance Criteria**:
  - [ ] Discovers test sources in the configured `[test] src` directory (default: `tests/`)
  - [ ] Builds one binary per test source file
  - [ ] Executes all test binaries and reports pass/fail
  - [ ] Returns non-zero exit code if any test fails
  - [ ] Supports `--jobs <n>` for parallel test execution

## REQ-TEST-002: Framework Auto-Detection

- **Priority**: Should
- **Status**: Draft
- **Description**: Ordo detects the test framework by scanning `#include` directives in test sources.
- **Acceptance Criteria**:
  - [ ] `[test] framework = "auto"` (default) triggers auto-detection
  - [ ] Detects GoogleTest (`gtest/gtest.h`), Catch2 (`catch2/catch.hpp`), doctest (`doctest/doctest.h`)
  - [ ] `"plain"` mode: no framework, test passes if exit code is 0
  - [ ] Explicit override: `[test] framework = "googletest"`
  - [ ] Links the correct framework library automatically when detected

## REQ-TEST-003: Test Filtering

- **Priority**: Should
- **Status**: Draft
- **Description**: `ordo test --filter <pattern>` runs a subset of tests.
- **Acceptance Criteria**:
  - [ ] Pattern matches against test binary names or (where framework supports it) test case names
  - [ ] Forwards filter to underlying framework (e.g., `--gtest_filter` for GoogleTest)
  - [ ] Glob-style matching for binary selection

## REQ-TEST-004: Test Library Extraction

- **Priority**: Must
- **Status**: Draft
- **Description**: For executable projects, all sources except `main.cpp` are compiled into an internal library that test binaries link against.
- **Acceptance Criteria**:
  - [ ] `src/main.cpp` (or the configured entry point) is excluded from the test library
  - [ ] All other `src/` files are compiled into a static library
  - [ ] Test binaries link against this library plus test framework
  - [ ] Library projects expose their public API directly to tests

## REQ-TEST-005: Benchmark Command

- **Priority**: Could
- **Status**: Draft
- **Description**: `ordo bench` builds and runs benchmarks (future feature).
- **Acceptance Criteria**:
  - [ ] Separate benchmark source directory or annotation
  - [ ] Builds with `bench` profile (inherits `release` + debug symbols)
  - [ ] Auto-detects Google Benchmark / Catch2 benchmark
  - [ ] Reports timing results
