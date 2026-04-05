# Phase 12: Foundation Safety - Context

**Gathered:** 2026-04-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish the safe error and credential baseline for fastmail-cli:

- HTTP 4xx responses from JMAP surface as actionable JSON errors, not serde panics
- `CardDavClient` and `CalDavClient` cannot hang indefinitely (30s timeout)
- Confirmation-guard exits (spam, delete masked-email, delete contact, delete calendar, delete event) emit `Output::error(..)` JSON instead of `eprintln!` + `process::exit(1)`
- `api_token` and `app_password` in Config redact to `[REDACTED]` under `{:?}`
- Attachment downloads cannot escape the user-specified output directory via server-supplied filenames
- `JmapClient::new()` returns `Result` instead of `.expect()`-panicking
- Config parse errors guide the user to recovery

**Out of scope for this phase:** vCard/iCal injection escaping (Phase 13), URL encoding (Phase 13), MCP DAV client pool (Phase 14), concurrent DAV fetching (Phase 15), newtyped IDs (deferred to v1.3).

</domain>

<decisions>
## Implementation Decisions

### Error variants (STAB-01)
- **D-01:** Reuse existing `Error::Server(String)` for the 4xx catch-all. Produces `format!("HTTP {} from API", status.as_u16())` to satisfy success criterion 1 verbatim. Do NOT add `Error::Http { status, body }` — flat enum is preferred.
- **D-02:** Both existing JMAP status-handling sites (`src/jmap/mod.rs:204-209` and `:262-267`) get the same catch-all arm placed after the existing 401/429/5xx arms.

### Secret redaction depth (SEC-06)
- **D-03:** Use `secrecy::SecretString` inside `Config`, `CoreConfig`, and `ContactsConfig` for `api_token` and `app_password` fields. Add `secrecy` with the `serde` feature so TOML deserialization is seamless.
- **D-04:** Do NOT propagate `SecretString` through callers. `get_token()` and `get_app_password()` keep their `Result<String>` signatures by calling `secret.expose_secret().to_string()` at the boundary. This preserves the narrow blast radius; deeper propagation is deferred (not in scope for v1.2).
- **D-05:** Derive `Debug` on Config structs as normal — `SecretString` automatically renders as `[REDACTED]` via its own Debug impl. No custom `impl Debug` needed.

### Confirmation-guard output contract (STAB-03)
- **D-06:** Replace each of the 5 `eprintln!` + `std::process::exit(1)` callsites (`src/main.rs:753, 856, 919, 945, 1095`) with inline `Output::<()>::error("Confirmation required: <message>").print()` followed by `anyhow::bail!("confirmation required")` (or equivalent) so main's error handler exits with status 1.
- **D-07:** Do NOT introduce an `Error::ConfirmationRequired` variant — 5 callsites does not justify centralized indirection. Keep the pattern explicit at each callsite for readability.
- **D-08:** Each callsite's error message must match its command's destructive action (e.g., "Confirmation required: pass --confirm to delete calendar").

### Config corruption guidance (STAB-10)
- **D-09:** `Config::load()` parse failures return `Error::Config(format!("Failed to parse config at {path}: {parse_err}. Delete this file or fix the TOML to recover.", path = path.display(), parse_err = e))`. Self-contained, no new docs.
- **D-10:** Include the resolved absolute config path (from `Self::config_path()?`) in the message so the user can copy-paste `rm` or `$EDITOR` it.

### Other fixes (no user decision needed — prescriptive)
- **D-11:** DAV timeout (STAB-02) — both `CardDavClient::new()` and `CalDavClient::new()` use `reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?`. Place the `Duration` inline (not a shared constant) — only 2 sites, drift risk is low and a shared constant is more ceremony than value at this stage. Revisit if a 3rd client is added.
- **D-12:** Path traversal (SEC-01) — `src/commands/download.rs:112` uses `Path::new(&final_filename).file_name().unwrap_or_else(|| OsStr::new("attachment"))` before joining to `out_dir`, per research recommendation.
- **D-13:** `JmapClient::new()` (STAB-09) — change signature to `pub fn new() -> Result<Self>`. Current callers are expected to propagate with `?`. Error variant: `Error::Config("HTTP client builder failed: ...")` since builder failure is a configuration concern.

### Claude's Discretion
- Exact phrasing of confirmation-required messages per callsite (must be actionable and name the flag needed, e.g., `--confirm` or `--yes`)
- Whether to unit-test the redacted Debug output (recommended: yes, one test per config struct)
- Whether to pin `secrecy` to 0.10.x exactly or allow minor bumps via `^0.10` (recommended: `^0.10`)

### Folded Todos
None — no pending todos matched Phase 12.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Source findings
- `CODEBASE-REVIEW.md` findings #1 (4xx handling), #2 (DAV timeouts), #3 (path traversal), #11 (process::exit breaks JSON contract), #15 (Debug on secrets), #32 (JmapClient expect), #33 (config corruption guidance)

### Research
- `.planning/research/STACK.md` — `secrecy` 0.10.3 API; `SecretString` Debug behavior; serde feature
- `.planning/research/FEATURES.md` — secret redaction pattern discussion (secrecy vs manual Debug)
- `.planning/research/ARCHITECTURE.md` — integration points and new component design
- `.planning/research/PITFALLS.md` — "Timeout placement is subtle" (split connect/total timeout drift), "4xx catch-all + process::exit = double regression risk" (why D-06 must ship with D-01/D-02)
- `.planning/research/SUMMARY.md` — cross-cutting signals

### Project
- `.planning/REQUIREMENTS.md` — STAB-01, STAB-02, STAB-03, STAB-09, STAB-10, SEC-01, SEC-06
- `.planning/ROADMAP.md` §"Phase 12: Foundation Safety" — success criteria

### Code (targets of change)
- `src/error.rs` — Error enum (no new variants, reuse Server)
- `src/jmap/mod.rs` §204-209, §262-267 — status handling (STAB-01)
- `src/jmap/mod.rs` (constructor) — `JmapClient::new()` (STAB-09)
- `src/carddav/mod.rs:81` — `CardDavClient::new()` timeout (STAB-02)
- `src/caldav/mod.rs:101` — `CalDavClient::new()` timeout (STAB-02)
- `src/config.rs` — `Config`, `CoreConfig`, `ContactsConfig`, `load()` (STAB-10, SEC-06)
- `src/main.rs:753, 856, 919, 945, 1095` — 5 confirmation-guard exits (STAB-03)
- `src/commands/download.rs:112` — filename handling (SEC-01)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Error::Server(String)` — already exists in `src/error.rs:77-78`; reuse for 4xx catch-all
- `Error::Config(String)` — already exists; reuse for `JmapClient::new()` builder failures and parse-error guidance
- `Output::<()>::error("..").print()` — already used across command handlers (via `src/models/mod.rs`)
- `JmapClient` already constructs via `reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?` — copy that pattern to `CardDavClient::new()` and `CalDavClient::new()`

### Established Patterns
- Error enum is flat, variant-per-concern, with `thiserror` `#[error("...")]` messages
- Config structs use `#[derive(Debug, Serialize, Deserialize, Default)]` — `SecretString` + serde feature drops in cleanly
- All Config accessors (`get_token`, `get_username`, `get_app_password`) take env-var-first-then-field pattern
- Error propagation uses `?`; command handlers use `anyhow::Result<T>`, internal JMAP uses custom `Result<T>`

### Integration Points
- `JmapClient::new()` change (STAB-09) cascades to every caller that instantiates it — search for `JmapClient::new(` callsites and propagate `?`
- `SecretString` in Config means `set_token()` signature becomes `set_token(&mut self, token: SecretString)` or stays as `String` and wraps internally — pick internally wrap to avoid auth command refactor (auth command passes bare `String` today)

</code_context>

<specifics>
## Specific Ideas

- The success criterion 1 wording `{"error": "Server error: HTTP 400"}` maps directly to `Error::Server(format!("HTTP {} from API", status.as_u16()))` because `thiserror`'s `#[error("Server error: {0}")]` prefixes "Server error: " automatically. Do not re-add the prefix in the `format!`.
- For SEC-06, the `secrecy` `SecretString` Debug output is literally `Secret([REDACTED alloc::string::String])` — close enough to satisfy the success criterion, but a test should assert the substring `REDACTED` rather than an exact match to avoid breakage across `secrecy` minor versions.

</specifics>

<deferred>
## Deferred Ideas

- **Shared HTTP_TIMEOUT constant** — deferred until a 3rd HTTP client exists. With only JMAP + Carddav + Caldav today, duplication is acceptable.
- **Deeper SecretString propagation through HTTP-header construction** — deferred; current Config-only wrapping satisfies v1.2 SEC-06 in full.
- **Structured per-status HTTP error variant** (`Error::Http { status, body }`) — deferred; flat `Error::Server` sufficient for current callers.
- **Central `Error::ConfirmationRequired` variant** — deferred; re-evaluate if more than ~8 confirmation guards accumulate.

### Reviewed Todos (not folded)
None.

</deferred>

---

*Phase: 12-foundation-safety*
*Context gathered: 2026-04-04*
