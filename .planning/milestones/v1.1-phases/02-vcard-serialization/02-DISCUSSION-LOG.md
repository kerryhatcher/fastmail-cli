# Phase 2: vCard Serialization - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-27
**Phase:** 02-vcard-serialization
**Areas discussed:** N property structure, ADR property handling, Update serialization strategy, Serializer location
**Mode:** Auto (all decisions selected by recommended defaults)

---

## N Property Structure

| Option | Description | Selected |
|--------|-------------|----------|
| Simple split on space | First token = given, last token = family, no middle/prefix/suffix | ✓ |
| Require separate first/last name fields | Add first_name, last_name to Contact struct | |
| Configurable name parsing | Allow user to specify name components | |

**User's choice:** [auto] Simple split on space (recommended default)
**Notes:** Matches existing parse_vcard behavior which only reads FN. Most CLI contact creation uses simple names. Complex name handling deferred to v2.

---

## ADR Property Handling

| Option | Description | Selected |
|--------|-------------|----------|
| Single freeform string | Store as street component, other ADR components empty | ✓ |
| Structured address fields | Separate street, city, state, zip, country fields | |
| No address support in v1 | Defer ADR entirely | |

**User's choice:** [auto] Single freeform string (recommended default)
**Notes:** Matches the --address CLI flag from CLI-01 which takes a single string argument. Structured addresses add complexity with minimal benefit for v1.

---

## Update Serialization Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Full rewrite from Contact fields | Always regenerate complete vCard, don't preserve unknowns | ✓ |
| Preserve-and-patch | Parse original, modify known fields, preserve unknown properties | |
| Hybrid | Preserve if original available, full rewrite otherwise | |

**User's choice:** [auto] Full rewrite from Contact fields (recommended default)
**Notes:** Simpler, stateless serializer. Contact struct captures all v1 fields. Unknown property preservation adds parsing complexity.

---

## Serializer Location

| Option | Description | Selected |
|--------|-------------|----------|
| In src/carddav/mod.rs | Co-locate with parse_vcard | ✓ |
| New src/carddav/serialize.rs | Separate module for serialization | |
| In src/util.rs | With other utility functions | |

**User's choice:** [auto] In src/carddav/mod.rs (recommended default)
**Notes:** Follows existing module organization — parse and serialize together in the CardDAV module.

---

## Claude's Discretion

- Line folding implementation details
- Character escaping strategy
- UUID v4 generation approach
- Whether to add address field to Contact in this phase
- Test fixture structure

## Deferred Ideas

None — discussion stayed within phase scope
