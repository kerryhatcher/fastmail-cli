---
phase: 03-carddav-write-operations
plan: 01
subsystem: carddav
tags: [carddav, contacts, put, delete, etag]
one-liner: "CardDAV write operations now support create, update, and delete with correct conditional header handling and error mapping."
key_files:
  created: []
  modified:
    - src/carddav/mod.rs
    - src/commands/contacts.rs
metrics:
  completed: "2026-04-03"
  tasks_completed: 2
  files_modified: 2
---

# Phase 3 Plan 1 Summary

Added the protocol-layer CardDAV write operations required for contact CRUD.

## What Changed

- `src/carddav/mod.rs`
  - added `ContactCreateResult`
  - added `create_contact`, `update_contact`, and `delete_contact`
  - added `default_addressbook_href` and `get_contact_by_id`
  - added pure helpers for contact href construction, `Location`/`ETag` extraction, and status-to-error mapping
- `src/commands/contacts.rs`
  - started consuming the new phase-3 contracts via reusable contact record helpers

## Verification-Relevant Outcomes

- create uses `PUT` with `If-None-Match: *`
- update uses `PUT` with `If-Match`
- delete uses `DELETE` with `If-Match`
- 412 maps to `ContactConflict`
- 404 maps to `ContactNotFound`
- helper-level unit tests cover the offline semantics without requiring live Fastmail access
