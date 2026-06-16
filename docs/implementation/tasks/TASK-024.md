# TASK-024: Scripts Command

- **Related Requirements**: REQ-PROJ-005
- **Milestone**: M4 — Testing & Quality
- **Size**: S
- **Dependencies**: TASK-002

## Summary

Implement `ordo run-script <name>` for executing user-defined scripts from `[scripts]`.

## Implementation Steps

1. Implement `src/cli/run_script.rs`:
   - Parse `[scripts]` from `Ordo.toml`
   - `ordo run-script <name>`: look up script by name, execute as shell command
   - Working directory: project root
   - Forward exit code from the script
2. Error handling:
   - Unknown script name → list available scripts
   - Empty `[scripts]` section → "no scripts defined"
3. Write tests

## Target Files

- `src/cli/run_script.rs`
