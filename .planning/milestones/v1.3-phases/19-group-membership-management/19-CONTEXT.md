# Phase 19: Group Membership Management - Context

**Gathered:** 2026-04-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can add and remove contacts from groups, and assign a new contact to a group in a single `contacts create --group` invocation. Covers CLI commands (`contacts groups add-member/remove-member`, `contacts create --group`), MCP mutations (`addGroupMember`, `removeGroupMember`), and the underlying CardDAV transport with ETag-guarded concurrent member updates.

</domain>

<decisions>
## Implementation Decisions

### Membership CardDAV Transport
- Add member: fetch group vCard → append `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` line → PUT with `If-Match: <etag>`
- Remove member: fetch group vCard → remove matching MEMBER line → PUT with `If-Match: <etag>` (same ETag-guarded pattern)
- ETag race handling: retry-on-412 loop — fetch fresh vCard+ETag, re-apply change, re-PUT (max 3 retries) — prevents silently dropped members on concurrent access
- Member validation: validate contact UID exists via `get_contact_by_id()` before adding — fail early with clear error

### CLI Commands & --group Flag
- `contacts groups add-member <group-id> <contact-id>` and `contacts groups remove-member <group-id> <contact-id>` under existing groups subcommand
- `contacts create --group <group-id>` partial failure behavior: if contact creates OK but group-add fails, report both outcomes clearly ("Contact created (ID: X) but group add failed: Y. Run `contacts groups add-member` to retry.")
- Group identifier accepts ID or name (reuse `resolve_group` from Phase 18); contact identifier accepts ID only
- Output format: JSON via `Output::success()` showing updated group with member count and resolved member list (matches `groups get` output)

### MCP/GraphQL Mutations
- `addGroupMember(groupId: ID!, contactId: ID!): ContactGroup!` and `removeGroupMember(groupId: ID!, contactId: ID!): ContactGroup!` — return full group with resolved members
- Error handling: GraphQL field error with descriptive message ("Contact not found: <id>") — consistent with existing MCP error patterns
- No separate `createContactInGroup` mutation — agents compose `createContact` + `addGroupMember` calls (simpler, more flexible, composable)

### Claude's Discretion
No items deferred to Claude's discretion — all decisions captured above.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets (from Phase 18)
- `ContactGroup` struct with `member_uids: Vec<String>` in `src/carddav/mod.rs`
- `CardDavClient::get_group_by_id()`, `list_groups()`, `create_group()` — existing group transport
- `serialize_group_vcard()` — generates vCard 3.0 with X-ADDRESSBOOKSERVER-MEMBER lines
- `parse_group_vcard()` — parses group vCards including member UIDs
- `resolve_group_members()` — batch resolves member UIDs to Contact objects
- `resolve_group()` helper in `src/commands/contacts.rs` — ID-or-name group lookup
- `GroupsCommands` enum in `src/main.rs` — extend with AddMember/RemoveMember variants
- `GqlContactGroup` in `src/mcp/graphql/types.rs` — already has members field
- `GroupNotFound`, `GroupConflict` error variants in `src/error.rs`

### Established Patterns
- ETag-guarded writes: `If-Match` on PUT, existing in contact update and group rename
- `ContactCreateResult` returned from `create_contact()` — pattern for create-then-add-to-group
- `build_contact()` and `contact_client()` in `src/commands/contacts.rs` — reuse for `--group` flag
- HMAC confirmation token pattern for destructive mutations in `mutation.rs`

### Integration Points
- `src/main.rs` GroupsCommands enum — add `AddMember` and `RemoveMember` variants
- `src/main.rs` ContactsCommands::Create — add `--group` optional arg
- `src/carddav/mod.rs` CardDavClient — add `add_group_member()` and `remove_group_member()` methods
- `src/commands/contacts.rs` — add `add_group_member()`, `remove_group_member()` handlers + modify `create_contact()` for --group
- `src/mcp/graphql/mutation.rs` — add `add_group_member()` and `remove_group_member()` resolvers

</code_context>

<specifics>
## Specific Ideas

- The retry-on-412 pattern is critical for concurrent safety: Fastmail may return 412 Precondition Failed if another client updates the group vCard between fetch and PUT
- `serialize_group_vcard()` already handles the MEMBER line format — add/remove should manipulate the `member_uids` Vec and re-serialize
- The `--group` flag on `contacts create` is a convenience wrapper: create contact first, then call add_group_member — not a new CardDAV operation

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
