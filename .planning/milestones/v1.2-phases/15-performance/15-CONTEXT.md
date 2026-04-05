# Phase 15: Performance - Context

**Gathered:** 2026-04-04
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

Multi-calendar and multi-address-book operations complete concurrently with partial-failure tolerance, single-event lookup no longer downloads full event history, memory allocations in JMAP and MCP layers are reduced through Bytes, Arc, and owned-parse patterns.

Requirements: STAB-06, PERF-01, PERF-02, PERF-04, PERF-05, PERF-06, PERF-07, PERF-08, PERF-09, PERF-10, PERF-11.

</domain>

<decisions>
## Implementation Decisions

### Concurrent DAV Fetches (STAB-06, PERF-01)

- **D-01**: Use `futures::future::join_all` to spawn concurrent per-book fetches. Each future returns `Result<Vec<Item>, Error>` so failures are collected, not propagated.
- **D-02**: Partial-failure handler: after `join_all` returns, iterate results; log each failure via `tracing::warn!(book = %book_id, error = %e, "DAV fetch failed")`; return `Vec<Item>` flattened from successes only. Empty result with all-failures returns `Ok(vec![])` + warnings, not `Err`.
- **D-03**: Apply to `search_contacts()` in `src/carddav/mod.rs` and `list_events()` in `src/caldav/mod.rs`. Both already iterate over discovered books/calendars.

### UID-Targeted CalDAV REPORT (PERF-02)

- **D-04**: Implement `get_event_by_id(uid)` via RFC 4791 CalDAV `calendar-query` REPORT with `VEVENT` + `C:prop-filter name="UID"` `text-match` for the UID value. Sends one REPORT per calendar in parallel via join_all.
- **D-05**: Fallback: if REPORT returns 400/501 or empty results across all calendars, fall back to existing full-fetch pattern. Log fallback via `tracing::warn!`.
- **D-06**: Cyrus IMAP quirk defense: the text-match uses `collation="i;unicode-casemap"` and `match-type="equals"` for exact UID match. No prefix/substring matching.

### Optional kreuzberg Feature (PERF-06)

- **D-07**: Add `extract` feature to Cargo.toml. Move `kreuzberg` to `[dependencies]` with `optional = true`. Wrap `[features]` section: `default = ["extract"]`, `extract = ["dep:kreuzberg"]`. Preserves backward-compat default binary.
- **D-08**: Gate `util::extract_text()` and all call sites with `#[cfg(feature = "extract")]`. Stub that returns "document extraction not enabled in this build" when feature is off. `cargo build --no-default-features` must succeed.

### Memory Allocation Reductions

- **D-09**: **PERF-04**: `download_blob()` returns `bytes::Bytes` (already available transitively via reqwest). Call sites in `src/commands/download.rs` use `&bytes` for write.
- **D-10**: **PERF-05**: `parse_response` in `src/jmap/mod.rs` takes `serde_json::Value` by value; uses `Value::take()` or direct destructuring to avoid `.clone()` on method results.
- **D-11**: **PERF-07**: Mailbox cache in `JmapClient` changes from `Vec<Mailbox>` to `Arc<Vec<Mailbox>>`. Accessors return `Arc<Vec<Mailbox>>` clones (cheap).
- **D-12**: **PERF-08**: `available_capabilities` field in `Session` changes to `Arc<Vec<String>>`. Or: return `&[String]` via accessor instead of cloning.
- **D-13**: **PERF-10**: `GqlEmail` field resolvers (`from`, `to`, `cc`, `bcc`, `reply_to`) take the vector once via resolver context and reuse. Use `OnceLock<Arc<Vec<GqlEmailAddress>>>` cache per GqlEmail instance or convert fields to shared references. Simplest: store addresses as `Arc<Vec<EmailAddress>>` in GqlEmail, clone the Arc per field call.

### Smaller Tweaks

- **D-14**: **PERF-09**: Change `FilterType::Lanczos3` to `FilterType::Triangle` in `src/util.rs` image resize call. One-line change.
- **D-15**: **PERF-11**: Narrow tokio features in Cargo.toml from `full` to `["rt-multi-thread", "macros", "io-util", "sync", "time", "signal", "net"]`. `signal` required by Phase 14; `net` may be needed by reqwest — verify build after narrowing.

### Claude's Discretion

- Exact ordering of PERF-04..PERF-10 work (can be one plan or split)
- Whether Arc migration uses `Arc::clone` explicitly or relies on `.clone()`
- Test strategy for partial-failure (inject failing book via mock)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `futures` crate — verify already in Cargo.toml; if not, add `futures = "0.3"`
- `bytes::Bytes` — available via reqwest transitively, may need direct dep for `bytes::Bytes` type in signatures
- `Arc`, `tokio::sync::Mutex` already used throughout MCP layer
- `tracing::warn!` established pattern

### Established Patterns

- Sequential `for book in books { fetch(book).await? }` — target for migration
- `Client::builder().timeout(Duration::from_secs(30))` from Phase 12
- `#[cfg(feature = "...")]` convention for optional build features

### Integration Points

- `src/carddav/mod.rs` — `search_contacts()`, `list_addressbooks()`
- `src/caldav/mod.rs` — `list_events()`, `get_event_by_id()` (to add), existing REPORT code
- `src/jmap/mod.rs` — `download_blob()`, `parse_response()`, mailbox cache, capabilities
- `src/mcp/graphql/types.rs` — `GqlEmail` resolvers
- `src/util.rs` — image resize
- `Cargo.toml` — feature gates, tokio features, new deps

</code_context>

<specifics>
## Specific Ideas

- Unit test: mock 3 books where middle book returns 500 — verify remaining 2 succeed, warning logged
- Unit test: `cargo build --no-default-features` succeeds (via CI or build.rs check)
- Unit test: `Arc::strong_count` on mailbox cache after 10 `get_mailboxes()` calls == expected count (no cloning)
- Benchmark sketch: old vs new sequential vs concurrent fetch time (if easy)
- Avoid breaking MCP surface — all changes should be internal refactors

</specifics>

<deferred>
## Deferred Ideas

- Benchmarks as CI artifacts (separate phase)
- Streaming blob downloads (out of scope — Bytes is sufficient)
- SQLite result caching (out of scope)
- Rayon parallelization for CPU-bound work (only image resize is CPU-bound here)

</deferred>

---

*Phase: 15-performance*
*Context gathered: 2026-04-04 via smart discuss*
