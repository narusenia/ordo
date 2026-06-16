# TASK-032: Ordo Registry (Client + Server Foundation)

- **Related Requirements**: REQ-PKG-006, REQ-PKG-003
- **Milestone**: M6 — Packaging & Ecosystem
- **Size**: L
- **Dependencies**: TASK-008, TASK-009

## Summary

Implement the registry client in Ordo and the foundation for the registry server.

## Implementation Steps

1. Implement `RegistryProvider` in `src/backend/provider/registry.rs`:
   - Fetch package metadata from registry API
   - Download source archives
   - Verify checksums
   - Cache registry index locally
2. Implement `ordo publish` in `src/cli/publish.rs`:
   - Load credentials from OS-appropriate path
   - Validate package metadata
   - Create source archive (exclude `target/`, `.git/`)
   - Upload to registry API
   - Handle duplicate version rejection
3. Implement `ordo yank` in `src/cli/publish.rs`:
   - `ordo yank --version <ver>`: mark version as yanked
4. Registry client features:
   - Scoped package names: `@org/pkg`
   - Token authentication via `credentials.toml`
   - HTTPS required
5. Registry server (separate project/repo, foundation only):
   - REST API: search, metadata, download, publish, yank
   - Git-based index
   - Source archive storage
   - Documented API specification
6. Write client tests with mock HTTP server

## Target Files

- `src/backend/provider/registry.rs`
- `src/cli/publish.rs`
