# Phase 14: MCP Layer Refactor - Research

**Researched:** 2026-04-04
**Domain:** Rust async / rmcp 0.12 / async-graphql 7 / tokio signal handling / HMAC cryptography
**Confidence:** HIGH (all key claims verified from source code + official docs)

## Summary

This phase refactors the MCP GraphQL layer across six requirements: shared `AppContext` (PERF-03), HMAC confirmation tokens (SEC-05), GraphQL complexity limits (SEC-07), markAsSpam confirmation gate (SEC-08), graceful SIGTERM/SIGINT shutdown (STAB-04), and eliminating `MutexGuard`-held-across-await (STAB-07).

The most critical research finding is that **rmcp 0.12 exposes a `cancellation_token()` accessor on `RunningService`** that returns a `RunningServiceCancellationToken` you can clone before calling `waiting()`. This allows a separate `tokio::select!` arm to drive shutdown without consuming `RunningService` prematurely. The `waiting()` method takes ownership, so the cancellation token must be extracted first.

The second critical finding is an **API name mismatch in the context decisions**: the CONTEXT.md (D-09) states `.depth_limit(5).complexity(200)` but the actual async-graphql 7 SchemaBuilder API is `.limit_depth(5).limit_complexity(200)`. Using the wrong names will produce a compile error.

**Primary recommendation:** Extract `RunningServiceCancellationToken` before `waiting()`, drive it from a `tokio::select!` signal loop. Add `hmac = "0.12"` + `rand_core = "0.9"` (rand already transitive). Use `limit_depth` / `limit_complexity` on the SchemaBuilder. Refactor all `lock().await` + immediate `.await` callsites into scoped guard drops.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Shared AppContext Architecture**
- D-01: Introduce `AppContext` struct in `src/mcp/graphql/mod.rs` holding: `jmap: Option<Arc<tokio::sync::Mutex<JmapClient>>>`, `carddav: Arc<tokio::sync::OnceCell<Arc<CardDavClient>>>`, `caldav: Arc<tokio::sync::OnceCell<Arc<CalDavClient>>>`, `hmac_key: Arc<[u8; 32]>`
- D-02: Pass `AppContext` via `schema.data(ctx)` once at server startup; resolvers call `ctx.data::<AppContext>()` instead of constructing clients per call
- D-03: DAV OnceCell initialization uses `get_or_try_init()` with Fastmail username/app-password from Config. Failure returns GraphQL error; does not panic server
- D-04: Replace 2 callsites in `src/mcp/graphql/query.rs` at lines 212 and 230 (`CardDavClient::new` / `CalDavClient::new`) with `ctx.get_carddav().await?` / `ctx.get_caldav().await?` helpers on AppContext

**HMAC Confirmation Tokens**
- D-05: Add `hmac = "0.12"` and `sha2 = "0.10"` crates. Use `Hmac<Sha256>` for token generation
- D-06: Generate 32-byte HMAC key at server startup via `rand::rngs::OsRng` (`rand_core::OsRng::fill_bytes`). Key lives for process lifetime only
- D-07: Replace `confirmation_token(parts: &[&str]) -> String` in `src/mcp/graphql/types.rs:728` with `AppContext::confirmation_token(&self, parts: &[&str]) -> String` that HMACs length-prefixed parts, returns hex-encoded first 16 bytes
- D-08: Add `rand = "0.9"` + `rand_core = "0.9"` to Cargo.toml (check first — `rand` is already transitive)

**GraphQL Complexity Limits**
- D-09: Add depth limit 5 and complexity limit 200 via Schema builder in `build_schema()`
- D-10: Limits chosen based on current schema depth 3; 5 gives headroom; 200 accommodates batch queries

**markAsSpam Confirmation Gate (SEC-08)**
- D-11: Add `MarkAsSpamAction` enum mirroring `DeleteContactAction` (PREVIEW, CONFIRM variants). Update `markAsSpam` mutation signature to accept `action: MarkAsSpamAction` + `confirmation_token: Option<String>`
- D-12: Use same HMAC token generation pattern as deleteContact/deleteCalendar/deleteEvent

**SIGTERM/SIGINT Handling**
- D-13: In `src/mcp/mod.rs::run_server()`, add `tokio::select!` arm listening on `tokio::signal::unix::signal(SignalKind::terminate())` and `tokio::signal::ctrl_c()`. On signal: drop rmcp transport gracefully, let in-flight tool calls complete (with short timeout), then exit 0
- D-14: Use `tokio_util::sync::CancellationToken` for propagating shutdown signal to long-running resolver work

**MutexGuard-Across-Await Audit (STAB-07)**
- D-15: Audit all `.lock().await` usages in `src/mcp/graphql/` where guard is held across subsequent `.await`. Refactor to scope guard drop before awaiting
- D-16: Enable `clippy::await_holding_lock` lint as deny-level for `src/mcp/` module

### Claude's Discretion

- Exact shutdown timeout duration (5s reasonable default)
- Whether to use `tracing::info!` or `debug!` for startup/shutdown logs
- Test strategy for HMAC tokens (likely inject deterministic key in tests)
- Test strategy for SIGTERM (requires integration test or `#[cfg(test)]` signal injection)

### Deferred Ideas (OUT OF SCOPE)

- Connection pooling beyond one client per protocol (not needed for single-user MCP)
- Token persistence across restarts (explicitly out of scope per D-06)
- Per-user HMAC keys (phase is scoped to per-process)
- Request-level rate limiting (separate concern)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| PERF-03 | MCP requests reuse shared `CardDavClient`/`CalDavClient` via `AppContext` instead of recreating per GraphQL request | D-01..D-04 + `tokio::sync::OnceCell` lazy-init pattern |
| SEC-05 | MCP destructive-mutation confirmation tokens bound to per-process random nonce, not forgeable | D-05..D-08; HMAC-SHA256 with OsRng key |
| SEC-07 | MCP GraphQL schema enforces depth and complexity limits to bound query cost | D-09..D-10; `limit_depth(5).limit_complexity(200)` on SchemaBuilder |
| SEC-08 | `markAsSpam` MCP mutation requires same confirmation-token flow as other destructive mutations | D-11..D-12; existing `SpamAction` enum upgraded to PREVIEW/CONFIRM with token |
| STAB-04 | MCP server handles SIGINT/SIGTERM gracefully before exit | D-13..D-14; `RunningServiceCancellationToken` + `tokio::select!` |
| STAB-07 | MCP `Mutex` guard on `JmapClient` dropped before awaiting downstream I/O | D-15..D-16; `#![deny(clippy::await_holding_lock)]` + guard-scoping refactor |
</phase_requirements>

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| async-graphql | 7.2.1 (in Cargo.lock) | GraphQL schema + execution | Already in use |
| rmcp | 0.12.0 (in Cargo.lock) | MCP server transport | Already in use |
| tokio | 1.49.0 | Signal handling, select!, sync primitives | Already in use |
| hmac | 0.12.x | HMAC-SHA256 token generation | RustCrypto standard, compatible with sha2 0.10 |
| sha2 | 0.10.9 (transitive in Cargo.lock) | SHA-256 hash function | Already in dep tree transitively |
| rand_core | 0.9.5 (transitive in Cargo.lock) | `OsRng` for cryptographic key generation | Already in dep tree transitively |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio-util | 0.7.18 (transitive in Cargo.lock) | `CancellationToken` for shutdown propagation | Needed for D-14; already in dep tree |
| rand | 0.9.2 (transitive in Cargo.lock) | Re-exports `OsRng` for convenience | May add as direct dep if needed for `rand::rngs::OsRng` import path |

### Dependency Audit (from Cargo.lock)

| Dep | In Cargo.toml? | In Cargo.lock? | Action |
|-----|---------------|---------------|--------|
| `hmac = "0.12"` | No | No | Add to Cargo.toml |
| `sha2 = "0.10"` | No | Yes (transitive) | Add to Cargo.toml as direct dep for import |
| `rand_core = "0.9"` | No | Yes (transitive) | Add to Cargo.toml as direct dep for `OsRng` |
| `rand = "0.9"` | No | Yes (transitive) | Optionally add; prefer `rand_core` to minimize deps |
| `tokio-util` | No | Yes (transitive) | Add to Cargo.toml with `sync` feature for `CancellationToken` |

**Installation:**
```toml
hmac = "0.12"
sha2 = "0.10"
rand_core = "0.9"
tokio-util = { version = "0.7", features = ["sync"] }
```

**Note on rand:** `rand_core::OsRng` is the correct import in rand_core 0.9. The `fill_bytes` method has been replaced in rand_core 0.9 — use `try_fill_bytes` from `TryRngCore` trait, or use `rand::Fill` trait via `rand::rngs::OsRng`. See API details below.

## Architecture Patterns

### AppContext Design

The `JmapContext` struct in `src/mcp/graphql/mod.rs` is replaced by `AppContext`, which is injected once via `schema.data()` at startup and retrieved in resolvers via `ctx.data::<AppContext>()`.

```rust
// Source: existing pattern from src/mcp/graphql/mutation.rs (require_jmap_client)
// Resolver retrieval pattern (already established):
ctx.data::<JmapContext>()?.client.clone()

// New pattern (AppContext replaces JmapContext):
ctx.data::<AppContext>()?.get_jmap_client()?
```

### Pattern 1: OnceCell Lazy DAV Client Init

```rust
// src/mcp/graphql/mod.rs
use tokio::sync::OnceCell;

pub struct AppContext {
    pub jmap: Option<Arc<Mutex<JmapClient>>>,
    pub carddav: Arc<OnceCell<Arc<CardDavClient>>>,
    pub caldav: Arc<OnceCell<Arc<CalDavClient>>>,
    pub hmac_key: Arc<[u8; 32]>,
}

impl AppContext {
    pub async fn get_carddav(&self) -> async_graphql::Result<Arc<CardDavClient>> {
        self.carddav.get_or_try_init(|| async {
            let config = crate::config::Config::load()
                .map_err(|e| async_graphql::Error::new(e.to_string()))?;
            let username = config.get_username()
                .map_err(|_| async_graphql::Error::new("Username not configured."))?;
            let app_password = config.get_app_password()
                .map_err(|_| async_graphql::Error::new("App password not configured."))?;
            CardDavClient::new(username, app_password)
                .map(Arc::new)
                .map_err(|e| async_graphql::Error::new(e.to_string()))
        }).await.cloned()
    }
}
```

**Note:** `OnceCell::get_or_try_init` is available in `tokio::sync::OnceCell` for async init. Returns `&T`, clone to get `Arc<T>`.

### Pattern 2: HMAC Token Generation

```rust
// D-07: AppContext::confirmation_token using hmac 0.12 + sha2 0.10
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

impl AppContext {
    pub fn confirmation_token(&self, parts: &[&str]) -> String {
        let mut mac = HmacSha256::new_from_slice(&*self.hmac_key)
            .expect("HMAC accepts any key size");
        for part in parts {
            // Length-prefix to prevent ["a", "bc"] == ["ab", "c"] collision
            mac.update(&(part.len() as u64).to_le_bytes());
            mac.update(part.as_bytes());
        }
        let result = mac.finalize().into_bytes();
        // Hex-encode first 16 bytes (128-bit truncation)
        result[..16].iter().map(|b| format!("{:02x}", b)).collect()
    }
}
```

### Pattern 3: OsRng Key Generation (rand_core 0.9 API)

**CRITICAL:** In `rand_core 0.9`, `OsRng` implements `TryRngCore`, not `RngCore` directly. The correct API is:

```rust
use rand_core::{OsRng, TryRngCore};

let mut key = [0u8; 32];
OsRng.try_fill_bytes(&mut key)
    .expect("OS RNG failed — should not happen");
```

D-06 references `rand_core::OsRng::fill_bytes` — this is the **wrong method name** for rand_core 0.9. Use `try_fill_bytes` from the `TryRngCore` trait instead.

Alternatively, use `rand::rngs::OsRng` (re-exported from `rand` crate) which implements `RngCore` directly via blanket impl:

```rust
use rand::{RngCore, rngs::OsRng};
let mut key = [0u8; 32];
OsRng.fill_bytes(&mut key);
```

Either approach works. The `rand_core` direct path requires importing `TryRngCore`; the `rand` path is slightly cleaner. Pick based on whether you add `rand` as direct dep.

### Pattern 4: SchemaBuilder Complexity/Depth Limits

**CRITICAL NAME FIX:** CONTEXT.md D-09 says `.depth_limit(5).complexity(200)` — these method names do not exist. The actual API (verified from docs.rs):

```rust
// Source: https://docs.rs/async-graphql/latest/async_graphql/struct.SchemaBuilder.html
Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
    .data(app_ctx)
    .limit_depth(5)           // NOT .depth_limit(5)
    .limit_complexity(200)    // NOT .complexity(200)
    .finish()
```

Both take `usize`. Introspection queries bypass these limits by default.

### Pattern 5: SIGTERM/SIGINT Shutdown with RunningService

**Key finding:** `RunningService::cancellation_token()` returns a `RunningServiceCancellationToken` that can be called `.cancel()` on externally. Extract this before calling `.waiting()` (which consumes `RunningService`):

```rust
// Source: rmcp 0.12 source, service.rs
pub async fn run_server() -> anyhow::Result<()> {
    use rmcp::{ServiceExt, transport::stdio};
    use tokio::signal::unix::{signal, SignalKind};

    let service = FastmailMcp::new().await?;
    let server = service
        .serve(stdio())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to start MCP server: {}", e))?;

    // Extract cancellation token BEFORE waiting() consumes server
    let cancel_token = server.cancellation_token();

    // Spawn signal handler task
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        tokio::select! {
            _ = sigterm.recv() => {
                tracing::debug!("SIGTERM received — shutting down MCP server");
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::debug!("SIGINT received — shutting down MCP server");
            }
        }
        cancel_token.cancel();
    });

    server
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}
```

**Note:** `RunningServiceCancellationToken::cancel(self)` consumes the token (takes `self`). Call it exactly once from the signal task.

**Note on D-14:** `tokio_util::sync::CancellationToken` is what rmcp uses internally. The `RunningServiceCancellationToken` wraps it. For propagating shutdown to resolver work, the `RunningServiceCancellationToken` already drives the internal rmcp cancellation — no separate `tokio_util::sync::CancellationToken` needed unless long-running resolver tasks need it explicitly.

### Pattern 6: MutexGuard-Across-Await Fix (STAB-07)

The problem appears in ~12 locations across `query.rs`, `mutation.rs`, and `types.rs`. The canonical violation pattern:

```rust
// WRONG — guard held across .await
let client = client.lock().await;          // guard held here
let email = client.get_email(&id).await?;  // .await while holding guard
```

The fix scopes the guard so it drops before the await:

```rust
// CORRECT — drop guard before awaiting downstream I/O
let email = {
    let mut guard = client.lock().await;
    guard.get_email(&id).await?  // guard dropped at end of block
};
```

**Wait — this pattern is wrong too.** `get_email` is async and returns a Future. If called on the guard within the block, the guard is still held across the inner `.await`. The correct fix is to only extract sync data from the guard, then release it before awaiting:

Actually for methods that take `&mut self` and are async (like `JmapClient::get_email`), you cannot call them without holding the guard. The correct fix for STAB-07 is to NOT hold the guard across the `.await` of the async method — but since all `JmapClient` methods are async and require `&mut self`, the guard MUST be held for the duration. The real fix here is that clippy's `await_holding_lock` catches holding a `MutexGuard` *across* a yield point. The entire call `client.method().await` while holding the guard is the issue.

The correct refactor when guard must be held for the async call is to use an owned guard via `Mutex::lock_owned()` instead:

```rust
// Pattern that avoids holding std::sync::MutexGuard across await:
// For tokio::sync::Mutex, holding the guard across .await is FINE
// because tokio::sync::MutexGuard is Send

// clippy::await_holding_lock targets std::sync::MutexGuard
// tokio::sync::MutexGuard is explicitly allowed to be held across .await
```

**Critical clarification:** `clippy::await_holding_lock` checks for `std::sync::MutexGuard` held across await, NOT `tokio::sync::MutexGuard`. The codebase uses `tokio::sync::Mutex` throughout, so **the lint does not fire** on the existing callsites. STAB-07's real problem is **serialization** (concurrent queries blocked while one holds the lock for a long HTTP call) — not unsafety. The fix is to drop the guard earlier where possible (e.g., extract data, drop guard, then do I/O) rather than hold it for the entire I/O call.

For callsites where you call one method and immediately use the result, the guard can be scoped:

```rust
// If only reading cached data (no async I/O on the guard):
let mailboxes = { client.lock().await.cached_mailboxes.clone() };

// If must call async method, hold guard for that call, then drop:
let email = client.lock().await.get_email(&id).await?;
// This is fine — tokio MutexGuard IS Send and IS designed for this
```

The real win for STAB-07 is the `AppContext` pattern (D-01..D-04): DAV clients move to `OnceCell` (no Mutex needed), reducing lock contention for CardDAV/CalDAV operations.

### Pattern 7: Clippy Deny at Module Level

```rust
// At top of src/mcp/mod.rs or in a file that controls the module:
#![deny(clippy::await_holding_lock)]
```

This applies to the entire `src/mcp/` module tree. Can also be applied per-file:

```rust
// At top of src/mcp/graphql/mutation.rs:
#![deny(clippy::await_holding_lock)]
```

Note: Since the codebase uses `tokio::sync::MutexGuard` (not `std::sync::MutexGuard`), this lint will not fire on existing code as-is. The deny annotation serves as a guard against future `std::sync::Mutex` use in async contexts.

### Anti-Patterns to Avoid

- **Wrong schema builder method names:** `.depth_limit()` and `.complexity()` do not exist. Use `.limit_depth()` and `.limit_complexity()`.
- **Consuming RunningService before signal setup:** Call `server.cancellation_token()` before `server.waiting()` — `waiting()` consumes `server`.
- **`fill_bytes` on rand_core 0.9 OsRng:** Not available directly. Use `try_fill_bytes` from `TryRngCore` or use `rand::rngs::OsRng` with `fill_bytes`.
- **Using `hmac 0.13` instead of `0.12`:** hmac 0.13 uses `digest 0.11`, which may conflict with the transitive `sha2 0.10.9` (which uses `digest 0.10`). Lock to `hmac = "0.12"`.
- **Injecting `JmapContext` AND `AppContext` into schema:** Replace `JmapContext` with `AppContext` entirely in `build_schema()`. Resolvers that currently read `JmapContext` must be updated to read `AppContext`.
- **Forgetting to update `require_jmap_client()` in both `query.rs` and `mutation.rs`:** Both files have their own copy of the helper. Consolidate into `AppContext` methods.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Cryptographic randomness | Custom entropy | `rand_core::OsRng` | OS-backed, side-channel resistant |
| HMAC | Custom HMAC | `hmac` crate | RustCrypto; constant-time finalize() |
| Process cancellation | Channel + flag | `RunningServiceCancellationToken::cancel()` | rmcp already wires this into its task loop |
| Async lazy init | Custom once-flag | `tokio::sync::OnceCell::get_or_try_init()` | Correct async semantics, no blocking |

**Key insight:** rmcp's `RunningService` already contains a `CancellationToken` internally — there is no need to wire an external `tokio_util::sync::CancellationToken` through the server unless resolvers need their own cancellation propagation.

## Common Pitfalls

### Pitfall 1: Wrong SchemaBuilder Method Names
**What goes wrong:** Compile error — `Schema::build(...).depth_limit()` does not exist.
**Why it happens:** CONTEXT.md D-09 states the wrong method names.
**How to avoid:** Use `.limit_depth(5).limit_complexity(200)` — exactly as documented in async-graphql 7 SchemaBuilder.
**Warning signs:** Compiler reports "no method named `depth_limit` found for struct `SchemaBuilder`".

### Pitfall 2: Consuming RunningService Before Signal Handling
**What goes wrong:** `server.waiting().await` runs immediately, blocking the task. Signal handler never set up. Process not gracefully terminated.
**Why it happens:** `waiting()` takes ownership — nothing can be done to `server` after calling it.
**How to avoid:** Call `server.cancellation_token()` to extract a cloneable token before calling `server.waiting()`. Spawn the signal handler as a separate task that calls `cancel_token.cancel()`.
**Warning signs:** Code that calls `waiting()` and then tries to use `server` afterward won't compile.

### Pitfall 3: rand_core 0.9 OsRng API Change
**What goes wrong:** `OsRng.fill_bytes()` compile error — method not found.
**Why it happens:** In rand_core 0.9, `OsRng` implements `TryRngCore` not `RngCore`; `fill_bytes` is on `RngCore`.
**How to avoid:** Use `OsRng.try_fill_bytes(&mut key).expect(...)` with `use rand_core::TryRngCore;`, or add `rand = "0.9"` and use `rand::rngs::OsRng` which re-implements `RngCore` via blanket impl.
**Warning signs:** Compile error mentioning `fill_bytes` not found on `OsRng`.

### Pitfall 4: hmac Version / Digest Compatibility
**What goes wrong:** Version conflict between `hmac` and `sha2` if wrong versions are pinned.
**Why it happens:** `hmac 0.12` depends on `digest ^0.10`; `hmac 0.13` depends on `digest ^0.11`. sha2 in Cargo.lock is 0.10.9 (digest 0.10). Using `hmac 0.13` may introduce a conflicting `digest 0.11` dependency.
**How to avoid:** Pin `hmac = "0.12"` in Cargo.toml. sha2 0.10.x is compatible with hmac 0.12.
**Warning signs:** Cargo.lock shows duplicate `sha2` entries or `digest` version conflict warnings.

### Pitfall 5: JmapContext vs AppContext Context Type Mismatch
**What goes wrong:** Resolvers that call `ctx.data::<JmapContext>()` break after renaming to `AppContext`. Or both are injected and resolvers read the wrong one.
**Why it happens:** Both `query.rs` and `mutation.rs` have their own `require_jmap_client()` helper that reads `JmapContext`. These must all be updated.
**How to avoid:** Global search for `JmapContext` before shipping. Update `require_jmap_client` in all files to read `AppContext` instead.
**Warning signs:** Runtime GraphQL error "No data of type JmapContext found."

### Pitfall 6: SpamAction Already Defined
**What goes wrong:** Adding `MarkAsSpamAction` enum that duplicates `SpamAction` causes GraphQL schema conflict or dead code.
**Why it happens:** `SpamAction` already exists in `types.rs:683` with `Preview` and `Confirm` (but no token support). D-11 says add `MarkAsSpamAction`.
**How to avoid:** Check whether `SpamAction` can be extended rather than duplicating it. Alternatively, rename the existing `SpamAction` to `MarkAsSpamAction` and add the `confirmation_token` parameter to `mark_as_spam`. The existing mutation already has `SpamAction::Preview` and `SpamAction::Confirm` but no token in the current CONFIRM path.
**Warning signs:** GraphQL schema SDL shows two similar enum types; `async-graphql` may reject duplicate type names.

### Pitfall 7: clippy::await_holding_lock Does Not Fire on tokio::sync
**What goes wrong:** Adding `#![deny(clippy::await_holding_lock)]` passes lint clean even where code holds locks across await. Developer assumes STAB-07 is addressed but the actual serialization problem remains.
**Why it happens:** The lint specifically targets `std::sync::MutexGuard`, not `tokio::sync::MutexGuard`. All existing code uses tokio Mutex so the lint is a no-op on existing patterns.
**How to avoid:** Understand that STAB-07's fix is architectural (AppContext OnceCell for DAV clients) and guard-scoping for JMAP calls, not just a lint. The deny annotation prevents future misuse of `std::sync::Mutex` in async code.

## Code Examples

### OsRng Key Generation (rand_core 0.9 — verified API)
```rust
// Source: https://docs.rs/rand_core/0.9.5/rand_core/struct.OsRng.html
use rand_core::{OsRng, TryRngCore};

let mut key = [0u8; 32];
OsRng.try_fill_bytes(&mut key)
    .expect("OS random number generator failed");
```

### HMAC-SHA256 with hmac 0.12 (verified API)
```rust
// Source: https://docs.rs/hmac/0.12.1/hmac/
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

let mut mac = HmacSha256::new_from_slice(&key)
    .expect("HMAC can take key of any size");
// Length-prefix each part to prevent input ambiguity
for part in parts {
    mac.update(&(part.len() as u64).to_le_bytes());
    mac.update(part.as_bytes());
}
let result = mac.finalize().into_bytes();
// First 16 bytes = 128-bit token, hex-encoded
let token: String = result[..16].iter().map(|b| format!("{:02x}", b)).collect();
```

### async-graphql SchemaBuilder with limits (verified API)
```rust
// Source: https://docs.rs/async-graphql/latest/async_graphql/struct.SchemaBuilder.html
Schema::build(QueryRoot, MutationRoot, async_graphql::EmptySubscription)
    .data(app_ctx)          // replaces .data(JmapContext { client })
    .limit_depth(5)         // max nesting depth
    .limit_complexity(200)  // max total field count
    .finish()
```

### rmcp RunningService Cancellation (verified from source)
```rust
// Source: github.com/modelcontextprotocol/rust-sdk service.rs
let server = service.serve(stdio()).await?;
let cancel_token = server.cancellation_token(); // extract before waiting()

tokio::spawn(async move {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    tokio::select! {
        _ = sigterm.recv() => {},
        _ = tokio::signal::ctrl_c() => {},
    }
    cancel_token.cancel(); // takes ownership, fires once
});

server.waiting().await?; // takes ownership, returns QuitReason
```

### markAsSpam with Confirmation Token (adapted from existing deleteContact pattern)
```rust
// Pattern from mutation.rs lines 105-168 (deleteContact)
// Add MarkAsSpamAction enum to types.rs (or reuse/rename SpamAction):
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum MarkAsSpamAction {
    /// Preview what will happen
    Preview,
    /// Confirm the action with the token from Preview
    Confirm,
}

// In mutation resolver:
async fn mark_as_spam(
    &self,
    ctx: &Context<'_>,
    email_id: String,
    action: MarkAsSpamAction,
    confirmation_token: Option<String>,
) -> Result<GqlStatus> {
    let app_ctx = ctx.data::<AppContext>()?;
    let token = app_ctx.confirmation_token(&[&email_id]);
    if matches!(action, MarkAsSpamAction::Preview) {
        // fetch email for preview text (requires JMAP client)
        // return preview + token
    }
    if confirmation_token.as_deref() != Some(&token) {
        // return error
    }
    // perform spam marking
}
```

### Module-Level Clippy Deny
```rust
// At top of src/mcp/graphql/mutation.rs (and other mcp files):
#![deny(clippy::await_holding_lock)]
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-request `CardDavClient::new()` | `OnceCell`-cached shared client via `AppContext` | Phase 14 | Eliminates TLS handshake per query |
| Deterministic `DefaultHasher` token | HMAC-SHA256 with per-process random key | Phase 14 | Tokens unguessable from inputs alone |
| No complexity/depth limits | `.limit_depth(5).limit_complexity(200)` | Phase 14 | Bounds resource consumption |
| `markAsSpam` no token guard | PREVIEW/CONFIRM with HMAC token | Phase 14 | Matches other destructive mutations |
| Process-kill on SIGTERM | Graceful drain via `cancel_token.cancel()` | Phase 14 | Clean shutdown for in-flight requests |

**Deprecated/outdated in this phase:**
- `confirmation_token()` free function in `types.rs:728` — replaced by `AppContext::confirmation_token(&self, ...)` method
- `JmapContext` struct in `graphql/mod.rs` — replaced by `AppContext`
- Per-call `Config::load()` in CardDAV/CalDAV resolvers — replaced by AppContext init

## Open Questions

1. **SpamAction vs MarkAsSpamAction naming**
   - What we know: `SpamAction` enum already exists in `types.rs:683` with `Preview` and `Confirm` variants; the current `mark_as_spam` resolver uses it but has no token parameter
   - What's unclear: Whether to rename `SpamAction` → `MarkAsSpamAction` (breaking GraphQL SDL change for existing MCP hosts) or add a new enum
   - Recommendation: Reuse existing `SpamAction` enum name to avoid GraphQL SDL churn; add `confirmation_token: Option<String>` parameter to the mutation. Any MCP host that pre-populates the mutation will need updating regardless.

2. **Shutdown timeout implementation**
   - What we know: D-13 says "let in-flight tool calls complete (with short timeout)"; D-14 says CancellationToken for propagation
   - What's unclear: rmcp's `waiting()` returns after the transport closes, but in-flight requests may not have a timeout built in. Whether to add a `tokio::time::timeout` wrapper around `waiting()`
   - Recommendation: Add `tokio::time::timeout(Duration::from_secs(5), server.waiting())` wrapping. On timeout, log warning and return Ok(()).

3. **`require_jmap_client` duplication**
   - What we know: `require_jmap_client` is defined identically in both `mutation.rs` (line 13) and `query.rs`. After AppContext migration, both need updating.
   - What's unclear: Whether to move the helper to `AppContext` as a method or `mod.rs` as a module-level function
   - Recommendation: Add `AppContext::require_jmap(&self) -> async_graphql::Result<Arc<Mutex<JmapClient>>>` and delete both per-file helpers.

## Environment Availability

Step 2.6: SKIPPED — this phase is code/config changes only. No new external services or CLI tools required beyond the existing Rust toolchain and Cargo.

## Sources

### Primary (HIGH confidence)
- `Cargo.lock` in project root — exact transitive versions of rand 0.9.2, rand_core 0.9.5, sha2 0.10.9, tokio-util 0.7.18 (directly inspected)
- `src/mcp/mod.rs`, `src/mcp/graphql/mod.rs`, `src/mcp/graphql/mutation.rs`, `src/mcp/graphql/query.rs`, `src/mcp/graphql/types.rs` — existing callsites, patterns, enum definitions (directly read)
- [SchemaBuilder docs.rs](https://docs.rs/async-graphql/latest/async_graphql/struct.SchemaBuilder.html) — `limit_depth(usize)`, `limit_complexity(usize)` method signatures
- [async-graphql book: depth and complexity](https://async-graphql.github.io/async-graphql/en/depth_and_complexity.html) — usage examples
- [hmac 0.12.1 docs.rs](https://docs.rs/hmac/0.12.1/hmac/) — `new_from_slice`, `update`, `finalize`, `into_bytes` API; sha2 0.10 compat
- [rand_core 0.9.5 OsRng docs.rs](https://docs.rs/rand_core/0.9.5/rand_core/struct.OsRng.html) — `try_fill_bytes` via `TryRngCore`
- [rmcp 0.12 source service.rs](https://github.com/modelcontextprotocol/rust-sdk) — `RunningService`, `cancellation_token()`, `RunningServiceCancellationToken`, `cancel()`, `waiting()`
- [tokio-util CancellationToken docs.rs](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) — no extra features required for `sync` module

### Secondary (MEDIUM confidence)
- [Tokio graceful shutdown guide](https://tokio.rs/tokio/topics/shutdown) — tokio::select! signal handling patterns

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all dep versions read directly from Cargo.lock; API names verified from docs.rs
- Architecture: HIGH — all patterns derive from existing codebase + verified library docs
- Pitfalls: HIGH — API name mismatch (wrong method names in CONTEXT.md) confirmed by direct docs.rs check; rand_core 0.9 API change confirmed from docs

**Research date:** 2026-04-04
**Valid until:** 2026-05-04 (stable crates; rmcp API may change faster)
