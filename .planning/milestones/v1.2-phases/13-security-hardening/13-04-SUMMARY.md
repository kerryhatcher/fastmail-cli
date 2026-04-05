---
phase: 13-security-hardening
plan: "04"
subsystem: auth/cli
tags: [security, breaking-change, auth, cli]
dependency_graph:
  requires: []
  provides: [SEC-04]
  affects: [src/main.rs, src/commands/auth.rs, README.md]
tech_stack:
  added: []
  patterns: [env-var-first token resolution, stdin fallback with is_terminal guard]
key_files:
  created: []
  modified:
    - src/main.rs
    - src/commands/auth.rs
    - README.md
    - src/caldav/mod.rs
decisions:
  - "Used std::io::stdin().is_terminal() (stable since Rust 1.70) instead of rpassword — per plan guidance, terminal hiding via `read -rs` shell pattern is sufficient"
  - "resolve_token() returns anyhow::Result<String> with Output::error+bail pattern consistent with existing command handlers"
metrics:
  duration: "212s"
  completed: "2026-04-04"
  tasks_completed: 2
  files_modified: 4
---

# Phase 13 Plan 04: Remove Positional Auth Token Arg (SEC-04) Summary

**One-liner:** Removed `fastmail-cli auth YOUR_TOKEN` positional argument; token now read from `FASTMAIL_API_TOKEN` env var with interactive stdin fallback to prevent token exposure in `ps` output and shell history.

## What Was Built

SEC-04 closes the token-on-command-line vulnerability. The `auth` CLI subcommand no longer accepts `token` as a positional argument. Token resolution now follows a two-step priority:

1. `FASTMAIL_API_TOKEN` environment variable (trimmed, rejects empty string)
2. Interactive stdin prompt (only when stdin is a terminal; non-interactive contexts fail fast with a JSON error)

Both the CLI command struct and the dispatch arm in `main.rs` were updated. The README authentication section was rewritten with the new patterns and a v1.2 migration note.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Remove positional token arg; read from env var or stdin | 996ce45 | src/main.rs, src/commands/auth.rs, src/caldav/mod.rs |
| 2 | README auth migration documentation | 17dbd5a | README.md |

## Verification

- `cargo build` exits 0
- `cargo clippy --all-targets -- -D warnings` exits 0 (clean)
- `cargo test` passes: 131/131 tests pass
- Smoke test (not run live per MEMORY constraints):
  - `FASTMAIL_API_TOKEN=foo cargo run -- auth` — would attempt auth with "foo"
  - `echo "" | cargo run -- auth` — would print JSON error about non-interactive context missing env var

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed pre-existing collapsible_if in caldav/mod.rs**
- **Found during:** Task 1 clippy run
- **Issue:** `src/caldav/mod.rs:1437` had nested `if let Some(...) { if ... { } }` pattern that clippy rejected as `-D warnings` violation, blocking the acceptance criterion
- **Fix:** Collapsed to `if let Some(until) = ... && is_valid_rrule_until(until) { ... }` using Rust let-chain syntax
- **Files modified:** src/caldav/mod.rs
- **Commit:** 996ce45

## Known Stubs

None.

## Self-Check: PASSED
