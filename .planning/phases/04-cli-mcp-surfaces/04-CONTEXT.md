# Phase 4: CLI & MCP Surfaces - Context

**Gathered:** 2026-04-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Expose the phase-3 CardDAV write operations through two user-facing surfaces:

- CLI commands under `contacts create`, `contacts update`, and `contacts delete`
- MCP GraphQL mutations `createContact`, `updateContact`, and `deleteContact`

Delete must remain an explicit-confirmation flow in both surfaces.
</domain>

<decisions>
## Implementation Decisions

### Surface Shape
- CLI keeps the existing `contacts` namespace and adds `create`, `update`, and `delete` subcommands.
- CLI create requires `--name`; all other fields are optional single-value flags for v1.
- CLI update is partial-update only: omitted flags preserve existing contact data.

### Contact Resolution
- CLI and GraphQL update/delete resolve contacts by exact `Contact.id`, using phase-3 lookup helpers.
- Create defaults to the first discovered CardDAV addressbook.

### Delete Safety
- CLI requires `--confirm` or `--yes`.
- GraphQL uses a PREVIEW/CONFIRM token flow consistent with the existing send-email safety pattern.

### Return Shapes
- CLI responses stay JSON via the existing `Output<T>` wrapper and include a human-readable `message`.
- `createContact` and `updateContact` return the full resulting contact payload.
- `deleteContact` returns preview metadata or deletion confirmation data.
</decisions>

<code_context>
## Existing Code Insights

- `src/main.rs` already routes subcommands in a flat, explicit style.
- `src/commands/contacts.rs` is the natural shared home for reusable contact helper logic.
- `src/mcp/graphql/mutation.rs` already has preview/confirm patterns for destructive or high-stakes actions.
</code_context>

<specifics>
## Specific Ideas

- Reuse a shared `ContactInput` / `ContactPatch` model between CLI and GraphQL so update semantics cannot drift.
</specifics>

<deferred>
## Deferred Ideas

- repeatable email / phone flags
- explicit addressbook selection
- interactive CLI delete prompts
</deferred>
