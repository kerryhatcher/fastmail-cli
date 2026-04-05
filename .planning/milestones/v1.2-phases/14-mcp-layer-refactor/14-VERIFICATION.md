---
phase: 14-mcp-layer-refactor
verified: 2026-04-04T00:00:00Z
status: passed
score: 14/14 must-haves verified
---

# Phase 14: MCP Layer Refactor Verification Report

**Phase Goal:** The MCP GraphQL layer uses a shared AppContext (no TLS handshake per tool call), confirmation tokens are bound to a per-process HMAC nonce, GraphQL query cost is bounded, SIGTERM/SIGINT are handled gracefully, and no MutexGuard is held across an await.
**Verified:** 2026-04-04
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | AppContext struct exists in `src/mcp/graphql/mod.rs` with jmap/carddav/caldav OnceCells and hmac_key | VERIFIED | File confirmed; struct at lines 23-28 with all four fields |
| 2  | `build_schema()` injects AppContext via `schema.data()` and applies `limit_depth(5).limit_complexity(200)` | VERIFIED | Lines 129-133 in mod.rs; single `.data(ctx)` call with both limits |
| 3  | A 32-byte random HMAC key is generated at server startup via OsRng | VERIFIED | `try_fill_bytes` on OsRng at line 36; correct rand_core 0.9 API |
| 4  | AppContext exposes `get_carddav`/`get_caldav`/`require_jmap`/`confirmation_token` methods | VERIFIED | All four methods present in AppContext impl (lines 53-123) |
| 5  | All resolvers read AppContext via `ctx.data::<AppContext>()` — no JmapContext references remain | VERIFIED | Zero JmapContext matches across all `src/mcp/graphql/` files; 11 AppContext reads in query.rs, 13 in mutation.rs |
| 6  | `query.rs` search_contacts/calendars call `get_carddav`/`get_caldav` instead of `::new()` per call | VERIFIED | Lines 192, 200 in query.rs; zero CardDavClient::new or CalDavClient::new in query.rs |
| 7  | All confirmation_token call sites use `app_ctx.confirmation_token()` (HMAC-backed) | VERIFIED | 7 occurrences in mutation.rs (lines 103, 214, 386, 453, 532, 646, 827); zero `super::types::confirmation_token` calls |
| 8  | The free function `confirmation_token()` in types.rs is removed | VERIFIED | Zero matches for `pub fn confirmation_token` in types.rs; DefaultHasher absent |
| 9  | `clippy::await_holding_lock` is denied at `src/mcp/` module root | VERIFIED | `#![deny(clippy::await_holding_lock)]` at line 7 of `src/mcp/mod.rs` |
| 10 | `require_jmap_client` helpers in types.rs/query.rs/mutation.rs are removed | VERIFIED | Zero matches for `fn require_jmap_client` across all `src/mcp/graphql/` files |
| 11 | `markAsSpam` CONFIRM without a valid token returns "Missing or invalid confirmation_token" | VERIFIED | Lines 831-838 in mutation.rs; two unit tests confirm (both pass in test suite) |
| 12 | `markAsSpam` PREVIEW returns a token derived from email_id via AppContext HMAC | VERIFIED | Lines 827 and 845-862 in mutation.rs; token injected into PREVIEW message |
| 13 | `run_server()` extracts a `RunningServiceCancellationToken` before calling `server.waiting()` | VERIFIED | Lines 184 vs 219 in mcp/mod.rs; structural unit test guards ordering |
| 14 | A spawned signal task listens on SIGTERM and SIGINT and calls `cancel_token.cancel()` on receipt | VERIFIED | Lines 188-217 in mcp/mod.rs; `SignalKind::terminate`, `ctrl_c()`, `tokio::select!`, `cancel_token.cancel()` all present |

**Score:** 14/14 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | hmac, sha2, rand_core, tokio-util direct deps | VERIFIED | hmac="0.12", sha2="0.10", rand_core={version="0.9",features=["os_rng"]}, tokio-util={version="0.7",features=["rt"]} at lines 53-56 |
| `src/mcp/graphql/mod.rs` | AppContext struct + build_schema with limits | VERIFIED | 220 lines; AppContext struct + full impl; build_schema with depth(5)+complexity(200); 7 unit tests |
| `src/mcp/graphql/query.rs` | search_contacts + calendars use shared DAV clients | VERIFIED | get_carddav at line 192, get_caldav at line 200; no per-call construction |
| `src/mcp/graphql/mutation.rs` | HMAC-backed token generation at all preview gates | VERIFIED | 7 `app_ctx.confirmation_token` calls including markAsSpam; 3 markAsSpam unit tests |
| `src/mcp/graphql/types.rs` | free confirmation_token removed; require_jmap_client removed | VERIFIED | Both absent; DefaultHasher absent |
| `src/mcp/mod.rs` | Graceful SIGTERM/SIGINT shutdown + clippy deny | VERIFIED | Full signal handler implementation; `#![deny(clippy::await_holding_lock)]` at line 7 |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/mcp/graphql/mod.rs` | AppContext | `schema.data(app_ctx)` | WIRED | `build_schema` calls `.data(ctx)` at line 130 |
| `build_schema` | SchemaBuilder limits | `limit_depth(5).limit_complexity(200)` | WIRED | Lines 131-132 in mod.rs |
| `query.rs` search_contacts | `AppContext::get_carddav` | `ctx.data::<AppContext>` | WIRED | Line 192 uses `app_ctx.get_carddav().await?` |
| `mutation.rs` preview gates | `AppContext::confirmation_token` | `app_ctx.confirmation_token` | WIRED | 7 call sites confirmed |
| `src/mcp/` | clippy lint | `#![deny(clippy::await_holding_lock)]` | WIRED | Line 7 of `src/mcp/mod.rs` |
| `run_server` | `RunningServiceCancellationToken` | `server.cancellation_token()` | WIRED | Line 184 before `waiting()` at line 219 |
| signal handler task | `cancel_token.cancel()` | `tokio::select!` on SIGTERM + ctrl_c | WIRED | Lines 188-217 in mcp/mod.rs |

---

### Behavioral Spot-Checks

| Behavior | Method | Result | Status |
|----------|--------|--------|--------|
| All 143 tests pass | `cargo test` | 143 passed; 0 failed | PASS |
| clippy clean | `cargo clippy --all-targets -- -D warnings` | Finished with no warnings | PASS |
| markAsSpam CONFIRM without token rejected | test `mark_as_spam_confirm_rejects_missing_token` | ok | PASS |
| markAsSpam CONFIRM with wrong token rejected | test `mark_as_spam_confirm_rejects_wrong_token` | ok | PASS |
| cancellation_token() called before waiting() | test `run_server_source_calls_cancellation_token_before_waiting` | ok (via `cargo test`) | PASS |
| AppContext HMAC tokens differ across instances | test `test_tokens_differ_across_different_key_instances` | ok | PASS |
| No MarkAsSpamAction enum introduced | grep across `src/mcp/graphql/` | zero matches | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| PERF-03 | 14-01, 14-02 | MCP requests reuse shared CardDavClient/CalDavClient via AppContext | SATISFIED | OnceCell lazy-init in AppContext; get_carddav/get_caldav used in query.rs; no per-request DAV construction |
| SEC-05 | 14-01, 14-02 | Confirmation tokens bound to per-process random nonce | SATISFIED | OsRng HMAC key in AppContext; all 7 confirmation gates use app_ctx.confirmation_token |
| SEC-07 | 14-01 | GraphQL schema enforces depth and complexity limits | SATISFIED | limit_depth(5) and limit_complexity(200) applied in build_schema |
| SEC-08 | 14-03 | markAsSpam requires same confirmation-token flow as other destructive mutations | SATISFIED | confirmation_token: Option<String> parameter added; PREVIEW/CONFIRM gate implemented; token validated before JMAP lock |
| STAB-04 | 14-04 | MCP server handles SIGINT/SIGTERM gracefully | SATISFIED | RunningServiceCancellationToken extracted before waiting(); signal handler task cancels on SIGTERM/SIGINT |
| STAB-07 | 14-02 | MCP Mutex guard dropped before awaiting downstream I/O | SATISFIED | clippy::await_holding_lock denied at module root; tokio::sync::Mutex analysis documented inline in query.rs (STAB-07 audit comment) |

No orphaned requirements detected — all six requirement IDs from PLAN frontmatter map to active traceability entries in REQUIREMENTS.md and all are marked Complete there.

---

### Anti-Patterns Found

None identified. Specific scans:

- Zero `JmapContext` references in `src/mcp/` (shim cleanly removed)
- Zero `require_jmap_client` free functions in `src/mcp/graphql/`
- Zero `DefaultHasher` / `super::types::confirmation_token` in mutation.rs
- Zero `CardDavClient::new` / `CalDavClient::new` in query.rs
- Zero `MarkAsSpamAction` enum anywhere in `src/`
- `try_fill_bytes` used (correct rand_core 0.9 API); `fill_bytes` absent

---

### Human Verification Required

#### 1. SIGTERM Clean Exit Under Load

**Test:** Start the MCP server (`cargo run -- mcp`), initiate a slow GraphQL query, then send `kill -TERM <pid>`.
**Expected:** Server flushes the in-flight response and exits with code 0 within ~5 seconds.
**Why human:** Runtime signal behavior requires a live process; automated tests verify structural ordering only.

#### 2. OnceCell DAV Sharing Across Requests

**Test:** Send two consecutive `search_contacts` GraphQL queries via the MCP server; observe that the second does not perform a TLS handshake to the CardDAV server.
**Expected:** Only one TLS connection established per process lifetime (observable via network tracing or RUST_LOG=debug output).
**Why human:** OnceCell initialization correctness is verified structurally (no CardDavClient::new in query.rs) but actual connection reuse requires a running process with a live Fastmail account.

---

### Gaps Summary

No gaps. All 14 observable truths verified, all six requirements satisfied, cargo test passes 143 tests with zero failures, clippy clean.

---

_Verified: 2026-04-04_
_Verifier: Claude (gsd-verifier)_
