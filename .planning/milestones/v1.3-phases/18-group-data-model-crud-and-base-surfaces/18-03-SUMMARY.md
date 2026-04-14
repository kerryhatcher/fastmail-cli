---
phase: 18-group-data-model-crud-and-base-surfaces
plan: "03"
subsystem: mcp-graphql
tags: [contact-groups, graphql, mcp, query, mutation, hmac]
dependency_graph:
  requires: [18-01]
  provides: [GqlContactGroup, GroupDeleteAction, GqlGroupMutationResult, GqlGroupDeleteResult, listGroups, getGroup, createGroup, renameGroup, deleteGroup]
  affects: [src/mcp/graphql/types.rs, src/mcp/graphql/query.rs, src/mcp/graphql/mutation.rs]
tech_stack:
  added: []
  patterns: [async-graphql SimpleObject, From<ContactGroup> conversion, HMAC confirmation_token Preview/Confirm pattern]
key_files:
  created: []
  modified:
    - src/mcp/graphql/types.rs
    - src/mcp/graphql/query.rs
    - src/mcp/graphql/mutation.rs
decisions:
  - "Method-level #[graphql(desc = ...)] is not valid in async-graphql v7 inside #[Object] impls — use doc comments (///) instead"
  - "delete_group uses HMAC token keyed on ['delete_group', id] matching existing delete_contact and delete_calendar patterns"
  - "list_groups returns members as empty Vec (count only); get_group resolves members via resolve_group_members()"
metrics:
  duration: "~25 minutes"
  completed: "2026-04-14T04:30:00Z"
  tasks_completed: 2
  files_changed: 3
---

# Phase 18 Plan 03: MCP/GraphQL Group Surface Summary

GqlContactGroup type with From/with_members conversion, GroupDeleteAction enum, result types, listGroups/getGroup query resolvers, and createGroup/renameGroup/deleteGroup mutation resolvers with HMAC confirmation token pattern.

## What Was Built

### GqlContactGroup Type (src/mcp/graphql/types.rs)

- `pub struct GqlContactGroup` — SimpleObject with `id`, `name`, `member_count: i32`, `members: Vec<GqlContact>`, `href: Option<String>`, `etag: Option<String>`
- `impl From<ContactGroup> for GqlContactGroup` — maps `member_uids.len() as i32` to `member_count`, leaves `members` empty (for list_groups performance)
- `impl GqlContactGroup { pub fn with_members(group, contacts) }` — constructor used by get_group that populates resolved member contacts
- `pub enum GroupDeleteAction { Preview, Confirm }` — mirrors ContactDeleteAction and CalendarDeleteAction patterns
- `pub struct GqlGroupMutationResult` — SimpleObject with `success`, `group: Option<GqlContactGroup>`, `message`, `error`
- `pub struct GqlGroupDeleteResult` — SimpleObject with `success`, `deleted_id`, `preview`, `confirmation_token`, `message`, `error`
- Unit tests: `gql_contact_group_from_maps_all_fields` and `gql_contact_group_from_empty_member_uids`

### Query Resolvers (src/mcp/graphql/query.rs)

- `async fn list_groups` — fetches all groups via `carddav.list_groups()`, maps to `GqlContactGroup::from`
- `async fn get_group(id)` — fetches group by ID via `carddav.get_group_by_id()`, resolves members via `carddav.resolve_group_members()`, returns `GqlContactGroup::with_members()`

### Mutation Resolvers (src/mcp/graphql/mutation.rs)

- `async fn create_group(name)` — generates UUID, constructs ContactGroup, calls `carddav.default_addressbook_href()` then `carddav.create_group()`, returns created group with server-assigned href/etag
- `async fn rename_group(id, new_name)` — fetches group by ID, calls `carddav.rename_group()` with href/etag guards, returns renamed group with updated etag
- `async fn delete_group(action, id, confirmation_token)` — Preview returns HMAC token keyed on `["delete_group", id]`; Confirm validates token then calls `carddav.delete_group()`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed invalid `#[graphql(desc = ...)]` method-level attributes**
- **Found during:** Task 2 build
- **Issue:** Plan template used `#[graphql(desc = "...")]` as a standalone attribute on `async fn` inside `#[Object]` impls. async-graphql v7 reports "Unknown field: `desc`" — method descriptions must use `///` doc comments instead.
- **Fix:** Replaced all `#[graphql(desc = "...")]` method attributes with `///` doc comments on list_groups, get_group, create_group, rename_group, and delete_group.
- **Files modified:** `src/mcp/graphql/query.rs`, `src/mcp/graphql/mutation.rs`
- **Commits:** 55a7721

## Known Stubs

None — all resolvers delegate to real CardDavClient methods implemented in Plan 01.

## Self-Check: PASSED

- `src/mcp/graphql/types.rs` contains `pub struct GqlContactGroup {` — FOUND
- `src/mcp/graphql/types.rs` contains `pub enum GroupDeleteAction {` — FOUND
- `src/mcp/graphql/types.rs` contains `pub struct GqlGroupMutationResult {` — FOUND
- `src/mcp/graphql/types.rs` contains `pub struct GqlGroupDeleteResult {` — FOUND
- `src/mcp/graphql/types.rs` contains `impl From<ContactGroup> for GqlContactGroup` — FOUND
- `src/mcp/graphql/types.rs` contains `pub fn with_members(` — FOUND
- `src/mcp/graphql/query.rs` contains `async fn list_groups(` — FOUND
- `src/mcp/graphql/query.rs` contains `async fn get_group(` — FOUND
- `src/mcp/graphql/query.rs` contains `resolve_group_members` — FOUND
- `src/mcp/graphql/mutation.rs` contains `async fn create_group(` — FOUND
- `src/mcp/graphql/mutation.rs` contains `async fn rename_group(` — FOUND
- `src/mcp/graphql/mutation.rs` contains `async fn delete_group(` — FOUND
- `src/mcp/graphql/mutation.rs` contains `GroupDeleteAction::Preview` — FOUND
- `src/mcp/graphql/mutation.rs` contains `GroupDeleteAction::Confirm` — FOUND
- `src/mcp/graphql/mutation.rs` contains `confirmation_token(&["delete_group"` — FOUND
- `cargo build -p fastmail-cli` exits 0 — PASSED
- `cargo clippy -p fastmail-cli -- -D warnings` exits 0 — PASSED
- All tests pass — PASSED
- Commits 0b9dfcc and 55a7721 exist — FOUND
