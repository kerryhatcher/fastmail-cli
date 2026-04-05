---
phase: 16-integration-test-coverage
plan: 04
subsystem: test-mcp-graphql
tags: [testing, graphql, mcp, wiremock, hmac, integration]
dependency_graph:
  requires: [16-01]
  provides: [mcp-graphql-resolver-tests]
  affects: [tests/mcp_graphql.rs]
tech_stack:
  added: []
  patterns: [build_schema-test-entry-point, wiremock-jmap-backend, schema-execute-inline]
key_files:
  created:
    - tests/mcp_graphql.rs
  modified: []
decisions:
  - "Used mailboxes query field (actual name in query.rs) not listMailboxes (plan template name)"
  - "5 tests: JMAP-backed mailboxes resolver, introspection, token determinism, token key-isolation, auth-absent error path"
  - "Wiremock mounts both GET /jmap/session and POST /jmap/api/ so authenticate() + list_mailboxes() both succeed"
metrics:
  duration: 70s
  completed: "2026-04-05T01:46:03Z"
  tasks: 1
  files: 1
---

# Phase 16 Plan 04: MCP GraphQL Resolver Integration Tests Summary

**One-liner:** GraphQL integration test suite using build_schema + test AppContext (zero HMAC key) and wiremock-backed JmapClient to smoke-test mailboxes resolver, introspection, and confirmation_token determinism.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | MCP GraphQL resolver test with test AppContext | da7d180 | tests/mcp_graphql.rs |

## What Was Built

`tests/mcp_graphql.rs` — 5 async integration tests covering the MCP GraphQL layer:

1. **graphql_mailboxes_query_resolves_via_wiremock** — mounts JMAP session + Mailbox/get mocks, builds schema with authenticated JmapClient in AppContext, executes `{ mailboxes { id name role } }`, asserts Inbox mailbox appears in response. This covers D-08 coverage 6 (JMAP-backed resolver path).

2. **graphql_introspection_works_without_jmap** — pure schema-only smoke test; no HTTP. Executes `__schema { queryType { name } }` and asserts `queryType` appears.

3. **confirmation_token_is_deterministic_with_same_key** — verifies HMAC-SHA256 token is stable: two AppContext instances with identical `[0u8; 32]` keys produce the same token for the same inputs.

4. **confirmation_token_differs_across_keys** — verifies key isolation: `[0u8; 32]` vs `[1u8; 32]` produce different tokens for same inputs.

5. **graphql_mailboxes_without_jmap_returns_auth_error** — verifies the resolver returns a meaningful "not authenticated" error when AppContext has no JmapClient.

## Verification

- `cargo test --test mcp_graphql` — 5 passed, 0 failed
- All tests run offline via wiremock (no live Fastmail network calls)
- `grep -rn "wiremock" src/` — empty (wiremock not in production code)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected query field name from plan template**
- **Found during:** Task 1 — plan template used `listMailboxes` but the actual QueryRoot field defined in query.rs is `mailboxes`
- **Fix:** Used `{ mailboxes { id name role } }` in the GraphQL query string
- **Files modified:** tests/mcp_graphql.rs
- **Commit:** da7d180

## Known Stubs

None.

## Self-Check: PASSED

- tests/mcp_graphql.rs exists: FOUND
- Commit da7d180: FOUND
- cargo test --test mcp_graphql: 5 passed, 0 failed
