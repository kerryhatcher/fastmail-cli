---
phase: 04-cli-mcp-surfaces
plan: 01
subsystem: contacts
tags: [cli, mcp, graphql, contacts]
one-liner: "CLI and MCP now expose contact create, update, and delete flows with shared partial-update logic and explicit delete confirmation."
key_files:
  created: []
  modified:
    - src/commands/contacts.rs
    - src/main.rs
    - src/mcp/graphql/types.rs
    - src/mcp/graphql/mutation.rs
metrics:
  completed: "2026-04-03"
  tasks_completed: 3
  files_modified: 4
---

# Phase 4 Plan 1 Summary

Added the user-facing contact write surfaces for both the CLI and the MCP GraphQL schema.

## What Changed

- `src/commands/contacts.rs`
  - added shared `ContactInput` and `ContactPatch` models
  - added reusable create/update/delete record helpers
  - added CLI wrappers that print JSON success messages
- `src/main.rs`
  - added `contacts create`, `contacts update`, and `contacts delete`
  - enforced `--confirm` / `--yes` for CLI delete
- `src/mcp/graphql/types.rs`
  - added result types for contact mutation responses
  - added `ContactDeleteAction`
- `src/mcp/graphql/mutation.rs`
  - added `createContact`, `updateContact`, and `deleteContact`
  - delete uses a PREVIEW/CONFIRM confirmation token

## Verification-Relevant Outcomes

- CLI create and update now wrap the phase-3 CardDAV write methods
- CLI update merges patches onto the existing contact so omitted fields survive
- CLI delete refuses to run without explicit confirmation flags
- GraphQL create/update return `GqlContact`
- GraphQL delete requires a valid preview token before destructive execution
