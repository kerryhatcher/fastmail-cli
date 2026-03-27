# Phase 2: vCard Serialization - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Build a vCard 3.0 serializer that converts Contact fields into a valid vCard string with proper line folding, character escaping, and a unique UID. This is a pure function with no network access — all behavior is verifiable through unit tests. The serializer will be consumed by Phase 3 (CardDAV write operations) for PUT request bodies.

</domain>

<decisions>
## Implementation Decisions

### N Property Structure
- **D-01:** The vCard N (structured name) property is derived from the single `name` (FN) field by splitting on whitespace: first token = given name, last token = family name, middle tokens joined as middle name. No prefix/suffix handling. This matches the simple name model already established by `parse_vcard` which only reads FN.
- **D-02:** FN is written verbatim from `Contact.name`. N is the split decomposition. Both are always present per vCard 3.0 requirement.

### ADR Property Handling
- **D-03:** Address is a single freeform string (matching the `--address` CLI flag from CLI-01). Serialized into the ADR property's street-address component with all other structured components (PO box, extended, locality, region, postal code, country) left empty. Format: `ADR:;;{street};;;;;\r\n`
- **D-04:** The Contact struct needs an `address: Option<String>` field added in this phase to support ADR serialization. This field was implied by REQUIREMENTS but not yet present in the struct.

### Update Serialization Strategy
- **D-05:** Full rewrite — when serializing a contact, always generate the complete vCard from the Contact struct fields. Unknown/unmodified properties from the original vCard are not preserved. This keeps the serializer simple and stateless. The Contact struct captures all fields we care about for v1.

### Serializer Location
- **D-06:** `serialize_vcard()` function lives in `src/carddav/mod.rs` alongside `parse_vcard()`. Keeps serialization and deserialization co-located in the same module.

### Claude's Discretion
- Exact line folding implementation (fold at 75 octets per RFC 6350, using CRLF + space continuation)
- Character escaping strategy for semicolons, commas, backslashes in property values
- UUID v4 generation approach (use `uuid` crate or manual implementation)
- Whether to add the `address` field to Contact in this phase or defer to Phase 3
- Test structure and specific test fixtures beyond the success criteria

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### vCard Format
- `src/carddav/mod.rs` -- Current Contact struct (lines 13-36), parse_vcard function (line 316), unfold_vcard function (line 267). The serializer is the inverse of these.

### Data Model
- `src/carddav/mod.rs` -- ContactEmail struct (line 38-42), ContactPhone struct (line 44-48). These have label fields that should map to TYPE parameters in serialized vCard.
- `src/mcp/graphql/types.rs` -- GqlContact mirrors Contact. If Contact struct changes (e.g., adding address field), GqlContact must be updated too.

### Requirements
- `.planning/REQUIREMENTS.md` -- VCARD-01 (properties), VCARD-02 (line folding), VCARD-03 (UUID v4)

### Error Handling
- `src/error.rs` -- Error enum. May need a serialization error variant if vCard generation can fail.

No external specs -- vCard 3.0 format is well-known; requirements fully captured in decisions above and REQUIREMENTS.md.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `unfold_vcard()` in `src/carddav/mod.rs:267` -- Line unfolding logic. The serializer needs the inverse: a `fold_line()` function that wraps at 75 octets.
- `parse_vcard()` in `src/carddav/mod.rs:316` -- The deserializer. Serializer output must round-trip through this function for tested properties.
- `hash_id()` in `src/carddav/mod.rs:404` -- Used to generate contact IDs when UID is missing. New contacts will use UUID v4 instead.

### Established Patterns
- Property parsing: `parse_vcard` splits on `:` and handles `TYPE=` parameters. Serializer must produce output compatible with this parsing.
- CRLF handling: `unfold_vcard` handles `\r\n` and `\n` line endings. Serializer should use `\r\n` per the vCard spec.
- Test fixtures: Tests use inline vCard strings. Serializer tests should follow the same pattern.

### Integration Points
- `serialize_vcard(&Contact) -> String` will be called by Phase 3's `create_contact()` and `update_contact()` methods to produce PUT request bodies.
- Contact struct may need an `address` field added — this will propagate to GqlContact via `From<Contact>` impl.

</code_context>

<specifics>
## Specific Ideas

No specific requirements -- open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None -- discussion stayed within phase scope

</deferred>

---

*Phase: 02-vcard-serialization*
*Context gathered: 2026-03-27*
