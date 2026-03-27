# Project Research Summary

**Project:** fastmail-cli — CardDAV Contact CRUD milestone
**Domain:** CLI write operations on a CardDAV (RFC 6352) server via an existing Rust async CLI
**Researched:** 2026-03-27
**Confidence:** HIGH

## Executive Summary

This milestone extends fastmail-cli's existing read-only CardDAV integration with full create, update, and delete operations for contacts. The existing codebase already performs PROPFIND address book discovery and REPORT-based contact listing, handles vCard 3.0 parsing (including line unfolding), and exposes contacts through both a CLI and a GraphQL/MCP layer. The write operations slot cleanly into this layered architecture — one new dependency (`uuid` v1.22.0 with the `v4` feature, already present in Cargo.lock), a vCard serializer that mirrors the existing parser, three new `CardDavClient` methods, thin CLI subcommands, and three GraphQL mutations. No structural changes to the application are required.

The recommended build order is strictly dictated by dependencies: extend the `Contact` struct first (adding `href` and `etag` fields), then build the vCard serializer in isolation with unit tests, then implement the CardDAV HTTP write methods, and finally wire up the CLI and MCP layers in parallel. This order ensures each layer is tested before the next depends on it and maps directly to the four-stage architecture outlined in research.

The primary risks are all concurrency-related and protocol-correctness-related: omitting `If-Match` headers on updates (silent data loss), building contact URLs from the UID property instead of the server-supplied `href` (404s and phantom duplicates), and generating vCard without proper line folding or character escaping (contacts that Fastmail accepts but mobile clients mangle). All three risks are well-understood and preventable with targeted unit and integration tests. Fastmail-specific behaviors (VERSION:3.0 enforcement, app password scoping) require live integration testing to fully validate.

---

## Key Findings

### Recommended Stack

No new crates are needed beyond promoting `uuid` (already in Cargo.lock at 1.22.0) to a direct dependency with the `v4` feature. The existing stack — `reqwest` 0.13.1, `roxmltree`, `serde`, `async_graphql` — covers all HTTP mechanics, XML parsing, and GraphQL exposure. The Rust vCard ecosystem has no suitable vCard 3.0 serialization crate; the correct approach is a hand-rolled `build_vcard()` function that mirrors the existing `parse_vcard()` in style and location.

**Core technologies:**
- `reqwest` 0.13.1: HTTP client for PUT/DELETE — already demonstrates custom `Method::from_bytes()` pattern in codebase, works unchanged for write operations
- `uuid` 1.22.0 (promote to direct dep): Generate RFC 4122 v4 UUIDs for new contact UID and resource filename — already present as a transitive dep, zero tree cost
- `roxmltree`: Parse PROPFIND responses to extract ETag for update/delete — already used for REPORT parsing
- `serde` + `async_graphql`: Expose mutations to MCP consumers — existing infrastructure, no changes needed

**What NOT to add:**
- `vcard4` crate: Targets vCard 4.0 only; Fastmail stores 3.0; adds a dep to avoid 30 lines of string formatting
- Any WebDAV client crate: These are server-side frameworks, not HTTP clients

See `.planning/research/STACK.md` for full detail.

### Expected Features

The milestone has a clear and well-bounded feature surface. All table-stakes features are direct protocol operations with known complexity. The notable complication is that CardDAV has no PATCH operation — partial updates require a full read-modify-write cycle with ETag-based optimistic locking.

**Must have (table stakes):**
- Create contact (CLI + MCP `createContact` mutation) — PUT new vCard to `{addressbook_href}/{uuid}.vcf`
- Update contact with partial field semantics (CLI + MCP `updateContact`) — fetch existing vCard, merge changed fields, PUT full vCard back with `If-Match`
- Delete contact (CLI + MCP `deleteContact`) — HTTP DELETE with `--confirm`/`-y` guard and MCP PREVIEW/CONFIRM token
- vCard 3.0 generation with RFC 6350 line-folding and character escaping
- `href` and `etag` fields on `Contact` struct — prerequisite for all write operations
- Structured 4xx/5xx error output distinguishing 404 (not found), 412 (conflict), 403 (insufficient permissions)

**Should have (differentiators):**
- ETag-based conditional writes on all PUT/DELETE operations — prevents silent data loss under concurrent edits
- Return created/updated contact in response — confirmation of what was written, follows existing `Output::success()` pattern
- Address book selection via `--addressbook` flag — users with multiple books need this; default to first discovered
- TYPE labels on emails/phones (work/home/cell) — already in struct, just needs CLI flag exposure

**Defer to follow-on:**
- Multi-valued field label parsing (complex CLI flag syntax)
- CSV or vCard file import/export
- Contact groups/categories
- Bulk operations (scriptable with shell loops over single operations)
- Interactive TUI / prompt-driven contact editor (anti-feature — breaks scripting)

See `.planning/research/FEATURES.md` for the full feature dependency graph.

### Architecture Approach

The write operations follow the same three-layer pattern already established by email operations: a protocol client method in `src/carddav/mod.rs`, a thin handler in `src/commands/contacts.rs`, and a GraphQL mutation in `src/mcp/graphql/mutation.rs`. `CardDavClient` is stateless (holds only a `reqwest::Client` and credentials), so it is stored directly in the GraphQL context without a Mutex — unlike `JmapClient`, which caches session state. The five build stages have a linear dependency chain (struct → serializer → HTTP methods) that fans out to CLI and MCP in parallel at the end.

**Major components:**
1. `Contact` struct extension (`href: String`, `etag: Option<String>`) — enables all write operations; populated during existing REPORT parsing
2. `build_vcard(uid, input)` + `fold_vcard_line()` + `escape_vcard_value()` in `src/carddav/mod.rs` — pure functions, fully unit-testable in isolation
3. `CardDavClient::create_contact`, `update_contact`, `delete_contact`, `get_contact_etag` — protocol implementation; integration-testable against live Fastmail CardDAV
4. CLI subcommands `contacts create/update/delete` in `src/commands/contacts.rs` — thin argument parsers, call client, print `Output::success()`
5. GraphQL mutations `createContact`, `updateContact`, `deleteContact` in `src/mcp/graphql/mutation.rs` — with `GqlContactMutationResult` return type and PREVIEW/CONFIRM token for delete

See `.planning/research/ARCHITECTURE.md` for data flow diagrams and component boundary detail.

### Critical Pitfalls

1. **ETag blindness on update** — PUT without `If-Match` silently overwrites concurrent edits. Fix: parse `d:getetag` from REPORT responses into `Contact.etag`; always send `If-Match: "{etag}"` on update PUT; treat 412 as a user-actionable conflict error, not a generic failure.

2. **Using UID as resource URL** — vCard UID property and the DAV resource `href` are distinct. Contacts created by other clients may have URLs that do not match their UID. Fix: store `<d:href>` from REPORT responses in `Contact.href`; use that for all PUT/DELETE addresses; only use UUID-based filenames for contacts this CLI creates.

3. **vCard line folding and escaping omitted** — Fastmail's server is lenient but mobile clients syncing the same address book will mangle contacts with lines over 75 octets or unescaped `;`, `,`, `\`. Fix: implement `fold_vcard_line()` and `escape_vcard_value()` before wiring any HTTP calls; unit-test with long notes, names containing commas, and multi-email contacts.

4. **Missing `N` property in generated vCard** — vCard 3.0 (RFC 2426) requires the structured name `N` property; emitting only `FN` produces technically invalid vCards that render incorrectly on iOS/Outlook. Fix: always emit `N:;;;{name};` as a minimum; accept `--first-name`/`--last-name` flags for proper structured name.

5. **Weak confirmation token on delete mutation** — The MCP PREVIEW/CONFIRM pattern requires the token to be derived from fields that uniquely identify the target. A token that does not include the contact `href` or UID allows accidental deletion of the wrong contact. Fix: include `contact.href` (or `contact.id`) in the `confirmation_token()` input slice, following the exact pattern used in `send_email`.

See `.planning/research/PITFALLS.md` for the full list of 13 pitfalls with phase-specific warnings.

---

## Implications for Roadmap

The research establishes a clear, dependency-driven build order. There are five natural stages, and the last two (CLI and MCP) are independent of each other and can proceed in parallel once the CardDAV HTTP methods are done.

### Phase 1: Contact Struct Foundation

**Rationale:** Every write operation depends on knowing the contact's server-assigned `href` and its current `etag`. These fields are already in the REPORT XML but are discarded by the parser. This is a zero-risk, zero-new-dependency change that unblocks all subsequent work.

**Delivers:** Updated `Contact` struct with `href: String` and `etag: Option<String>`; updated `parse_contacts_response()` that populates both; new `ContactInput` struct; `ContactConflict(String)` and `ContactNotFound(String)` error variants in `src/error.rs`.

**Addresses:** Table stakes: href/ETag tracking (unblocks all write features)

**Avoids:** Pitfall 1 (ETag blindness), Pitfall 2 (UID vs href confusion)

### Phase 2: vCard Serializer

**Rationale:** Both create and update depend on generating valid vCard text. Building and testing this as a pure function in isolation — before any HTTP calls are involved — means the serializer can be verified with unit tests alone. Bugs here corrupt contacts silently; catching them early is critical.

**Delivers:** `build_vcard(uid, input) -> String`; `fold_vcard_line(line) -> String`; `escape_vcard_value(s) -> String`; `uuid` added as direct Cargo.toml dependency; comprehensive unit tests for edge cases (long notes, special characters, multi-email, structured name derivation).

**Uses:** `uuid` 1.22.0 with `v4` feature (only new dependency)

**Avoids:** Pitfall 3 (no folding/escaping), Pitfall 4 (wrong Content-Type — caught when tested against real server), Pitfall 6 (VERSION:3.0 fixed in generator), Pitfall 8 (missing N property), Pitfall 11 (hash-based UID), Pitfall 12 (unescaped special chars)

### Phase 3: CardDAV HTTP Write Methods

**Rationale:** Once the serializer is proven, the HTTP layer can be implemented and integration-tested. This phase is the most protocol-sensitive and benefits from real Fastmail CardDAV testing. It is intentionally kept separate from CLI/MCP so the protocol layer can be tested via direct method calls.

**Delivers:** `CardDavClient::get_contact_etag()`, `create_contact()`, `update_contact()`, `delete_contact()`; correct `If-None-Match: *` on create; `If-Match: "{etag}"` on update/delete; distinct 404/412/403 error handling.

**Avoids:** Pitfall 1 (If-Match on update/delete), Pitfall 2 (href-based URLs), Pitfall 4 (Content-Type: text/vcard), Pitfall 5 (read-modify-write race), Pitfall 7 (address book discovery before write), Pitfall 10 (201 vs 204 distinction), Pitfall 13 (403 with helpful message)

### Phase 4a: CLI Subcommands

**Rationale:** Thin wrappers over Phase 3 methods. Can be built and tested in parallel with Phase 4b once Phase 3 is stable. CLI is the primary human-facing surface and easier to manually verify.

**Delivers:** `contacts create`, `contacts update`, `contacts delete` subcommands in `src/main.rs` and `src/commands/contacts.rs`; `--confirm`/`-y` guard on delete; `--id` flag for identifying target contact; structured error messages for 412/404/403.

**Implements:** CLI handler pattern from ARCHITECTURE.md

### Phase 4b: GraphQL Mutations (MCP)

**Rationale:** Independent of Phase 4a; shares the same Phase 3 protocol methods. The MCP surface adds the PREVIEW/CONFIRM token requirement for destructive operations, which must follow the existing `send_email` pattern exactly.

**Delivers:** `createContact`, `updateContact`, `deleteContact` mutations; `GqlContactMutationResult` type; `CardDavClient` added to MCP context (no Mutex); PREVIEW/CONFIRM token for delete that includes contact `href`/UID.

**Avoids:** Pitfall 9 (weak confirmation token), Anti-Pattern 3 (client-per-resolver)

### Phase Ordering Rationale

- Phase 1 before everything: `href` and `etag` on `Contact` are literal prerequisites — there is no way to implement safe writes without them.
- Phase 2 before Phase 3: The serializer is a pure function that can be tested without network access; isolating it reduces debugging surface when HTTP issues arise.
- Phase 3 before Phases 4a/4b: Both CLI and MCP are thin wrappers; putting protocol bugs in the HTTP layer means they are only exposed through two separate surfaces simultaneously.
- Phases 4a and 4b in parallel: No shared code; both call Phase 3 methods; team velocity benefit.

### Research Flags

Phases with well-documented patterns (no additional phase research needed):

- **Phase 1:** Pure struct/parser changes — mechanical, no ambiguity
- **Phase 2:** vCard 3.0 format is fully specified in RFC 6350/RFC 2426; folding and escaping rules are unambiguous
- **Phase 4a:** Follows established CLI handler pattern from existing codebase
- **Phase 4b:** Follows established GraphQL mutation pattern from `send_email`; PREVIEW/CONFIRM pattern is already implemented

Phases that benefit from targeted validation before implementation:

- **Phase 3:** Fastmail-specific CardDAV behaviors (how it handles VERSION mismatches, ETag format, address book URL conventions for non-default books) should be validated with a live integration test against a real Fastmail account before finalizing the implementation. The protocol research is HIGH confidence; Fastmail-specific edge cases are MEDIUM.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | One new dep (uuid 1.22.0) already in Cargo.lock; all others are existing codebase deps confirmed by direct inspection |
| Features | HIGH | Protocol specs (RFC 6352/6350) are finalized standards; feature scope confirmed against PROJECT.md and INTEGRATIONS.md |
| Architecture | HIGH | Build order dictated by hard code dependencies; component boundaries confirmed by codebase inspection; patterns confirmed from existing email mutation implementation |
| Pitfalls | HIGH (core protocol), MEDIUM (Fastmail-specific) | Protocol pitfalls are RFC-grounded; Fastmail-specific behaviors (VERSION enforcement, app password scoping, address book URL conventions) need live integration validation |

**Overall confidence:** HIGH

### Gaps to Address

- **Fastmail ETag format in practice:** ETags are described as opaque quoted strings (e.g., `"abc123-def456"`). The format should be validated against a live REPORT response to confirm they are stored and sent correctly before the update/delete implementation is considered complete.

- **Fastmail behavior on VERSION:4.0 vCard PUT:** Research confirms Fastmail stores VERSION:3.0, and the generator will emit 3.0. However, the exact server response for a VERSION mismatch has not been tested live. Treat as a MEDIUM risk; validate during Phase 3 integration testing.

- **Fastmail app password "Contacts" scope label:** The exact label in the Fastmail UI for the scope needed for CardDAV write access should be verified and documented in CLI help text. Error message guidance in Pitfall 13 assumes the scope is labeled "Contacts (Read/Write)".

- **vCard crate ecosystem currency:** The recommendation to avoid vCard crates was based on training data through August 2025. Verify no new widely-adopted vCard 3.0 serialization crate has emerged before finalizing the decision to build inline. The inline approach remains correct even if one has emerged, given the project's dep-minimization philosophy.

---

## Sources

### Primary (HIGH confidence)
- RFC 6352 (CardDAV): https://datatracker.ietf.org/doc/html/rfc6352 — PUT/DELETE semantics, ETag/If-Match/If-None-Match requirements, resource URL conventions
- RFC 6350 (vCard 4.0): https://datatracker.ietf.org/doc/html/rfc6350 — line folding (§3.2), CRLF endings (§3.3), character escaping (§3.4), property definitions
- RFC 2426 (vCard 3.0): https://datatracker.ietf.org/doc/html/rfc2426 — N property REQUIRED, VERSION:3.0 format
- RFC 4918 (WebDAV): https://datatracker.ietf.org/doc/html/rfc4918 — conditional requests, If-Match, If-None-Match, 412 Precondition Failed
- RFC 7232 (HTTP conditional requests): https://datatracker.ietf.org/doc/html/rfc7232 — ETag semantics
- `src/carddav/mod.rs` — direct codebase inspection: REPORT request (line 163), ETag discarded in parser, HTTP method pattern (line 82), existing vCard 3.0 test (line 423)
- `src/mcp/graphql/mutation.rs` — direct codebase inspection: existing mutation patterns, PREVIEW/CONFIRM token implementation
- `Cargo.lock` lines 4380-4387 — uuid 1.22.0 confirmed present as transitive dep

### Secondary (MEDIUM confidence)
- Fastmail CardDAV behavior (VERSION:3.0, address book URL `/dav/addressbooks/user/{username}/Default/`, ETag format): observed from existing PROPFIND requests in codebase and known Fastmail deployment characteristics
- crates.io vCard ecosystem assessment: knowledge as of August 2025 training data; no live verification

---

*Research completed: 2026-03-27*
*Ready for roadmap: yes*
