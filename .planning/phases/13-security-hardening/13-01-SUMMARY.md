---
phase: 13-security-hardening
plan: "01"
subsystem: carddav
tags: [security, vcard, injection, escaping, sec-02]
dependency_graph:
  requires: []
  provides: [SEC-02-closed]
  affects: [src/carddav/mod.rs]
tech_stack:
  added: []
  patterns: [escape_value applied to EMAIL/TEL value and label parameters]
key_files:
  modified:
    - src/carddav/mod.rs
decisions:
  - "Applied escape_value() to EMAIL/TEL value and label on serialize path — matches existing FN/N/ORG/TITLE/ADR/NOTE pattern, no new dependencies"
metrics:
  duration: "3 minutes"
  completed: "2026-04-04T23:06:42Z"
  tasks_completed: 2
  files_modified: 1
requirements:
  - SEC-02
---

# Phase 13 Plan 01: vCard EMAIL/TEL Injection Hardening Summary

**One-liner:** Closed SEC-02 by applying `escape_value()` to EMAIL and TEL property values and TYPE label parameters in `serialize_vcard`, backed by four unit tests proving newline/semicolon/comma/backslash injection is blocked.

## Tasks Completed

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Add escaping to EMAIL and TEL value + label in serialize_vcard | 3c1f136 | src/carddav/mod.rs |
| 2 | Unit tests for EMAIL/TEL injection prevention | 2da7818 | src/carddav/mod.rs |

## What Was Built

`serialize_vcard` in `src/carddav/mod.rs` previously emitted raw user-supplied data for EMAIL and TEL property values and TYPE label parameters, allowing property injection via embedded newlines. The fix applies the existing `escape_value()` helper (already used for FN/N/ORG/TITLE/ADR/NOTE) to these four remaining call sites.

### Changes in src/carddav/mod.rs (lines 747-762)

Before:
```rust
lines.push(format!("EMAIL;TYPE={}:{}", label, email.email));
lines.push(format!("EMAIL:{}", email.email));
lines.push(format!("TEL;TYPE={}:{}", label, phone.number));
lines.push(format!("TEL:{}", phone.number));
```

After:
```rust
lines.push(format!("EMAIL;TYPE={}:{}", escape_value(label), escape_value(&email.email)));
lines.push(format!("EMAIL:{}", escape_value(&email.email)));
lines.push(format!("TEL;TYPE={}:{}", escape_value(label), escape_value(&phone.number)));
lines.push(format!("TEL:{}", escape_value(&phone.number)));
```

### Tests Added (4 new, src/carddav/mod.rs)

- `test_vcard_email_newline_escaped` — newline in email value cannot produce a new property line
- `test_vcard_email_semicolon_escaped` — semicolon in email escapes to `\;`
- `test_vcard_tel_special_chars_escaped` — `;`, `,`, `\`, `n` in phone number all escape correctly
- `test_vcard_label_injection_blocked` — newline in TYPE label cannot inject a `MALICIOUS:` property

## Verification

- `cargo build` exits 0
- `cargo test -- carddav::tests` — 51 passed, 0 failed
- `cargo clippy -- -D warnings` — clean, no warnings

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED

- src/carddav/mod.rs: modified (verified grep matches all 4 escape_value call sites)
- Commit 3c1f136: exists (Task 1 — implementation)
- Commit 2da7818: exists (Task 2 — tests)
- 51 carddav tests passing
- clippy clean
