---
phase: 03-carddav-write-operations
generated: 2026-04-03
status: complete
---

# Phase 3 Research

## Existing Assets

- `src/carddav/mod.rs` already contained authenticated CardDAV read operations, vCard parsing, and the `serialize_vcard` primitive needed for PUT bodies.
- `Contact` already carries `href` and `etag`, so phase 3 can stay protocol-focused and avoid implicit lookup.
- `Error` already exposes `ContactNotFound` and `ContactConflict`, matching the roadmap’s write failure model.

## Implementation Direction

- Add low-level `create_contact`, `update_contact`, and `delete_contact` methods on `CardDavClient`.
- Keep create/update/delete request construction simple and explicit:
  - `PUT` with `If-None-Match: *` for creates
  - `PUT` with `If-Match` for updates
  - `DELETE` with `If-Match` for deletes
- Return the created href plus latest ETag on create, return the latest ETag on update, and return `()` on delete.
- Add small pure helpers for href construction, `Location`/`ETag` extraction, and status-to-error mapping so offline unit tests can verify behavior without a network mock server.

## Risks

- Fastmail may omit `ETag` or `Location` on some successful writes. The implementation should tolerate that and fall back to the constructed href / sent ETag when necessary.
- Phase 4 needs exact-contact resolution by `id`; phase 3 therefore also benefits from read-side helpers for default addressbook discovery and lookup by ID across addressbooks.
