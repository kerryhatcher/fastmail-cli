---
phase: 13-security-hardening
verified: 2026-04-04T23:45:00Z
status: passed
score: 12/12 must-haves verified
re_verification: false
---

# Phase 13: Security Hardening Verification Report

**Phase Goal:** All user-supplied string data is escaped or validated before being written into vCard and iCalendar wire format, blob download URLs are correctly percent-encoded, and the auth token input surface is documented for multi-user safety

**Verified:** 2026-04-04T23:45:00Z
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A vCard EMAIL value containing a newline cannot inject an additional vCard property line | VERIFIED | `escape_value()` applied at `src/carddav/mod.rs:749,751`; `test_vcard_email_newline_escaped` passes |
| 2 | A vCard TEL value containing a colon or semicolon is escaped in the serialized output | VERIFIED | `escape_value()` applied at `src/carddav/mod.rs:758,760`; `test_vcard_tel_special_chars_escaped` passes |
| 3 | A vCard EMAIL/TEL label containing ';' or newline cannot inject a new parameter or property line | VERIFIED | `escape_value(label)` wraps TYPE parameter at lines 749 and 758; `test_vcard_label_injection_blocked` passes |
| 4 | An iCalendar ATTENDEE email containing special characters does not break serialized VCALENDAR syntax | VERIFIED | `escape_ical_value(&attendee.email)` at `src/caldav/mod.rs:1401`; `test_attendee_email_newline_escaped` passes |
| 5 | An iCalendar ATTENDEE ROLE/PARTSTAT is validated against a known enum; unknown values are dropped | VERIFIED | `sanitize_role()` and `sanitize_partstat()` at lines 1362/1372, called in `serialize_attendee` at lines 1392/1395; tests pass |
| 6 | An iCalendar RRULE UNTIL value containing non-datetime characters is rejected or stripped | VERIFIED | `is_valid_rrule_until()` at line 1328 guards UNTIL emission in `serialize_rrule`; `test_rrule_until_invalid_dropped` passes |
| 7 | An iCalendar RRULE FREQ and BYDAY are validated against the RFC 5545 grammar | VERIFIED | `is_valid_freq()` at line 1320, `is_valid_byday()` at line 1339, both wired in `serialize_rrule`; tests pass |
| 8 | A JMAP blob download URL with a filename containing spaces or Unicode produces a correctly percent-encoded URL | VERIFIED | `encode_blob_url_segment()` at `src/jmap/mod.rs:27`; called in `download_blob` at lines 882-883; `test_encode_space_and_dot` and `test_encode_unicode` pass |
| 9 | The percent-encoding set is NON_ALPHANUMERIC (strictest safe set) | VERIFIED | `utf8_percent_encode(s, NON_ALPHANUMERIC)` at `src/jmap/mod.rs:28` |
| 10 | The percent-encoding crate is a dependency | VERIFIED | `percent-encoding = "2.3"` in `Cargo.toml:52` |
| 11 | The `auth` command no longer accepts the API token as a positional argument | VERIFIED | `Auth,` (no fields) at `src/main.rs:28`; dispatch `Commands::Auth => commands::auth().await` at line 664 |
| 12 | The auth command reads the token from FASTMAIL_API_TOKEN env var, or from interactive stdin prompt as fallback | VERIFIED | `resolve_token()` in `src/commands/auth.rs` checks `std::env::var("FASTMAIL_API_TOKEN")` first, then `stdin.is_terminal()` guard before interactive read |

**Score:** 12/12 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/carddav/mod.rs` | `serialize_vcard` applies `escape_value()` to EMAIL value, TEL value, and label parameters | VERIFIED | All 4 call sites confirmed at lines 749, 751, 758, 760; `contains: "escape_value(&email.email)"` matched |
| `src/caldav/mod.rs` | `serialize_attendee` escapes email; `serialize_rrule` validates FREQ/UNTIL/BYDAY | VERIFIED | `escape_ical_value` applied at line 1401; validators `is_valid_freq`, `is_valid_rrule_until`, `is_valid_byday`, `sanitize_role`, `sanitize_partstat` all present and wired |
| `Cargo.toml` | `percent-encoding = ^2.3` dependency | VERIFIED | `percent-encoding = "2.3"` found at line 52 |
| `src/jmap/mod.rs` | `download_blob()` percent-encodes `{name}` and `{blobId}` substitutions | VERIFIED | `encode_blob_url_segment(blob_id)` at line 882, `encode_blob_url_segment("attachment")` at line 883; `utf8_percent_encode` import confirmed |
| `src/main.rs` | `Commands::Auth` variant has NO token field | VERIFIED | `Auth,` with no fields at line 28; doc comment present |
| `src/commands/auth.rs` | `auth()` reads token from env or stdin | VERIFIED | `FASTMAIL_API_TOKEN` env var check + `is_terminal()` guard + `resolve_token()` function all present |
| `README.md` | Authentication section documents env var + `read -rs` pattern | VERIFIED | 6 occurrences of `FASTMAIL_API_TOKEN`; `read -rs TOKEN` present; `v1.2 breaking change` migration note present; old positional form absent from instructions |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/carddav/mod.rs::serialize_vcard` | `src/carddav/mod.rs::escape_value` | Direct call inside `format!` for EMAIL and TEL lines | WIRED | Pattern `escape_value(&email.email)` confirmed at lines 749, 751, 758, 760 |
| `src/caldav/mod.rs::serialize_attendee` | `src/caldav/mod.rs::escape_ical_value` | Direct call on email | WIRED | `format!("{prop}:mailto:{}", escape_ical_value(&attendee.email))` at line 1401 |
| `src/caldav/mod.rs::serialize_attendee` | `sanitize_role` / `sanitize_partstat` | `.and_then(sanitize_role/sanitize_partstat)` | WIRED | Lines 1392-1395; invalid values silently dropped |
| `src/caldav/mod.rs::serialize_rrule` | RFC 5545 grammar validators | `is_valid_rrule_until` / `is_valid_freq` guards | WIRED | `serialize_rrule` at lines 1424-1452 calls all three validators |
| `src/jmap/mod.rs::download_blob` | `percent_encoding::utf8_percent_encode` | `NON_ALPHANUMERIC` set applied in `encode_blob_url_segment` before URL template substitution | WIRED | `encode_blob_url_segment()` free function at line 27 called from `download_blob` at lines 882-883 |
| `src/main.rs::Commands::Auth dispatch` | `src/commands/auth.rs::auth` | Call with no arguments | WIRED | `Commands::Auth => commands::auth().await` at line 664 |
| `src/commands/auth.rs::auth` | `std::env::var` + `std::io::stdin` | Env-first, stdin-fallback token resolution | WIRED | `resolve_token()` checks `std::env::var(ENV_VAR)`, then `stdin.is_terminal()` guard |

---

### Data-Flow Trace (Level 4)

Not applicable — this phase produced security hardening logic (escaping, validation, encoding helpers), not UI components or data-rendering artifacts.

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| All 131 tests pass (includes all 4 SEC-02 tests, 7 SEC-03 tests, 4 SEC-09 tests) | `cargo test` | `131 passed; 0 failed` | PASS |
| Project builds without errors | `cargo build` | `BUILD OK` | PASS |
| `encode_blob_url_segment("hello world.pdf")` returns `hello%20world%2Epdf` | `test_encode_space_and_dot` | pass | PASS |
| `encode_blob_url_segment("日本.txt")` returns `%E6%97%A5%E6%9C%AC%2Etxt` | `test_encode_unicode` | pass | PASS |
| `serialize_vcard` with newline in EMAIL produces no extra property line | `test_vcard_email_newline_escaped` | pass | PASS |
| iCal attendee with `INJECT;X` role produces no ROLE= parameter | `test_attendee_role_invalid_dropped` | pass | PASS |
| iCal RRULE with invalid UNTIL is dropped | `test_rrule_until_invalid_dropped` | pass | PASS |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| SEC-02 | 13-01-PLAN.md | vCard EMAIL/TEL labels and values are escaped/validated so malicious labels cannot inject additional vCard properties | SATISFIED | `escape_value()` applied to all 4 EMAIL/TEL emit sites; 4 unit tests pass; REQUIREMENTS.md marked `[x]` |
| SEC-03 | 13-02-PLAN.md | iCalendar attendee fields and RRULE fields are validated or escaped during serialization | SATISFIED | 5 validator helpers exist and wired; `serialize_attendee` and `serialize_rrule` hardened; 7 unit tests pass; REQUIREMENTS.md marked `[x]` |
| SEC-04 | 13-04-PLAN.md | The `auth` command accepts token via stdin, env var, or interactive prompt — never positional CLI arg | SATISFIED | `Auth,` has no fields; `resolve_token()` implements env-first/stdin-fallback; README documents new patterns; REQUIREMENTS.md marked `[x]` |
| SEC-09 | 13-03-PLAN.md | Blob-download URL template values are URL-encoded before insertion into JMAP download URLs | SATISFIED | `encode_blob_url_segment()` with `NON_ALPHANUMERIC` applied to `{blobId}` and `{name}`; `percent-encoding = "2.3"` in Cargo.toml; 4 unit tests pass; REQUIREMENTS.md marked `[x]` |

No orphaned requirements found — all phase-13 mapped requirements (SEC-02, SEC-03, SEC-04, SEC-09) are claimed in plan frontmatter and verified in the codebase.

---

### Anti-Patterns Found

No blocking or warning-level anti-patterns found.

Scan notes:
- No `TODO/FIXME/PLACEHOLDER` comments introduced in phase-13 files
- No `return null` / `return {}` / empty handler stubs
- No hardcoded empty data arrays flowing to rendering paths
- The `serialize_rrule` fallback `String::new()` for invalid FREQ is intentional documented behavior (caller responsibility), not a stub
- `encode_blob_url_segment("attachment")` encodes to `"attachment"` (unchanged) — this is correct; the plan notes the real filename parameter is deferred to D-07

---

### Human Verification Required

The following behavior cannot be confirmed programmatically (requires a live Fastmail connection per project MEMORY constraints):

**1. Auth command non-interactive failure path**

**Test:** Run `echo "" | cargo run -- auth` (piped stdin, no `FASTMAIL_API_TOKEN` set)
**Expected:** Exits non-zero; JSON output contains an error message referencing `FASTMAIL_API_TOKEN` env var and the `read -rs` pattern
**Why human:** Requires actually running the binary; cannot test live API calls per project memory constraints

**2. Auth command env var path**

**Test:** Run `FASTMAIL_API_TOKEN=test_token cargo run -- auth` in a network-isolated environment
**Expected:** Attempts authentication with "test_token"; fails with an auth error from Fastmail (not a token-resolution error)
**Why human:** Requires live network to confirm the token reaches the JMAP endpoint correctly

---

### Gaps Summary

No gaps. All 12 observable truths verified, all artifacts substantive and wired, all 4 required requirements satisfied, 131 tests passing, build clean.

---

_Verified: 2026-04-04T23:45:00Z_
_Verifier: Claude (gsd-verifier)_
