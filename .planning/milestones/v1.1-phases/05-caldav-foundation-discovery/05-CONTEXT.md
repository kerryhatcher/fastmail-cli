# Phase 5: CalDAV Foundation & Discovery - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Establish the CalDAV foundation for the calendar milestone: authenticate against Fastmail's CalDAV endpoint, discover the user's calendar home and available calendar collections, and return stable calendar metadata that later CRUD, CLI, and MCP phases can build on. This phase is about discovery and identifiers, not event listing or calendar/event mutation behavior.

</domain>

<decisions>
## Implementation Decisions

### Authentication and Configuration
- **D-01:** Reuse the existing Fastmail username + app-password configuration path already used for contacts. Phase 5 should not introduce a separate `[calendars]` config section.
- **D-02:** Calendar auth should therefore follow the existing `FASTMAIL_USERNAME` and `FASTMAIL_APP_PASSWORD` env/config contract unless a later phase has a strong reason to expand it.

### Calendar Identification
- **D-03:** Calendars are exposed with both a canonical machine identifier and a human-readable display name.
- **D-04:** The canonical identifier is the calendar ID, not the display name. Display names remain visible for humans, but downstream surfaces should treat ID as the stable reference for follow-up operations.
- **D-05:** Phase 5 discovery output should preserve enough server metadata to resolve from the canonical ID to the underlying CalDAV resource safely in later phases.

### Calendar Listing Payload
- **D-06:** Default calendar-list output should include `id`, `name`, and `color`.
- **D-07:** Additional calendar metadata may be exposed later, but Phase 5 should treat it as optional/extendable rather than part of the required default payload.

### Default Calendar Behavior
- **D-08:** When later phases need a target calendar and the user has not explicitly specified one, the product should default to the discovered default/primary calendar rather than forcing explicit selection every time.
- **D-09:** Explicit calendar selection should override defaults whenever the user mentions or passes a calendar directly.

### the agent's Discretion
- Exact CalDAV discovery flow for finding the calendar home set and default calendar collection
- Internal struct layout for storing hrefs, ETags, optional metadata, and any server-specific properties needed later
- Whether color is sourced from a standard CalDAV property, Apple-style extension, or other Fastmail-supported property when available
- Error wording and fallback behavior when no calendars are found or optional metadata is missing

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone Planning
- `.planning/PROJECT.md` — milestone goals, protocol boundary, and product-level constraints for calendar support
- `.planning/REQUIREMENTS.md` — `CAL-01` plus adjacent milestone requirements that Phase 5 must enable without overreaching
- `.planning/ROADMAP.md` — Phase 5 scope anchor, dependencies, and success criteria
- `.planning/research/SUMMARY.md` — research-backed recommendation to implement calendars via CalDAV
- `.planning/research/STACK.md` — stack and protocol additions relevant to the CalDAV foundation
- `.planning/research/ARCHITECTURE.md` — proposed integration shape and build order for calendar support
- `.planning/research/PITFALLS.md` — Phase 5-specific risks such as assuming JMAP calendar access or losing href/etag metadata

### Existing Contact/Protocol Patterns
- `.planning/phases/01-contact-model-foundation/01-CONTEXT.md` — prior decisions on canonical user-facing IDs vs server hrefs and visibility of href/etag metadata
- `.planning/phases/03-carddav-write-operations/03-CONTEXT.md` — protocol-layer conventions for keeping client methods stateless and passing discovered resource identifiers through later surfaces
- `src/config.rs` — existing username/app-password config contract that Phase 5 should reuse
- `src/carddav/mod.rs` — existing Fastmail WebDAV discovery and metadata parsing patterns to mirror where appropriate

### Codebase Guidance
- `.planning/codebase/CONVENTIONS.md` — module, error-handling, and testing patterns for new protocol code
- `.planning/codebase/STRUCTURE.md` — where new CalDAV code should live and how it should connect to commands/MCP later
- `.planning/codebase/STACK.md` — current reqwest/tokio/roxmltree stack constraints
- `.planning/codebase/INTEGRATIONS.md` — current Fastmail integration boundaries and auth model

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/config.rs`: already provides the exact username + app-password retrieval path that Phase 5 should reuse for CalDAV.
- `src/carddav/mod.rs`: already demonstrates Fastmail-specific WebDAV discovery, XML parsing with `roxmltree`, and stateless protocol-client structure.
- `src/error.rs`: existing domain-specific error enum can absorb calendar-discovery errors in the same style as contacts/mail.

### Established Patterns
- Protocol clients live in sibling modules (`src/jmap/`, `src/carddav/`) and encapsulate transport logic behind focused async methods.
- CLI-facing command handlers stay thin and use shared helpers from protocol modules rather than embedding HTTP logic directly.
- Server metadata such as `href` and `etag` is kept available to callers rather than hidden, which is relevant for calendar discovery outputs.

### Integration Points
- A new `src/caldav/` module is the natural home for Fastmail calendar discovery logic.
- Later phases will likely consume Phase 5 outputs from new command helpers and GraphQL resolvers, so Phase 5 data structures should be shaped for both CLI JSON and MCP reuse.
- `src/main.rs`, `src/commands/`, and `src/mcp/graphql/` do not need user-facing calendar surfaces yet, but their existing patterns constrain how discovery models should be exposed later.

</code_context>

<specifics>
## Specific Ideas

- Reuse the contacts-style auth path rather than multiplying configuration sections.
- Treat calendar ID as canonical while still always surfacing the human-readable name beside it.
- Keep the default discovery payload compact: `id`, `name`, `color` first; extra metadata can be opt-in later.
- Default calendar selection should be implicit unless the user explicitly names another calendar.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 05-caldav-foundation-discovery*
*Context gathered: 2026-04-03*
