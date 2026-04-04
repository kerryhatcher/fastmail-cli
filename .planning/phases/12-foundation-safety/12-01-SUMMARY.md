---
phase: 12-foundation-safety
plan: "01"
subsystem: config
tags: [rust, secrecy, secret-redaction, serde, config]

# Dependency graph
requires: []
provides:
  - "secrecy v0.10.3 dependency in Cargo.toml"
  - "api_token and app_password wrapped in SecretString — Debug output redacts credentials"
  - "Config::load() parse errors include absolute config path and recovery instructions"
  - "Custom serde helpers for Option<SecretString> enabling round-trip TOML serialization"
affects:
  - 12-02-foundation-safety
  - any phase reading Config struct fields directly

# Tech tracking
tech-stack:
  added:
    - "secrecy 0.10.3 — SecretString wrapper with automatic Debug redaction and zeroize-on-drop"
  patterns:
    - "Custom serialize_opt_secret_string/deserialize_opt_secret_string for SecretString TOML round-trip"
    - "expose_secret() called only at accessor boundary (get_token, get_app_password) — narrow blast radius"
    - "SecretBox<str> (SecretString) cannot use secrecy serde feature for Serialize — custom helper required"

key-files:
  created: []
  modified:
    - "Cargo.toml — added secrecy = { version = \"^0.10\", features = [\"serde\"] }"
    - "Cargo.lock — secrecy v0.10.3 + zeroize transitive dependency"
    - "src/config.rs — SecretString fields, custom serde helpers, improved parse error, unit tests"
    - "src/commands/events.rs — clippy fix: needless_question_mark in calendar_client()"

key-decisions:
  - "Used custom serde helpers (serialize_opt_secret_string/deserialize_opt_secret_string) because secrecy 0.10 SecretString = SecretBox<str> and str: !SerializableSecret — the serde feature only provides Deserialize, not Serialize for SecretString"
  - "Kept get_token/get_app_password/set_token signatures unchanged — expose_secret() boundary stays inside Config impl"
  - "Parallel 12-02 executor committed src/config.rs SecretString implementation as a blocking fix — 12-01 commit added the missing Cargo.toml dependency"

patterns-established:
  - "Pattern: SecretString in Config structs uses custom serde module with serialize_opt_secret_string + deserialize_opt_secret_string helpers"
  - "Pattern: Debug redaction via SecretBox<str> automatic impl — produces SecretBox<str>([REDACTED])"
  - "Pattern: Config parse errors include path.display() and actionable recovery guidance"

requirements-completed: [SEC-06, STAB-10]

# Metrics
duration: 10min
completed: "2026-04-04"
---

# Phase 12 Plan 01: Foundation Safety — Config Secret Redaction Summary

**api_token and app_password wrapped in secrecy::SecretString with custom TOML serde helpers, Debug output redacts credentials, Config::load() parse errors include absolute config path and recovery guidance**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-04-04T22:30:29Z
- **Completed:** 2026-04-04T22:40:19Z
- **Tasks:** 2 (executed together — both target src/config.rs)
- **Files modified:** 4

## Accomplishments

- `api_token` and `app_password` fields changed from `Option<String>` to `Option<SecretString>` — `format!("{:?}", config)` no longer leaks credentials
- Custom `serialize_opt_secret_string` / `deserialize_opt_secret_string` helpers enable round-trip TOML serialization without writing `[REDACTED]` to disk
- `Config::load()` parse errors now include the absolute config file path and tell the user to delete or fix the file to recover
- 9 unit tests in `config::tests` — all pass, covering redaction, round-trip, accessor boundaries, and parse error substrings
- Full test suite: 116 tests pass, zero clippy warnings

## Task Commits

1. **Tasks 1 + 2: SecretString fields, parse error improvement, secrecy dep** - `2a30066` (feat)
   - Note: src/config.rs implementation was committed in the parallel 12-02 executor commit `a8fde42` as a blocking fix. This commit adds the Cargo.toml dependency and Cargo.lock that were missing.

## Files Created/Modified

- `/home/kwhatcher/projects/fastmail-cli/Cargo.toml` — added `secrecy = { version = "^0.10", features = ["serde"] }`
- `/home/kwhatcher/projects/fastmail-cli/Cargo.lock` — secrecy v0.10.3 + zeroize transitive dep
- `/home/kwhatcher/projects/fastmail-cli/src/config.rs` — SecretString fields, custom serde helpers, updated accessors, improved parse error message, 9 unit tests
- `/home/kwhatcher/projects/fastmail-cli/src/commands/events.rs` — clippy fix: `CalDavClient::new(...)` without redundant `Ok(...?)`

## Decisions Made

- **Custom serde helpers required (deviation from research):** `secrecy` 0.10.3 defines `SecretString = SecretBox<str>`. The `serde` feature only provides `Deserialize` for `SecretBox<T: DeserializeOwned>`. For `Serialize`, the type `T` must implement `SerializableSecret` — but `str: !Sized` and `str` has no `SerializableSecret` impl. Used `serialize_opt_secret_string` calling `.expose_secret()` explicitly (intentional — config file must store plaintext for round-trip correctness).

- **Narrow expose_secret() boundary maintained:** `get_token()` and `get_app_password()` remain `Result<String>`, calling `.expose_secret().to_string()` internally. Callers receive `String`, never `SecretString`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed pre-existing compile errors from partial STAB-02/STAB-03 implementation**
- **Found during:** Task 1 (initial build attempt)
- **Issue:** `async fn main()` returned `()` but match arms used `anyhow::bail!()` returning `anyhow::Result<()>`; `mcp/graphql/query.rs` called DAV constructors without `?` on `Result<Self>`
- **Fix:** Changed `async fn main()` to return `anyhow::Result<()>` and added `Ok(())` at end; fixed query.rs (already auto-fixed by IDE linter); fixed events.rs/calendars.rs clippy lint
- **Files modified:** `src/main.rs`, `src/commands/events.rs`, `src/commands/calendars.rs`
- **Verification:** `cargo build` succeeds, all 116 tests pass
- **Committed in:** `2a30066` (part of plan commit, also in `a8fde42` parallel executor)

**2. [Rule 1 - Bug] Custom serde serializer for SecretString — research inaccuracy**
- **Found during:** Task 1 (GREEN phase build)
- **Issue:** Research stated secrecy `serde` feature enables "seamless TOML deserialization" for SecretString. In secrecy 0.10.3, `SecretString = SecretBox<str>` and `str` does not implement `SerializableSecret`, making `#[derive(Serialize)]` fail to compile
- **Fix:** Implemented `serialize_opt_secret_string` + `deserialize_opt_secret_string` custom serde functions; kept the `serde` feature for `Deserialize` support
- **Files modified:** `src/config.rs`
- **Verification:** TOML round-trip test (`test_config_serialize_deserialize`) passes; serialized TOML contains raw token (not `[REDACTED]`)
- **Committed in:** `a8fde42` (parallel executor), `2a30066` (Cargo.toml dep)

---

**Total deviations:** 2 auto-fixed (1 blocking pre-existing, 1 research inaccuracy requiring alternative implementation)
**Impact on plan:** Both fixes were necessary. The custom serde approach satisfies all plan requirements — the key goal (Debug redaction via SecretString) is fully achieved.

## Issues Encountered

- Parallel plan executor (12-02) committed `src/config.rs` SecretString changes but omitted the `Cargo.toml` `secrecy` dependency entry. The repo HEAD was in a broken state (build would fail without the dep). This plan's commit (`2a30066`) restored the missing entry.

## Next Phase Readiness

- SEC-06 satisfied: `{:?}` on any Config struct redacts credentials
- STAB-10 satisfied: parse errors are actionable with path and recovery guidance
- Existing callers of `get_token()`, `get_app_password()`, `set_token()` unchanged — no cascading refactor needed

## Known Stubs

None — no stubs or placeholders introduced.

---
*Phase: 12-foundation-safety*
*Completed: 2026-04-04*
