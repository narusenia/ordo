# TASK-016: Workspace Build Integration

- **Related Requirements**: REQ-WORK-004, REQ-WORK-005
- **Milestone**: M3 — Workspace & Profiles
- **Size**: L
- **Dependencies**: TASK-015, TASK-006

## Summary

Extend the build system to generate a single `build.ninja` for the entire workspace and support `-p <member>` scoping.

## Implementation Steps

1. Extend Ninja generator for workspace mode:
   - Generate one `build.ninja` containing all members
   - Inter-member dependencies expressed as Ninja build edges
   - Shared `target/` at workspace root
   - Member artifacts in `target/<profile>/<member-name>/`
2. Implement topological sort for build ordering:
   - Use DAG from TASK-015
   - Detect cycles → error with clear message
3. Implement `-p <member>` flag:
   - Scope `build`, `test`, `fmt`, `lint` to a specific member + its dependencies
   - When unspecified, operate on all members
4. Implement workspace-aware `ordo clean`:
   - Removes single `target/` at root
5. Write integration tests:
   - Multi-member workspace build
   - Correct build order
   - `-p` scoping

## Target Files

- `src/backend/ninja.rs` (extend)
- `src/cli/build.rs` (extend)
