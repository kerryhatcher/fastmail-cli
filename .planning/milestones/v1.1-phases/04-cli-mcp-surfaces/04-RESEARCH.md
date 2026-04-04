---
phase: 04-cli-mcp-surfaces
generated: 2026-04-03
status: complete
---

# Phase 4 Research

## Existing Patterns

- CLI command routing is centralized in `src/main.rs` and uses `Output<T>` JSON responses for success and error reporting.
- GraphQL mutations in `src/mcp/graphql/mutation.rs` already implement preview/confirm token flows for send operations, which is the correct delete safety precedent.
- GraphQL contact queries already expose `GqlContact`, so the new mutation results should reuse that type instead of inventing a parallel contact representation.

## Implementation Direction

- Introduce shared contact input/patch helpers in `src/commands/contacts.rs`.
- Add CLI create/update/delete subcommands that delegate to shared helpers.
- Add GraphQL result types for create/update and preview/confirm delete flows.
- Keep partial-update semantics in one place by merging patches against the currently fetched contact before calling phase-3 `update_contact`.
