# Security Guide for HyprFile

## Data Safety Principles

1. **Trash by default**: Deleting files sends them to the freedesktop trash, never permanently.
2. **Permanent deletion requires explicit confirmation**: A configurable phrase or strong confirmation dialog.
3. **No silent overwrites**: Conflicts offer replace, rename, skip, or cancel.
4. **Symlink safety**: Dangerous symlinks are never followed without explicit user consent.
5. **No UI blocking**: Heavy operations run in async job queue; the UI remains responsive.
6. **Sandboxed plugins**: Future plugin system will require explicit permissions before executing external commands.

## Operational Safety

- All destructive operations display a summary before execution.
- Operations maintain a log for recent actions (undo pending future design).
- Cancelled jobs stop safely without leaving partial file states.
- Permission errors are surfaced clearly to the user, never swallowed.

## Testing Safety

- All tests use temporary directories (`tempfile` crate).
- No test touches real user files in `$HOME`.
- Tests cover symlink cycles, permission denied, and missing paths.

## Reporting Vulnerabilities

Please report security issues privately to the maintainers.
