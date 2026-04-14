---
phase: 19-group-membership-management
plan: "01"
subsystem: carddav
tags: [carddav, groups, membership, etag, retry]
dependency_graph:
  requires: []
  provides: [add_group_member, remove_group_member]
  affects: [src/carddav/mod.rs]
tech_stack:
  added: []
  patterns: [ETag-guarded PUT retry loop (max 3), idempotency guard, contact validation before mutation]
key_files:
  created: []
  modified:
    - src/carddav/mod.rs
decisions:
  - "No contact validation in remove_group_member — removing a reference to a deleted contact is a valid cleanup operation"
  - "MAX_RETRIES=3 hard cap matches threat model: prevents infinite spin while tolerating transient concurrent writes"
  - "Idempotency returns Ok(group) without PUT to avoid unnecessary network round-trips"
metrics:
  duration: "~8 minutes"
  completed: "2026-04-14T04:30:31Z"
  tasks_completed: 1
  tasks_total: 1
  files_modified: 1
---

# Phase 19 Plan 01: Group Membership Transport Methods Summary

ETag-guarded `add_group_member` and `remove_group_member` methods on `CardDavClient` using a 3-retry 412-conflict loop with idempotency guards and contact validation.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Implement add_group_member and remove_group_member on CardDavClient | b56afe8 | src/carddav/mod.rs |

## What Was Built

Two new public async methods added to `CardDavClient` in `src/carddav/mod.rs`, placed between `delete_group` and `resolve_group_members`:

**`add_group_member(group_id, contact_uid) -> Result<ContactGroup>`**
- Calls `get_contact_by_id(contact_uid)` first to validate the contact exists (fail fast with `ContactNotFound`)
- Loops up to `MAX_RETRIES=3` attempts
- Each attempt: fetches fresh group state via `get_group_by_id`, checks idempotency (skips PUT if already a member), performs ETag-guarded PUT
- On 412 conflict (ETag mismatch): retries with fresh group state from server
- Uses `serialize_group_vcard` for PUT body and `map_group_write_response` for response mapping
- After retry exhaustion: returns `GroupConflict`

**`remove_group_member(group_id, contact_uid) -> Result<ContactGroup>`**
- Identical retry structure, no contact validation (removing a reference to a deleted contact is valid)
- Idempotency inverted: returns Ok(group) immediately if contact is NOT a member
- Uses `member_uids.retain(|uid| uid != contact_uid)` for the mutation

**Unit tests added (3 new tests):**
- `test_serialize_group_vcard_member_lines_format`: verifies `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` format and no duplicates
- `test_add_group_member_idempotency_guard_logic`: unit-level verification of idempotency guard + push mutation
- `test_remove_group_member_idempotency_guard_logic`: unit-level verification of inverted guard + retain mutation

## Verification Results

- `cargo build`: exit 0
- `cargo clippy -- -D warnings`: exit 0 (zero warnings)
- `cargo test --lib carddav`: 74 passed, 0 failed

## Deviations from Plan

None — plan executed exactly as written. `ContactGroup` already had `Clone` derived (confirmed at line 72), no change needed.

## Known Stubs

None. Both methods are fully implemented with real HTTP PUT logic.

## Self-Check: PASSED

- `b56afe8` commit exists: confirmed via `git log`
- `src/carddav/mod.rs` contains all required patterns (verified via grep acceptance criteria)
- All 9 acceptance criteria grep checks passed
