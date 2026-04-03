# Research: Pitfalls for Calendar Access and Management

**Date:** 2026-04-03
**Milestone:** v1.1 Calendar Access and Management

## Watch Out For

### 1. Assuming JMAP calendar support exists in Fastmail production

- Fastmail's current developer docs expose calendars via CalDAV, not JMAP.
- If milestone planning assumes JMAP parity with mail, implementation will drift immediately.

### 2. Treating calendar events like simple JSON records

- Event payloads are iCalendar resources, not plain JSON objects.
- Attendees, recurrence, reminders, and timezone behavior all live in RFC 5545 semantics.

### 3. Mishandling timezones and all-day events

- "Rest of today" and "this week" depend on the user's timezone.
- All-day events and floating times can produce incorrect filters or CLI output if treated as UTC timestamps blindly.

### 4. Underestimating recurrence complexity

- Recurrence rules are part of the requested scope.
- Updating a recurring series is different from editing a single occurrence.
- Range queries should avoid silently dropping recurring instances that need expansion or summary handling.

### 5. Ignoring scheduling side effects

- Fastmail documents CalDAV scheduling support for attendee invitations and RSVPs.
- Writing attendee-bearing events may send outbound scheduling messages; tests and destructive operations should account for that.

### 6. Losing href / etag metadata

- Like CardDAV, safe updates and deletes depend on preserving server resource URLs and ETags.
- Parsing without storing those values will make later CRUD flows fragile.

### 7. Overloading MCP with natural-language behavior

- The requested agent workflow is better served by explicit calendar tools plus agent reasoning outside the API.
- Embedding NL parsing into the MCP schema would make the contract less deterministic and harder to test.

## Prevention Strategy

- Phase 5: lock protocol and discovery assumptions early
- Phase 6: prove iCalendar parsing/serialization with tests before network writes
- Phase 7: validate ETag and report semantics against Fastmail
- Phase 8: make CLI defaults timezone-aware
- Phase 9: keep GraphQL inputs explicit and run live end-to-end validation

## Sources

- Fastmail API docs:
  - https://www.fastmail.com/dev/
- Fastmail CalDAV scheduling support:
  - https://www.fastmail.com/blog/announcing-caldav-scheduling-support-for-clients/
- CalDAV protocol:
  - https://www.rfc-editor.org/rfc/rfc4791.html
- CalDAV scheduling extensions:
  - https://datatracker.ietf.org/doc/html/rfc6638
- iCalendar core model:
  - https://www.rfc-editor.org/rfc/rfc5545.html
