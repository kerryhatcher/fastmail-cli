# Phase 5: CalDAV Foundation & Discovery - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-04-03
**Phase:** 05-CalDAV Foundation & Discovery
**Areas discussed:** Authentication and configuration, calendar identification, calendar listing payload, default calendar behavior

---

## Authentication and Configuration

| Option | Description | Selected |
|--------|-------------|----------|
| Reuse contacts auth config | Use the existing Fastmail username + app password path for CalDAV too | ✓ |
| Separate calendars config | Add a distinct calendar config section even if credentials often match | |

**User's choice:** Reuse existing config
**Notes:** User chose to reuse the current contact-style auth/config path rather than introducing a new calendars section.

---

## Calendar Identification

| Option | Description | Selected |
|--------|-------------|----------|
| Name as canonical | Use display name as the primary reference for later operations | |
| ID as canonical, surface both | Show both id and name, but treat id as the stable reference | ✓ |
| Href-only | Expose low-level resource href as the main caller-facing identifier | |

**User's choice:** Both with ID as canonical
**Notes:** User wants both machine and human identifiers visible, with the ID treated as canonical for follow-up operations.

---

## Calendar Listing Payload

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal default | Default to `id`, `name`, and `color`; expose other metadata only when explicitly requested later | ✓ |
| Rich default | Include every discoverable metadata field in the default payload | |
| Name-only default | Keep the default payload human-first and very small | |

**User's choice:** Default to id, name, color; others optional
**Notes:** The default discovery contract should stay compact and stable.

---

## Default Calendar Behavior

| Option | Description | Selected |
|--------|-------------|----------|
| Default calendar unless explicit override | Use the default/primary calendar unless the user explicitly mentions another one | ✓ |
| Always explicit selection | Force callers to choose a calendar for safety | |
| Name-based inference | Infer by display name unless ambiguous | |

**User's choice:** Default unless explicitly mentioned
**Notes:** The user wants convenient defaults rather than mandatory selection friction.

---

## the agent's Discretion

- Exact CalDAV discovery sequence and metadata parsing details
- Internal model fields beyond the user-locked output contract
- Handling of optional/nonstandard calendar metadata

## Deferred Ideas

None.
