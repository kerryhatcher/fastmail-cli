---
phase: 18-group-data-model-crud-and-base-surfaces
verified: 2026-04-14T02:00:00Z
status: passed
score: 15/15 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 6/15
  gaps_closed:
    - "User can run `contacts groups list` and see all contact groups"
    - "User can run `contacts groups create <name>` and create a new empty group"
    - "User can run `contacts groups get <id>` and see group details with resolved members"
    - "User can run `contacts groups rename <id> <new-name>` and rename a group"
    - "User can run `contacts groups delete <id> --confirm` and delete a group"
    - "User running `contacts groups delete <id>` without --confirm gets an error"
    - "AI agent can call listGroups query and receive Vec of ContactGroup with id, name, memberCount"
    - "AI agent can call getGroup query with id and receive ContactGroup with resolved members array"
    - "AI agent can call createGroup mutation with name and receive the created group"
    - "AI agent can call renameGroup mutation with id and newName and receive updated group"
    - "AI agent can call deleteGroup mutation with Preview action and receive confirmation token"
    - "AI agent can call deleteGroup mutation with Confirm action and valid token and group is deleted"
  gaps_remaining: []
  regressions: []
---

# Phase 18: Group Data Model, CRUD, and Base Surfaces — Verification Report

**Phase Goal:** Users can create, list, inspect, rename, and delete contact groups — and existing `contacts list` is unaffected by group vCards
**Verified:** 2026-04-14T02:00:00Z
**Status:** PASSED
**Re-verification:** Yes — after gap closure (cherry-pick of Plans 18-02 and 18-03 implementation)

---

## Re-verification Summary

The initial verification (2026-04-14T00:30:00Z) found that Plans 18-02 (CLI surface) and 18-03 (MCP/GraphQL surface) had SUMMARY.md files committed but no source code implemented. The source code was subsequently cherry-picked in. This re-verification confirms all 15 truths now pass.

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | ContactGroup struct exists with id, name, member_uids, href, etag | VERIFIED | `src/carddav/mod.rs:73` — struct defined with all 5 fields |
| 2 | parse_vcard() returns None for group vCards | VERIFIED | `src/carddav/mod.rs` — early return on X-ADDRESSBOOKSERVER-KIND:group |
| 3 | parse_group_vcard() extracts id, name, member_uids | VERIFIED | `src/carddav/mod.rs` — function present with unit tests |
| 4 | serialize_group_vcard() emits valid vCard 3.0 with group extensions | VERIFIED | `src/carddav/mod.rs` — function present |
| 5 | CardDavClient has 6 group CRUD methods | VERIFIED | list_groups, get_group_by_id, get_group_by_name, create_group, rename_group, delete_group confirmed |
| 6 | Error enum has GroupNotFound, GroupConflict, GroupAmbiguous | VERIFIED | `src/error.rs:63,67,74` |
| 7 | Existing contacts list unaffected by group vCards | VERIFIED | parse_vcard() returns None early for groups; 174 carddav tests pass |
| 8 | User can run `contacts groups list` | VERIFIED | GroupsCommands::List → commands::list_groups() wired at `src/main.rs:955` |
| 9 | User can run `contacts groups create <name>` | VERIFIED | GroupsCommands::Create → commands::create_group() wired at `src/main.rs:956` |
| 10 | User can run `contacts groups get <id>` | VERIFIED | GroupsCommands::Get → commands::get_group() wired at `src/main.rs:957` |
| 11 | User can run `contacts groups rename <id> <new-name>` | VERIFIED | GroupsCommands::Rename → commands::rename_group() wired at `src/main.rs:958` |
| 12 | User can run `contacts groups delete <id> --confirm` | VERIFIED | GroupsCommands::Delete → if !(confirm || yes) guard → commands::delete_group() wired |
| 13 | Delete without --confirm gets an error | VERIFIED | `src/main.rs:962-968` — Output::error + bail! guard enforced |
| 14 | AI agent can call listGroups / getGroup queries | VERIFIED | Both resolvers present at `src/mcp/graphql/query.rs:188,199`; GqlContactGroup at `src/mcp/graphql/types.rs:482` |
| 15 | AI agent can call createGroup / renameGroup / deleteGroup mutations | VERIFIED | All three resolvers at `src/mcp/graphql/mutation.rs:264,300,338`; GroupDeleteAction::Preview/Confirm pattern with HMAC token |

**Score: 15/15 truths verified**

---

### Required Artifacts

| Artifact | Plan | Status | Details |
|----------|------|--------|---------|
| `src/carddav/mod.rs` | 18-01 | VERIFIED | ContactGroup struct, 6 CRUD methods, parse/serialize functions, error integration |
| `src/error.rs` | 18-01 | VERIFIED | GroupNotFound, GroupConflict, GroupAmbiguous all present |
| `src/main.rs` | 18-02 | VERIFIED | GroupsCommands enum at line 434; Groups(GroupsCommands) variant at line 430; all 5 match arms wired |
| `src/commands/contacts.rs` | 18-02 | VERIFIED | resolve_group helper + list_groups, create_group, get_group, rename_group, delete_group all present |
| `src/mcp/graphql/types.rs` | 18-03 | VERIFIED | GqlContactGroup, GroupDeleteAction, GqlGroupMutationResult, GqlGroupDeleteResult all present |
| `src/mcp/graphql/query.rs` | 18-03 | VERIFIED | list_groups and get_group resolvers present and wired to CardDavClient |
| `src/mcp/graphql/mutation.rs` | 18-03 | VERIFIED | create_group, rename_group, delete_group resolvers present; HMAC token pattern correct |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/main.rs GroupsCommands match` | `commands::list_groups()` | function call | WIRED | `src/main.rs:955` confirmed |
| `src/main.rs GroupsCommands::Delete` | confirm guard | `if !(confirm \|\| yes)` | WIRED | `src/main.rs:962` confirmed |
| `src/commands/contacts.rs list_groups` | `CardDavClient::list_groups()` | `client.list_groups().await` | WIRED | `src/commands/contacts.rs:221` confirmed |
| `src/commands/contacts.rs get_group` | `CardDavClient::resolve_group_members()` | `client.resolve_group_members(&group).await` | WIRED | `src/commands/contacts.rs:254` confirmed |
| `src/mcp/graphql/query.rs list_groups` | `CardDavClient::list_groups()` | `app_ctx.get_carddav().await` | WIRED | `src/mcp/graphql/query.rs:190-195` confirmed |
| `src/mcp/graphql/query.rs get_group` | `resolve_group_members` | `carddav.resolve_group_members(&group)` | WIRED | `src/mcp/graphql/query.rs:210-214` confirmed |
| `src/mcp/graphql/mutation.rs delete_group` | `AppContext::confirmation_token` | HMAC token for Preview/Confirm | WIRED | `src/mcp/graphql/mutation.rs:348` confirmed |
| `src/carddav/mod.rs parse_vcard()` | None return for groups | X-ADDRESSBOOKSERVER-KIND:group early return | WIRED | Confirmed from Plan 18-01 |
| `src/carddav/mod.rs list_groups()` | `parse_groups_from_xml()` | REPORT response | WIRED | Confirmed from Plan 18-01 |
| `src/carddav/mod.rs create_group()` | `serialize_group_vcard()` | PUT with If-None-Match | WIRED | Confirmed from Plan 18-01 |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|--------------|--------|--------------------|--------|
| `list_groups()` (carddav) | `Vec<ContactGroup>` | REPORT HTTP + parse_groups_from_xml | Yes — XML parsing of server response | FLOWING |
| `create_group()` (carddav) | `ContactCreateResult` | PUT HTTP response + location header | Yes — server response | FLOWING |
| `delete_group()` (carddav) | `()` | DELETE HTTP response | Yes — server response | FLOWING |
| `list_groups` (CLI handler) | `groups: Vec<ContactGroup>` | `client.list_groups().await` | Yes — live carddav call | FLOWING |
| `get_group` (CLI handler) | `group + members` | `resolve_group + resolve_group_members` | Yes — live carddav calls | FLOWING |
| `list_groups` (GraphQL resolver) | `Vec<GqlContactGroup>` | `carddav.list_groups().await` | Yes — live carddav call | FLOWING |
| `get_group` (GraphQL resolver) | `GqlContactGroup` with members | `get_group_by_id + resolve_group_members` | Yes — live carddav calls | FLOWING |
| `create_group` (GraphQL mutation) | `GqlGroupMutationResult` | `carddav.create_group()` | Yes — live carddav PUT | FLOWING |
| `rename_group` (GraphQL mutation) | `GqlGroupMutationResult` | `carddav.rename_group()` | Yes — live carddav PUT | FLOWING |
| `delete_group` (GraphQL mutation) | `GqlGroupDeleteResult` | `carddav.delete_group()` (Confirm) or token (Preview) | Yes — live carddav DELETE | FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Build compiles cleanly | `cargo build -p fastmail-cli` | Finished in 15.01s, 0 errors | PASS |
| Full test suite | `cargo test -p fastmail-cli` | 203 tests passing (174+9+1+1+3+8+2+5), 0 failed | PASS |
| GroupsCommands enum in binary | grep `enum GroupsCommands` src/main.rs | Found at line 433 | PASS |
| Groups variant in ContactsCommands | grep `Groups(GroupsCommands)` src/main.rs | Found at line 430 | PASS |
| Confirm guard wired | grep `if !(confirm \|\| yes)` src/main.rs | Found at line 962 | PASS |
| All 5 handler functions | grep `pub async fn.*_group` src/commands/contacts.rs | list_groups, create_group, get_group, rename_group, delete_group confirmed | PASS |
| GqlContactGroup type | grep `pub struct GqlContactGroup` src/mcp/graphql/types.rs | Found at line 482 | PASS |
| GroupDeleteAction enum | grep `pub enum GroupDeleteAction` src/mcp/graphql/types.rs | Found at line 770 | PASS |
| MCP query resolvers | grep `list_groups\|get_group` src/mcp/graphql/query.rs | Both found at lines 188, 199 | PASS |
| MCP mutation resolvers | grep `create_group\|rename_group\|delete_group` src/mcp/graphql/mutation.rs | All three found at lines 264, 300, 338 | PASS |
| HMAC token pattern in deleteGroup | grep `confirmation_token.*delete_group` src/mcp/graphql/mutation.rs | Found at line 348 | PASS |

---

### Requirements Coverage

| Requirement | Phase | Source Plan | Description | Status | Evidence |
|-------------|-------|-------------|-------------|--------|---------|
| GRP-01 | 18 | 18-01 | User can list contact groups showing name, member count, and group ID | SATISFIED | `list_groups()` in CardDavClient + CLI `contacts groups list` + `listGroups` GraphQL |
| GRP-02 | 18 | 18-01 | User can create an empty contact group with a name | SATISFIED | `create_group()` in CardDavClient + CLI `contacts groups create` + `createGroup` GraphQL |
| GRP-03 | 18 | 18-01 | User can get a group's details including resolved member contacts | SATISFIED | `get_group_by_id()` + `resolve_group_members()` + CLI `contacts groups get` + `getGroup` GraphQL |
| GRP-04 | 18 | 18-01 | User can rename an existing contact group | SATISFIED | `rename_group()` in CardDavClient + CLI `contacts groups rename` + `renameGroup` GraphQL |
| GRP-05 | 18 | 18-01 | User can delete a contact group (members NOT deleted) | SATISFIED | `delete_group()` in CardDavClient + CLI `contacts groups delete --confirm` + `deleteGroup` GraphQL |
| CLI-01 | 18 | 18-02 | User can manage groups via `contacts groups` subcommands | SATISFIED | GroupsCommands enum with List/Create/Get/Rename/Delete; all wired in main.rs match block |
| CLI-03 | 18 | 18-02 | Group delete requires `--confirm` flag | SATISFIED | `if !(confirm \|\| yes)` guard at `src/main.rs:962`; error printed and bail! called |
| MCP-01 | 18 | 18-03 | AI agents can query groups via `listGroups` and `getGroup` | SATISFIED | Both resolvers in `src/mcp/graphql/query.rs`; `get_group` includes resolved members |
| MCP-02 | 18 | 18-03 | AI agents can mutate groups via `createGroup`, `renameGroup`, `deleteGroup` | SATISFIED | All three resolvers in `src/mcp/graphql/mutation.rs`; `deleteGroup` uses HMAC Preview/Confirm pattern |

**All 9 phase requirements satisfied.**

---

### Anti-Patterns Found

None. No TODO/FIXME/placeholder comments found in modified files. No empty implementations. All handlers make real CardDavClient calls.

---

### Human Verification Required

None — all correctness claims are programmatically verifiable through file content and build/test results.

---

### Gaps Summary

No gaps. All 15 truths verified. All 9 requirements satisfied. Build clean, 203 tests passing.

The previously missing Plans 18-02 and 18-03 implementations have been cherry-picked in and are confirmed present:

- `GroupsCommands` enum defined at `src/main.rs:433`
- `Groups(GroupsCommands)` variant in `ContactsCommands` at `src/main.rs:430`
- Full CLI dispatch routing including `--confirm` guard in `src/main.rs` match block
- 5 group handler functions in `src/commands/contacts.rs` (resolve_group, list_groups, create_group, get_group, rename_group, delete_group)
- `GqlContactGroup`, `GroupDeleteAction`, `GqlGroupMutationResult`, `GqlGroupDeleteResult` in `src/mcp/graphql/types.rs`
- `list_groups`, `get_group` resolvers in `src/mcp/graphql/query.rs`
- `create_group`, `rename_group`, `delete_group` resolvers in `src/mcp/graphql/mutation.rs`

**Phase goal is ACHIEVED.** Users can create, list, inspect, rename, and delete contact groups via both CLI and MCP/GraphQL. Existing `contacts list` is unaffected by group vCards.

---

_Verified: 2026-04-14T02:00:00Z_
_Verifier: Claude (gsd-verifier)_
