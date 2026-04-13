# Stack Research

**Domain:** Rust CLI/MCP — CardDAV contact group CRUD via vCard 3.0 X-ADDRESSBOOKSERVER extensions
**Researched:** 2026-04-13
**Confidence:** HIGH (protocol claims verified against RFC 6350, rcmcarddav GROUPS.md, and Fastmail behavior reports; no new Cargo dependencies required)

---

## Context: This is an Extension Milestone, Not a New Stack

The entire production stack (tokio 1.49, reqwest 0.13.1, roxmltree 0.21.1, uuid 1, async-graphql 7, clap 4.5, serde 1.0, thiserror 2.0, wiremock 0.6 dev dep) is **validated, compiling, and passing 181 tests**. **No new Cargo.toml entries are required** for v1.3. All capability needed for contact groups already ships in the existing dependency set.

---

## Protocol Decision: Which vCard Group Format?

Fastmail uses **vCard 3.0 with Apple X-ADDRESSBOOKSERVER extensions** — not standard vCard 4.0 KIND/MEMBER.

### Why X-ADDRESSBOOKSERVER, not KIND:group

- Fastmail serializes all contact data as vCard 3.0 (confirmed: Fastmail docs, DAVx5 tested-with page)
- vCard 4.0 `KIND:group` + `MEMBER:urn:uuid:…` only applies when the server and all round-trip clients support vCard 4.0
- Fastmail's CardDAV server speaks vCard 3.0; the existing `serialize_vcard()` outputs `VERSION:3.0`
- The Apple extensions (`X-ADDRESSBOOKSERVER-KIND`, `X-ADDRESSBOOKSERVER-MEMBER`) are the de-facto standard for vCard 3.0 group representation — used by Apple macOS/iOS, iCloud, and Fastmail
- rcmcarddav (reference CardDAV implementation) documents: "vCard v4 specifies precisely this approach as the way to implement contact groups, except for using different property names" — meaning X-ADDRESSBOOKSERVER-KIND maps 1:1 to KIND, and X-ADDRESSBOOKSERVER-MEMBER maps 1:1 to MEMBER

### Group vCard Wire Format (confirmed)

```
BEGIN:VCARD
VERSION:3.0
UID:696cb4ce-792b-4b7b-833d-29727a33e9c9
FN:My Group Name
N:My Group Name;;;;
X-ADDRESSBOOKSERVER-KIND:group
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:613f2ccc-600a-47ee-84cb-9b30717c9f13
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:8ef07e3b-9dc1-4fef-862a-ee27af4296be
END:VCARD
```

Key points:
- `X-ADDRESSBOOKSERVER-KIND:group` is a single property identifying this vCard as a group, not a contact
- `X-ADDRESSBOOKSERVER-MEMBER` is **repeated** — one line per member — using `urn:uuid:<contact-uid>` URN format
- The contact UID in the URN matches the `UID` property of the member contact's vCard
- FN is required (group display name); N is present but semantically empty for groups
- Each group is a **separate `.vcf` resource** PUT into the same address book collection as individual contacts — no special collection or WebDAV path needed

---

## New Stack Additions

**None.** Every capability needed is already available in the existing dependency set.

### Existing Capabilities That Cover Groups

| Capability Needed | Existing Dep | How It Covers Groups |
|-------------------|-------------|----------------------|
| PUT group vCard to CardDAV | `reqwest 0.13.1` | Same PUT pattern as `create_contact` / `update_contact` |
| DELETE group vCard | `reqwest 0.13.1` | Same DELETE pattern as `delete_contact` |
| Parse group vCard from REPORT response | `roxmltree 0.21.1` + existing `parse_vcard()` | Extend parse_vcard to detect `X-ADDRESSBOOKSERVER-KIND:group` and extract `X-ADDRESSBOOKSERVER-MEMBER` lines |
| Serialize group vCard | existing `serialize_vcard()` pattern | Write a `serialize_group_vcard()` sibling that emits the X-ADDRESSBOOKSERVER properties |
| List groups (REPORT query) | `reqwest 0.13.1` | Same `addressbook-query` REPORT as `list_contacts()` |
| Generate group UID | `uuid 1` with `v4` feature | Already used for contacts; same `Uuid::new_v4()` call |
| ETag-guarded writes | existing `map_write_response()` + `IF_MATCH` / `IF_NONE_MATCH` | Identical concurrency pattern; groups are just resources |
| Error variants for group not found / conflict | `thiserror 2.0` in `src/error.rs` | Add `GroupNotFound` and `GroupConflict` variants — same pattern as ContactNotFound/ContactConflict |
| CLI group subcommands | `clap 4.5` derive | Add `GroupCommands` enum under `contacts groups` — same pattern as `CalendarCommands` |
| MCP GraphQL mutations/queries for groups | `async-graphql 7` | Add `GqlContactGroup`, `GqlContactGroupMutationResult` types; wire into `MutationRoot` / `QueryRoot` |

---

## Implementation Integration Points

### 1. New `ContactGroup` Struct in `src/carddav/mod.rs`

Mirrors the existing `Contact` struct shape. Carries `href` and `etag` for write operations. `members` is a `Vec<String>` of bare UUIDs (without the `urn:uuid:` prefix — strip on parse, re-add on serialize).

```rust
pub struct ContactGroup {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,   // bare UUIDs of member contacts
    pub href: Option<String>,
    pub etag: Option<String>,
}
```

### 2. Group vCard Parser Extension

`parse_vcard()` currently returns `Option<Contact>`. Add a parallel `parse_group_vcard()` that:
- Detects `X-ADDRESSBOOKSERVER-KIND:group` line
- Collects all `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` lines, stripping the `urn:uuid:` prefix
- Returns `Option<ContactGroup>`

The existing multistatus XML walk in `parse_contacts_from_xml()` can be extended or a parallel `parse_groups_from_xml()` written — same roxmltree traversal.

**Filter concern:** `list_contacts()` must exclude group vCards. The existing parser already returns `None` for vCards without FN — but a group vCard does have FN. Add an explicit KIND filter: skip any vCard with `X-ADDRESSBOOKSERVER-KIND:group` in `parse_vcard()` to prevent groups from appearing in contact lists.

### 3. Group vCard Serializer

Write `serialize_group_vcard(group: &ContactGroup) -> String` alongside `serialize_vcard()`. Uses same `fold_line()`, same CRLF discipline. Emits `X-ADDRESSBOOKSERVER-KIND:group` once and one `X-ADDRESSBOOKSERVER-MEMBER` line per member UUID, with `urn:uuid:` prefix added.

### 4. CardDavClient Methods

Add to `CardDavClient` in `src/carddav/mod.rs`:
- `list_groups(addressbook_href: &str) -> Result<Vec<ContactGroup>>`
- `get_group_by_id(group_id: &str) -> Result<ContactGroup>`
- `create_group(addressbook_href: &str, group: &ContactGroup) -> Result<ContactCreateResult>`
- `update_group(href: &str, etag: &str, group: &ContactGroup) -> Result<String>`
- `delete_group(href: &str, etag: &str, group_id: &str) -> Result<()>`

`create_group` / `update_group` / `delete_group` reuse `map_write_response()` directly — same HTTP semantics, same ETag guard, same error mapping. The only difference is `serialize_group_vcard()` instead of `serialize_vcard()`.

`Content-Type` header stays `text/vcard; charset=utf-8` — groups are still vCard resources.

The `build_contact_href()` helper can be reused as-is for groups — the `.vcf` extension and URL pattern are identical.

### 5. Error Variants (no new dep)

Add to `src/error.rs`:
```rust
#[error("Contact group not found: {0}")]
GroupNotFound(String),

#[error("Contact group conflict for '{id}': sent ETag '{sent_etag}', server has '{server_etag:?}'")]
GroupConflict {
    id: String,
    sent_etag: String,
    server_etag: Option<String>,
}
```

### 6. CLI Commands (`src/commands/contacts.rs`)

Add a `GroupCommands` enum under `contacts groups <subcommand>`:
- `create --name <name> [--member <uid>]...`
- `list`
- `get <id>`
- `update <id> [--name <name>] [--add-member <uid>]... [--remove-member <uid>]... [--clear-members]`
- `delete <id> --confirm`

The `--group <id>` flag on `contacts create` calls `create_contact_record()` as before then calls `add_member_to_group()` (which is just `update_group` with the new contact UID appended to `members`).

### 7. MCP/GraphQL Surface (`src/mcp/graphql/`)

Add to `types.rs`:
- `GqlContactGroup` (SimpleObject) — mirrors `ContactGroup` with `members: Vec<String>`
- `GqlContactGroupMutationResult` (SimpleObject) — mirrors `GqlContactMutationResult`

Add to `query.rs`: `list_groups`, `get_group` resolvers.
Add to `mutation.rs`: `create_group`, `update_group`, `delete_group`, `add_group_member`, `remove_group_member` resolvers.

---

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| Any vCard 4.0 `KIND:group` / `MEMBER:` serialization | Fastmail's server is vCard 3.0; vCard 4.0 properties will be stored as opaque X- extensions at best, silently ignored at worst | `X-ADDRESSBOOKSERVER-KIND:group` + `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:…` |
| A dedicated vCard parsing crate (e.g., `ical`, `vcard4`) | Adds a dep for something already implemented in 150 lines of existing, tested Rust; external crates vary in vCard 3.0 support | Extend existing `parse_vcard()` / `serialize_vcard()` in `src/carddav/mod.rs` |
| A separate CardDAV collection for groups | Groups are plain `.vcf` resources in the same address book; no MKCOL or special path logic is needed | PUT group vCards into `default_addressbook_href()` like contacts |
| A `CATEGORIES` field approach | Fastmail ignores CATEGORIES; group membership would not survive a round-trip through the Fastmail web UI | Apple X-ADDRESSBOOKSERVER approach |
| Fetching group members transitively on list | Every `list_groups` call would trigger N additional lookups; members are identifiers, not embedded objects | Return member UUIDs in `ContactGroup.members`; caller resolves by ID if needed |

---

## Cargo.toml Changes

**None required.** No additions, no removals, no feature flag changes.

---

## Version Compatibility

All existing deps are compatible with the group implementation — no new interactions to validate. The only constraint is that `roxmltree 0.21.1` continues to parse multi-value `X-ADDRESSBOOKSERVER-MEMBER` lines correctly; this is guaranteed because they are plain text node children of `<card:address-data>`, identical to any other vCard property line.

---

## Sources

- [RFC 6350 — vCard Format Specification](https://www.rfc-editor.org/rfc/rfc6350.html) — KIND and MEMBER properties (vCard 4.0); confirms X-ADDRESSBOOKSERVER-* are vCard 3.0 equivalents
- [rcmcarddav GROUPS.md](https://github.com/mstilkerich/rcmcarddav/blob/master/doc/GROUPS.md) — X-ADDRESSBOOKSERVER-KIND/MEMBER format, vCard 3.0 vs 4.0 mapping, confirmed HIGH confidence
- [DAVx5 tested-with/fastmail](https://www.davx5.com/tested-with/fastmail) — confirms Fastmail uses "Groups are separate vCards" (not CATEGORIES), MEDIUM confidence
- [Nextcloud issue #9369](https://github.com/nextcloud/server/issues/9369) — confirms `urn:uuid:` URI format in X-ADDRESSBOOKSERVER-MEMBER, vCard3 repeating property pattern, MEDIUM confidence
- WebSearch: Fastmail vCard 3.0 format and X-ADDRESSBOOKSERVER-KIND:group usage — corroborates Apple-style group format, MEDIUM confidence (multiple independent sources agree)

---
*Stack research for: fastmail-cli v1.3 Contact Groups*
*Researched: 2026-04-13*
