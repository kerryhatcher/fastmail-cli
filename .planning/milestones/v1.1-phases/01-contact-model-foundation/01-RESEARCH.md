# Phase 1: Contact Model Foundation - Research

**Researched:** 2026-03-27
**Domain:** Rust data model extension — CardDAV XML parsing, thiserror error variants
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** vCard UID is the user-facing contact identifier. Users pass UID to update/delete commands; the CLI resolves UID to href internally via a REPORT call before performing writes.
- **D-02:** `href` and `etag` are stored on the Contact struct but are not user-facing identifiers.
- **D-03:** `ContactConflict` uses a structured variant with three fields: `id` (contact UID), `sent_etag` (the ETag the client sent), and `server_etag` (the server's current ETag, if available).
- **D-04:** `ContactNotFound` follows the existing single-string tuple variant pattern: `ContactNotFound(String)` carrying the contact UID, consistent with `EmailNotFound` and `MailboxNotFound`.
- **D-05:** `href` and `etag` are always visible in JSON output (no `#[serde(skip)]`). Both fields appear in `contacts list` and `contacts search` responses.

### Claude's Discretion

- Implementation details of how REPORT response parsing extracts href and etag from the XML multistatus response
- Whether to use `Option<String>` or bare `String` for href/etag fields (consider that contacts without href/etag may exist in edge cases)
- Test structure and specific test cases beyond the success criteria

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MOD-01 | Contact struct includes `href` (resource URL) and `etag` fields populated from REPORT responses | XML multistatus structure documented; roxmltree traversal pattern established; `Option<String>` recommended for both fields |
| MOD-02 | Error type includes ContactConflict (412) and ContactNotFound variants | thiserror struct variant pattern confirmed; existing Error enum examined; variant signatures specified |
</phase_requirements>

---

## Summary

Phase 1 is a pure data model change with no external service calls, no new dependencies, and no CLI command changes. It consists of two tightly scoped modifications: (1) adding `href` and `etag` fields to the `Contact` struct and wiring them through `parse_contacts_response`, and (2) adding two error variants to the existing `Error` enum.

The existing codebase already requests `<d:getetag/>` in the REPORT XML body but discards the value during parsing. Phase 1 closes that gap by extracting both the `<d:href>` element (from the outer `<d:response>` wrapper) and the `<d:getetag>` element (from the `<d:propstat>/<d:prop>` block) and passing them into `parse_vcard` as extra parameters. The roxmltree traversal pattern is already established in the file — no new parsing techniques are needed.

The `GqlContact` struct in `src/mcp/graphql/types.rs` mirrors `Contact` via a `From<Contact>` impl. It must also receive the two new fields so GraphQL consumers can see them.

**Primary recommendation:** Add `href: Option<String>` and `etag: Option<String>` to `Contact`; extract from the XML `<d:response>` element in `parse_contacts_response`; mirror in `GqlContact`; add `ContactNotFound(String)` and `ContactConflict { id, sent_etag, server_etag }` to `Error`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| roxmltree | 0.21.1 | XML parsing for CardDAV multistatus responses | Already used in `carddav/mod.rs`; read-only DOM traversal |
| thiserror | 2.0.18 | Error type derivation | Already used in `src/error.rs`; powers the custom `Error` enum |
| serde | 1.0.228 | JSON serialization of Contact fields | Already derived on Contact, ContactEmail, ContactPhone |

No new dependencies required for this phase.

**Installation:** None — all dependencies already present in Cargo.toml.

---

## Architecture Patterns

### Recommended Project Structure

No new files needed. Changes touch:

```
src/
├── carddav/mod.rs     # Contact struct + parse_contacts_response + parse_vcard
├── error.rs           # Error enum — new ContactNotFound, ContactConflict variants
└── mcp/graphql/
    └── types.rs       # GqlContact + From<Contact> impl
```

### Pattern 1: Struct Field Addition with Option

**What:** Add `href: Option<String>` and `etag: Option<String>` to `Contact`.
**When to use:** Any time a field may legitimately be absent. Contacts returned via paths other than a REPORT (hypothetically) may not carry server metadata; `Option` is the safe choice and avoids requiring changes to any existing test fixtures.
**Example:**

```rust
// src/carddav/mod.rs — Contact struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub name: String,
    pub emails: Vec<ContactEmail>,
    pub phones: Vec<ContactPhone>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    /// Server-assigned resource URL (from CardDAV REPORT <d:href>).
    /// Required for PUT/DELETE write operations.
    pub href: Option<String>,
    /// HTTP ETag for optimistic concurrency control.
    /// Required for If-Match header in update/delete operations.
    pub etag: Option<String>,
}
```

### Pattern 2: Modifying parse_vcard to Accept Server Metadata

**What:** `parse_vcard` currently constructs a `Contact` from vCard text alone. After the change it must also receive `href` and `etag`, which come from the XML envelope, not the vCard body.
**When to use:** Any time struct construction spans two data sources.
**Example:**

```rust
// Before
fn parse_vcard(vcard_str: &str) -> Option<Contact>

// After — href and etag from the XML wrapper, not the vCard
fn parse_vcard(vcard_str: &str, href: Option<String>, etag: Option<String>) -> Option<Contact>
```

The call site in `parse_contacts_response` extracts both values before calling `parse_vcard`:

```rust
// Inside the for response in ... loop in parse_contacts_response
let href = response
    .descendants()
    .find(|n| n.has_tag_name((dav_ns, "href")))
    .and_then(|n| n.text())
    .map(|s| s.to_string());

let etag = response
    .descendants()
    .find(|n| n.has_tag_name((dav_ns, "getetag")))
    .and_then(|n| n.text())
    .map(|s| s.to_string());

if let Some(vcard_data) = response
    .descendants()
    .find(|n| n.has_tag_name((carddav_ns, "address-data")))
    .and_then(|n| n.text())
    && let Some(contact) = parse_vcard(vcard_data, href, etag)
{
    contacts.push(contact);
}
```

This keeps the let-chain structure matching the existing code at lines 205–213.

### Pattern 3: thiserror Struct Variant for ContactConflict

**What:** A struct variant in a thiserror enum carries named fields inline, unlike tuple variants.
**When to use:** When the error needs multiple named fields for diagnostics (user decision D-03).
**Example:**

```rust
// src/error.rs — two new variants

#[error("Contact not found: {0}")]
ContactNotFound(String),

#[error("Contact conflict for '{id}': sent ETag '{sent_etag}', server has '{server_etag:?}'")]
ContactConflict {
    id: String,
    sent_etag: String,
    server_etag: Option<String>,
},
```

`server_etag` is `Option<String>` because the server may not return the current ETag in every 412 response (it is implementation-defined).

### Pattern 4: Mirroring Contact Fields in GqlContact

**What:** `GqlContact` in `src/mcp/graphql/types.rs` is a `SimpleObject` that mirrors `Contact` via `From<Contact>`. New fields on `Contact` must propagate to `GqlContact`.
**When to use:** Whenever `Contact` gains a field that consumers (GraphQL) should expose.
**Example:**

```rust
#[derive(SimpleObject)]
#[graphql(name = "Contact")]
pub struct GqlContact {
    pub id: String,
    pub name: String,
    pub emails: Vec<GqlContactEmail>,
    pub phones: Vec<GqlContactPhone>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub href: Option<String>,
    pub etag: Option<String>,
}

impl From<Contact> for GqlContact {
    fn from(c: Contact) -> Self {
        Self {
            id: c.id,
            name: c.name,
            emails: c.emails.into_iter().map(GqlContactEmail::from).collect(),
            phones: c.phones.into_iter().map(GqlContactPhone::from).collect(),
            organization: c.organization,
            title: c.title,
            notes: c.notes,
            href: c.href,
            etag: c.etag,
        }
    }
}
```

### Anti-Patterns to Avoid

- **Storing href on AddressBook but not Contact:** `AddressBook` already has `href`, but `Contact` has historically lacked it. Do not conflate the two — Contact needs its own per-resource href that points to the `.vcf` file, not the address book collection.
- **Extracting href from vCard UID:** vCard UIDs are opaque identifiers chosen by the client and do not contain URL information. href must be taken from the XML `<d:href>` element.
- **Stripping ETag quotes:** Fastmail (like most servers) returns ETags with surrounding double-quotes (e.g., `"abc123"`). Store the value verbatim — RFC 2616 specifies the quotes are part of the ETag token and must be sent back in `If-Match` headers. Do not strip quotes.
- **Adding `#[serde(skip)]` to href/etag:** User decision D-05 prohibits this. Both fields must serialize to JSON.
- **Using a bare `String` for href/etag:** Contacts constructed by existing unit tests (`parse_vcard` called directly) have no server metadata. `Option<String>` avoids breaking existing tests and handles edge cases cleanly.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| XML namespace-aware element traversal | Custom string splitting on XML | `roxmltree` `.has_tag_name((ns, name))` | Already used; handles namespace aliases correctly |
| Error display formatting | Custom `Display` impl | `#[error("...")]` on thiserror variant | Already the project convention |
| JSON serialization of new fields | Manual `Serialize` impl | Derive — serde `#[derive(Serialize, Deserialize)]` already on Contact | Free via derive; new fields appear automatically |

**Key insight:** This phase adds no new logic — it wires existing infrastructure. All parsing machinery, serialization, and error formatting already exist in the codebase.

---

## Common Pitfalls

### Pitfall 1: ETag Value Includes Surrounding Quotes

**What goes wrong:** Code strips `"` from ETag values before storing them, then sends bare `abc123` in `If-Match: abc123` headers in later phases.
**Why it happens:** ETag values look like strings, but RFC 7232 defines them as quoted-string tokens. The quotes are significant.
**How to avoid:** Store the raw `.text()` value from roxmltree verbatim. E.g., `Some(r#""abc123""#.to_string())`. Later, when building the `If-Match` header, pass the stored value directly.
**Warning signs:** If `server_etag` in `ContactConflict` ever contains bare hex without quotes, the value was stripped.

### Pitfall 2: href Is a Path, Not a Full URL

**What goes wrong:** Code stores the href and later constructs a PUT URL by prepending CARDDAV_BASE directly, but the stored href already starts with `/dav/...` and the concatenation is correct — however if href is ever returned as a full URL by the server, the double-prefix breaks things.
**Why it happens:** Fastmail returns relative paths in href (e.g., `/dav/addressbooks/user/foo/Default/abc.vcf`), consistent with the `CARDDAV_BASE` pattern already used in `list_addressbooks`. But the spec allows full URLs.
**How to avoid:** In Phase 3, check whether href starts with `http` before prepending CARDDAV_BASE. For Phase 1, just store the raw value — no URL construction occurs here.
**Warning signs:** 404 errors on write operations in Phase 3 where the URL is doubled.

### Pitfall 3: parse_vcard Signature Change Breaks Existing Unit Tests

**What goes wrong:** Changing `parse_vcard(vcard_str: &str)` to `parse_vcard(vcard_str: &str, href: Option<String>, etag: Option<String>)` breaks the six existing unit tests that call `parse_vcard` directly.
**Why it happens:** Unit tests at lines 422–462 call `parse_vcard(vcard)` directly.
**How to avoid:** Update all six call sites in the test block to pass `None, None` for the new parameters. This is mechanical and the compiler will flag every missed site.
**Warning signs:** `error[E0061]: this function takes 3 arguments but 1 argument was supplied` at compile time — easy to catch.

### Pitfall 4: GqlContact Diverges from Contact

**What goes wrong:** `Contact` gains `href` and `etag` but `GqlContact` is not updated, causing a compilation error in the `From<Contact>` impl.
**Why it happens:** The two structs are defined in different files. A developer might update one and forget the other.
**How to avoid:** Update `GqlContact` and `From<Contact>` in the same commit as the `Contact` change. The compiler will fail the build if `From<Contact>` tries to set fields that don't exist on `GqlContact`, or if it leaves out fields that do.
**Warning signs:** `error[E0063]: missing field 'href' in initializer of GqlContact` or similar at compile time.

---

## Code Examples

Verified patterns from existing source:

### Existing roxmltree traversal pattern (carddav/mod.rs lines 114–149)

```rust
// Source: src/carddav/mod.rs parse_addressbooks_response
for response in doc
    .descendants()
    .filter(|n| n.has_tag_name((dav_ns, "response")))
{
    let href = response
        .descendants()
        .find(|n| n.has_tag_name((dav_ns, "href")))
        .and_then(|n| n.text())
        .unwrap_or_default();
    // ...
}
```

Extracting etag follows the identical pattern — swap `"href"` for `"getetag"` and use `.map(|s| s.to_string())` instead of `.unwrap_or_default()` to get an `Option<String>`.

### Existing thiserror pattern (error.rs)

```rust
// Source: src/error.rs — tuple variant pattern for "not found"
#[error("Email not found: {0}")]
EmailNotFound(String),

// Source: src/error.rs — struct variant pattern (existing example: Jmap)
#[error("JMAP error: {method} failed - {error_type}: {description}")]
Jmap {
    method: String,
    error_type: String,
    description: String,
},
```

`ContactNotFound` follows `EmailNotFound`. `ContactConflict` follows the `Jmap` struct variant pattern.

### Existing let-chain in parse_contacts_response (carddav/mod.rs lines 205–213)

```rust
// Source: src/carddav/mod.rs lines 205-213 (Rust 2024 edition let-chain)
if let Some(vcard_data) = response
    .descendants()
    .find(|n| n.has_tag_name((carddav_ns, "address-data")))
    .and_then(|n| n.text())
    && let Some(contact) = parse_vcard(vcard_data)
{
    contacts.push(contact);
}
```

The updated version adds `href` and `etag` extraction before this block and threads them into `parse_vcard`.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| thiserror 1.x attribute syntax | thiserror 2.x (2.0.18) — compatible syntax, same `#[error(...)]` macros | Cargo.lock: 2.0.18 | No breaking change for this phase |

**Deprecated/outdated:**
- Nothing relevant to this phase. All patterns in use are current.

---

## Open Questions

1. **Should `etag` store the raw value with surrounding quotes?**
   - What we know: RFC 7232 §2.3 defines ETag as a quoted-string. Fastmail CardDAV servers return values like `"abc123"`. The existing `parse_addressbooks_response` does not deal with ETags.
   - What's unclear: Whether Fastmail always includes the quotes in the getetag text node, or whether it sends bare hex.
   - Recommendation: Store verbatim (with quotes if present). Add a unit test with a fixture containing `<d:getetag>"abc123"</d:getetag>` to confirm the stored value. If bare values are observed, document and handle in Phase 3.

2. **`Option<String>` vs `String` for href and etag?**
   - What we know: All six existing unit tests construct contacts via `parse_vcard` directly without XML context. Using bare `String` would require those tests to supply hrefs, which they don't have.
   - Recommendation: Use `Option<String>` for both. It accurately models the reality that contacts created from vCard data alone (e.g., in tests or future import flows) have no server-assigned metadata yet.

---

## Environment Availability

Step 2.6: SKIPPED — this phase makes no external service calls and introduces no new tool dependencies. It is pure Rust code changes.

---

## Validation Architecture

`workflow.nyquist_validation` is explicitly `false` in `.planning/config.json`. This section is omitted per instructions.

---

## Project Constraints (from CLAUDE.md)

The following directives from `CLAUDE.md` apply to this phase:

| Directive | Impact on Phase 1 |
|-----------|-------------------|
| Follow existing patterns: clap derive, async-graphql, reqwest | New error variants must use thiserror `#[error(...)]` — confirmed |
| CardDAV protocol: PUT for create/update, DELETE for delete | No write operations in Phase 1 — not applicable |
| Reuse existing auth mechanism | No auth changes in Phase 1 — not applicable |
| snake_case for all functions, PascalCase for structs | `href`, `etag` fields are snake_case; `ContactNotFound`, `ContactConflict` are PascalCase |
| `Option<T>` for optional fields | Confirmed: `href` and `etag` recommended as `Option<String>` |
| Unit tests in `#[cfg(test)]` block at module end | New tests for parsing go at bottom of `carddav/mod.rs` |
| No logging in library code | Do not add `info!`/`warn!` to carddav parsing functions |
| Conventional Commits, branch for changes (branching_strategy: none in config — commit to main) | Config shows `branching_strategy: none`; commits go to main |
| Always commit after completing each task/story/phase | Commit after Phase 1 is fully implemented |
| Rust 2024 edition | let-chain syntax (`let ... && let ...`) is valid — already used in codebase |

---

## Sources

### Primary (HIGH confidence)

- `src/carddav/mod.rs` (read directly) — Contact struct definition, parse_contacts_response, parse_vcard, roxmltree traversal patterns
- `src/error.rs` (read directly) — Error enum with thiserror variants, tuple and struct variant patterns
- `src/mcp/graphql/types.rs` (read directly) — GqlContact struct, From<Contact> impl
- `src/commands/contacts.rs` (read directly) — CLI integration point
- `Cargo.toml` + `Cargo.lock` (read directly) — exact dependency versions

### Secondary (MEDIUM confidence)

- RFC 7232 §2.3 — ETag as quoted-string token (well-established HTTP spec, unambiguous)
- CLAUDE.md project conventions (read directly) — naming, error handling, test placement patterns

### Tertiary (LOW confidence)

- None — all claims in this document are supported by direct code inspection or established RFCs.

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH — all libraries already in Cargo.toml, versions confirmed from lockfile
- Architecture: HIGH — derived directly from reading existing source files; no external research needed
- Pitfalls: HIGH — derived from reading the code and identifying the specific gaps (missing etag extraction, signature change impact, GqlContact mirror)

**Research date:** 2026-03-27
**Valid until:** Stable — pure Rust model changes; no external service dependencies
