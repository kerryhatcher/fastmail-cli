---
phase: 18-group-data-model-crud-and-base-surfaces
plan: "01"
subsystem: carddav
tags: [contact-groups, carddav, vcard, crud, data-model]
dependency_graph:
  requires: []
  provides: [ContactGroup, serialize_group_vcard, parse_group_vcard, CardDavClient.group_crud]
  affects: [src/carddav/mod.rs, src/error.rs]
tech_stack:
  added: []
  patterns: [vCard 3.0 X-ADDRESSBOOKSERVER extensions, ETag-guarded writes, partial-failure collection]
key_files:
  created: []
  modified:
    - src/carddav/mod.rs
    - src/error.rs
decisions:
  - "Group vCards are identified by X-ADDRESSBOOKSERVER-KIND:group (vCard 3.0 Fastmail extension), not KIND:group (vCard 4.0)"
  - "parse_vcard() returns None early on KIND:group to prevent group vCards leaking into contact list"
  - "resolve_group_members() fetches all contacts once and filters in-memory to avoid N+1 HTTP calls"
  - "collect_partial_groups() mirrors collect_partial_contacts: partial failures return successes, never error"
metrics:
  duration: "~15 minutes"
  completed: "2026-04-14T03:42:57Z"
  tasks_completed: 2
  files_changed: 2
---

# Phase 18 Plan 01: Group Data Model, vCard Parse/Serialize, and CRUD Methods Summary

ContactGroup struct, X-ADDRESSBOOKSERVER vCard 3.0 parse/serialize round-trip, KIND filter in parse_vcard(), and all 6 CardDavClient group CRUD methods with ETag-guarded writes and fault-tolerant multi-book fetches.

## What Was Built

### ContactGroup Struct (src/carddav/mod.rs)
- `pub struct ContactGroup` with `id`, `name`, `member_uids: Vec<String>`, `href: Option<String>`, `etag: Option<String>`
- Mirrors `Contact` struct conventions — Serialize, Deserialize, Clone, PartialEq, Eq

### Error Variants (src/error.rs)
- `GroupNotFound(String)` — returned when group ID or name not found
- `GroupConflict { id, sent_etag, server_etag }` — returned on 412 Precondition Failed
- `GroupAmbiguous(String)` — returned when multiple groups share the same name

### vCard Functions (src/carddav/mod.rs)
- `parse_vcard()` — early return on `X-ADDRESSBOOKSERVER-KIND:group` (prevents group vCards leaking into contact lists)
- `parse_group_vcard(vcard_str, href, etag)` — extracts UID, FN, X-ADDRESSBOOKSERVER-MEMBER lines; returns None if not a group or no FN
- `serialize_group_vcard(group)` — emits vCard 3.0 with BEGIN:VCARD, VERSION:3.0, UID, FN, X-ADDRESSBOOKSERVER-KIND:group, member urn:uuid: lines, END:VCARD; applies RFC 6350 line folding
- `parse_groups_from_xml(xml)` — mirrors parse_contacts_from_xml, calls parse_group_vcard; sorts by name lowercase
- `build_group_href(addressbook_href, group_id)` — mirrors build_contact_href
- `map_group_write_response(group_id, sent_etag, status, headers, body)` — uses GroupConflict/GroupNotFound error variants
- `collect_partial_groups(results)` — fault-tolerant multi-book collect, mirrors collect_partial_contacts

### CardDavClient Group CRUD Methods (src/carddav/mod.rs)
- `list_groups(&self) -> Result<Vec<ContactGroup>>` — fetches all address books concurrently via join_all, calls fetch_addressbook_groups
- `fetch_addressbook_groups(href)` — private helper, sends REPORT, calls parse_groups_from_xml
- `get_group_by_id(&self, group_id)` — list_groups() + find first match, GroupNotFound on miss
- `get_group_by_name(&self, name)` — list_groups() + filter by exact name; 0 = GroupNotFound, 1 = Ok, 2+ = GroupAmbiguous
- `create_group(&self, addressbook_href, group)` — PUT with If-None-Match: *, returns ContactCreateResult
- `rename_group(&self, href, etag, group, new_name)` — clones group with new name, PUT with If-Match, returns new ETag
- `delete_group(&self, href, etag, group_id)` — DELETE with If-Match: etag
- `resolve_group_members(&self, group)` — fetches all contacts once, filters in-memory by member_uids

## Tests Added

15 new unit tests in `#[cfg(test)]` block in src/carddav/mod.rs:

- `test_parse_group_vcard_valid` — valid group vCard parses id, name, member_uids
- `test_parse_group_vcard_non_group_returns_none` — non-group vCard returns None
- `test_parse_group_vcard_missing_fn_returns_none` — group without FN returns None
- `test_parse_group_vcard_multiple_members` — 3 members extracted correctly
- `test_parse_vcard_filters_group` — parse_vcard returns None for group vCards
- `test_serialize_group_vcard_structure` — all required lines present
- `test_serialize_group_vcard_empty_members` — no MEMBER lines when empty
- `test_parse_groups_from_xml_extracts_only_groups` — ignores contact vCards, parses group vCards with href/etag
- `test_build_group_href` — correct path format
- `test_map_group_write_response_success` — returns ETag from header
- `test_map_group_write_response_conflict` — returns GroupConflict with correct fields
- `test_map_group_write_response_not_found` — returns GroupNotFound
- `test_collect_partial_groups_partial_failure` — partial failures return successes

## Verification Results

- `cargo test -p fastmail-cli`: 172 passed, 0 failed
- `cargo clippy -p fastmail-cli -- -D warnings`: 0 warnings
- `cargo build -p fastmail-cli`: compiles cleanly
- `grep -c "pub struct ContactGroup" src/carddav/mod.rs`: 1
- `grep -c "GroupNotFound" src/error.rs`: 1

## Deviations from Plan

None — plan executed exactly as written. Both tasks were implemented atomically in the same editing session since they targeted overlapping files; captured in a single task commit (945f039).

## Known Stubs

None — all data flows from real vCard parsing. No hardcoded empty values or placeholders reach the public API.

## Self-Check: PASSED

- src/carddav/mod.rs exists and contains `pub struct ContactGroup` ✓
- src/error.rs exists and contains `GroupNotFound` ✓
- Commit 945f039 exists ✓
- 172 tests passing ✓
- 0 clippy warnings ✓
