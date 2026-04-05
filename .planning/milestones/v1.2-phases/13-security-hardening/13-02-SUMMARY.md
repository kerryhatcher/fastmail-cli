---
phase: 13-security-hardening
plan: "02"
subsystem: caldav
tags: [security, ical, injection-prevention, rfc5545]
dependency_graph:
  requires: []
  provides: [SEC-03]
  affects: [src/caldav/mod.rs]
tech_stack:
  added: []
  patterns: [enum-validation, escape-free-form, rfc5545-grammar-validation]
key_files:
  created: []
  modified:
    - src/caldav/mod.rs
decisions:
  - "escape_ical_value() reused for email (already existed); new sanitize_role/sanitize_partstat perform RFC 5545 enum validation with uppercase normalization"
  - "serialize_rrule drops invalid UNTIL silently (caller responsibility); invalid FREQ produces empty FREQ= rather than panic"
  - "is_valid_byday handles 1-2 digit signed numeric prefix per RFC 5545 weekdaynum grammar"
metrics:
  duration: "~8 min"
  completed: "2026-04-04"
  tasks_completed: 2
  files_modified: 1
---

# Phase 13 Plan 02: iCal Attendee + RRULE Injection Prevention Summary

SEC-03 closed: iCalendar attendee email escaped via escape_ical_value, role/partstat validated against RFC 5545 enums, RRULE FREQ/UNTIL/BYDAY grammar-validated with seven unit tests proving injection attempts cannot break VCALENDAR syntax.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add RRULE validators + escape attendee fields | 0979590 | src/caldav/mod.rs |
| 2 | Unit tests for iCal injection prevention | 8fc8d6a | src/caldav/mod.rs |

## What Was Built

### New Private Helpers (src/caldav/mod.rs)

- `is_valid_freq(s)` — RFC 5545 §3.3.10: FREQ must be one of 7 enumerated values (case-insensitive, normalized to uppercase)
- `is_valid_rrule_until(s)` — validates DATE (YYYYMMDD) or DATE-TIME (YYYYMMDDTHHMMSSZ) format strictly
- `is_valid_byday(s)` — validates weekdaynum: optional signed 1-2 digit prefix + SU/MO/TU/WE/TH/FR/SA
- `sanitize_role(role)` — validates against CHAIR/REQ-PARTICIPANT/OPT-PARTICIPANT/NON-PARTICIPANT; returns None for unknown values
- `sanitize_partstat(partstat)` — validates against NEEDS-ACTION/ACCEPTED/DECLINED/TENTATIVE/DELEGATED/COMPLETED/IN-PROCESS; returns None for unknown

### Updated Functions

- `serialize_attendee` — email now escaped via `escape_ical_value()`, role/partstat filtered through sanitize functions (invalid values dropped silently)
- `serialize_rrule` — FREQ validated and uppercased (empty on invalid), UNTIL dropped if not matching date/datetime grammar, BYDAY entries filtered individually

### Tests (sec03_tests module)

7 tests proving injection prevention:
1. `test_attendee_email_newline_escaped` — `\n` in email becomes literal `\n`, output stays single line
2. `test_attendee_role_invalid_dropped` — `INJECT;X` role produces no ROLE= parameter
3. `test_attendee_partstat_case_normalized` — `accepted` serializes as `PARTSTAT=ACCEPTED`
4. `test_rrule_until_invalid_dropped` — `2026-04-30;INJECT` produces no UNTIL=
5. `test_rrule_until_valid_kept` — `20260430` and `20260430T120000Z` both pass through
6. `test_rrule_byday_invalid_dropped` — `BAD\n` entry filtered out; `MO` and `-1SU` kept
7. `test_rrule_freq_invalid_empty` — `EVIL;X` freq produces `FREQ=`

## Verification

- `cargo build` — OK
- `cargo test` — 131 passed; 0 failed (includes all 7 new sec03_tests)
- `cargo clippy --all-targets -- -D warnings` — clean

## Deviations from Plan

None - plan executed exactly as written.

The plan's `serialize_rrule` snippet used the experimental `let ... && ...` syntax in an `if let` (`if let Some(until) = ... && is_valid_rrule_until(until)`). This was rewritten as a standard nested `if` block for clarity while preserving identical semantics — Rust 2024 supports the syntax but the simpler form is more idiomatic for this pattern.

## Known Stubs

None.

## Self-Check: PASSED

- src/caldav/mod.rs modified: FOUND
- Commit 0979590 (feat): FOUND
- Commit 8fc8d6a (test): FOUND
- All 7 sec03_tests passing: VERIFIED
