# Phase 14: MCP Layer Refactor - Context

**Gathered:** 2026-04-04
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

The MCP GraphQL layer uses a shared `AppContext` (no TLS handshake per tool call), confirmation tokens are bound to a per-process HMAC nonce (no forgeable deterministic hash), GraphQL query cost is bounded, SIGTERM/SIGINT are handled gracefully, and no `MutexGuard` is held across an `await` in any resolver.

Requirements: PERF-03 (shared clients), SEC-05 (HMAC nonce), SEC-07 (complexity limits), SEC-08 (markAsSpam guard), STAB-04 (SIGTERM), STAB-07 (MutexGuard across await).

</domain>

<decisions>
## Implementation Decisions

### Shared AppContext Architecture

- **D-01**: Introduce `AppContext` struct in `src/mcp/graphql/mod.rs` holding:
  - `jmap: Option<Arc<tokio::sync::Mutex<JmapClient>>>` (existing pattern preserved)
  - `carddav: Arc<tokio::sync::OnceCell<Arc<CardDavClient>>>` (lazy init on first contacts query)
  - `caldav: Arc<tokio::sync::OnceCell<Arc<CalDavClient>>>` (lazy init on first calendar query)
  - `hmac_key: Arc<[u8; 32]>` (per-process random nonce generated at server start)
- **D-02**: Pass `AppContext` via `schema.data(ctx)` once at server startup; resolvers call `ctx.data::<AppContext>()` instead of constructing clients per call.
- **D-03**: DAV OnceCell initialization uses `get_or_try_init()` with the Fastmail username/app-password read from Config at init time. Failure to init returns GraphQL error; does not panic server.
- **D-04**: Replace the 2 call sites in `src/mcp/graphql/query.rs` at lines 212 and 230 (`CardDavClient::new` / `CalDavClient::new`) with `ctx.get_carddav().await?` / `ctx.get_caldav().await?` helpers on AppContext.

### HMAC Confirmation Tokens

- **D-05**: Add `hmac = "0.12"` and `sha2 = "0.10"` crates. Use `Hmac<Sha256>` for token generation.
- **D-06**: Generate 32-byte HMAC key at server startup via `rand::rngs::OsRng` (`rand_core::OsRng::fill_bytes`). Key lives for process lifetime only — server restart invalidates all tokens (matches SEC-05 success criterion 2).
- **D-07**: Replace `confirmation_token(parts: &[&str]) -> String` in `src/mcp/graphql/types.rs:728` with `AppContext::confirmation_token(&self, parts: &[&str]) -> String` that:
  1. HMACs the concatenation of parts (length-prefixed to prevent ambiguity)
  2. Returns hex-encoded first 16 bytes (128-bit truncation)
- **D-08**: Add `rand = "0.9"` + `rand_core = "0.9"` to Cargo.toml. Prefer existing crates if already present (check first).

### GraphQL Complexity Limits

- **D-09**: Add `async-graphql`'s built-in `depth_limit: 5` and `complexity_limit: 200` via `Schema::build(...).depth_limit(5).complexity(200)` in `build_schema()`.
- **D-10**: Limits chosen based on current schema: deepest legitimate query is email→thread→emails (depth 3), so 5 gives headroom. Complexity 200 accommodates batch queries of up to ~20 items with nested fields.

### markAsSpam Confirmation Gate (SEC-08)

- **D-11**: Add `MarkAsSpamAction` enum mirroring `DeleteContactAction` (PREVIEW, CONFIRM variants). Update the `markAsSpam` mutation signature to accept `action: MarkAsSpamAction` + `confirmation_token: Option<String>`.
- **D-12**: Use same HMAC token generation pattern as deleteContact/deleteCalendar/deleteEvent for uniformity.

### SIGTERM/SIGINT Handling

- **D-13**: In `src/mcp/mod.rs::run_server()`, add `tokio::select!` arm listening on `tokio::signal::unix::signal(SignalKind::terminate())` and `tokio::signal::ctrl_c()`. On signal: drop the rmcp transport gracefully, let in-flight tool calls complete (with short timeout), then exit 0.
- **D-14**: Use `tokio_util::sync::CancellationToken` for propagating shutdown signal to any long-running resolver work.

### MutexGuard-Across-Await Audit (STAB-07)

- **D-15**: Audit all `.lock().await` usages in `src/mcp/graphql/` for patterns where the guard is held across a subsequent `.await`. Refactor to scope the guard: `let result = { let mut guard = client.lock().await; guard.method().await }` → clippy warns `await_holding_lock`. Replace with pattern: take guard, extract needed data or call sync method, drop guard, then await.
- **D-16**: Enable `clippy::await_holding_lock` lint as deny-level for `src/mcp/` module.

### Claude's Discretion

- Exact shutdown timeout duration (5s reasonable default)
- Whether to use `tracing::info!` or `debug!` for startup/shutdown logs
- Test strategy for HMAC tokens (likely inject deterministic key in tests)
- Test strategy for SIGTERM (requires integration test or `#[cfg(test)]` signal injection)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `Arc<tokio::sync::Mutex<JmapClient>>` pattern already in use at `src/mcp/graphql/mod.rs:22, 26`
- `require_jmap_client()` helper in types.rs:11 — template for DAV helpers
- Existing confirmation_token call sites: `src/mcp/graphql/mutation.rs:114, 130` (deleteContact); also deleteCalendar, deleteEvent
- `Output::<()>::error` pattern from Phase 12 for CLI errors (not applicable for GraphQL errors — use `async_graphql::Error`)

### Established Patterns

- `ctx.data::<T>()?` for reading shared state in GraphQL resolvers
- `schema.data(value)` at build time to inject context
- `MarkAsSpamAction`-style PREVIEW/CONFIRM enums for mutation gates

### Integration Points

- `src/mcp/mod.rs::run_server()` — signal handling entry point
- `src/mcp/graphql/mod.rs::build_schema()` — context injection, complexity limits
- `src/mcp/graphql/types.rs::confirmation_token()` — HMAC replacement target
- `src/mcp/graphql/mutation.rs` — all 3 existing confirmation sites + markAsSpam addition
- `src/mcp/graphql/query.rs:212, 230` — DAV client call sites
- `Cargo.toml` — new deps (hmac, sha2, rand, tokio_util)

</code_context>

<specifics>
## Specific Ideas

- Check whether `rand` crate is already in Cargo.toml before adding
- Use length-prefixed encoding for HMAC input (e.g., `parts.len().to_le_bytes()` prefix + each part's len + bytes) to prevent `["a", "bc"]` colliding with `["ab", "c"]`
- Unit test: HMAC tokens differ across 2 server instances (different keys)
- Unit test: markAsSpam PREVIEW returns token, CONFIRM with wrong token is rejected
- Unit test: query with depth 6 returns error, depth 5 succeeds
- Integration test (if feasible): signal delivery triggers clean shutdown

</specifics>

<deferred>
## Deferred Ideas

- Connection pooling beyond one client per protocol (not needed for single-user MCP)
- Token persistence across restarts (explicitly out of scope per D-06)
- Per-user HMAC keys (phase is scoped to per-process)
- Request-level rate limiting (separate concern)

</deferred>

---

*Phase: 14-mcp-layer-refactor*
*Context gathered: 2026-04-04 via smart discuss*
