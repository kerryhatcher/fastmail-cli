# Phase 11: CLI Attendee Clearing Parity - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

## Scope

Close the CLI gap around attendee clearing so terminal users can intentionally send an empty attendee list during event updates without dropping to lower-layer tooling.

## Locked Decisions

- Add a dedicated `--clear-attendees` update flag instead of overloading an omitted or empty `--attendee` value.
- `--clear-attendees` must conflict with repeatable `--attendee` inputs so the intent stays unambiguous.
- Keep the underlying event patch semantics unchanged: the CLI should express parity with the existing lower-layer behavior, not invent new patch rules.
