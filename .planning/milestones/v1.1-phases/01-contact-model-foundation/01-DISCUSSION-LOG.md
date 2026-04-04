# Phase 1: Contact Model Foundation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md -- this log preserves the alternatives considered.

**Date:** 2026-03-27
**Phase:** 01-contact-model-foundation
**Areas discussed:** Contact identification, Error detail level, Field visibility

---

## Contact Identification

| Option | Description | Selected |
|--------|-------------|----------|
| UID as user-facing ID | Users pass vCard UID; CLI resolves UID to href internally via REPORT call. Simpler UX, hides CardDAV internals. Costs one extra REPORT call per write. | ✓ |
| href as user-facing ID | Users pass full server href path. No extra lookup, but exposes CardDAV internals. | |
| Both -- UID default, href flag | Default to UID lookup with `--href` flag for power users. Best of both, slightly more complex CLI surface. | |

**User's choice:** UID as user-facing ID (Recommended)
**Notes:** None -- straightforward selection

---

## Error Detail Level

| Option | Description | Selected |
|--------|-------------|----------|
| ID only | Match existing `MailboxNotFound(String)` pattern. Simple, consistent. Error message tells user to re-fetch and retry. | |
| ID + stale ETag | Carry the sent ETag for debugging: struct variant with `id` and `sent_etag`. More diagnostic info but breaks single-string pattern. | |
| Structured with both ETags | Include sent ETag and server ETag. Maximum debug info for automated retry logic. Heaviest variant. | ✓ |

**User's choice:** Structured with both ETags
**Notes:** User chose the richest diagnostic option for maximum debugging and retry support

---

## Field Visibility

| Option | Description | Selected |
|--------|-------------|----------|
| Always visible | Include href and etag in all JSON output. Useful for scripting, debugging, power users. Consistent exposure. | ✓ |
| Hidden by default, --verbose flag | Use `#[serde(skip)]` normally, include only with verbose/debug flag. Cleaner default but requires conditional serialization. | |
| Internal only | Never expose in JSON output. `#[serde(skip_serializing)]` on both fields. Clean output but limits debugging. | |

**User's choice:** Always visible (Recommended)
**Notes:** None -- straightforward selection

---

## Claude's Discretion

- REPORT XML parsing implementation details
- Whether href/etag fields use `Option<String>` or `String`
- Test structure and specific test cases

## Deferred Ideas

None -- discussion stayed within phase scope
