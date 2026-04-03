# Research: Stack for Calendar Access and Management

**Date:** 2026-04-03
**Milestone:** v1.1 Calendar Access and Management

## Recommendation

Use a new `caldav` module built with the same low-level stack already used for contacts:

- `reqwest` for authenticated HTTP/WebDAV requests
- `roxmltree` for `PROPFIND` / `REPORT` XML parsing
- custom iCalendar parsing/serialization helpers for `VCALENDAR` / `VEVENT`
- existing `Config` app-password auth path reused for calendars
- existing async-graphql schema pattern reused for MCP calendar queries and mutations

## Why this fits the repo

- Fastmail's published developer docs expose calendars via CalDAV today, not JMAP.
- The repo already has a successful CardDAV implementation using raw WebDAV calls and XML parsing.
- Calendar support needs iCalendar payload control for recurrence, attendees, reminders, and ETag-safe writes.

## Stack Additions

- New `src/caldav/` module for discovery, collection CRUD, event query/get, and event writes
- Event/calendar model structs in `src/models/` or `src/caldav/` for:
  - calendar collection metadata
  - event identifiers / href / etag
  - title, start, end, timezone
  - location, description
  - attendees
  - recurrence rules / overrides where supported
  - reminders / alarms
- Date/time utilities for:
  - RFC 5545 date-time formatting
  - "today" and "week" range expansion in user timezone
  - all-day vs timed event handling
- Shared confirmation-token pattern for destructive MCP mutations

## Protocol Requirements

- Discover calendar home and calendars via CalDAV/WebDAV properties
- Use `MKCALENDAR` for calendar creation where supported
- Use `PROPPATCH` for calendar metadata updates such as display name
- Use `DELETE` for calendar and event deletion
- Use `calendar-query REPORT` for time-range event listing
- Use `calendar-multiget REPORT` for fetching specific event resources
- Use ETags for optimistic concurrency on updates/deletes

## What Not To Add

- Do not depend on hypothetical Fastmail JMAP calendar support for milestone v1.1.
- Do not hide protocol semantics behind a natural-language API layer inside MCP.
- Do not introduce a heavyweight calendar crate unless manual RFC 5545 handling proves clearly unmaintainable.

## Sources

- Fastmail API docs: calendars are available via CalDAV; app passwords are used for non-JMAP protocols.
  - https://www.fastmail.com/dev/
- Fastmail help: manual calendar import endpoint is `https://caldav.fastmail.com`.
  - https://www.fastmail.help/hc/en-us/articles/1500000277502-Importing-users-into-an-account
- CalDAV core protocol: calendar collections, `MKCALENDAR`, `calendar-query`, `calendar-multiget`.
  - https://www.rfc-editor.org/rfc/rfc4791.html
- iCalendar core object model for `VEVENT`, `ATTENDEE`, `RRULE`, `VALARM`.
  - https://www.rfc-editor.org/rfc/rfc5545.html
