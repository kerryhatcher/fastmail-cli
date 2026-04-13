# Project Research Summary

**Project:** fastmail-cli v1.3 — Contact Groups
**Domain:** CardDAV contact group CRUD via vCard 3.0 X-ADDRESSBOOKSERVER extensions
**Researched:** 2026-04-13
**Confidence:** HIGH

## Executive Summary

This milestone adds contact group management (create, list, get, rename, delete, add/remove members) to an existing Rust CLI and MCP server that already handles contact CRUD via CardDAV. Fastmail uses the Apple/iCloud-originated vCard 3.0 group format: groups are standalone `.vcf` resources stored in the same address book as individual contacts, identified by the `X-ADDRESSBOOKSERVER-KIND:group` property and referencing members via repeating `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` lines. This is not the vCard 4.0 `KIND:group` / `MEMBER:` format — using vCard 4.0 properties against Fastmail's vCard 3.0 server produces stored-but-ignored data that neither Fastmail's web UI nor Apple clients recognize as groups.

The implementation is almost entirely an extension of existing patterns. No new Cargo dependencies are required. The project already has reqwest for HTTP, roxmltree for XML/vCard parsing, uuid for UID generation, async-graphql for MCP mutations, and clap for CLI command definitions. Every group operation (PUT, DELETE, REPORT) maps directly to patterns already used by `create_contact`, `update_contact`, and `delete_contact`. The primary engineering work is: adding a `ContactGroup` struct alongside the existing `Contact` struct, writing `serialize_group_vcard()` and `parse_group_from_vcard()` siblings, adding a KIND-based filter to `parse_vcard()` to prevent groups from leaking into contact listings, and wiring up the new `CardDavClient` group methods through commands and GraphQL resolvers.

The most consequential risk is front-loaded into the data model: if the multi-valued `X-ADDRESSBOOKSERVER-MEMBER` parser collapses repeated lines to a single value (a known production bug documented in Nextcloud), every group with more than one member will be silently corrupted on write. The second-order risks are the `urn:uuid:` prefix strip-on-parse / re-add-on-serialize discipline, and the ETag-guarded read-modify-write pattern required for membership changes (CardDAV has no PATCH). Both are well-understood and easily prevented with unit tests. The build order in ARCHITECTURE.md — data model first, client methods second, CLI and MCP last — is the correct sequencing to catch these issues before they reach integration.

## Key Findings

### Recommended Stack

The full production stack is already validated and compiling with 181 passing tests. There are zero new Cargo.toml entries required for v1.3. The only additions are code: new structs, new methods on existing structs, new CLI enum variants, and new GraphQL resolvers — all following already-established patterns in the codebase.

**Core technologies (existing, reused):**
- `roxmltree 0.21.1`: XML parsing — extend `parse_vcard()` to detect and parse group-specific properties
- `reqwest 0.13.1`: HTTP client — group PUT/DELETE/REPORT use the exact same HTTP patterns as contact writes
- `uuid 1` (v4 feature): UID generation — `Uuid::new_v4()` already used for contacts; identical call for groups
- `async-graphql 7`: GraphQL mutations — add `GqlContactGroup` type and group resolvers following `GqlContact` patterns
- `clap 4.5`: CLI commands — add `GroupsCommands` enum under `contacts groups` following existing command patterns

### Expected Features

**Must have (table stakes — all P1):**
- List groups — users must discover existing groups before operating on them
- Create group (empty) — core CRUD prerequisite for all membership operations
- Get group with member list — users need to inspect membership
- Rename group — standard update; users fix typos
- Delete group (with `--confirm` when members > 0) — standard CRUD completion
- Add member to group — core membership management
- Remove member from group — core membership management
- `--group` flag on `contacts create` — natural one-step workflow for creating and assigning in one command
- MCP GraphQL mutations for all group operations — enables AI-agent group management

**Should have (differentiators — P2):**
- Resolve member UIDs to Contact structs in `group get` output — AI agents need structured data, not raw UUIDs
- Group membership UID validation (warn on unknown UID before adding)

**Defer (v2+):**
- Bulk add/remove members in one command — defer until workflow need is validated
- `contacts list --group <id>` filter — add when scripting use cases emerge
- Group-targeted email send (`fastmail-cli send --to-group <id>`) — cross-feature, needs mail integration

### Architecture Approach

Group operations follow a strict four-layer mirror of the existing contact CRUD pattern: `CardDavClient` methods handle HTTP and vCard serialization/parsing; `src/commands/groups.rs` (new file) wires client methods to input/output; `src/main.rs` defines the clap `GroupsCommands` enum and dispatch arms; `src/mcp/graphql/mutation.rs` and `query.rs` expose the operations as GraphQL resolvers. The only structural novelty is the KIND-based type separation in the parser — `parse_vcard()` returns `None` early on `X-ADDRESSBOOKSERVER-KIND:group` (keeping contact listings clean), and a parallel `parse_group_from_vcard()` returns `None` when KIND:group is absent (keeping group listings clean).

**Major components:**
1. `src/carddav/mod.rs` — `ContactGroup` struct + group vCard serializer/parser + 7 new `CardDavClient` methods (`create_group`, `list_groups`, `get_group_by_id`, `update_group`, `delete_group`, `add_member`, `remove_member`)
2. `src/commands/groups.rs` (new file) — CLI business logic for all group operations; `src/commands/contacts.rs` modified for `--group` flag
3. `src/mcp/graphql/` — `GqlContactGroup` type, `listContactGroups`/`contactGroup` queries, 5 group mutation resolvers
4. `src/error.rs` — `GroupNotFound` and `GroupConflict` variants (mirrors `ContactNotFound`/`ContactConflict`)
5. `src/main.rs` — `GroupsCommands` enum, `--group` on `ContactsCommands::Create`, dispatch arms

### Critical Pitfalls

1. **Wrong group format (KIND vs. X-ADDRESSBOOKSERVER-KIND)** — Fastmail's server is vCard 3.0; writing `KIND:group` from RFC 6350 (vCard 4.0) produces data Fastmail and Apple clients silently ignore. Always use `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>`.

2. **Multi-member MEMBER lines collapsed to one** — The existing parser uses scalar assignment for single-valued properties. `X-ADDRESSBOOKSERVER-MEMBER` is multi-valued (one line per member). Must use `Vec<String>` accumulation in the parse loop, identical to how `emails` and `phones` are accumulated. A round-trip unit test with 3 members is mandatory before any membership code ships.

3. **`urn:uuid:` prefix strip/re-add discipline** — Wire format uses `urn:uuid:<uid>`; contact `UID` property is bare UUID. Must strip on parse, store bare UIDs in `members: Vec<String>`, re-add prefix on serialize. Skipping this causes doubled-prefix round-trip bugs invisible to unit tests but visible against live Fastmail.

4. **Membership add/remove requires ETag-guarded read-modify-write with retry** — CardDAV has no PATCH. Every member add/remove is: GET group (fresh ETag) → mutate member list → PUT with `If-Match`. On 412, retry up to 3 times with backoff. Without retry, AI agents adding multiple members sequentially will fail on the second call.

5. **Group vCards leak into `contacts list`** — `parse_vcard()` must return `None` on `X-ADDRESSBOOKSERVER-KIND:group` to prevent group vCards (which have `FN:` and so pass the existing name guard) from appearing as malformed contacts with no email/phone.

## Implications for Roadmap

Based on research, the feature set naturally splits into two sequential phases. All data-model, serialization, and parser work must precede any command implementation. The CLI and MCP layers can proceed in parallel once the CardDAV client methods are stable.

### Phase 1: Group Data Model, CRUD Foundation, and Parser Separation

**Rationale:** Five of seven critical pitfalls are data-model-level issues that will corrupt all subsequent work if not addressed first. The type separation (`parse_vcard` KIND filter, `parse_group_from_vcard`, `ContactGroup` struct, `urn:uuid:` boundary convention) is the prerequisite for every other piece of this milestone. CRUD operations (create, list, get, rename, delete) can then be built on this foundation with confidence.

**Delivers:**
- `ContactGroup` struct with `id`, `name`, `member_ids: Vec<String>`, `href`, `etag`
- `serialize_group_vcard()` with correct `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` format
- `parse_group_from_vcard()` with Vec accumulation for MEMBER lines and `urn:uuid:` stripping
- `parse_vcard()` modified to return `None` on KIND:group (backward-compatible)
- `GroupNotFound` and `GroupConflict` error variants in `src/error.rs`
- `CardDavClient` methods: `create_group`, `list_groups`, `get_group_by_id`, `update_group`, `delete_group`
- CLI: `contacts groups list`, `create <name>`, `get <id>`, `rename <id> <new-name>`, `delete <id> [--confirm]`
- MCP: `listContactGroups`, `contactGroup`, `createContactGroup`, `updateContactGroup`, `deleteContactGroup`
- Unit tests: serialize/parse round-trip with 3 members; `urn:uuid:` prefix invariant; `contacts list` excludes groups; `contacts group list` excludes contacts

**Addresses from FEATURES.md:** list groups, create group, get group, rename group, delete group (all P1 table stakes)

**Avoids from PITFALLS.md:** Wrong group format (P1), parse_vcard silent discard (P2), multi-member collapse (P3), urn:uuid prefix confusion (P4), group delete without member count (P6), groups leaking into contacts list (P7)

### Phase 2: Group Membership Management and --group on contacts create

**Rationale:** Membership operations (`add_member`, `remove_member`) require the ETag-guarded read-modify-write pattern with retry logic, which is more complex than the straightforward CRUD of Phase 1. Separating this work allows Phase 1 to be fully tested (including live Fastmail round-trip verification) before the retry loop and sequencing logic of `--group on contacts create` are added. The `--group` flag on `contacts create` also requires sequencing: contact created first (to get UID), then group updated — partial failure must be handled explicitly.

**Delivers:**
- `CardDavClient::add_member` and `remove_member` with 3-attempt ETag-guarded retry and backoff
- CLI: `contacts groups add-member <group-id> <contact-id>`, `remove-member <group-id> <contact-id>`
- CLI: `contacts create --group <group-id>` with validation before contact creation
- MCP: `addContactGroupMember`, `removeContactGroupMember` mutations
- Operations return full updated `ContactGroup` (not just success boolean)
- WireMock integration test: concurrent `add-member` calls produce correct final member list

**Addresses from FEATURES.md:** add member, remove member, `--group` on contacts create (remaining P1 table stakes); resolved member contacts in `group get` (P2 differentiator)

**Avoids from PITFALLS.md:** Membership read-modify-write race condition (P5); UX pitfalls (member count in delete confirmation, `--group` validation before contact creation, updated member list returned from add/remove)

### Phase Ordering Rationale

- Protocol format correctness (Pitfalls 1-4) is entirely a data-model and serialization concern — it must be validated with unit tests and a live Fastmail write before any membership code is written, because membership operations depend on the serializer/parser being correct
- The KIND filter in `parse_vcard()` is a behavioral regression risk to existing `contacts list` output — it must land in Phase 1 and be verified before Phase 2 adds more REPORT traffic
- The retry loop for membership writes (Pitfall 5) is isolated to `CardDavClient` internals — it does not affect Phase 1 CRUD operations and is cleanly separable
- CLI and MCP layers for each phase can be developed in parallel once the CardDavClient methods for that phase are stable (per the build order in ARCHITECTURE.md: data model, errors, client methods, CLI handlers, CLI args, MCP types, MCP queries, MCP mutations)

### Research Flags

Phases with standard patterns (skip `research-phase`):
- **Phase 1:** Data model, serialization, and CRUD are fully documented in ARCHITECTURE.md with explicit code sketches; protocol format confirmed by multiple independent sources. No unknowns remain.
- **Phase 2:** ETag-guarded retry pattern is standard CardDAV practice and mirrors the existing `update_contact` pattern with an added retry loop. MCP mutation patterns are established in the existing `mutation.rs`.

Neither phase needs a `research-phase` run during planning — the research files provide sufficient implementation specificity.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | No new deps; all capabilities verified in existing codebase with 181 passing tests |
| Features | HIGH (protocol), MEDIUM (edge cases) | DAVx5 Fastmail page is authoritative for group method; Apple-originated empty-FN edge case is MEDIUM |
| Architecture | HIGH | Derived from direct codebase inspection + confirmed protocol sources; code sketches provided for all key patterns |
| Pitfalls | HIGH (format/parsing), MEDIUM (Fastmail server behavior) | Multi-member collapse documented in production bug report; Fastmail-specific quirks inferred from community sources (no official Fastmail developer API docs for groups) |

**Overall confidence:** HIGH

### Gaps to Address

- **Fastmail server-side property filter support in REPORT:** Fastmail's Cyrus IMAP server may not support `<card:prop-filter>` for KIND filtering in REPORT queries. Architecture correctly calls for client-side KIND filtering — verify this early in Phase 1 integration testing, not a blocker.
- **Empty FN on Apple-originated group vCards:** Some Apple clients create groups with empty `FN:`. The `parse_vcard()` name guard returns `None` on empty FN. For groups, fall back to UID as display name rather than returning `None`. Handle in `parse_group_from_vcard()` during Phase 1.
- **Partial failure shape for `--group` flag:** When `contacts create --group <id>` creates the contact but fails to update the group, the JSON output shape for partial success is not yet defined. Decide during Phase 2 command handler implementation using the existing `Output` struct.

## Sources

### Primary (HIGH confidence)
- [DAVx5: Tested With Fastmail](https://www.davx5.com/tested-with/fastmail) — "Contact group method: Groups are separate vCards"; confirms Apple/X-ADDRESSBOOKSERVER format
- [rcmcarddav GROUPS.md](https://github.com/mstilkerich/rcmcarddav/blob/master/doc/GROUPS.md) — Authoritative vCard-type vs CATEGORIES-type group breakdown; X-ADDRESSBOOKSERVER format documentation
- [RFC 6350 — vCard Format Specification](https://www.rfc-editor.org/rfc/rfc6350.html) — KIND and MEMBER properties (vCard 4.0); confirms X-ADDRESSBOOKSERVER-* are vCard 3.0 equivalents
- [RFC 6352 — CardDAV](https://datatracker.ietf.org/doc/html/rfc6352) — CardDAV REPORT and PUT semantics; If-Match ETag discipline

### Secondary (MEDIUM confidence)
- [Nextcloud issue #9369](https://github.com/nextcloud/server/issues/9369) — Real-world example of multi-member MEMBER line collapse bug; confirms `urn:uuid:` URI format
- [DAVx5 FAQ: Can't manage groups on device](https://www.davx5.com/faq/cant-manage-groups-on-device) — Group format compatibility matrix
- [Nicolas Grilly: Migrating Google Contacts to Fastmail](https://www.grilly.com/posts/migrate-google-contacts-labels-to-fastmail-groups/) — Confirms Fastmail uses vCard-type groups, not CATEGORIES
- [Fastmail Contact Groups help](https://www.fastmail.help/hc/en-us/articles/360058753114-Contact-groups) — Delete does not delete members; rename behavior
- [Fastmail Troubleshooting CardDAV fields](https://www.fastmail.help/hc/en-us/articles/360058753094-Troubleshooting-CardDAV-fields) — vCard 3.0 format, X- extension field storage behavior

### Tertiary (LOW confidence / corroborating only)
- rcmcarddav issue #331 — vCard 4 KIND/MEMBER not recognized by vCard 3-only parsers (confirms interop risk of using vCard 4 format)
- Fastmail JMAP discuss thread — Community confirmation of Apple-compatible group vCard format

---
*Research completed: 2026-04-13*
*Ready for roadmap: yes*
