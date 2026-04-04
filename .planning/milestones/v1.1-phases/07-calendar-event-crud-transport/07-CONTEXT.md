# Phase 7: Calendar & Event CRUD Transport - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

## Scope

Use the Phase 5 discovery metadata and Phase 6 event semantics to implement CalDAV collection CRUD and event CRUD with ETag-safe writes.

## Locked Decisions

- Calendar and event mutations must preserve raw href/etag metadata for follow-up operations.
- Event update/delete operations must fail explicitly on missing or stale ETags rather than silently clobbering changes.
