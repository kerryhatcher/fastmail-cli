# Phase 3: CardDAV Write Operations - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement PUT and DELETE HTTP methods on CardDavClient for creating, updating, and deleting contacts on Fastmail's CardDAV server. Each write operation uses correct conditional headers (If-None-Match, If-Match) to prevent data loss from concurrent edits. This phase delivers the protocol-level write operations that Phase 4 (CLI & MCP Surfaces) will wrap.

</domain>

<decisions>
## Implementation Decisions

### Address Book Targeting
- **D-01:** `create_contact` accepts an `addressbook_href: &str` parameter (the address book URL path). The caller provides this; the method does not auto-discover. This is consistent with the existing `list_contacts(&self, addressbook_href: &str)` pattern. Phase 4 CLI will default to the first discovered address book.
- **D-02:** The new contact URL is constructed as `{addressbook_href}{uuid}.vcf` where uuid is the contact's UID (already a UUID v4 per Phase 2).

### Contact Resolution for Writes
- **D-03:** `update_contact` and `delete_contact` accept `href: &str` and `etag: &str` as parameters. The caller is responsible for having a current Contact with its href/etag (from a prior list or search). Write methods do not perform implicit lookups.
- **D-04:** This follows the existing data model where Contact carries href/etag (Phase 1 D-01/D-02). The caller passes these values through, keeping write methods simple and stateless.

### Write Method Signatures
- **D-05:** `create_contact` returns a struct/tuple with the new href (from server Location header or constructed URL) and the new etag (from server ETag response header). This allows callers to perform follow-up operations without re-fetching.
- **D-06:** `update_contact` returns the new etag (from server ETag response header) so the caller can track the updated version.
- **D-07:** `delete_contact` returns `()` on success. No data to return after deletion.

### Error Mapping
- **D-08:** HTTP status code mapping uses the existing error variants:
  - 201 Created / 204 No Content → success (create/update/delete)
  - 412 Precondition Failed → `ContactConflict { id, sent_etag, server_etag }` (extract server ETag from response if available)
  - 404 Not Found → `ContactNotFound(id)`
  - Other non-success → `Error::Server(format!(...))` with status code and body text
- **D-09:** ETag values are passed verbatim including surrounding double-quotes per RFC 7232, consistent with Phase 1 decision on ETag storage.

### Testing Approach
- **D-10:** Unit tests validate request construction (URL, headers, Content-Type, body) and response/error mapping. No live API calls per project constraint. Test helpers build expected HTTP request parameters and verify the logic paths for success and error cases.
- **D-11:** The `serialize_vcard` output is tested separately (Phase 2). Write operation tests focus on HTTP protocol correctness: correct method, correct URL, correct headers, correct error mapping.

### Claude's Discretion
- Whether to return a named struct or tuple for create_contact's return value
- Content-Type header value (`text/vcard` vs `text/vcard; charset=utf-8`)
- Whether to extract the server's ETag from a 412 response body or ETag header
- Internal helper structure for building authenticated PUT/DELETE requests
- Specific test fixture structure and mock patterns

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### CardDAV Protocol Implementation
- `src/carddav/mod.rs` -- Current CardDavClient with list_addressbooks(), list_contacts(), search_contacts() methods. New write methods go here. Lines 69-268 show the HTTP request patterns (PROPFIND, REPORT) to follow for PUT/DELETE.
- `src/carddav/mod.rs` lines 518-587 -- serialize_vcard() function that produces PUT request bodies
- `src/carddav/mod.rs` lines 14-41 -- Contact struct with href/etag fields (Phase 1)

### Error Handling
- `src/error.rs` -- Error enum with ContactNotFound(String) and ContactConflict { id, sent_etag, server_etag } variants (Phase 1)

### Requirements
- `.planning/REQUIREMENTS.md` -- DAV-01 (create with If-None-Match), DAV-02 (update with If-Match), DAV-03 (delete with If-Match), DAV-04 (address book discovery)

### Prior Phase Context
- `.planning/phases/01-contact-model-foundation/01-CONTEXT.md` -- D-01 through D-05: Contact identification, error detail, field visibility decisions
- `.planning/phases/02-vcard-serialization/02-CONTEXT.md` -- D-05: Full rewrite strategy, D-06: serializer location

No external specs -- CardDAV protocol details (RFC 6352, RFC 7232 for ETags) are well-known; requirements fully captured in decisions above and REQUIREMENTS.md.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `CardDavClient` struct (line 63) -- Already has authenticated HTTP client, username, app_password. New write methods are added as `impl CardDavClient` methods.
- `serialize_vcard(&Contact) -> String` (line 518) -- Produces valid vCard 3.0 for PUT request bodies. Tested and proven via round-trip tests.
- `uuid::Uuid` re-export (line 10) -- Available for generating new contact URLs. `Uuid::new_v4().to_string()` produces the path component.
- `list_addressbooks()` (line 80) -- Returns `Vec<AddressBook>` with href for each address book. Callers use this to get the target href for create operations.
- Existing error variants -- `ContactNotFound`, `ContactConflict` already defined (Phase 1).

### Established Patterns
- HTTP requests: Build with `self.client.request(Method, url).basic_auth().header().body().send().await?` pattern
- Custom HTTP methods: `reqwest::Method::from_bytes(b"PROPFIND")` for non-standard methods. PUT and DELETE are standard reqwest methods (`reqwest::Method::PUT`, `reqwest::Method::DELETE`).
- Response handling: Check `status.is_success()` or specific status codes, extract text body for error messages
- Tracing: `#[instrument(skip(self))]` on public async methods, `debug!()` for response status

### Integration Points
- New methods (`create_contact`, `update_contact`, `delete_contact`) are added to `impl CardDavClient` in `src/carddav/mod.rs`
- Phase 4 will call these methods from CLI command handlers (`src/commands/contacts.rs`) and GraphQL mutation resolvers (`src/mcp/graphql/`)
- Return types must carry enough info (href, etag) for Phase 4 to produce useful JSON output

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

*Phase: 03-carddav-write-operations*
*Context gathered: 2026-03-27*
