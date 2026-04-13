# Feature Research

**Domain:** Contact group CRUD and membership management via CardDAV (Fastmail-specific)
**Researched:** 2026-04-13
**Confidence:** HIGH (protocol behavior), MEDIUM (Fastmail-specific edge cases)

## Protocol Foundation

Fastmail uses the **vCard-type group** format — groups are separate vCard 3.0 objects stored alongside individual contact vCards in the same address book. This is the Apple/iCloud-originated format:

```
BEGIN:VCARD
VERSION:3.0
UID:986D6D27-B37D-425F-8A7C-DE10E04155C2
N:FamilyGroup;;;;
FN:FamilyGroup
X-ADDRESSBOOKSERVER-KIND:group
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:52A079D5-5884-4AF9-9367-6BF82251838B
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:d4c1baf6-f603-4fb5-8f19-d45eb1e7fb23
REV:20240101T000000Z
END:VCARD
```

Key facts confirmed:
- DAVx5 explicitly documents Fastmail as "Contact group method: Groups are separate vCards" (HIGH confidence)
- vCard 3.0 has no native group spec; `X-ADDRESSBOOKSERVER-KIND:group` is the de facto standard for vCard 3.0 servers
- `X-ADDRESSBOOKSERVER-MEMBER` values are `urn:uuid:<contact-uid>` URIs
- Group vCards are stored at the same CardDAV endpoint (same address book) as contact vCards
- The current `parse_vcard()` skips vCards without `FN:` — group vCards have `FN:` (the group name), so they will be returned by existing `list_contacts` calls but treated as contacts. This is the **primary structural dependency** for this milestone.

The alternative **CATEGORIES-type groups** (used by Nextcloud, Android) does NOT apply to Fastmail.

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist based on standard contact management behavior. Missing these makes group management feel broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| List groups | Users must discover existing groups before operating on them | LOW | Requires filtering `list_contacts` results by `X-ADDRESSBOOKSERVER-KIND:group`; existing REPORT query already returns group vCards |
| Create group | Core CRUD — no way to assign contacts to groups without creating one first | LOW | PUT a new vCard with `X-ADDRESSBOOKSERVER-KIND:group` and no members; same HTTP path as contact create |
| Get group (show members) | Users need to inspect group membership | LOW | Read group vCard, extract `X-ADDRESSBOOKSERVER-MEMBER` UIDs, resolve UIDs to Contact structs |
| Rename group | Standard update operation — users fix typos, reorganize | LOW | PUT updated group vCard with changed `FN:`/`N:` fields via `update_contact` path; requires ETag |
| Delete group | Standard CRUD completion | LOW | DELETE the group vCard only — does NOT delete member contacts (vCard-type groups: only the group object is deleted, members are untouched) |
| Add member to group | Core membership management | LOW | Fetch group vCard, append `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>`, PUT back with ETag |
| Remove member from group | Core membership management | LOW | Fetch group vCard, remove matching `X-ADDRESSBOOKSERVER-MEMBER` line, PUT back with ETag |
| `--group` flag on `contacts create` | Natural workflow: assign contact to group at creation time | MEDIUM | Requires two PUT operations: create contact, then fetch+update group vCard to add member |

### Differentiators (Competitive Advantage)

Features that go beyond baseline functionality for this CLI/MCP use case.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Resolve member UIDs to Contact structs in group get | AI agents and scripts need structured data, not raw UID strings | MEDIUM | Requires fetching all contacts, matching by UID; adds one extra REPORT round-trip |
| MCP GraphQL mutations for all group operations | Enables AI-agent group management without CLI | MEDIUM | Follows existing `createContact`/`updateContact` mutation pattern in `mutation.rs` |
| `--group` on `contacts create` (atomic UX) | Single command for a common workflow instead of two steps | MEDIUM | Must handle partial failure: contact created but group update fails |
| Empty group support | Can create a group placeholder before assigning contacts | LOW | vCard-type groups natively support empty groups (unlike CATEGORIES-type) — just omit MEMBER lines |
| Group membership validation (warn on unknown UID) | Prevents silent dangling references | LOW | Cross-reference provided contact ID against known contacts before adding to group |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Delete group AND delete all member contacts | "Clean up everything at once" | Contacts may belong to multiple groups; bulk delete is destructive and hard to undo | Delete group only; provide separate `contacts delete` calls for members if desired |
| CATEGORIES-type group support | Some clients use it | Fastmail does not use CATEGORIES for groups; implementing it adds code complexity with no benefit for this server | Stick to `X-ADDRESSBOOKSERVER-KIND:group` only |
| Automatically sync/mirror groups across address books | Seems useful for multi-book setups | Fastmail's typical setup is single address book; cross-book group membership is not a CardDAV primitive | Scope to single default address book |
| Group ACL / sharing management | Fastmail supports shared contacts | Out of scope per PROJECT.md; separate protocol surface | Explicit out-of-scope in roadmap |
| vCard 4.0 `KIND:group` / `MEMBER:` format | The "standard" group format | Fastmail uses vCard 3.0 with X-ADDRESSBOOKSERVER extensions; using vCard 4.0 properties would break compatibility | Use `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:` |

---

## Feature Dependencies

```
[List groups]
    required-by --> [Get group members]
    required-by --> [Add member to group]
    required-by --> [Remove member from group]
    required-by --> [Rename group]
    required-by --> [Delete group]

[Create group]
    required-by --> [--group on contacts create]

[contacts create (existing)]
    required-by --> [--group on contacts create]

[Get group members]
    enhances --> [MCP group query]

[Existing list_contacts / parse_vcard]
    must-update-for --> [List groups]
    must-update-for --> [Get group members]
    (currently parses all vCards as contacts; groups need to be filtered/identified)
```

### Dependency Notes

- **List groups requires parse_vcard update:** The existing `parse_vcard()` function skips vCards with no `FN:`, but group vCards have `FN:`. They will currently be returned mixed into the contacts list since the parser has no concept of `X-ADDRESSBOOKSERVER-KIND`. The parser needs a `kind` field or a separate code path to distinguish group vCards from contact vCards.
- **Add/remove member requires ETag-guarded fetch-then-write:** Group membership updates are not atomic on the server. The pattern is fetch (get current ETag + member list), modify in memory, PUT with `If-Match`. This is the same pattern as `update_contact` — no new HTTP primitives needed.
- **`--group on contacts create` requires sequencing:** Contact must be created first (to get its UID), then the group vCard must be fetched and updated with the new member UID. If the group update fails after contact creation, the contact exists but is not in the group — the CLI should report the partial success clearly.
- **Get group members enhances MCP query:** Full member resolution (UID to Contact struct) is a value-add but requires an extra REPORT round-trip. The raw group vCard (with member UIDs) is sufficient for CLI listing; resolved contacts are needed for MCP responses to be useful to AI agents.

---

## MVP Definition

### Launch With (v1.3)

Minimum viable group management — covers all specified milestone targets.

- [ ] `ContactGroup` struct with `id`, `name`, `member_uids`, `href`, `etag` fields
- [ ] Filter group vCards from contact vCards in `parse_contacts_from_xml` (detect `X-ADDRESSBOOKSERVER-KIND:group`)
- [ ] `list_groups(addressbook_href)` on `CardDavClient` — returns `Vec<ContactGroup>`
- [ ] `create_group(addressbook_href, name)` — PUT new group vCard with no members
- [ ] `get_group(group_id)` — fetch group vCard, return `ContactGroup` with resolved member contacts
- [ ] `rename_group(href, etag, new_name)` — PUT updated group vCard with changed `FN:`/`N:`
- [ ] `delete_group(href, etag)` — DELETE group vCard only (members untouched)
- [ ] `add_member(group_href, group_etag, contact_uid)` — fetch-modify-PUT to append member
- [ ] `remove_member(group_href, group_etag, contact_uid)` — fetch-modify-PUT to remove member
- [ ] CLI `contacts groups list` — print groups as JSON
- [ ] CLI `contacts groups create <name>` — create empty group
- [ ] CLI `contacts groups get <id>` — show group with member list
- [ ] CLI `contacts groups rename <id> <new-name>` — rename group
- [ ] CLI `contacts groups delete <id> [--confirm]` — delete group, require `--confirm` flag (same pattern as contact delete)
- [ ] CLI `contacts groups add-member <group-id> <contact-id>` — add contact to group
- [ ] CLI `contacts groups remove-member <group-id> <contact-id>` — remove contact from group
- [ ] CLI `contacts create --group <group-id>` — create contact and assign to group in one command
- [ ] MCP `listGroups` query
- [ ] MCP `getGroup` query (with resolved member contacts)
- [ ] MCP `createGroup`, `renameGroup`, `deleteGroup` mutations
- [ ] MCP `addGroupMember`, `removeGroupMember` mutations

### Add After Validation (v1.x)

- [ ] Group membership validation warning (check contact UID exists before adding) — add when users report confusing behavior with dangling UIDs
- [ ] `contacts list --group <id>` filter — filter contact list to group members — add when scripting use cases emerge

### Future Consideration (v2+)

- [ ] Bulk add/remove members in one command — defer until workflow need is validated
- [ ] Group-targeted email send (`fastmail-cli send --to-group <id>`) — cross-feature, needs mail integration work

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| List groups | HIGH | LOW | P1 |
| Create group | HIGH | LOW | P1 |
| Get group (with members) | HIGH | LOW | P1 |
| Rename group | MEDIUM | LOW | P1 |
| Delete group | HIGH | LOW | P1 |
| Add member | HIGH | LOW | P1 |
| Remove member | HIGH | LOW | P1 |
| `--group` on contacts create | MEDIUM | MEDIUM | P1 |
| MCP mutations for all group ops | HIGH | MEDIUM | P1 |
| Resolved member contacts in MCP get | MEDIUM | MEDIUM | P2 |
| Group membership UID validation | LOW | LOW | P2 |
| `contacts list --group` filter | LOW | LOW | P3 |

---

## Protocol Reference Notes

**Serializing a group vCard (vCard 3.0 format for Fastmail):**
```
BEGIN:VCARD
VERSION:3.0
UID:<uuid-v4>
N:<group name>;;;;
FN:<group name>
X-ADDRESSBOOKSERVER-KIND:group
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<member-uid-1>
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<member-uid-2>
END:VCARD
```

**Member UID URI format:** `urn:uuid:<uid>` where `<uid>` is the contact's `UID` property value (already a UUID in this codebase).

**Identifying group vCards during parse:** Check for line `X-ADDRESSBOOKSERVER-KIND:group` in the unfolded vCard string. The existing `parse_vcard` function will need a parallel `parse_vcard_group` path or a `kind` discriminant returned alongside the parsed struct.

**ETag discipline:** All group write operations (rename, add/remove member, delete) must follow the same ETag-guarded pattern as existing contact writes. Fetch returns ETag; subsequent PUT/DELETE sends `If-Match: <etag>`. 412 Precondition Failed means concurrent modification.

**Delete behavior:** Deleting a group vCard does NOT cascade to member contacts. Member contact vCards remain untouched on the server. This is the expected behavior for vCard-type groups and must be clearly documented in CLI help text.

**Empty groups:** Fastmail supports empty groups (group vCard with no `X-ADDRESSBOOKSERVER-MEMBER` lines). This contrasts with CATEGORIES-type groups which cease to exist when empty.

---

## Sources

- [DAVx5: Tested With Fastmail](https://www.davx5.com/tested-with/fastmail) — "Contact group method: Groups are separate vCards" (HIGH confidence)
- [rcmcarddav GROUPS.md](https://github.com/mstilkerich/rcmcarddav/blob/master/doc/GROUPS.md) — Authoritative breakdown of vCard-type vs CATEGORIES-type groups
- [RFC 6350 vCard Format Specification](https://www.rfc-editor.org/rfc/rfc6350.html) — vCard 4.0 KIND and MEMBER properties
- [RFC 6352 CardDAV](https://datatracker.ietf.org/doc/html/rfc6352) — CardDAV REPORT and PUT semantics
- [Fastmail Contact Groups help](https://www.fastmail.help/hc/en-us/articles/360058753114-Contact-groups) — User-facing behavior (rename, delete does not delete members)
- [DAVx5: Can't manage groups on device](https://www.davx5.com/faq/cant-manage-groups-on-device) — Group format compatibility matrix
- [Nicolas Grilly: Migrating Google Contacts to Fastmail](https://www.grilly.com/posts/migrate-google-contacts-labels-to-fastmail-groups/) — Confirms Fastmail and Apple use vCard-type groups (not CATEGORIES)
- [Fastmail Troubleshooting CardDAV fields](https://www.fastmail.help/hc/en-us/articles/360058753094-Troubleshooting-CardDAV-fields) — vCard 3.0 format, X- extension field storage behavior

---
*Feature research for: contact group CRUD and membership management (CardDAV/vCard-type groups, Fastmail)*
*Researched: 2026-04-13*
