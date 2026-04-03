# Fastmail CLI for Mail, Contacts, and Calendars

## What This Is

fastmail-cli is a terminal and AI-facing Fastmail integration that already covers JMAP mail workflows and CardDAV contact CRUD. The next milestone expands it into calendar management so users and AI agents can inspect schedules and create, update, and delete calendars and events without leaving the CLI or MCP server.

## Core Value

Users can manage Fastmail data without leaving the terminal or AI assistant, with automation-friendly APIs that stay faithful to Fastmail's actual protocol boundaries.

## Current State

- Shipped milestone: `v1.0` on 2026-04-03
- Delivered: CardDAV-backed contact CRUD in both CLI and MCP GraphQL surfaces
- Verification status: milestone audit passed; local tests and lint gates were green during completion
- Remaining risk: live Fastmail validation is still recommended for final confidence in server-specific CardDAV behavior

## Requirements

### Validated

- ✓ Mail listing, search, read, send, reply, forward, move, spam/read management via JMAP — existing
- ✓ Contact list and search via CardDAV — existing
- ✓ MCP server with GraphQL queries for mail and contacts — existing
- ✓ CLI command structure with mail and `contacts` subcommands — existing
- ✓ Contact struct carries href/etag from server — Validated in Phase 1: Contact Model Foundation
- ✓ Write-specific error variants (ContactNotFound, ContactConflict) — Validated in Phase 1: Contact Model Foundation
- ✓ Generate valid vCard 3.0 with FN, N, EMAIL, ORG, TEL, ADR, NOTE — Validated in Phase 2: vCard Serialization
- ✓ Line folding at 75 octets with CRLF, character escaping, UUID v4 UIDs — Validated in Phase 2: vCard Serialization
- ✓ CardDAV create/update/delete with ETag-guarded writes — Validated in Phase 3: CardDAV Write Operations
- ✓ CLI contact create/update/delete with partial updates and explicit delete confirmation — Validated in Phase 4: CLI & MCP Surfaces
- ✓ MCP `createContact`, `updateContact`, and `deleteContact` mutations — Validated in Phase 4: CLI & MCP Surfaces

### Active

- [ ] Full CalDAV calendar discovery and calendar CRUD against Fastmail
- [ ] Event listing with default future-today behavior, week views, and explicit date ranges
- [ ] Event get/create/update/delete flows with title, start/end, timezone, location, description, attendees, recurrence, and reminders
- [ ] CLI calendar commands that make schedule inspection and event management scriptable
- [ ] MCP GraphQL calendar/event queries and mutations for AI-agent workflows
- [ ] Live Fastmail validation of calendar and event CRUD behavior

### Out of Scope

- Natural-language event extraction inside the API surface itself — agents can interpret email/prompt content and call explicit MCP mutations
- Full attendee inbox workflow automation beyond CalDAV scheduling side effects — separate product area
- Calendar sharing/ACL management — significant protocol and UX expansion beyond CRUD
- Free/busy planning assistant logic — useful later, but not required for milestone v1.1
- ICS subscription/feed management — adjacent calendar feature, not part of requested CRUD scope

## Context

- **Existing codebase:** Rust 2024 edition, async with tokio, clap for CLI, async-graphql for MCP
- **Mail surface:** `src/jmap/` powers email and masked-email flows using API-token auth
- **Contact surface:** `src/carddav/mod.rs` already handles WebDAV discovery, XML parsing, serialization, and ETag-guarded CRUD with app-password auth
- **MCP GraphQL:** `src/mcp/graphql/` exposes GraphQL queries and mutations over the underlying mail/contact clients
- **Fastmail protocol boundary:** Fastmail developer docs currently expose calendars via CalDAV and app passwords, not via JMAP
- **Calendar data model:** Event CRUD will need iCalendar (`VCALENDAR` / `VEVENT`) parsing and serialization, not JSON-only transport
- **Scheduling behavior:** Fastmail documents CalDAV scheduling support, so attendee changes may trigger invite / RSVP side effects during event writes
- **Validation target:** This milestone explicitly requires live Fastmail verification, not only local fixtures

## Constraints

- **Tech stack**: Rust, must follow existing patterns (clap derive, async-graphql, reqwest, roxmltree)
- **Protocol**: Fastmail calendars must use CalDAV + iCalendar today; do not assume JMAP calendar support exists in production
- **Auth**: Calendar access must use Fastmail username + app password / OAuth-for-non-JMAP patterns, consistent with contacts
- **Compatibility**: Must work with Fastmail's CalDAV server specifically, including scheduling behavior
- **Data correctness**: Recurrence, attendees, reminders, and timezone semantics must round-trip safely
- **Safety**: Destructive mutations in MCP should preserve explicit confirmation patterns where appropriate

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Partial updates for contact update | Better UX — user only specifies changed fields | ✓ Shipped in v1.0 |
| Flag-based delete confirmation (`--confirm`/`--yes`) | No interactive prompts, works in scripts and AI workflows | ✓ Shipped in v1.0 |
| Support name, email, org, phone, address, notes | Covers common contact fields without over-engineering | ✓ Shipped in v1.0 |
| Store ETag verbatim including surrounding quotes | Required for correct `If-Match` behavior against CardDAV servers | ✓ Shipped in v1.0 |
| Serialize `href` and `etag` in contact JSON/GraphQL output | Callers need raw server metadata for inspection and follow-on operations | ✓ Shipped in v1.0 |
| `ContactConflict.server_etag` remains optional | 412 responses may not include the server's latest ETag | ✓ Shipped in v1.0 |
| Implement calendar support via a sibling CalDAV layer, not JMAP | Fastmail's published API surface exposes calendars over CalDAV today | — Pending |
| Keep MCP calendar actions explicit rather than natural-language | Lets AI agents compose higher-level behavior without baking brittle interpretation into the API | — Pending |

## Current Milestone: v1.1 Calendar Access and Management

**Goal:** Add full Fastmail calendar and event management to the CLI and MCP so users and AI agents can inspect schedules and CRUD calendars/events from terminal workflows.

**Target features:**
- Calendar object management: list, create, rename/update, delete
- Event management: list, inspect, create, update, delete
- CLI event listing defaults for future events today, plus week and explicit-range views
- Event fields for v1.1: title, start/end, timezone, location, description, attendees, recurrence, reminders
- Minimal MCP GraphQL calendar/event operations for agent workflows
- Live Fastmail validation for calendar and event CRUD flows

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-03 after starting milestone v1.1*
