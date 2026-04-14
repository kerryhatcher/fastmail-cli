---
phase: 19-group-membership-management
plan: "03"
subsystem: mcp
tags: [graphql, mcp, carddav, groups, membership, async-graphql]

dependency_graph:
  requires:
    - phase: 19-group-membership-management
      plan: "01"
      provides: [add_group_member, remove_group_member on CardDavClient]
  provides:
    - addGroupMember GraphQL mutation in MCP server
    - removeGroupMember GraphQL mutation in MCP server
  affects: [src/mcp/graphql/mutation.rs]

tech-stack:
  added: []
  patterns:
    - "Membership mutation returns GqlContactGroup directly (not wrapped in GqlGroupMutationResult) — non-destructive ops need no confirmation token"
    - "Mutation resolvers follow get_group query pattern: fetch updated group, resolve members, return GqlContactGroup::with_members"

key-files:
  created: []
  modified:
    - src/mcp/graphql/mutation.rs

key-decisions:
  - "No HMAC confirmation token for membership mutations — add/remove member is non-destructive (members are not deleted, only association changes)"
  - "Return GqlContactGroup directly (not GqlGroupMutationResult) to match get_group query return type and enable composable agent workflows"

patterns-established:
  - "Membership mutation pattern: carddav.add/remove_group_member -> resolve_group_members -> GqlContactGroup::with_members"

requirements-completed: [MCP-03]

duration: 5min
completed: 2026-04-13
---

# Phase 19 Plan 03: GraphQL Membership Mutations Summary

**`addGroupMember` and `removeGroupMember` mutations added to MCP GraphQL schema, returning full GqlContactGroup with resolved member contacts via ETag-guarded CardDav transport**

## Performance

- **Duration:** ~5 min
- **Started:** 2026-04-13T00:00:00Z
- **Completed:** 2026-04-13T00:05:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Added `addGroupMember(groupId, contactId): ContactGroup!` mutation to MCP GraphQL schema
- Added `removeGroupMember(groupId, contactId): ContactGroup!` mutation to MCP GraphQL schema
- Both resolvers fetch the updated group state then resolve all member contacts for full return payload
- Error mapping converts CardDavClient typed errors to descriptive GraphQL field errors

## Task Commits

Each task was committed atomically:

1. **Task 1: Add addGroupMember and removeGroupMember GraphQL mutations** - `d78f874` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/mcp/graphql/mutation.rs` - Two new mutation resolvers added after `delete_group`, before `create_event`

## Decisions Made
- No HMAC confirmation token needed — membership operations are non-destructive (contacts themselves are not deleted, only group associations change). This simplifies the AI agent workflow.
- Returns `GqlContactGroup` directly (not `GqlGroupMutationResult`) to match the `get_group` query return type, enabling composable agent workflows that can treat mutation responses and query responses identically.

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- MCP mutations for group membership are complete
- AI agents can now call `addGroupMember` and `removeGroupMember` and receive the updated `ContactGroup` with resolved member contacts
- Phase 19 Plans 01 and 03 together complete the MCP surface for group membership management

---
*Phase: 19-group-membership-management*
*Completed: 2026-04-13*
