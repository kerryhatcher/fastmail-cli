# Architecture Patterns: CardDAV Contact Write Operations

**Domain:** CardDAV write operations (PUT/DELETE) in existing Rust async CLI
**Researched:** 2026-03-27
**Overall confidence:** HIGH — based on RFC 6352 (CardDAV), RFC 6350 (vCard 4.0), RFC 2426 (vCard 3.0), and direct analysis of the existing codebase

---

## Recommended Architecture

The write milestone slots directly into the existing layered architecture without structural changes. The pattern mirrors how the email send/move operations work in the JMAP layer: a new method on the protocol client, called from a thin command handler, exposed as a GraphQL mutation.

Four components are added:

1. **vCard builder** — a pure function (or struct) that generates valid vCard text from contact field inputs
2. **CardDAV write methods** — `create_contact`, `update_contact`, `delete_contact` on `CardDavClient`
3. **CLI command handlers** — `create`, `update`, `delete` subcommands in `src/commands/contacts.rs`
4. **GraphQL mutations** — `createContact`, `updateContact`, `deleteContact` in `src/mcp/graphql/mutation.rs`

---

## Component Boundaries

| Component | Location | Responsibility | Communicates With |
|-----------|----------|---------------|-------------------|
| `vCard builder` | `src/carddav/mod.rs` (private fn) | Generate valid vCard text from `ContactInput` | Called by `create_contact`, `update_contact` |
| `ContactInput` struct | `src/carddav/mod.rs` | Bundle create/update field parameters | Passed by CLI handlers and GraphQL mutations into `CardDavClient` |
| `CardDavClient::create_contact` | `src/carddav/mod.rs` | PUT new vCard to server; return created `Contact` with server-assigned ETag | CLI handler, GraphQL mutations |
| `CardDavClient::update_contact` | `src/carddav/mod.rs` | Fetch current ETag via PROPFIND, merge fields, PUT updated vCard with `If-Match` | CLI handler, GraphQL mutations |
| `CardDavClient::delete_contact` | `src/carddav/mod.rs` | DELETE the resource URL; optionally use `If-Match` for safety | CLI handler, GraphQL mutations |
| `CardDavClient::get_contact_etag` | `src/carddav/mod.rs` (private) | PROPFIND a single resource URL to retrieve its current ETag | Called by `update_contact`, `delete_contact` |
| CLI handlers (create/update/delete) | `src/commands/contacts.rs` | Parse clap args into `ContactInput`; call client; print `Output::success` | `CardDavClient`, `Config` |
| GraphQL mutations | `src/mcp/graphql/mutation.rs` | Expose write operations as `createContact`, `updateContact`, `deleteContact` mutations | `CardDavClient` via context |
| `GqlContactMutationResult` | `src/mcp/graphql/types.rs` | Return type for contact mutations (success flag, contact or error) | GraphQL mutations |
| Error variants | `src/error.rs` | `ContactNotFound(String)`, `ContactConflict(String)` | `CardDavClient`, propagated up |

---

## Data Flow

### Create Contact

```
CLI args (--name, --email, --phone, --org, --notes, --address)
  → ContactInput struct (validated in command handler)
    → build_vcard(&input) → vCard 3.0 text string
      → generate UUID for UID property (uuid crate, or std via random bytes)
        → PUT https://carddav.fastmail.com{addressbook_href}{uid}.vcf
          Content-Type: text/vcard
          (no If-Match — creating new resource)
          → 201 Created → extract ETag from response headers
            → return Contact { id: uid, ... }
              → Output::success(contact).print() / GqlContactMutationResult
```

### Update Contact (partial)

```
CLI args (--id <uid>, --name?, --email?, ...)
  → command handler: client.search for contact by UID to get href and current ETag
    → GET (or PROPFIND) to retrieve current vCard text + ETag
      → merge: apply only non-None fields from ContactInput over existing Contact
        → build_vcard(&merged) → updated vCard text
          → PUT {contact_href}
              If-Match: "{current_etag}"
              Content-Type: text/vcard
              → 204 No Content → success
              → 412 Precondition Failed → Error::ContactConflict("ETag mismatch — contact modified elsewhere")
                → Output::success(updated_contact) / GqlContactMutationResult
```

### Delete Contact

```
CLI args (--id <uid>, --confirm)
  → guard: if --confirm not passed → print error, exit 1
    → resolve contact href by UID (search contacts, match UID)
      → DELETE {contact_href}
          If-Match: "{current_etag}"   (optional but strongly recommended)
          → 204 No Content → success
          → 412 Precondition Failed → Error::ContactConflict
          → 404 Not Found → Error::ContactNotFound(uid)
            → Output::success({ "deleted": true, "id": uid }) / GqlContactMutationResult
```

### GraphQL Mutation Flow

```
MCP client → GraphQL mutation (createContact / updateContact / deleteContact)
  → mutation resolver acquires CardDavClient from Context<'_>
    → calls CardDavClient write method
      → returns GqlContactMutationResult { success, contact?, error? }
```

**Note on CardDavClient in GraphQL context:** The existing MCP server holds `JmapClient` in a `tokio::sync::Mutex`. `CardDavClient` must be added alongside it in the same context map, instantiated in `FastmailMcp::new()` from config, and retrieved in resolvers with `ctx.data::<CardDavClient>()`. `CardDavClient` is stateless (no session caching) so it does not need a Mutex — it can be stored directly (it already holds a `reqwest::Client` which is `Clone + Send + Sync`).

---

## ETag Semantics

ETags are the CardDAV mechanism for optimistic concurrency. The server assigns an ETag to each vCard resource when created or modified (RFC 6352 §6.3.2, RFC 7232).

**How the existing code ignores ETags:** The current `list_contacts` REPORT request fetches `d:getetag` but the parse step discards it — `parse_contacts_response` does not surface the ETag in the `Contact` struct.

**What needs to change for writes:**

1. **`Contact` struct needs an `etag: Option<String>` field.** The REPORT response already returns it via `<d:getetag>` — the parser just needs to capture it.

2. **Update flow uses `If-Match`:** Before PUT, the client must know the current ETag. Two sub-approaches:
   - **Fetch-then-write (recommended):** Issue a PROPFIND on the specific resource URL to retrieve the current ETag, then PUT with `If-Match: "{etag}"`.
   - **Store-and-use:** If the caller already has a `Contact` with a populated `etag` field (e.g., just fetched via list), pass it through. This is the faster path for the MCP use case where context is short-lived.
   - Use `If-Match: *` as a last resort if no ETag is available — this prevents creating a new resource but does not guard against concurrent updates.

3. **Create flow uses `If-None-Match: *`:** This tells the server to reject the PUT if a resource already exists at that URL. Use this on create to prevent accidental overwrites.

4. **412 Precondition Failed response:** Map to `Error::ContactConflict("Contact was modified by another client. Fetch the latest version and retry.")`.

**ETag format on Fastmail:** ETags are opaque quoted strings, e.g., `"abc123-def456"`. They must be stored and sent verbatim including the surrounding double-quotes in the header value.

---

## vCard Generation

The write path requires generating vCard text from structured input. The existing `parse_vcard` function is the inverse of what is needed.

**vCard 3.0 is the safe default** — the existing parser handles 3.0 cards, Fastmail's server uses 3.0, and vCard 4.0 is not needed for the fields in scope.

**Minimal valid vCard 3.0 structure:**

```
BEGIN:VCARD
VERSION:3.0
UID:<uuid>
FN:<full name>
N:<family>;<given>;<additional>;<prefix>;<suffix>
EMAIL;TYPE=INTERNET:<email>
TEL;TYPE=CELL:<phone>
ORG:<organization>
TITLE:<job title>
ADR;TYPE=HOME:;;;<street>;<city>;<region>;<postal code>;<country>
NOTE:<notes>
END:VCARD
```

**Implementation notes for `build_vcard`:**

- `FN` (formatted name) is required; derived from the `name` input field.
- `N` (structured name) is required by vCard 3.0 spec; if only a full name is available, place it in the "given name" slot and leave others empty: `N:;Full Name;;;;`.
- `UID` must be stable across updates. On create: generate a UUID (use `uuid` crate with v4). On update: preserve the existing UID.
- **Line folding:** RFC 6350 §3.2 requires lines longer than 75 octets to be folded with CRLF + space. The `unfold_vcard` function already handles unfolding for reads; a `fold_vcard_line` function is needed for writes. In practice, most field values in the scope (name, email, phone) are short enough to skip folding — implement it for correctness but it rarely triggers.
- **Character escaping:** vCard property values must escape `,`, `;`, `\`, and newlines as `\,`, `\;`, `\\`, `\n` respectively. Notes field is the most likely to contain these.
- **CRLF line endings:** vCard spec requires `\r\n` line endings. Many servers (including Fastmail) accept `\n`, but use `\r\n` for correctness.
- The `NOTE` property may contain newlines — encode them as `\n` (literal backslash-n in the vCard text).

**`build_vcard` function signature:**

```rust
fn build_vcard(uid: &str, input: &ContactInput) -> String
```

Where `ContactInput` holds `Option<String>` for every editable field:

```rust
pub struct ContactInput {
    pub name: Option<String>,
    pub emails: Option<Vec<ContactEmail>>,
    pub phones: Option<Vec<ContactPhone>>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub notes: Option<String>,
    pub address: Option<String>,   // free-form; maps to ADR
}
```

For update operations, merge `ContactInput` over an existing `Contact` — take `input.name.unwrap_or(existing.name)` for each field — then call `build_vcard` with the merged values.

---

## URL Conventions for Contact Resources

Fastmail's CardDAV URL pattern (confirmed from existing `list_contacts` REPORT responses):

```
Base: https://carddav.fastmail.com
Address book: /dav/addressbooks/user/{username}/Default/
Contact resource: /dav/addressbooks/user/{username}/Default/{uid}.vcf
```

**Create:** PUT to `{addressbook_href}{uid}.vcf` where `uid` is the newly generated UUID.

**Update/Delete:** The resource URL must be known. Three strategies:
1. Store it in `Contact.href` — add an `href: String` field to `Contact`. The REPORT response already provides `d:href` per resource alongside `address-data`. **This is the recommended approach** — it avoids a second lookup.
2. Reconstruct it as `{addressbook_href}{uid}.vcf` — works only if the UID matches the filename, which is true for contacts created by this client but not guaranteed for contacts created by other clients.
3. Issue a REPORT query with a filter on UID — expensive (fetches all vCards).

**Recommendation:** Add `href: Option<String>` to the `Contact` struct so it is populated during list/search, making update/delete O(1) lookups without extra network calls.

---

## Patterns to Follow

### Pattern 1: Thin Command Handlers

**What:** Command handlers in `src/commands/contacts.rs` stay thin (30-50 lines). They parse arguments, validate required flags (`--confirm` for delete), call `CardDavClient`, and print output. No protocol logic in the handler.

**Example sketch:**
```rust
pub async fn create_contact(name: String, emails: Vec<String>, ...) -> anyhow::Result<()> {
    let config = Config::load()?;
    let client = CardDavClient::new(config.get_username()?, config.get_app_password()?);
    let addressbooks = client.list_addressbooks().await?;
    let default_ab = addressbooks.first().ok_or_else(|| Error::Server("No address book".into()))?;
    let input = ContactInput { name: Some(name), emails: Some(...), ... };
    let contact = client.create_contact(&default_ab.href, input).await?;
    Output::success(contact).print();
    Ok(())
}
```

### Pattern 2: GqlStatus / GqlContactMutationResult for Mutations

**What:** Contact mutations return a result type with `success: bool`, optional `contact: Option<GqlContact>`, and `error: Option<String>`. This mirrors `GqlStatus` (used for simpler mutations) and `GqlComposeResult` (used for email mutations). Use a dedicated `GqlContactMutationResult` to carry back the created/updated contact.

```rust
#[derive(SimpleObject)]
pub struct GqlContactMutationResult {
    pub success: bool,
    pub contact: Option<GqlContact>,
    pub error: Option<String>,
}
```

### Pattern 3: ETag Fetch-Then-Write for Update

**What:** Before PUT on update, fetch the current ETag via PROPFIND on the specific resource. Send `If-Match: "{etag}"` on the PUT. On 412 response, surface a clear error message.

**Why:** Avoids silent data loss when two agents (e.g., CLI and an AI workflow via MCP) edit the same contact concurrently.

### Pattern 4: CardDavClient in MCP Context (no Mutex needed)

**What:** `CardDavClient` holds only a `reqwest::Client` (which is `Send + Sync + Clone`) and two `String` credentials. Unlike `JmapClient` which caches session state, `CardDavClient` has no mutable state. Store it directly in the GraphQL context without a Mutex.

```rust
// In FastmailMcp::new():
let carddav = CardDavClient::new(username, app_password);
let schema = Schema::build(QueryRoot, MutationRoot, EmptySubscription)
    .data(Mutex::new(jmap_client))
    .data(carddav)   // no Mutex needed
    .finish();
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: Reconstructing Contact URL from UID

**What:** Building the contact URL as `{addressbook_href}{uid}.vcf` for update/delete without verifying it.

**Why bad:** Contacts created by other clients (Fastmail web app, iOS, third-party CardDAV clients) may use different filename conventions. The UID in the vCard body does not have to match the resource filename.

**Instead:** Surface `href` from the REPORT response in the `Contact` struct. Use that href directly for write operations.

### Anti-Pattern 2: PUT Without If-Match on Update

**What:** Doing a PUT to update a contact without sending `If-Match`.

**Why bad:** Silent last-writer-wins clobber. If a contact was edited in the Fastmail web UI between when the CLI fetched it and when it writes, the web UI's changes are silently discarded.

**Instead:** Always send `If-Match: "{etag}"` on updates. Treat 412 as a user-actionable error.

### Anti-Pattern 3: Separate CardDavClient per mutation resolver

**What:** Constructing a new `CardDavClient::new(username, app_password)` inside each GraphQL resolver by reading config.

**Why bad:** Forces config reads on every mutation call; doesn't match the existing pattern for JMAP client (initialized once, passed via context).

**Instead:** Initialize `CardDavClient` once in `FastmailMcp::new()` and inject via `schema.data(carddav)`.

### Anti-Pattern 4: Skipping vCard Line Folding

**What:** Generating vCard text without folding long lines.

**Why bad:** RFC 6350 §3.2 requires lines > 75 octets to be folded. While Fastmail's server is lenient, this is technically invalid and could cause issues with other clients syncing the same address book.

**Instead:** Apply line folding in `build_vcard` for any line exceeding 75 bytes.

---

## Suggested Build Order

The components have clear dependencies that dictate build order:

**Stage 1 — Foundation (no new dependencies on other new code)**
- Add `etag: Option<String>` and `href: String` to `Contact` struct
- Update `parse_contacts_response` to extract `d:href` and `d:getetag` per response entry
- Add `ContactConflict(String)` and `ContactNotFound(String)` variants to `Error` enum
- Add `ContactInput` struct to `src/carddav/mod.rs`

**Stage 2 — vCard Builder (depends on Stage 1)**
- Implement `build_vcard(uid: &str, input: &ContactInput) -> String` in `src/carddav/mod.rs`
- Implement `fold_vcard_line(line: &str) -> String` helper
- Implement `escape_vcard_value(s: &str) -> String` helper
- Unit test all three with edge cases (long lines, special characters, multiple emails/phones)

**Stage 3 — CardDAV Write Methods (depends on Stage 2)**
- Implement `get_contact_etag(&self, href: &str) -> Result<String>` (PROPFIND single resource)
- Implement `create_contact(&self, addressbook_href: &str, input: ContactInput) -> Result<Contact>`
- Implement `update_contact(&self, contact: &Contact, input: ContactInput) -> Result<Contact>`
- Implement `delete_contact(&self, contact: &Contact) -> Result<()>`
- Integration-test against real Fastmail CardDAV (or mock HTTP responses)

**Stage 4 — CLI Commands (depends on Stage 3)**
- Add `create`, `update`, `delete` subcommands to `ContactsCommand` enum in `src/main.rs`
- Implement handlers in `src/commands/contacts.rs`
- The `delete` handler must require `--confirm` / `-y` flag; return error if absent

**Stage 5 — GraphQL Mutations (depends on Stage 3)**
- Add `GqlContact`, `GqlContactEmail`, `GqlContactPhone`, `GqlContactMutationResult` to `src/mcp/graphql/types.rs`
- Add `From<Contact>` and `From<ContactEmail>` / `From<ContactPhone>` conversions
- Add `CardDavClient` to MCP context in `src/mcp/graphql/mod.rs`
- Implement `createContact`, `updateContact`, `deleteContact` in `src/mcp/graphql/mutation.rs`

**Dependency graph summary:**
```
Stage 1 (Contact struct + errors)
  └─ Stage 2 (vCard builder)
       └─ Stage 3 (CardDavClient write methods)
            ├─ Stage 4 (CLI handlers)
            └─ Stage 5 (GraphQL mutations)
```

Stages 4 and 5 are independent of each other and can be built in parallel after Stage 3 completes.

---

## Scalability Considerations

These are irrelevant at this scope — contact books have hundreds to low thousands of contacts. The only performance concern is that `search_contacts` (and the address book resolution needed for write operations) fetches all contacts across all address books. For Fastmail's typical contact book size this is fine; the fetch-then-write latency is dominated by network round trips, not data volume.

---

## Sources

- RFC 6352 (CardDAV) — authoritative for PUT, DELETE, If-Match, If-None-Match, ETag semantics (HIGH confidence — RFC)
- RFC 6350 (vCard 4.0) — authoritative for vCard line structure, folding, escaping, property syntax (HIGH confidence — RFC)
- RFC 2426 (vCard 3.0) — defines VERSION:3.0 format used by Fastmail (HIGH confidence — RFC)
- RFC 7232 — HTTP conditional requests: If-Match, If-None-Match, ETag (HIGH confidence — RFC)
- `src/carddav/mod.rs` — direct analysis of existing URL patterns, REPORT XML, vCard parsing (HIGH confidence — source code)
- `src/mcp/graphql/mutation.rs` — direct analysis of existing mutation patterns for context injection, result types (HIGH confidence — source code)
- Fastmail CardDAV URL structure (`https://carddav.fastmail.com/dav/addressbooks/user/{username}/`) — observed from existing PROPFIND requests in codebase (HIGH confidence — source code)
