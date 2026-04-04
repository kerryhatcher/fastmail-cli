# Feature Research

**Domain:** Rust CLI + MCP server hardening (security, stability, performance, quality, testing)
**Researched:** 2026-04-04
**Confidence:** HIGH — all 33 findings sourced from codebase review; patterns verified against official docs and ecosystem sources

---

## Framing: This Is Not a Feature Milestone, It Is a Hardening Milestone

v1.2 closes debt rather than adding capabilities. The "features" here are the 33 codebase-review findings, organized into groups the planner can sequence. Each finding is a concrete fix with known scope — the research task is to understand *how* each class of fix works, flag complexity, and surface dependencies.

---

## Finding Catalogue (All 33, Categorized)

### Category A — Security (9 findings)

| # | Finding | P-tier | Complexity | Depends On |
|---|---------|--------|------------|------------|
| 3 | Path traversal in attachment download | P1 | LOW | — |
| 2 | No timeout on CardDAV/CalDAV HTTP clients | P1 | LOW | — |
| 8 | vCard property injection via email/phone labels | P2 | LOW | — |
| 9 | iCalendar property injection via attendee fields | P2 | LOW | — |
| 10 | Auth token visible in process list | P2 | MEDIUM | — |
| 14 | Deterministic confirmation token (no nonce) | P2 | MEDIUM | — |
| 15 | Debug derive on Config structs holding secrets | P2 | LOW | — |
| 24 | No GraphQL depth/complexity limits | P3 | LOW | — |
| 25 | `mark_as_spam` lacks confirmation token | P3 | LOW | Finding 14 pattern must exist first |

### Category B — Stability (9 findings)

| # | Finding | P-tier | Complexity | Depends On |
|---|---------|--------|------------|------------|
| 1 | HTTP 4xx codes silently ignored in JMAP client | P1 | LOW | — |
| 11 | `process::exit(1)` breaks JSON output contract | P2 | LOW | — |
| 17 | MCP server has no graceful signal handling | P2 | MEDIUM | — |
| 18 | Fragile `unwrap()` pattern in download.rs | P2 | LOW | — |
| 19 | Contact search fetches ALL contacts for in-memory filtering | P2 | MEDIUM | — |
| 26 | MCP mutex held across async I/O | P3 | MEDIUM | Finding 6 (DAV client reuse) |
| 30 | Blob download URL template values not URL-encoded | P3 | LOW | — |
| 32 | `expect()` in `JmapClient::new()` | P3 | LOW | — |
| 33 | Config corruption error lacks recovery guidance | P3 | LOW | — |

### Category C — Performance (10 findings)

| # | Finding | P-tier | Complexity | Depends On |
|---|---------|--------|------------|------------|
| 4 | Sequential multi-calendar/address-book fetching | P1 | MEDIUM | — |
| 5 | `get_event_by_id()` fetches ALL events from ALL calendars | P1 | MEDIUM | — |
| 6 | DAV clients recreated per MCP request | P1 | MEDIUM | — |
| 12 | Blob download double-allocation (`bytes().to_vec()`) | P2 | LOW | — |
| 13 | JSON parse clones entire response tree | P2 | LOW | — |
| 16 | `bundled-pdfium` inflates binary size | P2 | MEDIUM | — |
| 20 | Mailbox cache returns cloned `Vec` — use `Arc` | P3 | LOW | — |
| 21 | `available_capabilities` cloned per request | P3 | LOW | — |
| 22 | Lanczos3 overkill for MCP image resize | P3 | LOW | — |
| 31 | GqlEmail clones address vectors on every field resolution | P3 | LOW | — |

### Category D — Testing (1 finding)

| # | Finding | P-tier | Complexity | Depends On |
|---|---------|--------|------------|------------|
| 7 | No test coverage for critical paths | P1 | HIGH | Findings 1, 2, 6 should land first (tests exercise fixed behaviour) |

### Category E — Code Quality (4 findings)

| # | Finding | P-tier | Complexity | Depends On |
|---|---------|--------|------------|------------|
| 23 | Stringly-typed IDs — no newtypes for email_id, mailbox_id, etc. | P3 | HIGH | Touches every module — sequence last or defer |
| 27 | `DefaultHasher` not stable across Rust versions | P3 | LOW | — |
| 28 | Stale `#[allow(unused_imports)]` | P3 | LOW | — |
| 29 | `tokio` with `full` features — only needs ~5 | P3 | LOW | — |

---

## Feature Landscape

### Table Stakes (Users Expect These)

For a hardening milestone, "table stakes" = fixes whose absence makes the tool feel unreliable or unsafe to any experienced user.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| HTTP 4xx errors surfaced as actionable messages (#1) | Any HTTP client that silently eats errors is broken | LOW | Two sites in jmap/mod.rs; pattern is a match arm catch-all for 400..500 |
| DAV client timeouts (#2) | A hung connection that never resolves is not acceptable | LOW | `Client::builder().timeout(Duration::from_secs(30))` — already done in JmapClient, copy that pattern |
| Path traversal defense in download (#3) | Server-supplied filenames must not write outside target dir | LOW | `Path::file_name()` strips leading path components; no canonicalize needed for filename-only strip |
| Structured JSON output from all error paths (#11) | Tool consumers (AI agents) parse stdout; eprintln+exit(1) is invisible to them | LOW | Replace five `process::exit(1)` callsites with `Output::error().print()` return |
| Secret redaction in Debug (#15) | Accidental `{:?}` of Config must not leak credentials to logs | LOW | Manual `impl Debug` — see pattern notes below |
| vCard/iCal injection escaping (#8, #9) | Input escaping is table-stakes for any serialiser that writes to a protocol wire format | LOW | Validate enum fields against allowed set; strip CRLF/semicolons from free-form params |
| Confirmation token security (#14) | A bypass-able PREVIEW token is security theatre | MEDIUM | Add per-session HMAC nonce — see nonce pattern notes below |

### Differentiators (What Makes This Hardening Genuinely Better)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Concurrent DAV fetching (#4) | Multi-calendar users see 3–10x faster list/search operations | MEDIUM | `futures::future::try_join_all` over per-book REPORT requests; collect per-book errors with tolerance |
| Targeted event lookup (#5) | Getting one event should not download entire history | MEDIUM | CalDAV REPORT with `<C:comp-filter name="VEVENT"><C:prop-filter name="UID">` |
| DAV client reuse in MCP (#6) | Avoids TLS handshake per tool call; matters for agent workflows with many operations | MEDIUM | Store `Arc<Mutex<CardDavClient>>` and `Arc<Mutex<CalDavClient>>` in schema data alongside JmapContext |
| wiremock integration tests (#7) | Makes the send/auth/JMAP path verifiable without a live server | HIGH | `MockServer::start().await` per test; stubs with `Mock::given(method("POST")).and(path("/.well-known/jmap"))` |
| Optional pdfium feature flag (#16) | Users who do not need document extraction get a 10–20MB smaller binary | MEDIUM | Cargo feature gate on `kreuzberg`; CI matrix updated to test both with and without |
| GraphQL depth + complexity limits (#24) | Protects the MCP server from runaway queries even in a personal context | LOW | `limit_depth(7).limit_complexity(200)` — two builder method calls, see defaults rationale below |

### Anti-Features (Do Not Build These in v1.2)

| Anti-Feature | Why Requested | Why Problematic | Alternative |
|--------------|---------------|-----------------|-------------|
| Newtyped IDs (#23) in this milestone | Type safety across modules is genuinely valuable | Touches every model struct, command handler, and GraphQL type — mid-milestone creates massive merge conflicts; high risk for low user-visible value | Schedule as first item in v1.3; do as standalone PR with no concurrent work |
| Full server-side confirmation token store | Stateless HMAC nonce is simpler and equally secure for a personal-use MCP server | Requires in-memory store with TTL eviction, complicates restarts and multiple sessions, adds state to an otherwise stateless GraphQL layer | Use HMAC-SHA256 with a per-process random nonce (stateless, unguessable, replay-limited by session lifetime) |
| Interactive stdin prompt for auth token (#10) | Fully interactive prompts are the most user-friendly UX | MCP server and scripted CI contexts have no TTY; breaking those is worse than the process-list exposure | Accept via env var `FASTMAIL_API_TOKEN` (already supported as override); add docs recommending `read -rs TOKEN && fastmail-cli auth "$TOKEN"` for shell use |

---

## Pattern Notes: How Each Class of Fix Works

### Secret Redaction (#15)

**The problem:** `Config`, `CoreConfig`, and `ContactsConfig` all derive `#[derive(Debug)]`. Any accidental `{:?}` in a log, test failure output, or tracing span leaks `api_token` and `app_password` in plaintext.

**Two acceptable approaches:**

1. **Manual impl (zero new dependencies — recommended for this milestone):**

   ```rust
   impl fmt::Debug for CoreConfig {
       fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
           f.debug_struct("CoreConfig")
               .field("api_token", &self.api_token.as_ref().map(|_| "[REDACTED]"))
               .finish()
       }
   }
   ```
   Output when formatted with `{:?}`: `CoreConfig { api_token: Some("[REDACTED]") }`

2. **`secrecy` crate newtype (`Secret<String>`):**
   - `Secret<String>` implements `Debug` via the `DebugSecret` trait
   - Confirmed output format: `Secret([REDACTED std::string::String])`
   - Adds `zeroize`-on-drop memory safety as a bonus
   - Requires adding `secrecy` to `Cargo.toml` and calling `.expose_secret()` at all callsites that currently call `.clone()` on the token
   - Higher effort; cross-cutting change

**Recommendation:** Manual impl for v1.2. The `secrecy` newtype is the better long-term design but is the same kind of cross-cutting refactor as newtyped IDs — schedule together in v1.3.

**Convention vs project-specific:** Manual redacted Debug is a well-known Rust pattern. The `secrecy` crate is the ecosystem standard for new projects. Both are valid; the choice here is scope management.

---

### GraphQL Depth/Complexity Limits (#24)

**Mechanism:** `async-graphql` `Schema::build()` supports `.limit_depth(n)` and `.limit_complexity(n)` as chained builder methods. Validation happens at parse time before execution — an over-limit query never partially executes.

**Recommended defaults for a personal-productivity API:**

- `limit_depth(7)` — The fastmail-cli schema has queries like `email { attachments { ... } }` and `event { attendees { ... } }`. Useful queries reach 4–5 nesting levels. 7 provides headroom without permitting runaway nesting. AWS AppSync allows up to 75; for a personal tool that is unnecessary.
- `limit_complexity(200)` — With default per-field complexity of 1, a full email list of 100 items with 5 fields each = 500 raw, but list resolvers should carry `#[graphql(complexity = "count * child_complexity")]` to properly weight list queries. Without custom complexity annotations yet, 200 is a reasonable flat cap for initial deployment. Raise it if legitimate AI agent queries hit the limit.

```rust
// src/mcp/graphql/mod.rs — in build_schema()
Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
    .data(JmapContext { client })
    .limit_depth(7)
    .limit_complexity(200)
    .finish()
```

**Convention:** No GraphQL implementation sets defaults automatically — this is always an explicit server-side decision. The fix is two method calls. Do it.

---

### Confirmation Token Nonce (#14)

**Current problem:** `confirmation_token()` uses `DefaultHasher` over the input parameters. Anyone who knows the mutation parameters can reproduce the token offline and submit it directly to CONFIRM, bypassing PREVIEW entirely.

**Three patterns, compared:**

| Pattern | How It Works | Security | Complexity | Best For |
|---------|-------------|----------|------------|---------|
| Server-side token store | Issue UUID, store in `HashMap<token, params>` with TTL | HIGH | HIGH | Multi-user, multi-session servers |
| TTL-bound HMAC | HMAC-SHA256(nonce + params + unix_timestamp); verify within window | HIGH | MEDIUM | Systems where replay across restarts matters |
| Per-process nonce HMAC | HMAC-SHA256(rand_nonce_at_startup + params); nonce lives in `Arc<[u8;32]>` | HIGH | LOW | Personal-use MCP server, single session |

**Recommendation for fastmail-cli:** Per-process nonce HMAC. The MCP server is a single-process, personal-use tool. A 32-byte random nonce generated at server startup and threaded into `JmapContext` (or a separate `NonceContext`) makes tokens unpredictable to anyone who doesn't have access to the running process memory, without requiring a token store, TTL logic, or timestamp parsing.

```rust
// At server startup:
use rand::RngCore;
let mut nonce = [0u8; 32];
rand::rngs::OsRng.fill_bytes(&mut nonce);
let nonce = Arc::new(nonce);

// Token computation:
fn confirmation_token(nonce: &[u8], parts: &[&str]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(nonce)
        .expect("HMAC accepts any key length");
    for part in parts {
        mac.update(part.as_bytes());
    }
    hex::encode(mac.finalize().into_bytes())
}
```

**New dependencies needed:** `hmac`, `sha2`, `hex` (all from RustCrypto, `no_std`-compatible, minimal footprint). `rand` with `getrandom` feature for `OsRng`.

**Convention:** HMAC-based stateless tokens are the standard approach for CSRF tokens and one-time confirmation patterns in API design. Server-side stores are appropriate for multi-user systems; they are overkill here.

---

### Path Traversal Defense (#3)

**The problem:** `download.rs:112` constructs the output path as `Path::new(out_dir).join(&final_filename)` where `final_filename` comes from the email attachment name (server-provided). A name like `../../../.bashrc` writes outside the output directory.

**Three approaches:**

| Approach | Code | Covers | Does Not Cover |
|----------|------|--------|---------------|
| Strip with `file_name()` | `Path::new(name).file_name()` | `../` traversal, absolute paths like `/etc/passwd` | Symlinks in target dir (irrelevant for this use case) |
| Normalize + prefix check | `canonicalize(base.join(name))` then `starts_with(base)` | All traversal including symlinks | Requires target dir to exist at call time |
| Whitelist characters | Regex `[a-zA-Z0-9._-]+` | All traversal | Breaks filenames with spaces, unicode, accented characters |

**Recommendation:** `Path::file_name()` strip approach. It is the fix already specified in the codebase review. It requires no I/O, no regex, handles all `..` and absolute path forms, and preserves legitimate filenames including spaces and unicode. Use a concrete fallback name rather than `unwrap_or_default()`:

```rust
let safe_name = Path::new(&final_filename)
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("attachment");
let path = Path::new(out_dir).join(safe_name);
```

**Convention:** Strip with `file_name()` is the idiomatic Rust recommendation from OWASP and the Rust security community for attachment download patterns. Normalize+prefix-check is the correct approach when the full path (not just the filename) is user-controlled.

---

### Integration Testing Patterns (#7)

**The gap:** Zero tests cover the email send path, authentication flow, JMAP HTTP request/response cycle, GraphQL resolvers, CalDAV HTTP interaction, or any HTTP error handling paths.

**Recommended approach: wiremock-rs black-box tests.**

wiremock-rs (current version 0.6) starts a real HTTP server on a random local port. Tests point `reqwest::Client` at `mock_server.uri()`. No mocking of internal Rust types — tests exercise the actual `JmapClient::request()` code path end-to-end.

**Core pattern:**
```rust
#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path, header, body_json_schema};

    #[tokio::test]
    async fn test_send_email_success() {
        let mock_server = MockServer::start().await;
        
        // Stub the JMAP session endpoint
        Mock::given(method("GET"))
            .and(path("/.well-known/jmap"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(session_fixture()))
            .mount(&mock_server)
            .await;

        // Stub the JMAP API call
        Mock::given(method("POST"))
            .and(path("/jmap/api/"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(send_email_response_fixture()))
            .expect(1)   // verify exactly one API call was made
            .mount(&mock_server)
            .await;

        // Point JmapClient at mock server URL, invoke, assert
    }
}
```

**Three test sub-types needed:**

| Sub-type | What to Test | How to Implement |
|----------|-------------|-----------------|
| Happy path | send, auth, list emails, get email | Mock stubs for JMAP session + method responses |
| HTTP error paths | 401 triggers NotAuthenticated, 429 triggers retry/error, 403/400 trigger Server error (fixing #1) | `ResponseTemplate::new(status_code)` per case |
| DAV error tolerance | One address book returns 500 while others succeed (fixing #19) | Per-path stubs with different status codes; assert partial results returned |

**Contract tests, not snapshot tests:** Assert on typed struct fields after deserialization, not on raw JSON bytes. JMAP response field order varies by server version. Snapshot tests (comparing exact JSON strings) are brittle for protocol responses. Assert structure and values.

**Expectations:** `Mock` objects support `.expect(n)` — this fires a panic during `MockServer` drop if the stub was not called the expected number of times. Use this to verify that the code actually made the HTTP call, not just that it returned without error.

**Convention:** wiremock-rs is the ecosystem-standard tool for Rust HTTP client testing (used in Zero To Production in Rust, recommended in multiple Rust testing guides). The alternative `httpmock` crate exists but is less actively maintained. Use wiremock-rs.

---

## Feature Dependencies

```
[#14 HMAC nonce token]
    └──enables──> [#25 mark_as_spam confirmation token]
                  (same token function used; add nonce before wiring new mutation)

[#1 HTTP 4xx handling]
[#2 DAV timeouts]
[#6 DAV client reuse]
    └──should precede──> [#7 integration tests]
                         (tests exercise the fixed behaviour; write tests after fixes land)

[#4 Concurrent DAV fetching]
[#19 Server-side contact search filter]
    └──independent, but both live in carddav/mod.rs
    └──batch in same PR to avoid repeated file churn]

[#5 Targeted event lookup]
[#4 Concurrent calendar fetching]
    └──independent, but both live in caldav/mod.rs
    └──batch in same PR]

[#26 MCP mutex across async I/O]
    └──depends on──> [#6 DAV client reuse]
                     (mutex scope fix is entangled with how DAV clients are stored)

[#23 Newtyped IDs]
    └──touches──> ALL model structs, command handlers, GraphQL types
    └──DEFER outside v1.2 scope — schedule first in v1.3]
```

### Dependency Notes

- **#25 requires #14 first:** `mark_as_spam` confirmation token reuses the same `confirmation_token()` function. Adding the mutation before the nonce is in place gives it the same insecure hash token. Fix the function first.
- **#7 test coverage should follow #1/#2/#6:** Tests that verify HTTP error handling (#1) and timeout behavior (#2) need those behaviors to exist. DAV client reuse (#6) changes how the test fixture initializes the shared client.
- **#26 depends on #6:** The mutex-across-async-I/O problem (#26) is entangled with how DAV clients are stored. If the DAV clients are moved into `Arc<Mutex<>>` context (#6), the mutex scope fix comes as part of that same change.
- **#23 newtyped IDs is deferred:** This is a cross-module refactor that should not run concurrently with any other module-touching work. It belongs in v1.3 as the first PR, before any other feature work begins.
- **#16 optional feature flag:** Makes the `kreuzberg`/pdfium dep conditional; affects how the download command compiles. Plan for a documentation update and CI matrix change (`cargo test --no-default-features` added) in the same PR.

---

## MVP Definition

### Phase 1 — Security + Stability Quick Wins (all LOW complexity, ship first)
- [ ] Path traversal defense (#3) — single-expression fix
- [ ] DAV client timeouts (#2) — two-line fix, prevents indefinite hangs
- [ ] HTTP 4xx handling (#1) — one match arm, clears confusing deserialization errors
- [ ] Secret Debug redaction (#15) — manual impl on three structs, zero deps
- [ ] vCard/iCal injection escaping (#8, #9) — strip CRLF and validate enum values
- [ ] process::exit(1) output contract (#11) — five callsite replacements
- [ ] Blob URL encoding (#30) — percent-encode URL template values

### Phase 2 — Performance Wins (MEDIUM complexity, network-facing)
- [ ] Concurrent DAV fetching (#4) — `try_join_all` over per-book fetches, with per-book error tolerance
- [ ] Targeted event lookup (#5) — CalDAV REPORT with UID filter
- [ ] DAV client reuse in MCP (#6) — store in schema data alongside JmapClient

### Phase 3 — Security Hardening (MEDIUM complexity, new dependencies)
- [ ] HMAC confirmation token nonce (#14) — per-process nonce, hmac+sha2+hex crates
- [ ] mark_as_spam confirmation token (#25) — wire after #14 nonce exists
- [ ] Auth token input hardening (#10) — document env var approach; no TTY changes needed

### Phase 4 — Integration Tests (HIGH complexity, ongoing)
- [ ] wiremock integration tests for JMAP send/auth/error paths (#7)
- [ ] CalDAV HTTP interaction tests — per-book error tolerance (#19 tests)
- [ ] GraphQL resolver test coverage (#7 extension)

### Phase 5 — Polish (LOW complexity, quality and binary size)
- [ ] GraphQL depth/complexity limits (#24) — two builder method calls in build_schema()
- [ ] DefaultHasher stability fix (#27) — replace with FNV or xxHash
- [ ] Optional pdfium feature flag (#16)
- [ ] let-else refactor in download.rs (#18) — also fixes the unwrap pattern
- [ ] Remaining P3 polish: #20, #21, #22, #26, #28, #29, #31, #32, #33

---

## Feature Prioritization Matrix

| Finding | User/Operator Value | Implementation Cost | Priority |
|---------|--------------------|--------------------|---------|
| #3 Path traversal | HIGH — data safety | LOW | P1 |
| #2 DAV timeouts | HIGH — availability | LOW | P1 |
| #1 HTTP 4xx handling | HIGH — debuggability | LOW | P1 |
| #11 exit(1) output contract | HIGH — tool consumers | LOW | P1 |
| #15 Secret Debug redaction | HIGH — log hygiene | LOW | P1 |
| #8/#9 Injection escaping | HIGH — data correctness | LOW | P1 |
| #4 Concurrent DAV fetch | HIGH — latency | MEDIUM | P1 |
| #5 Targeted event lookup | HIGH — latency and memory | MEDIUM | P1 |
| #6 DAV client reuse | MEDIUM — MCP per-call latency | MEDIUM | P2 |
| #14 HMAC nonce token | HIGH — security | MEDIUM | P2 |
| #7 Integration tests | HIGH — long-term stability | HIGH | P2 |
| #25 spam confirmation | MEDIUM — consistency with other mutations | LOW (after #14) | P2 |
| #19 server-side contact filter | MEDIUM — scalability for large accounts | MEDIUM | P2 |
| #16 optional pdfium | MEDIUM — binary size | MEDIUM | P2 |
| #10 auth token input | MEDIUM — multi-user safety | MEDIUM | P2 |
| #17 MCP signal handling | MEDIUM — clean shutdown | MEDIUM | P2 |
| #18 let-else unwrap refactor | LOW — latent panic risk | LOW | P2 |
| #30 URL encoding | MEDIUM — correctness | LOW | P2 |
| #24 GraphQL limits | LOW — internal personal API | LOW | P3 |
| #12 Bytes over Vec | LOW — memory efficiency | LOW | P3 |
| #13 owned JSON parse | LOW — memory efficiency | LOW | P3 |
| #26 mutex scope | MEDIUM — throughput | MEDIUM | P3 (after #6) |
| #20 Arc mailbox cache | LOW | LOW | P3 |
| #21 capabilities clone | LOW | LOW | P3 |
| #22 resize filter | LOW | LOW | P3 |
| #27 DefaultHasher stability | LOW | LOW | P3 |
| #28 stale allow | LOW | LOW | P3 |
| #29 tokio features | LOW | LOW | P3 |
| #31 GqlEmail clone | LOW | LOW | P3 |
| #32 expect in JmapClient::new() | LOW | LOW | P3 |
| #33 config error message | LOW | LOW | P3 |
| #23 Newtyped IDs | HIGH — type safety | HIGH | DEFER to v1.3 |

---

## Sources

- [async-graphql depth and complexity docs](https://async-graphql.github.io/async-graphql/en/depth_and_complexity.html) — HIGH confidence; `.limit_depth()` and `.limit_complexity()` methods confirmed
- [secrecy crate source — Debug impl](https://docs.rs/secrecy/0.8.0/src/secrecy/lib.rs.html) — HIGH confidence; confirmed output format `Secret([REDACTED std::string::String])`
- [wiremock-rs README](https://github.com/LukeMathWalker/wiremock-rs) — HIGH confidence; v0.6 current, `MockServer::start().await` pattern confirmed
- [RustCrypto MACs — hmac crate](https://github.com/RustCrypto/MACs) — HIGH confidence; HMAC-SHA256 pattern for stateless tokens
- [AWS AppSync GraphQL limits](https://docs.aws.amazon.com/appsync/latest/devguide/configuration-limits.html) — MEDIUM confidence; used for benchmarking defaults (AppSync max is 75 for depth)
- [Rust path traversal guide — StackHawk](https://www.stackhawk.com/blog/rust-path-traversal-guide-example-and-prevention/) — MEDIUM confidence; corroborated by std::path::Path docs
- [Leapcell: Secure config with secrecy crate](https://leapcell.io/blog/secure-configuration-and-secrets-management-in-rust-with-secrecy-and-environment-variables) — MEDIUM confidence
- CODEBASE-REVIEW.md (2026-04-04) — source of all 33 findings; HIGH confidence

---

*Feature research for: fastmail-cli v1.2 Hardening & Quality*
*Researched: 2026-04-04*
