---
phase: 11-cli-attendee-clearing-parity
plan: 01
requirements-completed:
  - EVT-08
  - CLI-01
completed: 2026-04-03
---

# Summary 11-01: CLI Attendee Clearing Parity

## Completed

- Added `--clear-attendees` to `events update` in `src/main.rs` and made it conflict with repeatable `--attendee` inputs.
- Wired the CLI update branch to send `EventPatch { attendees: Some(vec![]) }` when the clear flag is used while leaving omission semantics unchanged.
- Added automated parser coverage in `src/main.rs` for both successful `--clear-attendees` parsing and the `--attendee`/`--clear-attendees` conflict.
- Added patch-level unit coverage in `src/commands/events.rs` proving an explicit empty attendee patch clears attendees.
- Live-validated the new CLI path against Fastmail with an isolated tagged event that was created, cleared, fetched, and deleted.

## Outcome

Terminal users can now intentionally clear attendees through the CLI without dropping to MCP or lower-layer tooling, and the behavior matches the existing event patch semantics.
