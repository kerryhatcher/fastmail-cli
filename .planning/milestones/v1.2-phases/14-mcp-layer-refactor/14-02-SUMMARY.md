---
phase: 14-mcp-layer-refactor
plan: 02
subsystem: api
tags: [rust, hmac, appccontext, oncecell, graphql, carddav, caldav, mutation, clippy]

requires:
  - phase: 14-mcp-layer-refactor
    plan: 01
    provides: AppContext struct with HMAC tokens, OnceCell DAV clients, JmapContext shim

provides:
  - All MCP GraphQL resolvers using AppContext exclusively (no JmapContext)
  - Shared CardDav/CalDav clients via OnceCell (PERF-03 complete)
  - All 6 confirmation gates using HMAC-backed AppContext::confirmation_token (SEC-05 complete)
  - JmapContext shim fully removed
  - clippy::await_holding_lock denied at src/mcp/ module root (STAB-07)

affects: [14-03, 14-04]

tech-stack:
  added: []
  patterns:
    - All resolvers extract AppContext via ctx.data::<AppContext>() then call require_jmap() or get_carddav()/get_caldav()
    - Confirmation tokens always HMAC-bound to process lifetime via AppContext::confirmation_token
    - delete_contact, delete_calendar, delete_event now accept ctx: &Context<'_> for AppContext access

key-files:
  created: []
  modified:
    - src/mcp/graphql/mod.rs (JmapContext shim removed; build_schema simplified; 2 new isolation tests)
    - src/mcp/graphql/query.rs (require_jmap_client removed; contacts/calendars use shared DAV clients; 9 AppContext calls)
    - src/mcp/graphql/types.rs (require_jmap_client removed; confirmation_token free fn removed; DefaultHasher gone)
    - src/mcp/graphql/mutation.rs (require_jmap_client removed; 6 confirmation_token sites migrated to HMAC; ctx added to 3 resolvers)
    - src/mcp/mod.rs (#![deny(clippy::await_holding_lock)] added)

key-decisions:
  - "delete_contact, delete_calendar, delete_event lacked ctx parameter — added ctx: &Context<'_> as needed for AppContext access (Rule 1 auto-fix)"
  - "Per-instance key isolation tests added in mod.rs (co-located with AppContext) as plan specified"
  - "STAB-07 audit: tokio::sync::MutexGuard is Send and safe to hold across .await — documented inline in query.rs"

patterns-established:
  - "AppContext-first pattern: always get app_ctx before token or client in confirmation gate resolvers"
  - "Shared DAV client pattern: app_ctx.get_carddav()/get_caldav() instead of per-request CardDavClient::new"

requirements-completed: [PERF-03, SEC-05, STAB-07]

duration: 320s
completed: 2026-04-04
---

# Phase 14 Plan 02: MCP Resolver Migration to AppContext Summary

**All MCP GraphQL resolvers migrated from JmapContext shim to AppContext; DAV clients shared via OnceCell; all confirmation gates use HMAC tokens; JmapContext shim eliminated**

## Performance

- **Duration:** 320 seconds (~5 min)
- **Started:** 2026-04-04T23:45:27Z
- **Completed:** 2026-04-04
- **Tasks:** 2 (Task 1: query.rs + types.rs + mod.rs; Task 2: mutation.rs)
- **Files modified:** 5

## Accomplishments

- Removed `require_jmap_client` free function from all 3 files (query.rs, types.rs, mutation.rs) — no more duplication
- All 9 query.rs resolvers now call `ctx.data::<AppContext>()?.require_jmap()?`
- `contacts` and `calendars` resolvers migrated to `app_ctx.get_carddav().await?` / `app_ctx.get_caldav().await?` — eliminates TLS handshake per call (PERF-03 complete)
- `GqlAttachment::content` in types.rs migrated from JmapContext to AppContext
- `JmapContext` shim struct and its `.data(JmapContext{...})` injection in `build_schema` fully removed
- `confirmation_token()` free function (DefaultHasher-based) removed from types.rs
- 6 `super::types::confirmation_token()` call sites in mutation.rs replaced with `app_ctx.confirmation_token()` (HMAC-SHA256, SEC-05 complete)
- `#![deny(clippy::await_holding_lock)]` added to `src/mcp/mod.rs` (STAB-07)
- Per-instance HMAC key isolation tests: `test_tokens_differ_across_different_key_instances` and `test_token_stable_for_same_key` added to mod.rs
- All 140 tests pass; `cargo clippy --all-targets -- -D warnings` clean

## Task Commits

1. **Tasks 1+2: full AppContext migration** — `57ecfdd` (feat)

## Files Created/Modified

- `src/mcp/graphql/mod.rs` — JmapContext shim removed; build_schema simplified; 2 isolation tests added
- `src/mcp/graphql/query.rs` — require_jmap_client removed; 9 AppContext require_jmap calls; contacts/calendars use shared OnceCell DAV clients
- `src/mcp/graphql/types.rs` — require_jmap_client removed; confirmation_token free fn (DefaultHasher) removed; GqlAttachment::content uses AppContext
- `src/mcp/graphql/mutation.rs` — require_jmap_client removed; 6 confirmation_token sites → HMAC; ctx added to 3 previously ctx-less resolvers
- `src/mcp/mod.rs` — `#![deny(clippy::await_holding_lock)]` added

## Decisions Made

- Added `ctx: &Context<'_>` parameter to `delete_contact`, `delete_calendar`, `delete_event` — these resolvers originally had no ctx because they only called free functions; now they need AppContext for HMAC tokens
- Co-located isolation tests in `src/mcp/graphql/mod.rs` with AppContext rather than mutation.rs — cleaner separation and mirrors where AppContext is defined
- `reply_to_email` and `forward_email` keep their original structure of acquiring the JMAP client before the preview gate (needed to fetch original email for preview formatting); `app_ctx` is acquired first so both token and client come from same context

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Added ctx parameter to three confirmation-gate resolvers**
- **Found during:** Task 2 (cargo build error)
- **Issue:** `delete_contact`, `delete_calendar`, `delete_event` in mutation.rs had no `ctx: &Context<'_>` parameter — they used the free `confirmation_token()` function which didn't need ctx. Migrating to `app_ctx.confirmation_token()` requires ctx to call `ctx.data::<AppContext>()`.
- **Fix:** Added `ctx: &Context<'_>` as second argument to all three resolver functions (after `&self`)
- **Files modified:** src/mcp/graphql/mutation.rs
- **Commit:** 57ecfdd

---

**Total deviations:** 1 auto-fixed (Rule 1 - missing ctx parameter blocked compilation)

## Known Stubs

None — all data sources wired correctly.

## Self-Check

- `src/mcp/graphql/mod.rs` — exists, JmapContext struct absent, build_schema contains single .data(ctx)
- `src/mcp/graphql/query.rs` — exists, contacts/calendars use get_carddav/get_caldav
- `src/mcp/graphql/types.rs` — exists, confirmation_token free fn absent
- `src/mcp/graphql/mutation.rs` — exists, 6 app_ctx.confirmation_token calls
- `src/mcp/mod.rs` — exists, deny(clippy::await_holding_lock) present
- Commit `57ecfdd` — verified present

## Self-Check: PASSED
