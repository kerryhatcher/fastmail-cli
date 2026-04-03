# Requirements: Calendar Access and Management

**Defined:** 2026-04-03
**Core Value:** Users can manage Fastmail data without leaving the terminal or AI assistant, with automation-friendly APIs that stay faithful to Fastmail's actual protocol boundaries

## v1 Requirements

### Calendar Collections

- [ ] **CAL-01**: User can list all calendars available in their Fastmail account
- [ ] **CAL-02**: User can create a new calendar with a display name
- [ ] **CAL-03**: User can rename or update basic metadata on an existing calendar
- [ ] **CAL-04**: User can delete an existing calendar

### Event Discovery

- [ ] **EVT-01**: User can list future events for the rest of the current day without supplying a date range
- [ ] **EVT-02**: User can list events for the current week using a convenience flag
- [ ] **EVT-03**: User can list events for an explicit start/end range
- [ ] **EVT-04**: User can filter event listings to a specific calendar
- [ ] **EVT-05**: User can fetch a single event with full stored details

### Event CRUD

- [ ] **EVT-06**: User can create a one-off event with title, start, end, and timezone
- [ ] **EVT-07**: User can set or update an event's location and description
- [ ] **EVT-08**: User can add, update, and remove attendees on an event
- [ ] **EVT-09**: User can add, update, and remove recurrence on an event
- [ ] **EVT-10**: User can add, update, and remove reminders on an event
- [ ] **EVT-11**: User can update an existing event safely without overwriting concurrent server changes
- [ ] **EVT-12**: User can delete an event

### CLI Experience

- [ ] **CLI-01**: User can manage calendars and events from dedicated CLI subcommands without hand-writing CalDAV requests
- [ ] **CLI-02**: User can use both shortcut range flags (`--today`, `--week`) and explicit `--start` / `--end` filters for event listing
- [ ] **CLI-03**: CLI JSON output for calendars and events includes identifiers needed for follow-up CRUD actions

### MCP GraphQL

- [ ] **MCP-01**: AI agents can list calendars through MCP GraphQL
- [ ] **MCP-02**: AI agents can list events through MCP GraphQL using default and explicit date ranges
- [ ] **MCP-03**: AI agents can fetch a full event through MCP GraphQL
- [ ] **MCP-04**: AI agents can create and update calendars through MCP GraphQL
- [ ] **MCP-05**: AI agents can create and update events through MCP GraphQL
- [ ] **MCP-06**: AI agents can delete calendars and events through MCP GraphQL with explicit confirmation semantics

### Validation

- [ ] **VAL-01**: Calendar collection CRUD is validated against a live Fastmail account
- [ ] **VAL-02**: Event list/get/create/update/delete flows are validated against a live Fastmail account
- [ ] **VAL-03**: Live validation confirms attendee, recurrence, and reminder fields round-trip without unexpected data loss

## v2 Requirements

### Calendar Intelligence

- **INT-01**: User can query free/busy availability across a time range
- **INT-02**: User can ask the system to suggest meeting times

### Calendar Extensions

- **EXT-01**: User can subscribe to remote ICS / CalDAV calendars
- **EXT-02**: User can manage calendar sharing / ACL permissions
- **EXT-03**: MCP exposes a high-level "create event from text/email" helper

## Out of Scope

| Feature | Reason |
|---------|--------|
| JMAP calendar implementation | Fastmail publicly exposes calendars via CalDAV today |
| Built-in natural-language parsing in MCP calendar tools | Keep the API explicit and testable; agents can interpret email/prompt content externally |
| Calendar sharing / ACL administration | Larger scope than the requested CRUD baseline |
| Free/busy planning assistant flows | Useful follow-on, but not required to ship calendar CRUD |
| ICS feed subscription management | Adjacent import/sync feature, not part of the requested milestone |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| CAL-01 | Phase 5 | Pending |
| CAL-02 | Phase 7 | Pending |
| CAL-03 | Phase 7 | Pending |
| CAL-04 | Phase 7 | Pending |
| EVT-01 | Phase 8 | Pending |
| EVT-02 | Phase 8 | Pending |
| EVT-03 | Phase 8 | Pending |
| EVT-04 | Phase 8 | Pending |
| EVT-05 | Phase 7 | Pending |
| EVT-06 | Phase 7 | Pending |
| EVT-07 | Phase 6 | Pending |
| EVT-08 | Phase 6 | Pending |
| EVT-09 | Phase 6 | Pending |
| EVT-10 | Phase 6 | Pending |
| EVT-11 | Phase 7 | Pending |
| EVT-12 | Phase 7 | Pending |
| CLI-01 | Phase 8 | Pending |
| CLI-02 | Phase 8 | Pending |
| CLI-03 | Phase 8 | Pending |
| MCP-01 | Phase 9 | Pending |
| MCP-02 | Phase 9 | Pending |
| MCP-03 | Phase 9 | Pending |
| MCP-04 | Phase 9 | Pending |
| MCP-05 | Phase 9 | Pending |
| MCP-06 | Phase 9 | Pending |
| VAL-01 | Phase 9 | Pending |
| VAL-02 | Phase 9 | Pending |
| VAL-03 | Phase 9 | Pending |

**Coverage:**
- v1 requirements: 28 total
- Mapped to phases: 28
- Unmapped: 0 ✓

---
*Requirements defined: 2026-04-03*
*Last updated: 2026-04-03 after initial milestone definition*
