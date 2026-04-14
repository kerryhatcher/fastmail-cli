---
phase: 18-group-data-model-crud-and-base-surfaces
plan: "02"
subsystem: cli
tags: [contact-groups, cli, clap, command-routing]
dependency_graph:
  requires: [ContactGroup, CardDavClient.group_crud]
  provides: [GroupsCommands, list_groups, create_group, get_group, rename_group, delete_group]
  affects: [src/main.rs, src/commands/contacts.rs]
tech_stack:
  added: []
  patterns: [clap subcommand nesting, confirm-guard on delete, ID-first then name fallback resolution]
key_files:
  created: []
  modified:
    - src/main.rs
    - src/commands/contacts.rs
decisions:
  - "resolve_group() tries get_group_by_id() first, falls back to get_group_by_name() — matching plan spec; GroupAmbiguous propagates naturally"
  - "get_group() returns serde_json::json! composite (id, name, href, etag, member_count, members) rather than a typed struct — avoids a new serialization type for a one-off composite response"
  - "--confirm guard on groups delete uses same pattern as contacts delete and calendars delete for CLI consistency"
metrics:
  duration: "~13 minutes"
  completed: "2026-04-14T03:55:37Z"
  tasks_completed: 1
  files_changed: 2
---

# Phase 18 Plan 02: CLI Surface for Contact Group Management Summary

GroupsCommands clap enum with 5 subcommands (list, create, get, rename, delete) wired into ContactsCommands, 5 async handler functions in contacts.rs, resolve_group ID-first/name-fallback helper, and 7 CLI parsing unit tests.

## What Was Built

### GroupsCommands Enum (src/main.rs)

- `enum GroupsCommands` with variants: `List`, `Create { name }`, `Get { id }`, `Rename { id, new_name }`, `Delete { id, confirm, yes }`
- Added `Groups(GroupsCommands)` variant to `ContactsCommands` with `#[command(subcommand)]` attribute
- Match arm `ContactsCommands::Groups(cmd)` in main dispatch block routes all 5 subcommands to handler functions
- `Delete` match arm enforces `if !(confirm || yes)` guard — prints `Output::<()>::error(...)` and bails with `anyhow::bail!` when neither flag is set, consistent with contact/calendar/event delete patterns

### Handler Functions (src/commands/contacts.rs)

- `ContactGroup` added to the `use crate::carddav::` import line
- `resolve_group(&client, id_or_name)` — private async helper; tries `get_group_by_id()` first, on `GroupNotFound` falls back to `get_group_by_name()`, propagates all other errors
- `list_groups()` — calls `client.list_groups()`, wraps in `Output::success`
- `create_group(name)` — builds `ContactGroup` with `Uuid::new_v4()` ID, calls `client.default_addressbook_href()` then `client.create_group()`, sets href/etag from result, prints Output with message "Group created"
- `get_group(id_or_name)` — resolves group, calls `client.resolve_group_members()`, returns composite `serde_json::json!` with id, name, href, etag, member_count, members
- `rename_group(id_or_name, new_name)` — resolves group, extracts href/etag (errors via GroupNotFound/GroupConflict on None), calls `client.rename_group()`, prints success message including new ETag
- `delete_group(id_or_name)` — resolves group, extracts href/etag, calls `client.delete_group()`, prints success message with group ID

### CLI Parsing Tests (src/main.rs #[cfg(test)])

7 new unit tests covering all GroupsCommands variants:

- `cli_parses_contacts_groups_list` — verifies List variant
- `cli_parses_contacts_groups_create` — verifies Create with name "Family"
- `cli_parses_contacts_groups_get` — verifies Get with id "abc-123"
- `cli_parses_contacts_groups_rename` — verifies Rename with id and new_name
- `cli_parses_contacts_groups_delete_with_confirm` — confirm=true, yes=false
- `cli_parses_contacts_groups_delete_with_yes_flag` — confirm=false, yes=true (-y short form)
- `cli_parses_contacts_groups_delete_without_confirm_has_no_flags` — clap accepts parse; app logic rejects at runtime

## Verification Results

- `cargo test`: 172 lib tests + 9 main tests = 181 passed, 0 failed
- `cargo clippy -- -D warnings`: 0 warnings
- `cargo build`: compiles cleanly (01392d2)
- `grep "GroupsCommands" src/main.rs`: enum and match arms present
- `grep "pub async fn.*group" src/commands/contacts.rs`: all 5 handlers present

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all handler functions call real CardDavClient methods. No hardcoded empty values reach output.

## Self-Check: PASSED

- src/main.rs contains `enum GroupsCommands {` ✓
- src/main.rs contains `Groups(GroupsCommands)` ✓
- src/main.rs contains `GroupsCommands::List =>` ✓
- src/main.rs contains `GroupsCommands::Create { name } =>` ✓
- src/main.rs contains `GroupsCommands::Delete { id, confirm, yes }` ✓
- src/main.rs contains `if !(confirm || yes)` ✓
- src/commands/contacts.rs contains `pub async fn list_groups()` ✓
- src/commands/contacts.rs contains `pub async fn create_group(` ✓
- src/commands/contacts.rs contains `pub async fn get_group(` ✓
- src/commands/contacts.rs contains `pub async fn rename_group(` ✓
- src/commands/contacts.rs contains `pub async fn delete_group(` ✓
- src/commands/contacts.rs contains `async fn resolve_group(` ✓
- Commit 01392d2 exists ✓
- 181 tests passing ✓
- 0 clippy warnings ✓
