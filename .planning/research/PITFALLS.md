# Pitfalls Research

**Domain:** Hardening an existing async Rust CLI + MCP server (fastmail-cli v1.2)
**Researched:** 2026-04-04
**Confidence:** HIGH — grounded in the actual CODEBASE-REVIEW.md findings and live-validated v1.1 codebase

---

## Critical Pitfalls

### Pitfall 1: Timeout Addition Changes Wire Behavior for Live Fastmail Users

**What goes wrong:**
Adding a 30-second timeout to CardDAV/CalDAV clients (finding #2) is correct, but the value and placement matter. If timeout is set too aggressively (e.g., 10s), event REPORT queries that retrieve large calendar datasets across many events will start failing in prod for users with full calendars. If set only at the connection level rather than the full-request level, slow-reading Fastmail responses still hang. If both a connection timeout and a request timeout are applied, the shorter one silently wins and surprises developers expecting the longer guard to apply.

**Why it happens:**
`reqwest::ClientBuilder` has three distinct timeout settings: `connect_timeout`, `timeout` (total request), and `read_timeout`. Developers often only add `timeout()` matching the JMAP 30s pattern without realizing CalDAV REPORT payloads for large calendars can legitimately take longer to stream than JMAP JSON responses. The regression is invisible in unit tests and only surfaces against a live Fastmail account with years of calendar data.

**How to avoid:**
Match the existing JMAP pattern exactly: `Client::builder().timeout(Duration::from_secs(30)).build()`. Use the same value across all three DAV clients (CardDAV, CalDAV) for consistency. Do not add `connect_timeout` separately unless you have evidence the value differs from request timeout in this codebase. Add a comment citing the JMAP client as the source of the 30s constant so future editors keep them in sync.

**Warning signs:**
- Users with large calendar histories report `"operation timed out"` errors on `events list` after the fix
- CI tests pass but live smoke test on a full account fails
- The CalDAV REPORT for `list_events` with no calendar filter (fetches all calendars) is the highest-risk path

**Phase to address:**
Security/Stability hardening phase (P1 fixes). The exact constant should be shared or at minimum co-located with the JMAP timeout to prevent drift.

---

### Pitfall 2: HTTP 4xx Catch-All Breaks Fastmail-Specific Flows That Rely on Non-401 Status Codes

**What goes wrong:**
Adding a catch-all `400..500` handler (finding #1) can intercept status codes that the existing code handles downstream via JSON parsing or specific match arms. Two concrete risks in this codebase: (1) a `304 Not Modified` is technically 3xx but could be caught if the range is off-by-one; (2) if Fastmail ever returns `410 Gone` for a deleted resource and the code tries to parse it as a successful response, the new catch-all correctly surfaces an error — but callers written before the fix may expect the old `Err(serde deserialization)` error variant and have match arms that no longer trigger. The regression is silent: the error is now a different `Error::Server(...)` variant instead of `Error::Http(reqwest::Error)`.

**Why it happens:**
When callers already existed before the fix was applied, their error-handling match arms were written against the old error shape. Adding a new error path introduces a new variant that may not be caught by exhaustive matches on the caller side. Rust's type system only helps if the error type is an enum — and `anyhow::Error` used in command handlers provides no exhaustiveness check.

**How to avoid:**
Audit all call sites in `src/commands/` and `src/mcp/graphql/` for `match` on `Error` after adding the 4xx catch-all. Search for any code that tests for `reqwest::Error` or deserialization errors as a proxy for "server rejected request." Run the full test suite and add a wiremock test that sends a 403 and 400 and verifies the `Output::error()` JSON is well-formed.

**Warning signs:**
- `cargo test` passes but a wiremock test for 403 panics or produces empty output
- Command handlers that have `_ =>` catch-alls are OK; those with specific `Error::Http` arms are at risk
- MCP mutations that previously returned `{"success": false, "error": "..."}` now return nothing on stdout (broken `process::exit` path, finding #11)

**Phase to address:**
Security/Stability hardening phase. Fix #11 (process::exit JSON contract) BEFORE or alongside fix #1 to avoid the double regression of "wrong error variant" + "no JSON output."

---

### Pitfall 3: Secret Redaction Debug Impl Loses All Diagnostic Utility

**What goes wrong:**
Implementing `Debug` on `Config` / `CoreConfig` / `ContactsConfig` (finding #15) by redacting only `api_token` and `app_password` fields is correct, but the common over-correction is to implement `Debug` for the entire `Config` as `write!(f, "[REDACTED]")`. This redacts `username` and config path information too, making it impossible to diagnose "wrong account" bugs from a debug log without revealing secrets. The opposite failure — manually listing every field — means new fields added later silently appear unredacted in logs.

**Why it happens:**
Developers writing a custom `Debug` impl choose the path of least resistance: either fully opaque or copy-paste from `derive(Debug)` output. The `secrecy` crate approach (wrapping the sensitive field in `Secret<String>`) is more composable but adds a new dependency. The manual approach requires updating the impl whenever `Config` gains a new field.

**How to avoid:**
Use a targeted manual `Debug` implementation that prints non-sensitive fields normally and replaces sensitive fields with a fixed string. For `CoreConfig`, print `CoreConfig { api_token: "[REDACTED]" }` if `Some`, `CoreConfig { api_token: None }` if not set — callers can still see whether auth is configured without leaking the value. For `ContactsConfig`, print `username` in plain text (not sensitive) and redact only `app_password`. Do not add `secrecy` as a new dependency unless the project already uses it, as it adds zeroize semantics that require the wrapped type to implement `Zeroize`, constraining future config field types.

**Warning signs:**
- Debug output from a `tracing::debug!("{:?}", config)` call in a test log shows `[REDACTED]` for the username field — useful information lost
- New config fields (e.g., a future `calendars.username` field) appear unredacted after being added without updating the manual `Debug` impl
- `cargo clippy` does not warn on incomplete manual `Debug` impls

**Phase to address:**
Security hardening phase. This is a pure security fix with no wire behavior change; lower regression risk than timeout or 4xx changes.

---

### Pitfall 4: Confirmation Token Migration Breaks Existing MCP Hosts Mid-Milestone

**What goes wrong:**
Changing the `confirmation_token` function (finding #14) from a pure hash to a nonce-bound token changes the token format. Any MCP host (Claude Desktop, a script, an agent) that completed a PREVIEW step to obtain a token, then upgrades or reconnects to the new server version before submitting the CONFIRM step, will submit a token with the old format. The CONFIRM step will reject it with a cryptic "invalid token" error. The two-step PREVIEW/CONFIRM flow is stateful across two separate MCP tool calls, and the host has no way to know the token format changed.

**Why it happens:**
Stateless token validation (current design) looks safe in isolation but creates a migration window where old-format and new-format tokens coexist. Adding a per-session nonce means tokens issued by one server process are invalid on a restarted process — any agent mid-workflow during the MCP server restart will fail. The current implementation uses `DefaultHasher` (finding #27 in carddav, similar pattern in types.rs), which adds a second failure mode: hash values can change between Rust versions even without a code change.

**How to avoid:**
If the token format changes, bump the token format version prefix (e.g., `v2:...`) and add a graceful rejection message: "Token format has changed. Re-run the preview step to obtain a new token." Avoid a per-session nonce unless the architecture stores issued tokens server-side — a stateless per-process nonce still breaks across restarts and provides minimal security benefit over a randomly salted hash with a stable algorithm. For the `DefaultHasher` replacement, switch to a stable algorithm like SipHash from the `siphasher` crate or a simple keyed HMAC using `hmac` + `sha2` with a process-local secret. Do not use `sha2` alone (no key = still forgeable by parameter enumeration).

**Warning signs:**
- Integration tests that store a PREVIEW-phase token in a variable and submit it in a separate CONFIRM call fail after the nonce fix
- The MCP server is restarted between preview and confirm in a long-running agent session
- `DefaultHasher` is present anywhere token values are computed — both `types.rs:728-733` and `carddav/mod.rs:781-786` need auditing

**Phase to address:**
Security hardening phase. Document the token format change in the phase notes and verify the wiremock-based integration tests cover the full PREVIEW→CONFIRM flow.

---

### Pitfall 5: Concurrent DAV Fetch Aggregates Errors Incorrectly, Masking Partial Failures

**What goes wrong:**
Converting sequential `for calendar in calendars` loops to `join_all` / `JoinSet` (finding #4) changes error semantics. The current sequential code returns `Err` on the first failure and stops. With `join_all`, if one of five calendar fetches fails (e.g., a 503 from Fastmail on a specific calendar), the other four results are discarded and the entire `list_events()` returns an error. Users see "failed to list events" when in reality four out of five calendars returned successfully. With `JoinSet`, dropped set aborts all in-flight tasks, so a panic in one task cancels all others.

**Why it happens:**
`futures::future::join_all` collects all `Result<T, E>` into a `Vec<Result<T, E>>` — callers must explicitly decide whether to return all errors, the first error, or ignore errors from individual sub-fetches. The natural idiom `join_all(futures).await.into_iter().collect::<Result<Vec<_>, _>>()?` silently applies "fail on first error" semantics, which is strictly worse than the sequential loop it replaced for partial-failure tolerance.

**How to avoid:**
Use the "partial success" pattern: `join_all` returning `Vec<Result<T, E>>`, then partition into successes and failures, log failures with `tracing::debug!`, and return only the successes. For `list_events()` and `search_contacts()`, a failure on one calendar/address-book should not abort the entire result — this is explicitly called out in finding #19 ("Add per-address-book error tolerance"). Add a connection semaphore (e.g., `tokio::sync::Semaphore` with limit 5) to avoid opening unbounded concurrent connections to Fastmail, which may trigger rate limiting or connection limits.

**Warning signs:**
- A user with a read-only shared calendar (that returns 403) suddenly gets "events list failed" instead of seeing their writable calendars
- `cargo test` mocks all calendars as successful; only a live test with one bad calendar triggers the regression
- `join_all` used without explicit error partitioning anywhere in the concurrent-fetch implementation

**Phase to address:**
Performance/concurrency phase. The error-aggregation behavior must be specified in the acceptance criteria, not left as an implementation detail.

---

### Pitfall 6: Connection Pool Exhaustion When DAV Clients Are Reconstructed Per MCP Request

**What goes wrong:**
Each `CardDavClient::new()` / `CalDavClient::new()` call creates a new `reqwest::Client` with a fresh connection pool (finding #6). When the MCP server is used by an AI agent running 10-20 tool calls in quick succession, this creates 10-20 separate TLS sessions to Fastmail's servers. Since reqwest's `Client` is designed to be reused (its pool is internal to the instance), each abandoned `Client` drops its connections on `Drop`, but the TLS teardown still consumes server resources. Under sustained load, Fastmail's server may start returning 429 rate-limit or connection refused responses.

**Why it happens:**
The `JmapClient` is correctly wrapped in `Arc<Mutex<>>` and stored in the GraphQL schema's context data (see `graphql/mod.rs:22-30`). But `CardDavClient` and `CalDavClient` are instantiated inside each resolver function (`query.rs:199-214`), outside the context. The pattern exists for JMAP but was not replicated for DAV clients when they were added in v1.0/v1.1.

**How to avoid:**
Add `carddav_client: Option<Arc<Mutex<CardDavClient>>>` and `caldav_client: Option<Arc<Mutex<CalDavClient>>>` to `JmapContext` in `graphql/mod.rs`. Initialize them in `FastmailMcp::new()` alongside the JMAP client. At minimum, extract the underlying `reqwest::Client` from CardDAV/CalDAV constructors into a single shared instance — this ensures all three DAV protocols reuse the same connection pool. Do not wrap the lock around individual HTTP calls; hold the lock only for the mutation of cached state (ETags, addressbook list), not for the duration of network I/O.

**Warning signs:**
- `cargo clippy` will not catch this — it is an architectural problem, not a lint
- Integration tests with wiremock pass (each test creates its own clients) but live AI-agent sessions stall
- `RUST_LOG=debug` trace output shows "new TLS handshake" on every MCP tool call

**Phase to address:**
Performance/DAV client reuse phase. Pair with finding #6. If the Mutex-across-await concern (finding #26) is addressed in the same phase, restructure both together to avoid introducing a new Mutex-held-across-await regression while fixing client reuse.

---

### Pitfall 7: tokio::Mutex Held Across Async I/O Serializes All MCP Requests

**What goes wrong:**
Finding #26 documents that the JMAP client mutex is held across async I/O, serializing all MCP requests. If the DAV client reuse fix (finding #6) naively copies the `Arc<Mutex<JmapClient>>` pattern to `Arc<Mutex<CardDavClient>>` and `Arc<Mutex<CalDavClient>>`, the serialization problem is replicated for three clients instead of one. A `contacts list` call that fetches all address books will block `events list` for the duration of all REPORT requests.

**Why it happens:**
`tokio::sync::Mutex` is correct for protecting shared mutable state, but the lock scope in the current resolvers (`client.lock().await` at the top of `async fn emails(...)`) holds the lock for the entire resolver body, including all `await` points within it. This is the documented anti-pattern from the Tokio team: the async mutex should guard only the state mutation, not the I/O operation.

**How to avoid:**
Refactor the client lock pattern: extract needed config (username, password, base URL) under the lock, clone them (cheap), drop the lock, then execute the HTTP operation without holding the lock. For CalDAV/CardDAV, the clients are largely stateless HTTP wrappers — their constructors only need config, not mutable state. Consider restructuring `CardDavClient` and `CalDavClient` to hold an immutable config struct and a shared `reqwest::Client` rather than holding mutable state that requires a mutex at all.

**Warning signs:**
- Two concurrent MCP tool calls (e.g., an agent querying emails and events at the same time) run sequentially instead of in parallel — observable with `RUST_LOG=debug` timing
- `tokio::sync::Mutex::lock()` future is pending for the duration of a REPORT query (seconds), visible in a tokio-console trace
- Resolvers that lock the client at the top of their body and do not release it until the last `await` point

**Phase to address:**
Performance/stability phase. Must be addressed when implementing client reuse (finding #6) to avoid introducing the new serialization problem while fixing pool exhaustion.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| `DefaultHasher` for fallback contact ID generation | No extra dependency | Hash changes across Rust versions → stored IDs become invalid in future binary update | Never for IDs that may be stored or compared across binary versions |
| `process::exit(1)` instead of `Output::error().print()` | Simple, familiar | Breaks structured JSON contract — MCP hosts and scripts get nothing on stdout | Never in this codebase — the JSON output contract is the public API |
| Recreating `reqwest::Client` per request | Simple, no shared state | TLS session overhead, connection exhaustion under load, pool disabled | Only acceptable in CLI commands that run once per process lifetime |
| Holding `tokio::sync::Mutex` across all await points | Simple single-lock pattern | Serializes all concurrent MCP requests; high lock contention | Acceptable only in MVP when concurrency is not yet a requirement — v1.1 has shipped, this is now tech debt |
| Stringly-typed IDs (`String` everywhere) | No registration boilerplate | Cross-type ID confusion, no compile-time guarantee that an email_id is not passed where a mailbox_id is expected | Acceptable for prototype; not for a hardened API surface |
| `kreuzberg` always compiled in | No feature-flag complexity | 10-20MB PDFium bloat in every binary, even for users who never use `download --format json` | Acceptable during initial development; not for distribution |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Fastmail CardDAV `PUT` with `If-None-Match` | Sending `If-None-Match: *` to create-or-fail returns 412 always from Fastmail (known Cyrus IMAP behavior) | Use `If-None-Match: *` only when no ETag exists; for updates always use the stored ETag in `If-Match` |
| Fastmail CalDAV REPORT range | Omitting `<C:time-range>` from the REPORT body fetches the entire event history — unbounded response | Always include a time range in the REPORT body; the default "future events from today" path already does this |
| CardDAV `DELETE` with stale ETag | Fastmail returns 412 with no new ETag in the response on delete-with-stale-ETag (unlike PUT) | The existing "retry delete without If-Match on 412" decision (PROJECT.md) is correct; do not remove this workaround during hardening |
| JMAP 400 Bad Request | Previously fell through to JSON deserialization error, hiding the actual server message | After adding the 4xx catch-all, verify 400 responses include the Fastmail error body in the Error::Server message for debugging |
| MCP stdio transport and stdout pollution | Any `println!()` or `print!()` in the MCP server path corrupts the JSON-RPC stream; only stderr logging is safe | Use `tracing` with stderr output; never add debug `println!()` inside `src/mcp/` |
| Fastmail 429 Rate Limiting | Rate-limited responses from concurrent DAV fetches are currently not retried | Add a brief exponential-backoff retry (1-2 attempts) specifically for 429 responses in the DAV clients; do not apply globally |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| `join_all` without concurrency limit on DAV fetch | Fastmail returns 429; TLS handshake storm | Use `tokio::sync::Semaphore` to cap concurrent connections to 5 | Once a user has more than ~5 calendars/address books |
| `serde_json::from_value(data.clone())` (finding #13) | High memory usage on email list queries with large bodies | Accept owned `Value` in `parse_response`, avoid deep-clone | At ~50 emails with 10KB bodies each, ~500KB wasted per request |
| `resp.bytes().await?.to_vec()` (finding #12) | Double memory allocation for large attachments | Return `bytes::Bytes` directly | For attachments > 1MB |
| `get_event_by_id` fetches all events from all calendars (finding #5) | Multi-second lookup for a single event; times out on large calendars | Use targeted CalDAV REPORT with UID filter | Users with > ~100 events across multiple calendars |
| Lanczos3 resize filter for MCP images (finding #22) | Noticeably slow image processing in MCP context responses | Switch to `Triangle` or `CatmullRom` — visually indistinguishable at thumbnail sizes | Every image attachment processed through the MCP path |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| vCard EMAIL/TEL values not escaped for `\r\n` (finding #8) | Attacker-controlled contact data injects arbitrary vCard properties that Fastmail stores and serves to other clients | Validate and strip `\r`, `\n`, `;`, `:` from parameter values; apply `escape_value()` consistently to ALL fields including email address and phone number |
| iCalendar attendee role/partstat not validated (finding #9) | Injection of arbitrary iCal properties through `role` or `partstat` fields in create/update event API; Fastmail stores and distributes the corrupted iCal | Enforce allowlists for enum-like fields; escape freeform string fields |
| Auth token as positional CLI argument (finding #10) | Token visible in `ps aux`, shell history, `/proc/self/cmdline` on any multi-user system | Accept token via stdin (`--token -`), env var (already supported), or interactive prompt with `rpassword` crate |
| Deterministic confirmation token without nonce (finding #14) | Any caller who knows the mutation parameters can compute the token and skip the PREVIEW step entirely | Add a per-process random nonce (generated once at startup, not per-call) mixed into the hash; document that tokens are process-scoped |
| `Debug` on `Config` structs (finding #15) | `api_token` and `app_password` appear in plaintext in tracing output, CI logs, crash dumps | Custom `Debug` impl that redacts sensitive fields; never use `derive(Debug)` on types that hold credentials |
| Blob download URL template not URL-encoded (finding #30) | A filename or account ID containing `&`, `+`, or `%` in the JMAP download URL template produces a malformed URL | Use `percent-encoding` crate or `urlencoding` for template substitution in `src/jmap/mod.rs:858-863` |
| No GraphQL depth/complexity limits (finding #24) | A deeply nested GraphQL query (e.g., email → attachments → content recursively) can exhaust server memory or trigger unbounded JMAP requests | Set `max_depth` and `max_complexity` on the `async_graphql::Schema` builder |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| `Output::error()` contract broken by `process::exit(1)` (finding #11) | MCP hosts and scripts parsing stdout JSON receive empty output on confirmation-guard failures; agents cannot distinguish "no confirmation given" from a crash | Replace all `eprintln!() + process::exit(1)` with `Output::<()>::error("...").print()` and normal return; let main() exit with status 1 |
| Auth token visible in shell history | Users who run `fastmail-cli auth <token>` have the token in `~/.zsh_history` and `~/.bash_history` permanently | Add `--token-stdin` flag or interactive prompt mode; update the auth command documentation |
| Config corruption error message lacks recovery guidance (finding #33) | Users see "Failed to parse config" with no next step; they may delete the config file unnecessarily | Include the config file path in the error and suggest `fastmail-cli auth` to reinitialize |
| `download` command crashes (panics) rather than erroring on attachment-less emails (finding #18) | The process terminates without a JSON error; scripts receive non-zero exit with no stdout | Replace three `unwrap()` calls with `let Some(...) else { Output::error("...").print(); return Ok(()); }` |

---

## "Looks Done But Isn't" Checklist

- [ ] **4xx handling:** Adding the catch-all match arm does not break existing callers — verify all `src/commands/` error match arms still compile and correctly map `Error::Server` to user-facing messages
- [ ] **Timeout fix:** Both `src/carddav/mod.rs` and `src/caldav/mod.rs` clients have the timeout set — check both constructors, not just one
- [ ] **vCard injection fix:** `escape_value()` applied to EMAIL address value AND EMAIL type parameter AND TEL number AND TEL type parameter — all four, not just the label
- [ ] **process::exit replacement:** All five call sites in `src/main.rs` replaced, not just the most obvious one — confirm by grepping for `process::exit` in main.rs after the fix
- [ ] **Concurrent fetch:** Error partitioning logic correctly handles the partial-failure case — the "some calendars failed" path returns partial results, not Err
- [ ] **DAV client reuse:** The new shared client in GraphQL context is initialized before the first query, not lazily on first use (lazy init requires a Mutex acquisition that could race)
- [ ] **Debug redaction:** New fields added to `Config` structs in the future do not silently appear unredacted — add a note in the struct definition or a compile-time reminder
- [ ] **kreuzberg feature flag:** The `download` CLI command with `--format json` still works when the feature is enabled; the default binary without the feature gives a clear error message explaining it needs to be compiled with `--features document-extraction`
- [ ] **Newtype IDs in GraphQL:** Each newtype that appears in the GraphQL schema has been registered with `scalar!()` or `#[derive(NewType)]` — missing registration causes a schema build panic, not a compile error
- [ ] **Signal handling:** `tokio::signal::ctrl_c()` is wired into the MCP server `select!` loop — verify with a `kill -INT` test, not just a normal exit

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Timeout regression breaks users with large calendars | MEDIUM | Roll back DAV timeout value; add a `--timeout` CLI flag as a short-term escape hatch while the right value is determined from user data |
| Confirmation token format change breaks mid-workflow agents | LOW | Version-prefix the token format (`v2:...`), add graceful "re-run preview" error message; no data migration needed since tokens are ephemeral |
| `join_all` partial-failure drops valid results in prod | MEDIUM | Revert to sequential fetch (safe fallback) until partial-success aggregation is implemented and tested |
| Connection pool exhaustion under AI-agent load | LOW | DAV clients are cheap to reconstruct; the immediate fix is to add the shared `reqwest::Client` without full context sharing; full client reuse can follow |
| Secret leaked in log (Debug impl regression) | HIGH | Immediately rotate the leaked credential; audit log aggregation systems for past occurrences; the code fix is one impl block change |
| kreuzberg feature flag breaks default download path | LOW | Re-add kreuzberg to default features; the feature flag is additive and can be backed out without breaking any existing users |
| Newtype ID GraphQL scalar registration missing | LOW | Caught at schema build time (panic), not at runtime; fix is adding `scalar!(NewTypeId)` and rebuilding |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Timeout regression (live calendar hang) | Phase: DAV stability (timeout + 4xx fixes) | Smoke test against live Fastmail account with > 50 events after fix; confirm 30s is sufficient |
| 4xx catch-all breaks existing error variant callers | Phase: DAV stability | Run full test suite; add wiremock tests for 400, 403, 410 responses |
| Debug secret redaction over-scoping | Phase: Security hardening | Inspect `Debug` output in tests; grep for `api_token` and `app_password` in test log output |
| Confirmation token migration | Phase: Security hardening | Integration test covers PREVIEW→restart→CONFIRM flow; document token format version |
| Concurrent fetch error aggregation | Phase: Performance + concurrency | Unit test with mock that returns error for one of three calendars; verify partial results returned |
| Connection pool exhaustion | Phase: Performance + DAV client reuse | Load test: 10 sequential MCP tool calls; observe TLS handshake count in debug log |
| Mutex held across I/O serializes MCP | Phase: Performance + DAV client reuse | Concurrent integration test: two MCP tool calls in parallel; measure wall clock time vs. sequential |
| process::exit breaks JSON contract | Phase: DAV stability (Output contract fix) | Test: invoke `spam` and `delete` confirmation-guard path; verify stdout JSON is valid |
| vCard/iCal injection | Phase: Security hardening | Fuzzing or property-based tests for escape functions; test with `\r\n` in email address field |
| kreuzberg optional feature flag | Phase: Performance (binary size + feature flags) | `cargo build --no-default-features` succeeds and produces a working binary; `--features document-extraction` build also passes |
| Newtype IDs in serde/GraphQL | Phase: Code quality (newtyping) | Schema introspection query returns without panic; serde round-trip tests for each newtype |
| Signal handling in rmcp | Phase: Stability (MCP signal handling) | `kill -TERM <pid>` against running MCP server; verify clean exit and no zombie process |
| DefaultHasher instability for contact IDs | Phase: Code quality | Replace with `siphasher::SipHasher13`; verify existing unit tests for contact ID generation still pass |

---

## Sources

- CODEBASE-REVIEW.md (root of fastmail-cli repo) — 33 findings, P1/P2/P3 tiers
- Fastmail CalDAV known 412 quirk: https://sourceforge.net/p/outlookcaldavsynchronizer/tickets/1607/
- Fastmail DAVx5 interop notes: https://www.davx5.com/tested-with/fastmail
- Fastmail CalDAV blog (scheduling side effects): https://www.fastmail.com/blog/announcing-caldav-scheduling-support-for-clients/
- tokio Mutex anti-pattern (held across await): https://tokio.rs/tokio/tutorial/shared-state
- tokio deadlock with Mutex: https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex
- reqwest connection pool best practices: https://docs.rs/reqwest/latest/reqwest/struct.Client.html
- DefaultHasher instability across Rust versions: https://internals.rust-lang.org/t/stability-of-hash-values/2241
- async-graphql NewType and scalar registration: https://async-graphql.github.io/async-graphql/en/custom_scalars.html
- wiremock Rust (port conflicts, random allocation): https://github.com/LukeMathWalker/wiremock-rs
- secrecy crate for secret redaction: https://docs.rs/secrecy/latest/secrecy/
- rmcp (official Rust MCP SDK): https://github.com/modelcontextprotocol/rust-sdk
- MCP stdio transport and stdout corruption: https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust
- Tokio task cancellation patterns: https://cybernetist.com/2024/04/19/rust-tokio-task-cancellation-patterns/
- Cargo optional features (breaking defaults): https://doc.rust-lang.org/cargo/reference/features.html

---
*Pitfalls research for: fastmail-cli v1.2 Hardening & Quality*
*Researched: 2026-04-04*
