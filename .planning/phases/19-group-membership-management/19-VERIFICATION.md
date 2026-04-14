---
phase: 19-group-membership-management
verified: 2026-04-13T12:00:00Z
status: passed
score: 6/6 must-haves verified
gaps: []
human_verification:
  - test: "Run `contacts groups add-member <real-group-id> <real-contact-id>` against live Fastmail CardDAV"
    expected: "JSON output with updated group object and resolved members array; member_count increments"
    why_human: "Requires live Fastmail credentials and a real address book — cannot be tested programmatically without API access"
  - test: "Run `contacts groups remove-member <real-group-id> <real-contact-id>` against live Fastmail CardDAV"
    expected: "JSON output with updated group; member removed from members array; idempotent if contact not in group"
    why_human: "Same — requires live server to exercise ETag retry loop and real HTTP responses"
  - test: "Run `contacts create --group <group-name> --name 'Test User' --email test@example.com` then check partial-failure path by supplying an invalid group name"
    expected: "Contact is created (visible in contact list), output includes success:true + error message with retry instructions when group not found"
    why_human: "Requires live CardDAV to create the contact; partial-failure branch depends on server returning GroupNotFound"
  - test: "Invoke addGroupMember and removeGroupMember mutations via MCP client (e.g., Claude Desktop)"
    expected: "Returns ContactGroup object with id, name, member_count, and members array populated with resolved contact data"
    why_human: "MCP server requires running process and live Fastmail credentials; GraphQL schema introspection would confirm field presence but not data correctness"
---

# Phase 19: Group Membership Management Verification Report

**Phase Goal:** Users can add and remove contacts from groups, and assign a new contact to a group in a single `contacts create --group` invocation
**Verified:** 2026-04-13T12:00:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `add_group_member` appends a member UID to a group's vCard via ETag-guarded PUT with retry-on-412 | VERIFIED | `src/carddav/mod.rs` lines 624-696: full 3-retry loop with `IF_MATCH` header, 412 conflict detection, idempotency guard, `member_uids.push` |
| 2 | `remove_group_member` removes a member UID from a group's vCard via ETag-guarded PUT with retry-on-412 | VERIFIED | `src/carddav/mod.rs` lines 704-773: identical retry structure, `member_uids.retain(|uid| uid != contact_uid)` |
| 3 | User can run `contacts groups add-member <group-id> <contact-id>` and see JSON output | VERIFIED | `GroupsCommands::AddMember` in `src/main.rs` lines 478-483; dispatch at lines 995-997; handler `pub async fn add_group_member` in `src/commands/contacts.rs` lines 367-382 with `Output::success(data).print()` |
| 4 | User can run `contacts groups remove-member <group-id> <contact-id>` and see JSON output | VERIFIED | `GroupsCommands::RemoveMember` in `src/main.rs` lines 486-491; dispatch at lines 998-1000; handler at `src/commands/contacts.rs` lines 385-399 |
| 5 | User can run `contacts create --group <group-id> --name 'X' --email 'y@z'` and the contact is created and added to the group | VERIFIED | `--group: Option<String>` at `src/main.rs` line 381; `group.as_deref()` passed at line 939; `create_contact(input, group: Option<&str>)` handler at `src/commands/contacts.rs` line 169 with full group-add logic at lines 181-222 |
| 6 | AI agent can call `addGroupMember` and `removeGroupMember` mutations and receive updated ContactGroup with resolved members | VERIFIED | `src/mcp/graphql/mutation.rs` lines 412-450: both resolvers call `carddav.add/remove_group_member`, then `carddav.resolve_group_members`, then return `GqlContactGroup::with_members(updated, members)` |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/carddav/mod.rs` | `add_group_member` and `remove_group_member` on `CardDavClient` | VERIFIED | Both public async methods present at lines 624 and 704; `#[instrument(skip(self))]` on both; `ContactGroup` derives `Clone` at line 72 |
| `src/main.rs` | `AddMember` and `RemoveMember` variants in `GroupsCommands`; `--group` flag on `Create` | VERIFIED | `AddMember` at line 478, `RemoveMember` at line 486, `group: Option<String>` at line 381 |
| `src/commands/contacts.rs` | `pub async fn add_group_member` and `pub async fn remove_group_member` handlers; modified `create_contact` | VERIFIED | Handlers at lines 367 and 385; `create_contact(input, group: Option<&str>)` at line 169 with partial-failure reporting |
| `src/mcp/graphql/mutation.rs` | `addGroupMember` and `removeGroupMember` mutation resolvers | VERIFIED | Resolvers at lines 413 and 433; return `GqlContactGroup` directly (not wrapped) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `CardDavClient::add_group_member` | `get_contact_by_id` | validation call before retry loop | WIRED | `src/carddav/mod.rs` line 630: `self.get_contact_by_id(contact_uid).await?` |
| `CardDavClient::add_group_member` | `serialize_group_vcard` | vCard generation for PUT body | WIRED | `src/carddav/mod.rs` line 660: `let vcard = serialize_group_vcard(&updated)` |
| `CardDavClient::add_group_member` | `map_group_write_response` | HTTP response to Result mapping | WIRED | `src/carddav/mod.rs` line 678: `match map_group_write_response(group_id, Some(&etag), status, &headers, &body)` |
| `src/main.rs GroupsCommands::AddMember` dispatch | `commands::add_group_member` | match arm | WIRED | `src/main.rs` lines 995-997 |
| `src/commands/contacts.rs add_group_member` | `CardDavClient::add_group_member` | client method call | WIRED | `src/commands/contacts.rs` line 370: `client.add_group_member(&group.id, contact_id).await?` |
| `src/commands/contacts.rs create_contact` (--group path) | `CardDavClient::add_group_member` | post-create group assignment | WIRED | `src/commands/contacts.rs` line 197: `client.add_group_member(&group_obj.id, &contact.id).await` |
| `MutationRoot::add_group_member` | `CardDavClient::add_group_member` | carddav client from AppContext | WIRED | `src/mcp/graphql/mutation.rs` line 422: `carddav.add_group_member(&group_id, &contact_id).await` |
| `MutationRoot::add_group_member` | `GqlContactGroup::with_members` | response conversion | WIRED | `src/mcp/graphql/mutation.rs` line 429: `Ok(GqlContactGroup::with_members(updated, members))` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `src/commands/contacts.rs add_group_member` | `updated` (ContactGroup) | `client.add_group_member` -> ETag-guarded HTTP PUT -> `get_group_by_id` on each retry | Yes — reads from server, returns modified group from real HTTP PUT | FLOWING |
| `src/commands/contacts.rs add_group_member` | `members` (Vec<Contact>) | `client.resolve_group_members(&updated)` -> batch fetch of contacts | Yes — resolves UIDs against real contacts | FLOWING |
| `src/mcp/graphql/mutation.rs add_group_member` | `updated`, `members` | same CardDavClient transport | Yes — identical call chain | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `contacts groups add-member` CLI subcommand is registered and shows help | `cargo run -- contacts groups add-member --help` | Shows `<GROUP_ID>` and `<CONTACT_ID>` positional args with descriptions | PASS |
| `contacts groups remove-member` CLI subcommand is registered and shows help | `cargo run -- contacts groups remove-member --help` | Shows `<GROUP_ID>` and `<CONTACT_ID>` positional args with descriptions | PASS |
| `contacts create --help` shows `--group` option | `cargo run -- contacts create --help` | Shows `--group <GROUP>  Assign to group at creation (group ID or name)` | PASS |
| Full library test suite passes | `cargo test --lib` | 177 passed, 0 failed | PASS |
| `cargo build` compiles clean | `cargo build` | exit 0, no errors | PASS |
| `cargo clippy -- -D warnings` passes | `cargo clippy -- -D warnings` | exit 0, zero warnings | PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| MBR-01 | 19-01 | User can add a contact to a group | SATISFIED | `CardDavClient::add_group_member` implemented in `src/carddav/mod.rs` lines 624-696 |
| MBR-02 | 19-01 | User can remove a contact from a group | SATISFIED | `CardDavClient::remove_group_member` implemented in `src/carddav/mod.rs` lines 704-773 |
| MBR-03 | 19-02 | User can create a contact and assign it to a group in one command via `--group` | SATISFIED | `create_contact(input, group: Option<&str>)` in `src/commands/contacts.rs` lines 169-224; `--group` flag in `src/main.rs` line 381 |
| CLI-02 | 19-02 | User can manage membership via `contacts groups add-member` and `remove-member` | SATISFIED | Both `GroupsCommands::AddMember` and `GroupsCommands::RemoveMember` variants wired to handlers in `src/main.rs` |
| CLI-04 | 19-02 | `contacts create --group <id>` assigns the new contact to a group at creation | SATISFIED | `--group: Option<String>` arg on `ContactsCommands::Create`; `group.as_deref()` passed through at dispatch |
| MCP-03 | 19-03 | AI agents can manage membership via `addGroupMember`, `removeGroupMember` | SATISFIED | Both resolvers present in `src/mcp/graphql/mutation.rs` lines 413-450 |

All 6 requirements assigned to Phase 19 in REQUIREMENTS.md are satisfied. No orphaned requirements found (REQUIREMENTS.md assigns MBR-01, MBR-02, MBR-03, CLI-02, CLI-04, MCP-03 to Phase 19 — exactly matching the plan frontmatter declarations).

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| None | — | — | No stubs, TODOs, or placeholder returns found in phase-modified files |

Scan performed on: `src/carddav/mod.rs`, `src/main.rs`, `src/commands/contacts.rs`, `src/mcp/graphql/mutation.rs`

Key findings: No `return null`, no `return []`, no hardcoded empty responses. The partial-failure path in `create_contact` correctly returns `success: true` with both `data` and `error` fields — this is intentional design (not a stub), as documented in the Plan 02 decisions.

### Human Verification Required

#### 1. Live add-member against Fastmail CardDAV

**Test:** With valid Fastmail credentials configured, run `fastmail-cli contacts groups add-member <group-id> <contact-id>` where both IDs exist in the account's address book.
**Expected:** JSON output with `{"success":true,"data":{"id":"...","name":"...","member_count":N,"members":[...]}}` where the added contact appears in the `members` array and `member_count` increments. Running the command again should be idempotent (same output, no error).
**Why human:** Requires live Fastmail credentials and a real CardDAV endpoint with populated address book data.

#### 2. Live remove-member against Fastmail CardDAV

**Test:** Run `fastmail-cli contacts groups remove-member <group-id> <contact-id>` where the contact is a member of the group.
**Expected:** JSON output with updated group where the removed contact no longer appears in `members`. Idempotency: running again when contact is not a member should return `Ok` without error.
**Why human:** Same — requires live server to exercise ETag retry logic and observe real HTTP responses.

#### 3. Partial-failure path for `contacts create --group`

**Test:** Run `fastmail-cli contacts create --group nonexistent-group-name --name "Test" --email test@example.com`.
**Expected:** Contact is created (appears in `contacts list` output), and the JSON response shows `"success":true` with a `"data"` field containing the contact and an `"error"` field with the message `"Contact created (ID: ...) but group not found: .... Run contacts groups add-member ... to retry."`.
**Why human:** Requires live CardDAV to create the contact; partial-failure branch only executes when server returns a real GroupNotFound error.

#### 4. MCP addGroupMember / removeGroupMember via GraphQL client

**Test:** Start the MCP server (`fastmail-cli mcp`) and invoke `addGroupMember(groupId: "...", contactId: "...")` via a GraphQL client or MCP-connected agent.
**Expected:** Returns a `ContactGroup` object with `id`, `name`, `memberCount`, and a non-empty `members` array containing `GqlContact` objects with resolved name/email fields.
**Why human:** MCP server is a running process requiring live credentials; schema introspection alone cannot verify that member resolution produces real contact data.

### Gaps Summary

No gaps found. All must-haves from Plans 19-01, 19-02, and 19-03 are present and wired in the actual codebase. Phase goal is achieved at the automated verification level.

---

_Verified: 2026-04-13T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
