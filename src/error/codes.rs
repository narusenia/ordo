#![allow(dead_code)]

// E00xx — Configuration / Manifest
pub const E0001: &str = "E0001"; // Failed to read Ordo.toml
pub const E0002: &str = "E0002"; // TOML parse error
pub const E0003: &str = "E0003"; // Manifest validation error
pub const E0004: &str = "E0004"; // Config resolution error

// E01xx — Dependency Resolution
pub const E0100: &str = "E0100"; // Dependency not found
pub const E0101: &str = "E0101"; // Version conflict
pub const E0102: &str = "E0102"; // Provider not specified (ambiguous)
pub const E0103: &str = "E0103"; // Lock file out of sync
pub const E0104: &str = "E0104"; // Hash mismatch

// E02xx — Build
pub const E0200: &str = "E0200"; // Ninja not found
pub const E0201: &str = "E0201"; // Compilation failed
pub const E0202: &str = "E0202"; // Linking failed
pub const E0203: &str = "E0203"; // Build artifact not found

// E03xx — Toolchain
pub const E0300: &str = "E0300"; // No compiler found
pub const E0301: &str = "E0301"; // Compiler version too old
pub const E0302: &str = "E0302"; // Unsupported target triple
pub const E0303: &str = "E0303"; // Sysroot not found

// E04xx — Test
pub const E0400: &str = "E0400"; // Test binary failed to build
pub const E0401: &str = "E0401"; // Test execution failed
pub const E0402: &str = "E0402"; // No tests found
