# Architecture Research

**Domain:** Rust CLI + MCP server — Fastmail integration (v1.2 Hardening & Quality)
**Researched:** 2026-04-04
**Confidence:** HIGH — based on direct codebase analysis of all 10 source files

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────┐
│  Entry Points                                                     │
│  ┌──────────────────┐      ┌────────────────────────────────┐    │
│  │  src/main.rs     │      │  src/mcp/mod.rs (FastmailMcp)  │    │
│  │  CLI (clap)      │      │  MCP server (rmcp + stdio)     │    │
│  └──────┬───────────┘      └──────────────┬─────────────────┘    │
│         │                                 │                       │
│         │                  ┌──────────────▼─────────────────┐    │
│         │                  │  src/mcp/graphql/              │    │
│         │                  │  Schema + Query + Mutation      │    │
│         │                  │  Arc<FastmailSchema>            │    │
│         │                  └──────────────┬─────────────────┘    │
├─────────▼─────────────────────────────────▼────────────────────┤
│  Command Handlers  src/commands/                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐            │
│  │ send.rs  │ │contacts  │ │ events   │ │download  │  …         │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘            │
├───────▼─────────────▼───────────▼─────────────▼─────────────────┤
│  Protocol Clients                                                 │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────┐  │
│  │  src/jmap/       │  │  src/carddav/    │  │  src/caldav/   │  │
│  │  JmapClient      │  │  CardDavClient   │  │  CalDavClient  │  │
│  │  Arc<Mutex<>>    │  │  per-req (v1.1)  │  │  per-req (v1.1)│  │
│  └──────────────────┘  └──────────────────┘  └────────────────┘  │
├──────────────────────────────────────────────────────────────────┤
│  Cross-Cutting                                                    │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐    │
│  │ src/config │ │ src/error  │ │ src/models │ │ src/util   │    │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘    │
└──────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities (current state, pre-v1.2)

| Component | Responsibility | Current State |
|-----------|----------------|---------------|
| `src/main.rs` | CLI entry, clap parse, dispatch | `process::exit` misuse (#11); token as positional arg (#10) |
| `src/commands/` | Per-command handlers | `unwrap()` fragility (#18); exit misuse (#11) |
| `src/jmap/mod.rs` | JMAP HTTP, session + mailbox cache | Shared `Arc<Mutex<>>`; 4xx gap (#1); allocation waste (#12, #13, #20, #21) |
| `src/carddav/mod.rs` | CardDAV HTTP, vCard parse/serialize | No timeout (#2); injection (#8); sequential (#4); per-req recreation (#6); `DefaultHasher` (#27) |
| `src/caldav/mod.rs` | CalDAV HTTP, iCal parse/serialize | No timeout (#2); injection (#9); sequential (#4); full-scan (#5); per-req recreation (#6) |
| `src/mcp/mod.rs` | rmcp server, routes to schema | No signal handling (#17) |
| `src/mcp/graphql/` | Schema, query/mutation resolvers, types | No depth limits (#24); stateless nonce (#14); DAV recreation (#6); mutex over I/O (#26) |
| `src/config.rs` | Config load/save, env var fallback | `Debug` exposes secrets (#15) |
| `src/error.rs` | Thiserror error enum | Missing 4xx variant (#1) |
| `src/models/mod.rs` | JMAP domain types, `Output` wrapper | Stringly-typed IDs (#23); address clone (#31) |
| `src/util.rs` | Address parse, text/image extract | `kreuzberg` always compiled (#16); overkill resize filter (#22) |

---

## Recommended Project Structure (after v1.2)

```
src/
├── ids.rs              # NEW — newtyped EmailId, MailboxId, ThreadId, AccountId
├── nonce.rs            # NEW — NonceStore for MCP confirmation token lifecycle
├── http.rs             # NEW (optional) — shared reqwest::Client factory
├── main.rs             # CLI entry, auth via stdin (#10), Output::error exits (#11)
├── error.rs            # add HttpClient(u16) variant for 4xx (#1)
├── config.rs           # redact secrets in Debug (#15)
├── models/mod.rs       # import ids.rs types, Arc<Vec> for addresses (#31)
├── util.rs             # feature-gate kreuzberg (#16), Triangle filter (#22)
├── jmap/mod.rs         # 4xx catch (#1), Bytes return (#12), owned parse (#13)
├── carddav/mod.rs      # timeout (#2), join_all (#4), server-filter (#19), escape (#8)
├── caldav/mod.rs       # timeout (#2), join_all (#4), UID REPORT (#5), escape (#9)
├── commands/
│   ├── download.rs     # Path::file_name() traversal guard (#3), let-else (#18)
│   └── …              # Output::error exits (#11)
└── mcp/
    ├── mod.rs          # tokio::select! for shutdown (#17)
    └── graphql/
        ├── mod.rs      # AppContext replaces JmapContext (#6), depth limits (#24)
        ├── query.rs    # use AppContext DAV clients, release mutex before await (#26)
        ├── mutation.rs # nonce tokens (#14), mark_as_spam gate (#25)
        └── types.rs    # remove stateless confirmation_token(), stable hash (#27)

tests/
├── common/
│   └── mod.rs          # wiremock server setup, fixture helpers
├── jmap_client.rs      # JmapClient HTTP layer: auth, 4xx, 429, 5xx
├── send_email.rs       # send_email happy path, draft, missing identity
├── auth_flow.rs        # auth command, token storage
├── carddav_client.rs   # CardDavClient CRUD, timeout, 4xx
└── caldav_client.rs    # CalDavClient CRUD, concurrent fetch, UID lookup
```

### Structure Rationale

- **`src/ids.rs`:** Cross-cutting type-safety concern; separate from domain structs in `models/`. Used by `jmap/`, `carddav/`, `caldav/`, `mcp/graphql/` — a dedicated module avoids circular import risk.
- **`src/nonce.rs`:** MCP-specific lifecycle management. Keeps token issuance logic out of GraphQL resolver files.
- **`tests/`:** Wiremock tests are Cargo integration tests (separate binary, external to modules). Must live here per Rust convention.

---

## Architectural Patterns

### Pattern 1: AppContext — Unified Schema Data Injection

**What:** Replace the existing `JmapContext` struct (which holds only the JMAP client) with a broader `AppContext` that holds all three shared protocol clients and the nonce store.

**When to use:** Any time a new MCP-layer dependency needs to be shared across request lifecycles.

**Trade-offs:** Adds one struct to manage; eliminates per-request client construction for DAV calls. The `async-graphql` `.data()` mechanism is the correct DI container for schema-level state — do not fight it.

```rust
// src/mcp/graphql/mod.rs
pub struct AppContext {
    pub jmap:        Option<Arc<Mutex<JmapClient>>>,
    pub carddav:     Option<Arc<Mutex<CardDavClient>>>,
    pub caldav:      Option<Arc<Mutex<CalDavClient>>>,
    pub nonce_store: Arc<NonceStore>,
}

pub fn build_schema(ctx: AppContext) -> FastmailSchema {
    Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
        .limit_depth(5)
        .limit_complexity(200)
        .data(ctx)
        .finish()
}
```

Construct all three clients once in `FastmailMcp::new()`, mirroring the existing `JmapClient` construction pattern. For CLI commands that build DAV clients per-invocation, no change is needed — single-shot process runs don't benefit from client pooling.

### Pattern 2: Mutex-Narrow Locking — Release Before Async I/O

**What:** Clone the `Arc`, acquire the lock, extract what you need (credentials, cached data), drop the guard, then perform the async HTTP operation without holding the lock.

**When to use:** Every async resolver that currently holds `Arc<Mutex<Client>>.lock().await` across `.send().await`.

**Trade-offs:** Requires the DAV clients to own their credentials (they already do: `username`, `app_password` fields). No data races: each request gets its own call stack; HTTP is stateless at the protocol level.

```rust
// Pattern: clone Arc, lock briefly for state, drop guard before I/O
async fn contacts(&self, ctx: &Context<'_>, query: String) -> Result<Vec<GqlContact>> {
    let client_arc = require_carddav_client(ctx)?;
    // Lock only to clone the client data needed, then release
    let contacts = {
        let client = client_arc.lock().await;
        client.search_contacts(&query).await?
        // MutexGuard drops here before any further awaits
    };
    Ok(contacts.into_iter().map(Into::into).collect())
}
```

For read-heavy operations like listing mailboxes or calendars, consider `Arc<RwLock<>>` to allow concurrent reads without serialization.

### Pattern 3: Feature-Gated Heavy Dependencies

**What:** Make `kreuzberg` an optional Cargo feature, with a stable function signature that compiles to a no-op stub when the feature is disabled.

**When to use:** Any dependency that is only needed for one subcommand and inflates binary size significantly.

**Trade-offs:** Users must know to pass `--features document-extraction` for the full build. Default can include it (preserving current behavior) while allowing opt-out builds.

```toml
# Cargo.toml
[features]
default = ["document-extraction"]
document-extraction = ["kreuzberg"]

[dependencies]
kreuzberg = { version = "4.4", features = ["bundled-pdfium"], optional = true }
```

```rust
// src/util.rs — both branches have identical public signatures
#[cfg(feature = "document-extraction")]
pub async fn extract_text(bytes: &[u8], filename: &str) -> anyhow::Result<Option<String>> {
    use kreuzberg::{ExtractionConfig, extract_bytes};
    // ... existing body unchanged
}

#[cfg(not(feature = "document-extraction"))]
pub async fn extract_text(_bytes: &[u8], _filename: &str) -> anyhow::Result<Option<String>> {
    Ok(None)
}
```

All call sites in `src/commands/download.rs` remain unchanged.

---

## Six Architectural Questions — Direct Answers

### Q1: Shared DAV Client Pool — Where Does It Live?

**Answer: `AppContext` inside `src/mcp/graphql/mod.rs` — not `OnceCell`, not a new top-level `AppState`.**

Rationale: The GraphQL schema already uses `.data(JmapContext { client })` as the DI mechanism. `async-graphql` context data is the correct container for schema-level shared state. A module-level `OnceCell<CardDavClient>` in `src/carddav/mod.rs` would mix protocol and lifecycle concerns — the DAV modules should own construction, not lifecycle. A separate `AppState` struct in the crate root adds an indirection layer with no benefit.

The shared `reqwest::Client` inside each DAV client provides connection-pool reuse. A single `Client::builder().timeout(Duration::from_secs(30)).build()` call (or a factory function in `src/http.rs`) should supply the same `reqwest::Client` instance to `CardDavClient::new()` and `CalDavClient::new()`. This also fixes finding #2 (no timeout) as a side effect.

### Q2: Newtyped IDs — `models/` or `src/ids.rs`?

**Answer: `src/ids.rs`, a new dedicated module.**

`src/models/mod.rs` owns domain structs tied to JMAP serialization. ID newtypes are a type-safety concern that cuts across `jmap/`, `carddav/`, `caldav/`, `models/`, and MCP types. Colocating them in `models/` entangles serialization and type-safety. A dedicated `src/ids.rs` follows the single-concern module convention already established by `src/error.rs`, `src/config.rs`, `src/util.rs`.

Implement `Deref<Target = str>`, `AsRef<str>`, and `Display` on each newtype to minimize call-site rewrites. The migration should be mechanical — clippy will surface every site that needs updating after the struct definitions land.

Finding #23 is the highest-ripple change in the entire review. Phase it last in a dedicated quality pass after all security and stability fixes land, to avoid conflicts.

### Q3: wiremock Integration Tests — `tests/` or Colocated?

**Answer: `tests/` top-level directory, organized by layer.**

Wiremock tests are Cargo integration tests (separate compiled binaries). They belong in `tests/` by Rust convention and cannot be placed in `#[cfg(test)]` module blocks at the source level. Colocated `#[cfg(test)]` blocks remain appropriate for the 103 existing unit tests that test internal module logic.

Add `wiremock` to `[dev-dependencies]` only. Create `tests/common/mod.rs` first as the shared mock-server scaffold, then layer the domain-specific test files on top. The wiremock test for `JmapClient::request()` is the right place to exercise the 4xx handling fix from finding #1 — findings #1 and #7 should be worked in the same phase.

### Q4: GraphQL Depth Limits — Schema Builder or Wrapper?

**Answer: Schema builder — `async-graphql` provides first-class `limit_depth` and `limit_complexity` methods.**

Call them in `build_schema()` in `src/mcp/graphql/mod.rs`:

```rust
Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
    .limit_depth(5)
    .limit_complexity(200)
    .data(ctx)
    .finish()
```

Do not wrap `schema.execute()` in `FastmailMcp::graphql()` with manual AST inspection. That duplicates logic the framework already provides, puts the constraint far from the schema definition, and is easy to accidentally remove. The depth of 5 is conservative but generous for this schema (deepest legitimate query is approximately `email → attachments → content → text`, which is 4 levels).

### Q5: Per-Session Confirmation Nonces — Where Stored?

**Answer: `NonceStore` in `src/nonce.rs`, referenced via `AppContext`.**

The current `confirmation_token()` in `src/mcp/graphql/types.rs` is a pure hash of input parameters with no server secret. Anyone who knows the inputs can bypass the PREVIEW gate (finding #14). The fix requires server-side state. The MCP server process is already long-running and stateful — process memory is the right place.

Structure:

```rust
// src/nonce.rs
use std::collections::HashSet;
use tokio::sync::Mutex;

pub struct NonceStore {
    issued: Mutex<HashSet<String>>,
}

impl NonceStore {
    pub fn new() -> Self {
        Self { issued: Mutex::new(HashSet::new()) }
    }

    pub async fn issue(&self, token: String) {
        self.issued.lock().await.insert(token);
    }

    /// Returns true and removes the token if it was previously issued.
    pub async fn consume(&self, token: &str) -> bool {
        self.issued.lock().await.remove(token)
    }
}
```

Add `nonce_store: Arc<NonceStore>` to `AppContext`. In PREVIEW mutations, generate `uuid::Uuid::new_v4().to_string()` and call `nonce_store.issue(token).await`. In CONFIRM mutations, call `nonce_store.consume(token).await` and reject if it returns `false` (token never issued or already consumed — single-use).

`NonceStore` uses its own `Mutex` — do not reuse the client `Arc<Mutex<>>`. Tokens live in process memory; they are invalidated on MCP server restart, which is the correct behavior.

### Q6: kreuzberg Feature Flag — Does It Reshape `util.rs`?

**Answer: Minimal reshaping — `#[cfg(feature)]` attributes at the function level, no module split required.**

`util.rs` already has the right logical structure (address parsing at top, extraction/image below). Adding `#[cfg(feature = "document-extraction")]` guards around `extract_text` and its `use kreuzberg::*` import is sufficient. All callers continue to compile unchanged because both the featured and non-featured versions have identical public signatures.

Do not split `util.rs` into a submodule. The `image` crate is unconditional — it serves MCP image resize which is core functionality regardless of document extraction preference.

---

## New Components Required for v1.2

| Component | File | Purpose | Triggered By |
|-----------|------|---------|--------------|
| `AppContext` struct | `src/mcp/graphql/mod.rs` | Replace `JmapContext`; hold all three shared clients + nonce store | #6, #14, #24 |
| `NonceStore` | `src/nonce.rs` | Per-session single-use confirmation token lifecycle | #14 |
| `src/ids.rs` | `src/ids.rs` | Newtyped `EmailId`, `MailboxId`, `ThreadId`, `AccountId` | #23 |
| `tests/common/mod.rs` | `tests/common/mod.rs` | Shared wiremock server scaffold and fixture helpers | #7 |
| `tests/jmap_client.rs` | `tests/jmap_client.rs` | JMAP HTTP layer integration tests | #7 |
| `tests/send_email.rs` | `tests/send_email.rs` | Email send and auth flow integration tests | #7 |
| `tests/carddav_client.rs` | `tests/carddav_client.rs` | CardDAV CRUD + timeout + error path tests | #7 |
| `tests/caldav_client.rs` | `tests/caldav_client.rs` | CalDAV CRUD + concurrent fetch + UID lookup tests | #7 |
| `src/http.rs` (optional) | `src/http.rs` | Shared `reqwest::Client` factory for timeout-configured clients | #2, #6 |

---

## Modified Components for v1.2

| Component | Finding(s) | Change Summary |
|-----------|-----------|----------------|
| `src/jmap/mod.rs` | #1, #12, #13, #20, #21, #30, #32 | 4xx catch-all arm; `Bytes` blob return; owned `Value` in `parse_response`; `Arc<Vec<Mailbox>>` cache; `Arc<Vec<String>>` capabilities; URL-encode blob template; `Result` from `new()` |
| `src/carddav/mod.rs` | #2, #4, #8, #19, #27, #28 | Timeout in constructor; `join_all` multi-book fetch; escape EMAIL/TEL injection fields; server-side REPORT text-match; stable hash; remove stale `#[allow(unused_imports)]` |
| `src/caldav/mod.rs` | #2, #4, #5, #9 | Timeout in constructor; `join_all` multi-calendar fetch; UID-targeted CalDAV REPORT; validate/escape attendee and RRULE fields |
| `src/config.rs` | #15, #33 | Manual `Debug` impl redacting `api_token` and `app_password`; recovery guidance in parse-error message |
| `src/error.rs` | #1 | Add catch-all 4xx variant or extend `Server` variant for HTTP status codes |
| `src/util.rs` | #16, #22 | `#[cfg(feature = "document-extraction")]` on `extract_text`; `FilterType::Triangle` instead of `Lanczos3` |
| `src/main.rs` | #10, #11 | Accept auth token via stdin/prompt instead of positional argument; replace all `eprintln!+process::exit(1)` with `Output::<()>::error("...").print()` |
| `src/commands/download.rs` | #3, #18 | `Path::file_name()` before joining output path; `let Some(x) = … else { return }` patterns |
| `src/mcp/mod.rs` | #17 | `tokio::select!` on `server.waiting()` vs `tokio::signal::ctrl_c()` |
| `src/mcp/graphql/mod.rs` | #6, #14, #24 | `AppContext` replacing `JmapContext`; `.limit_depth(5).limit_complexity(200)` on schema builder |
| `src/mcp/graphql/query.rs` | #6, #26 | Read DAV clients from `AppContext`; drop `MutexGuard` before `await` in all resolvers |
| `src/mcp/graphql/mutation.rs` | #14, #25 | Use `NonceStore` for all confirmation gates; add PREVIEW/CONFIRM gate to `mark_as_spam` |
| `src/mcp/graphql/types.rs` | #14, #27, #31 | Remove stateless `confirmation_token()`; replace `DefaultHasher` with stable hasher for contact ID fallbacks; `Arc<Vec<GqlEmailAddress>>` for address fields |
| `src/models/mod.rs` | #23, #31 | Use `EmailId`/`MailboxId` from `src/ids.rs`; `Arc<Vec>` for address fields |
| `Cargo.toml` | #16, #29 | `kreuzberg` as optional dependency; trim `tokio` from `full` to the five features actually used (`rt`, `rt-multi-thread`, `macros`, `sync`, `io-util`) |

---

## Findings That Ripple Across Many Modules — Flag for Early or Isolated Phasing

### Must Phase First (unblocks all subsequent work)

**Finding #6 + AppContext refactor** — touches `src/mcp/graphql/mod.rs`, `query.rs`, `mutation.rs`, `src/mcp/mod.rs`. Every subsequent DAV-touching change in the MCP layer will conflict with any in-flight AppContext work. Land this as the first MCP-layer commit.

**Finding #11 (process::exit)** — touches `src/main.rs` at 5+ confirmed call sites. Wide but mechanical: `eprintln!+exit` becomes `Output::error().print()` then `return`. Do it as a single atomic commit so any later rebases are clean.

### Must Phase After AppContext (depends on AppContext)

**Finding #14 + NonceStore** — `NonceStore` lives in `AppContext`. Cannot land before AppContext refactor. Touches `src/nonce.rs` (new), `src/mcp/graphql/types.rs`, and `mutation.rs` at multiple mutation resolvers.

### Must Phase Last (highest ripple, independent of other fixes)

**Finding #23 (newtyped IDs)** — This is the broadest mechanical change in the review. It touches `src/ids.rs` (new), `src/models/mod.rs`, `src/jmap/mod.rs` (every method accepting `&str` IDs), `src/mcp/graphql/types.rs`, `query.rs`, `mutation.rs`, and all command handler files. It is completely independent of security and stability fixes. Phase it after all other work is merged to avoid merge conflicts in heavily-edited files. Implement `Deref<Target = str>` on each newtype to reduce call-site blast radius.

### Phase Together (logical coupling)

**Findings #4 + #5** — Concurrent fetching and UID-targeted REPORT both modify `src/caldav/mod.rs` inner fetch loops. UID REPORT reduces the number of joins needed; they interact. Do in one pass.

**Findings #8 + #9** — vCard injection and iCal injection use the same escape/validate pattern applied to different modules. Implement together to share the design decision.

**Findings #1 + #7** — The wiremock tests for JMAP are the test harness for the 4xx fix. Write fix and test in the same phase.

---

## Recommended Build Order

```
Phase 1 — Foundation Safety (unblocks all layers)
  #2   DAV timeouts in carddav + caldav constructors
  #3   Path traversal guard in download.rs
  #1   4xx HTTP catch-all in jmap/mod.rs + error.rs
  #11  Output contract fix: all process::exit → Output::error in main.rs
  #15  Config Debug secret redaction in config.rs
  #32  JmapClient::new() → Result<Self> instead of expect()

Phase 2 — Security Hardening
  #8   vCard injection escaping in carddav/mod.rs
  #9   iCal injection validation in caldav/mod.rs
  #10  Auth token via stdin/prompt in main.rs
  #30  URL-encode blob download template in jmap/mod.rs

Phase 3 — MCP Layer Refactor (land atomically; high-ripple)
  #6   AppContext with shared DAV clients (mcp/graphql/mod.rs, query.rs, mutation.rs)
  #24  GraphQL depth + complexity limits (mcp/graphql/mod.rs)
  #14  NonceStore + nonce-bound confirmation tokens (nonce.rs + types.rs + mutation.rs)
  #25  mark_as_spam confirmation gate (mutation.rs)
  #17  MCP signal handling (mcp/mod.rs)
  #26  Release MutexGuard before async I/O (query.rs)

Phase 4 — Performance
  #4   Concurrent DAV fetching via join_all (carddav, caldav)
  #5   Targeted CalDAV UID REPORT (caldav/mod.rs)
  #12  Return bytes::Bytes from blob download (jmap/mod.rs)
  #13  Owned Value in parse_response (jmap/mod.rs)
  #19  Server-side CardDAV text-match filter + per-book error tolerance
  #20  Arc<Vec<Mailbox>> mailbox cache (jmap/mod.rs)
  #21  Arc<Vec<String>> capabilities (jmap/mod.rs)
  #31  Arc<Vec<GqlEmailAddress>> in GqlEmail (types.rs)

Phase 5 — Integration Test Coverage
  Add wiremock to [dev-dependencies]
  tests/common/ scaffold
  #7   JMAP + send + auth + CardDAV + CalDAV integration tests

Phase 6 — Quality Polish
  #16  kreuzberg feature flag (Cargo.toml + util.rs)
  #22  Triangle resize filter (util.rs)
  #27  Stable hash for contact IDs (carddav/mod.rs)
  #28  Remove stale #[allow(unused_imports)] (carddav/mod.rs)
  #29  Trim tokio features (Cargo.toml)
  #33  Config parse error recovery message (config.rs)
  #18  let-else patterns in download.rs

Phase 7 — Newtype IDs (isolated, broad-but-mechanical)
  #23  src/ids.rs + update all call sites
```

---

## Data Flow

### Request Flow (MCP, target state after v1.2)

```
MCP Host (AI agent)
    │ JSON-RPC tool call
    ▼
FastmailMcp::graphql()            [src/mcp/mod.rs]
    │ build async_graphql::Request
    ▼
FastmailSchema::execute()         [async-graphql runtime]
    │ depth check (limit_depth: 5)     — new in v1.2
    │ complexity check (limit: 200)    — new in v1.2
    │ dispatch to resolver
    ▼
QueryRoot / MutationRoot          [src/mcp/graphql/query.rs | mutation.rs]
    │ ctx.data::<AppContext>()         — was JmapContext
    │ clone Arc<Mutex<CalDavClient>>   — was per-request construction
    │ lock briefly for state
    │ drop MutexGuard                  — before await (fixes #26)
    ▼
CalDavClient::list_events()       [src/caldav/mod.rs]
    │ shared reqwest::Client           — connection pool reuse
    │ 30s timeout enforced             — fixes #2
    │ join_all across calendars        — fixes #4
    │ UID-targeted REPORT              — fixes #5
    ▼
Fastmail CalDAV server
    │ HTTP 207 Multi-Status (or 4xx — now surfaced as clear error)
    ▼
CalDavClient parses iCal response
    ▼
Resolver maps to GqlCalendarEvent
    ▼
FastmailSchema serializes JSON
    ▼
MCP Host receives structured result
```

### Confirmation Gate Flow (target state after v1.2)

```
Mutation: deleteEvent(action: PREVIEW, id: "uid")
    │
    ▼
MutationRoot::delete_event()
    │ generate UUID token
    │ nonce_store.issue(token).await     — new: server-side storage
    │ return GqlEventDeleteResult { preview, confirmation_token }
    ▼
MCP Host shows preview to user
    │
    ▼ (user approves)
    │
Mutation: deleteEvent(action: CONFIRM, id: "uid", confirmationToken: "…")
    │
    ▼
MutationRoot::delete_event()
    │ nonce_store.consume(token).await
    │   → false: reject ("token invalid or already used")
    │   → true: proceed with CalDAV DELETE
    ▼
Event deleted; confirmation single-use enforced
```

---

## Anti-Patterns

### Anti-Pattern 1: Lock-Before-Await

**What people do:** `let mut client = arc_mutex.lock().await; let result = client.some_network_call().await?;`

**Why it is wrong:** Holds the `MutexGuard` across the network round-trip (which may take hundreds of milliseconds). Serializes all concurrent MCP requests through a single-lane bottleneck. Finding #26.

**Do this instead:** Lock only to read cached credentials or state, then drop the guard before any `await`. HTTP operations are stateless at the protocol level — no lock needed during the I/O itself.

### Anti-Pattern 2: Bundling High-Ripple Changes With Security Fixes

**What people do:** Include AppContext refactor (#6), NonceStore (#14), and newtype IDs (#23) in one PR along with injection fixes.

**Why it is wrong:** All three touch `mutation.rs` and `query.rs`. Merge conflicts are guaranteed. Security fixes get held up by mechanical refactoring review.

**Do this instead:** Phase 1–2 are pure security/stability fixes with narrow blast radius. Phase 3 is the MCP refactor. Phase 7 is newtype IDs alone. Never co-mingle them.

### Anti-Pattern 3: `process::exit` After Phase 1

**What people do:** Add new confirmation guards or error paths using the `eprintln!` + `std::process::exit(1)` pattern that already exists in `main.rs`.

**Why it is wrong:** Tool consumers (MCP, scripts) receive nothing on stdout. The structured JSON output contract is broken silently. Finding #11.

**Do this instead:** After Phase 1 lands, treat `process::exit` in `main.rs` as a lint violation. All exit paths use `Output::<()>::error("...").print()` then return.

### Anti-Pattern 4: Placing Integration Tests in `#[cfg(test)]` Blocks

**What people do:** Add wiremock-based tests as `#[cfg(test)]` modules inside the source files they test.

**Why it is wrong:** Wiremock tests bind external HTTP mock servers; they are integration tests, not unit tests. Cargo requires them in `tests/` to compile as separate test binaries. Adding them colocated also pulls `wiremock` into the regular test compilation unit.

**Do this instead:** All wiremock tests go in `tests/`. Unit tests (pure logic, no I/O) remain colocated in `#[cfg(test)]` blocks.

---

## Integration Points Summary

### Security Cluster (#3, #8, #9, #10, #14, #15, #25, #30)

| Finding | Integration Point | Layer |
|---------|------------------|-------|
| #3 Path traversal | `src/commands/download.rs:112` — `Path::file_name()` before `join` | commands |
| #8 vCard injection | `src/carddav/mod.rs:744,753` — `escape_value()` on EMAIL/TEL fields | carddav |
| #9 iCal injection | `src/caldav/mod.rs:1320-1329,1352` — validate enum fields, escape emails | caldav |
| #10 Token in args | `src/main.rs:29` — replace positional arg with stdin read | main |
| #14 Nonce tokens | `src/mcp/graphql/types.rs` + `mutation.rs` + `src/nonce.rs` (new) | mcp |
| #15 Secret Debug | `src/config.rs:6,14,19` — manual `Debug` impl | config |
| #25 spam gate | `src/mcp/graphql/mutation.rs:820-864` — add PREVIEW/CONFIRM pattern | mcp |
| #30 URL encoding | `src/jmap/mod.rs:858-863` — percent-encode template values | jmap |

### Stability Cluster (#1, #2, #11, #17, #18, #19, #26, #32, #33)

| Finding | Integration Point | Layer |
|---------|------------------|-------|
| #1 4xx status | `src/jmap/mod.rs:204-209,262-267` — `400..500` arm in both match blocks | jmap |
| #2 DAV timeout | `src/carddav/mod.rs:81`, `src/caldav/mod.rs:101` — `Client::builder().timeout()` | carddav/caldav |
| #11 process::exit | `src/main.rs:753,856,919,945,1095` — `Output::error().print()` | main |
| #17 MCP shutdown | `src/mcp/mod.rs:170-185` — `tokio::select!` with `ctrl_c()` | mcp |
| #18 let-else | `src/commands/download.rs:18,27,59` — `let Some(x) = … else { return }` | commands |
| #19 search filter | `src/carddav/mod.rs:252-277` — server-side text-match + per-book error tolerance | carddav |
| #26 mutex/IO | `src/mcp/graphql/query.rs:28-29` — drop guard before `await` | mcp |
| #32 JmapClient::new | `src/jmap/mod.rs:183` — return `Result<Self>`, remove `expect()` | jmap |
| #33 config error | `src/config.rs:46-47` — append recovery command to error message | config |

### Performance Cluster (#4, #5, #6, #12, #13, #16, #20, #21, #22, #29, #31)

| Finding | Integration Point | Layer |
|---------|------------------|-------|
| #4 concurrent fetch | `src/caldav/mod.rs:349-366`, `src/carddav/mod.rs:252-277` — `join_all` | carddav/caldav |
| #5 targeted lookup | `src/caldav/mod.rs:401-420` — REPORT with UID filter | caldav |
| #6 DAV client reuse | `src/mcp/graphql/mod.rs` + `query.rs` — `AppContext` shared clients | mcp |
| #12 Bytes return | `src/jmap/mod.rs:881-882` — return `bytes::Bytes` from blob download | jmap |
| #13 owned parse | `src/jmap/mod.rs:312` — `parse_response` takes owned `Value` | jmap |
| #16 feature flag | `Cargo.toml` + `src/util.rs` — `kreuzberg` optional + `#[cfg(feature)]` | util/cargo |
| #20 Arc cache | `src/jmap/mod.rs:320-346` — `Option<Arc<Vec<Mailbox>>>` | jmap |
| #21 cap clone | `src/jmap/mod.rs:249` — `Arc<Vec<String>>` for capabilities | jmap |
| #22 resize filter | `src/util.rs:244` — `FilterType::Triangle` | util |
| #29 tokio features | `Cargo.toml` — replace `full` with explicit 5 features | cargo |
| #31 GqlEmail clone | `src/mcp/graphql/types.rs:163-174` — `Arc<Vec<GqlEmailAddress>>` | mcp |

### Quality Cluster (#7, #23, #27, #28)

| Finding | Integration Point | Layer |
|---------|------------------|-------|
| #7 integration tests | `tests/` directory; `[dev-dependencies]` wiremock | tests |
| #23 newtyped IDs | `src/ids.rs` (new) → `src/models/mod.rs` → all callers | models + all |
| #27 stable hash | `src/carddav/mod.rs:781-786` — `sha2` digest or `FxHasher` | carddav |
| #28 stale allow | `src/carddav/mod.rs:12` — remove `#[allow(unused_imports)]` | carddav |

---

## Sources

- Direct analysis: `src/jmap/mod.rs`, `src/carddav/mod.rs`, `src/caldav/mod.rs`, `src/mcp/mod.rs`, `src/mcp/graphql/mod.rs`, `src/mcp/graphql/query.rs`, `src/mcp/graphql/mutation.rs`, `src/mcp/graphql/types.rs`, `src/config.rs`, `src/util.rs`, `src/error.rs`, `src/main.rs`
- `CODEBASE-REVIEW.md` — 33 findings, all incorporated
- `.planning/PROJECT.md` — v1.2 milestone scope and constraints
- `async-graphql` v7 API: `SchemaBuilder::limit_depth`, `SchemaBuilder::limit_complexity`
- Tokio documentation: mutex with async I/O anti-pattern
- Cargo reference: optional dependencies, feature flags, `[dev-dependencies]`

---
*Architecture research for: fastmail-cli v1.2 Hardening & Quality*
*Researched: 2026-04-04*
