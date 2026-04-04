---
phase: 03-carddav-write-operations
verified: 2026-04-03T00:00:00Z
status: passed
score: 6/6 must-haves verified
re_verification: false
gaps: []
human_verification: []
---

# Phase 3 Verification

**Phase Goal:** The CardDAV client can create, update, and delete contacts on Fastmail's server with correct conditional headers that prevent data loss from concurrent edits.

## Checks

| # | Requirement | Status | Evidence |
|---|-------------|--------|----------|
| 1 | Create uses PUT with `If-None-Match: *` | VERIFIED | `src/carddav/mod.rs` `create_contact` sends `PUT`, `Content-Type: text/vcard; charset=utf-8`, and `IF_NONE_MATCH` `*` |
| 2 | Update uses PUT with `If-Match` | VERIFIED | `src/carddav/mod.rs` `update_contact` sends `PUT` with `IF_MATCH` and serializes the full vCard body |
| 3 | Delete uses DELETE with `If-Match` | VERIFIED | `src/carddav/mod.rs` `delete_contact` sends `DELETE` with `IF_MATCH` |
| 4 | 412 maps to `ContactConflict` | VERIFIED | `map_write_response` returns `Error::ContactConflict`; `test_map_write_response_conflict_uses_server_etag` passes |
| 5 | 404 maps to `ContactNotFound` | VERIFIED | `map_write_response` returns `Error::ContactNotFound`; `test_map_write_response_not_found` passes |
| 6 | Phase 4 has lookup primitives it needs | VERIFIED | `default_addressbook_href` and `get_contact_by_id` were added on `CardDavClient` and are used by the higher-level command layer |

## Result

Phase 3 passed local verification. The write-layer code is complete, compiles cleanly, and is covered by offline tests. Live Fastmail integration remains a normal post-merge validation step, not a local blocker.
