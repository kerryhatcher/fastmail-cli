# Project Research Summary

**Project:** fastmail-cli v1.2 Hardening & Quality
**Domain:** Rust CLI + MCP server hardening — security, stability, performance, testing
**Researched:** 2026-04-04
**Confidence:** HIGH

## Executive Summary

fastmail-cli v1.2 is a hardening milestone, not a feature milestone. The production stack (tokio 1.49, reqwest 0.13.1, async-graphql 7, rmcp 0.12, clap 4.5, serde 1.0, thiserror 2.0, roxmltree 0.21) is validated and must not change. The milestone closes 33 codebase-review findings across security, stability, performance, testing, and code quality. The recommended approach is a 7-phase sequence that delivers security and stability fixes first (narrow blast radius, immediate safety value), then restructures the MCP layer, then adds performance improvements, test coverage, and quality polish — with newtyped IDs deferred to an isolated final phase.

The single most important sequencing constraint is that the MCP AppContext refactor (finding #6) must land before the NonceStore confirmation token work (finding #14), which must land before the `mark_as_spam` confirmation gate (finding #25). This chain represents the MCP layer's entire security improvement arc and must be delivered atomically to avoid a window where the nonce infrastructure exists but is not yet protecting all mutations. Separately, the output contract fix (finding #11) must ship alongside or before the 4xx HTTP error handling fix (finding #1) to ensure error paths always produce JSON on stdout, not silent exits.

The key risk is accidental merge conflicts from high-ripple changes. Finding #23 (newtyped IDs) touches every model struct, command handler, and GraphQL type in the codebase. It must not be worked concurrently with any other module-touching change. All four researchers independently flagged it for deferral to an isolated phase — this is the strongest cross-cutting signal in the research. Secondary risks are behavioral regressions from the timeout and concurrent DAV fetch changes, which require smoke testing against live Fastmail accounts with large calendar/contact histories.

---

## Key Findings

### Recommended Stack

The existing production stack requires no changes. Three production dependencies are added and one dev dependency is added to close specific findings. `secrecy 0.10.3` provides `SecretString` with automatic `[REDACTED]` Debug output and zeroize-on-drop memory safety, fixing finding #15. `percent-encoding 2.3.2` (already a transitive dep via reqwest/url) is pinned explicitly for URL-safe blob download template substitution, fixing finding #30. `futures 0.3.32` (already transitive) is pinned explicitly for `join_all` / `try_join_all` to enable concurrent DAV fetching, fixing findings #4 and #19. `wiremock 0.6.5` is added as a dev dependency for integration tests, fixing finding #7.

`async-graphql 7` already ships `limit_depth` and `limit_complexity` on `SchemaBuilder` — no new dependency is needed for finding #24. Tokio's `full` feature set can be narrowed to 5 explicit features (`rt-multi-thread`, `macros`, `sync`, `signal`, and optionally `time`) with MEDIUM confidence — if compilation fails after narrowing, add `net` and/or `io-util` before reverting.

**New dependencies:**

- `secrecy 0.10.3` (prod): Secret redaction + zeroize-on-drop — fixes #15; ecosystem standard over manual Debug impl
- `percent-encoding 2.3.2` (prod): RFC 3986 URL encoding — fixes #30; zero binary-size cost (already transitive)
- `futures 0.3.32` (prod, `alloc` features only): `join_all` for concurrent DAV fetches — fixes #4, #19; already transitive
- `wiremock 0.6.5` (dev): Async HTTP mock server — fixes #7; runs real TCP server on random port, parallel-safe, zero production code refactor
- `hmac` + `sha2` + `hex` (prod, RustCrypto): HMAC-SHA256 for nonce-bound confirmation tokens — fixes #14
- `rand` with `getrandom` (prod): `OsRng` for per-process nonce generation — fixes #14

**Stack note on `secrecy` vs manual Debug (finding #15):** FEATURES.md recommends manual `Debug` impl for v1.2 scope management (zero new deps, lower call-site churn); STACK.md and PITFALLS.md recommend `secrecy` as the ecosystem standard (zeroize-on-drop, `expose_secret()` audit trail). User decision needed before Phase 1 planning. If manual impl is chosen now, schedule the `secrecy` migration together with newtyped IDs (#23) in v1.3 to batch the cross-cutting refactor.

---

### Expected Features (Findings Organized by Value)

This milestone closes debt rather than adding capabilities. "Features" are the 33 codebase-review findings.

**Must have — table stakes (users expect reliability and safety):**
- HTTP 4xx errors surfaced as actionable messages (#1) — silent error swallowing is broken behavior
- DAV client timeouts (#2) — hung connections with no timeout are not acceptable
- Path traversal defense in attachment download (#3) — server-supplied filenames must not escape target directory
- Structured JSON output from all error paths (#11) — MCP hosts and scripts parse stdout; eprintln+exit(1) is invisible to them
- Secret redaction in Debug (#15) — accidental `{:?}` of Config must not leak credentials to logs
- vCard/iCal property injection escaping (#8, #9) — input escaping is table stakes for any serializer writing to a protocol wire format
- Confirmation token security (#14) — current deterministic hash is security theatre; per-process nonce HMAC required

**Should have — differentiators:**
- Concurrent DAV fetching (#4) — multi-calendar users see 3-10x faster list/search operations
- Targeted event lookup via UID REPORT (#5) — getting one event should not download entire history
- DAV client reuse in MCP via AppContext (#6) — avoids TLS handshake per tool call; matters for AI agent workflows
- wiremock integration tests (#7) — makes the send/auth/JMAP path verifiable without a live server
- Optional pdfium feature flag (#16) — users who do not need document extraction get a 10-20MB smaller binary
- GraphQL depth + complexity limits (#24) — protects MCP server from runaway queries

**Defer to v1.3 or later:**
- Newtyped IDs (#23) — cross-module refactor touching every model, command handler, and GraphQL type; schedule as first item in v1.3 before any concurrent work begins
- Full server-side confirmation token store — stateless HMAC nonce is simpler and equally secure for personal-use MCP server
- Interactive stdin TTY prompt for auth token (#10) — accept via env var (already supported); document `read -rs TOKEN` for shell use; full interactive prompt deferred

---

### Architecture Approach

The architecture evolves from per-request DAV client construction to a unified `AppContext` struct stored in the async-graphql schema's context data. `AppContext` holds all three shared protocol clients (`Arc<Mutex<JmapClient>>`, `Arc<Mutex<CardDavClient>>`, `Arc<Mutex<CalDavClient>>`) and the new `NonceStore` for confirmation token lifecycle. This mirrors the existing JMAP client pattern that was already working correctly and extends it to cover CardDAV and CalDAV. A new `src/nonce.rs` module manages single-use token issuance and consumption via UUID generation. A new `tests/` top-level directory (Cargo integration tests) holds all wiremock-based HTTP mock tests — these are separate compiled binaries and cannot be placed in `#[cfg(test)]` blocks.

**Major components and v1.2 changes:**

1. `src/mcp/graphql/mod.rs` — Replace `JmapContext` with `AppContext`; add `.limit_depth(5).limit_complexity(200)` on SchemaBuilder (findings #6, #24)
2. `src/nonce.rs` (new) — `NonceStore` with `issue()` / `consume()` for single-use UUID confirmation tokens (finding #14)
3. `src/ids.rs` (new, DEFERRED) — Newtyped `EmailId`, `MailboxId`, `ThreadId`, `AccountId` — deferred to v1.3; documented here for planning awareness only
4. `tests/` (new) — `tests/common/mod.rs` scaffold + domain test files using wiremock (finding #7)
5. `src/carddav/mod.rs` + `src/caldav/mod.rs` — Timeout constructors, `join_all` concurrent fetching, injection escaping, UID-targeted REPORT (findings #2, #4, #5, #8, #9)
6. `src/config.rs` — Manual `Debug` impl (or `SecretString`) redacting `api_token` and `app_password` (finding #15)
7. `src/mcp/graphql/query.rs` — Drop `MutexGuard` before every `await` in all resolvers (finding #26)

**Key pattern — Mutex-Narrow Locking:** Lock only to clone credentials or read cached state; drop the guard before any `.send().await`. HTTP is stateless at the protocol level — no lock is needed during I/O. This prevents the serialization anti-pattern (finding #26) from being reintroduced when DAV clients are wrapped in `Arc<Mutex<>>` for AppContext.

---

### Critical Pitfalls

1. **Concurrent DAV fetch drops valid partial results** (finding #4 + Pitfall 5) — `join_all(...).await.into_iter().collect::<Result<Vec<_>, _>>()?` silently applies first-error-wins semantics, which is strictly worse than the sequential loop it replaced. Use the partial-success pattern: collect `Vec<Result<T, E>>`, partition into successes and failures, log failures with `tracing::debug!`, return successes only. Add `tokio::sync::Semaphore` with cap 5 to prevent TLS connection storms to Fastmail.

2. **Confirmation token format change breaks mid-workflow agents** (finding #14 + Pitfall 4) — Changing the token format invalidates any token an agent obtained during PREVIEW before the server was updated. Version-prefix the new token format (`v2:...`) and emit a graceful rejection: "Token format has changed. Re-run the preview step to obtain a new token." `DefaultHasher` instability means the existing format is already unreliable across Rust versions; migration is necessary.

3. **DAV timeout regression for large calendar users** (Pitfall 1) — Setting CalDAV/CardDAV timeout too aggressively (e.g., 10s) breaks REPORT queries for users with full calendars. Match the existing JMAP 30s constant exactly. Use `reqwest::ClientBuilder::timeout()` (full-request guard), not `connect_timeout` alone. Co-locate the constant with the JMAP timeout value for future synchronization.

4. **MCP mutex held across async I/O serializes all requests** (finding #26 + Pitfall 7) — If the AppContext DAV client fix (#6) naively copies `Arc<Mutex<>>` wrapping without narrowing lock scope, the concurrency serialization problem is replicated across three clients instead of one. Drop `MutexGuard` before every `await` in resolvers. Consider `Arc<RwLock<>>` for read-heavy operations.

5. **Output contract broken before 4xx fix is tested** (findings #1 + #11 + Pitfall 2) — Fix #11 (`process::exit` to `Output::error().print()`) alongside or before fix #1 (4xx catch-all). Without #11, the new `Error::Server` variant surfaces as silent exit with no JSON. Add wiremock tests for 400, 403, 410 responses to verify JSON output is well-formed.

---

## Implications for Roadmap

Based on combined research, a 7-phase sequence is recommended. Phases 1 and 2 have narrow blast radius and should be completed as sequential small PRs. Phase 3 is the high-ripple MCP refactor and must be delivered atomically. Phase 7 is newtyped IDs alone with no concurrent work.

### Phase 1: Foundation Safety
**Rationale:** All LOW-complexity P1 fixes with no dependencies between them. Establishes safe error handling, output contract, and timeout behavior that all subsequent phases rely on. The output contract fix (#11) must be in this phase to prevent any new code added in later phases from accidentally using the `process::exit` pattern.
**Delivers:** No more silent hangs (timeouts), no more credential leaks in logs (secret redaction), no more broken JSON output on error paths (output contract), no more path traversal in attachment downloads.
**Addresses findings:** #1 (4xx HTTP errors), #2 (DAV timeouts), #3 (path traversal), #11 (process::exit output contract), #15 (Config Debug secrets), #32 (JmapClient::new expect)
**Hard pairing:** #1 + #11 must ship together — 4xx errors need the output contract to be correct first, otherwise new error paths produce no JSON output.
**Pitfall to avoid:** Pitfall 2 — audit all `src/commands/` error match arms after adding the 4xx catch-all to verify they correctly handle the new `Error::Server` variant rather than the old deserialization error shape.
**Research flag:** Standard patterns — skip phase research.

### Phase 2: Security Hardening
**Rationale:** LOW-complexity injection fixes and URL encoding, grouped together because they share the same escape/validate pattern across two DAV modules. Auth token hardening (#10) is documentation-only in v1.2 (env var already supported).
**Delivers:** vCard and iCal property injection prevented, blob download URLs correctly encoded, auth token input documented for multi-user safety.
**Addresses findings:** #8 (vCard injection), #9 (iCal injection), #10 (auth token input documentation), #30 (URL encoding)
**Pitfall to avoid:** Pitfall 3 — apply `escape_value()` to all four sites: EMAIL address value, EMAIL type parameter, TEL number, TEL type parameter. Do not stop at the label field only.
**Research flag:** Standard patterns — skip phase research.

### Phase 3: MCP Layer Refactor (ship atomically)
**Rationale:** This is the highest-ripple phase. AppContext (#6), NonceStore (#14), confirmation gate (#25), GraphQL limits (#24), signal handling (#17), and mutex narrowing (#26) all touch `src/mcp/graphql/`. They must land as a single atomic delivery to avoid merge conflicts and a security window where nonce infrastructure exists but is not yet protecting all mutations.
**Delivers:** Shared DAV client pool (no TLS handshake per tool call), per-session single-use confirmation tokens, GraphQL depth/complexity limits, graceful MCP shutdown on SIGTERM/SIGINT, no mutex serialization on concurrent requests.
**Addresses findings:** #6 (AppContext + DAV client reuse), #14 (HMAC nonce tokens), #24 (GraphQL limits), #25 (mark_as_spam confirmation gate), #17 (MCP signal handling), #26 (mutex scope)
**Hard pairing — #6 + #14 + #26 must ship together:** AppContext is the container for NonceStore (#14) and the foundation for the mutex-narrow pattern (#26). Landing #14 before #6 means NonceStore has no home in the schema data. Landing #6 without #26 introduces new mutex-held-across-await instances while claiming to fix the problem.
**Hard dependency chain — #14 before #25:** `mark_as_spam` confirmation gate reuses the same `NonceStore` infrastructure. Wiring it before the nonce pattern exists reinstates the insecure hash token on the new mutation.
**New files:** `src/nonce.rs`, updated `src/mcp/graphql/mod.rs` (AppContext), `src/mcp/graphql/query.rs` (mutex-narrow), `src/mcp/graphql/mutation.rs` (nonce gates), `src/mcp/mod.rs` (signal handling)
**Pitfall to avoid:** Pitfall 4 — version-prefix the nonce token format (`v2:...`) with a graceful rejection message for old-format tokens. Document the token format change in the phase notes.
**Research flag:** May benefit from a focused research pass on `tokio::select!` + rmcp 0.12 shutdown lifecycle before implementation.

### Phase 4: Performance
**Rationale:** Concurrent DAV fetching and targeted REPORT queries require care around partial-failure semantics (Pitfall 5). Group caldav and carddav changes together to avoid repeated file churn. Depends on Phase 1 (timeouts established) so concurrent fetches inherit correct timeout behavior automatically.
**Delivers:** 3-10x faster multi-calendar/multi-addressbook operations, single-event lookup without full history download, reduced memory allocations in JMAP layer.
**Addresses findings:** #4 (concurrent DAV fetch), #5 (UID REPORT), #12 (Bytes blob return), #13 (owned JSON parse), #19 (server-side contact filter + per-book error tolerance), #20 (Arc mailbox cache), #21 (Arc capabilities), #31 (Arc address fields)
**Hard pairing — #4 + #5 in caldav.rs together:** UID REPORT reduces the number of joins needed; they interact in the same inner fetch loops. Batching avoids repeated churn in `src/caldav/mod.rs`.
**Pitfall to avoid:** Pitfall 5 — use `Vec<Result<T, E>>` + partition pattern, not `collect::<Result<Vec<_>, _>>()?`. Add `tokio::sync::Semaphore` with cap 5 for concurrent DAV connections.
**Research flag:** Standard patterns — skip phase research. Smoke test UID REPORT against live Fastmail before marking complete (Fastmail has known CalDAV quirks).

### Phase 5: Integration Test Coverage
**Rationale:** Tests land after the fixes they exercise, verifying correct behavior rather than documenting pre-fix broken state. wiremock tests must live in `tests/` (top-level Cargo integration tests) — they are separate compiled binaries and cannot be colocated in `#[cfg(test)]` blocks inside source files.
**Delivers:** Verifiable JMAP send/auth/error paths, CalDAV concurrent fetch with partial-failure tolerance tests, CardDAV CRUD tests — all without requiring a live Fastmail account.
**Addresses findings:** #7 (integration test coverage across all HTTP layers)
**New files:** `tests/common/mod.rs`, `tests/jmap_client.rs`, `tests/send_email.rs`, `tests/auth_flow.rs`, `tests/carddav_client.rs`, `tests/caldav_client.rs`
**wiremock placement constraint:** Integration tests in `tests/` only. Colocated `#[cfg(test)]` blocks remain correct for pure unit tests. `wiremock` goes in `[dev-dependencies]` only.
**Research flag:** Standard patterns — wiremock is well-documented (zero2prod, multiple Rust testing guides). Skip phase research.

### Phase 6: Quality Polish
**Rationale:** All LOW-to-MEDIUM complexity findings with minimal blast radius. kreuzberg feature flag (#16) requires a CI matrix update. Tokio feature narrowing (#29) has MEDIUM confidence and needs compilation verification — if it fails, it can be reverted independently without affecting any other Phase 6 work.
**Delivers:** Optional 10-20MB binary size reduction (no bundled pdfium for users who don't need document extraction), faster CI cold-cache builds, stable contact ID hashing across Rust versions, clean stale code removed.
**Addresses findings:** #16 (kreuzberg optional feature flag), #18 (let-else patterns in download.rs), #22 (Triangle resize filter), #27 (stable hash for contact IDs), #28 (stale allow), #29 (tokio features trim), #33 (config error recovery message)
**Pitfall to avoid:** kreuzberg feature flag — `cargo build --no-default-features` must succeed; `download --format json` without the feature must emit a clear error explaining it needs `--features document-extraction`.
**Research flag:** Standard patterns — skip phase research.

### Phase 7: Newtype IDs (isolated, broad-but-mechanical)
**Rationale:** Finding #23 touches every model struct, command handler, and GraphQL type. All four researchers independently flagged it for deferral. It is completely independent of all security and stability fixes. It must run after all other work is merged to avoid guaranteed merge conflicts in heavily-edited files.
**Delivers:** Compile-time guarantee that email_id is never passed where mailbox_id is expected; eliminates an entire class of cross-type ID confusion bugs.
**Addresses findings:** #23 (stringly-typed IDs)
**New files:** `src/ids.rs` (newtyped `EmailId`, `MailboxId`, `ThreadId`, `AccountId`)
**User decision required:** This is the strongest cross-cutting signal in the combined research — all four researchers independently flagged it. Decision: defer to v1.3 as its first item (recommended), or include as Phase 7 within v1.2. If included in v1.2, it must be the only work in flight during its execution with no concurrent PRs touching models, commands, or MCP types.
**Implementation constraint:** Each newtype appearing in the GraphQL schema requires `scalar!()` or `#[derive(NewType)]` registration — missing registration causes a schema build panic at runtime, not a compile error. Implement `Deref<Target = str>`, `AsRef<str>`, and `Display` on each newtype to minimize call-site blast radius. `clippy` will surface every site needing update after struct definitions land.
**Research flag:** Skip phase research — pattern is mechanical once `src/ids.rs` types are defined.

---

### Phase Ordering Rationale

- **Phases 1-2 first** because they have the lowest blast radius (1-5 lines per fix) and establish the safety baseline every later phase builds on. The output contract (#11) and timeout (#2) fixes are prerequisites for all test writing.
- **Phase 3 atomic** because AppContext, NonceStore, and mutex-narrowing all touch the same MCP files. Splitting them creates a security window where the shared client exists but nonce tokens are still insecure, or where new mutex-across-await is introduced while claiming to fix client reuse.
- **Phase 4 after Phase 1** because concurrent DAV fetches should inherit the timeout configuration established in Phase 1 automatically. The `join_all` implementation assumes `reqwest::Client` already has a timeout set.
- **Phase 5 after Phases 1-4** because tests verify fixed behavior. Tests written before fixes exist document broken behavior and must be rewritten.
- **Phase 7 last** because newtyped IDs conflict with everything. Any concurrent PR touching `src/models/mod.rs`, `src/jmap/mod.rs`, or `src/mcp/graphql/types.rs` will have unresolvable merge conflicts with the ID newtype migration.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 3:** `tokio::select!` + rmcp 0.12 shutdown integration may have undocumented edge cases. Verify rmcp's server lifecycle API before planning the signal handling implementation.
- **Phase 4 (smoke only):** Fastmail CalDAV UID-targeted REPORT syntax — verify `<C:comp-filter name="VEVENT"><C:prop-filter name="UID">` against Fastmail's Cyrus IMAP implementation. Fastmail has known non-standard 412 behaviors on CardDAV; CalDAV may have similar quirks.

Phases with standard patterns (skip research-phase):
- **Phase 1:** All LOW complexity, well-documented patterns (Path::file_name(), match arm, manual Debug impl, Duration::from_secs).
- **Phase 2:** Injection escaping is standard input validation; URL encoding uses a pinned transitive dep.
- **Phase 5:** wiremock 0.6 is well-documented with established patterns.
- **Phase 6:** All mechanical cleanups with no protocol interaction.
- **Phase 7:** Mechanical newtype migration — clippy-guided once types are defined.

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified against docs.rs. One exception: tokio feature narrowing is MEDIUM — reqwest's internal tokio feature requirements are not explicitly documented. Safe to attempt with CI gating. |
| Features | HIGH | All 33 findings sourced from direct CODEBASE-REVIEW.md analysis; patterns verified against official docs. Feature prioritization matrix includes exact line number references. |
| Architecture | HIGH | Based on direct analysis of all 12 source files. Component responsibilities, integration points, and exact line numbers for each finding are confirmed in ARCHITECTURE.md. |
| Pitfalls | HIGH | Grounded in actual codebase and live-validated v1.1 code. Fastmail-specific quirks (Cyrus 412 behavior, CalDAV scheduling side effects) sourced from DAVx5 interop notes and Fastmail engineering blog. |

**Overall confidence:** HIGH

### Gaps to Address

- **`secrecy` vs manual Debug for #15:** Research is split on approach. FEATURES.md recommends manual impl (zero new deps); STACK.md and PITFALLS.md recommend `secrecy` (ecosystem standard, zeroize-on-drop). User decision needed before Phase 1 planning. Recommendation: choose `secrecy` now to avoid a second migration in v1.3; accept the call-site churn as part of Phase 1.

- **Tokio feature narrowing compilation (#29):** MEDIUM confidence only. Safe migration path: narrow in a dedicated commit with CI gating. If `cargo build` fails, add `net` and/or `io-util` before reverting to `full`. Can be isolated to Phase 6 and reverted independently without affecting any other work.

- **Fastmail CalDAV UID REPORT syntax (#5):** The UID filter syntax is standard CalDAV RFC but Fastmail runs Cyrus IMAP with known non-standard behaviors. Smoke test against a live account before marking Phase 4 complete. The regression risk (timeout from unbounded REPORT) is higher than the risk of incorrect results.

- **Nonce token format version prefix (#14):** The exact `v2:...` prefix format and where to document the migration window for running MCP host sessions needs a decision before Phase 3 implementation begins. The pitfall researcher is explicit: failing to communicate the format change creates "invalid token" errors for agents mid-workflow.

---

## Cross-Cutting Signals (User Decisions Required)

These signals were independently flagged by multiple researchers and require explicit acknowledgement before roadmap creation:

| Signal | Flagged By | Decision |
|--------|-----------|----------|
| #23 Newtyped IDs: defer to v1.3 or isolate as Phase 7 | All 4 researchers | Confirm deferral strategy before roadmap is finalized |
| #1 + #11 output contract hard pairing | FEATURES + ARCHITECTURE + PITFALLS | Must ship together; cannot split across phases |
| #6 + #14 + #26 MCP refactor must be atomic | FEATURES + ARCHITECTURE + PITFALLS | Phase 3 is a single atomic delivery; no partial landing |
| #14 before #25 (nonce-dependent spam confirmation) | FEATURES + ARCHITECTURE | Sequencing constraint; #25 cannot land in a phase before #14 |
| wiremock tests in `tests/` not colocated | STACK + ARCHITECTURE | Implementation constraint; must be captured in phase spec |
| async-graphql depth/complexity limits are built-in | STACK + ARCHITECTURE | No new middleware needed; two SchemaBuilder method calls only |
| `secrecy` crate vs manual Debug for #15 | STACK vs FEATURES | User decision needed before Phase 1 planning |

---

## Sources

### Primary (HIGH confidence)
- `CODEBASE-REVIEW.md` (root, 2026-04-04) — 33 findings, P1/P2/P3 tiers; source of all finding numbers and integration point line numbers
- `docs.rs/wiremock/0.6.5` — version, tokio 1.47.1+ compatibility, `MockServer::start().await` API confirmed
- `docs.rs/secrecy/0.10.3` — `SecretString`, `[REDACTED]` Debug output, `expose_secret()` API confirmed
- `docs.rs/percent-encoding/2.3.2` — `utf8_percent_encode`, `NON_ALPHANUMERIC` AsciiSet confirmed
- `docs.rs/futures/0.3.32` — `join_all` / `try_join_all` API confirmed
- `async-graphql.github.io/async-graphql/en/depth_and_complexity.html` — `limit_depth` / `limit_complexity` builder methods confirmed for v7
- `docs.rs/async-graphql/latest/async_graphql/struct.SchemaBuilder.html` — exact method signatures and defaults verified
- `docs.rs/tokio/latest` — feature flag breakdown (`rt-multi-thread`, `macros`, `sync`, `signal`, `time`) verified
- Direct codebase analysis of all 12 source files — exact line numbers and integration points in ARCHITECTURE.md

### Secondary (MEDIUM confidence)
- `tokio.rs/tokio/tutorial/shared-state` — Mutex-before-await anti-pattern documented
- `davx5.com/tested-with/fastmail` — Fastmail CalDAV interop quirks (Cyrus IMAP 412 on `If-None-Match: *`)
- `fastmail.com/blog/announcing-caldav-scheduling-support-for-clients/` — CalDAV scheduling side effects
- GitHub tokio#2057 + Rust Users Forum — reqwest internal tokio dependency discussion (informs MEDIUM on feature narrowing)
- `LukeMathWalker/wiremock-rs` GitHub — string-based method matching, parallel isolation design confirmed

### Tertiary (LOW confidence)
- AWS AppSync GraphQL limits documentation — used for benchmarking `limit_depth` defaults; AppSync max is 75; calibrated to 5-7 for fastmail-cli personal-use context

---
*Research completed: 2026-04-04*
*Ready for roadmap: yes*
