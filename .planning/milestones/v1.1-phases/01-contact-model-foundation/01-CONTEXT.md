# Phase 1: Contact Model Foundation - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Extend the Contact struct with server-assigned `href` and `etag` fields populated from CardDAV REPORT responses, and add write-specific error variants (`ContactConflict`, `ContactNotFound`) to the error type. This phase delivers the data model foundation that all write operations in Phases 2-4 depend on.

</domain>

<decisions>
## Implementation Decisions

### Contact Identification
- **D-01:** vCard UID remains the user-facing contact identifier. Users pass UID to update/delete commands; the CLI resolves UID to href internally via a REPORT call before performing writes. This hides CardDAV internals from the user.
- **D-02:** `href` and `etag` are stored on the Contact struct but are not used as user-facing identifiers.

### Error Detail Level
- **D-03:** `ContactConflict` uses a structured variant with three fields: `id` (contact UID), `sent_etag` (the ETag the client sent), and `server_etag` (the server's current ETag, if available). This gives maximum diagnostic info for debugging and supports potential automated retry logic.
- **D-04:** `ContactNotFound` follows the existing single-string pattern: `ContactNotFound(String)` carrying the contact UID, consistent with `EmailNotFound` and `MailboxNotFound`.

### Field Visibility
- **D-05:** `href` and `etag` are always visible in JSON output (no `#[serde(skip)]`). Both fields appear in `contacts list` and `contacts search` responses. Useful for scripting, debugging, and power users.

### Claude's Discretion
- Implementation details of how REPORT response parsing extracts href and etag from the XML multistatus response
- Whether to use `Option<String>` or bare `String` for href/etag fields (consider that contacts without href/etag may exist in edge cases)
- Test structure and specific test cases beyond the success criteria

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Data Model
- `src/carddav/mod.rs` -- Current Contact struct (line 14-30), AddressBook struct, parse_contacts_response, parse_vcard function
- `src/error.rs` -- Current Error enum with domain-specific variants pattern

### Requirements
- `.planning/REQUIREMENTS.md` -- MOD-01 (href + etag fields), MOD-02 (error variants)

No external specs -- requirements fully captured in decisions above and REQUIREMENTS.md.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `parse_contacts_response()` in `src/carddav/mod.rs:193` -- Already requests `getetag` in the REPORT XML body but doesn't extract it from the response. Needs modification to extract both href and etag from each `<response>` element.
- `parse_vcard()` in `src/carddav/mod.rs:298` -- Parses vCard string into Contact. Will need to accept href/etag as additional parameters (they come from the XML envelope, not the vCard itself).
- `Error` enum in `src/error.rs` -- Uses thiserror with `#[from]` for conversion and domain-specific named variants. New variants should follow this pattern.

### Established Patterns
- Error variants: Single-string tuple variants for "not found" cases (`MailboxNotFound(String)`, `EmailNotFound(String)`). ContactNotFound follows this. ContactConflict breaks the pattern with a struct variant (user's choice for richer diagnostics).
- Serde serialization: Contact derives `Serialize, Deserialize` and is directly serialized to JSON output. New fields will automatically appear in output.
- Test placement: `#[cfg(test)] mod tests` blocks at the end of each module file.

### Integration Points
- `parse_contacts_response()` is the only place where server XML is decoded into Contact structs -- this is where href and etag extraction must happen
- `Contact` struct is consumed by CLI commands (`src/commands/contacts.rs`) and GraphQL schema (`src/mcp/graphql/`) -- both will see new fields automatically via serde

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

*Phase: 01-contact-model-foundation*
*Context gathered: 2026-03-27*
