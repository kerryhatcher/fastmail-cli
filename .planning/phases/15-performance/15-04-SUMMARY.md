---
phase: 15-performance
plan: 04
subsystem: build
tags: [rust, cargo, features, optional-dep, tokio, image, performance, kreuzberg]

# Dependency graph
requires:
  - phase: 15-01
    provides: concurrent DAV fetch infrastructure
  - phase: 15-02
    provides: bytes::Bytes, Arc mailbox cache
  - phase: 15-03
    provides: GqlEmail Arc address fields
provides:
  - optional extract cargo feature gating kreuzberg
  - Triangle image resize filter
  - narrowed tokio feature set
affects:
  - Cargo.toml (feature flags, tokio dep)
  - src/util.rs (cfg-gated extract_text, Triangle filter)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "optional dep: kreuzberg marked optional=true, gated behind extract feature"
    - "cfg-gating: #[cfg(feature = \"extract\")] on real impl + private helpers; #[cfg(not(feature = \"extract\"))] on stub"
    - "tokio narrowing: explicit feature list replaces 'full'; comment documents rationale"

key-files:
  created: []
  modified:
    - Cargo.toml (optional kreuzberg, [features] section, narrowed tokio)
    - src/util.rs (cfg-gated extract_text + helpers, Triangle filter)

key-decisions:
  - "kreuzberg optional=true with extract feature gate; default = [\"extract\"] preserves backward-compat binary"
  - "is_image_extension and mime_from_filename also cfg-gated with #[cfg(feature = \"extract\")] — both private fns only used by extract_text, avoid unused-fn lint"
  - "tokio narrowed to [rt-multi-thread, rt, macros, io-util, sync, time, signal, net] — covers tokio::main, tokio::test, tokio::spawn, sync::Mutex, sync::OnceCell, signal::unix, select!, time usage via reqwest"
  - "Triangle filter replaces Lanczos3 — lower CPU cost for attachment preview sizing; quality trade-off acceptable for MCP context window images"

# Metrics
duration: 7min
completed: 2026-04-05
---

# Phase 15 Plan 04: Optional Extract Feature + Triangle Filter + Narrow Tokio Summary

**Gate kreuzberg behind optional `extract` cargo feature (default-on), switch image resize from Lanczos3 to Triangle, and narrow tokio from `full` to an explicit minimum feature list**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-04-05T00:26:26Z
- **Completed:** 2026-04-05T00:33:19Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `kreuzberg` is now `optional = true` in Cargo.toml; a new `[features]` section declares `default = ["extract"]` and `extract = ["dep:kreuzberg"]`
- `extract_text()` and its private helpers (`is_image_extension`, `mime_from_filename`) are gated with `#[cfg(feature = "extract")]`; a stub implementation gated with `#[cfg(not(feature = "extract"))]` returns `Ok(Some("document extraction not enabled in this build"))`
- `cargo build --no-default-features` compiles successfully without kreuzberg (no pdfium, no bundled extraction)
- `cargo build` (default features) compiles with kreuzberg and full extraction support
- Image resize in `resize_image()` switched from `FilterType::Lanczos3` to `FilterType::Triangle` (PERF-09)
- tokio features narrowed from `full` to `["rt-multi-thread", "rt", "macros", "io-util", "sync", "time", "signal", "net"]` (PERF-11); comment above the dep documents the change with reference to this plan

## Tokio Feature Verification

| Feature | Required By |
|---------|------------|
| `rt-multi-thread` | `#[tokio::main]` multi-threaded runtime in main.rs |
| `rt` | `#[tokio::test]` single-thread test runtime, `tokio::spawn` in mcp/mod.rs |
| `macros` | `#[tokio::main]`, `#[tokio::test]`, `tokio::select!` |
| `sync` | `tokio::sync::{Mutex, OnceCell}` in mcp/graphql/mod.rs, mcp/mod.rs |
| `signal` | `tokio::signal::unix::{SignalKind, signal}`, `tokio::signal::ctrl_c()` in mcp/mod.rs |
| `io-util` | reqwest/rmcp internal use (link-time); `tokio::io::*` |
| `net` | reqwest TCP transport |
| `time` | reqwest timeout handling, rmcp |

## Task Commits

1. **Task 1: Gate kreuzberg behind optional extract feature (PERF-06)** - `4e09dcb` (feat)
2. **Task 2: Triangle filter + narrow tokio features (PERF-09, PERF-11)** - `eb16c18` (feat)

## Files Created/Modified

- `/home/kwhatcher/projects/fastmail-cli/Cargo.toml` - Optional kreuzberg dep, [features] section, narrowed tokio feature list with comment
- `/home/kwhatcher/projects/fastmail-cli/src/util.rs` - cfg-gated extract_text and helpers, FilterType::Triangle

## Decisions Made

1. **`is_image_extension` and `mime_from_filename` also gated** — both are private functions used exclusively by `extract_text`. Without the gate they would trigger unused-function lints under `--no-default-features`.

2. **`tokio::rt` added alongside `rt-multi-thread`** — `#[tokio::test]` uses the single-thread runtime from `rt`; `rt-multi-thread` alone is insufficient for tests.

3. **No call-site changes needed in download.rs or types.rs** — the stub function has the same signature as the real implementation, so both call sites compile unchanged under both feature combinations.

4. **Triangle filter choice** — Per D-14. The images being resized are attachment previews for MCP context windows. Triangle (bilinear) is significantly faster than Lanczos3 (sinc-based) with acceptable quality for this use case.

## Deviations from Plan

None — plan executed exactly as written. No additional features were required beyond the planned `["rt-multi-thread", "rt", "macros", "io-util", "sync", "time", "signal", "net"]` list.

## Verification Results

- `cargo build --no-default-features` — clean
- `cargo build` (default features) — clean
- `cargo build --release` — clean
- `cargo test` — 157 passed, 0 failed
- `cargo clippy --all-targets --all-features -- -D warnings` — clean
- `rg "Lanczos3" src/util.rs` — zero matches
- `rg 'features = \["full"\]' Cargo.toml` — zero matches
- `rg 'dep:kreuzberg' Cargo.toml` — 1 match

## Known Stubs

The no-extract stub `extract_text` returns `Ok(Some("document extraction not enabled in this build"))` — this is intentional per the plan spec and CONTEXT.md D-08 (not a gap).

## Self-Check: PASSED

- `Cargo.toml` — `optional = true` and `[features]` section present
- `src/util.rs` — `#[cfg(feature = "extract")]` on real impl, `#[cfg(not(feature = "extract"))]` on stub, `Triangle` filter present
- Commit `4e09dcb` — exists in git log
- Commit `eb16c18` — exists in git log
