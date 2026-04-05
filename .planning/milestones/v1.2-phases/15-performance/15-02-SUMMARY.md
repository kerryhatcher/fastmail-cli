---
phase: 15-performance
plan: 02
subsystem: api
tags: [rust, bytes, arc, serde_json, jmap, memory, performance]

# Dependency graph
requires:
  - phase: 15-01
    provides: concurrent DAV fetch infrastructure, futures dep, CalDAV REPORT
provides:
  - download_blob returning bytes::Bytes (zero-copy blob downloads)
  - parse_response taking owned serde_json::Value (no .clone() on method responses)
  - Arc<Vec<Mailbox>> mailbox cache (O(1) cache hits)
  - Arc<Vec<String>> available_capabilities field
affects: [15-03, 15-04, any plan touching JMAP client or blob downloads]

# Tech tracking
tech-stack:
  added: [bytes = "1" (direct dep for Bytes in public signatures)]
  patterns:
    - "bytes::Bytes returned from HTTP blob downloads instead of Vec<u8>"
    - "Arc<Vec<T>> field pattern for shared read-only caches"
    - "serde_json::Value consumed by value with arr.remove(1) for zero-clone deserialization"
    - "Cow<[u8]> at call sites that need both Bytes and Vec<u8> code paths"

key-files:
  created: []
  modified:
    - src/jmap/mod.rs
    - src/commands/download.rs
    - src/mcp/graphql/types.rs
    - src/mcp/graphql/query.rs
    - Cargo.toml

key-decisions:
  - "bytes::Bytes returned from download_blob; Cow<[u8]> used in download.rs resize path to unify Bytes and Vec<u8> code paths without to_vec()"
  - "parse_response takes Value by value; arr.remove(1) extracts data without clone; parse_email_create_response takes Vec<Value> to enable same pattern for two-response cases"
  - "Arc<Vec<Mailbox>> cache: cache hit returns Arc::clone (O(1)); cache write uses Arc::new(resp.list) + Arc::clone for symmetric storage"
  - "available_capabilities: Arc<Vec<String>>; request() clones (*arc).clone() for serialization boundary since JmapRequest.using is Vec<String>"
  - "list_mailboxes() return type changed to Result<Arc<Vec<Mailbox>>>; callers in query.rs updated to iterate via .iter().map(|m| GqlMailbox::from(m.clone()))"

patterns-established:
  - "Pattern: Arc cache fields — use Arc::clone on read, Arc::new on write; avoid cloning Vec contents"
  - "Pattern: bytes::Bytes at JMAP blob boundary; .as_ref() at &[u8] call sites; Cow at mixed-type paths"

requirements-completed: [PERF-04, PERF-05, PERF-07, PERF-08]

# Metrics
duration: 10min
completed: 2026-04-05
---

# Phase 15 Plan 02: Memory Allocation Reductions in JMAP Layer Summary

**bytes::Bytes for blob downloads, Arc<Vec<T>> for mailbox cache and capabilities, and owned-Value parse_response eliminate the major Vec clone hot paths in the JMAP client**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-04-05T00:13:36Z
- **Completed:** 2026-04-05T00:23:56Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `download_blob` now returns `bytes::Bytes` directly from reqwest, eliminating the `.to_vec()` allocation that doubled memory for every attachment download
- `parse_response` takes `serde_json::Value` by value and uses `arr.remove(1)` to extract the data element without cloning the entire subtree
- `cached_mailboxes` field changed to `Option<Arc<Vec<Mailbox>>>`; `list_mailboxes()` returns `Arc<Vec<Mailbox>>` — repeated cache hits clone only the Arc pointer (O(1)) not the Vec
- `available_capabilities` changed to `Arc<Vec<String>>`; populated once at authenticate time
- Unit test `test_mailbox_cache_returns_arc_clone` verifies Arc::strong_count grows without Vec allocation

## Task Commits

1. **Task 1: download_blob returns bytes::Bytes (PERF-04)** - `24589cd` (feat)
   - Added `bytes = "1"` dep; changed return type; updated download.rs with Cow; updated mcp/graphql/types.rs
2. **Task 2: Arc mailbox cache + Arc capabilities + owned parse_response (PERF-05/07/08)** - `85fe528` + `522bac9`
   - Arc fields, owned parse_response, updated call sites, perf_tests unit test

## Files Created/Modified

- `/home/kwhatcher/projects/fastmail-cli/Cargo.toml` - Added `bytes = "1"` direct dependency
- `/home/kwhatcher/projects/fastmail-cli/src/jmap/mod.rs` - All four changes: Bytes, owned parse_response, Arc fields, unit test
- `/home/kwhatcher/projects/fastmail-cli/src/commands/download.rs` - Call sites updated to use Cow<[u8]> for file-write path
- `/home/kwhatcher/projects/fastmail-cli/src/mcp/graphql/types.rs` - download_blob call site: .as_ref() for slice params
- `/home/kwhatcher/projects/fastmail-cli/src/mcp/graphql/query.rs` - list_mailboxes() call updated for Arc<Vec<Mailbox>>

## Decisions Made

- Used `Cow<[u8]>` in download.rs file-write path: `resize_image` returns `Vec<u8>` on the resize branch, while the non-resize branch can stay as `Bytes`. Avoids introducing `to_vec()` while keeping a unified write type.
- `parse_email_create_response` changed from `&[Value]` to `Vec<Value>` — two `parse_response` calls on the same response vec require ownership; `responses.remove(0)` extracts first element, remainder used for submission response.
- `(*self.available_capabilities).clone()` at the serialization boundary: `JmapRequest.using` is `Vec<String>`, so a clone of the inner Vec is still required for serde. Comment left explaining the constraint.
- `list_mailboxes()` return type is public-facing; callers in query.rs updated from `into_iter().map(GqlMailbox::from)` to `iter().map(|m| GqlMailbox::from(m.clone()))` since Arc does not impl DerefMut or IntoIterator that consumes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Mailbox struct literal in perf_tests with wrong fields**
- **Found during:** Task 2 unit test implementation
- **Issue:** Test used `is_personal`, `is_read_only`, `may_delete` fields that do not exist on `Mailbox` struct — compile would have failed
- **Fix:** Removed those three fields from the struct literal
- **Files modified:** src/jmap/mod.rs
- **Verification:** `cargo test jmap::perf_tests::test_mailbox_cache_returns_arc_clone` passes
- **Committed in:** `522bac9`

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Minor test literal fix; no behavior change. No scope creep.

## Issues Encountered

- A parallel execution had already committed the bulk of Task 2 changes under commit `85fe528` (labeled feat(15-03) due to ordering). The current execution found those changes already present at HEAD and committed only the remaining test fix.

## Next Phase Readiness

- PERF-04, PERF-05, PERF-07, PERF-08 complete
- Ready for Phase 15 Plan 03 (GqlEmail Arc precompute, PERF-10) and Plan 04 (tokio feature narrowing, PERF-11)
- All 157 tests passing, clippy clean

---
*Phase: 15-performance*
*Completed: 2026-04-05*
