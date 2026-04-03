# Research: Features for Calendar Access and Management

**Date:** 2026-04-03
**Milestone:** v1.1 Calendar Access and Management

## Table Stakes

### Calendar Collections

- List all available calendars
- Create a new calendar with a display name
- Rename or update calendar metadata needed for basic management
- Delete a calendar

### Event Discovery

- List upcoming events for the rest of today by default
- List events for the current week
- List events for an explicit date/time range
- Filter by calendar when multiple calendars exist
- Fetch a single event with full details

### Event CRUD

- Create an event with title, start, end, timezone
- Update those same core fields safely
- Delete an event
- Preserve server href + etag so follow-up writes are safe

### Rich Event Fields

- Description / notes
- Location
- Multiple attendees
- Recurrence rules for repeating events
- Reminders / alarms

### MCP / Agent Support

- Explicit list/get/create/update/delete operations for calendars and events
- Outputs stable enough for an agent to read an email, extract details externally, then call MCP mutations

## Differentiators Worth Including In v1.1

- A friendly CLI default that answers "what's left today?" without requiring explicit date math
- Both convenience flags (`--today`, `--week`) and exact `--start` / `--end` range controls
- Live Fastmail validation rather than fixture-only confidence

## Deferred Features

- Free/busy search and meeting-slot suggestions
- ICS subscription/feed management
- Calendar sharing / ACL administration
- High-level "create event from text/email" MCP helper
- Complex recurrence exception editing UX beyond whole-rule updates

## Research Notes

- Fastmail's CalDAV scheduling support means attendee changes may trigger invitation / RSVP email side effects.
- Recurring-event support is important for the requested scope, but editing a single occurrence versus the whole series should be treated carefully in phase planning.
- A minimal MCP surface is the right fit here; agent reasoning belongs above the transport layer.

## Sources

- Fastmail CalDAV scheduling announcement:
  - https://www.fastmail.com/blog/announcing-caldav-scheduling-support-for-clients/
- Fastmail developer docs:
  - https://www.fastmail.com/dev/
- CalDAV reports and collection model:
  - https://www.rfc-editor.org/rfc/rfc4791.html
- iCalendar event/alarm/attendee/recurrence model:
  - https://www.rfc-editor.org/rfc/rfc5545.html
