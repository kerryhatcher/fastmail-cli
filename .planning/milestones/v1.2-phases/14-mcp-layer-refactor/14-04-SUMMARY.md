---
phase: 14-mcp-layer-refactor
plan: "04"
subsystem: mcp
tags: [sigterm, sigint, graceful-shutdown, rmcp, cancellation-token, signal-handling]
dependency_graph:
  requires: []
  provides: [graceful-shutdown-for-mcp-server]
  affects: [src/mcp/mod.rs]
tech_stack:
  added: []
  patterns: [RunningServiceCancellationToken, tokio::select!, cfg-gated unix signal handling]
key_files:
  modified:
    - src/mcp/mod.rs
decisions:
  - No tokio::time::timeout wrapper around waiting() — rmcp cancellation already drains in-flight requests cleanly
  - SIGTERM handler cfg-gated to unix targets; ctrl_c fallback for non-unix cross-platform support
  - Structural unit test via include_str! for compile-time ordering assertion (runtime signal testing requires child process)
  - tracing::debug! used for shutdown logs per CONTEXT.md discretion note
metrics:
  duration: "147s (~2min)"
  completed_date: "2026-04-04T23:41:54Z"
  tasks_completed: 1
  files_modified: 1
---

# Phase 14 Plan 04: SIGTERM/SIGINT Graceful Shutdown Summary

Adds graceful SIGTERM/SIGINT shutdown to `run_server()` in `src/mcp/mod.rs` via rmcp's native `RunningServiceCancellationToken`, with a `tokio::select!` signal handler task and cfg-gated unix/non-unix fallback.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add SIGTERM/SIGINT handler that cancels RunningService token | d2e103e | src/mcp/mod.rs |

## What Was Built

Replaced the bare `server.waiting().await` call in `run_server()` with:

1. **Token extraction**: `let cancel_token = server.cancellation_token()` called before `server.waiting()` (per Pitfall 2 from RESEARCH.md — `waiting()` consumes `server`, so token must be extracted first)

2. **Signal handler task**: A `tokio::spawn`ed async task using `tokio::select!` on two arms:
   - `#[cfg(unix)]`: `sigterm.recv()` from `tokio::signal::unix::signal(SignalKind::terminate())` + `tokio::signal::ctrl_c()`
   - `#[cfg(not(unix))]`: `tokio::signal::ctrl_c()` only (cross-platform fallback)

3. **Cancellation**: On first signal received, `cancel_token.cancel()` is called (consumes the token), which causes `waiting()` to return `QuitReason::Cancelled`

4. **Logging**: `tracing::debug!` messages identify which signal triggered shutdown; post-`waiting()` exit log confirms clean exit

5. **Structural unit test**: `run_server_source_calls_cancellation_token_before_waiting` uses `include_str!("mod.rs")` to assert `.cancellation_token()` byte position precedes `.waiting()` byte position — guards against future Pitfall 2 regressions

## Verification

All acceptance criteria met:
- `grep -n '.cancellation_token()'` matches line 182
- `grep -n 'SignalKind::terminate'` matches line 190
- `grep -n 'tokio::signal::ctrl_c'` matches lines 201, 208
- `grep -n 'cancel_token.cancel()'` matches line 214
- `grep -n 'tokio::select!'` matches line 197
- `grep -n 'SIGTERM received'` matches line 199
- `grep -n 'SIGINT received'` matches line 202
- `cancellation_token()` at line 182 < `waiting()` at line 218 (PASS)
- `cargo build` exits 0
- `cargo test mcp::tests::run_server_source_calls_cancellation_token_before_waiting` PASSES

## Deviations from Plan

None — plan executed exactly as written. The plan provided complete implementation code (from RESEARCH.md Pattern 5) with all edge cases addressed.

Note: `cargo clippy --all-targets` showed errors in `src/mcp/graphql/mod.rs` (referencing `AppContext` not yet defined in that file), but these are from the parallel plan 14-01 executor that is simultaneously modifying that file. These errors are pre-existing relative to this plan's scope (14-04 only touches `src/mcp/mod.rs`) and will be resolved when plan 14-01 completes.

## Known Stubs

None.

## Self-Check: PASSED

- src/mcp/mod.rs: FOUND
- Commit d2e103e: FOUND (confirmed via git log)
- Structural test passes: CONFIRMED
