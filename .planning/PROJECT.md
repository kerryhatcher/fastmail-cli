# Contact CRUD via CardDAV

## What This Is

fastmail-cli now supports contact create, update, and delete operations on top of the existing CardDAV read integration. These flows are exposed as CLI commands (`contacts create/update/delete`) and GraphQL mutations in the MCP server. Implements radiosilence/fastmail-cli#17.

## Core Value

Users can manage contacts (create, update, delete) without leaving the terminal or AI assistant, building on the existing CardDAV plumbing.

## Current State

- Shipped milestone: `v1.0` on 2026-04-03
- Delivered: CardDAV-backed contact CRUD in both CLI and MCP GraphQL surfaces
- Verification status: milestone audit passed; local tests and lint gates were green during completion
- Remaining risk: live Fastmail validation is still recommended for final confidence in server-specific CardDAV behavior

## Requirements

### Validated

- ✓ Contact list via CardDAV — existing
- ✓ Contact search via CardDAV — existing
- ✓ MCP server with GraphQL queries for contacts — existing
- ✓ CLI command structure with `contacts` subcommand — existing
- ✓ Contact struct carries href/etag from server — Validated in Phase 1: Contact Model Foundation
- ✓ Write-specific error variants (ContactNotFound, ContactConflict) — Validated in Phase 1: Contact Model Foundation
- ✓ Generate valid vCard 3.0 with FN, N, EMAIL, ORG, TEL, ADR, NOTE — Validated in Phase 2: vCard Serialization
- ✓ Line folding at 75 octets with CRLF, character escaping, UUID v4 UIDs — Validated in Phase 2: vCard Serialization
- ✓ CardDAV create/update/delete with ETag-guarded writes — Validated in Phase 3: CardDAV Write Operations
- ✓ CLI contact create/update/delete with partial updates and explicit delete confirmation — Validated in Phase 4: CLI & MCP Surfaces
- ✓ MCP `createContact`, `updateContact`, and `deleteContact` mutations — Validated in Phase 4: CLI & MCP Surfaces

### Active

- [ ] Multi-value email support for contact create/update
- [ ] Multi-value phone support for contact create/update
- [ ] TYPE parameter support for email and phone labels
- [ ] Address book selection for create operations
- [ ] Address book listing for user discovery

### Out of Scope

- Contact groups/categories management — adds complexity, not requested
- Contact photo/avatar upload — binary data handling, separate concern
- Batch operations (bulk create/delete) — can add later if needed
- Contact import/export (CSV, vCard file) — separate feature
- Interactive prompts for delete — using flag-based confirmation instead

## Context

- **Existing codebase:** Rust 2024 edition, async with tokio, clap for CLI, async-graphql for MCP
- **CardDAV client:** `src/carddav/mod.rs` now handles discovery, parsing, serialization, and CRUD write operations
- **Contact commands:** `src/commands/contacts.rs` now supports `list`, `search`, `create`, `update`, and `delete`
- **MCP GraphQL:** `src/mcp/graphql/` now exposes both contact queries and mutations
- **vCard format:** Contacts are written as vCard 3.0 for Fastmail compatibility
- **Issue:** radiosilence/fastmail-cli#17

## Constraints

- **Tech stack**: Rust, must follow existing patterns (clap derive, async-graphql, reqwest)
- **Protocol**: CardDAV (WebDAV + vCard) — PUT for create/update, DELETE for delete
- **Compatibility**: Must work with Fastmail's CardDAV server specifically
- **Auth**: Reuse existing authentication mechanism (app-specific password in config)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Partial updates for contact update | Better UX — user only specifies changed fields | ✓ Shipped in v1.0 |
| Flag-based delete confirmation (`--confirm`/`--yes`) | No interactive prompts, works in scripts and AI workflows | ✓ Shipped in v1.0 |
| Support name, email, org, phone, address, notes | Covers common contact fields without over-engineering | ✓ Shipped in v1.0 |
| Store ETag verbatim including surrounding quotes | Required for correct `If-Match` behavior against CardDAV servers | ✓ Shipped in v1.0 |
| Serialize `href` and `etag` in contact JSON/GraphQL output | Callers need raw server metadata for inspection and follow-on operations | ✓ Shipped in v1.0 |
| `ContactConflict.server_etag` remains optional | 412 responses may not include the server's latest ETag | ✓ Shipped in v1.0 |

## Next Milestone Goals

- Expand contact writes to support multi-value emails and phones
- Add richer TYPE handling for contact methods
- Let users choose an address book explicitly during create flows
- Keep the current CRUD flows stable while validating against live Fastmail behavior

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
*Last updated: 2026-04-03 after v1.0 milestone completion*
