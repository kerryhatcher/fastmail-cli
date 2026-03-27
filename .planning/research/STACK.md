# Technology Stack: CardDAV Write Operations

**Project:** fastmail-cli — Contact CRUD milestone
**Researched:** 2026-03-27
**Scope:** What's needed to add CardDAV write (PUT/DELETE) to the existing read-only CardDAV client
**Overall Confidence:** HIGH — based on RFC 6352/6350 protocol specs, direct codebase inspection, and Cargo.lock version verification

---

## Executive Finding

No new crate dependencies are strictly required for CardDAV write operations. The existing stack
(reqwest, roxmltree, serde, uuid as a transitive dep) covers all HTTP mechanics. The only addition
needed is `uuid` as a direct dependency with the `v4` feature to generate stable contact resource
identifiers. vCard generation is plain string formatting — a dedicated vCard crate would add
complexity without meaningful benefit at this field-count scope.

---

## What Changes vs the Existing Stack

### New Direct Dependency: uuid 1.22.0 with v4 feature

| Crate | Version | Purpose | Feature flags |
|-------|---------|---------|---------------|
| `uuid` | `1.22.0` | Generate RFC 4122 v4 UUIDs for new contact UID and resource URL | `v4` |

**Why uuid:** CardDAV requires each contact resource to have a stable URL. The URL is constructed
from the contact's UID: `{addressbook_href}{uid}.vcf`. A UUID v4 ensures global uniqueness
without coordination. The crate is already in `Cargo.lock` as a transitive dependency at 1.22.0
(pulled by another dep); promoting it to a direct dep with the `v4` feature is zero-cost in
dependency tree terms.

**Why not generate IDs another way:** The existing `hash_id()` function in `src/carddav/mod.rs`
uses DefaultHasher on the contact name. This is acceptable for read-side deduplication but
unsuitable for write-side resource naming: name-based hashes collide on duplicates, change if
the name is updated, and do not satisfy the CardDAV requirement for a stable, globally unique
resource identifier.

**Confidence:** HIGH — uuid 1.22.0 is already in Cargo.lock (line 4380-4387), confirmed via
direct inspection.

```toml
# Add to Cargo.toml [dependencies]
uuid = { version = "1.22.0", features = ["v4"] }
```

### No New Dependencies for HTTP Mechanics

`reqwest` 0.13.1 already handles arbitrary HTTP methods. The existing code at
`src/carddav/mod.rs:82` shows the pattern:

```rust
self.client
    .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
```

The same pattern works for PUT and DELETE. No additional HTTP client, no WebDAV library.

### No vCard Generation Crate

**Verdict: Build it inline as a function in `src/carddav/mod.rs`.**

**Rationale:**

The codebase already hand-parses vCard in `parse_vcard()` (~80 lines). The fields in scope
(FN, N, EMAIL, TEL, ORG, TITLE, NOTE, UID, VERSION) are a subset of vCard 3.0. Generating
these is straightforward string construction with RFC 6350 line-folding at 75 octets.

The Rust vCard ecosystem is thin: there is no widely-adopted, actively-maintained vCard
serialization crate with significant download counts. Candidates reviewed:

- `vcard4` — targets vCard 4.0 only, not vCard 3.0 which Fastmail stores (confirmed from
  the test in `src/carddav/mod.rs:423` showing `VERSION:3.0`). Adds a compile dependency
  for a format mismatch.
- `icalendar` — primarily iCal, vCard is secondary and sparsely maintained.
- Hand-rolled serializer — matches the project's existing hand-rolled parser, stays in the
  same file, trivially testable, zero dependency footprint.

**Confidence:** MEDIUM — ecosystem assessment based on crates.io knowledge as of August 2025
training data. No network access was available to verify current download counts. The
recommendation to avoid vCard crates is robust regardless because vCard 3.0 generation for
the required fields is genuinely simple and the project's pattern is to avoid unnecessary deps.

---

## CardDAV PUT/DELETE: Protocol Specifics

### PUT — Create a New Contact

**HTTP method:** `PUT`
**URL pattern:** `{CARDDAV_BASE}{addressbook_href}{uuid}.vcf`
Example: `https://carddav.fastmail.com/dav/addressbooks/user/alice@fastmail.com/Default/550e8400-e29b-41d4-a716-446655440000.vcf`

**Required headers:**

| Header | Value | Purpose |
|--------|-------|---------|
| `Content-Type` | `text/vcard; charset=utf-8` | CardDAV spec requirement (RFC 6352 §10.1) |
| `Authorization` | `Basic {b64}` | Existing pattern — reqwest `.basic_auth()` |
| `If-None-Match` | `*` | **Create guard:** server rejects with 412 if resource already exists |

**Success response:** `201 Created`

**Body:** Complete vCard 3.0 document (see vCard format section below).

**Confidence:** HIGH — RFC 6352 §5.3.1 specifies PUT for resource creation. `If-None-Match: *`
is the standard WebDAV conditional request guard for creation-only semantics (RFC 4918 §14.26).

---

### PUT — Update an Existing Contact

**HTTP method:** `PUT`
**URL pattern:** Same as create — `{CARDDAV_BASE}{addressbook_href}{uid}.vcf`

**The ETag problem:** The existing `list_contacts()` REPORT query already requests `<d:getetag/>`
in its body (line 163) but the parser in `parse_contacts_response()` discards the ETag value.
To safely update, the ETag must be captured and sent as `If-Match`.

**Required headers:**

| Header | Value | Purpose |
|--------|-------|---------|
| `Content-Type` | `text/vcard; charset=utf-8` | Same as create |
| `Authorization` | `Basic {b64}` | Existing pattern |
| `If-Match` | `"{etag}"` | **Update guard:** prevents overwriting concurrent edits (RFC 4918 §14.24) |

**Success response:** `204 No Content` or `200 OK`

**Failure response:** `412 Precondition Failed` — ETag mismatch (concurrent edit); retry by
re-fetching the contact.

**Implication for the Contact model:** `Contact` struct in `src/carddav/mod.rs` needs an
`etag: Option<String>` field, populated when fetching. The `parse_contacts_response()` method
needs to extract the `<d:getetag>` value from each `<d:response>` element.

**Confidence:** HIGH — RFC 4918 §9.6 (conditional PUT), RFC 6352 §5.3.2.

---

### DELETE — Remove a Contact

**HTTP method:** `DELETE`
**URL pattern:** `{CARDDAV_BASE}{addressbook_href}{uid}.vcf`

**Required headers:**

| Header | Value | Purpose |
|--------|-------|---------|
| `Authorization` | `Basic {b64}` | Existing pattern |
| `If-Match` | `"{etag}"` | **Recommended.** Prevents deleting a stale/wrong version |

**Success response:** `204 No Content`

**`If-Match` for DELETE:** Technically optional per RFC 4918, but strongly recommended.
Without it, a contact that was concurrently modified would be deleted anyway. Given the
project already fetches ETags in REPORT requests, using them for DELETE is straightforward.

**Confidence:** HIGH — RFC 4918 §9.6, standard WebDAV practice.

---

## vCard 3.0 Format for PUT Bodies

The project stores vCard 3.0 (confirmed from existing test at `src/carddav/mod.rs:423`).
Fastmail's CardDAV server has been observed serving `VERSION:3.0` contacts.

### Required Structure

```
BEGIN:VCARD
VERSION:3.0
UID:{uuid}
FN:{full name}
N:{last};{first};;;
[optional fields]
END:VCARD
```

### Line Folding (RFC 6350 §3.2)

Lines exceeding 75 octets must be folded by inserting CRLF followed by a single space.
The existing `unfold_vcard()` handles the read side. A `fold_vcard_line()` function is
needed for the write side.

**Folding rule:** Split at 75-octet boundaries (byte count, not character count for Unicode
correctness), inserting `\r\n ` (CRLF + space) before the continuation.

### Line Endings

RFC 6350 specifies CRLF (`\r\n`) as line endings for vCard. Most CardDAV servers including
Fastmail accept LF-only, but CRLF is correct per spec. Use `\r\n` to be safe.

### Special Character Escaping (RFC 6350 §3.4)

For property values, the following characters must be escaped:
- `\` → `\\`
- `,` → `\,`
- `;` → `\;`
- Newline → `\n`

The N property (structured name) uses `;` as a field separator, so values in name components
must have `;` escaped.

### Field Mapping for the Contact Struct

| vCard property | Contact field | Notes |
|----------------|---------------|-------|
| `UID` | `id` | UUID v4 string |
| `FN` | `name` | Full display name |
| `N` | Derived from `name` | Split on last space for `Last;First;;;` heuristic, or `{name};;;;` |
| `EMAIL;TYPE={label}` | `emails[]` | label defaults to `internet` if None |
| `TEL;TYPE={label}` | `phones[]` | label defaults to `voice` if None |
| `ORG` | `organization` | |
| `TITLE` | `title` | |
| `NOTE` | `notes` | |

### N Property Heuristic

vCard requires the N (structured name) property. The `Contact` struct only has a `name` string
(full display name from FN). Use this heuristic:

- If `name` contains a space: split on last space → `Family;Given;;;`
- If `name` has no space: `{name};;;;`

This is consistent with how Fastmail generates N from FN on its own web interface.

**Confidence for format details:** HIGH — RFC 6350 is authoritative and stable.

---

## ETag Storage: Required Model Change

To support update and delete operations, ETags must be captured at fetch time.

**Change needed in `src/carddav/mod.rs`:**

1. Add `etag: Option<String>` to the `Contact` struct.
2. In `parse_contacts_response()`, extract the `<d:getetag>` text from each `<d:response>`
   element and populate `contact.etag`.
3. The ETag string from DAV responses is typically quoted (e.g., `"abc123"`). Store it as-is
   including quotes, since `If-Match` expects the quoted form.

**Impact on existing code:** `Contact` gains one optional field. All existing serialization
continues to work. The GraphQL `GqlContact` type in `src/mcp/graphql/types.rs` does not need
to expose ETag (it's a protocol detail, not a user-facing field).

**Confidence:** HIGH — direct code inspection confirms ETags are fetched but discarded.

---

## URL Construction for Write Operations

The existing code only stores `addressbook_href` (e.g., `/dav/addressbooks/user/alice/Default/`).
To address individual contacts, the href of each vCard resource is needed.

**For CREATE:** Construct as `{addressbook_href}{new_uuid}.vcf`

**For UPDATE/DELETE:** The contact's resource URL must be known. Two approaches:

1. **Store the resource href on Contact** (recommended): Add `href: String` to `Contact`,
   populated from the `<d:href>` element within each `<d:response>` in REPORT parsing.
   This is already in the XML — it just isn't stored.

2. **Reconstruct from UID**: `{addressbook_href}{uid}.vcf`. Works only when the UID matches
   the filename, which is conventional but not guaranteed.

**Recommendation:** Store `href: String` on `Contact`. The REPORT response already provides
it in `<d:href>`. This is the authoritative URL — reconstructing from UID is an assumption
that breaks on servers that use non-UID filenames (Fastmail typically uses UIDs but it's
not guaranteed).

**Confidence:** HIGH — RFC 6352 §8.7 specifies that the response href is the resource URL.

---

## What NOT to Use

| Option | Why Not |
|--------|---------|
| `vcard4` crate | Targets vCard 4.0 only; Fastmail stores 3.0; adds a dep to avoid a trivial string builder |
| `icalendar` crate | Primarily iCal; vCard support is secondary and sparsely used |
| A WebDAV client crate (`webdav-handler`, `dav-server`) | These are server-side frameworks, not HTTP clients; reqwest is the right tool |
| `hyper` directly | Already abstracted by reqwest; no benefit for this use case |
| Custom HTTP method via `.method()` enum | reqwest supports custom methods via `Method::from_bytes()`, already demonstrated in codebase |
| `If-None-Match` for updates | Using `*` on PUT without checking existing ETag will silently overwrite concurrent edits |

---

## Summary: Cargo.toml Changes

```toml
# Add to [dependencies] — one new direct dependency
uuid = { version = "1.22.0", features = ["v4"] }
```

No other Cargo.toml changes are needed. All HTTP mechanics, XML parsing, and serialization
requirements are covered by the existing stack.

---

## Confidence Assessment

| Area | Confidence | Basis |
|------|------------|-------|
| HTTP method/header requirements | HIGH | RFC 6352, RFC 4918, confirmed by codebase pattern at line 82 |
| vCard 3.0 format | HIGH | RFC 6350, cross-referenced with existing parse_vcard() tests |
| ETag requirement | HIGH | Direct code inspection — ETags fetched but not stored |
| uuid crate version | HIGH | Cargo.lock direct inspection shows 1.22.0 already present |
| vCard crate ecosystem | MEDIUM | Knowledge as of Aug 2025; no live crates.io query possible; recommendation is robust regardless |
| URL construction approach | HIGH | RFC 6352 §8.7, existing addressbook parsing pattern |

---

## Sources

- RFC 6352 (CardDAV): https://datatracker.ietf.org/doc/html/rfc6352
- RFC 6350 (vCard 4.0/3.0): https://datatracker.ietf.org/doc/html/rfc6350
- RFC 4918 (WebDAV, conditional requests): https://datatracker.ietf.org/doc/html/rfc4918
- Cargo.lock: `/home/kwhatcher/projects/fastmail-cli/Cargo.lock` lines 4379-4387 (uuid 1.22.0)
- Existing ETag request: `src/carddav/mod.rs` line 163
- Existing HTTP method pattern: `src/carddav/mod.rs` line 82

---

*Research date: 2026-03-27*
