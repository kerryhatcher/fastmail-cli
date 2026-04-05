# Phase 12: Foundation Safety - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-04
**Phase:** 12-foundation-safety
**Areas discussed:** 4xx error variant strategy, SecretString propagation depth, Output::error pattern for confirmation guards, STAB-10 config corruption guidance format

---

## Area selection

User selected all 4 offered gray areas. HTTP_TIMEOUT constant placement was offered but cut to fit the 4-option AskUserQuestion limit; left to Claude's discretion (resolved to inline `Duration::from_secs(30)` per-client).

---

## 4xx Error Variant Strategy (STAB-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Reuse Error::Server (Recommended) | Catch-all returns `Error::Server(format!("HTTP {} from API", status))`. Matches success criterion 1 wording verbatim. Zero new variants; minimal diff. | ✓ |
| New Error::Http variant | Add `Error::Http { status: u16, body: String }` for structured per-status matching. Richer callers can match on status; costs a new variant + ~5 callsite updates. | |

**User's choice:** Reuse Error::Server (recommended)
**Notes:** Success criterion wording already assumes this variant. Defer structured HTTP variant to a future milestone if per-status handling becomes valuable.

---

## SecretString Propagation Depth (SEC-06)

| Option | Description | Selected |
|--------|-------------|----------|
| Config-only (Recommended) | SecretString wraps api_token and app_password inside Config. `get_token()`/`get_app_password()` still return `Result<String>` by calling `expose_secret().to_string()`. Narrow blast radius; satisfies success criterion 5 fully. | ✓ |
| Propagate through callers | `get_token()` returns `Result<SecretString>`; callers pass it through until HTTP header construction. Deeper protection, ~8-12 callsite touches, potentially conflicts with Phase 14 MCP refactor. | |

**User's choice:** Config-only (recommended)
**Notes:** Preserves narrow blast radius for v1.2. Deeper propagation deferred — may be revisited in v1.3 alongside newtyped IDs.

---

## Output::error Pattern for Confirmation Guards (STAB-03)

| Option | Description | Selected |
|--------|-------------|----------|
| Inline at each callsite (Recommended) | Each of 5 callsites calls `Output::<()>::error(...).print()` then bails. Explicit, minimal abstraction, ~5 small diffs. | ✓ |
| New Error::ConfirmationRequired variant | Central variant: main() detects it and prints Output::error then exits 1. DRYs the pattern but adds a variant and indirection. | |

**User's choice:** Inline at each callsite (recommended)
**Notes:** 5 callsites does not justify central indirection. Re-evaluate if more confirmation guards accumulate.

---

## STAB-10 Config Corruption Guidance Format

| Option | Description | Selected |
|--------|-------------|----------|
| Path + reset instruction (Recommended) | `Error::Config(format!("Failed to parse config at {path}: {parse_err}. Delete this file or fix the TOML to recover."))` — self-contained, actionable, no new deps. | ✓ |
| Path + expected-schema snippet | Include a minimal valid TOML snippet in the error. More helpful but longer error strings. | |
| Path + link to docs | Point to a CONFIG.md or README section. Requires that doc exists and stays in sync. | |

**User's choice:** Path + reset instruction (recommended)
**Notes:** Self-contained message is the simplest fix; no new doc dependencies.

---

## Claude's Discretion

- HTTP_TIMEOUT constant placement → inline `Duration::from_secs(30)` per-client (2 sites only)
- Confirmation-required message phrasing per callsite
- secrecy version pin format → `^0.10`

## Deferred Ideas

- Shared HTTP_TIMEOUT constant (revisit at 3+ HTTP clients)
- Deeper SecretString propagation through HTTP header construction
- Structured per-status Error::Http variant
- Central Error::ConfirmationRequired variant
