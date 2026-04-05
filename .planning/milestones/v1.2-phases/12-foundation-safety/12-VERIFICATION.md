---
phase: 12-foundation-safety
verified: 2026-04-04T00:00:00Z
status: passed
score: 19/19 must-haves verified
re_verification: false
---

# Phase 12: Foundation Safety Verification Report

**Phase Goal:** The codebase has a correct safety baseline — HTTP errors surface as actionable JSON, DAV clients cannot hang indefinitely, credential Debug output is redacted, attachment downloads cannot escape their target directory, and client constructors cannot panic
**Verified:** 2026-04-04
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| #  | Truth | Status | Evidence |
|----|-------|--------|----------|
| 1  | Config printed with `{:?}` shows REDACTED for api_token | VERIFIED | `test_core_config_debug_redacts_api_token` passes; `CoreConfig.api_token: Option<SecretString>` in `src/config.rs:47` |
| 2  | Config printed with `{:?}` shows REDACTED for app_password | VERIFIED | `test_contacts_config_debug_redacts_app_password` passes; `ContactsConfig.app_password: Option<SecretString>` in `src/config.rs:59` |
| 3  | Config.save() then Config.load() round-trips the token value without corruption | VERIFIED | `test_config_serialize_deserialize` passes; custom `serialize_opt_secret_string` / `deserialize_opt_secret_string` helpers ensure TOML written as plaintext |
| 4  | Config::load() parse failure names the absolute config path and tells the user to delete or fix the file | VERIFIED | `src/config.rs:83-87` contains `"Failed to parse config at {path}: {e}. Delete this file or fix the TOML to recover."` with `path.display()`; `test_config_parse_error_includes_path_and_recovery_guidance` passes |
| 5  | CardDavClient::new() sets a 30-second HTTP timeout | VERIFIED | `src/carddav/mod.rs:82` — `.timeout(Duration::from_secs(30))`; returns `Result<Self>` |
| 6  | CalDavClient::new() sets a 30-second HTTP timeout | VERIFIED | `src/caldav/mod.rs:101` — `.timeout(std::time::Duration::from_secs(30))`; returns `Result<Self>` |
| 7  | Downloading an attachment named `../../../etc/passwd` writes `passwd` inside the output directory | VERIFIED | `safe_filename("../../../etc/passwd")` returns `"passwd"` — `test_safe_filename_strips_parent_traversal` passes |
| 8  | Downloading an attachment named `..` (no basename) writes to a file named `attachment` | VERIFIED | `safe_filename("..")` returns `"attachment"` — `test_safe_filename_dotdot_falls_back_to_attachment` passes |
| 9  | A JMAP HTTP 400 response returns Error::Server with message prefixed by `HTTP 400` | VERIFIED | `src/jmap/mod.rs:207-211` (authenticate) and `271-276` (request) both have `400..=499 =>` arm producing `"HTTP {} from API"`; `test_status_4xx_maps_to_server_error` passes |
| 10 | A JMAP HTTP 403 response returns Error::Server with message prefixed by `HTTP 403` | VERIFIED | Same arms as truth 9; test verifies 400, 403, and 404 codes all produce `"Server error: HTTP NNN from API"` via thiserror |
| 11 | A JMAP HTTP 401 response still returns Error::InvalidToken | VERIFIED | `401 =>` arm at line 204 and 268 precedes `400..=499 =>` — Rust evaluates in declared order; `test_status_401_not_captured_by_4xx_range_format` passes |
| 12 | A JMAP HTTP 429 response still returns Error::RateLimited | VERIFIED | `429 =>` arm at line 205 and 269 precedes `400..=499 =>` arm |
| 13 | JmapClient::new() returns Result<Self> and callers propagate with `?` | VERIFIED | `src/jmap/mod.rs:179` — `pub fn new(token: String) -> Result<Self>`; production callers at `jmap/mod.rs:33`, `commands/auth.rs:6`, `mcp/mod.rs:50` all use `?`; 4 test callers use `.expect("test client")` |
| 14 | `spam` without -y emits JSON `{"error": "Confirmation required: pass -y to mark email as spam"}` and exits non-zero | VERIFIED | `src/main.rs:752-753` — `Output::<()>::error("Confirmation required: pass -y to mark email as spam").print(); anyhow::bail!("confirmation required")` |
| 15 | `masked delete` without -y emits JSON error with 'masked email' wording | VERIFIED | `src/main.rs:855-856` — correct message present |
| 16 | `contacts delete` without --confirm emits JSON error with 'contact' wording | VERIFIED | `src/main.rs:915-916` — correct message present |
| 17 | `calendars delete` without --confirm emits JSON error with 'calendar' wording | VERIFIED | `src/main.rs:938-939` — correct message present |
| 18 | `events delete` without --confirm emits JSON error with 'event' wording | VERIFIED | `src/main.rs:1085-1086` — correct message present |
| 19 | No `std::process::exit(1)` call remains at any of the 5 confirmation-guard sites | VERIFIED | The only remaining `std::process::exit(1)` in `src/main.rs` is at line 1097 — the top-level error handler, not a confirmation-guard site |

**Score:** 19/19 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | secrecy dependency declaration | VERIFIED | Line 42: `secrecy = { version = "^0.10", features = ["serde"] }` |
| `src/config.rs` | Config structs with SecretString fields and recovery-guidance parse error | VERIFIED | `api_token: Option<SecretString>` (line 47), `app_password: Option<SecretString>` (line 59), recovery message at lines 83-87; 9 tests in `#[cfg(test)]` block |
| `src/carddav/mod.rs` | CardDavClient with reqwest timeout | VERIFIED | `pub fn new(...) -> Result<Self>` at line 80 with `Duration::from_secs(30)` at line 82 |
| `src/caldav/mod.rs` | CalDavClient with reqwest timeout | VERIFIED | `pub fn new(...) -> Result<Self>` at line 99 with `Duration::from_secs(30)` at line 101 |
| `src/commands/download.rs` | path-traversal-safe attachment writer | VERIFIED | `safe_filename()` at lines 11-17; `join(safe_filename(&final_filename))` at line 125; 5 unit tests |
| `src/jmap/mod.rs` | 4xx catch-all arms and fallible JmapClient::new() | VERIFIED | Two `400..=499` arms at lines 207 and 271; `pub fn new(token: String) -> Result<Self>` at line 179 |
| `src/commands/auth.rs` | caller that propagates JmapClient::new() error | VERIFIED | Line 6: `JmapClient::new(token.to_string())?` |
| `src/mcp/mod.rs` | caller that propagates JmapClient::new() error | VERIFIED | Line 50: `JmapClient::new(token)?` |
| `src/main.rs` | JSON-contract-preserving confirmation guards | VERIFIED | 5 guards replaced with `Output::<()>::error(...).print(); anyhow::bail!("confirmation required")` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/config.rs::CoreConfig.api_token` | `secrecy::SecretString` | field type | VERIFIED | `api_token: Option<SecretString>` at line 47 |
| `src/config.rs::ContactsConfig.app_password` | `secrecy::SecretString` | field type | VERIFIED | `app_password: Option<SecretString>` at line 59 |
| `src/config.rs::Config::load` | user guidance | error message | VERIFIED | `"Delete this file or fix the TOML to recover."` at line 84 |
| `src/carddav/mod.rs::CardDavClient::new` | reqwest timeout | Client::builder | VERIFIED | `.timeout(Duration::from_secs(30))` at line 82 inside `Client::builder()` chain |
| `src/caldav/mod.rs::CalDavClient::new` | reqwest timeout | Client::builder | VERIFIED | `.timeout(std::time::Duration::from_secs(30))` at line 101 inside `Client::builder()` chain |
| `src/commands/download.rs` | safe filename | Path::file_name | VERIFIED | `.file_name()` at line 13; used at line 125 via `join(safe_filename(...))` |
| `src/jmap/mod.rs::authenticate` | Error::Server for 4xx | match arm | VERIFIED | Lines 207-212: `400..=499 =>` arm with `Error::Server` |
| `src/jmap/mod.rs::request` | Error::Server for 4xx | match arm | VERIFIED | Lines 271-276: `400..=499 =>` arm with `Error::Server` |
| `src/jmap/mod.rs::JmapClient::new` | Result<Self> | signature | VERIFIED | `pub fn new(token: String) -> Result<Self>` at line 179 |
| `src/main.rs confirmation guards` | Output::error().print() + anyhow::bail!() | replacement pattern | VERIFIED | All 5 guards at lines 752-753, 855-856, 915-916, 938-939, 1085-1086 use the new pattern |

---

### Data-Flow Trace (Level 4)

Not applicable — this phase produces no components that render dynamic data. All artifacts are security/stability primitives (client constructors, error paths, serialization wrappers).

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All phase tests pass | `cargo test` (116 tests) | `test result: ok. 116 passed; 0 failed` | PASS |
| Config redaction tests pass | `cargo test config::tests` | 9 passed; 0 failed | PASS |
| Download safety tests pass | `cargo test download` | 5 passed; 0 failed | PASS |
| JMAP 4xx tests pass | `cargo test jmap::tests::test_status` | 2 passed; 0 failed | PASS |
| DAV constructor tests pass | `cargo test caldav::tests caldav::tests` | 2 passed; 0 failed | PASS |
| Clippy clean | `cargo clippy --all-targets -- -D warnings` | `Finished` with 0 warnings | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| SEC-01 | 12-02 | Attachment downloads never write outside output directory | SATISFIED | `safe_filename()` in `src/commands/download.rs`; `join(safe_filename(...))` at line 125; 5 tests cover traversal, absolute path, dotdot, empty, and plain name |
| SEC-06 | 12-01 | Config structs never print secrets via `{:?}` | SATISFIED | `api_token` and `app_password` are `Option<SecretString>`; 2 debug-redaction tests pass |
| STAB-01 | 12-03 | JMAP HTTP 4xx responses produce clear Server error | SATISFIED | Two `400..=499` match arms in `authenticate()` and `request()`; arm order preserves 401 and 429 specificity |
| STAB-02 | 12-02 | CardDavClient and CalDavClient set 30-second HTTP timeout | SATISFIED | Both constructors use `Client::builder().timeout(Duration::from_secs(30)).build()?` |
| STAB-03 | 12-04 | Confirmation-guard exit paths emit valid JSON envelope | SATISFIED | All 5 guard sites use `Output::<()>::error(...).print(); anyhow::bail!("confirmation required")` |
| STAB-09 | 12-03 | JmapClient::new() returns Result instead of panicking | SATISFIED | Signature changed to `-> Result<Self>`; `.expect()` removed; all callers use `?` or `.expect("test client")` |
| STAB-10 | 12-01 | Config corruption errors include recovery guidance | SATISFIED | `Config::load()` error message includes absolute path via `path.display()` and the recovery instruction string |

No orphaned requirements — all 7 requirement IDs declared across the 4 plans are accounted for.

---

### Anti-Patterns Found

No blockers or warnings found.

- The `std::process::exit(1)` at `src/main.rs:1097` is the top-level program error handler, not a confirmation-guard site. This is the correct and expected location for this call.
- No `TODO`, `FIXME`, or placeholder comments in any modified file.
- No `eprintln!` calls remaining at any of the 5 confirmation-guard sites.
- No `Client::new()` (unconfigured client) in `src/carddav/mod.rs` or `src/caldav/mod.rs`.

---

### Human Verification Required

None. All phase-12 must-haves are verifiable programmatically through code inspection and test execution.

---

### Gaps Summary

No gaps. All 7 requirement IDs (STAB-01, STAB-02, STAB-03, STAB-09, STAB-10, SEC-01, SEC-06) are fully implemented with passing tests. The codebase compiles cleanly with zero clippy warnings and all 116 tests pass.

---

_Verified: 2026-04-04_
_Verifier: Claude (gsd-verifier)_
