# Phase 18: Group Data Model, CRUD, and Base Surfaces - Research

**Researched:** 2026-04-13
**Domain:** CardDAV / vCard 3.0 contact group extensions (Fastmail), Rust async, async-graphql, clap derive
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Group Data Model & vCard Representation**
- `ContactGroup` struct lives in `src/carddav/mod.rs` alongside `Contact`
- Members represented as `Vec<String>` storing contact UIDs (extracted from `X-ADDRESSBOOKSERVER-MEMBER` URN values) — resolve to full contacts only at display/query time
- `parse_vcard()` checks for `X-ADDRESSBOOKSERVER-KIND:group` line and returns `None` to filter group vCards from contact listings; separate `parse_group_vcard()` function handles group parsing
- Group ID is the vCard UID (same as contact IDs)

**CLI Command Structure**
- Groups nested under contacts: `contacts groups list/create/get/rename/delete`
- Group identifier accepts both group ID (UID) and group name for convenience — name lookup errors on ambiguity
- `groups get` outputs JSON object with group metadata + resolved member contacts array
- `groups delete` requires `--confirm` flag (matches existing `contacts delete` pattern), rejected without it

**MCP/GraphQL Surface & CardDAV Transport**
- Separate `ContactGroup` GraphQL type with fields: `id`, `name`, `memberCount`, `members: [Contact!]!` (resolved)
- Group creation: PUT a vCard with `X-ADDRESSBOOKSERVER-KIND:group` and `FN:<name>` to a new UUID-based href on the default address book
- Group rename: fetch current vCard → update `FN:` line → PUT back with `If-Match: <etag>` (ETag-guarded)
- MCP group types live in new `src/mcp/graphql/types/group.rs` with resolvers added to existing `mutation.rs`/`query.rs`

### Claude's Discretion

None — all decisions captured in locked decisions above.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GRP-01 | User can list contact groups showing name, member count, and group ID | `list_groups()` CardDAV method returns groups filtered from REPORT response; client-side KIND filtering |
| GRP-02 | User can create an empty contact group with a name | PUT with `X-ADDRESSBOOKSERVER-KIND:group` + `FN:<name>` to UUID-based href; existing `create_contact` pattern |
| GRP-03 | User can get a group's details including resolved member contacts | Fetch group by ID, then resolve each member UID via `get_contact_by_id()`; returns `ContactGroup` with resolved `Vec<Contact>` |
| GRP-04 | User can rename an existing contact group | Fetch group vCard → update `FN:` line → PUT with `If-Match: <etag>`; mirrors existing `update_contact` ETag pattern |
| GRP-05 | User can delete a contact group (members are NOT deleted) | DELETE href with `If-Match: <etag>`; mirrors `delete_contact`; members untouched |
| CLI-01 | User can manage groups via `contacts groups` subcommands | Add `Groups(GroupsCommands)` variant to `ContactsCommands`; nested subcommand enum pattern |
| CLI-03 | Group delete requires `--confirm` flag, consistent with contact delete | `--confirm` flag with `if !(confirm || yes)` guard in `main.rs` match arm — exact contact delete pattern |
| MCP-01 | AI agents can query groups via `listGroups` and `getGroup` | New resolvers in `query.rs`; `AppContext::get_carddav()` for client; `GqlContactGroup` SimpleObject |
| MCP-02 | AI agents can mutate groups via `createGroup`, `renameGroup`, `deleteGroup` | New resolvers in `mutation.rs`; `GroupDeleteAction` enum (Preview/Confirm) following `ContactDeleteAction` pattern |
</phase_requirements>

---

## Summary

Phase 18 implements the full contact group CRUD lifecycle on Fastmail's CardDAV server using the `X-ADDRESSBOOKSERVER` vCard 3.0 extension format. All required patterns already exist in the codebase — the work is additive, not architectural. The primary technical challenge is correctly filtering group vCards out of the contact list while parsing them accurately as a first-class type, and serializing/deserializing the `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` lines.

The existing `CardDavClient` already provides all required HTTP primitives (PROPFIND, REPORT, PUT with `If-None-Match`/`If-Match`, DELETE), and the existing `Contact`/`parse_vcard()`/`serialize_vcard()` functions establish the exact patterns that `ContactGroup`/`parse_group_vcard()`/`serialize_group_vcard()` must replicate. The CLI integration follows the established nested-subcommand pattern (`CalendarsCommands`, `EventsCommands`), and the MCP/GraphQL integration follows the `GqlContact`/`GqlCalendar` type + resolver pattern.

The key correctness constraint: Fastmail uses `X-ADDRESSBOOKSERVER-KIND:group` (vCard 3.0 extension), NOT `KIND:group` (vCard 4.0). Using the wrong property causes the server to silently accept but not recognize the group.

**Primary recommendation:** Add `ContactGroup` struct and group methods to `src/carddav/mod.rs`, modify `parse_contacts_from_xml()` to filter group vCards, add `GroupsCommands` subcommand to `src/main.rs`, add group handler functions to `src/commands/contacts.rs`, and add `GqlContactGroup` type plus resolvers to the MCP layer. All following the exact patterns of their contact/calendar counterparts.

---

## Standard Stack

### Core (already in Cargo.toml — no new dependencies required)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| reqwest | 0.13.1 | HTTP client for CardDAV PUT/DELETE/REPORT | Already used for all CardDAV ops |
| roxmltree | 0.21.1 | XML parsing for CardDAV REPORT multistatus | Already used in `parse_contacts_from_xml()` |
| async-graphql | 7 | GraphQL types and resolvers for MCP | Already used for all MCP surfaces |
| serde | 1.0.228 | `#[derive(Serialize, Deserialize)]` on `ContactGroup` | Pattern established on `Contact` |
| uuid | (already used) | UUID v4 generation for new group IDs | Same as contacts |
| tokio | 1.49.0 | Async runtime | Already present |

**No new Cargo dependencies are required for this phase.**

---

## Architecture Patterns

### Files to Modify and Create

```
src/
├── carddav/mod.rs          # Add ContactGroup struct, parse_group_vcard(),
│                           #   serialize_group_vcard(), list_groups(),
│                           #   create_group(), get_group_by_id(),
│                           #   rename_group(), delete_group() methods
│                           #   Modify parse_contacts_from_xml() to filter KIND:group
├── error.rs                # Add GroupNotFound, GroupConflict, GroupAmbiguous variants
├── commands/contacts.rs    # Add GroupInput, group handler fns (list_groups, create_group,
│                           #   get_group, rename_group, delete_group)
│                           #   Add group record helpers (create_group_record, etc.)
├── main.rs                 # Add GroupsCommands enum, Groups variant in ContactsCommands,
│                           #   match arms for all GroupsCommands variants
└── mcp/graphql/
    ├── types.rs            # Add GqlContactGroup, GqlGroupMutationResult,
    │                       #   GqlGroupDeleteResult, GroupDeleteAction enum
    ├── query.rs            # Add list_groups, get_group resolvers
    └── mutation.rs         # Add create_group, rename_group, delete_group resolvers
```

### Pattern 1: ContactGroup Struct (mirrors Contact)

```rust
// src/carddav/mod.rs — follows Contact struct pattern exactly
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactGroup {
    /// Unique ID (from UID property) — also serves as the CLI/API group identifier
    pub id: String,
    /// Display name (FN property)
    pub name: String,
    /// Member contact UIDs (from X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid> lines)
    /// Stored raw; resolved to full Contact objects only at display time.
    pub member_uids: Vec<String>,
    /// Server-assigned resource URL — required for PUT/DELETE
    pub href: Option<String>,
    /// HTTP ETag — required for If-Match in update/delete
    pub etag: Option<String>,
}
```

### Pattern 2: KIND Filtering in parse_contacts_from_xml()

The existing `parse_vcard()` function is called for every vCard in the REPORT response. The filter must happen there, before pushing to the contacts vec:

```rust
// Existing call site in parse_contacts_from_xml() — add KIND check
if let Some(vcard_data) = response
    .descendants()
    .find(|n| n.has_tag_name((carddav_ns, "address-data")))
    .and_then(|n| n.text())
{
    // parse_vcard() must return None for group vCards
    if let Some(contact) = parse_vcard(vcard_data, href, etag) {
        contacts.push(contact);
    }
    // group vCards silently skipped — no error, no malformed contact
}
```

Inside `parse_vcard()`, add early return before constructing the Contact:

```rust
// In parse_vcard(), after unfolding — check for group KIND
for line in unfolded.lines() {
    let line = line.trim();
    if line.eq_ignore_ascii_case("X-ADDRESSBOOKSERVER-KIND:group") {
        return None; // Silently exclude group vCards from contact list
    }
    // ... existing field parsing continues unchanged
}
```

### Pattern 3: parse_group_vcard() (parallel to parse_vcard())

```rust
/// Parse a vCard that is known to be a group (X-ADDRESSBOOKSERVER-KIND:group)
/// into a ContactGroup. Returns None if parsing fails.
fn parse_group_vcard(vcard_str: &str, href: Option<String>, etag: Option<String>) -> Option<ContactGroup> {
    let unfolded = unfold_vcard(vcard_str);
    let mut id = String::new();
    let mut name = String::new();
    let mut is_group = false;
    let mut member_uids = Vec::new();

    for line in unfolded.lines() {
        let line = line.trim();
        if line.eq_ignore_ascii_case("X-ADDRESSBOOKSERVER-KIND:group") {
            is_group = true;
        } else if line.starts_with("UID") && line.contains(':') {
            id = line.split_once(':').map(|(_, v)| v).unwrap_or("").to_string();
        } else if line.starts_with("FN") && line.contains(':') {
            name = line.split_once(':').map(|(_, v)| v).unwrap_or("").to_string();
        } else if line.starts_with("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:") {
            // Extract UID from urn:uuid:<uid>
            if let Some(uid) = line.strip_prefix("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:") {
                let uid = uid.trim().to_string();
                if !uid.is_empty() {
                    member_uids.push(uid);
                }
            }
        }
    }

    if !is_group || name.is_empty() {
        return None;
    }
    if id.is_empty() {
        id = Uuid::new_v4().to_string();
    }

    Some(ContactGroup { id, name, member_uids, href, etag })
}
```

### Pattern 4: serialize_group_vcard() (mirrors serialize_vcard())

```rust
/// Serialize a ContactGroup to a vCard 3.0 string with X-ADDRESSBOOKSERVER extensions.
/// Uses same fold_line() and CRLF conventions as serialize_vcard().
pub fn serialize_group_vcard(group: &ContactGroup) -> String {
    let mut lines: Vec<String> = Vec::new();
    lines.push("BEGIN:VCARD".to_string());
    lines.push("VERSION:3.0".to_string());
    lines.push(format!("UID:{}", group.id));
    lines.push(format!("FN:{}", escape_value(&group.name)));
    lines.push("X-ADDRESSBOOKSERVER-KIND:group".to_string());
    for uid in &group.member_uids {
        lines.push(format!("X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:{uid}"));
    }
    lines.push("END:VCARD".to_string());
    lines.iter().map(|l| fold_line(l)).collect::<String>()
}
```

### Pattern 5: CardDavClient Group Methods

All follow the exact shape of existing contact methods:

```rust
impl CardDavClient {
    /// List all groups across all address books (from REPORT responses, client-side filtered)
    pub async fn list_groups(&self) -> Result<Vec<ContactGroup>> { ... }

    /// Find a group by ID (UID) across all address books
    pub async fn get_group_by_id(&self, group_id: &str) -> Result<ContactGroup> { ... }

    /// Find a group by name — errors if ambiguous (multiple matches)
    pub async fn get_group_by_name(&self, name: &str) -> Result<ContactGroup> { ... }

    /// Create an empty group in the default address book
    pub async fn create_group(&self, addressbook_href: &str, group: &ContactGroup) -> Result<ContactCreateResult> { ... }

    /// Rename a group: fetch vCard, update FN:, PUT with If-Match
    pub async fn rename_group(&self, href: &str, etag: &str, new_name: &str) -> Result<String> { ... }

    /// Delete a group (members are NOT affected)
    pub async fn delete_group(&self, href: &str, etag: &str, group_id: &str) -> Result<()> { ... }
}
```

For `list_groups()` — the REPORT body is identical to `list_contacts()`. Parse all vCards, collect those where `parse_group_vcard()` returns `Some`, discard the rest.

For `create_group()` — identical to `create_contact()` but calls `serialize_group_vcard()` and uses `build_group_href()` (same convention: `{addressbook_href.trim_end_matches('/')}/{group.id}.vcf`).

For `rename_group()` — mirrors `update_contact()`: PUT with `If-Match: <etag>`, call `serialize_group_vcard()` with modified name. The full vCard must be serialized (not a patch) because CardDAV PUT replaces the entire resource.

For `delete_group()` — identical to `delete_contact()`. The `map_write_response()` helper accepts a `&Contact` but only uses `contact.id` for error messages — for groups, construct a minimal placeholder or adapt the helper to accept `(&str, Option<&str>)`. The simplest approach: pass a fake `Contact` with the group's id, or create a parallel `map_group_write_response()`.

### Pattern 6: CLI Subcommand Structure

```rust
// src/main.rs — add to ContactsCommands enum
#[derive(Subcommand)]
enum ContactsCommands {
    // ... existing variants ...

    /// Manage contact groups
    #[command(subcommand)]
    Groups(GroupsCommands),
}

#[derive(Subcommand)]
enum GroupsCommands {
    /// List all contact groups
    List,

    /// Create a new empty contact group
    Create {
        /// Group name
        name: String,
    },

    /// Get group details including resolved member contacts
    Get {
        /// Group ID (UID) or name
        id: String,
    },

    /// Rename a contact group
    Rename {
        /// Group ID (UID) or name
        id: String,
        /// New group name
        new_name: String,
    },

    /// Delete a contact group (members are NOT deleted)
    Delete {
        /// Group ID (UID) or name
        id: String,
        /// Confirm deletion
        #[arg(long)]
        confirm: bool,
        /// Alias for --confirm
        #[arg(short = 'y', long)]
        yes: bool,
    },
}
```

The `main.rs` match arm follows the exact `CalendarsCommands` pattern:

```rust
Commands::Contacts(cmd) => match cmd {
    // ... existing arms ...
    ContactsCommands::Groups(cmd) => match cmd {
        GroupsCommands::List => commands::list_groups().await,
        GroupsCommands::Create { name } => commands::create_group(&name).await,
        GroupsCommands::Get { id } => commands::get_group(&id).await,
        GroupsCommands::Rename { id, new_name } => commands::rename_group(&id, &new_name).await,
        GroupsCommands::Delete { id, confirm, yes } => {
            if !(confirm || yes) {
                Output::<()>::error("Confirmation required: pass --confirm to delete group").print();
                anyhow::bail!("confirmation required");
            }
            commands::delete_group(&id).await
        }
    },
},
```

### Pattern 7: MCP GraphQL Type

```rust
// src/mcp/graphql/types.rs — add alongside GqlContact
#[derive(SimpleObject)]
#[graphql(name = "ContactGroup")]
pub struct GqlContactGroup {
    pub id: String,
    pub name: String,
    pub member_count: i32,
    pub members: Vec<GqlContact>,  // resolved at query time
    pub href: Option<String>,
    pub etag: Option<String>,
}
```

Note: `members: Vec<GqlContact>` carries fully resolved contact data. The resolver fetches contacts for each member UID. This is consistent with the decision in CONTEXT.md (`members: [Contact!]!` resolved).

For the delete mutation — use the same HMAC confirmation token pattern as `delete_contact`:

```rust
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum GroupDeleteAction {
    Preview,
    Confirm,
}
```

### Pattern 8: Group Identifier Resolution (ID or Name)

For `get`, `rename`, and `delete` — the CLI accepts both group UID and group name. The lookup logic:

```rust
async fn resolve_group(client: &CardDavClient, id_or_name: &str) -> crate::error::Result<ContactGroup> {
    // Try as exact UID first
    match client.get_group_by_id(id_or_name).await {
        Ok(group) => return Ok(group),
        Err(crate::error::Error::GroupNotFound(_)) => {} // fall through to name lookup
        Err(e) => return Err(e),
    }
    // Try as name
    client.get_group_by_name(id_or_name).await
}
```

`get_group_by_name()` collects all groups where `name == id_or_name` (case-sensitive to match the server's FN property), then:
- 0 matches → `GroupNotFound`
- 1 match → `Ok(group)`
- 2+ matches → `GroupAmbiguous`

### Anti-Patterns to Avoid

- **Using `KIND:group` (vCard 4.0):** Fastmail requires `X-ADDRESSBOOKSERVER-KIND:group`. The 4.0 `KIND` property is silently ignored.
- **Server-side KIND filtering in REPORT:** Do not add `card:prop-filter` for KIND to the REPORT XML body. Fastmail's Cyrus IMAP may not support it. Use client-side filtering.
- **Trying to PATCH vCards:** CardDAV has no PATCH. PUT replaces the entire resource. `rename_group()` must fetch, modify, and PUT the complete vCard.
- **Forgetting `If-None-Match: *` on create:** Create must use `If-None-Match: *` to prevent clobbering an existing resource with the same UUID. This is already handled by `create_contact()` — replicate it exactly.
- **Forgetting `If-Match: <etag>` on rename/delete:** Without ETag-guarded writes, concurrent updates can corrupt data. The ETag must be fetched immediately before the write.
- **Resolving members in `list_groups()`:** Member resolution (fetching full Contact for each UID) is expensive (N+1 HTTP calls). List shows only member count. Resolution happens only in `get_group()`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UUID generation for new group IDs | Custom ID scheme | `Uuid::new_v4().to_string()` (already imported) | Consistency with contact IDs; guaranteed uniqueness |
| ETag conflict detection | Custom version tracking | Existing `map_write_response()` / `ContactConflict` error pattern | Already handles 412 Precondition Failed correctly |
| vCard line folding | Custom 75-char fold | Existing `fold_line()` function in `carddav/mod.rs` | Already handles UTF-8 char boundaries correctly |
| vCard value escaping | Custom escaping | Existing `escape_value()` / `unescape_value()` | Already handles `\\`, `\;`, `\,`, `\n` per RFC 2426 |
| HMAC confirmation tokens for delete | Custom token scheme | Existing `AppContext::confirmation_token()` pattern | Already implemented for contact/calendar/event delete |
| XML parsing of REPORT response | Custom XML parser | Existing `roxmltree` pattern in `parse_contacts_from_xml()` | Already handles namespaced CardDAV XML |

---

## Common Pitfalls

### Pitfall 1: Group vCards Leaking into Contact Listings

**What goes wrong:** A group vCard has `FN:My Group` but no email/phone/etc. If `parse_vcard()` does not filter it, the contact list shows "My Group" as a contact with empty email and phone fields. This is the explicit success criterion in the phase — "existing `contacts list` is unaffected by group vCards."

**Why it happens:** The REPORT response returns ALL vCards in the address book, including groups. There is no server-side filter for Fastmail's Cyrus IMAP.

**How to avoid:** In `parse_vcard()`, scan for the `X-ADDRESSBOOKSERVER-KIND:group` line during the initial line-by-line pass and return `None` immediately. The check must happen before the name-empty guard to avoid groups with names being returned.

**Warning signs:** `contacts list` shows contacts with empty email/phone but a group-like name.

### Pitfall 2: Wrong vCard Property Name for Groups

**What goes wrong:** Using `KIND:group` (vCard 4.0) instead of `X-ADDRESSBOOKSERVER-KIND:group` (vCard 3.0 extension). The server may accept the PUT (201 Created) but the group will not function as a group — it will appear as a regular contact.

**Why it happens:** vCard 4.0 documentation uses `KIND:group`. Fastmail runs vCard 3.0 with Apple-originated extensions.

**How to avoid:** Always use `X-ADDRESSBOOKSERVER-KIND:group` in `serialize_group_vcard()`. Test by listing groups after creation — a wrong-format group will not appear in the group list (it has no KIND line the parser recognizes).

### Pitfall 3: Member UID Format in X-ADDRESSBOOKSERVER-MEMBER

**What goes wrong:** Storing bare UIDs (e.g., `abc-123`) in the member list instead of the full URN format `urn:uuid:abc-123`. When fetching the vCard back, the member line reads `X-ADDRESSBOOKSERVER-MEMBER:abc-123` which is non-standard and may not be recognized by other clients.

**Why it happens:** UIDs are just UUIDs; the `urn:uuid:` prefix is the standard wrapper.

**How to avoid:** In `serialize_group_vcard()`, always emit `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:{uid}`. In `parse_group_vcard()`, strip the `urn:uuid:` prefix when extracting the stored UID.

### Pitfall 4: ETag Race on Rename

**What goes wrong:** `rename_group()` fetches the group to get the ETag, then waits before PUTting. If another client modified the group in between, the PUT returns 412 Precondition Failed.

**Why it happens:** CardDAV ETag-guarded writes are optimistic concurrency — they detect but do not prevent conflicts.

**How to avoid:** The existing `map_write_response()` already converts 412 to `ContactConflict`. For groups, create a parallel `GroupConflict` error. The caller receives a descriptive error and can retry. Phase 19 (membership operations) will add ETag-retry logic — for Phase 18, surface the error clearly.

### Pitfall 5: map_write_response() is Contact-Specific

**What goes wrong:** `map_write_response()` takes `&Contact` and uses `contact.id` and `contact.etag` in error messages. Passing a fake `Contact` for group deletes compiles but produces misleading error messages (e.g., "Contact conflict for group-uid").

**How to avoid:** Two options:
1. Create `map_group_write_response()` that takes `&ContactGroup` — straightforward parallel.
2. Refactor `map_write_response()` to take `id: &str, sent_etag: Option<&str>` — cleaner but larger diff.

Option 1 (parallel function) is preferred for Phase 18 to minimize diff scope.

### Pitfall 6: Nested Subcommand Registration in main.rs

**What goes wrong:** Adding `Groups(GroupsCommands)` to `ContactsCommands` without adding the corresponding match arm in `main.rs`. Rust will compile but produce a non-exhaustive match warning (or error if `#[deny(non_exhaustive_patterns)]`).

**How to avoid:** Add the match arm for `ContactsCommands::Groups(cmd)` in the same commit as the enum variant. The compiler enforces exhaustiveness.

---

## Code Examples

### Existing: How create_contact PUT Works (to replicate for groups)

```rust
// src/carddav/mod.rs — create_contact (existing)
pub async fn create_contact(&self, addressbook_href: &str, contact: &Contact) -> Result<ContactCreateResult> {
    let href = build_contact_href(addressbook_href, &contact.id);
    let url = format!("{}{}", self.base_url, href);
    let vcard = serialize_vcard(contact);

    let response = self
        .client
        .put(&url)
        .basic_auth(&self.username, Some(&self.app_password))
        .header("Content-Type", "text/vcard; charset=utf-8")
        .header(IF_NONE_MATCH, "*")   // <-- must not overwrite existing
        .body(vcard)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await?;
    let etag = map_write_response(contact, None, status, &headers, &body)?;
    let created_href = extract_location_path(&headers).unwrap_or(href);
    Ok(ContactCreateResult { href: created_href, etag })
}
```

`create_group()` replicates this exactly, substituting `serialize_group_vcard(group)` for `serialize_vcard(contact)`.

### Existing: How ETag-guarded rename/update Works

```rust
// src/carddav/mod.rs — update_contact (existing)
pub async fn update_contact(&self, href: &str, etag: &str, contact: &Contact) -> Result<String> {
    let url = format!("{}{}", self.base_url, href);
    let vcard = serialize_vcard(contact);

    let response = self.client.put(&url)
        .basic_auth(&self.username, Some(&self.app_password))
        .header("Content-Type", "text/vcard; charset=utf-8")
        .header(IF_MATCH, etag)   // <-- optimistic concurrency
        .body(vcard)
        .send()
        .await?;
    // ...
}
```

`rename_group()` follows this pattern: accept `href`, `etag`, `new_name`; construct updated `ContactGroup` in-memory with new name but same member_uids; call `serialize_group_vcard()`; PUT with `If-Match`.

### Existing: How GqlContact is structured (to replicate for GqlContactGroup)

```rust
// src/mcp/graphql/types.rs (existing)
#[derive(SimpleObject)]
#[graphql(name = "Contact")]
pub struct GqlContact {
    pub id: String,
    pub name: String,
    pub emails: Vec<GqlContactEmail>,
    // ...
}

impl From<Contact> for GqlContact { ... }
```

`GqlContactGroup` follows the same `SimpleObject` + `From<ContactGroup>` pattern.

### Existing: How ContactDeleteAction + confirmation token works in MCP

```rust
// src/mcp/graphql/mutation.rs (existing pattern for delete_contact)
async fn delete_contact(
    &self,
    ctx: &Context<'_>,
    action: ContactDeleteAction,
    id: String,
    confirmation_token: Option<String>,
) -> Result<GqlContactDeleteResult> {
    match action {
        ContactDeleteAction::Preview => {
            let token = ctx.data::<AppContext>()?.confirmation_token(&["delete_contact", &id]);
            Ok(GqlContactDeleteResult {
                success: true,
                preview: Some(format!("Will delete contact {id}")),
                confirmation_token: Some(token),
                ..Default::default()
            })
        }
        ContactDeleteAction::Confirm => {
            // Validate token, then delete
        }
    }
}
```

`delete_group` MCP mutation replicates this with `GroupDeleteAction` and `GqlGroupDeleteResult`.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| vCard 4.0 `KIND:group` | vCard 3.0 `X-ADDRESSBOOKSERVER-KIND:group` | Apple's CardDAV extension pre-dates RFC 6350 | Fastmail uses the 3.0 extension; 4.0 is not supported |
| Server-side `card:prop-filter` for KIND | Client-side KIND filtering | Cyrus IMAP has limited filter support | Client filters REPORT results; no server-side filtering |

---

## Open Questions

1. **`map_write_response()` coupling to `Contact`**
   - What we know: The function takes `&Contact` for error message construction; groups need equivalent behavior.
   - What's unclear: Whether to create a parallel `map_group_write_response()` or generalize `map_write_response()` to accept `id: &str`.
   - Recommendation: Create parallel `map_group_write_response(group: &ContactGroup, ...)` for Phase 18. Generalization is a separate refactor.

2. **Member resolution performance in `getGroup` MCP query**
   - What we know: `groups get` resolves member UIDs to full contacts. Each contact requires scanning all address book entries (current `get_contact_by_id()` does linear scan per address book). For a group with 50 members, this is 50 sequential scans.
   - What's unclear: Whether this is acceptable latency for Phase 18.
   - Recommendation: Use `list_contacts()` once to get all contacts, then filter in-memory by member UIDs. This is O(contacts) not O(members * contacts). Implement this optimization from the start.

3. **`list_groups_from_xml()` vs reusing `parse_contacts_from_xml()`**
   - What we know: The REPORT request body for groups is identical to contacts. The XML response structure is identical. Only the vCard parsing differs.
   - What's unclear: Whether to add a second pass to `parse_contacts_from_xml()` (returning both contacts and groups) or make two separate functions.
   - Recommendation: Two separate functions — `parse_contacts_from_xml()` (filters out groups, returns contacts) and `parse_groups_from_xml()` (filters in groups, returns groups). Avoids a complex return type and keeps each function's purpose clear.

---

## Environment Availability

Step 2.6: SKIPPED — this phase is purely code changes to an existing Rust project. No external services, databases, or CLI tools beyond the existing Rust toolchain are required. Fastmail's CardDAV server is the only external dependency and is already used in production.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[cfg(test)]` + `#[test]` / `#[tokio::test]` |
| Config file | None (uses Cargo's built-in test runner) |
| Quick run command | `cargo test -p fastmail-cli` |
| Full suite command | `cargo test -p fastmail-cli` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GRP-01 | `parse_group_vcard()` correctly extracts id, name, member_uids | unit | `cargo test -p fastmail-cli parse_group_vcard` | No — Wave 0 |
| GRP-01 | `parse_vcard()` returns None for group vCards | unit | `cargo test -p fastmail-cli parse_vcard_filters_group` | No — Wave 0 |
| GRP-01 | `list_contacts()` does not include group vCards | unit (mock XML) | `cargo test -p fastmail-cli list_contacts_excludes_groups` | No — Wave 0 |
| GRP-02 | `serialize_group_vcard()` emits correct properties | unit | `cargo test -p fastmail-cli serialize_group_vcard` | No — Wave 0 |
| GRP-02 | `build_contact` creates group href in correct format | unit | `cargo test -p fastmail-cli build_group_href` | No — Wave 0 |
| GRP-03 | `get_group_by_id()` finds group and resolves members | unit (mock) | `cargo test -p fastmail-cli get_group_by_id` | No — Wave 0 |
| GRP-04 | `rename_group()` updates FN line in serialized vCard | unit | `cargo test -p fastmail-cli rename_group_serializes_new_name` | No — Wave 0 |
| GRP-05 | `delete_group()` does not affect member contacts | manual-only (requires live server) | — | N/A |
| CLI-01 | CLI parses `contacts groups list/create/get/rename/delete` | unit | `cargo test -p fastmail-cli cli_groups_subcommands` | No — Wave 0 |
| CLI-03 | `contacts groups delete` rejects without `--confirm` | unit | `cargo test -p fastmail-cli cli_groups_delete_requires_confirm` | No — Wave 0 |
| MCP-01 | `GqlContactGroup` serializes correctly | unit | `cargo test -p fastmail-cli gql_contact_group_from` | No — Wave 0 |
| MCP-02 | GroupDeleteAction enum values present in schema | unit (SDL check) | `cargo test -p fastmail-cli build_schema_has_group_delete_action` | No — Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p fastmail-cli`
- **Per wave merge:** `cargo test -p fastmail-cli`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] Unit tests for `parse_group_vcard()` with valid group vCard
- [ ] Unit test for `parse_vcard()` returning None on group vCard
- [ ] Unit test for `serialize_group_vcard()` output structure
- [ ] Unit test for CLI parsing of `contacts groups` subcommands (using `Cli::try_parse_from`)
- [ ] Unit test for CLI `groups delete` requiring `--confirm`
- [ ] Unit test for `GqlContactGroup::from(ContactGroup)`

*(All unit tests go in the same file as their implementation, following the existing `#[cfg(test)]` pattern.)*

---

## Sources

### Primary (HIGH confidence)

- Codebase inspection: `src/carddav/mod.rs` — full `parse_vcard()`, `serialize_vcard()`, `create_contact()`, `update_contact()`, `delete_contact()`, `parse_contacts_from_xml()` implementations
- Codebase inspection: `src/mcp/graphql/types.rs` — `GqlContact`, `ContactDeleteAction`, `GqlContactDeleteResult` patterns
- Codebase inspection: `src/mcp/graphql/query.rs`, `mutation.rs` — resolver patterns for contacts
- Codebase inspection: `src/main.rs` — `ContactsCommands`, `CalendarsCommands` subcommand patterns
- Codebase inspection: `src/error.rs` — `ContactNotFound`, `ContactConflict` error patterns
- CONTEXT.md — locked decisions on vCard format, struct placement, CLI shape

### Secondary (MEDIUM confidence)

- Apple CardDAV Developer Documentation (via known industry documentation): `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` are the correct Fastmail-compatible group vCard properties (vCard 3.0 extension, not vCard 4.0 `KIND:group`)
- Cyrus IMAP CardDAV implementation notes: server-side `card:prop-filter KIND` filtering is unreliable; client-side filtering is the correct approach

### Tertiary (LOW confidence)

None.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all required libraries already in Cargo.toml
- Architecture: HIGH — every pattern has a direct existing analogue in the codebase (Contact → ContactGroup, CalendarsCommands → GroupsCommands, GqlContact → GqlContactGroup)
- CardDAV protocol: HIGH — locked in CONTEXT.md from prior research; vCard extension format well-documented
- Pitfalls: HIGH — derived directly from code reading and CONTEXT.md specifics

**Research date:** 2026-04-13
**Valid until:** 2026-06-13 (stable protocol; Fastmail CardDAV behavior is not changing)
