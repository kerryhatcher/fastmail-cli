---
phase: 14-mcp-layer-refactor
plan: 03
subsystem: api
tags: [rust, hmac, graphql, mutation, spam, confirmation-token, sec-08, tdd]

requires:
  - phase: 14-mcp-layer-refactor
    plan: 02
    provides: All resolvers using AppContext; HMAC confirmation_token; SpamAction enum

provides:
  - markAsSpam mutation with HMAC confirmation-token gate (SEC-08 complete)
  - Token validated BEFORE JMAP lock acquisition (defense-in-depth)
  - SpamAction enum reused — no GraphQL SDL type churn

affects: [14-04]

tech-stack:
  added: []
  patterns:
    - markAsSpam follows deleteContact/deleteCalendar/deleteEvent confirmation gate pattern
    - Token check before require_jmap() prevents unauthenticated JMAP lock acquisition

key-files:
  created: []
  modified:
    - src/mcp/graphql/mutation.rs (mark_as_spam rewritten; confirmation_token gate added; 3 unit tests added)

key-decisions:
  - "Reused SpamAction enum per RESEARCH.md Pitfall 6 / Open Question 1 — no MarkAsSpamAction introduced"
  - "Token validation placed BEFORE require_jmap() call — malicious CONFIRM cannot force JMAP acquisition"
  - "GqlStatus has no confirmation_token field — PREVIEW surfaces token inline in message body with consistent marker for MCP hosts"

requirements-completed: [SEC-08]

duration: 145s
completed: 2026-04-04
---

# Phase 14 Plan 03: markAsSpam Confirmation Gate Summary

**HMAC confirmation-token gate added to markAsSpam mutation; token validated before JMAP acquisition; SpamAction enum reused; SEC-08 complete**

## Performance

- **Duration:** ~145 seconds
- **Started:** 2026-04-04T23:53:04Z
- **Completed:** 2026-04-04T23:55:29Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments

- Rewrote `mark_as_spam` resolver to follow the `deleteContact` confirmation-gate pattern
- Added `confirmation_token: Option<String>` parameter to the resolver (7th total in mutation.rs)
- Token validation happens **before** `require_jmap()` — unauthenticated CONFIRM is rejected at token check, not at JMAP acquisition (defense-in-depth)
- PREVIEW branch surfaces token inline in message body: `"Confirmation token: {token}"` — MCP hosts (LLMs) can parse this
- CONFIRM without valid token returns `GqlStatus { success: false, error: "Missing or invalid confirmation_token..." }`
- Reused existing `SpamAction` enum from types.rs — no `MarkAsSpamAction` duplicate introduced (zero GraphQL SDL churn)
- 3 unit tests added:
  - `mark_as_spam_confirm_rejects_missing_token` — CONFIRM with no token rejected before JMAP
  - `mark_as_spam_confirm_rejects_wrong_token` — CONFIRM with wrong token rejected
  - `mark_as_spam_different_keys_produce_different_tokens` — per-process nonce binding verified
- All 143 tests pass; clippy clean; cargo build clean

## Task Commits

1. **Task 1: markAsSpam confirmation gate (TDD: RED → GREEN → REFACTOR)** — `7a41b1f`

## Files Created/Modified

- `src/mcp/graphql/mutation.rs` — mark_as_spam rewritten with confirmation gate; 3 unit tests added

## Decisions Made

- Reused `SpamAction` enum (not `MarkAsSpamAction`) per RESEARCH.md Open Question 1 resolution and CONTEXT.md D-11 note — the plan explicitly deprecated the MarkAsSpamAction approach in favor of SpamAction reuse
- Token validated BEFORE `require_jmap()` per plan's "Restructure recommendation" — this also enables tests to run without a JMAP client
- Surfaced token in GqlStatus.message body (not a new field) — per plan action note, GqlStatus changes would ripple to other resolvers

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — all data sources wired correctly.

## Self-Check

- `src/mcp/graphql/mutation.rs` — exists and contains mark_as_spam with confirmation_token parameter
- Commit `7a41b1f` — verified present
- `grep MarkAsSpamAction src/mcp/graphql/` — zero matches (PASS)
- `cargo test -- 143 passed` (PASS)
- `cargo clippy --all-targets -- -D warnings` — clean (PASS)

## Self-Check: PASSED
