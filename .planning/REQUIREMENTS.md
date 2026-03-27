# Requirements: Contact CRUD via CardDAV

**Defined:** 2026-03-27
**Core Value:** Users can manage contacts (create, update, delete) without leaving the terminal or AI assistant

## v1 Requirements

### Contact Model

- [ ] **MOD-01**: Contact struct includes `href` (resource URL) and `etag` fields populated from REPORT responses
- [ ] **MOD-02**: Error type includes ContactConflict (412) and ContactNotFound variants

### vCard Serialization

- [ ] **VCARD-01**: Generate valid vCard 3.0 with FN, N, EMAIL, ORG, TEL, ADR, NOTE properties
- [ ] **VCARD-02**: Line folding at 75 octets with CRLF line endings per RFC 6350
- [ ] **VCARD-03**: UUID v4 generation for new contact UIDs

### CardDAV Write Operations

- [ ] **DAV-01**: Create contact via PUT with `If-None-Match: *` to discovered address book URL
- [ ] **DAV-02**: Update contact via read-modify-write PUT with `If-Match` ETag guard
- [ ] **DAV-03**: Delete contact via DELETE with `If-Match` ETag guard
- [ ] **DAV-04**: Address book discovery to determine target URL for create operations

### CLI Commands

- [ ] **CLI-01**: `contacts create` with --name, --email, --organization, --phone, --address, --notes flags
- [ ] **CLI-02**: `contacts update CONTACT_ID` with partial field flags (only modify fields passed)
- [ ] **CLI-03**: `contacts delete CONTACT_ID` requiring --confirm or --yes flag

### MCP GraphQL Mutations

- [ ] **MCP-01**: `createContact` mutation with name, email, organization, phone, address, notes inputs
- [ ] **MCP-02**: `updateContact` mutation with id and optional field inputs
- [ ] **MCP-03**: `deleteContact` mutation using PREVIEW/CONFIRM token pattern

## v2 Requirements

### Multi-Value Fields

- **MULTI-01**: Repeatable --email flag for multiple email addresses
- **MULTI-02**: Repeatable --phone flag for multiple phone numbers
- **MULTI-03**: TYPE parameters for email/phone (work, home, mobile)

### Address Book Selection

- **ABOOK-01**: User can specify target address book for create operations
- **ABOOK-02**: List available address books

## Out of Scope

| Feature | Reason |
|---------|--------|
| Contact groups/categories | Adds complexity, not requested |
| Contact photo/avatar upload | Binary data handling, separate concern |
| Batch operations (bulk create/delete) | Can add later if needed |
| Contact import/export (CSV, vCard file) | Separate feature |
| Interactive delete prompts | Using flag-based confirmation instead |
| vCard 4.0 support | Fastmail uses vCard 3.0 |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| MOD-01 | Phase ? | Pending |
| MOD-02 | Phase ? | Pending |
| VCARD-01 | Phase ? | Pending |
| VCARD-02 | Phase ? | Pending |
| VCARD-03 | Phase ? | Pending |
| DAV-01 | Phase ? | Pending |
| DAV-02 | Phase ? | Pending |
| DAV-03 | Phase ? | Pending |
| DAV-04 | Phase ? | Pending |
| CLI-01 | Phase ? | Pending |
| CLI-02 | Phase ? | Pending |
| CLI-03 | Phase ? | Pending |
| MCP-01 | Phase ? | Pending |
| MCP-02 | Phase ? | Pending |
| MCP-03 | Phase ? | Pending |

**Coverage:**
- v1 requirements: 15 total
- Mapped to phases: 0
- Unmapped: 15 ⚠️

---
*Requirements defined: 2026-03-27*
*Last updated: 2026-03-27 after initial definition*
