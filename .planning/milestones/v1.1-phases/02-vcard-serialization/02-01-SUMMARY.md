---
phase: 02-vcard-serialization
plan: 01
subsystem: carddav
tags: [vcard, serialization, tdd, rfc2426, rfc6350]
dependency_graph:
  requires:
    - 01-contact-model-foundation/01-01
  provides:
    - serialize_vcard() pub function in src/carddav/mod.rs
    - address field on Contact struct
    - address field on GqlContact with From<Contact> mapping
    - uuid crate with v4 feature in Cargo.toml
  affects:
    - src/carddav/mod.rs (Contact struct, parse_vcard, new serializer functions)
    - src/mcp/graphql/types.rs (GqlContact struct and From<Contact> impl)
    - Cargo.toml (uuid dependency)
tech_stack:
  added:
    - uuid 1 with v4 feature -- RFC 4122 compliant UUID generation for new contact UIDs
  patterns:
    - TDD RED-GREEN-REFACTOR cycle
    - Pure function serializer (infallible, returns String directly)
    - RFC 6350 §3.2 line folding with is_char_boundary() UTF-8 safety
    - RFC 2426 §5 backslash escaping (backslash-first ordering)
key_files:
  created: []
  modified:
    - src/carddav/mod.rs
    - src/mcp/graphql/types.rs
    - Cargo.toml
decisions:
  - "unescape_value() added to parse_vcard as auto-fix (Rule 1): round-trip test revealed parse_vcard did not unescape backslash sequences, breaking special-character preservation"
  - "uuid::Uuid re-exported (pub use) from carddav module so Phase 3 callers can call Uuid::new_v4() without a direct uuid import"
metrics:
  duration: "5m 21s"
  completed: "2026-03-27"
  tasks_completed: 2
  files_modified: 3
---

# Phase 2 Plan 1: vCard Serialization Summary

**One-liner:** vCard 3.0 serializer with RFC-compliant 75-octet line folding, backslash escaping, N-property name decomposition, and round-trip compatibility with parse_vcard.

## What Was Built

Implemented a pure-function vCard 3.0 serializer in `src/carddav/mod.rs` that converts a `Contact` struct to a standards-compliant vCard string for use as CardDAV PUT request bodies in Phase 3.

### New Functions

- `pub fn serialize_vcard(contact: &Contact) -> String` — produces a valid vCard 3.0 string with all required properties
- `fn fold_line(line: &str) -> String` — folds lines at 75 octets with CRLF+space continuation, UTF-8 character boundary safe
- `fn escape_value(s: &str) -> String` — backslash-escapes `\`, `;`, `,`, `\n` per RFC 2426 §5
- `fn unescape_value(s: &str) -> String` — inverse of escape_value, applied in parse_vcard for round-trip correctness

### Struct Changes

- `Contact.address: Option<String>` added (after `notes`, before `href`)
- `GqlContact.address: Option<String>` added with `From<Contact>` mapping
- `parse_vcard()` now parses ADR property extracting street component (index 2)

### Dependency

- `uuid = { version = "1", features = ["v4"] }` added to Cargo.toml

### Tests Added

26 new unit tests covering:
- `escape_value`: backslash, semicolon, comma, newline, combined, no-special-chars (6 tests)
- `fold_line`: short line, exactly-75, 76-bytes, 200-byte multi-fold, UTF-8 boundary safety (5 tests)
- `serialize_vcard`: basic, full-contact, single-name, three-part-name, four-part-name, address,
  optional-fields-none, email-with/without-label, phone-with-label, escaping, CRLF-endings,
  UID-present, round-trip, round-trip-with-special-chars (14 tests)
- Contact.address field tests (2 tests) and ADR parsing tests (2 tests) from Task 1

Total test count: 72 (was 42, +30 new tests — 4 from Task 1 struct tests + 26 from Task 2)

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add address field to Contact/GqlContact, uuid dependency | 291f981 | Cargo.toml, src/carddav/mod.rs, src/mcp/graphql/types.rs |
| 1 (TDD tests) | Add RED tests for address field behavior | (in 291f981) | src/carddav/mod.rs |
| 2 RED | Add failing tests for vCard serializer | 03bfed9 | src/carddav/mod.rs |
| 2 GREEN | Implement vCard 3.0 serializer with fold/escape | 93b6e26 | src/carddav/mod.rs |

## Decisions Made

1. **unescape_value() added to parse_vcard** — The round-trip test `test_serialize_vcard_round_trip_special_chars` revealed that `parse_vcard` did not unescape backslash sequences. A name containing "Comma, Inc" would serialize to `ORG:Comma\, Inc` but parse back as `Comma\, Inc`. Added `unescape_value()` applied in the `extract_value` closure. This makes round-trip semantics correct for all fields.

2. **uuid::Uuid re-exported as pub use** — The import `use uuid::Uuid` was flagged by clippy as unused (the serializer uses `contact.id` directly, not `Uuid::new_v4()`). Made it `pub use uuid::Uuid` so Phase 3 can access it via `crate::carddav::Uuid::new_v4()` without needing a separate direct dependency. Suppressed the remaining dead_code warning with `#[allow(unused_imports)]`.

3. **Dead-code clippy warnings on private helpers** — `escape_value`, `fold_line`, and `serialize_vcard` show dead_code warnings because they are not yet called from the binary entry point. These warnings are expected and will resolve in Phase 3 when write commands are added.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added unescape_value() to parse_vcard for round-trip correctness**
- **Found during:** Task 2 GREEN phase — `test_serialize_vcard_round_trip_special_chars` failed
- **Issue:** `parse_vcard` returned raw escaped strings (e.g., `Comma\, Inc` instead of `Comma, Inc`) because it had no backslash unescaping. `serialize_vcard` escapes values on write, but the inverse was never implemented in the parser.
- **Fix:** Added `fn unescape_value(s: &str) -> String` and applied it in the `extract_value` closure within `parse_vcard`. This corrects all text fields (FN, ORG, TITLE, NOTE, ADR street).
- **Files modified:** src/carddav/mod.rs
- **Commit:** 93b6e26 (included in GREEN phase commit)

## Verification Results

- `cargo test`: 72 passed, 0 failed
- `cargo clippy`: warnings-only (no errors) — 4 expected dead_code warnings for functions not yet called from binary
- `cargo check`: clean compilation

## Known Stubs

None. All implemented functions produce real output. The `address` field is wired through the full stack (Contact → parse_vcard → serialize_vcard → GqlContact).

## Self-Check: PASSED
