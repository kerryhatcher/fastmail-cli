# Pitfalls Research

**Domain:** CardDAV contact group CRUD and membership management — adding to an existing Rust CLI + MCP server targeting Fastmail
**Researched:** 2026-04-13
**Confidence:** HIGH for format and parsing pitfalls (multiple sources corroborate); MEDIUM for Fastmail-specific group behavior (no official Fastmail developer docs for groups exist; inferred from DAVx5 compatibility page + community migration reports)

---

## Critical Pitfalls

### Pitfall 1: Using the Wrong Group Format for Fastmail (KIND vs. X-ADDRESSBOOKSERVER-KIND)

**What goes wrong:**
vCard 4.0 uses `KIND:group` and `MEMBER:urn:uuid:<uid>` to represent groups. vCard 3.0 (Apple/iCloud extension) uses `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>`. Fastmail uses vCard 3.0 on its CardDAV server, and DAVx5's Fastmail compatibility page explicitly states "Contact group method: Groups are separate vCards." Community evidence and Apple-lineage interoperability reports consistently place Fastmail in the Apple/X-ADDRESSBOOKSERVER camp, not the vCard 4 KIND camp.

If you write groups using `KIND:group` and `MEMBER:`, Fastmail stores the data (it never discards unknown properties) but its web UI does not recognize the group — it appears as a plain contact, not a group. Existing Apple clients that sync via the same account will not recognize the group either because they look for `X-ADDRESSBOOKSERVER-KIND`, not `KIND`.

**Why it happens:**
RFC 6350 (vCard 4) is the standard and looks cleaner. Developers reading the spec implement `KIND`/`MEMBER` without checking whether the target server has actually upgraded to vCard 4. The existing `serialize_vcard()` in this codebase already produces `VERSION:3.0`; writing `KIND:group` on a 3.0 vCard is technically invalid and will confuse both Fastmail and interoperating clients.

**How to avoid:**
Always emit `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` for groups when the server uses vCard 3.0. Check the `VERSION` line in the existing `serialize_vcard()` — it hardcodes `VERSION:3.0`, confirming the correct family is `X-ADDRESSBOOKSERVER-*`. Write a separate `serialize_group_vcard()` function (or extend `serialize_vcard()` with a `kind: Option<ContactKind>` parameter) that emits the Apple-extension properties. Do not mix vCard 4 `KIND`/`MEMBER` with `VERSION:3.0`.

**Warning signs:**
- Group vCards stored on Fastmail do not appear as groups in Fastmail's web UI after creation
- A `list_contacts()` REPORT returns the group vCard with a name but no `X-ADDRESSBOOKSERVER-KIND` line in the returned `address-data`
- Apple Contacts.app syncing the same account ignores your programmatically created groups

**Phase to address:**
Group CRUD foundation phase — must be verified with a live Fastmail write before any membership management work begins.

---

### Pitfall 2: parse_vcard() Silently Discards Group vCards (Missing NAME Guard)

**What goes wrong:**
The existing `parse_vcard()` function returns `None` when the `FN` (full name) property is missing or empty:

```
// Need at least a name
if name.is_empty() {
    return None;
}
```

A freshly created group vCard for "Work Contacts" has `FN:Work Contacts` — this is fine. But group vCards created by Apple clients or older CardDAV tools sometimes set an empty `FN` (just `FN:`) for internal groups, or omit it entirely. Any such group vCard is silently dropped by `parse_contacts_from_xml()`, making `list_contacts()` appear to succeed but return zero groups.

Additionally, the current parser does not extract `X-ADDRESSBOOKSERVER-KIND` or `X-ADDRESSBOOKSERVER-MEMBER` properties at all — when `list_contacts()` returns results, the caller has no way to distinguish a group from a regular contact. This forces every caller to re-fetch and re-parse the vCard body to determine group membership.

**Why it happens:**
`parse_vcard()` was written for contacts, which always have a name. Groups are a new first-class object type not modeled in the original design. The `Contact` struct has no `kind` or `members` field to carry group metadata.

**How to avoid:**
Add a `ContactKind` enum (`Individual`, `Group`) and a `members: Vec<String>` field (containing bare UIDs, without `urn:uuid:` prefix — see Pitfall 4) to the `Contact` struct, or create a parallel `ContactGroup` struct. Extend `parse_vcard()` to extract the `X-ADDRESSBOOKSERVER-KIND` line and all `X-ADDRESSBOOKSERVER-MEMBER` lines, storing members in order. When `kind == Group`, relax the name-required guard (or ensure the group serializer always emits a non-empty FN). Add a `list_groups()` method on `CardDavClient` that filters `list_contacts()` results by `kind == Group`, or issues a targeted REPORT with a property filter on `X-ADDRESSBOOKSERVER-KIND`.

**Warning signs:**
- `contacts group list` returns an empty list even after creating a group via the CLI
- `contacts list` returns both contacts and group vCards intermixed — the caller has no way to filter
- Adding a contact to a group and then listing the group's members returns an empty `members` vec

**Phase to address:**
Group CRUD foundation phase — the data model extension must land before any mutation or membership code is written.

---

### Pitfall 3: Multiple X-ADDRESSBOOKSERVER-MEMBER Lines Silently Collapsed to One

**What goes wrong:**
A group vCard with three members looks like this on the wire:

```
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:d4c1baf6-f603-4fb5-8f19-d45eb1e7fb23
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:b8767877-b4a1-4c70-9acc-505d3819e519
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:03a0e51f-d1aa-4385-8a53-e29025acd8af
```

The three lines have identical property names. The existing `parse_vcard()` loop processes lines with `if line.starts_with(...)` and simple assignment (`organization = Some(extract_value(line))`). If `X-ADDRESSBOOKSERVER-MEMBER` is naively added to this loop with a single `Option<String>` or a `=` assignment, only the last member UID is retained. This is a well-documented bug in at least one production CardDAV implementation (Nextcloud issue #9369).

**Why it happens:**
Most vCard properties are single-valued (FN, UID, ORG). The existing parser uses scalar variables and overwrites on each match. Multi-valued properties (EMAIL, TEL, and now MEMBER) require accumulation into a `Vec`. The pattern for EMAIL and TEL already exists in `parse_vcard()` — developers adding MEMBER support must follow the Vec accumulation pattern, not the scalar assignment pattern.

**How to avoid:**
Follow the exact pattern used for `emails: Vec<ContactEmail>` and `phones: Vec<ContactPhone>`. Declare `let mut members: Vec<String> = Vec::new();` before the loop and `members.push(uid)` inside the match arm. Serialize groups by emitting one `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` line per member, relying on `fold_line()` only for long UIDs (which are always short, so folding will not occur). Add a unit test that round-trips a group vCard with three members and asserts all three UIDs are present after parsing.

**Warning signs:**
- A group with N members consistently shows only one member after a list/get round-trip
- `serialize_group_vcard()` produces correct multi-line output but `parse_vcard()` returns `members.len() == 1`
- Adding a second contact to a group that already has one member appears to succeed but the first member disappears on the next list

**Phase to address:**
Group CRUD foundation phase — add a round-trip unit test for multi-member groups before any membership management commands are wired.

---

### Pitfall 4: urn:uuid: Prefix Confusion — Stored vs. Wire Format

**What goes wrong:**
On the wire, `X-ADDRESSBOOKSERVER-MEMBER` values include the `urn:uuid:` prefix:
```
X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:d4c1baf6-...
```

The contact's own `UID` property does NOT include this prefix:
```
UID:d4c1baf6-...
```

This asymmetry means: when adding a contact to a group, you must store `urn:uuid:<contact.id>` in the group vCard, not `<contact.id>` bare. When looking up which contacts belong to a group, you must strip the `urn:uuid:` prefix before comparing against `contact.id`. If the prefix is stored as-is in `members: Vec<String>`, every membership lookup requires string manipulation at the call site. If the prefix is stripped on parse and not re-added on serialize, the group vCard written to Fastmail has invalid `MEMBER` lines.

**Why it happens:**
This is a vCard 3.0 extension quirk — the `urn:uuid:` prefix is a URN scheme required only in the `MEMBER` / `X-ADDRESSBOOKSERVER-MEMBER` property context. Developers familiar only with the UID property expect raw UUIDs everywhere. RFC 6350 (vCard 4) uses the same `urn:uuid:` scheme for `MEMBER`, so the asymmetry exists in both formats.

**How to avoid:**
Establish a single convention at the boundary: strip `urn:uuid:` during parse, store bare UIDs in `members: Vec<String>`, and re-add the prefix during serialization. Document this convention with a comment in both the parse and serialize paths. Add constants:
```rust
const MEMBER_URN_PREFIX: &str = "urn:uuid:";
```
Use these to strip on parse and prepend on serialize. Add a unit test asserting that `parse` → `serialize` → `parse` produces identical bare UIDs.

**Warning signs:**
- Membership lookups that compare `group.members.contains(&contact.id)` always return false for groups fetched from the server
- A round-trip (fetch group, add member, write group) doubles the `urn:uuid:` prefix: `urn:uuid:urn:uuid:...`
- Integration test against WireMock passes (test fixture uses bare UIDs) but live Fastmail test fails (server returns full URN format)

**Phase to address:**
Group CRUD foundation phase — this is a serialization invariant that must be established before membership management, not after.

---

### Pitfall 5: Membership Add/Remove Is a Full Group vCard Read-Modify-Write Under ETag Guard

**What goes wrong:**
CardDAV has no PATCH operation. Adding or removing a member from a group requires:
1. GET (or use cached version) of the current group vCard — obtain current ETag
2. Parse the current `X-ADDRESSBOOKSERVER-MEMBER` list
3. Add or remove the target UID
4. PUT the full rewritten group vCard with `If-Match: <etag>`

If two CLI invocations (or two MCP tool calls) race to add members to the same group concurrently, the second PUT will receive a 412 Precondition Failed because the first PUT changed the group's ETag. The caller must GET the latest version and retry. Without retry logic, the second concurrent membership change is silently lost or returns an opaque 412 error to the user.

This is more acute than for regular contact updates because AI agents composing multiple `add-member` calls back-to-back (e.g., "add Alice, Bob, and Carol to Work group") will naturally serialize or parallelize these and hit the race window.

**Why it happens:**
The existing `update_contact()` returns a `ContactConflict` error on 412 and propagates it to the caller — there is no retry. For single contact updates this is acceptable (user re-runs the command). For membership management, callers expect `add_member_to_group(group_id, contact_id)` to be a logical atomic operation, not a manual read-modify-write they have to retry.

**How to avoid:**
Implement `add_member_to_group` and `remove_member_from_group` as retrying read-modify-write operations within `CardDavClient`:
1. Fetch the current group by href (GET, not list)
2. Mutate the members list
3. PUT with `If-Match`
4. On 412: back off briefly (50-100ms), repeat from step 1
5. Give up after 3 attempts and return `ContactConflict`

For the MCP `addGroupMember` / `removeGroupMember` mutations, document in the GraphQL schema that the operation is idempotent on success but may fail with a conflict error on concurrent modification. Do not expose the retry logic to callers — encapsulate it.

**Warning signs:**
- `contacts group add-member` returns a `ContactConflict` error when no other client is active — indicates the group's ETag was not fetched before the PUT
- Agent sessions that add multiple members to the same group fail on the second member
- WireMock integration test for concurrent adds receives two PUTs on the same resource without any GET between them

**Phase to address:**
Group membership management phase — the retry pattern must be in place before membership commands are exposed in CLI and MCP.

---

### Pitfall 6: Group Deletion Does Not Cascade — Members Still Exist on Server, But Membership Is Orphaned Silently

**What goes wrong:**
In the vCard-type group model (which Fastmail uses), group membership is encoded only in the group's vCard via `X-ADDRESSBOOKSERVER-MEMBER`. The member contact vCards themselves have no back-reference to the group. When a group is deleted (DELETE on the group's href), the member contacts are unaffected — they remain on the server as normal contacts.

This is correct and expected behavior. The pitfall is different: if the group vCard is deleted but the `Contact` struct in the application's data model (or a GraphQL resolver) caches group membership, stale membership data will be served until the cache is invalidated. In the MCP context, a tool call sequence of `deleteGroup(id)` → `listContacts()` that reads from a local cache will still show contacts as group members.

The secondary pitfall: a `deleteGroup` command that does NOT first warn the user how many contacts are currently in the group provides a worse UX than `deleteCalendar` (which already has a `--confirm` flag). Silent deletion of a group with 50 members is surprising.

**Why it happens:**
The existing `delete_contact()` is a direct DELETE with ETag — no cascade check, no membership scan. The same pattern applied naively to `delete_group()` is technically correct at the protocol level but surprising at the UX level. There is no CardDAV server-side cascade; the server never modifies member vCards on group deletion.

**How to avoid:**
Before deleting a group, fetch its member count. If > 0, require an explicit confirmation flag (`--confirm` or `--yes`, consistent with existing delete commands). In the GraphQL mutation, surface member count in the mutation's return type so MCP callers can show it to users. Do NOT attempt to "clean up" member vCards by removing their back-references — there are no back-references in the vCard-type model. Cache invalidation: the `CardDavClient` holds no per-group cache, so no invalidation is needed unless a caching layer is added.

**Warning signs:**
- `contacts group delete <id>` succeeds silently with no mention of 30 members that were in the group
- After group deletion, `contacts list` shows members that the user expected to be "in the group" are still present (this is correct but confusing if users expect cascade delete)
- An MCP agent calls `deleteGroup` without checking member count and then reports to the user that "50 contacts have been deleted" — they have not been

**Phase to address:**
Group CRUD foundation phase — the delete command confirmation pattern must match the existing `contacts delete --confirm` convention from the start.

---

### Pitfall 7: list_contacts() Returns Groups and Contacts Intermixed — Filtering Requires KIND Parsing

**What goes wrong:**
Fastmail stores group vCards in the same address book as regular contact vCards. A `list_contacts()` REPORT returns both. The current `parse_contacts_from_xml()` does not check `X-ADDRESSBOOKSERVER-KIND`, so all vCards — including groups — are deserialized as `Contact` structs with empty `emails` and `phones` vecs. The `contacts list` command will show group vCards as contacts with just a name and no email, which is confusing.

A `contacts group list` command built on top of `list_contacts()` requires post-filtering by `kind == Group`. If `kind` is not modeled in the `Contact` struct, this filtering is impossible without re-fetching individual vCards.

The CardDAV `addressbook-query` REPORT supports a `<card:prop-filter>` element that can filter by property presence, but Fastmail's Cyrus IMAP-based server is known to have incomplete property filter support. Relying on server-side kind filtering may not work and is not tested.

**Why it happens:**
When groups were out of scope, ignoring `X-ADDRESSBOOKSERVER-KIND` was safe. With groups in scope, the address book is a mixed collection of two distinct resource types that the data model must differentiate.

**How to avoid:**
Add `kind: ContactKind` to the `Contact` struct with `#[serde(default)]` so existing JSON serialization does not break. Default to `ContactKind::Individual`. Parse `X-ADDRESSBOOKSERVER-KIND:group` in `parse_vcard()` and set `kind = ContactKind::Group`. Filter in `list_contacts()` callers or add explicit `list_groups()` / `list_individual_contacts()` methods that call `list_contacts()` and filter by kind. Do not rely on server-side REPORT filters for kind — use client-side filtering.

**Warning signs:**
- `contacts list` shows entries with no email and no phone — these are group vCards leaking into the contact list
- `contacts group list` returns an empty list (groups not parsed) or the full contact list including individuals (no kind filtering)
- `contacts create alice@example.com` creates a contact that appears alongside group entries in list output

**Phase to address:**
Group CRUD foundation phase — the data model must distinguish kinds before any command is implemented.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Reuse `Contact` struct for groups (no `kind` field) | No data model change | `list_contacts()` returns intermixed groups and contacts; callers cannot filter; membership info is inaccessible | Never for v1.3 — groups need a distinguishable type |
| Store `urn:uuid:` prefix in `members: Vec<String>` | Skip prefix stripping logic | Every membership lookup must strip prefix; easy to double-prefix on a round-trip | Never — establish the bare-UID convention at the boundary |
| Skip ETag guard on group membership writes | Simpler implementation | Lost updates when multiple agents modify the same group; 412 errors surfaced to callers unexpectedly | Never — the existing ETag discipline must extend to groups |
| Re-fetch group on every member add/remove (no cache) | Always fresh | Acceptable for MVP; two round-trips per membership change | Acceptable in v1.3 MVP; optimize with a stale-while-revalidate cache in v1.4 if needed |
| Use `contacts delete` pattern for group delete (no member count warning) | Consistent CLI behavior | Users silently delete large groups without understanding what membership is being discarded | Never — group delete needs member count disclosure |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Fastmail CardDAV group format | Writing `KIND:group` on a `VERSION:3.0` vCard | Use `X-ADDRESSBOOKSERVER-KIND:group` and `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` on 3.0 vCards |
| Fastmail stores but does not display X- extensions | Assuming stored data is visible in the Fastmail web UI | Test group creation by round-tripping via CardDAV (REPORT), not by checking the Fastmail web contact list |
| Group vCard href and ETag | Using `list_contacts()` cached result for a group PUT | Fetch the group's current ETag immediately before every write (GET or targeted REPORT) to avoid stale ETag 412 |
| REPORT returns groups in contact list | Assuming `list_contacts()` returns only contacts | Always filter by `kind` after parsing; do not assume an empty `emails` list means "not a group" |
| Fastmail 412 on group writes | Treating 412 as a fatal error in membership operations | Implement GET-then-PUT retry with backoff; expose `ContactConflict` only after exhausting retries |
| vCard property filter in REPORT | Using `<card:prop-filter name="X-ADDRESSBOOKSERVER-KIND">` to fetch only groups | Fastmail's Cyrus IMAP server may not support property filters — do client-side kind filtering instead |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Re-fetching all contacts to find group members | `contacts group get` takes >1s for address books with >200 contacts | Cache the address book listing per `CardDavClient` instance; for targeted lookups use GET by href directly | Users with large Fastmail contact books (>200 contacts) |
| Re-fetching entire address book on every membership add | Each `add_member` does a full `list_contacts()` just to get the group's current vCard | Store the group's `href` + `etag` from the create/last-update response; use GET-by-href (single resource) for membership updates | Every `add_member` call — the full list is never needed |
| Concurrent membership adds trigger repeated 412 retry loops | Two agents adding to the same group simultaneously both retry indefinitely | Cap retries at 3; add jitter to backoff; surface conflict error to callers after 3 attempts | If an AI agent fires N parallel `add_member` calls — unlikely but possible |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Group name not escaped in vCard serialization | Group name containing `\r\n` or `;` injects arbitrary vCard properties into the group vCard stored on Fastmail | Apply `escape_value()` to the group `FN` and `N` fields exactly as done for contact names in the existing `serialize_vcard()` |
| Member UID not validated before inserting into group vCard | Attacker-supplied UID containing `\r\n` breaks the vCard line structure | Validate that each member UID matches UUID format before including in `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>`; the UID field on contact create is a UUID v4 — enforce the same constraint on group membership inputs |
| Group delete without ETag | Without `If-Match` on DELETE, two concurrent agents can both believe they succeeded on the same group | Apply the same ETag-guarded DELETE pattern used by `delete_contact()` — pass the group's current ETag in `If-Match` |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| `contacts group delete` with no member count disclosure | User unknowingly destroys a group with 50 members, assuming "delete group" also deletes contacts | Show member count before deletion: "Deleting group 'Work' (3 members). Pass --confirm to proceed." |
| `--group` flag on `contacts create` fails silently if group does not exist | Contact is created without group membership; user sees no error | Validate the group ID before creating the contact; return an error if the group is not found |
| Group list output mixes raw UIDs for members | Members appear as opaque UUIDs rather than names | Resolve member UIDs to contact names in `group get` output; include both UID and name in JSON response |
| Membership add does not return updated member list | User runs `add-member` and must separately run `group get` to confirm | Return the full updated group (including resolved member names) from `add_member` and `remove_member` operations |

---

## "Looks Done But Isn't" Checklist

- [ ] **Group vCard format:** Verify Fastmail web UI shows the created group as a group (not a nameless contact) — the `X-ADDRESSBOOKSERVER-KIND:group` line must be present and recognized
- [ ] **Multi-member round-trip:** Create a group with 3 members, fetch it back, assert all 3 UIDs are returned — confirms no last-value-wins overwrite
- [ ] **urn:uuid prefix consistency:** serialize → parse → serialize produces identical `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:...` lines with no doubled prefix
- [ ] **Contacts list excludes groups:** `contacts list` does not show group vCards in its output after group creation
- [ ] **Group list excludes contacts:** `contacts group list` shows only group vCards, not individual contacts
- [ ] **ETag guard on membership writes:** A `add-member` PUT is always preceded by a GET/REPORT of the current group ETag — never uses a stale ETag from a prior `list_contacts()` call
- [ ] **Group delete with --confirm:** Deleting a group with >0 members requires `--confirm`; without it, the command exits with a non-zero status and a clear message
- [ ] **--group flag on contacts create:** Assigning a non-existent group returns an actionable error, not a silent no-op
- [ ] **MCP mutations return updated state:** `addGroupMember` and `removeGroupMember` return the updated group object, not just a success boolean

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Wrong group format (KIND instead of X-ADDRESSBOOKSERVER-KIND) | MEDIUM | Delete incorrectly formatted group vCards; re-create using the correct Apple-extension format; no contact data is lost |
| Multi-member parse collapsing to one | HIGH | All groups with >1 member have corrupted membership data on the server after any write; must re-add all members; add a migration tool that fetches all group hrefs, re-parses with the fixed parser, and reports discrepancies before writing |
| urn:uuid prefix doubled on round-trip | MEDIUM | Groups written with doubled prefix are unrecognized by Apple clients; re-write all group vCards after fixing the serializer; testable by fetching raw vCard from Fastmail via REPORT |
| Stale ETag on membership write | LOW | Already handled by `ContactConflict` error; user re-runs the command; no data loss since 412 means the write was rejected |
| Group deleted without confirmation | HIGH | Group vCard is gone; member contacts still exist; group must be re-created and members re-added; no automatic recovery |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Wrong vCard group format (KIND vs. X-ADDRESSBOOKSERVER-KIND) | Phase 1: Group data model + CRUD foundation | Live Fastmail write: create a group, verify it appears as a group in Fastmail web UI and via DAVx5 sync |
| parse_vcard silently discards group vCards | Phase 1: Group data model + CRUD foundation | Unit test: parse group vCard with empty FN; unit test: kind field populated for group vCards |
| Multi-member X-ADDRESSBOOKSERVER-MEMBER collapsed to one | Phase 1: Group data model + CRUD foundation | Unit test: round-trip vCard with 3 members; assert members.len() == 3 |
| urn:uuid prefix confusion | Phase 1: Group data model + CRUD foundation | Unit test: serialize → parse → serialize produces identical MEMBER lines |
| Membership add/remove race condition (read-modify-write) | Phase 2: Group membership management | WireMock integration test: two concurrent add-member calls; verify final group has both members |
| Group deletion without member count warning | Phase 1: Group CRUD (delete command) | CLI test: `contacts group delete <id>` with non-empty group exits non-zero without --confirm |
| list_contacts() returns groups and contacts intermixed | Phase 1: Group data model + CRUD foundation | Integration test: create one group + one contact; assert `contacts list` returns only the contact; `contacts group list` returns only the group |

---

## Sources

- DAVx5 Fastmail compatibility page (confirms "Groups are separate vCards" for Fastmail): https://www.davx5.com/tested-with/fastmail
- rcmcarddav GROUPS.md (comprehensive vCard group type comparison, X-ADDRESSBOOKSERVER vs CATEGORIES vs KIND): https://github.com/mstilkerich/rcmcarddav/blob/master/doc/GROUPS.md
- Nextcloud issue #9369 (X-ADDRESSBOOKSERVER-MEMBER overwrite bug in production code): https://github.com/nextcloud/server/issues/9369
- DAVx5 FAQ on group management: https://www.davx5.com/faq/cant-manage-groups-on-device
- DAVx5 technical documentation (KIND vs X-ADDRESSBOOKSERVER-KIND fallback): https://manual.davx5.com/technical_information.html
- RFC 6352 §6.3.2 (lost update prevention with If-Match on CardDAV PUT): https://www.rfc-editor.org/rfc/rfc6352
- RFC 6350 (vCard 4 KIND:group and MEMBER:urn:uuid format): https://www.rfc-editor.org/rfc/rfc6350.html
- rcmcarddav issue #331 (vCard 4 KIND/MEMBER not recognized by vCard 3-only parsers): https://github.com/mstilkerich/rcmcarddav/issues/331
- Nicolas Grilly migration post (Fastmail uses groups, not CATEGORIES): https://www.grilly.com/posts/migrate-google-contacts-labels-to-fastmail-groups/
- Fastmail contact groups help article: https://www.fastmail.help/hc/en-us/articles/360058753114-Contact-groups
- Existing codebase: src/carddav/mod.rs (parse_vcard, serialize_vcard, parse_contacts_from_xml)

---
*Pitfalls research for: fastmail-cli v1.3 Contact Groups*
*Researched: 2026-04-13*
