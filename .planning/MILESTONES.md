# Milestones

## v1.1 Calendar Access and Management (Shipped: 2026-04-04)

**Phases completed:** 7 phases, 7 plans, 0 tasks

**Key accomplishments:**

- CalDAV foundation with Fastmail calendar-home discovery and collection listing (Phase 5)
- iCalendar parsing/serialization for events, attendees, recurrence, and reminders (Phase 6)
- Full calendar & event CRUD transport with ETag-safe writes (Phase 7)
- CLI calendar experience with default-today, --week, and explicit range controls (Phase 8)
- MCP GraphQL surface for AI agent calendar operations, validated against live Fastmail (Phase 9)
- Explicit range contract hardening and CLI attendee clearing parity (Phases 10-11)

---

## v1.0 milestone (Shipped: 2026-04-03)

**Phases completed:** 4 phases, 4 plans, 9 tasks

**Key accomplishments:**

- Contact struct extended with server-assigned href/etag fields for CardDAV write operations, plus ContactNotFound and ContactConflict error variants for Phase 2-4 write operation error handling
- CardDAV write operations now support create, update, and delete with correct conditional header handling and error mapping.
- CLI and MCP now expose contact create, update, and delete flows with shared partial-update logic and explicit delete confirmation.

---
