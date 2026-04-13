# Architecture Research

**Domain:** Contact group CRUD on CardDAV (v1.3 milestone)
**Researched:** 2026-04-13
**Confidence:** HIGH — Fastmail's group format confirmed via community sources; integration points derived from direct codebase inspection

## Standard Architecture

### System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        CLI Layer (main.rs)                       │
│  ContactsCommands   GroupsCommands (new)                        │
│  Create/Update/Delete/List  +  --group flag on Create           │
└────────────────────────┬────────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────────┐
│                    Commands Layer                                 │
│  src/commands/contacts.rs (existing — add --group wiring)       │
│  src/commands/groups.rs   (NEW)                                  │
└────────────────────────┬────────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────────┐
│                    CardDAV Client                                 │
│  src/carddav/mod.rs — CardDavClient                              │
│  Existing: create_contact, update_contact, delete_contact        │
│  NEW:  create_group, get_group_by_id, list_groups,               │
│        update_group, delete_group,                               │
│        add_member, remove_member  (mutate group vCard in-place)  │
└────────────────────────┬────────────────────────────────────────┘
                         │ HTTP (PUT / DELETE / PROPFIND / REPORT)
┌────────────────────────▼────────────────────────────────────────┐
│             Fastmail CardDAV Server                              │
│  /dav/addressbooks/user/<user>/Default/                          │
│  Groups are regular .vcf resources — same namespace as contacts │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                    MCP / GraphQL Layer                           │
│  src/mcp/graphql/types.rs    — GqlContactGroup (NEW)             │
│  src/mcp/graphql/query.rs    — contactGroup, listContactGroups   │
│  src/mcp/graphql/mutation.rs — createContactGroup, updateContactGroup,   │
│                                deleteContactGroup,               │
│                                addContactGroupMember,            │
│                                removeContactGroupMember          │
└─────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Status |
|-----------|----------------|--------|
| `src/carddav/mod.rs` — `ContactGroup` struct | Data model: group id, name, member UIDs, href, etag | NEW struct, same file |
| `src/carddav/mod.rs` — `CardDavClient` methods | Protocol: serialize/parse group vCards, HTTP write ops | NEW methods, existing client |
| `src/commands/groups.rs` | CLI business logic for group CRUD and membership ops | NEW file |
| `src/commands/contacts.rs` | `--group` flag routing: create contact then add to group | MODIFIED — one new optional arg |
| `src/main.rs` — `GroupsCommands` enum | Clap command definitions for groups subcommand tree | NEW enum + dispatch arm |
| `src/mcp/graphql/types.rs` | `GqlContactGroup` + `GqlGroupMutationResult` output/result types | NEW types |
| `src/mcp/graphql/query.rs` | `contactGroup(id)` and `listContactGroups` resolvers | NEW resolver methods |
| `src/mcp/graphql/mutation.rs` | Group CRUD and membership mutation resolvers | NEW mutation methods |
| `src/error.rs` | `GroupNotFound(String)` and `GroupConflict{...}` variants | NEW variants |

## Data Model

### ContactGroup (new struct in src/carddav/mod.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContactGroup {
    /// UID of the group vCard (our stable identifier)
    pub id: String,
    /// Display name (FN property)
    pub name: String,
    /// UIDs of member contacts (from X-ADDRESSBOOKSERVER-MEMBER: urn:uuid:<uid>)
    pub member_ids: Vec<String>,
    /// Server resource URL — required for PUT/DELETE
    pub href: Option<String>,
    /// HTTP ETag — required for If-Match header
    pub etag: Option<String>,
}
```

Groups are deliberately a **separate type from Contact**. A group vCard has `X-ADDRESSBOOKSERVER-KIND:group` and carries no email/phone/address data. The only shared protocol surface is: same address book namespace, same PUT/DELETE mechanics, same ETag concurrency model.

### vCard serialization for groups

```
BEGIN:VCARD
VERSION:3.0
UID:<uuid>
FN:<group name>
X-ADDRESSBOOKSERVER-KIND:group
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<member-uid-1>
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<member-uid-2>
END:VCARD
```

Fastmail confirmed this format is the one it stores and syncs (Apple/iCloud compatible). Members are referenced by UID only — no href coupling. This means adding/removing a member requires fetching the group's current vCard, modifying `X-ADDRESSBOOKSERVER-MEMBER` lines, and re-PUTting with `If-Match`.

## Project Structure (changes only)

```
src/
├── carddav/
│   └── mod.rs             # Add: ContactGroup struct, serialize_group_vcard(),
│                          #      parse_group_from_vcard(), CardDavClient::create_group(),
│                          #      list_groups(), get_group_by_id(), update_group(),
│                          #      delete_group(), add_member(), remove_member()
│                          # Modify: parse_contacts_from_xml() to skip KIND:group vCards
│
├── commands/
│   ├── contacts.rs        # Modify: create_contact_record() accepts optional group_id,
│   │                      #         calls add_member after contact creation
│   └── groups.rs          # New: create_group_record(), list_groups_command(),
│                          #      get_group_command(), update_group_record(),
│                          #      delete_group_record(), add_member_record(),
│                          #      remove_member_record()
│
├── mcp/graphql/
│   ├── types.rs           # Add: GqlContactGroup, GqlGroupMutationResult,
│   │                      #      GqlGroupDeleteResult, GroupDeleteAction enum
│   ├── query.rs           # Add: contact_group(), list_contact_groups()
│   └── mutation.rs        # Add: create_contact_group(), update_contact_group(),
│                          #      delete_contact_group(), add_contact_group_member(),
│                          #      remove_contact_group_member()
│
├── error.rs               # Add: GroupNotFound(String), GroupConflict{id, sent_etag, server_etag}
└── main.rs                # Add: GroupsCommands enum, Contacts::Create --group flag,
                           #      dispatch arms for Groups subcommands
```

## Architectural Patterns

### Pattern 1: Group vCard as Sibling Resource

**What:** Groups live in the same address book directory as contacts. They are ordinary `.vcf` resources identified by `X-ADDRESSBOOKSERVER-KIND:group`. No separate collection, no special CardDAV property — just a vCard that happens to have a `KIND` extension property.

**When to use:** Always — this is Fastmail's implementation.

**Trade-offs:** The `list_contacts()` method currently returns ALL vCards via REPORT. Group vCards will appear in the XML response. The parser must skip them (detect `KIND:group` and exclude from the contacts list) and `list_groups()` must do the inverse. This is a single-pass filter in the existing `parse_vcard` function.

**Example parser guard (in parse_vcard):**

```rust
// Early-exit if this is a group vCard — groups are handled separately
if unfolded.lines().any(|l| {
    l.starts_with("X-ADDRESSBOOKSERVER-KIND") && l.contains("group")
}) {
    return None;
}
```

And `parse_group_from_vcard()` returns `None` when `KIND:group` is absent.

### Pattern 2: Membership via In-Place vCard Mutation

**What:** There is no dedicated CardDAV operation to add/remove a single member. The full group vCard must be re-serialized with the updated `X-ADDRESSBOOKSERVER-MEMBER` list and PUT back with `If-Match`.

**When to use:** `add_member` and `remove_member` operations in `CardDavClient`.

**Trade-offs:** Requires a read-before-write (REPORT the group, mutate the member list, PUT). Adds one round-trip per membership change. This is the same read-modify-write pattern used by `update_contact`.

**Sketch:**

```rust
pub async fn add_member(&self, group_id: &str, member_uid: &str) -> Result<ContactGroup> {
    let mut group = self.get_group_by_id(group_id).await?;
    if !group.member_ids.contains(&member_uid.to_string()) {
        group.member_ids.push(member_uid.to_string());
    }
    let href = group.href.clone().ok_or_else(|| Error::GroupNotFound(group_id.to_string()))?;
    let etag = group.etag.clone().ok_or_else(|| Error::GroupConflict { ... })?;
    let new_etag = self.update_group(&href, &etag, &group).await?;
    group.etag = Some(new_etag);
    Ok(group)
}
```

### Pattern 3: Mirror the Contact CRUD Pattern Exactly

**What:** Group operations follow the same three-layer structure contact operations use:
1. `CardDavClient` method (HTTP + parse)
2. `commands/groups.rs` record function (wires client + config)
3. CLI handler in `main.rs` (clap args to record function)
4. GraphQL resolver in `mutation.rs` (calls record function)

**When to use:** All group operations.

**Trade-offs:** More files touched, but maximally consistent with the existing codebase. All existing patterns (ETag guards, `Output::success().print()`, `GqlXxxMutationResult`, `ContactDeleteAction` enum for preview/confirm) carry over directly.

## Data Flow

### Create Group

```
CLI: fastmail-cli groups create --name "Team"
     |
main.rs GroupsCommands::Create { name } dispatch
     |
commands::groups::create_group_record(name)
     -> carddav::CardDavClient::create_group(&addressbook_href, &group)
         -> PUT /dav/addressbooks/user/<user>/Default/<uuid>.vcf
            Body: serialized group vCard (KIND:group, no members yet)
            Headers: If-None-Match: *
         -> 201 Created + ETag
     -> ContactGroup { id, name, member_ids: [], href, etag }
     |
Output::success(group).print()
```

### Add Member

```
CLI: fastmail-cli groups add-member <group-id> <contact-id>
     |
commands::groups::add_member_record(group_id, contact_id)
     -> CardDavClient::add_member(group_id, contact_id)
         1. get_group_by_id(group_id)   -- REPORT across address books
         2. push contact_id into member_ids
         3. update_group(&href, &etag, &group)  -- PUT with If-Match
     -> ContactGroup with updated member_ids + new etag
     |
Output::success(group).print()
```

### Create Contact with --group flag

```
CLI: fastmail-cli contacts create --name "Alice" --group <group-id>
     |
commands::contacts::create_contact_record(input, group_id: Option<String>)
     1. Build contact, PUT to CardDAV -> Contact with href/etag
     2. If group_id is Some:
        CardDavClient::add_member(group_id, &contact.id)
     -> Contact
     |
Output::success(contact).print()   // group membership is a side-effect, not returned separately
```

### List Groups (distinguishing groups from contacts)

```
list_groups(&addressbook_href)
    -> REPORT (same XML body as list_contacts)
    -> for each response: extract vCard data
        -> parse_group_from_vcard(): accepts only if KIND:group present
        -> skip if no KIND:group
    -> Vec<ContactGroup>
```

The existing `list_contacts` call remains unchanged — `parse_vcard()` returns `None` when `KIND:group` is detected, so group vCards are silently filtered out. This is backward-compatible: no existing tests break.

## Integration Points

### CardDAV Client Modifications

| Method | Status | Notes |
|--------|--------|-------|
| `list_contacts()` | UNCHANGED | delegates to parse_contacts_from_xml |
| `parse_contacts_from_xml()` | MODIFIED | skips KIND:group vCards in parse_vcard() call |
| `default_addressbook_href()` | UNCHANGED | reused by group create |
| `create_group()` | NEW | PUT with If-None-Match: * |
| `list_groups()` | NEW | REPORT + filter to KIND:group only |
| `get_group_by_id()` | NEW | REPORT across books, find by UID |
| `update_group()` | NEW | PUT with If-Match (replaces full vCard) |
| `delete_group()` | NEW | DELETE with If-Match |
| `add_member()` | NEW | read-modify-write on group vCard |
| `remove_member()` | NEW | read-modify-write on group vCard |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `commands/contacts.rs` <-> `carddav::CardDavClient` | Direct async call | `create_contact_record` signature gains `Option<String>` group_id; calls `add_member` as post-create side effect |
| `commands/groups.rs` <-> `carddav::CardDavClient` | Direct async call | mirrors existing contacts pattern |
| `mcp/graphql/mutation.rs` <-> `commands::groups` | Calls `*_record` functions | same pattern as contact mutations calling `commands::*_contact_record` |
| `mcp/graphql/query.rs` <-> `carddav::CardDavClient` | Via AppContext | groups queries use CardDavClient from context, same as contacts |

### Error Variants to Add (src/error.rs)

```rust
#[error("Contact group not found: {0}")]
GroupNotFound(String),

#[error("Contact group conflict for '{id}': sent ETag '{sent_etag}', server has '{server_etag:?}'")]
GroupConflict {
    id: String,
    sent_etag: String,
    server_etag: Option<String>,
},
```

These mirror the existing `ContactNotFound` and `ContactConflict` variants exactly.

## Build Order (dependency-first)

Build in this order to enable incremental testing at each step:

**Step 1 — Data model and serialization (src/carddav/mod.rs)**
- Add `ContactGroup` struct
- Add `serialize_group_vcard()` function
- Add `parse_group_from_vcard()` function
- Modify `parse_vcard()` to return `None` on KIND:group (backward-compatible filter)
- Unit tests for serialize/parse round-trip

**Step 2 — Error variants (src/error.rs)**
- Add `GroupNotFound` and `GroupConflict`
- Zero risk, no dependencies

**Step 3 — CardDavClient group methods (src/carddav/mod.rs)**
- Add `create_group()`, `list_groups()`, `get_group_by_id()`, `update_group()`, `delete_group()`
- Add `add_member()`, `remove_member()`
- Wiremock integration tests for each (following existing contact test patterns)

**Step 4 — CLI command handlers (src/commands/groups.rs)**
- Add `create_group_record()`, `list_groups_command()`, `update_group_record()`, `delete_group_record()`, `add_member_record()`, `remove_member_record()`
- Unit tests for input/output wiring

**Step 5 — CLI argument definitions (src/main.rs)**
- Add `GroupsCommands` enum with: List, Get, Create, Update, Delete, AddMember, RemoveMember
- Add `--group` option to `ContactsCommands::Create`
- Wire dispatch arms
- Modify `commands::create_contact_record` signature to accept `Option<String>` group_id

**Step 6 — MCP GraphQL types (src/mcp/graphql/types.rs)**
- Add `GqlContactGroup`, `GqlGroupMutationResult`, `GqlGroupDeleteResult`, `GroupDeleteAction`

**Step 7 — MCP GraphQL query resolvers (src/mcp/graphql/query.rs)**
- Add `contact_group(id)`, `list_contact_groups()`

**Step 8 — MCP GraphQL mutation resolvers (src/mcp/graphql/mutation.rs)**
- Add `create_contact_group()`, `update_contact_group()`, `delete_contact_group()`
- Add `add_contact_group_member()`, `remove_contact_group_member()`

Steps 1-3 are the core protocol work and must precede everything else. Steps 4-5 (CLI) and 6-8 (MCP) can be developed in parallel once step 3 is done, but are sequenced here for coherent review.

## Anti-Patterns

### Anti-Pattern 1: Separate Namespace or URL Prefix for Groups

**What people do:** Attempt to store group vCards in a dedicated sub-collection or with a path prefix like `/groups/`.

**Why it's wrong:** Fastmail stores groups as plain `.vcf` resources in the same address book collection as contacts. There is no special namespace. Using one produces 404s or permission errors.

**Do this instead:** Use `default_addressbook_href()` and the same `build_contact_href()` pattern — groups get a UUID-named `.vcf` file like `<group-uuid>.vcf` in the same collection.

### Anti-Pattern 2: Storing Member hrefs Instead of UIDs

**What people do:** Store the full `/dav/addressbooks/.../<uid>.vcf` path inside `X-ADDRESSBOOKSERVER-MEMBER`.

**Why it's wrong:** The spec and Fastmail both use `urn:uuid:<uid>` — the UUID value from the contact's `UID` property, not the resource URL. Storing hrefs breaks compatibility with iOS and any client that resolves membership by UID.

**Do this instead:** `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<contact.id>`

### Anti-Pattern 3: Modifying Contact vCards to Record Group Membership

**What people do:** Add a `CATEGORIES` property or back-reference to the contact vCard when adding it to a group.

**Why it's wrong:** Fastmail's group model is group-owns-membership. Only the group vCard changes when a member is added or removed. Writing to the contact vCard for membership purposes creates drift and unnecessary ETag churn.

**Do this instead:** Only mutate the group vCard. `add_member()` and `remove_member()` touch only the group resource.

### Anti-Pattern 4: Letting Groups Leak into Contact Listings

**What people do:** Add the KIND:group filter at the command level rather than in the parser, and forget edge cases.

**Why it's wrong:** Groups are mixed into the REPORT response. Without a parser-level filter, `list_contacts()` starts returning group vCards as malformed `Contact` structs (empty emails, organization-less, name is the group name).

**Do this instead:** Filter in `parse_vcard()` — return `None` early on `X-ADDRESSBOOKSERVER-KIND:group`. The existing push-to-contacts code skips the entry with zero other changes. This is the single canonical place to enforce the type separation.

## Sources

- Fastmail confirmed group vCard format (Apple/iCloud compatible): https://groups.google.com/g/jmap-discuss/c/Rr--mPbdy5M
- rcmcarddav GROUPS.md (vCard-type group documentation): https://github.com/mstilkerich/rcmcarddav/blob/master/doc/GROUPS.md
- RFC 6350 vCard Format Specification: https://datatracker.ietf.org/doc/html/rfc6350
- RFC 6352 CardDAV: https://www.rfc-editor.org/rfc/rfc6352
- Fastmail CardDAV group format confirmed via community source (Bitfire forums, Fastmail developer response)

---
*Architecture research for: Contact Groups (v1.3 milestone)*
*Researched: 2026-04-13*
