# Feature Landscape: CardDAV Contact CRUD

**Domain:** CLI contact management via CardDAV (RFC 6352 / RFC 6350)
**Researched:** 2026-03-27
**Confidence:** HIGH — codebase analysis + protocol knowledge (RFC 6352/6350 are stable standards)

---

## Context

This feature set extends `fastmail-cli`'s existing read-only CardDAV integration with write operations.
The existing `Contact` struct (id/name/emails/phones/organization/title/notes) and `CardDavClient`
(PROPFIND discovery + REPORT listing) define the baseline. All features below reference what needs
to be added.

---

## Table Stakes

Features users expect. Missing = the contact CRUD milestone is incomplete.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Create contact via CLI | Core milestone requirement | Medium | Needs vCard generation + PUT to `{ab_href}/{uid}.vcf` |
| Update contact via CLI (partial) | Core milestone requirement | High | Must read existing vCard, merge changed fields, PUT full vCard back — CardDAV has no PATCH |
| Delete contact via CLI | Core milestone requirement | Low | HTTP DELETE to resource URL; `-y`/`--yes` flag per project convention |
| `createContact` GraphQL mutation (MCP) | Core milestone requirement | Low | Thin wrapper over CardDAV create; mirrors CLI behaviour |
| `updateContact` GraphQL mutation (MCP) | Core milestone requirement | Medium | Same read-merge-PUT logic as CLI path |
| `deleteContact` GraphQL mutation (MCP) | Core milestone requirement | Low | Follows PREVIEW/CONFIRM token pattern per project convention |
| vCard generation (RFC 6350 compliant) | Required by CardDAV PUT | Medium | Must produce valid vCard 3.0 with correct line-folding (75-octet limit), UID, FN, N properties |
| Contact URL/href tracking | Required for any write | Low | `Contact` struct needs a `href: Option<String>` field so CLI can address PUT/DELETE without a second lookup |
| Identify target contact by UID or name | UX requirement — user must be able to reference the contact to mutate | Low | CLI `--id` flag accepting UID from `contacts list`/`contacts search` output |
| Structured error output on write failure | Users need to know if write succeeded | Low | HTTP 4xx/5xx → anyhow error with status + body; follows existing error pattern |

---

## Differentiators

Features that make the tool stand out. Not required for baseline completeness, but add significant value.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Partial update semantics (field-level) | Users don't need to specify every field to update one; "only change what I pass" | High | Requires read-before-write: fetch existing vCard, parse it, overlay new values, serialise — no server-side partial update exists in CardDAV |
| ETag-based conditional write | Prevents overwriting a contact that was changed elsewhere between read and write | Medium | Store ETag from GET, send `If-Match` header on PUT/DELETE; server returns 412 on conflict |
| Address book selection | Users with multiple address books can target a specific one | Low | `--addressbook` flag defaulting to first/default book; already discoverable via `list_addressbooks()` |
| Multi-valued field support (multiple emails/phones) | vCard allows multiple EMAIL/TEL entries with TYPE params | Medium | CLI: comma-separated or repeated flags; MCP: array input type |
| TYPE labels on emails/phones (work/home/cell) | Contacts apps expect typed fields | Low | Already in `ContactEmail.label` and `ContactPhone.label`; just needs CLI flag exposure e.g. `--email work:alice@example.com` |
| Return created/updated contact in output | Confirmation of what was written | Low | After PUT, re-fetch and return the contact as JSON — follows existing `Output::success()` pattern |

---

## Anti-Features

Things to deliberately NOT build in this milestone.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Interactive TUI / prompt-driven contact editor | Breaks scripting, AI workflows, and pipe composition; inconsistent with project philosophy | Flag-only interface (`--name`, `--email`, etc.) |
| Contact photo/avatar upload | Binary data handling is a separate concern; not requested; adds significant complexity (MIME multipart or base64 in vCard) | Out of scope — noted in PROJECT.md |
| Contact groups / categories management | Requires address book collection semantics beyond vCard properties; separate feature | Out of scope — noted in PROJECT.md |
| Bulk/batch create or delete | Not requested; adds retry/rollback complexity | Can be scripted with shell loop over single operations |
| CSV or vCard file import/export | Separate feature with its own parsing concerns | Separate future milestone |
| Server-side search via CardDAV REPORT filters | CardDAV supports addressbook-query with filter elements, but current search is client-side and sufficient | Keep existing client-side search; no benefit for small contact lists |
| Undo / soft-delete / trash semantics | CardDAV DELETE is permanent; Fastmail has no CardDAV trash | Document irreversibility; use confirmation flag |
| Optimistic locking UI (retry on 412) | Over-engineering for a CLI; 412 should surface as an error the user resolves manually | Return clear error message on ETag conflict |

---

## Feature Dependencies

```
Contact struct href field
  → (required by all write operations — no href means no address to PUT/DELETE to)

vCard generation (RFC 6350)
  → Create contact
  → Update contact (partial)

Contact struct href field
  → Update contact (partial)   [need URL to PUT back to]
  → Delete contact             [need URL to DELETE]

contacts list / contacts search (existing)
  → Update contact             [user discovers UID/href from listing]
  → Delete contact             [user discovers UID/href from listing]

Update contact (partial): read-before-write
  → ETag-based conditional write (optional but strongly recommended)

Create contact (CLI)
  → createContact GraphQL mutation   [same CardDAV logic, different caller]

Update contact (CLI)
  → updateContact GraphQL mutation   [same read-merge-PUT logic]

Delete contact (CLI)
  → deleteContact GraphQL mutation   [same DELETE logic, MCP adds PREVIEW/CONFIRM token]
```

---

## Protocol Details (CardDAV Write Operations)

These are not user-facing features but constrain implementation choices and affect which
features are feasible at what complexity.

### Create (PUT — new resource)

- **URL:** `PUT {addressbook_href}{uid}.vcf`
  - UID generated by client (UUID v4 recommended per RFC 6352 §6.3.2)
  - File name conventionally matches UID: `{uuid}.vcf`
- **Headers required:** `Content-Type: text/vcard`
- **Headers recommended:** `If-None-Match: *` (fail if resource already exists — prevents accidental overwrite)
- **Success:** HTTP 201 Created or 204 No Content; server may return `ETag` response header
- **Failure:** 409 Conflict (UID collision), 412 Precondition Failed (If-None-Match violated)
- **vCard minimum:** BEGIN:VCARD, VERSION:3.0, UID, FN, END:VCARD

### Update (PUT — replace existing resource)

- **CardDAV has no PATCH** — full vCard must be PUT each time
- **Partial update approach:** GET existing vCard → parse → overlay changed fields → PUT
- **URL:** Same as resource href (discovered via PROPFIND/REPORT)
- **Headers recommended:** `If-Match: {etag}` (conditional PUT — prevents overwriting concurrent changes)
- **Success:** HTTP 204 No Content; server returns new `ETag`
- **Failure:** 412 Precondition Failed (ETag mismatch — contact was changed by another client)

### Delete (DELETE)

- **URL:** Resource href
- **Headers recommended:** `If-Match: {etag}` (conditional delete)
- **Success:** HTTP 204 No Content
- **Failure:** 404 Not Found, 412 Precondition Failed

### ETag acquisition

- ETags are returned in PROPFIND (`d:getetag` property) and REPORT responses
- The current `list_contacts` REPORT already requests `d:getetag` but the `Contact` struct
  does not store it — this needs to be added alongside `href`

---

## vCard Generation Requirements

For the vCard serialiser (new code needed — no existing generator in codebase):

| Property | Required | Notes |
|----------|----------|-------|
| BEGIN:VCARD | Yes | First line |
| VERSION:3.0 | Yes | Match what Fastmail returns |
| UID | Yes | Use UUID v4; must be stable (same UID = same resource URL) |
| FN | Yes | Full display name |
| N | Recommended | Structured name; for simple CLI input, derive from FN as `;;;FN;` or parse last/first |
| EMAIL | When present | `EMAIL;TYPE=work:addr` or `EMAIL:addr` |
| TEL | When present | `TEL;TYPE=cell:+1234` or `TEL:number` |
| ORG | When present | `ORG:Company Name` |
| TITLE | When present | `TITLE:Job Title` |
| NOTE | When present | `NOTE:freeform text` |
| END:VCARD | Yes | Last line |
| Line folding | Yes | Lines exceeding 75 octets must be folded with CRLF + SPACE (RFC 6350 §3.2) |

**Confidence:** HIGH — RFC 6350 is a finalized standard; vCard 3.0 is what Fastmail currently returns
per codebase evidence (`VERSION:3.0` in test data at `src/carddav/mod.rs` line 423).

---

## MVP Recommendation

Prioritize in this order:

1. **Add `href` and `etag` to `Contact` struct** — unblocks all write operations; zero user-facing
   change, purely internal
2. **vCard generation function** — needed by both create and update; test in isolation
3. **Create contact** (CLI + MCP) — most straightforward write; validates the PUT plumbing
4. **Delete contact** (CLI + MCP) — simple DELETE; validates the href/ETag path end-to-end
5. **Update contact** (CLI + MCP) — most complex due to read-merge-PUT; build last when
   GET/PUT/DELETE are proven

Defer to follow-on work:
- **Multi-valued field labels** (TYPE=work/home) — supported by struct already but CLI flag
  parsing adds complexity; default to no TYPE label is acceptable for MVP
- **ETag conditional writes** — implement, but treat 412 as a non-fatal user error rather than
  adding retry logic
- **Address book selection** — default to first discovered address book; `--addressbook` flag
  can be added without breaking changes later

---

## Sources

- RFC 6352 (CardDAV): https://www.rfc-editor.org/rfc/rfc6352 — HIGH confidence (finalized standard)
- RFC 6350 (vCard 4.0 / 3.0 compat): https://www.rfc-editor.org/rfc/rfc6350 — HIGH confidence
- Codebase analysis: `src/carddav/mod.rs`, `src/commands/contacts.rs`, `src/mcp/graphql/mutation.rs`,
  `src/mcp/graphql/types.rs`, `src/main.rs` — HIGH confidence (direct inspection)
- PROJECT.md and INTEGRATIONS.md — HIGH confidence (authoritative project documents)
