# Fastmail CLI for Mail, Contacts, and Calendars

## What This Is

fastmail-cli is a terminal and AI-facing Fastmail integration covering JMAP mail workflows, CardDAV contact CRUD, and CalDAV calendar/event management. Users and AI agents can manage mail, contacts, and calendars without leaving the CLI or MCP server.

## Core Value

Users can manage Fastmail data without leaving the terminal or AI assistant, with automation-friendly APIs that stay faithful to Fastmail's actual protocol boundaries.

## Current State

- Shipped milestone: `v1.2` on 2026-04-05
- Delivered: Hardening & Quality — 32 in-scope codebase-review findings closed across safety, security, MCP refactor, performance, integration test coverage, and polish
- Previous: `v1.1` (2026-04-04) — CalDAV calendar and event management; `v1.0` (2026-04-03) — CardDAV contact CRUD
- All phases complete: `v1.3` Contact Groups — Phase 18 (group CRUD + CLI + MCP surfaces) + Phase 19 (membership management + --group flag) delivered
- Codebase: 206 tests passing, zero clippy warnings
- Known deferred: live-account integration validation for SIGTERM, OnceCell reuse, and CalDAV UID REPORT (Cyrus IMAP quirks)

## Requirements

### Validated

- ✓ Mail listing, search, read, send, reply, forward, move, spam/read management via JMAP — existing
- ✓ Contact list and search via CardDAV — existing
- ✓ MCP server with GraphQL queries for mail and contacts — existing
- ✓ CLI command structure with mail and `contacts` subcommands — existing
- ✓ Contact struct carries href/etag from server — v1.0
- ✓ Write-specific error variants (ContactNotFound, ContactConflict) — v1.0
- ✓ Generate valid vCard 3.0 with FN, N, EMAIL, ORG, TEL, ADR, NOTE — v1.0
- ✓ Line folding at 75 octets with CRLF, character escaping, UUID v4 UIDs — v1.0
- ✓ CardDAV create/update/delete with ETag-guarded writes — v1.0
- ✓ CLI contact create/update/delete with partial updates and explicit delete confirmation — v1.0
- ✓ MCP `createContact`, `updateContact`, and `deleteContact` mutations — v1.0
- ✓ Full CalDAV calendar discovery and calendar CRUD against Fastmail — v1.1
- ✓ Event listing with default future-today behavior, week views, and explicit date ranges — v1.1
- ✓ Event get/create/update/delete with title, start/end, timezone, location, description, attendees, recurrence, and reminders — v1.1
- ✓ CLI calendar commands that make schedule inspection and event management scriptable — v1.1
- ✓ MCP GraphQL calendar/event queries and mutations for AI-agent workflows — v1.1
- ✓ Live Fastmail validation of calendar and event CRUD behavior — v1.1

### Active

(Defined during v1.3 requirements step — see REQUIREMENTS.md)

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
| Implement calendar support via a sibling CalDAV layer, not JMAP | Fastmail's published API surface exposes calendars over CalDAV today | ✓ Shipped in v1.1 |
| Keep MCP calendar actions explicit rather than natural-language | Lets AI agents compose higher-level behavior without baking brittle interpretation into the API | ✓ Shipped in v1.1 |
| ETag-safe writes for calendar & event CRUD | Consistent with contact CRUD concurrency safety | ✓ Shipped in v1.1 |
| Retry calendar delete without If-Match on 412 | Fastmail returns 412 without replacement ETag in narrow case | ✓ Shipped in v1.1 |
| Require both --start and --end for explicit ranges | Avoids ambiguous one-bound semantics in event listing | ✓ Shipped in v1.1 |
| --clear-attendees flag conflicts with --attendee | Prevents ambiguous intent when clearing vs setting attendees | ✓ Shipped in v1.1 |

## Current Milestone: v1.3 Contact Groups

**Goal:** Users can manage contact groups (create, list, update, delete), add/remove contacts from groups, and assign a group at contact creation time — via both CLI and MCP/GraphQL.

**Target features:**
- Group CRUD (create, list/get, update, delete) via CardDAV
- Group membership management (add/remove contacts)
- `--group` flag on `contacts create` for group assignment at creation
- CLI commands for all group operations
- MCP/GraphQL mutations and queries for all group operations

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
*Last updated: 2026-04-13 — started milestone v1.3 Contact Groups*
