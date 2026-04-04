# Phase 8: CLI Calendar Experience - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

## Scope

Expose the new calendar and event flows through dedicated CLI subcommands, with default today/week/range listing behavior and JSON output that preserves follow-up identifiers.

## Locked Decisions

- Use dedicated `calendars` and `events` subcommands rather than folding calendar operations into unrelated existing commands.
- Default event listing should return future events for the rest of today when no explicit range is supplied.
