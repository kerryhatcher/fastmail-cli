# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.1 — Calendar Access and Management

**Shipped:** 2026-04-04
**Phases:** 7 | **Plans:** 7

### What Was Built
- CalDAV foundation reusing existing CardDAV auth/discovery patterns for calendar-home discovery
- iCalendar parsing/serialization for VEVENT, ATTENDEE, RRULE, VALARM with round-trip fidelity
- Full calendar and event CRUD transport with ETag-safe concurrency
- CLI calendar commands with smart defaults (today, week, explicit range)
- MCP GraphQL calendar/event operations for AI agent workflows
- Gap-closure phases for explicit range contracts and attendee clearing parity

### What Worked
- Mirroring the v1.0 contact CRUD milestone structure (foundation → semantics → transport → CLI → MCP) kept each phase focused
- Live Fastmail validation caught real server behavior (412 without replacement ETag) that unit tests couldn't
- Milestone audit after Phase 9 caught two genuine gaps (range contracts, attendee clearing) that became Phases 10-11

### What Was Inefficient
- All 7 phases executed in a single large commit — finer git history would aid future bisection
- Phase summaries for 5-11 had inconsistent detail levels; some lacked one-liner extraction fields

### Patterns Established
- Milestone audit as a gap-finding tool, not just a checkbox — proved its value by generating two closure phases
- Explicit range validation pattern: require both bounds rather than silently defaulting
- `--clear-*` flag pattern for CLI commands that need to distinguish "omit" from "set empty"

### Key Lessons
1. Live validation against the actual server is non-negotiable for protocol work — Fastmail's CalDAV behavior differs from spec in subtle ways
2. Post-feature audits that generate additional phases are valuable, not scope creep — they prevent shipping with known integration gaps

### Cost Observations
- Model mix: primarily opus for planning and execution
- Notable: The audit → gap-closure loop (Phases 10-11) added ~10% more work but significantly improved contract consistency

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v1.0 | 4 | 4 | Established foundation → CRUD → surfaces pattern |
| v1.1 | 7 | 7 | Added milestone audit → gap-closure loop |

### Cumulative Quality

| Milestone | Tests | Clippy | Unsafe |
|-----------|-------|--------|--------|
| v1.0 | ~80 | Clean | None |
| v1.1 | 103 | Clean | None |

### Top Lessons (Verified Across Milestones)

1. Protocol-level work (CardDAV, CalDAV) benefits from live server validation — unit tests alone miss server-specific behavior
2. Layered build order (foundation → semantics → transport → surfaces) scales well across different protocol domains
