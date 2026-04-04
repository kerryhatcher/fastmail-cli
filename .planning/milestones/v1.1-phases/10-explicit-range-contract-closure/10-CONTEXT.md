# Phase 10: Explicit Range Contract Closure - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

## Scope

Close the one-bound explicit-range gap so event listing behaves consistently across the shared helper, CLI, and MCP surfaces.

## Locked Decisions

- Partial explicit ranges are rejected rather than silently falling back to the default-today path.
- The shared event-listing helper remains the single source of truth for range selection and validation.
- CLI and MCP surface text must describe the same narrow contract the shared helper enforces.
