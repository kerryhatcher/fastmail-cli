---
phase: 14-mcp-layer-refactor
plan: 01
subsystem: api
tags: [rust, hmac, sha2, rand_core, tokio-util, async-graphql, oncecell, graphql]

requires:
  - phase: 12-foundation-safety
    provides: CardDavClient::new/CalDavClient::new as Result<Self>
  - phase: 13-security-hardening
    provides: confirmation token infrastructure in mutation.rs

provides:
  - AppContext struct replacing JmapContext in src/mcp/graphql/mod.rs
  - HMAC-SHA256 confirmation_token method with per-process OsRng key (SEC-05 infra)
  - Lazy-init OnceCell DAV clients (PERF-03 infra)
  - require_jmap consolidated helper on AppContext
  - GraphQL depth limit 5 + complexity limit 200 (SEC-07)
  - Temporary JmapContext shim preserving downstream compilation

affects: [14-02, 14-03, 14-04]

tech-stack:
  added:
    - hmac 0.12 (HMAC-SHA256 token generation)
    - sha2 0.10 (SHA-256, direct dep for import)
    - rand_core 0.9 with os_rng feature (OsRng::try_fill_bytes)
    - tokio-util 0.7 with rt feature (CancellationToken, used by plan 04)
  patterns:
    - AppContext injected via schema.data() once at startup; resolvers use ctx.data::<AppContext>()
    - OnceCell lazy-init for shared DAV clients (eliminates TLS handshake per request)
    - HMAC-SHA256 with length-prefixed parts for collision-resistant confirmation tokens
    - TDD with failing tests committed before implementation

key-files:
  created: []
  modified:
    - Cargo.toml (new deps: hmac, sha2, rand_core, tokio-util)
    - src/mcp/graphql/mod.rs (AppContext struct + impl + tests; build_schema with limits)
    - src/mcp/mod.rs (FastmailMcp::new calls AppContext::new)

key-decisions:
  - "Used rand_core::TryRngCore::try_fill_bytes (not fill_bytes) for rand_core 0.9 OsRng - CONTEXT.md D-06 had wrong method name"
  - "Added #[allow(dead_code)] on AppContext struct+impl to suppress forward-declared API warnings until plans 02-04 use them"
  - "Injected both AppContext and JmapContext shim in build_schema so unmigrated resolvers compile until plan 02"
  - "Used .limit_depth(5).limit_complexity(200) not .depth_limit/.complexity - CONTEXT.md D-09 had wrong method names"

patterns-established:
  - "AppContext pattern: shared server context injected once via schema.data(), extracted via ctx.data::<AppContext>()"
  - "HMAC length-prefix: each part prefixed by 8-byte LE length to prevent concatenation collisions"
  - "TDD for cryptographic code: tests prove HMAC determinism, randomness across instances, collision resistance, output format"

requirements-completed: [PERF-03, SEC-05, SEC-07]

duration: 4min
completed: 2026-04-04
---

# Phase 14 Plan 01: AppContext Foundation Summary

**AppContext struct with HMAC-SHA256 confirmation tokens (OsRng key), OnceCell lazy DAV init, and GraphQL depth/complexity limits replacing bare JmapContext**

## Performance

- **Duration:** 4 min
- **Started:** 2026-04-04T23:59:24Z
- **Completed:** 2026-04-04T23:59:24Z
- **Tasks:** 2 (executed as single TDD cycle)
- **Files modified:** 3

## Accomplishments

- AppContext replaces JmapContext with jmap Mutex, carddav/caldav OnceCells, and 32-byte HMAC key
- Per-process random HMAC key generated via OsRng::try_fill_bytes at server startup; invalidated on restart (SEC-05 infra)
- confirmation_token() HMAC-SHA256 with length-prefix encoding prevents ["a","bc"] == ["ab","c"] collision
- build_schema applies .limit_depth(5).limit_complexity(200) bounding GraphQL query cost (SEC-07)
- Temporary JmapContext shim keeps types.rs/query.rs/mutation.rs compiling; removed in plan 02
- 6 unit tests proving HMAC determinism, OsRng randomness, collision resistance, hex output, SDL generation

## Task Commits

1. **Tasks 1+2: AppContext struct + DAV helpers + schema limits** - `543c903` (feat)

**Plan metadata:** TBD (docs commit)

## Files Created/Modified

- `Cargo.toml` - Added hmac 0.12, sha2 0.10, rand_core 0.9 (os_rng), tokio-util 0.7 (rt)
- `src/mcp/graphql/mod.rs` - AppContext struct+impl, build_schema with limits, JmapContext shim, 6 unit tests
- `src/mcp/mod.rs` - Updated FastmailMcp::new to call AppContext::new(jmap_client)

## Decisions Made

- Used `rand_core::TryRngCore::try_fill_bytes` (not `fill_bytes`) for rand_core 0.9 — CONTEXT.md D-06 referenced the wrong method name; RESEARCH.md correctly identified this
- Applied `#[allow(dead_code)]` on AppContext struct and impl block because methods are forward-declared APIs for plans 02-04; `-D warnings` in clippy would otherwise treat them as errors
- Injected both `AppContext` and temporary `JmapContext` shim in `build_schema` to maintain compilation of unmigrated resolvers through plan 01 scope
- Used `.limit_depth(5).limit_complexity(200)` — CONTEXT.md D-09 incorrectly specified `.depth_limit().complexity()`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added #[allow(dead_code)] to AppContext struct and impl**
- **Found during:** Task 2 (clippy verification)
- **Issue:** `cargo clippy --all-targets -- -D warnings` fails on forward-declared public APIs not yet used by unmigrated resolvers; `-D dead-code` treats unused pub items as errors
- **Fix:** Added `#[allow(dead_code)]` attribute to AppContext struct definition and impl block; comment explains methods will be used by plans 02-04
- **Files modified:** src/mcp/graphql/mod.rs
- **Verification:** `cargo clippy --all-targets -- -D warnings` exits 0
- **Committed in:** 543c903 (task commit)

---

**Total deviations:** 1 auto-fixed (Rule 2 - missing critical)
**Impact on plan:** Required for clippy to pass. No scope creep — attribute will be removed when plans 02-04 use the methods.

## Issues Encountered

- CONTEXT.md D-06 and D-09 had incorrect API method names (`fill_bytes`, `depth_limit`, `complexity`). RESEARCH.md correctly flagged these as Pitfalls 1 and 3 with the correct alternatives. Plan instructions used the corrected names from RESEARCH.md throughout.

## Next Phase Readiness

- AppContext fully wired into build_schema and FastmailMcp::new
- Plan 02 can migrate types.rs/query.rs/mutation.rs to read AppContext instead of JmapContext and remove the shim
- confirmation_token() on AppContext ready to replace free function in types.rs (plan 02/03)
- get_carddav/get_caldav helpers ready to replace per-request CardDavClient::new/CalDavClient::new in query.rs (plan 02)

---
*Phase: 14-mcp-layer-refactor*
*Completed: 2026-04-04*
