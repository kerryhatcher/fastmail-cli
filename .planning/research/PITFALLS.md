# Domain Pitfalls: CardDAV Write Operations

**Domain:** CardDAV contact CRUD (create, update, delete) on Fastmail
**Researched:** 2026-03-27
**Confidence:** HIGH (core protocol), MEDIUM (Fastmail-specific behaviors)

---

## Critical Pitfalls

Mistakes in this category cause silent data loss, server rejections, or contact corruption.

---

### Pitfall 1: ETag Blindness on Update

**What goes wrong:** `PUT` for an update is issued without an `If-Match` header containing the current ETag. The server accepts the write because it is technically valid, but if two clients (e.g. the CLI and the Fastmail web interface) both read the same contact, modify it, and write back, the second writer silently overwrites the first. The RFC 6352 / WebDAV spec requires `If-Match` for safe conditional updates.

**Why it happens:** Developers forget that a contact's resource URL and ETag are distinct from its `UID` property. The `list_contacts` REPORT response in this codebase currently extracts `address-data` but discards `getetag` (the body already requests it via `<d:getetag/>` but `parse_contacts_response` ignores it). Without storing ETags alongside contacts, there is nothing to put in `If-Match` at update time.

**Consequences:**
- Lost writes under concurrent modification (last writer wins rather than safe merge/reject)
- Server returns `412 Precondition Failed` if you attempt forced `If-Match: *` on a non-existent resource
- Fastmail's server may return `409 Conflict` when ETag mismatch is detected

**Prevention:**
- Extend `Contact` (or a parallel `ContactResource` struct) to carry `href: String` and `etag: String`
- Parse `d:getetag` from REPORT/PROPFIND responses alongside `address-data`
- Always send `If-Match: "<etag>"` on `PUT` for updates
- Send `If-None-Match: *` on `PUT` for creates (guarantees you are not overwriting an existing resource)

**Warning signs:**
- Code does PUT without building an `If-Match` header
- `Contact` struct has no `href` or `etag` field
- `parse_contacts_response` ignores the `d:getetag` node

**Phase:** Must be addressed in the initial write-operations implementation phase.

---

### Pitfall 2: Using UID as the Resource URL

**What goes wrong:** The contact's `UID` vCard property is mistaken for the CardDAV resource URL. They are often similar but are not the same thing. The resource URL is the server-assigned `href` (e.g. `/dav/addressbooks/user/alice@fastmail.com/Default/abc123.vcf`). The UID is a property inside the vCard body. Using the UID directly to construct a URL for PUT/DELETE will produce 404s when the server uses different naming conventions.

**Why it happens:** It is tempting to synthesize the resource URL from the UID because REPORT responses contain both, but some servers (including Fastmail) may assign URLs that differ from the UID value, especially for contacts created via other clients.

**Consequences:**
- DELETE hits the wrong URL (or 404)
- PUT creates a duplicate rather than updating the existing resource

**Prevention:**
- Store the `href` returned in the REPORT response's `<d:href>` element per contact
- Build PUT/DELETE URLs from the stored `href`, not from the UID
- For creates, generate a stable UUID-based filename (e.g. `<uuid>.vcf`) and PUT to `<addressbook_href><uuid>.vcf`

**Warning signs:**
- Code constructs contact URLs by string-formatting the `id` field
- No `href` field stored alongside parsed contacts

**Phase:** Must be addressed in the initial write-operations implementation phase.

---

### Pitfall 3: vCard Line Folding Omitted on Generation

**What goes wrong:** vCard properties with long values (e.g. long names, notes, addresses) are emitted as single lines that exceed 75 octets. RFC 6350 §3.2 requires lines longer than 75 octets to be folded by inserting CRLF followed by a single space. Fastmail's server accepts unfolded content but some CardDAV clients (mobile apps syncing the same address book) will misparse contacts created by this CLI, producing truncated or corrupted display names.

**Why it happens:** The existing `unfold_vcard` function handles inbound folding correctly on reads, but there is no corresponding fold function for outbound vCard generation. It is easy to forget that both directions must be handled.

**Consequences:**
- Contacts appear correct in CLI (server accepts them) but are corrupted in Fastmail mobile app or third-party CardDAV clients
- Notes fields are especially vulnerable — multi-line notes may have embedded newlines that also require escaping

**Prevention:**
- Implement a `fold_vcard_line(property: &str) -> String` function that inserts CRLF + space every 75 octets
- Escape special characters in values: commas, semicolons, and backslashes must be backslash-escaped; literal newlines in NOTE/ADR become `\n` (backslash-n, not a real newline)
- Use CRLF (`\r\n`) as the line separator in generated vCards per RFC 6350 §3.3

**Warning signs:**
- vCard generation writes plain `\n` line endings
- No folding pass over generated property values
- NOTE values with newlines are not escaped to `\n`

**Phase:** Must be addressed in vCard generation implementation.

---

### Pitfall 4: Missing or Wrong Content-Type on PUT

**What goes wrong:** PUT requests omit the `Content-Type` header or use `application/xml` (used for PROPFIND/REPORT). CardDAV requires `Content-Type: text/vcard; charset=utf-8` for PUT requests. Fastmail's server may return `415 Unsupported Media Type` or silently reject with a non-200 response.

**Why it happens:** The existing REPORT and PROPFIND calls in `carddav/mod.rs` all use `Content-Type: application/xml`. It is easy to copy that header verbatim when building PUT.

**Consequences:**
- Server rejects the request or stores the vCard with the wrong content type, causing sync failures

**Prevention:**
- Use `Content-Type: text/vcard; charset=utf-8` for all PUT requests
- Use `Content-Type: application/xml; charset=utf-8` only for PROPFIND/REPORT/MKCOL

**Warning signs:**
- PUT request builder copies `application/xml` header from PROPFIND code path

**Phase:** Implementation phase, easily caught by a live integration test.

---

### Pitfall 5: Partial Update via Read-Modify-Write Race

**What goes wrong:** The project requires partial updates (only modify fields explicitly passed). The only safe way to do partial updates in CardDAV is the full read-modify-write cycle: fetch the current vCard, merge the changed fields, PUT the full vCard back. If the fetch happens and then the user's session modifies the same contact (in the web UI) before the PUT lands, the intermediate change is silently lost.

**Why it happens:** CardDAV has no PATCH operation. `If-Match` with the ETag from the initial fetch is the sole concurrency control. If `If-Match` is not sent, the race window is invisible. If it is sent but the ETag changed, the server returns `412` and the implementation must surface a meaningful error.

**Consequences:**
- Without `If-Match`: silent data loss when contact is edited concurrently
- With `If-Match` but no retry logic: user gets a raw `412` error with no guidance

**Prevention:**
- Always send `If-Match` with the ETag obtained at fetch time
- On `412 Precondition Failed`, return a clear error: "Contact was modified by another client; re-run the update to apply changes to the current version"
- Do not auto-retry silently — the user must re-inspect and re-issue the command

**Warning signs:**
- Update path does not fetch the current contact before PUT
- No `If-Match` header set
- `412` response is treated as a generic server error without specific messaging

**Phase:** Update implementation phase.

---

## Moderate Pitfalls

---

### Pitfall 6: Fastmail Rejects Invalid vCard VERSION

**What goes wrong:** Fastmail's CardDAV server enforces vCard `VERSION:3.0` for existing contacts. Emitting `VERSION:4.0` for new contacts may work, but updating a contact that Fastmail stores as 3.0 by emitting a 4.0 vCard can cause a `400 Bad Request` or the contact being re-stored with mixed-version properties that other clients cannot parse.

**Why it happens:** RFC 6350 defines vCard 4.0; RFC 2426 defines 3.0. Fastmail defaults to 3.0 in its address book. The existing parser accepts both (it does not validate VERSION), so the issue only surfaces during writes.

**Consequences:**
- Intermittent 400 errors on update if VERSION is changed
- Contact round-trip breakage (read 3.0, write 4.0)

**Prevention:**
- When updating an existing contact, preserve the VERSION found in the fetched vCard
- For creates, emit `VERSION:3.0` to match Fastmail's default
- Include VERSION as the first property after BEGIN:VCARD per spec

**Warning signs:**
- vCard generator always emits `VERSION:4.0` regardless of source vCard
- VERSION is not preserved during read-modify-write

**Phase:** vCard generation implementation.

---

### Pitfall 7: Address Book Discovery Needed Before Write

**What goes wrong:** Creating a contact requires knowing which address book `href` to PUT into. The current `list_addressbooks()` is called every time because there is no caching. If the create path hardcodes a URL or skips discovery, it will fail for users whose address book is not at the default Fastmail path.

**Why it happens:** Fastmail's default address book is typically at `/dav/addressbooks/user/<username>/Default/`, which can tempt hardcoding. But some accounts have differently named address books, or the default can change.

**Consequences:**
- 404 on PUT for users with non-default address book names
- Creating contacts in the wrong address book if multiple exist

**Prevention:**
- Always call `list_addressbooks()` and pick the first (or default-named) address book at create time
- Consider a `--addressbook` flag for power users with multiple books
- Cache the address book list within a single CLI invocation (not across invocations)

**Warning signs:**
- Hardcoded `/Default/` path segment in create code
- No address book discovery step in the create flow

**Phase:** Create implementation phase.

---

### Pitfall 8: Missing `N` Property in Generated vCard

**What goes wrong:** The existing `Contact` struct only exposes `name` as a single `FN` (formatted name) string. vCard 3.0 specifies that `N` (structured name: family;given;additional;prefix;suffix) is required. Fastmail's server may accept a vCard without `N` but other synced clients (iOS Contacts, Outlook) will display the contact incorrectly or refuse to import it.

**Why it happens:** `FN` is the display name and is the only field stored by the current parser. `N` requires splitting the display name into components which is non-trivial and ambiguous for non-Western names.

**Consequences:**
- Contact syncs to mobile app but shows blank or garbled name
- RFC 2426 (vCard 3.0) §3.1.2 says N is required; some validators/clients reject vCards without it

**Prevention:**
- Emit `N:;;;;<FN_value>;` (empty structured fields, full name in the "additional" position) as a minimum valid `N` property
- Better: accept `--first-name` / `--last-name` CLI flags and use them to populate `N` properly
- Always emit both `FN` and `N`

**Warning signs:**
- Generated vCard body does not include an `N:` line
- Code only writes `FN:` from the display name

**Phase:** vCard generation implementation.

---

### Pitfall 9: Confirmation Token Must Cover Contact Identity

**What goes wrong:** The existing email mutation pattern uses a `confirmation_token` derived from message content to prevent accidental re-execution. The delete contact mutation needs the same guard, but the token must be derived from fields that uniquely identify the specific contact (its `href` or `uid`), not just a generic operation name. Using a weak or absent token allows an AI agent to accidentally delete the wrong contact.

**Why it happens:** It is easy to copy the `send_email` mutation pattern but forget to include the contact identifier in the token derivation inputs.

**Consequences:**
- Delete can be executed without confirmation in an MCP/AI workflow
- Token reuse: if two different contacts produce the same token (e.g. both derive from just "delete"), one confirmation re-executes the other

**Prevention:**
- Include the contact's `href` (or `uid`) in the `confirmation_token()` input slice for delete and update mutations
- Follow the exact same PREVIEW/CONFIRM pattern used in `send_email` and `mark_as_spam`

**Warning signs:**
- Token derivation inputs do not include the contact identifier
- Delete mutation has no PREVIEW step

**Phase:** MCP GraphQL mutation implementation phase.

---

### Pitfall 10: HTTP Status 204 vs 201 Not Handled Distinctly

**What goes wrong:** CardDAV PUT for a new resource returns `201 Created`. PUT for an update returns `204 No Content`. Treating both as a generic "success" is fine for the happy path, but if a create is retried after a network failure, the second attempt hits an existing resource and returns `412` (if `If-None-Match: *` was sent). If the code only checks `is_success()`, a `201` from an unexpected create will be silently accepted as an update and vice versa.

**Why it happens:** The existing PROPFIND/REPORT code pattern checks `!status.is_success() && status.as_u16() != 207` which is correct for multi-status responses. PUT does not return 207, so the check pattern must be adapted.

**Consequences:**
- Ambiguous success state makes it hard to give the user accurate feedback ("contact created" vs "contact updated")
- Re-creation on retry if `If-None-Match: *` is absent

**Prevention:**
- Check for `201 Created` on create, `204 No Content` on update — return distinct success messages
- Check for `204 No Content` on delete
- Treat `404` on delete as a distinct "contact not found" error, not a generic failure

**Warning signs:**
- `response.status().is_success()` used without distinguishing 201/204
- No `ContactNotFound` error variant in `error.rs`

**Phase:** Write-operations implementation phase.

---

## Minor Pitfalls

---

### Pitfall 11: vCard UID Must Be a Stable, Globally Unique Identifier

**What goes wrong:** The current parser generates a fallback UID via `hash_id(&name)` (SipHash of the display name). If a new contact is created with this mechanism as its UID, the UID changes every time the display name changes. CardDAV servers use UID for deduplication and sync conflict detection across devices.

**Why it happens:** The hash fallback was designed for read-only display purposes where a stable internal ID was needed for the Rust model. It was never intended as a durable UID for server storage.

**Prevention:**
- For new contacts, generate a proper UUID v4 (use the `uuid` crate with the `v4` feature)
- Set the `UID` property in the generated vCard to the UUID string
- The resource filename should also be `<uuid>.vcf` for consistency

**Warning signs:**
- `hash_id()` is used to set UID in a newly generated vCard
- No `uuid` crate dependency in `Cargo.toml`

**Phase:** vCard generation implementation.

---

### Pitfall 12: Special Characters in vCard Values Not Escaped

**What goes wrong:** vCard 3.0 §5 requires that commas (`,`), semicolons (`;`), and backslashes (`\`) in property values be backslash-escaped. In `NOTE` and `ADR` properties, literal newlines must be represented as the two-character sequence `\n`. The existing parser handles this implicitly (it reads raw values), but the generator must produce escaped output.

**Consequences:**
- A contact note containing a semicolon causes the vCard to be parsed as multiple subfields
- A contact name with a backslash produces a malformed vCard line

**Prevention:**
- Write a `escape_vcard_value(s: &str) -> String` helper that escapes `\`, `;`, `,`, and replaces `\n` with `\\n`
- Apply it to every text value written into the vCard body
- Test with names containing commas (e.g. "Smith, John") and notes with newlines

**Warning signs:**
- No escaping helper in vCard generation code
- Tests for generated vCards do not include special characters

**Phase:** vCard generation implementation.

---

### Pitfall 13: Fastmail Requires App Password, Not API Token, for CardDAV

**What goes wrong:** This is already known (see `CONCERNS.md`) but is worth flagging specifically for write operations: the app password must have "Contacts" scope enabled. An app password with only "Mail" scope will fail with `403 Forbidden` on write operations even though it may successfully read contacts. The error message from Fastmail is not always clear about the missing scope.

**Prevention:**
- Document in CLI help text that the app password must have "Contacts (Read/Write)" scope
- Surface the specific 403 error as a distinct "Insufficient permissions — ensure your app password has Contacts write access" message rather than a generic server error

**Warning signs:**
- Generic `Error::Server` wrapping of 403 responses with no guidance to the user

**Phase:** Error handling, documentation.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|----------------|------------|
| Contact struct extension | No `href`/`etag` fields → cannot do safe update/delete | Add fields to `Contact` or introduce `ContactResource` wrapper before writing any PUT code |
| vCard generation | Missing `N`, VERSION mismatch, no folding, no escaping | Write and unit-test vCard builder function in isolation before wiring to HTTP |
| PUT create | Wrong URL construction from UID, missing `If-None-Match: *` | Use address book discovery + UUID filename; always set `If-None-Match: *` |
| PUT update | Missing `If-Match`, no ETag storage | Parse ETag from REPORT response; send `If-Match` on every update PUT |
| DELETE | Missing contact-not-found distinction, no `If-Match` | Parse 404 vs 204; optionally send `If-Match` on delete for safety |
| MCP mutations | Weak confirmation token, no PREVIEW step for delete | Token must include contact `href`/`uid`; follow existing send_email PREVIEW/CONFIRM pattern |
| Error handling | 412 surfaced as generic error | Add specific `ContactConflict` or descriptive `Server` message; guide user to retry |

---

## Sources

- RFC 6352 (CardDAV): HTTP/WebDAV protocol for vCard address books — ETag, If-Match, If-None-Match, 207 Multi-Status requirements (HIGH confidence — protocol spec)
- RFC 6350 (vCard 4.0): Line folding at 75 octets, CRLF line endings, property escaping, N vs FN requirements (HIGH confidence — format spec)
- RFC 2426 (vCard 3.0): N property REQUIRED in version 3.0, VERSION property rules (HIGH confidence — format spec)
- Codebase analysis: `src/carddav/mod.rs`, `src/mcp/graphql/mutation.rs`, `src/error.rs` — existing patterns and gaps (HIGH confidence — direct code inspection)
- CONCERNS.md: CardDAV app password scope, XML parsing fragility, vCard parsing edge cases (HIGH confidence — existing project analysis)
- Fastmail-specific behaviors (app password scopes, address book path conventions, VERSION:3.0 default): MEDIUM confidence — based on known Fastmail CardDAV deployment characteristics; should be validated with a live integration test against a Fastmail account
