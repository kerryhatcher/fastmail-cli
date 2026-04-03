# Milestones

## v1.0 milestone (Shipped: 2026-04-03)

**Phases completed:** 4 phases, 4 plans, 2 tasks

**Key accomplishments:**

- Contact struct extended with server-assigned href/etag fields for CardDAV write operations, plus ContactNotFound and ContactConflict error variants for Phase 2-4 write operation error handling
- CardDAV write operations now support create, update, and delete with correct conditional header handling and error mapping.
- CLI and MCP now expose contact create, update, and delete flows with shared partial-update logic and explicit delete confirmation.

---
