---
phase: 10-explicit-range-contract-closure
generated: 2026-04-03
status: complete
---

# Phase 10 Research

## Phase Requirements

- `EVT-03`: explicit event listing must behave as an explicit range contract, not silently degrade to the default-today path.
- `CLI-02`: CLI `events list` must keep shortcut modes and explicit range mode distinct and predictable.
- `MCP-02`: MCP GraphQL `events` must expose the same range semantics as CLI because both surfaces share the same helper.

## Project Constraints

- Keep the implementation in Rust and follow existing repo patterns: clap derive in `src/main.rs`, shared command helpers in `src/commands/`, and async-graphql resolvers in `src/mcp/graphql/`.
- Reuse existing CalDAV/date parsing helpers instead of inventing a parallel range parser.
- Preserve current JSON/error behavior: CLI errors already surface through `Output::<()>::error(e.to_string())`.

## Existing Patterns

- `src/commands/events.rs:list_events_record` is already the single shared event-listing entry point for both CLI and MCP.
- `src/main.rs` passes `calendar`, `week`, `start`, and `end` straight through to that helper without extra validation.
- `src/mcp/graphql/query.rs` does the same in the `events` resolver, so helper behavior is the contract for both surfaces.
- `src/caldav/mod.rs` already has the only date-range primitives this phase needs:
  - `default_today_range()`
  - `current_week_range()`
  - `parse_range_start()`
  - `parse_range_end()`

## Contract Decision

- Partial explicit ranges should be rejected, not honored.

Why this is the right closure:

- The requirement and roadmap language consistently describe explicit listing as a `start/end` range, not as open-ended filtering.
- The audit only requires that one-bound input be handled explicitly rather than silently falling back to `default_today_range()`.
- Rejecting one-bound input fixes the bug without expanding the public contract.
- Honoring `start`-only or `end`-only would create a new open-ended query mode that is not documented in Phase 8 or Phase 9 artifacts and would need fresh CLI/MCP contract wording.

Recommended range contract:

- No `start`, no `end`, `week = false`: use `default_today_range()`.
- `week = true`: use `current_week_range()`.
- Both `start` and `end` present: parse and use them as the explicit range.
- Exactly one of `start` or `end` present: return a validation/config error that explicitly says both bounds are required for an explicit range.

## Implementation Direction

- Keep the fix in `src/commands/events.rs` so CLI and MCP stay aligned automatically.
- Extract the current branching in `list_events_record` into a small private helper that resolves the effective range or returns an error.
- Preserve the existing parsing split:
  - `parse_range_start(start)` for the lower bound
  - `parse_range_end(end)` for the upper bound
- Do not change `src/caldav/mod.rs` query semantics for this phase; the bug is contract selection, not CalDAV transport.
- Update surface descriptions so they match the enforced rule:
  - CLI help text should make it clear that explicit range mode requires both `--start` and `--end`.
  - GraphQL field descriptions should say the same for `events(start:, end:)`.

## Likely File Touch Points

- `src/commands/events.rs`
  - add the shared validation/normalization helper
  - update `list_events_record` to reject partial explicit ranges
  - add focused unit tests for range selection
- `src/main.rs`
  - tighten the `events list` help text so the CLI contract matches the helper behavior
- `src/mcp/graphql/query.rs`
  - tighten the `events` argument descriptions/doc comment to match the helper behavior

## Verification Guidance

- Add local unit coverage around the shared helper, not the network path.
- Minimum cases to cover:
  - no bounds + `week = false` => default-today path still succeeds
  - `week = true` => week path still succeeds
  - both `start` and `end` => explicit path still succeeds
  - `start` only => returns an error
  - `end` only => returns an error
- Keep the error message stable enough to assert intent, for example: explicit event ranges require both start and end.
- Re-run `cargo test`.

Targeted manual checks after code change:

- CLI:
  - `cargo run -- events list --start 2026-04-05` should fail with a JSON error instead of listing today’s events.
  - `cargo run -- events list --end 2026-04-05` should fail the same way.
  - `cargo run -- events list --start 2026-04-05 --end 2026-04-06 --calendar <id>` should still behave as the explicit-range flow used in Phase 8 verification.
- MCP GraphQL:
  - `events(start: "2026-04-05")` should return a GraphQL error sourced from the shared helper.
  - `events(end: "2026-04-05")` should return the same class of error.
  - `events(start: "2026-04-05", end: "2026-04-06", calendarId: "...")` should still succeed.

## Notes For Planning

- This is a narrow contract-closure phase. The safest plan is to change only shared range selection, help text, and tests.
- Avoid expanding scope into open-ended range support unless the requirement language is intentionally revised first.
