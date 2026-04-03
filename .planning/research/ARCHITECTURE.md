# Research: Architecture for Calendar Access and Management

**Date:** 2026-04-03
**Milestone:** v1.1 Calendar Access and Management

## Proposed Integration Shape

1. Add `src/caldav/` as a sibling to `src/carddav/`.
2. Reuse `Config` username + app-password accessors for calendar auth.
3. Add calendar and event domain structs that carry:
   - resource identifiers
   - href
   - etag
   - parsed event fields
4. Build command helpers similar to `src/commands/contacts.rs`:
   - list calendars
   - create/update/delete calendar
   - list/get/create/update/delete event
5. Expose those helpers through:
   - clap CLI subcommands
   - GraphQL query/mutation resolvers in `src/mcp/graphql/`

## Likely Internal Layers

### Discovery Layer

- discover calendar home set
- discover available calendar collections
- detect collection metadata and supported component set

### Calendar Collection Layer

- list calendars
- create calendar
- update calendar display properties
- delete calendar

### Event Transport Layer

- time-range event query via `calendar-query REPORT`
- fetch full event resource by href via `calendar-multiget REPORT`
- create/update/delete calendar object resources with ETag handling

### Event Data Layer

- serialize `VCALENDAR` / `VEVENT`
- parse returned iCalendar into repo-native structs
- normalize attendee, recurrence, and alarm representations

### Surface Layer

- CLI flags and defaults for human usage
- GraphQL input/output types tuned for agent orchestration

## Build Order

- Foundation and discovery first
- iCalendar parsing/serialization second
- collection CRUD and event query/write transport third
- CLI surface fourth
- MCP surface and live validation last

That order matches the current codebase's successful pattern from CardDAV contact CRUD.

## Cross-Cutting Concerns

- Timezone handling must be explicit end-to-end
- All destructive writes should preserve href + etag semantics
- Recurring event UX must decide whether updates target the whole series or a single instance
- Live validation should cover both single-calendar and multi-calendar setups if possible

## Sources

- Fastmail API docs:
  - https://www.fastmail.com/dev/
- CalDAV collection and report architecture:
  - https://www.rfc-editor.org/rfc/rfc4791.html
- Fastmail CalDAV endpoint reference:
  - https://www.fastmail.help/hc/en-us/articles/1500000277502-Importing-users-into-an-account
