---
phase: 15-performance
plan: 03
subsystem: mcp-graphql
tags: [performance, arc, graphql, refactor]
requirements: [PERF-10]
dependency_graph:
  requires: []
  provides: [gqlemail-arc-addresses]
  affects: [src/mcp/graphql/types.rs, src/mcp/graphql/query.rs]
tech_stack:
  added: []
  patterns: [Arc<Vec<T>> precomputed fields, &[T] resolver returns]
key_files:
  created: []
  modified:
    - src/mcp/graphql/types.rs
    - src/mcp/graphql/query.rs
    - src/jmap/mod.rs
decisions:
  - "Resolvers return &[GqlEmailAddress] (slice ref into Arc) rather than cloning — async-graphql 7 accepts &[T] for list output types"
  - "Arc<Vec<GqlEmailAddress>> fields stored on GqlEmail struct; precomputed once in GqlEmail::new()"
  - "All GqlEmail(email) construction sites migrated to GqlEmail::new(email) including GqlThread::emails resolver"
metrics:
  duration: "~5min"
  completed: "2026-04-04"
  tasks_completed: 1
  files_modified: 3
---

# Phase 15 Plan 03: GqlEmail Arc-Shared Address Fields Summary

**One-liner:** Precompute GqlEmail address vecs once via Arc<Vec<GqlEmailAddress>>; resolvers return &[T] instead of cloning and converting per call (PERF-10).

## What Was Built

Refactored `GqlEmail` from a tuple struct `GqlEmail(pub Email)` to a named-fields struct that holds five precomputed `Arc<Vec<GqlEmailAddress>>` fields (`from`, `to`, `cc`, `bcc`, `reply_to`). Address conversion (`convert_addrs`) now runs once at construction in `GqlEmail::new()` rather than on every GraphQL resolver invocation. The five address resolvers return `&[GqlEmailAddress]` (a borrowed slice into the Arc), completely eliminating the per-call Vec conversion and allocation.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Precompute GqlEmail address Arcs; resolvers return borrowed slices | 85fe528 | src/mcp/graphql/types.rs, src/mcp/graphql/query.rs, src/jmap/mod.rs |

## Verification Results

- `cargo test` — 151 passed, 0 failed
- `cargo build` — clean
- `cargo clippy --all-features -- -D warnings` — clean
- `rg "convert_addrs\(self\.(inner|0)"` — zero matches (per-resolver conversion removed)
- `rg "Arc<Vec<GqlEmailAddress>>"` — 5 matches (5 address fields confirmed)
- Arc sharing unit test (`gqlemail_addresses_are_arc_shared`) — passes
- Empty-field Arc test (`gqlemail_empty_to_is_arc_shared`) — passes

## Decisions Made

1. **`&[GqlEmailAddress]` resolver return type** — async-graphql 7 accepts `&[T]` for list output types. Resolvers dereference the `Arc<Vec<T>>` to return a borrowed slice with zero allocation cost per resolver call.

2. **Named-fields struct over tuple struct** — `GqlEmail { inner: Email, from: Arc<...>, ... }` replaces `GqlEmail(pub Email)`. Allows co-location of precomputed fields alongside the original email.

3. **All construction sites migrated** — Three `GqlEmail(email)` sites in `query.rs` and one in `GqlThread::emails` in `types.rs` updated to `GqlEmail::new(email)`. Tuple constructor is removed (no `pub` tuple field).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] Suppress forward-declared Arc import in jmap/mod.rs**
- **Found during:** Task 1 (clippy `--all-features` run)
- **Issue:** `src/jmap/mod.rs` had `use std::sync::Arc;` added in phase 15-02 for the PERF-07 mailbox cache Arc migration but PERF-07 call sites haven't been plumbed yet, leaving the import as unused under current feature combinations.
- **Fix:** Added `#[allow(unused_imports)]` with a comment referencing PERF-07 (15-02 plan) to keep the import available for plan 15-02 while suppressing the clippy warning.
- **Files modified:** src/jmap/mod.rs
- **Commit:** 85fe528

## Known Stubs

None — all changes are internal refactors with no user-facing stubs or placeholder values.

## Self-Check: PASSED

- `src/mcp/graphql/types.rs` — verified Arc struct and new() constructor present
- `src/mcp/graphql/query.rs` — verified all GqlEmail::new() call sites
- Commit 85fe528 — verified exists in git log
