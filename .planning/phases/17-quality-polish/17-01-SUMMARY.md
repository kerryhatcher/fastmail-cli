---
phase: 17-quality-polish
plan: "01"
subsystem: commands/download
tags: [refactor, safety, let-else, rust-idiomatic]
dependency_graph:
  requires: []
  provides: [stable-attachment-guard]
  affects: [src/commands/download.rs]
tech_stack:
  added: []
  patterns: [let-else guard, idiomatic Option handling]
key_files:
  modified:
    - src/commands/download.rs
decisions:
  - "Use let-else guard at the top of download_attachment() so both for-loop sites see &Vec<Attachment> directly — eliminates all three .unwrap() calls on email.attachments"
metrics:
  duration: "3 minutes"
  completed: "2026-04-05T01:58:01Z"
  tasks_completed: 1
  tasks_total: 1
  files_changed: 1
requirements:
  - STAB-05
---

# Phase 17 Plan 01: let-else guard for download.rs triple-unwrap Summary

**One-liner:** Replaced triple `.unwrap()` pattern on `email.attachments` with a single `let Some(attachments) else` guard, eliminating all panic sites in `download_attachment()`.

## What Was Built

The `download_attachment()` function in `src/commands/download.rs` previously called `.unwrap()` on `email.attachments` three times:

1. Line 31 — `attachments.unwrap().is_empty()` in the early-return check
2. Line 40 — `attachments.unwrap()` in the JSON format for-loop
3. Line 72 — `attachments.unwrap()` in the file download for-loop

All three sites checked the same `Option<Vec<Attachment>>`. The refactor consolidates them into one let-else guard at line 30 that returns early with an error if the field is `None`. A separate `is_empty()` check follows for the empty-slice case. Both for-loops then iterate `attachments` directly as `&Vec<Attachment>` with no unwrap needed.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Replace triple-unwrap with let-else guard | 93c1316 | src/commands/download.rs |

## Verification

- `grep "attachments.unwrap()" src/commands/download.rs` — returns empty (zero matches)
- `grep "let Some(attachments)" src/commands/download.rs` — returns exactly one match (line 30)
- `cargo test --lib commands::download` — 5/5 tests pass
- `cargo clippy -- -D warnings` — exits 0, no errors

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED

- src/commands/download.rs modified: FOUND
- Commit 93c1316 exists: FOUND
- Zero bare .unwrap() on email.attachments: CONFIRMED
- let Some(attachments) guard present at line 30: CONFIRMED
