# Phase 19: Group Membership Management - Research

**Researched:** 2026-04-13
**Domain:** CardDAV vCard membership operations (Rust/reqwest), ETag-guarded retry, CLI/MCP surfaces
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Membership CardDAV Transport**
- Add member: fetch group vCard -> append `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` line -> PUT with `If-Match: <etag>`
- Remove member: fetch group vCard -> remove matching MEMBER line -> PUT with `If-Match: <etag>` (same ETag-guarded pattern)
- ETag race handling: retry-on-412 loop — fetch fresh vCard+ETag, re-apply change, re-PUT (max 3 retries) — prevents silently dropped members on concurrent access
- Member validation: validate contact UID exists via `get_contact_by_id()` before adding — fail early with clear error

**CLI Commands and --group Flag**
- `contacts groups add-member <group-id> <contact-id>` and `contacts groups remove-member <group-id> <contact-id>` under existing groups subcommand
- `contacts create --group <group-id>` partial failure behavior: if contact creates OK but group-add fails, report both outcomes clearly ("Contact created (ID: X) but group add failed: Y. Run `contacts groups add-member` to retry.")
- Group identifier accepts ID or name (reuse `resolve_group` from Phase 18); contact identifier accepts ID only
- Output format: JSON via `Output::success()` showing updated group with member count and resolved member list (matches `groups get` output)

**MCP/GraphQL Mutations**
- `addGroupMember(groupId: ID!, contactId: ID!): ContactGroup!` and `removeGroupMember(groupId: ID!, contactId: ID!): ContactGroup!` — return full group with resolved members
- Error handling: GraphQL field error with descriptive message ("Contact not found: <id>") — consistent with existing MCP error patterns
- No separate `createContactInGroup` mutation — agents compose `createContact` + `addGroupMember` calls (simpler, more flexible, composable)

### Claude's Discretion
No items deferred to Claude's discretion — all decisions captured above.

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MBR-01 | User can add a contact to a group | `add_group_member()` method on `CardDavClient`: fetch group vCard, append MEMBER line, ETag-guarded PUT with 3-retry loop |
| MBR-02 | User can remove a contact from a group | `remove_group_member()` method on `CardDavClient`: fetch group vCard, filter out MEMBER line, ETag-guarded PUT with 3-retry loop |
| MBR-03 | User can create a contact and assign it to a group in one command via `--group` | Add `--group Option<String>` to `ContactsCommands::Create`; post-create call to `add_group_member`; partial-failure reporting pattern |
| CLI-02 | User can manage membership via `contacts groups add-member` and `remove-member` | Two new `GroupsCommands` variants dispatching to new handler functions in `src/commands/contacts.rs` |
| CLI-04 | `contacts create --group <id>` assigns the new contact to a group at creation | Modify `ContactsCommands::Create` struct and dispatch arm in `src/main.rs` |
| MCP-03 | AI agents can manage membership via `addGroupMember`, `removeGroupMember` | Two new `async fn` methods in `MutationRoot` in `src/mcp/graphql/mutation.rs`; return `GqlContactGroup` with resolved members |
</phase_requirements>

## Summary

Phase 19 adds the membership write operations that complete the contact groups feature. The core work is two new `CardDavClient` methods (`add_group_member` and `remove_group_member`) that encapsulate the fetch-mutate-PUT loop with ETag retry, then surfacing those through CLI (`GroupsCommands::AddMember/RemoveMember`) and MCP (`addGroupMember`/`removeGroupMember` mutations).

All primitive building blocks already exist in the codebase from Phase 18. `serialize_group_vcard()` takes a `ContactGroup` struct and emits the correct MEMBER lines. `parse_group_vcard()` (called indirectly via `get_group_by_id()`) returns the full vCard including existing member UIDs. The `map_group_write_response()` helper already maps HTTP 412 to `Error::GroupConflict`, which is the signal to retry. The `resolve_group_members()` method handles the post-mutation resolution needed by both CLI output and MCP return type.

The `--group` flag on `contacts create` is a wrapper — not a new CardDAV primitive — so it calls existing `create_contact_record()` then `add_group_member`. The only novel complexity is the partial-failure reporting path and the retry loop for concurrent ETag conflicts.

**Primary recommendation:** Implement `add_group_member` and `remove_group_member` as methods on `CardDavClient` with the retry-on-412 pattern. All CLI and MCP surfaces are thin wrappers over those two methods plus existing Phase 18 infrastructure.

## Standard Stack

### Core (already in Cargo.toml — no new dependencies)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| reqwest | 0.13.1 | HTTP PUT/GET for CardDAV writes | Already used for all CardDAV operations |
| tokio | 1.49.0 | Async runtime | Project-wide async runtime |
| async-graphql | 7.x | GraphQL mutation definitions | Existing MCP schema |
| thiserror | 2.0.17 | Error derivation | `GroupConflict` already defined |
| serde / serde_json | 1.0 | JSON output serialization | Used by `Output::success()` |

No new dependencies required. All needed libraries are already declared in `Cargo.toml`.

## Architecture Patterns

### Recommended Project Structure

The phase touches four existing files only. No new files needed.

```
src/
├── carddav/mod.rs          # Add: add_group_member(), remove_group_member()
├── commands/contacts.rs    # Add: add_group_member_cmd(), remove_group_member_cmd()
│                           # Modify: create_contact() for --group flag
├── main.rs                 # Add: AddMember/RemoveMember variants to GroupsCommands
│                           # Modify: ContactsCommands::Create to include group: Option<String>
└── mcp/graphql/mutation.rs # Add: add_group_member(), remove_group_member() resolvers
```

### Pattern 1: ETag-Guarded Retry Loop (read-modify-write)

**What:** Fetch current group vCard (with ETag), apply the membership change, PUT with `If-Match: <current_etag>`. If server returns 412, refetch and retry up to 3 times.

**When to use:** Any time a write depends on reading current server state first — prevents silently dropped members when concurrent clients also modify the group.

**Example — intended implementation:**
```rust
// In src/carddav/mod.rs, CardDavClient impl
#[instrument(skip(self))]
pub async fn add_group_member(&self, group_id: &str, contact_uid: &str) -> Result<ContactGroup> {
    // Step 1: validate contact exists (fail-fast)
    self.get_contact_by_id(contact_uid).await?;

    const MAX_RETRIES: u8 = 3;
    for attempt in 0..MAX_RETRIES {
        // Step 2: fetch current group state (fresh ETag on each attempt)
        let group = self.get_group_by_id(group_id).await?;
        let href = group.href.as_deref()
            .ok_or_else(|| Error::GroupNotFound(group_id.to_string()))?;
        let etag = group.etag.as_deref()
            .ok_or_else(|| Error::GroupConflict {
                id: group_id.to_string(),
                sent_etag: String::new(),
                server_etag: None,
            })?;

        // Step 3: idempotency check — skip if already a member
        if group.member_uids.contains(&contact_uid.to_string()) {
            return Ok(group);
        }

        // Step 4: build updated group with new member
        let mut updated = group.clone();
        updated.member_uids.push(contact_uid.to_string());

        // Step 5: PUT with ETag guard
        let url = format!("{}{}", self.base_url, href);
        let vcard = serialize_group_vcard(&updated);
        let response = self.client
            .put(&url)
            .basic_auth(&self.username, Some(&self.app_password))
            .header("Content-Type", "text/vcard; charset=utf-8")
            .header(IF_MATCH, etag)
            .body(vcard)
            .send()
            .await?;

        let status = response.status();
        let headers = response.headers().clone();
        let body = response.text().await?;

        match map_group_write_response(group_id, Some(etag), status, &headers, &body) {
            Ok(new_etag) => {
                // Success — return updated group with new ETag
                let mut result = updated;
                result.etag = new_etag.or_else(|| Some(etag.to_string()));
                return Ok(result);
            }
            Err(Error::GroupConflict { .. }) if attempt < MAX_RETRIES - 1 => {
                // 412 Precondition Failed — retry with fresh fetch
                debug!(attempt, group_id, "ETag conflict on add_group_member, retrying");
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Err(Error::GroupConflict {
        id: group_id.to_string(),
        sent_etag: String::new(),
        server_etag: None,
    })
}
```

`remove_group_member` is identical except it filters instead of pushes:
```rust
updated.member_uids.retain(|uid| uid != contact_uid);
```
And the idempotency check inverts: skip if NOT in the list (contact already absent — success without PUT).

### Pattern 2: CLI Handler for Membership Commands

**What:** Thin async command handler that calls `CardDavClient` method and formats `Output::success()` with resolved member list — matches existing `get_group()` output shape.

**Example:**
```rust
// In src/commands/contacts.rs
pub async fn add_group_member(group_id_or_name: &str, contact_id: &str) -> AnyResult<()> {
    let client = contact_client()?;
    let group = resolve_group(&client, group_id_or_name).await?;
    let updated = client.add_group_member(&group.id, contact_id).await?;
    let members = client.resolve_group_members(&updated).await?;
    let data = serde_json::json!({
        "id": updated.id,
        "name": updated.name,
        "href": updated.href,
        "etag": updated.etag,
        "member_count": updated.member_uids.len(),
        "members": members,
    });
    Output::success(data).print();
    Ok(())
}
```

### Pattern 3: --group Flag with Partial Failure Reporting

**What:** After `create_contact_record()` succeeds, attempt `add_group_member`. On failure, print both outcomes rather than propagating the error as a fatal failure.

**Example:**
```rust
// In src/commands/contacts.rs
pub async fn create_contact_with_group(input: ContactInput, group_id: Option<&str>) -> AnyResult<()> {
    let contact = create_contact_record(input).await?;
    let contact_id = contact.id.clone();

    if let Some(gid) = group_id {
        let client = contact_client()?;
        let resolved = resolve_group(&client, gid).await;
        match resolved {
            Ok(group) => match client.add_group_member(&group.id, &contact_id).await {
                Ok(_) => {
                    Output {
                        success: true,
                        data: Some(&contact),
                        error: None,
                        message: Some(format!("Contact created and added to group {}", group.id)),
                    }.print();
                }
                Err(e) => {
                    // Partial failure: contact created, group-add failed
                    Output {
                        success: true,  // contact creation succeeded
                        data: Some(&contact),
                        error: Some(format!(
                            "Contact created (ID: {}) but group add failed: {}. \
                             Run `contacts groups add-member {} {}` to retry.",
                            contact_id, e, gid, contact_id
                        )),
                        message: Some("Contact created".to_string()),
                    }.print();
                }
            },
            Err(e) => {
                Output {
                    success: true,
                    data: Some(&contact),
                    error: Some(format!(
                        "Contact created (ID: {}) but group not found: {}. \
                         Run `contacts groups add-member {} {}` to retry.",
                        contact_id, e, gid, contact_id
                    )),
                    message: Some("Contact created".to_string()),
                }.print();
            }
        }
    } else {
        Output {
            success: true,
            data: Some(contact),
            error: None,
            message: Some("Contact created".to_string()),
        }.print();
    }
    Ok(())
}
```

Note: The `Output` struct must be checked to confirm its `error` field is `Option<String>` (verify in `src/models/mod.rs` before writing).

### Pattern 4: MCP Mutation Returning Resolved ContactGroup

**What:** Both `addGroupMember` and `removeGroupMember` return `ContactGroup!` (not a result wrapper) because they are straightforward mutations that either succeed or return a GraphQL field error. The resolved members are fetched after the mutation succeeds.

**Example:**
```rust
// In src/mcp/graphql/mutation.rs
async fn add_group_member(
    &self,
    ctx: &Context<'_>,
    #[graphql(desc = "Group ID (UID)")] group_id: String,
    #[graphql(desc = "Contact ID (UID)")] contact_id: String,
) -> Result<GqlContactGroup> {
    let app_ctx = ctx.data::<super::AppContext>()?;
    let carddav = app_ctx.get_carddav().await?;
    let updated = carddav
        .add_group_member(&group_id, &contact_id)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
    let members = carddav
        .resolve_group_members(&updated)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
    Ok(GqlContactGroup::with_members(updated, members))
}
```

This mirrors the `get_group` query resolver pattern exactly.

### Pattern 5: main.rs CLI Variant Additions

**GroupsCommands enum — add two variants:**
```rust
// In GroupsCommands enum in src/main.rs
/// Add a contact to a group
AddMember {
    /// Group ID (UID) or name
    group_id: String,
    /// Contact ID (UID)
    contact_id: String,
},

/// Remove a contact from a group
RemoveMember {
    /// Group ID (UID) or name
    group_id: String,
    /// Contact ID (UID)
    contact_id: String,
},
```

**ContactsCommands::Create — add optional group field:**
```rust
// In Create variant struct in src/main.rs
/// Assign to group at creation (group ID or name)
#[arg(long)]
group: Option<String>,
```

**Dispatch arm in match:**
```rust
GroupsCommands::AddMember { group_id, contact_id } => {
    commands::add_group_member(&group_id, &contact_id).await
}
GroupsCommands::RemoveMember { group_id, contact_id } => {
    commands::remove_group_member(&group_id, &contact_id).await
}
```

### Anti-Patterns to Avoid

- **Fetching group twice on add-member (one to validate, one to get current state):** `get_contact_by_id()` validation and the fetch in the retry loop are sequential. Do the contact validation once before the loop, then the loop fetches the group exactly once per attempt.
- **Mutating `member_uids` in place on `ContactGroup` and passing it by reference to serialize:** Clone the group struct, modify the clone's `member_uids` Vec, serialize the clone. The original group struct (with its known-good ETag) must remain unchanged for retry comparison.
- **Swallowing the idempotency case as an error:** Adding a contact that is already a member should succeed silently (return the current group unchanged), not error. Removing a contact that isn't a member should also succeed silently.
- **Using `get_group_by_name` directly in `add_group_member`:** The `CardDavClient` method should accept a group UID only. Name resolution (`resolve_group`) belongs in the command handler layer, not in the transport layer.
- **Returning `GqlGroupMutationResult` (the wrapper type) for membership mutations:** The context decision specifies `ContactGroup!` (direct type, not wrapped), matching how `get_group` query works. The existing `GqlGroupMutationResult` wrapper is used only for create/rename operations.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| ETag conflict detection | Custom HTTP response inspection | `map_group_write_response()` (already in `src/carddav/mod.rs`) | Already maps 412 -> `Error::GroupConflict`, tested |
| vCard serialization with MEMBER lines | String concatenation | `serialize_group_vcard()` (already in `src/carddav/mod.rs`) | Handles line folding, escaping, produces spec-compliant output |
| vCard parsing including MEMBER extraction | Custom line parser | `parse_group_vcard()` (called via `get_group_by_id()`) | Handles unfolding, urn:uuid stripping, already tested |
| Group lookup by name-or-ID | Inline search in handler | `resolve_group()` in `src/commands/contacts.rs` | Already handles GroupNotFound and GroupAmbiguous cases |
| Member contact resolution | Inline contact list fetch | `resolve_group_members()` on `CardDavClient` | Batch-fetches all contacts once, filters in-memory — avoids N+1 |
| JSON output formatting | Custom serde serialization | `Output::success()` + `serde_json::json!()` | Consistent output schema across all commands |
| HTTP client with basic auth | New reqwest Client | `self.client` (from `CardDavClient`) | Pre-configured with timeout, TLS, auth |

## Common Pitfalls

### Pitfall 1: ETag Staleness After Retry Fetch

**What goes wrong:** On a 412 retry, the code fetches a fresh group via `get_group_by_id()`. This new fetch returns a group with a new ETag. If the code re-uses the ETag from the *first* fetch, the retry will immediately 412 again.

**Why it happens:** The ETag variable is bound before the loop, then not updated from the new fetch.

**How to avoid:** Bind the ETag from the fetched group *inside* the loop, at the start of each iteration. Never carry an ETag from one loop iteration to the next.

**Warning signs:** Retry exhausted after 3 identical 412 responses with the same ETag value in all three.

### Pitfall 2: Duplicate Member UIDs on Concurrent Adds

**What goes wrong:** Two concurrent `add_group_member` calls for different contacts both read the same group vCard. Both succeed (different ETags if fast), and the second write wins — but the second writer's base state did not include the first writer's change. The first writer's contact is lost.

**Why it happens:** This is exactly the race condition the retry loop prevents. The retry loop works by re-fetching the group on 412, getting the version that includes the first writer's change, and then PUTting the combined state.

**How to avoid:** The retry loop must re-fetch the group (not just update the ETag) so it incorporates changes from other writers. The code pattern in Pattern 1 above handles this correctly.

**Warning signs:** Missing members after concurrent `add-member` calls — no error reported, but group `get` shows fewer members than expected.

### Pitfall 3: Idempotency Violation on Add

**What goes wrong:** Adding a contact that is already a member results in a duplicate `X-ADDRESSBOOKSERVER-MEMBER:urn:uuid:<uid>` line in the vCard, producing a corrupt group representation.

**Why it happens:** If `member_uids.push()` is called without checking for existence first, and the server or parser accepts duplicates silently.

**How to avoid:** Check `group.member_uids.contains(&contact_uid.to_string())` before pushing. Return `Ok(group)` early if already present.

**Warning signs:** `groups get` shows a contact listed twice in `members`, or `member_count` does not match `members.len()`.

### Pitfall 4: --group Flag Creates Contact Then Orphans It on Group-Not-Found

**What goes wrong:** If `--group <bad-id>` is passed and the contact creation succeeds, but the group lookup fails, the contact is left in the address book with no group assignment and no clear output about what happened.

**Why it happens:** Propagating the group-lookup error as a fatal error causes the handler to `bail!()`, and the output only shows an error — hiding the fact that the contact was successfully created.

**How to avoid:** The partial failure reporting pattern (Pattern 3 above) must catch both group-not-found and group-add-failure cases and emit `Output` with the contact data AND an `error` message. The contact was created — report that clearly.

**Warning signs:** User re-runs `contacts create --group` after failure and creates a duplicate contact.

### Pitfall 5: `get_group_by_id` Double-Fetch in ETag Retry

**What goes wrong:** The retry loop calls `get_group_by_id()` on each iteration, which calls `list_groups()` which calls `fetch_addressbook_groups()` for every address book. On a fast Fastmail account this is acceptable, but the overhead multiplies by 3 on full retry exhaustion.

**Why it happens:** `get_group_by_id` does not cache. This is by design for freshness, but the planner should be aware the retry loop is not zero-cost.

**How to avoid:** Accept this cost — 3 retries means at most 3 full REPORT roundtrips, which is acceptable for an interactive CLI. Do not introduce caching to work around it.

### Pitfall 6: `Output` Struct `error` Field Semantics

**What goes wrong:** Setting `success: true` AND `error: Some(...)` in the partial-failure case may not match the `Output` struct's intended semantics, leading to confusing JSON output or compilation failure.

**How to avoid:** Read `src/models/mod.rs` before implementing the partial-failure path to confirm the `Output` struct fields. The existing pattern in the codebase treats `error` as an optional diagnostic message that can accompany a partially-successful result. If the struct only supports `success: bool` + `data` + `error` (no combined case), use a custom `serde_json::json!()` literal instead.

## Code Examples

### Verified Pattern: How rename_group Does ETag-Guarded PUT (from `src/carddav/mod.rs`)

```rust
// Source: src/carddav/mod.rs lines 554-592
pub async fn rename_group(
    &self,
    href: &str,
    etag: &str,
    group: &ContactGroup,
    new_name: &str,
) -> Result<String> {
    let updated = ContactGroup {
        id: group.id.clone(),
        name: new_name.to_string(),
        member_uids: group.member_uids.clone(),  // preserve existing members
        href: group.href.clone(),
        etag: group.etag.clone(),
    };
    let url = format!("{}{}", self.base_url, href);
    let vcard = serialize_group_vcard(&updated);

    let response = self
        .client
        .put(&url)
        .basic_auth(&self.username, Some(&self.app_password))
        .header("Content-Type", "text/vcard; charset=utf-8")
        .header(IF_MATCH, etag)
        .body(vcard)
        .send()
        .await?;

    let status = response.status();
    let headers = response.headers().clone();
    let body = response.text().await?;

    let new_etag =
        map_group_write_response(&group.id, Some(etag), status, &headers, &body)?
            .unwrap_or_else(|| etag.to_string());
    Ok(new_etag)
}
```

`add_group_member` follows this exact pattern with a retry wrapper and `member_uids.push()`.

### Verified Pattern: How resolve_group Works (from `src/commands/contacts.rs`)

```rust
// Source: src/commands/contacts.rs lines 206-216
async fn resolve_group(
    client: &CardDavClient,
    id_or_name: &str,
) -> crate::error::Result<ContactGroup> {
    match client.get_group_by_id(id_or_name).await {
        Ok(group) => return Ok(group),
        Err(crate::error::Error::GroupNotFound(_)) => {}
        Err(e) => return Err(e),
    }
    client.get_group_by_name(id_or_name).await
}
```

Add-member CLI handler calls this first to get the canonical group UID before calling `client.add_group_member(&group.id, contact_id)`.

### Verified Pattern: How get_group Returns Resolved Members (from `src/commands/contacts.rs`)

```rust
// Source: src/commands/contacts.rs lines 251-265
pub async fn get_group(id_or_name: &str) -> AnyResult<()> {
    let client = contact_client()?;
    let group = resolve_group(&client, id_or_name).await?;
    let members = client.resolve_group_members(&group).await?;
    let data = serde_json::json!({
        "id": group.id,
        "name": group.name,
        "href": group.href,
        "etag": group.etag,
        "member_count": group.member_uids.len(),
        "members": members,
    });
    Output::success(data).print();
    Ok(())
}
```

`add_group_member` and `remove_group_member` command handlers emit the same JSON shape.

### Verified Pattern: MCP Mutation Returning GqlContactGroup with Members (from `src/mcp/graphql/query.rs`)

```rust
// Source: src/mcp/graphql/query.rs lines 198-215
async fn get_group(
    &self,
    ctx: &Context<'_>,
    #[graphql(desc = "Group ID (UID)")] id: String,
) -> Result<GqlContactGroup> {
    let app_ctx = ctx.data::<super::AppContext>()?;
    let carddav = app_ctx.get_carddav().await?;
    let group = carddav
        .get_group_by_id(&id)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
    let members = carddav
        .resolve_group_members(&group)
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
    Ok(GqlContactGroup::with_members(group, members))
}
```

MCP membership mutations return `GqlContactGroup` directly (not wrapped) — same pattern as above.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Custom ETag re-fetch on conflict | retry-on-412 loop (max 3) | Phase 19 design decision | Prevents silently dropped members on concurrent writes |
| vCard 4.0 KIND:group | vCard 3.0 X-ADDRESSBOOKSERVER-KIND:group | Phase 18 — Fastmail specific | Fastmail ignores KIND:group; must use X-ADDRESSBOOKSERVER prefix |

**Note:** The existing `rename_group` implementation does NOT have a retry loop (single ETag-guarded PUT). This is intentional for rename — a rename conflict is less likely and the user has the group name to retry with. Membership operations have the retry loop because concurrent adds are a realistic scenario (e.g., two AI agents adding different contacts simultaneously).

## Open Questions

1. **Output struct compatibility with partial-failure pattern**
   - What we know: `Output<T>` has `success: bool`, `data: Option<T>`, `error: Option<String>`, `message: Option<String>` (inferred from usage in `src/commands/contacts.rs`)
   - What's unclear: Whether the planner should use `Output` with `success: true` + `error: Some(...)`, or fall back to `serde_json::json!()` for the partial-failure case
   - Recommendation: Read `src/models/mod.rs` Output struct definition before implementing the `--group` partial-failure handler. If `success: true` + `error: Some(...)` is valid (structurally), use it. If not, use a raw JSON value.

2. **Whether `get_group_by_id` needs a direct-fetch variant**
   - What we know: Current `get_group_by_id` calls `list_groups()` which fetches ALL vCards from ALL address books, then scans. Works fine for single-user scenarios.
   - What's unclear: Whether a targeted single-resource GET (by href) would be more efficient for the retry loop's re-fetch.
   - Recommendation: Do not optimize yet. Current approach is correct. A targeted GET would require storing href separately, and the retry loop runs at most 3 times on a rare concurrent-conflict path.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — phase is pure code changes to existing Rust project using established Cargo dependencies).

## Validation Architecture

`nyquist_validation: false` in `.planning/config.json` — section skipped.

## Sources

### Primary (HIGH confidence)
- `src/carddav/mod.rs` (lines 1-636, 750-920, 1262-end) — existing group CRUD methods, ETag pattern, serialization/parsing, test infrastructure
- `src/commands/contacts.rs` (full file) — resolve_group, handler patterns, Output usage, create_contact_record
- `src/mcp/graphql/mutation.rs` (lines 1-411) — existing group mutations (create/rename/delete), AppContext pattern, error handling
- `src/mcp/graphql/query.rs` (lines 185-215) — get_group with member resolution — direct template for membership mutation return
- `src/mcp/graphql/types.rs` (lines 479-523) — GqlContactGroup struct, From<ContactGroup>, with_members constructor
- `src/main.rs` (lines 338-472, 895-972) — GroupsCommands enum, ContactsCommands::Create struct, dispatch patterns
- `src/error.rs` — GroupConflict, GroupNotFound, GroupAmbiguous error variants already defined
- `.planning/phases/19-group-membership-management/19-CONTEXT.md` — locked implementation decisions

### Secondary (MEDIUM confidence)
- `.planning/phases/18-group-data-model-crud-and-base-surfaces/18-CONTEXT.md` — Phase 18 design decisions that define the infrastructure Phase 19 builds on
- `.planning/REQUIREMENTS.md` — MBR-01/02/03, CLI-02/04, MCP-03 requirement definitions

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; all libraries verified as already in Cargo.toml
- Architecture patterns: HIGH — all patterns derived directly from existing Phase 18 code in the same files
- Pitfalls: HIGH — most pitfalls are mechanical consequences of the retry loop design and the partial-failure path, verified against actual code structure
- ETag retry semantics: HIGH — directly derived from `map_group_write_response()` return type and `rename_group()` as the template

**Research date:** 2026-04-13
**Valid until:** 2026-08-01 (stable Fastmail CardDAV protocol; no external API changes expected)
