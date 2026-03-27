# Contact CRUD via CardDAV

## What This Is

Adding contact create, update, and delete operations to fastmail-cli, extending the existing CardDAV read-only integration. Exposes these as both CLI commands (`contacts create/update/delete`) and GraphQL mutations in the MCP server. Implements radiosilence/fastmail-cli#17.

## Core Value

Users can manage contacts (create, update, delete) without leaving the terminal or AI assistant, building on the existing CardDAV plumbing.

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

### Active

- [ ] Create contacts via CLI with name, email, organization, phone, address, notes fields
- [ ] Update contacts via CLI with partial updates (only modify fields explicitly passed)
- [ ] Delete contacts via CLI with `--confirm`/`--yes` flag requirement
- [ ] Expose `createContact` GraphQL mutation in MCP server
- [ ] Expose `updateContact` GraphQL mutation in MCP server
- [ ] Expose `deleteContact` GraphQL mutation in MCP server
- [ ] CardDAV write operations (PUT for create/update, DELETE for delete)

### Out of Scope

- Contact groups/categories management — adds complexity, not requested
- Contact photo/avatar upload — binary data handling, separate concern
- Batch operations (bulk create/delete) — can add later if needed
- Contact import/export (CSV, vCard file) — separate feature
- Interactive prompts for delete — using flag-based confirmation instead

## Context

- **Existing codebase:** Rust 2024 edition, async with tokio, clap for CLI, async-graphql for MCP
- **CardDAV client:** `src/carddav/mod.rs` handles contact discovery and vCard parsing via reqwest + roxmltree
- **Contact commands:** `src/commands/contacts.rs` has `list` and `search` subcommands
- **MCP GraphQL:** `src/mcp/graphql/` exposes queries; mutations need to be added
- **vCard format:** Contacts stored as vCard 3.0/4.0 on Fastmail's CardDAV server
- **Issue:** radiosilence/fastmail-cli#17

## Constraints

- **Tech stack**: Rust, must follow existing patterns (clap derive, async-graphql, reqwest)
- **Protocol**: CardDAV (WebDAV + vCard) — PUT for create/update, DELETE for delete
- **Compatibility**: Must work with Fastmail's CardDAV server specifically
- **Auth**: Reuse existing authentication mechanism (app-specific password in config)

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Partial updates for contact update | Better UX — user only specifies changed fields | — Pending |
| Flag-based delete confirmation (`--confirm`/`--yes`) | No interactive prompts, works in scripts and AI workflows | — Pending |
| Support name, email, org, phone, address, notes | Covers common contact fields without over-engineering | — Pending |

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
*Last updated: 2026-03-27 after Phase 2 completion*
