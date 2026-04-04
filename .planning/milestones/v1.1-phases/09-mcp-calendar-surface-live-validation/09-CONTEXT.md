# Phase 9: MCP Calendar Surface & Live Validation - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

## Scope

Expose calendar and event operations through MCP GraphQL, then validate the milestone end-to-end against a live Fastmail account.

## Locked Decisions

- Use the existing GraphQL-first MCP design instead of adding separate MCP tools per calendar action.
- Destructive calendar/event mutations must retain preview/confirm confirmation semantics.
- Live validation is required to close the milestone; local unit tests are not sufficient on their own.
