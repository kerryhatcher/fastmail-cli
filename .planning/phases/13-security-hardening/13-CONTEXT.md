# Phase 13: Security Hardening - Context

**Gathered:** 2026-04-04
**Status:** Ready for planning
**Mode:** Smart discuss (autonomous)

<domain>
## Phase Boundary

All user-supplied string data is escaped or validated before being written into vCard and iCalendar wire format, blob download URLs are correctly percent-encoded, and the auth token input surface is documented for multi-user safety.

Requirements: SEC-02 (vCard injection), SEC-03 (iCal injection), SEC-04 (URL encoding), SEC-09 (auth input surface).

</domain>

<decisions>
## Implementation Decisions

### Escaping Strategy

- **D-01**: Extend existing `escape_value()` in `src/carddav/mod.rs` usage to EMAIL and TEL property values. Currently `escape_value()` is only applied to FN, N, ORG, TITLE, ADR, NOTE — EMAIL and TEL are formatted with raw values at lines 749, 751, 760. Apply the same escape to keep the pattern uniform.
- **D-02**: Add a matching iCalendar `escape_text()` helper to `src/caldav/mod.rs` that escapes `\\`, `\;`, `\,`, and `\n` per RFC 5545 §3.3.11. Hand-rolled, no new crate dependency — mirrors the carddav pattern the project already uses.
- **D-03**: Apply iCal escaping to user-supplied fields in VEVENT serialization: SUMMARY, LOCATION, DESCRIPTION, ATTENDEE CN, URL value. Also guard RRULE `UNTIL` by rejecting/filtering non-datetime characters (UNTIL is not a free-form string — validate format).
- **D-04**: Escape-only approach (no rejection) — if a user legitimately puts a newline in a NOTE, we escape it. Rejection is reserved for structural fields like UNTIL that must match a grammar.

### URL Encoding

- **D-05**: Add `percent-encoding` crate (^2.3) — small, standard, stable API.
- **D-06**: Use `NON_ALPHANUMERIC` encode set for filename segment in blob download URLs (strictest, safest). Percent-encode the `{name}` substitution in `JmapClient::download_blob()` at `src/jmap/mod.rs:869-874` before URL construction.
- **D-07**: Currently `{name}` is hardcoded to `"attachment"`. Change to pass the user-intended filename (or fallback) through percent-encoding. Keep existing `{type}` hardcoded for now — out of phase scope.

### Auth Command (SEC-09)

- **D-08**: Remove positional `token` argument from `Auth` variant in `src/main.rs:28-31`. Breaking change — documented in README migration notes.
- **D-09**: `auth` command reads token in this priority order: (1) `FASTMAIL_API_TOKEN` env var, (2) interactive stdin prompt via `rpassword` or `std::io::stdin` with terminal echo off. If both unavailable (non-interactive context, env var not set), fail with actionable error message.
- **D-10**: README documents both `FASTMAIL_API_TOKEN=...` env var usage and shell pattern `read -rs TOKEN && FASTMAIL_API_TOKEN=$TOKEN fastmail-cli auth` for multi-user safety.

### Claude's Discretion

- Exact error message wording for stdin-unavailable case
- Whether to add `rpassword` crate (for hidden stdin) vs use plain `stdin().read_line()`
- Test strategy for stdin-based auth (likely integration test with pipe)

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets

- `escape_value()` in `src/carddav/mod.rs:621-626` — already implements RFC 2426 §5 escaping
- `unescape_value()` at line 631 — round-trip validated via existing tests
- `Error::Config(String)` variant — suitable for missing-token errors
- `Output::<()>::error(...).print()` + `anyhow::bail!()` pattern from Phase 12 for CLI errors

### Established Patterns

- vCard/iCal builders use `lines.push(format!("PROP:{}", ...))` style — easy to drop `escape_value()` or `escape_text()` into the format args
- Hand-rolled protocol code (no generated serializers) — consistent with RFC-compliance philosophy
- `reqwest::Url` parsing done downstream — percent-encoding must happen BEFORE passing string to `.get()`

### Integration Points

- `src/carddav/mod.rs` contact serialization (`create_contact`, `update_contact` paths around lines 717-785)
- `src/caldav/mod.rs` event serialization (VEVENT builder around line 815, ATTENDEE at line 1320)
- `src/jmap/mod.rs:864` `download_blob()` URL construction
- `src/main.rs:28-31` (Auth command variant) and `src/main.rs:667` (dispatch)
- `src/commands/auth.rs` (auth handler)
- `README.md` (auth documentation)

</code_context>

<specifics>
## Specific Ideas

- Preserve existing `escape_value()` function — do not rename or move it
- Add unit tests for each escape edge case: newline, colon, semicolon, backslash in EMAIL/TEL
- Add unit test asserting blob URL with spaces/Unicode produces correctly-encoded output
- Add integration-style test for auth command missing token (both env and stdin)

</specifics>

<deferred>
## Deferred Ideas

- Migrating entire vCard/iCal serialization to a library (icalendar crate) — phase 13 scope is escaping, not rewrite
- Adding rate limiting or token rotation — out of scope
- Encrypting token at rest in config file — separate concern, not this phase

</deferred>

---

*Phase: 13-security-hardening*
*Context gathered: 2026-04-04 via smart discuss*
