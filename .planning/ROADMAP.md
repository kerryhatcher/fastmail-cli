# Roadmap: Fastmail CLI

## Milestones

- ✅ **v1.0 Contact CRUD** — Phases 1-4 (shipped 2026-04-03)
- ✅ **v1.1 Calendar Access and Management** — Phases 5-11 (shipped 2026-04-04)
- ✅ **v1.2 Hardening & Quality** — Phases 12-17 (shipped 2026-04-05)
- 🚧 **v1.3 Contact Groups** — Phases 18-19 (in progress)

## Phases

<details>
<summary>✅ v1.0 Contact CRUD (Phases 1-4) — SHIPPED 2026-04-03</summary>

- [x] Phase 1: Contact Model Foundation (1/1 plans) — completed 2026-04-03
- [x] Phase 2: vCard Serialization (1/1 plans) — completed 2026-04-03
- [x] Phase 3: CardDAV Write Operations (1/1 plans) — completed 2026-04-03
- [x] Phase 4: CLI & MCP Surfaces (1/1 plans) — completed 2026-04-03

</details>

<details>
<summary>✅ v1.1 Calendar Access and Management (Phases 5-11) — SHIPPED 2026-04-04</summary>

- [x] Phase 5: CalDAV Foundation & Discovery (1/1 plans) — completed 2026-04-03
- [x] Phase 6: iCalendar Event Semantics (1/1 plans) — completed 2026-04-03
- [x] Phase 7: Calendar & Event CRUD Transport (1/1 plans) — completed 2026-04-03
- [x] Phase 8: CLI Calendar Experience (1/1 plans) — completed 2026-04-03
- [x] Phase 9: MCP Calendar Surface & Live Validation (1/1 plans) — completed 2026-04-03
- [x] Phase 10: Explicit Range Contract Closure (1/1 plans) — completed 2026-04-03
- [x] Phase 11: CLI Attendee Clearing Parity (1/1 plans) — completed 2026-04-03

</details>

<details>
<summary>✅ v1.2 Hardening & Quality (Phases 12-17) — SHIPPED 2026-04-05</summary>

- [x] Phase 12: Foundation Safety (4/4 plans) — completed 2026-04-04
- [x] Phase 13: Security Hardening (4/4 plans) — completed 2026-04-04
- [x] Phase 14: MCP Layer Refactor (4/4 plans) — completed 2026-04-04
- [x] Phase 15: Performance (4/4 plans) — completed 2026-04-05
- [x] Phase 16: Integration Test Coverage (4/4 plans) — completed 2026-04-05
- [x] Phase 17: Quality Polish (3/3 plans) — completed 2026-04-05

Full details: `.planning/milestones/v1.2-ROADMAP.md`

</details>

### 🚧 v1.3 Contact Groups (In Progress)

**Milestone Goal:** Users can manage contact groups (create, list, get, rename, delete), add/remove contacts from groups, and assign a group at contact creation time — via both CLI and MCP/GraphQL.

- [x] **Phase 18: Group Data Model, CRUD, and Base Surfaces** - ContactGroup struct, vCard 3.0 X-ADDRESSBOOKSERVER serialization, parser KIND-filter, and full group CRUD via CLI and MCP (completed 2026-04-14)
- [ ] **Phase 19: Group Membership Management** - ETag-guarded add/remove member with retry, CLI membership commands, and --group flag on contacts create with two-step atomic sequencing

## Phase Details

### Phase 18: Group Data Model, CRUD, and Base Surfaces
**Goal**: Users can create, list, inspect, rename, and delete contact groups — and existing `contacts list` is unaffected by group vCards
**Depends on**: Phase 17 (v1.2 complete)
**Requirements**: GRP-01, GRP-02, GRP-03, GRP-04, GRP-05, CLI-01, CLI-03, MCP-01, MCP-02
**Success Criteria** (what must be TRUE):
  1. User can run `contacts groups list` and see all contact groups with name, member count, and group ID
  2. User can run `contacts groups create <name>` and the new empty group appears in subsequent list output
  3. User can run `contacts groups get <id>` and see the group's name, ID, and resolved member contacts
  4. User can run `contacts groups rename <id> <new-name>` and the rename is reflected in subsequent get/list output
  5. User can run `contacts groups delete <id> --confirm` and the group is removed; running without `--confirm` is rejected; `contacts list` never shows group vCards as malformed contacts
**Plans:** 3/3 plans complete
Plans:
- [x] 18-01-PLAN.md — ContactGroup data model, vCard parse/serialize, CardDAV CRUD methods
- [x] 18-02-PLAN.md — CLI surface: contacts groups subcommands
- [x] 18-03-PLAN.md — MCP/GraphQL surface: group queries and mutations

### Phase 19: Group Membership Management
**Goal**: Users can add and remove contacts from groups, and assign a new contact to a group in a single `contacts create --group` invocation
**Depends on**: Phase 18
**Requirements**: MBR-01, MBR-02, MBR-03, CLI-02, CLI-04, MCP-03
**Success Criteria** (what must be TRUE):
  1. User can run `contacts groups add-member <group-id> <contact-id>` and the contact appears in subsequent `contacts groups get` output
  2. User can run `contacts groups remove-member <group-id> <contact-id>` and the contact is absent from subsequent `contacts groups get` output
  3. Concurrent `add-member` calls against the same group produce a correct final member list (no silently dropped members due to ETag race)
  4. User can run `contacts create --group <group-id> ...` and the new contact is created and added to the group in one command; partial failure (contact created but group update fails) is reported clearly
  5. AI agent can call `addContactGroupMember` and `removeContactGroupMember` mutations and receive the updated ContactGroup with resolved members
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Contact Model Foundation | v1.0 | 1/1 | Complete | 2026-04-03 |
| 2. vCard Serialization | v1.0 | 1/1 | Complete | 2026-04-03 |
| 3. CardDAV Write Operations | v1.0 | 1/1 | Complete | 2026-04-03 |
| 4. CLI & MCP Surfaces | v1.0 | 1/1 | Complete | 2026-04-03 |
| 5. CalDAV Foundation & Discovery | v1.1 | 1/1 | Complete | 2026-04-03 |
| 6. iCalendar Event Semantics | v1.1 | 1/1 | Complete | 2026-04-03 |
| 7. Calendar & Event CRUD Transport | v1.1 | 1/1 | Complete | 2026-04-03 |
| 8. CLI Calendar Experience | v1.1 | 1/1 | Complete | 2026-04-03 |
| 9. MCP Calendar Surface & Live Validation | v1.1 | 1/1 | Complete | 2026-04-03 |
| 10. Explicit Range Contract Closure | v1.1 | 1/1 | Complete | 2026-04-03 |
| 11. CLI Attendee Clearing Parity | v1.1 | 1/1 | Complete | 2026-04-03 |
| 12. Foundation Safety | v1.2 | 4/4 | Complete | 2026-04-04 |
| 13. Security Hardening | v1.2 | 4/4 | Complete | 2026-04-04 |
| 14. MCP Layer Refactor | v1.2 | 4/4 | Complete | 2026-04-04 |
| 15. Performance | v1.2 | 4/4 | Complete | 2026-04-05 |
| 16. Integration Test Coverage | v1.2 | 4/4 | Complete | 2026-04-05 |
| 17. Quality Polish | v1.2 | 3/3 | Complete | 2026-04-05 |
| 18. Group Data Model, CRUD, and Base Surfaces | v1.3 | 3/3 | Complete    | 2026-04-14 |
| 19. Group Membership Management | v1.3 | 0/? | Not started | - |
