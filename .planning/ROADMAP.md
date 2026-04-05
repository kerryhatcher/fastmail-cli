# Roadmap: Fastmail CLI

## Milestones

- ✅ **v1.0 Contact CRUD** — Phases 1-4 (shipped 2026-04-03)
- ✅ **v1.1 Calendar Access and Management** — Phases 5-11 (shipped 2026-04-04)
- 🚧 **v1.2 Hardening & Quality** — Phases 12-17 (in progress)

## Phases

<details>
<summary>✅ v1.0 Contact CRUD (Phases 1-4) — SHIPPED 2026-04-03</summary>

- [x] Phase 1: Contact Model Foundation (1/1 plans) — completed 2026-04-03
- [x] Phase 2: vCard Serialization (1/1 plans) — completed 2026-04-03
- [x] Phase 3: CardDAV Write Operations (1/1 plans) — completed 2026-04-03
- [x] Phase 4: CLI & MCP Surfaces (1/1 plans) — completed 2026-04-03

</details>

<details>
<summary>✅ v1.1 Calendar Access and Management (Phases 5-11) — SHIPPED 2026-04-04</summary>

- [x] Phase 5: CalDAV Foundation & Discovery (1/1 plans) — completed 2026-04-03
- [x] Phase 6: iCalendar Event Semantics (1/1 plans) — completed 2026-04-03
- [x] Phase 7: Calendar & Event CRUD Transport (1/1 plans) — completed 2026-04-03
- [x] Phase 8: CLI Calendar Experience (1/1 plans) — completed 2026-04-03
- [x] Phase 9: MCP Calendar Surface & Live Validation (1/1 plans) — completed 2026-04-03
- [x] Phase 10: Explicit Range Contract Closure (1/1 plans) — completed 2026-04-03
- [x] Phase 11: CLI Attendee Clearing Parity (1/1 plans) — completed 2026-04-03

</details>

### 🚧 v1.2 Hardening & Quality (In Progress)

**Milestone Goal:** Close all 32 in-scope codebase-review findings — security fixes, stability guards, performance wins, and integration test coverage — without regressing any v1.0/v1.1 capability.

- [x] **Phase 12: Foundation Safety** - Establish the safe baseline: HTTP 4xx surfacing, DAV timeouts, JSON output contract, path traversal defense, secret redaction, and constructor hardening (completed 2026-04-04)
- [x] **Phase 13: Security Hardening** - Injection escaping for vCard/iCal serialization, URL encoding for blob download URLs, and auth token input documentation (completed 2026-04-04)
- [x] **Phase 14: MCP Layer Refactor** - Atomic delivery: shared AppContext DAV client pool, nonce-bound confirmation tokens, GraphQL limits, signal handling, and mutex-narrowing (completed 2026-04-04)
- [x] **Phase 15: Performance** - Concurrent DAV fetching, targeted UID REPORT lookup, allocation reductions across JMAP and MCP layers, optional kreuzberg feature flag (completed 2026-04-05)
- [ ] **Phase 16: Integration Test Coverage** - wiremock-based tests in `tests/` verifying JMAP, send, auth, CalDAV, CardDAV, and HTTP error paths without a live server
- [ ] **Phase 17: Quality Polish** - let-else patterns, stable contact ID hashing, faster image resize filter, tokio feature trim, and stale allow cleanup

## Phase Details

### Phase 12: Foundation Safety
**Goal**: The codebase has a correct safety baseline — HTTP errors surface as actionable JSON, DAV clients cannot hang indefinitely, credential Debug output is redacted, attachment downloads cannot escape their target directory, and client constructors cannot panic
**Depends on**: Nothing (first v1.2 phase)
**Requirements**: STAB-01, STAB-02, STAB-03, STAB-09, STAB-10, SEC-01, SEC-06
**Success Criteria** (what must be TRUE):
  1. A JMAP 400 or 403 response produces a JSON `{"error": "Server error: HTTP 400"}` on stdout, not a serde deserialization panic or empty output
  2. All confirmation-guard exit paths (spam, delete masked email, delete contact, delete calendar, delete event) emit a `{"error": "..."}` JSON envelope; no `eprintln!` + `process::exit(1)` path remains
  3. `CardDavClient` and `CalDavClient` respect a 30-second timeout; a hung server connection does not block indefinitely
  4. Downloading an attachment whose server-supplied filename contains `../` writes only inside the user-specified output directory
  5. `Config` printed with `{:?}` shows `[REDACTED]` for `api_token` and `app_password`; `JmapClient::new()` returns `Result` rather than panicking
**Plans**: 4 plans
- [x] 12-01-PLAN.md — Config secret redaction (SecretString) and parse-error recovery guidance
- [x] 12-02-PLAN.md — DAV client 30s timeouts and attachment path-traversal defense
- [x] 12-03-PLAN.md — JMAP 4xx catch-all and fallible JmapClient::new()
- [x] 12-04-PLAN.md — Confirmation-guard JSON contract in src/main.rs

### Phase 13: Security Hardening
**Goal**: All user-supplied string data is escaped or validated before being written into vCard and iCalendar wire format, blob download URLs are correctly percent-encoded, and the auth token input surface is documented for multi-user safety
**Depends on**: Phase 12
**Requirements**: SEC-02, SEC-03, SEC-04, SEC-09
**Success Criteria** (what must be TRUE):
  1. A vCard EMAIL or TEL value containing a newline or colon cannot inject an additional vCard property line into the serialized output
  2. An iCalendar attendee `email` or RRULE `until` value containing special characters does not break the serialized VCALENDAR syntax
  3. A JMAP blob download URL constructed from a filename containing spaces or Unicode characters produces a correctly percent-encoded URL, not a rejected request
  4. The `auth` command does not accept the API token as a positional argument; the README documents `FASTMAIL_API_TOKEN` env var and `read -rs` shell pattern
**Plans**: 4 plans
- [x] 13-01-PLAN.md — vCard EMAIL/TEL escaping (SEC-02)
- [x] 13-02-PLAN.md — iCal attendee + RRULE escaping/validation (SEC-03)
- [x] 13-03-PLAN.md — JMAP blob download URL percent-encoding (SEC-09)
- [x] 13-04-PLAN.md — Auth command env/stdin + README migration (SEC-04)

### Phase 14: MCP Layer Refactor
**Goal**: The MCP GraphQL layer uses a shared `AppContext` (no TLS handshake per tool call), confirmation tokens are bound to a per-process HMAC nonce (no forgeable deterministic hash), GraphQL query cost is bounded, SIGTERM/SIGINT are handled gracefully, and no `MutexGuard` is held across an `await` in any resolver
**Depends on**: Phase 12
**Requirements**: PERF-03, SEC-05, SEC-07, SEC-08, STAB-04, STAB-07
**Success Criteria** (what must be TRUE):
  1. Successive MCP tool calls reuse the same `CardDavClient` and `CalDavClient` instances; no new TLS handshake occurs between calls within a session
  2. A confirmation token obtained from a preview step cannot be reproduced by replaying the same mutation inputs; each server restart invalidates all prior tokens
  3. A GraphQL query with depth > 5 or complexity > 200 is rejected with a clear error before execution
  4. `markAsSpam` requires the same nonce-bound confirmation token as `deleteContact`, `deleteCalendar`, and `deleteEvent`
  5. Sending SIGTERM to the MCP server process results in a clean exit; no pending response is silently dropped
**Plans**: 4 plans
- [x] 14-01-PLAN.md — AppContext foundation (OnceCell DAV, HMAC key, schema limits)
- [x] 14-02-PLAN.md — Migrate resolvers to AppContext (DAV sharing, HMAC tokens, STAB-07 audit)
- [x] 14-03-PLAN.md — markAsSpam HMAC confirmation gate (SEC-08)
- [x] 14-04-PLAN.md — SIGTERM/SIGINT graceful shutdown (STAB-04)

### Phase 15: Performance
**Goal**: Multi-calendar and multi-address-book operations complete concurrently with partial-failure tolerance, single-event lookup no longer downloads the full event history, and memory allocations in the JMAP and MCP layers are reduced through Bytes, Arc, and owned-parse patterns
**Depends on**: Phase 12
**Requirements**: PERF-01, PERF-02, PERF-04, PERF-05, PERF-06, PERF-07, PERF-08, PERF-09, PERF-10, PERF-11, STAB-06
**Success Criteria** (what must be TRUE):
  1. `list_events` across multiple calendars and `search_contacts` across multiple address books issue all fetches concurrently; a single failing book logs a warning and does not abort the operation
  2. `get_event_by_id` issues a UID-targeted CalDAV REPORT rather than fetching all events from every calendar
  3. `cargo build --no-default-features` succeeds and produces a binary without bundled pdfium; `cargo build` (default features) still includes document extraction
  4. Blob downloads return `bytes::Bytes` without a double-allocation; `parse_response` consumes owned JSON without cloning the response subtree
**Plans**: 4 plans
- [x] 15-01-PLAN.md — Concurrent DAV fetches + UID-targeted REPORT (STAB-06, PERF-01, PERF-02)
- [x] 15-02-PLAN.md — JMAP memory: Bytes blob, owned parse_response, Arc caches (PERF-04, PERF-05, PERF-07, PERF-08)
- [x] 15-03-PLAN.md — GqlEmail Arc-shared address resolvers (PERF-10)
- [x] 15-04-PLAN.md — Optional extract feature + Triangle filter + narrowed tokio (PERF-06, PERF-09, PERF-11)

### Phase 16: Integration Test Coverage
**Goal**: The JMAP send, auth, and error-path behaviors, CalDAV concurrent fetch with partial-failure tolerance, and CardDAV CRUD flows are verifiable via a wiremock-based integration test suite that runs without a live Fastmail account
**Depends on**: Phases 12, 13, 14, 15
**Requirements**: TEST-01
**Success Criteria** (what must be TRUE):
  1. `cargo test` runs the integration suite without network access and all tests pass
  2. Tests cover: JMAP auth flow, email send, HTTP 401/429/500/4xx error paths returning JSON, CalDAV concurrent fetch with one failing book, CardDAV CRUD round-trip, and MCP GraphQL resolver happy paths
  3. Tests live exclusively in `tests/` (top-level Cargo integration tests); no wiremock usage appears in `#[cfg(test)]` blocks inside `src/`
**Plans**: 4 plans
- [x] 16-01-PLAN.md — Test infrastructure: wiremock dev-dep + URL-override constructors + tests/common harness
- [ ] 16-02-PLAN.md — JMAP integration tests: auth, send wire-format, 401/429/500/4xx error mapping
- [x] 16-03-PLAN.md — DAV integration tests: CalDAV concurrent partial-failure + CardDAV CRUD round-trip
- [x] 16-04-PLAN.md — MCP GraphQL resolver smoke test with test AppContext

### Phase 17: Quality Polish
**Goal**: Fragile `unwrap()` patterns are replaced with `let-else` guards, contact fallback IDs use a stable hasher, image resize uses a faster filter, tokio pulls in only the features it needs, and stale allow attributes are removed
**Depends on**: Phase 12
**Requirements**: STAB-05, STAB-08, QUAL-01
**Success Criteria** (what must be TRUE):
  1. `download.rs` has no triple-`unwrap()` chain; the file-name guard uses `let Some(..) else { return }`
  2. Contact fallback IDs are identical across Rust versions and builds (stable hasher, not `DefaultHasher`)
  3. `cargo clippy --all-targets --all-features` reports zero warnings; stale `#[allow(unused_imports)]` annotations on active imports are gone
**Plans**: 4 plans
- [ ] 14-01-PLAN.md — AppContext foundation (OnceCell DAV, HMAC key, schema limits)
- [ ] 14-02-PLAN.md — Migrate resolvers to AppContext (DAV sharing, HMAC tokens, STAB-07 audit)
- [ ] 14-03-PLAN.md — markAsSpam HMAC confirmation gate (SEC-08)
- [x] 14-04-PLAN.md — SIGTERM/SIGINT graceful shutdown (STAB-04)

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Contact Model Foundation | v1.0 | 1/1 | Complete | 2026-04-03 |
| 2. vCard Serialization | v1.0 | 1/1 | Complete | 2026-04-03 |
| 3. CardDAV Write Operations | v1.0 | 1/1 | Complete | 2026-04-03 |
| 4. CLI & MCP Surfaces | v1.0 | 1/1 | Complete | 2026-04-03 |
| 5. CalDAV Foundation & Discovery | v1.1 | 1/1 | Complete | 2026-04-03 |
| 6. iCalendar Event Semantics | v1.1 | 1/1 | Complete | 2026-04-03 |
| 7. Calendar & Event CRUD Transport | v1.1 | 1/1 | Complete | 2026-04-03 |
| 8. CLI Calendar Experience | v1.1 | 1/1 | Complete | 2026-04-03 |
| 9. MCP Calendar Surface & Live Validation | v1.1 | 1/1 | Complete | 2026-04-03 |
| 10. Explicit Range Contract Closure | v1.1 | 1/1 | Complete | 2026-04-03 |
| 11. CLI Attendee Clearing Parity | v1.1 | 1/1 | Complete | 2026-04-03 |
| 12. Foundation Safety | v1.2 | 4/4 | Complete   | 2026-04-04 |
| 13. Security Hardening | v1.2 | 4/4 | Complete   | 2026-04-04 |
| 14. MCP Layer Refactor | v1.2 | 4/4 | Complete   | 2026-04-04 |
| 15. Performance | v1.2 | 4/4 | Complete   | 2026-04-05 |
| 16. Integration Test Coverage | v1.2 | 3/4 | In Progress|  |
| 17. Quality Polish | v1.2 | 0/? | Not started | - |
