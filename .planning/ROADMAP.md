# Roadmap: Calendar Access and Management

## Overview

This milestone adds full Fastmail calendar and event management to fastmail-cli, but it must do so over CalDAV rather than the repo's existing JMAP mail path. The safest build order mirrors the successful contact CRUD milestone: establish protocol/discovery foundations first, then prove iCalendar event semantics locally, then add collection/event CRUD transport, then user-facing CLI flows, and finally MCP GraphQL plus live Fastmail verification. Phase numbering continues from the prior milestone, so the new work starts at Phase 5.

## Phases

**Phase Numbering:**
- Integer phases (5, 6, 7): Planned milestone work
- Decimal phases (6.1, 6.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 5: CalDAV Foundation & Discovery** - Add calendar auth/discovery primitives and calendar listing models needed by every later phase
- [ ] **Phase 6: iCalendar Event Semantics** - Build and verify event parsing/serialization for title, timing, location, description, attendees, recurrence, and reminders
- [ ] **Phase 7: Calendar & Event CRUD Transport** - Implement CalDAV collection/event create, update, delete, and detail retrieval with ETag-safe writes
- [ ] **Phase 8: CLI Calendar Experience** - Expose calendar and event workflows through CLI commands with default today/week/range listing UX
- [ ] **Phase 9: MCP Calendar Surface & Live Validation** - Add GraphQL calendar/event operations for AI agents and verify the full milestone against live Fastmail

## Phase Details

### Phase 5: CalDAV Foundation & Discovery
**Goal**: The codebase can authenticate to Fastmail's CalDAV endpoint, discover the user's calendar home and calendar collections, and return stable calendar metadata for later CRUD flows
**Depends on**: Nothing in this milestone
**Requirements**: CAL-01
**Success Criteria** (what must be TRUE):
1. The client can authenticate with Fastmail username + app password and discover the user's calendar home set
2. Listing calendars returns enough metadata to identify collections for later event queries and collection mutations
3. Unsupported assumptions about JMAP calendar access are removed from the milestone implementation path
4. Foundation tests prove XML parsing and discovery logic without requiring live network access
**Plans:** 0/1 plans complete

- [ ] 05-01-PLAN.md — CalDAV discovery, calendar collection model, config/auth reuse, and local tests

### Phase 6: iCalendar Event Semantics
**Goal**: Given raw iCalendar event resources, the codebase can parse and serialize the requested event fields correctly before network CRUD logic depends on them
**Depends on**: Phase 5
**Requirements**: EVT-07, EVT-08, EVT-09, EVT-10
**Success Criteria** (what must be TRUE):
1. A parsed event preserves title, start/end, timezone, location, and description
2. Multiple attendees round-trip through the internal event model without silent loss
3. Recurrence and reminder/alarm fields round-trip through serialization helpers for supported v1.1 shapes
4. Local tests cover all-day vs timed events, timezone-bearing events, and representative recurrence/reminder cases
**Plans:** 0/1 plans complete

- [ ] 06-01-PLAN.md — event model plus RFC 5545 parsing/serialization helpers with fixture-driven tests

### Phase 7: Calendar & Event CRUD Transport
**Goal**: The CalDAV client can create, update, delete, and fetch calendar collections and event resources safely against Fastmail's server
**Depends on**: Phase 6
**Requirements**: CAL-02, CAL-03, CAL-04, EVT-05, EVT-06, EVT-11, EVT-12
**Success Criteria** (what must be TRUE):
1. Creating a calendar produces a new collection that appears in later calendar listings
2. Renaming/updating a calendar persists the expected metadata change
3. Deleting a calendar removes it from later listings
4. Creating an event writes a valid calendar object resource with the requested fields
5. Fetching a single event returns full stored details plus href/etag metadata
6. Updating or deleting an event uses the current ETag and fails explicitly rather than silently clobbering concurrent changes
**Plans:** 0/1 plans complete

- [ ] 07-01-PLAN.md — collection CRUD plus event get/create/update/delete transport and error mapping

### Phase 8: CLI Calendar Experience
**Goal**: Users can inspect schedules and manage calendars/events from the terminal with convenient defaults and explicit range controls
**Depends on**: Phase 7
**Requirements**: EVT-01, EVT-02, EVT-03, EVT-04, CLI-01, CLI-02, CLI-03
**Success Criteria** (what must be TRUE):
1. Running the default event-listing command returns future events for the rest of today in JSON
2. A week flag returns the current week's events without manual date math
3. Explicit `--start` / `--end` ranges return matching events, optionally filtered to one calendar
4. CLI commands exist for calendar list/create/update/delete and event list/get/create/update/delete
5. CLI outputs include identifiers and metadata needed for follow-up edits or deletes
**Plans:** 0/1 plans complete

- [ ] 08-01-PLAN.md — clap surface, range-default UX, and JSON output contracts

### Phase 9: MCP Calendar Surface & Live Validation
**Goal**: AI agents can perform explicit calendar/event operations through MCP GraphQL, and the milestone is proven against a live Fastmail account
**Depends on**: Phase 8
**Requirements**: MCP-01, MCP-02, MCP-03, MCP-04, MCP-05, MCP-06, VAL-01, VAL-02, VAL-03
**Success Criteria** (what must be TRUE):
1. GraphQL queries can list calendars, list events by default/range, and fetch a full event
2. GraphQL mutations can create/update calendars and create/update events with the milestone fields
3. GraphQL delete flows require explicit confirmation semantics for destructive operations
4. A live Fastmail validation pass confirms collection CRUD and event CRUD work end-to-end
5. Live validation confirms attendee, recurrence, and reminder fields round-trip without unexpected corruption
**Plans:** 0/1 plans complete

- [ ] 09-01-PLAN.md — GraphQL calendar schema plus live Fastmail validation checklist and evidence capture

## Progress

**Execution Order:**
Phases execute in numeric order: 5 → 6 → 7 → 8 → 9

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 5. CalDAV Foundation & Discovery | 0/1 | Pending | - |
| 6. iCalendar Event Semantics | 0/1 | Pending | - |
| 7. Calendar & Event CRUD Transport | 0/1 | Pending | - |
| 8. CLI Calendar Experience | 0/1 | Pending | - |
| 9. MCP Calendar Surface & Live Validation | 0/1 | Pending | - |
