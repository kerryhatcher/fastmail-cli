# Phase 16: Integration Test Coverage - Context

**Gathered:** 2026-04-04
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

JMAP send/auth/error-path behaviors, CalDAV concurrent fetch with partial-failure tolerance, and CardDAV CRUD flows are verifiable via a wiremock-based integration test suite that runs without a live Fastmail account.

Requirements: TEST-01.

</domain>

<decisions>
## Implementation Decisions

### Test Infrastructure

- **D-01**: Add `wiremock = "0.6"` and `serde_json` to `[dev-dependencies]`. wiremock chosen for async-first ergonomics and stateless matcher API.
- **D-02**: Create `tests/common/mod.rs` exposing:
  - `start_mock_server() -> MockServer` — spawns wiremock server
  - `jmap_session_response(base_url: &str) -> String` — returns realistic JMAP Session JSON with URLs rewritten to `base_url`
  - `load_fixture(name: &str) -> String` — reads from `tests/fixtures/`
  - `test_jmap_client(server: &MockServer) -> JmapClient` — builds client pointed at mock
- **D-03**: One file per scenario in `tests/`:
  - `tests/jmap_auth.rs` — auth session flow, token validation
  - `tests/jmap_send.rs` — email send (draft create, submission)
  - `tests/jmap_errors.rs` — 401/429/500/4xx response handling
  - `tests/caldav_concurrent.rs` — partial-failure tolerance with one failing book
  - `tests/carddav_crud.rs` — create/update/delete round-trip
  - `tests/mcp_graphql.rs` — GraphQL resolver happy paths using AppContext
- **D-04**: Large response payloads in `tests/fixtures/*.json` (e.g., `jmap_session.json`, `caldav_event.xml`). Small inline fixtures OK.

### Making JmapClient/CardDavClient/CalDavClient Testable

- **D-05**: `JmapClient::SESSION_URL` is currently a `const &str`. Add a constructor variant or environment override:
  - `JmapClient::new_with_session_url(token: String, session_url: String) -> Result<Self>`
  - Existing `JmapClient::new()` calls `new_with_session_url(token, SESSION_URL)`
  - Session URL stored in client, used in `authenticate()`
- **D-06**: For DAV clients, allow base URL override via constructor parameter. Current `CardDavClient::new(username, app_password)` hardcodes Fastmail URL — add `new_with_base_url(username, app_password, base_url)` variant.
- **D-07**: Use environment variables `FASTMAIL_SESSION_URL` and `FASTMAIL_DAV_BASE_URL` in tests ONLY (not production). Tests set these before constructing clients.

### Test Coverage Targets

- **D-08**: Required coverage per success criterion:
  1. JMAP auth: session endpoint returns valid JSON → client parses capabilities and account_id
  2. JMAP send: setup session + Email/set → assert request body has correct email structure
  3. JMAP errors: 401 returns Error::Unauthorized, 429 returns Error::RateLimit, 500 returns Error::Server, 4xx catch-all returns Error::Server with HTTP code in message
  4. CalDAV concurrent: 3 mock books, middle returns 500 → assert 2 successes + 1 warn log
  5. CardDAV CRUD: PUT create → GET back → PUT update → DELETE → 404 on re-GET
  6. MCP GraphQL: schema query with AppContext returns valid responses (may need simpler auth-free queries)
- **D-09**: Assert wire format correctness (actual JSON request body) alongside response handling — catches serialization regressions.

### Isolation from src/

- **D-10**: Per success criterion 3, no wiremock usage inside `src/` `#[cfg(test)]` blocks. All wiremock tests live in `tests/`.
- **D-11**: Existing unit tests in `src/` remain — they test pure functions and parsing. Integration tests exercise HTTP layer.

### Claude's Discretion

- Exact JSON fixture content (realistic Fastmail response shapes)
- Whether to use `#[tokio::test(flavor = "multi_thread")]` for parallelism
- How to test MCP GraphQL resolvers without spinning up full rmcp server (likely via direct schema.execute() calls)
- Error type name mapping (need to verify Error variants in src/error.rs)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `Error` enum in `src/error.rs` with `Server`, `Unauthorized`, `RateLimit` variants (from Phase 12)
- `reqwest::Client` with rustls — wiremock serves HTTP, so TLS is stripped for tests
- `JmapClient::new()` returns `Result<Self>` (Phase 12 STAB-09)
- `AppContext` from Phase 14 with test-friendly HMAC key injection pattern

### Established Patterns

- `tests/` directory already used by Cargo for integration tests (standard convention)
- Async tests use `#[tokio::test]`
- JSON request/response pattern throughout codebase

### Integration Points

- `src/jmap/mod.rs:15` — `SESSION_URL` const to abstract
- `src/carddav/mod.rs` — Fastmail base URL (find and abstract)
- `src/caldav/mod.rs` — same
- `Cargo.toml` — `[dev-dependencies]` section for wiremock
- New: `tests/common/mod.rs`, `tests/fixtures/`, 6 `tests/*.rs` scenario files

</code_context>

<specifics>
## Specific Ideas

- Use wiremock's `.and(body_json_schema(...))` or explicit body match for request assertions
- Shared fixture: `tests/fixtures/jmap_session.json` with placeholder `{{BASE_URL}}` substituted at test time
- MCP GraphQL test: build schema with test AppContext + deterministic HMAC key, execute query string, assert JSON output
- Document in tests/common/mod.rs how to add new scenarios

</specifics>

<deferred>
## Deferred Ideas

- Property-based testing (proptest/quickcheck) — out of scope
- Performance benchmarks for integration paths — separate phase
- E2E tests against live Fastmail sandbox — requires account provisioning, deferred
- Fuzz testing for parsers — out of scope

</deferred>

---

*Phase: 16-integration-test-coverage*
*Context gathered: 2026-04-04 via smart discuss*
