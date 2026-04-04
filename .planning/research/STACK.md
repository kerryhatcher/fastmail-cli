# Stack Research

**Domain:** Rust CLI/MCP hardening — integration tests, secret redaction, security limits, concurrency, URL encoding
**Researched:** 2026-04-04
**Confidence:** HIGH (all version claims verified against docs.rs / official docs)

---

## Context: This is a Hardening Milestone

The existing production stack (tokio 1.49, reqwest 0.13.1, async-graphql 7, rmcp 0.12, clap 4.5, serde 1.0, thiserror 2.0, roxmltree 0.21, chrono 0.4, uuid 1) is **validated and must not change**. This document covers only the **additions** needed for v1.2 Hardening & Quality.

---

## New Stack Additions

### Dev Dependencies (Test Infrastructure)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `wiremock` | 0.6.5 | Async HTTP mock server for integration tests | Authoritative choice for tokio-based HTTP mocking in Rust. Starts a real local TCP server; no trait injection required. Works with reqwest unchanged. Supports any HTTP method string including WebDAV `PROPFIND`, `REPORT`, `MKCALENDAR`. Isolation per test via random port assignment. Mock expectations auto-verified on drop. Last release: 2025-08-24. Maintained by Luca Palmieri (zero2prod). |

**Why not mockito?** mockito 1.x is single-threaded and global-state based. It cannot run tests in parallel cleanly. wiremock is parallel-safe by design.

**Why not trait-based mocking (mockall)?** Requires introducing an abstraction layer (trait over `reqwest::Client`) through the entire call chain. wiremock requires zero refactoring of production code — the existing `reqwest::Client` just hits the mock server URL.

### Production Dependencies (New)

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| `secrecy` | 0.10.3 | `Secret<T>` wrapper for API tokens and app passwords | Fixes finding #15 (Debug derive on Config exposes plaintext secrets). `SecretString` alias wraps `String`; `Debug` outputs `[REDACTED]` automatically. Memory is zeroed on drop via `zeroize`. `expose_secret()` makes access explicit and grep-auditable. No unsafe code (`forbid(unsafe_code)`). Last release: 2024-10-09. |
| `percent-encoding` | 2.3.2 | RFC 3986 percent-encoding for URL path/query components | Fixes finding #30 (blob download URL template values not URL-encoded). Already a transitive dep via reqwest/url; adding directly is zero binary-size cost. `utf8_percent_encode()` + `NON_ALPHANUMERIC` or a custom `AsciiSet` for path-safe encoding. Last release: 2025-08-21. |
| `futures` | 0.3.32 | `join_all` / `try_join_all` for concurrent DAV fetches | Fixes findings #4 (sequential multi-calendar fetches) and #19 (sequential address-book fetches). Already a transitive dep. Prefer `futures::future::join_all` for ordered results (same order as input calendars). See JoinSet note below. Last release: 2026-02-15. |

---

## Tokio Feature Narrowing (Finding #29)

**Current:** `tokio = { version = "1.49.0", features = ["full"] }`

**Problem:** `full` enables signal handling, file I/O, UDP, Unix sockets, test utilities, and process spawning — none of which this codebase uses today.

**Verified actual tokio usage in codebase:**
- `tokio::sync::Mutex` — requires `sync`
- `#[tokio::main]` — requires `macros` + `rt-multi-thread`
- `reqwest` internals use `rt-multi-thread` (multi-thread executor for connection pooling and `spawn_blocking` DNS resolution)

**For signal handling (finding #17 — MCP graceful shutdown):**
- `tokio::signal::ctrl_c()` requires `signal` feature

**Recommended minimal feature set:**

```toml
tokio = { version = "1.49.0", features = [
  "rt-multi-thread",   # multi-thread scheduler; required by reqwest connection pooling
  "macros",            # #[tokio::main], #[tokio::test]
  "sync",              # tokio::sync::Mutex used in MCP context
  "time",              # tokio::time::timeout for DAV client timeout guards (finding #2, if using tokio timeout)
  "signal",            # tokio::signal::ctrl_c() for graceful MCP shutdown (finding #17)
] }
```

**Note on `time`:** The existing DAV timeout fix (finding #2) uses `std::time::Duration` with `reqwest::ClientBuilder::timeout()`, not `tokio::time`. Only add `time` if implementing explicit `tokio::time::timeout()` wrappers in the DAV layers. If the reqwest builder timeout is sufficient, `time` can be omitted.

**Confidence:** MEDIUM — reqwest's internal tokio feature requirements are not explicitly documented. If compilation fails after narrowing, add `net` and/or `io-util` before reverting to `full`. Safe migration path: narrow in a dedicated commit with CI gating.

---

## Concurrency Primitive Decision: `futures::join_all` vs `tokio::task::JoinSet`

**Recommendation: `futures::future::join_all`** for findings #4 and #19.

| Criterion | `futures::join_all` | `tokio::task::JoinSet` |
|-----------|--------------------|-----------------------|
| Result order | Preserves input order | Completion order (unordered) |
| Spawning | Concurrent poll, no new OS threads | Spawns separate tokio tasks |
| Cancellation | Cancel all on drop | Granular per-task abort |
| Error handling | Returns `Vec<Result<T>>` | Result per `.join_next()` |
| DAV fetch fit | Ideal — order matches addressbook list | Overkill for this use case |
| Already a dep | Yes (transitive) | Yes (tokio `rt` feature) |

`join_all` is correct for `list_events()` and `search_contacts()` because callers expect results in addressbook/calendar order, and failures should be surfaced per-book (finding #19 asks for error tolerance: log and continue, which fits `Vec<Result<T>>` iteration). `JoinSet` adds complexity with no benefit for this pattern.

Use `try_join_all` only if you want first-error-wins semantics (not appropriate for tolerant per-book error handling).

---

## async-graphql Depth/Complexity Limits (Finding #24)

**No new dependency required.** The existing `async-graphql 7` `SchemaBuilder` already exposes these methods:

```rust
// In src/mcp/graphql/mod.rs, on the Schema::build(...) call:
Schema::build(QueryRoot, MutationRoot, EmptySubscription)
    .limit_depth(10)           // rejects queries nested > 10 levels deep
    .limit_complexity(200)     // rejects queries with field count > 200
    .limit_recursive_depth(32) // default is 32; set explicitly for clarity
    .finish()
```

**Method signatures (verified against docs.rs async-graphql latest):**
- `fn limit_depth(self, depth: usize) -> Self` — "Set the maximum depth a query can have. By default, there is no limit."
- `fn limit_complexity(self, complexity: usize) -> Self` — "Set the maximum complexity a query can have. By default, there is no limit."
- `fn limit_recursive_depth(self, depth: usize) -> Self` — "Set the maximum recursive depth a query can have. (default: 32)"

**Recommended values for this CLI/MCP context:**
- `limit_depth(10)` — MCP tool calls are simple; legitimate queries never need more than 5-6 levels
- `limit_complexity(200)` — generous for structured queries, blocks pathological fan-out

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `mockito` | Global/single-threaded mock state; breaks parallel tests | `wiremock` |
| `httpmock` | Less ecosystem traction, fewer matchers, less tokio integration documentation | `wiremock` |
| `mockall` | Requires production-code trait refactor for every HTTP call; high noise-to-signal | `wiremock` |
| `redact` crate (0.1.x) | Niche, low adoption, no zeroize integration | `secrecy` 0.10.x |
| `veil` crate | Derive macro for Debug redaction only — doesn't wipe memory | `secrecy` |
| Custom `Debug` impl on Config | Manual maintenance burden, no memory zeroing | `secrecy::SecretString` |
| `urlencoding` crate | Thin wrapper with less control over which chars are encoded; `percent-encoding` is already a transitive dep | `percent-encoding` |
| `tokio::full` (long-term) | Compiles unused subsystems; slower CI builds on cold caches | Minimal feature list above |
| `ical` / `icalendar` crates | Not needed for v1.2; custom iCal serialization already ships | Keep existing approach |

---

## Installation

```toml
# Cargo.toml additions for v1.2

[dependencies]
# Secret redaction — fixes #15
secrecy = { version = "0.10.3", features = ["serde"] }

# URL encoding — fixes #30; percent-encoding is already a transitive dep,
# but pin it explicitly so the version is stable and the usage is auditable
percent-encoding = "2.3"

# Concurrent DAV fetches — fixes #4, #19; futures is already a transitive dep
futures = { version = "0.3", default-features = false, features = ["alloc"] }

# Narrow tokio features — fixes #29
tokio = { version = "1.49.0", features = [
  "rt-multi-thread",
  "macros",
  "sync",
  "signal",
  # "time" — add only if tokio::time::timeout wrappers are introduced
] }

[dev-dependencies]
# Integration test mock server — fixes #7
wiremock = "0.6.5"
```

**Note on `secrecy` serde feature:** The `serde` feature allows deserializing directly into `SecretString` from the TOML config file. Without it, you must deserialize into `String` then wrap. Since `Config` is loaded from TOML via `serde`, enabling `serde` is the clean path.

**Note on `futures` features:** `default-features = false, features = ["alloc"]` pulls in `join_all` / `try_join_all` without the `executor` and `io` features that overlap with tokio. This keeps the dep minimal and avoids runtime conflicts.

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| `wiremock 0.6.5` | `tokio ^1.47.1`, `reqwest 0.13.x` | wiremock's own Cargo.lock pins tokio ^1.47.1; compatible with project's 1.49.0 |
| `secrecy 0.10.3` | `serde 1.0.x`, `zeroize 1.x` | No conflicts with existing deps; MSRV 1.60 |
| `percent-encoding 2.3.2` | `reqwest 0.13.x` (already deps it via `url`) | Pinning same major version that reqwest already resolved |
| `futures 0.3.32` | `tokio 1.x`, `reqwest 0.13.x` | Already transitive; making explicit has zero version risk |
| Narrowed `tokio` features | `rmcp 0.12`, `reqwest 0.13.1`, `async-graphql 7` | If rmcp or reqwest require features not in the narrow list, compiler will error clearly — safe to discover |

---

## Sources

- `docs.rs/wiremock/latest` — version 0.6.5, release date 2025-08-24, tokio ^1.47.1 dependency confirmed
- `docs.rs/secrecy/latest` — version 0.10.3, release date 2024-10-09, `SecretString` API and `[REDACTED]` Debug behavior confirmed
- `docs.rs/percent-encoding/latest` — version 2.3.2, release date 2025-08-21, `utf8_percent_encode` API confirmed
- `docs.rs/futures/latest` — version 0.3.32, release date 2026-02-15
- `async-graphql.github.io/async-graphql/en/depth_and_complexity.html` — `limit_depth` / `limit_complexity` builder methods confirmed for async-graphql 7
- `docs.rs/async-graphql/latest/async_graphql/struct.SchemaBuilder.html` — exact method signatures and defaults verified
- `docs.rs/tokio/latest/tokio/index.html` — feature flag breakdown: `rt`, `rt-multi-thread`, `sync`, `macros`, `time`, `signal` definitions verified
- GitHub: `LukeMathWalker/wiremock-rs` — `method("PROPFIND")` string-based matching confirmed, parallel test isolation design confirmed
- Rust Users Forum + GitHub tokio#2057 — reqwest internal tokio dependency discussion (informs MEDIUM confidence on feature narrowing)

---
*Stack research for: fastmail-cli v1.2 Hardening & Quality*
*Researched: 2026-04-04*
