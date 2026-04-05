---
phase: 17-quality-polish
plan: "02"
subsystem: carddav
tags: [siphasher, hashing, contact-ids, stability, determinism]

# Dependency graph
requires:
  - phase: 16-contact-crud
    provides: CardDAV contact pipeline with hash_id() fallback ID generation

provides:
  - SipHasher13-based hash_id() with fixed zero seed in src/carddav/mod.rs
  - siphasher = "1" dependency in Cargo.toml
  - Golden-value unit test asserting exact u64 for "John Doe"

affects: [future contact CRUD operations, any phase using fallback contact IDs]

# Tech tracking
tech-stack:
  added: ["siphasher = \"1\""]
  patterns:
    - "Golden-value tests for determinism contracts: assert_eq!(f(x), KNOWN_CONST) records stable output"
    - "Fixed-seed SipHasher13 for stable cross-version contact ID hashing"

key-files:
  created: []
  modified:
    - Cargo.toml
    - src/carddav/mod.rs

key-decisions:
  - "Use SipHasher13::new_with_keys(0, 0) — zero seed is an arbitrary public constant; determinism not secrecy is the goal (D-02, D-03, D-04)"
  - "Golden value 17102779196494968154 recorded as compile-time contract — any deviation signals hasher or seed change"

patterns-established:
  - "Golden-value test pattern: compute once, record as constant, assert forever"

requirements-completed: [STAB-08]

# Metrics
duration: 5min
completed: 2026-04-04
---

# Phase 17 Plan 02: SipHasher13 Stable Contact ID Hasher Summary

**SipHasher13 with fixed zero seed replaces DefaultHasher in hash_id(), pinned by golden-value test asserting exact u64 output 17102779196494968154 for "John Doe"**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-04-04T00:00:00Z
- **Completed:** 2026-04-04T00:05:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments
- Replaced `std::collections::hash_map::DefaultHasher` with `siphasher::sip::SipHasher13::new_with_keys(0, 0)` in `hash_id()` — contact fallback IDs now identical across all Rust versions
- Added `siphasher = "1"` to `[dependencies]` in `Cargo.toml`
- Added 4 tests: golden-value assertion (`17102779196494968154`), determinism, distinct-input collision check, and empty-string stability
- Full test suite passes (156 unit tests + integration tests)

## Task Commits

1. **Task 1: Add siphasher dep and replace DefaultHasher with SipHasher13** - `ea48936` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `Cargo.toml` - Added `siphasher = "1"` in `[dependencies]`
- `Cargo.lock` - Updated lockfile with siphasher resolved dependency
- `src/carddav/mod.rs` - Updated `hash_id()` body; added `hash_id_golden`, `hash_id_deterministic`, `hash_id_distinct_inputs`, `hash_id_empty_string_deterministic` tests

## Decisions Made
- Used `SipHasher13::new_with_keys(0, 0)` — zero keys are arbitrary public constants matching the plan spec (D-03, D-04). Determinism is the requirement, not cryptographic secrecy.
- Recorded golden value `17102779196494968154` as the stable contract. If this changes, a future developer will see an immediate test failure and know to investigate the cause before merging.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- STAB-08 requirement satisfied; hash_id() now produces byte-identical contact fallback IDs across all Rust toolchain versions
- Plan 17-01 (download.rs let-else guards) and 17-03 (remove stale allow attributes) are independent and ready to execute

---
*Phase: 17-quality-polish*
*Completed: 2026-04-04*
