# Phase 6: iCalendar Event Semantics - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

## Scope

Build the internal event model plus iCalendar parsing/serialization helpers for the v1.1 event fields: title, start/end, timezone, location, description, attendees, recurrence, and reminders.

## Locked Decisions

- Reuse the CalDAV module rather than inventing a second event-serialization layer.
- Keep event identifiers stable via `UID`, while preserving `href` / `etag` for transport operations.
- Treat all-day dates and timezone-bearing timed events as first-class cases in both parsing and serialization.
