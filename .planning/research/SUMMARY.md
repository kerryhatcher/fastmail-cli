# Research Summary: Calendar Access and Management

**Date:** 2026-04-03
**Milestone:** v1.1 Calendar Access and Management

## Stack additions

- Add a sibling `caldav` module using the existing `reqwest` + `roxmltree` pattern
- Reuse Fastmail username + app-password auth already used for contacts
- Add iCalendar parsing/serialization helpers for `VCALENDAR` / `VEVENT`
- Extend GraphQL schema with explicit calendar/event query and mutation types

## Feature table stakes

- Calendar list/create/update/delete
- Event list/get/create/update/delete
- Default future-today listing plus week and explicit range queries
- Event fields: title, start/end, timezone, location, description, attendees, recurrence, reminders
- Minimal MCP operations usable by AI agents
- Live Fastmail validation

## Architecture recommendation

Build calendar support as CalDAV, not JMAP. Reuse the current CardDAV architecture: protocol discovery and transport first, then domain serialization, then CLI/MCP surfaces. Continue phase numbering from the prior milestone and separate foundation, event semantics, transport, CLI, and MCP/live-validation concerns.

## Watch out for

- Fastmail calendars are CalDAV-only in current public docs
- iCalendar semantics are richer than the existing contact data model
- timezone and recurrence bugs will break the core "today/week" experience
- attendee writes may have real scheduling side effects
- href/etag handling must survive round-trips for safe updates/deletes

## Recommended milestone stance

Proceed with research-backed requirements for a CalDAV-based calendar milestone. Keep MCP explicit, keep natural-language reasoning outside the API contract, and reserve live Fastmail validation as a release gate rather than a nice-to-have.
