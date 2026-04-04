---
phase: 12-foundation-safety
plan: 02
subsystem: infra
tags: [rust, reqwest, carddav, caldav, http-timeout, path-traversal, security]

requires:
  - phase: 12-foundation-safety plan 01
    provides: secrecy crate added to config; foundation safety groundwork

provides:
  - CardDavClient::new() with 30-second HTTP timeout (returns Result<Self>)
  - CalDavClient::new() with 30-second HTTP timeout (returns Result<Self>)
  - safe_filename() helper that strips directory components from server-supplied filenames

affects:
  - any future phase that constructs CardDavClient or CalDavClient
  - any future phase that adds attachment download functionality

tech-stack:
  added: []
  patterns:
    - "reqwest::Client::builder().timeout(Duration::from_secs(30)).build() for all DAV client constructors"
    - "Path::file_name() with OsStr fallback for safe server-supplied filename handling"

key-files:
  created: []
  modified:
    - src/carddav/mod.rs
    - src/caldav/mod.rs
    - src/commands/contacts.rs
    - src/commands/calendars.rs
    - src/commands/events.rs
    - src/mcp/graphql/query.rs
    - src/commands/download.rs
    - src/config.rs

key-decisions:
  - "CardDavClient::new() and CalDavClient::new() changed from infallible -> Self to Result<Self> to surface HTTP builder failures"
  - "CalDavClient uses std::time::Duration (fully qualified) to avoid collision with chrono::Duration already in scope"
  - "MCP GraphQL resolvers use .map_err(|e| async_graphql::Error::new(e.to_string())) to bridge crate::error::Error to async_graphql::Error"
  - "safe_filename() is a module-level private fn in download.rs, not a method, for testability"

patterns-established:
  - "DAV constructors return Result<Self> — callers must propagate ? or .expect() in tests"
  - "Attachment filename sanitization via Path::file_name() with 'attachment' fallback"

requirements-completed: [STAB-02, SEC-01]

duration: 25min
completed: 2026-04-04
---

# Phase 12 Plan 02: Foundation Safety — DAV Timeouts and Path Traversal Fix Summary

**30-second HTTP timeouts added to CardDavClient and CalDavClient constructors; server-supplied attachment filenames sanitized via Path::file_name() to prevent path traversal**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-04-04T22:40:00Z
- **Completed:** 2026-04-04T23:05:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- `CardDavClient::new()` now returns `Result<Self>` and builds a reqwest client with a 30s timeout
- `CalDavClient::new()` now returns `Result<Self>` and builds a reqwest client with a 30s timeout
- All 5 call sites across commands and MCP GraphQL resolvers updated to propagate `?`
- `safe_filename()` helper added to `download.rs` strips `../` traversal and absolute paths, falls back to `"attachment"` for empty/bare-directory names
- 7 new unit tests added (2 for DAV constructors, 5 for `safe_filename` edge cases); all 116 tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Add 30s timeout to CardDavClient::new() and CalDavClient::new()** - `a8fde42` (feat)
2. **Task 2: Strip directory components from attachment filenames before write** - `cc7d1da` (feat)

## Files Created/Modified

- `src/carddav/mod.rs` - `CardDavClient::new()` returns `Result<Self>` with 30s timeout; test updated; new test added
- `src/caldav/mod.rs` - `CalDavClient::new()` returns `Result<Self>` with 30s timeout; new test added
- `src/commands/contacts.rs` - `contact_client()` propagates `?` from `CardDavClient::new()`
- `src/commands/calendars.rs` - `calendar_client()` propagates `?` from `CalDavClient::new()`
- `src/commands/events.rs` - `calendar_client()` propagates `?` from `CalDavClient::new()`
- `src/mcp/graphql/query.rs` - Both DAV constructors use `.map_err(|e| async_graphql::Error::new(e.to_string()))?`
- `src/commands/download.rs` - `safe_filename()` helper added; `Path::new(out_dir).join(&final_filename)` replaced with `Path::new(out_dir).join(safe_filename(&final_filename))`
- `src/config.rs` - Added custom `secret_string_serde` module to fix `SecretString` serialization (deviation fix)

## Decisions Made

- Changed DAV constructors from infallible `-> Self` to `-> Result<Self>` so HTTP client builder errors surface to callers rather than panicking
- Used fully-qualified `std::time::Duration::from_secs(30)` in caldav to avoid shadowing `chrono::Duration` already imported
- Used `.map_err(|e| async_graphql::Error::new(e.to_string()))` in MCP resolvers to bridge error types without adding `From` impls
- `safe_filename()` placed as a private module-level function for direct unit testability via `use super::safe_filename`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed SecretString Serialize error in config.rs blocking compilation**
- **Found during:** Task 1 (attempting to build after DAV constructor changes)
- **Issue:** Parallel plan 12-01 added `secrecy::SecretString` fields to `CoreConfig` and `ContactsConfig` with `#[derive(Serialize)]`, but `SecretString = SecretBox<str>` cannot implement `Serialize` because `str: !Sized` — this blocked `cargo build`
- **Fix:** Added custom serde helper functions `serialize_opt_secret_string` / `deserialize_opt_secret_string` and annotated the two `Option<SecretString>` fields with `#[serde(serialize_with = ..., deserialize_with = ...)]`
- **Files modified:** `src/config.rs`
- **Verification:** `cargo build` succeeds, `cargo test` passes (116 tests)
- **Committed in:** `a8fde42` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (Rule 3 — blocking issue from parallel plan)
**Impact on plan:** Fix was required for compilation; no scope creep. The auto-fix is strictly a serialization correctness fix for a type added by another plan.

## Issues Encountered

- The linter reverted several edits mid-execution (contacts.rs, events.rs, query.rs). Each was re-applied and verified with `cargo build` before test execution.

## Known Stubs

None — both tasks fully implement their requirements with no placeholder values.

## Next Phase Readiness

- STAB-02 and SEC-01 requirements satisfied
- DAV clients are now connection-timeout-safe; no hung connections on unresponsive servers
- Attachment downloads are path-traversal-safe against malicious server filenames
- All existing tests continue to pass (116/116)

---
*Phase: 12-foundation-safety*
*Completed: 2026-04-04*
