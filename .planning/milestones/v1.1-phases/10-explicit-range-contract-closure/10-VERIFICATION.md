---
phase: 10-explicit-range-contract-closure
verified: 2026-04-03T20:43:20Z
status: passed
score: 3/3 requirements verified
re_verification: true
---

# Phase 10 Verification

## Goal Achievement

- `cargo test` passed after the shared range helper and user-facing description updates landed.
- One-bound explicit range requests now fail with `Config error: Explicit event ranges require both start and end.` before any network-dependent listing work begins.
- Live Fastmail verification on 2026-04-03 confirmed the same contract through both CLI and MCP GraphQL.

## Local Verification

- `cargo test resolve_event_list_range -- --nocapture`
- `cargo test`
- `cargo run -- events list --help`

## Live Validation

Calendar ID source:

- `cargo run -- calendars list` returned default calendar `26a34e61-285d-492a-9944-a98623919fd1` (`Events`)

CLI contract checks against that calendar:

- `cargo run -- events list --calendar 26a34e61-285d-492a-9944-a98623919fd1 --start 2026-04-05` returned `Config error: Explicit event ranges require both start and end.`
- `cargo run -- events list --calendar 26a34e61-285d-492a-9944-a98623919fd1 --end 2026-04-05` returned the same error
- `cargo run -- events list --calendar 26a34e61-285d-492a-9944-a98623919fd1` succeeded with `data: []`
- `cargo run -- events list --calendar 26a34e61-285d-492a-9944-a98623919fd1 --week` succeeded with `data: []`
- `cargo run -- events list --calendar 26a34e61-285d-492a-9944-a98623919fd1 --start 2026-04-05 --end 2026-04-06` succeeded with `data: []`

MCP GraphQL contract checks against the same calendar via the local `fastmail-cli mcp` server:

- `events(calendarId: "26a34e61-285d-492a-9944-a98623919fd1", start: "2026-04-05")` returned a GraphQL error with `Config error: Explicit event ranges require both start and end.`
- `events(calendarId: "26a34e61-285d-492a-9944-a98623919fd1", end: "2026-04-05")` returned the same GraphQL error
- `events(calendarId: "26a34e61-285d-492a-9944-a98623919fd1")` returned `events: []`
- `events(calendarId: "26a34e61-285d-492a-9944-a98623919fd1", week: true)` returned `events: []`
- `events(calendarId: "26a34e61-285d-492a-9944-a98623919fd1", start: "2026-04-05", end: "2026-04-06")` returned `events: []`

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| EVT-03 | 10-01-PLAN.md | Explicit event listing honors the documented start/end contract | SATISFIED | The shared helper now rejects one-bound requests and still accepts explicit start/end ranges; live CLI and MCP checks exercised both paths |
| CLI-02 | 10-01-PLAN.md | CLI event listing keeps shortcut modes while enforcing explicit-range validation | SATISFIED | CLI help documents the required pair, and live CLI checks verified start-only/end-only failure plus default/week/explicit success |
| MCP-02 | 10-01-PLAN.md | MCP GraphQL event listing exposes the same explicit-range contract as CLI | SATISFIED | The GraphQL `events` resolver still delegates to `list_events_record`, and live `graphql` tool calls returned the same one-bound error and explicit-range success behavior |

## Result

Phase 10 is verified. The explicit-range gap identified in the v1.1 audit is closed across shared helpers, CLI, and MCP.
