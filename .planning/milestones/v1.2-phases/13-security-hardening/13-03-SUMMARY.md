---
phase: 13-security-hardening
plan: 03
subsystem: api
tags: [percent-encoding, url-encoding, jmap, security, rust]

# Dependency graph
requires:
  - phase: 12-foundation-safety
    provides: stable JmapClient with HTTP client and download_blob() foundation
provides:
  - JMAP blob download URL segments are percent-encoded with NON_ALPHANUMERIC set
  - encode_blob_url_segment() helper function for URL-safe segment encoding
  - Unit tests proving encoding behavior for spaces, Unicode, URL-reserved chars, and alphanumeric passthrough
affects: [13-security-hardening, future D-07 filename support in download_blob]

# Tech tracking
tech-stack:
  added: [percent-encoding 2.3]
  patterns: [extract encoding logic to testable free function, apply NON_ALPHANUMERIC for strictest URL path segment safety]

key-files:
  created: []
  modified:
    - Cargo.toml
    - src/jmap/mod.rs

key-decisions:
  - "Used free function encode_blob_url_segment() rather than inlining per plan recommendation — enables unit testing without live HTTP"
  - "Applied NON_ALPHANUMERIC (strictest set) to {blobId} and {name}; left {accountId} and {type} unencoded per plan (accountId always safe, type is hardcoded)"
  - "Placed encode_blob_url_segment() at module level (not in impl block) so test module can import it directly via super::"

patterns-established:
  - "URL template substitution: always percent-encode user-supplied segments before substituting into JMAP URL templates"
  - "Testable encoding helpers: extract encoding logic to named free functions for isolated unit testing"

requirements-completed: [SEC-09]

# Metrics
duration: 15min
completed: 2026-04-04
---

# Phase 13 Plan 03: SEC-09 JMAP Blob Download URL Percent-Encoding Summary

**JMAP blob download URLs now percent-encode {blobId} and {name} template segments using NON_ALPHANUMERIC set via the percent-encoding 2.3 crate, closing SEC-09**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-04-04
- **Completed:** 2026-04-04
- **Tasks:** 2
- **Files modified:** 3 (Cargo.toml, Cargo.lock, src/jmap/mod.rs)

## Accomplishments
- Added `percent-encoding = "2.3"` dependency to Cargo.toml
- Implemented `encode_blob_url_segment()` free function applying `NON_ALPHANUMERIC` set
- Updated `download_blob()` to encode `blob_id` and the literal `"attachment"` name before URL template substitution
- Added 4 unit tests in `sec09_tests` module proving encoding correctness for spaces/dots, Unicode, URL-reserved characters, and alphanumeric passthrough

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Failing tests for encode_blob_url_segment** - `e234a74` (test)
2. **Task 1+2 GREEN: Implementation + tests pass** - `f8fa38c` (feat)

_Note: TDD tasks — RED commit added failing tests, GREEN commit added implementation making all tests pass_

## Files Created/Modified
- `Cargo.toml` - Added `percent-encoding = "2.3"` to `[dependencies]`
- `Cargo.lock` - Updated with resolved percent-encoding 2.3.2 dependency
- `src/jmap/mod.rs` - Added `use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode}`, `encode_blob_url_segment()` free function, updated `download_blob()` URL construction, added `sec09_tests` module with 4 tests

## Decisions Made
- Used free function `encode_blob_url_segment()` (not method on JmapClient) so the `sec09_tests` module can import it directly via `super::` — enables isolated unit testing without constructing an authenticated client
- Applied NON_ALPHANUMERIC to `{blobId}` and `{name}` only; left `{accountId}` (always alphanumeric server IDs) and `{type}` (hardcoded MIME type) unencoded per plan scope (D-07)
- The `{name}` value remains hardcoded `"attachment"` — encoding it produces `attachment` (unchanged, all alphanumeric) — but the encoding mechanism is now in place for D-07's future filename support

## Deviations from Plan

None - plan executed exactly as written. The helper function pattern (vs inline encoding) was explicitly recommended by the plan and implemented as specified.

## Issues Encountered

None. Build was clean on first attempt, all 4 tests passed on first GREEN run. Clippy clean throughout.

## URL Template Substitution Audit

Ran `grep -n 'replace("{' src/jmap/mod.rs` — only one location found (`download_blob` lines 888-891). No other URL template substitutions exist in the file that would require encoding.

## Known Stubs

None — `encode_blob_url_segment("attachment")` produces `attachment` (unchanged). When D-07 ships a real filename parameter to `download_blob()`, callers will supply the filename and `encode_blob_url_segment` will encode it correctly without any further changes to the encoding logic.

## Next Phase Readiness
- SEC-09 fully closed
- `encode_blob_url_segment()` ready for D-07 filename support without modification
- 131 tests passing, zero clippy warnings, build clean

---
*Phase: 13-security-hardening*
*Completed: 2026-04-04*
