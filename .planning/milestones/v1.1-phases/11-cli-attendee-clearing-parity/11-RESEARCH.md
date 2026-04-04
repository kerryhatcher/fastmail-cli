---
phase: 11-cli-attendee-clearing-parity
generated: 2026-04-03
status: complete
---

# Phase 11 Research

## Phase Requirements

- `EVT-08`: users can add, update, and remove attendees on an event.
- `CLI-01`: calendar/event management must be available through the CLI surface without requiring lower-layer workarounds.

## Existing Patterns

- `src/commands/events.rs::EventPatch` already models attendee replacement as `Option<Vec<EventAttendee>>`.
- `apply_event_patch` already treats `Some(vec![])` as an intentional attendee clear and `None` as "leave attendees unchanged".
- GraphQL update mutations already expose this lower-layer behavior because `attendees: Some([])` flows through to `EventPatch`.
- The CLI update branch in `src/main.rs` currently converts attendee flags with `(!attendee.is_empty()).then(...)`, so there is no way to express `Some(vec![])`.

## Contract Decision

- Use a dedicated `--clear-attendees` flag on `events update`.

Why this is the right closure:

- It matches existing CLI conventions such as `--clear-recurrence` and `--clear-reminders`.
- It keeps "no attendee flags supplied" distinct from "replace attendees with an empty list".
- It avoids relying on awkward empty-string parsing or a magic attendee sentinel value.
- It preserves MCP and lower-layer parity without expanding scope beyond the CLI gap identified in the milestone audit.

## Implementation Direction

- Add `clear_attendees: bool` to `EventsCommands::Update` in `src/main.rs`.
- Make `--clear-attendees` conflict with repeatable `--attendee` so the caller cannot ask to replace and clear in the same command.
- In the update command branch, map:
  - `--clear-attendees` => `EventPatch { attendees: Some(vec![]) }`
  - one or more `--attendee` values => `EventPatch { attendees: Some(vec![...]) }`
  - neither => `EventPatch { attendees: None }`
- Add focused unit coverage in `src/commands/events.rs` proving an explicit empty attendee patch clears attendees while ordinary omitted patches preserve the existing list.

## Likely File Touch Points

- `src/main.rs`
  - add the `--clear-attendees` CLI flag
  - wire the update branch to emit `Some(vec![])` for attendee clearing
- `src/commands/events.rs`
  - add unit coverage around explicit attendee clearing semantics

## Verification Guidance

- `cargo test` should pass.
- CLI contract checks should cover:
  - update with repeatable `--attendee` still replaces attendees
  - update with `--clear-attendees` clears attendees
  - `--attendee` and `--clear-attendees` together are rejected by clap before any network call
