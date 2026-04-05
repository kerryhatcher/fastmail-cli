---
phase: 15-performance
plan: 01
subsystem: api
tags: [rust, carddav, caldav, futures, concurrency, join_all, webdav, vcard, icalendar]

# Dependency graph
requires:
  - phase: 14-mcp-layer-refactor
    provides: CalDavClient and CardDavClient used via Arc<> in AppContext
provides:
  - concurrent CardDAV address-book fetches with partial-failure tolerance
  - concurrent CalDAV calendar fetches with partial-failure tolerance
  - UID-targeted CalDAV calendar-query REPORT for get_event_by_id
  - fallback path from UID REPORT to full-fetch on 400/501
affects:
  - Phase 15 plans 02-04 (concurrent DAV fetches established as pattern)

# Tech tracking
tech-stack:
  added:
    - futures 0.3 (join_all for concurrent async futures)
  patterns:
    - "collect_partial: join_all results -> flatten successes, warn on failures, return Ok(vec) not Err"
    - "UID REPORT + fallback: issue RFC 4791 calendar-query REPORT, fall back to full-fetch on 400/501"

key-files:
  created: []
  modified:
    - Cargo.toml (added futures = "0.3")
    - src/carddav/mod.rs (concurrent search_contacts via join_all, collect_partial_contacts helper)
    - src/caldav/mod.rs (concurrent list_events via join_all, UID REPORT in get_event_by_id, collect_partial_events, build_uid_report_body)

key-decisions:
  - "collect_partial_contacts and collect_partial_events return Ok(vec) not Err on failures — empty result + warnings is correct partial-failure behavior"
  - "build_uid_report_body uses inline xml_escape_uid (not xml_escape) to also escape quote chars for UID embedding"
  - "get_event_by_id falls back to full-fetch (get_event_by_id_full_fetch) when any calendar returns 400/501, not when all do — erring on the safe side"
  - "UID REPORT uses collation=i;unicode-casemap and match-type=equals per D-06 Cyrus IMAP quirk defense"

patterns-established:
  - "Pattern: join_all with per-item (key, Result<Vec<T>>) futures + collect_partial helper for DAV concurrent fetches"
  - "Pattern: UID REPORT with fallback — issue concurrent REPORTs, check 400/501 for fallback trigger"

requirements-completed: [STAB-06, PERF-01, PERF-02]

# Metrics
duration: 9min
completed: 2026-04-05
---

# Phase 15 Plan 01: Concurrent DAV Fetches + UID REPORT Summary

**Concurrent CardDAV/CalDAV fetches via join_all with partial-failure tolerance, plus UID-targeted CalDAV REPORT replacing full event-history sweep in get_event_by_id**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-04-05T00:40:33Z
- **Completed:** 2026-04-05T00:49:45Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `search_contacts` in carddav/mod.rs now fetches all address books concurrently via `futures::future::join_all`, tolerating per-book failures with `tracing::warn` and returning flattened results from successful books
- `list_events` in caldav/mod.rs now fetches all calendars concurrently via `join_all`, with the same partial-failure tolerance pattern
- `get_event_by_id` in caldav/mod.rs now issues RFC 4791 UID-targeted `calendar-query` REPORT per calendar (concurrently), with automatic fallback to full-fetch when any calendar returns 400/501 (indicating unsupported REPORT)
- `build_uid_report_body` helper generates correct REPORT XML with `collation="i;unicode-casemap"`, `match-type="equals"`, and full XML escaping of UID
- 157 tests passing (6 new carddav partial-failure tests, 5 new build_uid_report_body tests, 3 new caldav partial-failure tests)

## Task Commits

1. **Task 1: Concurrent search_contacts + list_events with partial-failure tolerance** - `b086bb4` (feat)
2. **Task 2: UID-targeted CalDAV REPORT for get_event_by_id with fallback** - `a9279f2` (feat)

## Files Created/Modified

- `/home/kwhatcher/projects/fastmail-cli/Cargo.toml` - Added `futures = "0.3"` dependency
- `/home/kwhatcher/projects/fastmail-cli/src/carddav/mod.rs` - Concurrent search_contacts via join_all, parse_contacts_from_xml free function, collect_partial_contacts helper with tests
- `/home/kwhatcher/projects/fastmail-cli/src/caldav/mod.rs` - Concurrent list_events via join_all, collect_partial_events helper, UID REPORT in get_event_by_id, get_event_by_id_full_fetch fallback, build_uid_report_body, xml_escape_uid, with tests

## Decisions Made

- `collect_partial_contacts` and `collect_partial_events` return `Ok(Vec)` (never `Err`) on failures — per D-02, empty result + per-failure warnings is the correct behavior
- `xml_escape_uid` escapes `&`, `<`, `>`, `"`, `'` — superset of `xml_escape` which only handles the first three
- `get_event_by_id` triggers fallback when **any** calendar returns 400/501 (not all) — conservative approach ensures correctness even if one calendar supports REPORT and another doesn't
- `build_uid_report_body` is `pub(crate)` to enable unit testing without exposing as public API

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Pre-existing Arc<Vec<Mailbox>> migration incomplete in cached_mailboxes**
- **Found during:** Task 1 (build verification)
- **Issue:** `jmap/mod.rs` and `mcp/graphql/query.rs` had uncommitted changes from Phase 15-02 PERF-07 Arc migration that were preventing successful test runs. The `query.rs` caller was already updated to use Arc-aware iteration; jmap/mod.rs had the Arc field types.
- **Fix:** These were pre-existing uncommitted changes from prior plan execution. Included in Task 1 commit to bring repo to clean-build state.
- **Files modified:** src/jmap/mod.rs, src/mcp/graphql/query.rs
- **Verification:** cargo build && cargo test all pass with 157 tests
- **Committed in:** b086bb4 (Task 1 commit)

**2. [Rule 1 - Bug] Clippy collapsible_if in get_event_by_id**
- **Found during:** Task 2 (clippy -D warnings check)
- **Issue:** Nested `if let Ok(events) = ... { if let Some(event) = ... { return Ok(event); } }` triggered clippy::collapsible_if
- **Fix:** Collapsed to `if let Ok(events) = ... && let Some(event) = ... { return Ok(event); }`
- **Files modified:** src/caldav/mod.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` passes clean
- **Committed in:** a9279f2 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs)
**Impact on plan:** Both auto-fixes necessary for clean build and clippy compliance. No scope creep.

## Issues Encountered

- `cargo test --lib` failed with "no library targets found" — crate is a binary not a library; used `cargo test` instead. Plan's verify command was wrong; adapted.
- Pre-existing uncommitted Arc migration from Phase 15-02/03 in the working tree needed to be included in the Task 1 commit to produce a compilable state.

## Known Stubs

None — all implementations are functional. The UID REPORT fallback is intentional design (not a stub).

## Next Phase Readiness

- Phase 15 plans 02-04 can proceed; concurrent DAV patterns and join_all usage are now established
- Smoke-test note from STATE.md still applies: Fastmail CalDAV UID REPORT syntax should be verified against a live account before marking Phase 15 complete (Cyrus IMAP quirks)

---
*Phase: 15-performance*
*Completed: 2026-04-05*
