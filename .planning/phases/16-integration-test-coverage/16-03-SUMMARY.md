---
phase: 16-integration-test-coverage
plan: 03
subsystem: integration-tests
tags: [testing, wiremock, caldav, carddav, crud, concurrent, partial-failure]
dependency_graph:
  requires: [16-01]
  provides: [caldav-concurrent-test, carddav-crud-test]
  affects: [tests/caldav_concurrent.rs, tests/carddav_crud.rs]
tech_stack:
  added: []
  patterns: [wiremock-mocking, partial-failure-tolerance, http-header-assertions]
key_files:
  created:
    - tests/caldav_concurrent.rs
    - tests/carddav_crud.rs
    - tests/fixtures/caldav_calendar_multiget.xml
    - tests/fixtures/carddav_vcard_get.xml
    - tests/fixtures/carddav_contact_created.xml
  modified: []
decisions:
  - "Principal discovery XML must use path-only hrefs (not full URLs) because CalDavClient prepends self.base_url when building the PROPFIND URL"
  - "wiremock up_to_n_times(1) used per PUT variant to serve create 201 first then update 204"
  - "Middle failing calendar is Personal (alphabetically second after sorting: Default, Personal, Work)"
metrics:
  duration: 275s
  completed: "2026-04-05T01:47:17Z"
  tasks: 2
  files: 5
---

# Phase 16 Plan 03: CalDAV Concurrent + CardDAV CRUD Integration Tests Summary

**One-liner:** CalDAV partial-failure tolerance test (3-calendar PROPFIND, middle returns 500) and CardDAV CRUD round-trip test (PUT/PUT/DELETE with header assertions) against wiremock.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | CalDAV concurrent partial-failure test | 619d446 | tests/caldav_concurrent.rs, tests/fixtures/caldav_calendar_multiget.xml |
| 2 | CardDAV CRUD round-trip test | 12bfb52 | tests/carddav_crud.rs, tests/fixtures/carddav_vcard_get.xml, tests/fixtures/carddav_contact_created.xml |

## What Was Built

### Task 1: CalDAV Concurrent Partial-Failure Test

`tests/caldav_concurrent.rs` tests the Phase 15-01 STAB-06 behavior: `list_events` fans out REPORT requests to all discovered calendars concurrently via `join_all`, and a single failing calendar does not abort the entire call.

Setup: 3 calendars discovered via PROPFIND (fixture `caldav_calendars_propfind.xml` — Default, Personal, Work). Personal (alphabetically middle) responds with HTTP 500 to its REPORT. Default and Work respond with `caldav_calendar_multiget.xml` (1 event each).

Assertions:
- `list_events` returns `Ok` with at least 2 events (from Default + Work)
- Exactly 3 REPORT requests were issued (no short-circuit on failure)

`tests/fixtures/caldav_calendar_multiget.xml`: Minimal CalDAV multistatus with one VEVENT (UID: event-1@test, SUMMARY: Event One, 2026-04-10).

### Task 2: CardDAV CRUD Round-Trip Test

`tests/carddav_crud.rs` exercises the full HTTP wire sequence for contact lifecycle:
1. `create_contact` — PUT with `If-None-Match: *` → 201 Created + ETag + Location
2. `update_contact` — PUT with `If-Match: "etag-1"` → 204 No Content + refreshed ETag
3. `delete_contact` — DELETE with `If-Match: "etag-2"` → 204 No Content

Wire-format assertions verify:
- 2 PUTs and 1 DELETE observed at server level
- First PUT carries `If-None-Match: *` (optimistic create guard)
- Second PUT carries `If-Match: "etag-1"` (concurrency check on update)
- DELETE carries `If-Match: "etag-2"` (final ETag after update)

Fixtures `carddav_vcard_get.xml` and `carddav_contact_created.xml` are included but not referenced in test assertions — they serve as reference documents for future search/list tests.

## Verification

```
cargo test --test caldav_concurrent --test carddav_crud
  Running tests/caldav_concurrent.rs
  test list_events_tolerates_single_failing_calendar ... ok
  Running tests/carddav_crud.rs
  test carddav_crud_roundtrip ... ok
```

Both tests run offline (no live Fastmail account required).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed InvalidPort error in CalDAV principal discovery XML**
- **Found during:** Task 1 (first test run)
- **Issue:** The plan's example code used the full mock server URI (including scheme+host+port) in the `<d:href>` of the calendar-home-set. `CalDavClient` then prepended `self.base_url` to that value, producing a double-scheme URL that reqwest rejected with `InvalidPort`.
- **Fix:** Changed principal discovery XML to use a path-only href (`/dav/calendars/user/test@example.com/`) matching how the Fastmail server returns it.
- **Files modified:** tests/caldav_concurrent.rs
- **Commit:** 619d446 (included in task commit)

## Known Stubs

None.

## Self-Check: PASSED

- tests/caldav_concurrent.rs: FOUND
- tests/carddav_crud.rs: FOUND
- tests/fixtures/caldav_calendar_multiget.xml: FOUND
- tests/fixtures/carddav_vcard_get.xml: FOUND
- tests/fixtures/carddav_contact_created.xml: FOUND
- Commit 619d446: FOUND
- Commit 12bfb52: FOUND
