# Phase 18: Group Data Model, CRUD, and Base Surfaces - Context

**Gathered:** 2026-04-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can create, list, inspect, rename, and delete contact groups — and existing `contacts list` is unaffected by group vCards. Covers the full group CRUD lifecycle across CLI (`contacts groups` subcommands) and MCP/GraphQL surfaces, plus the underlying CardDAV transport using Fastmail's X-ADDRESSBOOKSERVER vCard extensions.

</domain>

<decisions>
## Implementation Decisions

### Group Data Model & vCard Representation
- `ContactGroup` struct lives in `src/carddav/mod.rs` alongside `Contact` — keeps all vCard types together, matches existing pattern
- Members represented as `Vec<String>` storing contact UIDs (extracted from X-ADDRESSBOOKSERVER-MEMBER URN values) — resolve to full contacts only at display/query time
- `parse_vcard()` checks for `X-ADDRESSBOOKSERVER-KIND:group` line and returns `None` to filter group vCards from contact listings; separate `parse_group_vcard()` function handles group parsing
- Group ID is the vCard UID (same as contact IDs) — consistent with existing contact ID pattern

### CLI Command Structure
- Groups nested under contacts: `contacts groups list/create/get/rename/delete`
- Group identifier accepts both group ID (UID) and group name for convenience — name lookup errors on ambiguity (multiple groups with same name)
- `groups get` outputs JSON object with group metadata + resolved member contacts array (matches `contacts list` JSON output pattern via Output struct)
- `groups delete` requires `--confirm` flag (matches existing `contacts delete` pattern), rejected without it

### MCP/GraphQL Surface & CardDAV Transport
- Separate `ContactGroup` GraphQL type with fields: `id`, `name`, `memberCount`, `members: [Contact!]!` (resolved) — distinct from Contact type
- Group creation: PUT a vCard with `X-ADDRESSBOOKSERVER-KIND:group` and `FN:<name>` to a new UUID-based href on the default address book
- Group rename: fetch current vCard → update `FN:` line → PUT back with `If-Match: <etag>` (ETag-guarded, matches contact update pattern)
- MCP group types live in new `src/mcp/graphql/types/group.rs` with resolvers added to existing mutation.rs/query.rs files

### Claude's Discretion
No items deferred to Claude's discretion — all decisions captured above.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CardDavClient` in `src/carddav/mod.rs` — HTTP client with basic auth, PROPFIND, REPORT, PUT, DELETE methods already implemented
- `parse_vcard()` function (line 593) — line-by-line vCard parser, can be extended with KIND detection
- `serialize_vcard()` — vCard 3.0 serializer with line folding, escaping, UUID generation
- `Contact` struct with href/etag fields — pattern to replicate for `ContactGroup`
- `ContactCreateResult` struct — pattern for group create/rename return values
- `contact_client()` helper in `src/commands/contacts.rs` — config → client factory
- `Output::success()`/`Output::error()` — JSON output wrapper used by all commands
- `ContactsCommands` enum in `src/main.rs` (line 339) — clap derive pattern to extend

### Established Patterns
- CardDAV operations: PROPFIND for discovery, addressbook-query REPORT for listing, PUT for create/update, DELETE for delete
- ETag-guarded writes: `If-Match` header on PUT/DELETE, `If-None-Match: *` on create
- Error types: `ContactNotFound`, `ContactConflict` in `src/error.rs` — pattern to replicate for groups
- MCP GraphQL: types in `src/mcp/graphql/types/`, queries in `query.rs`, mutations in `mutation.rs`
- AppContext holds `Arc<OnceCell<Arc<CardDavClient>>>` for lazy CardDAV initialization

### Integration Points
- `src/main.rs` ContactsCommands enum — add `Groups(GroupsCommands)` variant
- `src/carddav/mod.rs` — add `ContactGroup` struct, `parse_group_vcard()`, group CRUD methods on `CardDavClient`
- `src/commands/contacts.rs` — add group command handler functions
- `src/mcp/graphql/types/` — add `group.rs` module
- `src/mcp/graphql/query.rs` and `mutation.rs` — add group resolvers
- `src/error.rs` — add `GroupNotFound` variant

</code_context>

<specifics>
## Specific Ideas

- Fastmail uses `X-ADDRESSBOOKSERVER-KIND:group` (vCard 3.0 extension), NOT `KIND:group` (vCard 4.0) — using wrong format produces silently ignored data
- `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<contact-uid>` lines list members
- Client-side KIND filtering is the correct approach (Fastmail may not support server-side card:prop-filter KIND)
- Group vCard example structure:
  ```
  BEGIN:VCARD
  VERSION:3.0
  UID:<uuid>
  FN:My Group
  X-ADDRESSBOOKSERVER-KIND:group
  X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<member-uid-1>
  X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<member-uid-2>
  END:VCARD
  ```

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
