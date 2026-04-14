---
phase: 19-group-membership-management
plan: "02"
subsystem: cli-commands
tags: [cli, groups, membership, carddav, partial-failure]
dependency_graph:
  requires: [19-01]
  provides: [add_group_member_cmd, remove_group_member_cmd, create_contact_group_flag]
  affects: [src/main.rs, src/commands/contacts.rs]
tech_stack:
  added: []
  patterns: [partial-failure reporting with retry instructions, group resolution by ID-or-name, JSON output with resolved members]
key_files:
  created: []
  modified:
    - src/main.rs
    - src/commands/contacts.rs
decisions:
  - "Output::success used for full success; direct Output struct construction for partial-failure (contact created but group add failed) to allow success:true + error:Some simultaneously"
  - "create_contact signature changed from (input) to (input, group: Option<&str>) — coordinated change across main.rs dispatch and contacts.rs handler"
  - "Partial failure retry instructions embed the CLI command with concrete IDs so users can recover without guessing"
metrics:
  duration: "~10 minutes"
  completed: "2026-04-14T05:00:00Z"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 2
---

# Phase 19 Plan 02: Group Membership CLI Commands Summary

CLI commands `contacts groups add-member`, `contacts groups remove-member`, and `contacts create --group` wired to the Plan 01 CardDavClient transport methods with JSON output, group-by-ID-or-name resolution, and partial-failure reporting with retry instructions.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add AddMember/RemoveMember CLI variants and --group flag | c121fa2 | src/main.rs |
| 2 | Implement CLI command handlers for membership and --group | 15998d6 | src/commands/contacts.rs |

## What Was Built

**Task 1 — `src/main.rs`:**
- `GroupsCommands::AddMember { group_id, contact_id }` variant with doc comments
- `GroupsCommands::RemoveMember { group_id, contact_id }` variant with doc comments
- `--group: Option<String>` field on `ContactsCommands::Create`
- Dispatch arms for AddMember and RemoveMember calling `commands::add_group_member` / `commands::remove_group_member`
- Modified `Create` dispatch to destructure `group` and pass `group.as_deref()` to `create_contact`

**Task 2 — `src/commands/contacts.rs`:**

`pub async fn add_group_member(group_id_or_name, contact_id)`:
- Resolves group via `resolve_group` (ID first, then name fallback)
- Calls `client.add_group_member(&group.id, contact_id)` (ETag-retry transport from Plan 01)
- Resolves member contacts via `client.resolve_group_members`
- Outputs JSON with `{id, name, href, etag, member_count, members}`

`pub async fn remove_group_member(group_id_or_name, contact_id)`:
- Identical structure, calls `client.remove_group_member`

`create_contact(input, group: Option<&str>)` modified:
- `None` path: original behavior, outputs contact JSON with "Contact created" message
- `Some(gid)` path:
  - `resolve_group` failure: outputs `success:true, data:contact, error:"Contact created (ID: …) but group not found: … Run contacts groups add-member … to retry."`
  - `add_group_member` failure: same partial-failure pattern with "group add failed"
  - Full success: outputs `{contact, group_id, message: "Contact created and added to group <name>"}`

## Verification Results

- `cargo build`: exit 0
- `cargo clippy -- -D warnings`: exit 0 (zero warnings)
- `cargo test --lib`: 177 passed, 0 failed
- `cargo run -- contacts groups add-member --help`: shows GROUP_ID and CONTACT_ID args
- `cargo run -- contacts groups remove-member --help`: shows GROUP_ID and CONTACT_ID args
- `cargo run -- contacts create --help`: shows `--group <GROUP>` option

## Deviations from Plan

None — plan executed exactly as written. The `Output` struct construction for partial-failure cases matched the plan's guidance (direct struct instantiation with `success: true` and `error: Some(...)`). The `serde_json::json!(contact)` wrapper in the full-success `None` branch ensures type consistency across all branches.

## Known Stubs

None. All handlers call real CardDavClient methods implemented in Plan 01. No mocked or hardcoded data.

## Self-Check: PASSED

- Commit `c121fa2` exists: confirmed via git log
- Commit `15998d6` exists: confirmed via git log
- `src/main.rs` contains `AddMember`, `RemoveMember`, `group: Option<String>`, `commands::add_group_member`, `commands::remove_group_member`, `group.as_deref()`
- `src/commands/contacts.rs` contains `pub async fn add_group_member`, `pub async fn remove_group_member`, `group: Option<&str>`, `Contact created (ID:`, `contacts groups add-member`, `resolve_group`
- All 9 acceptance criteria from both tasks satisfied
