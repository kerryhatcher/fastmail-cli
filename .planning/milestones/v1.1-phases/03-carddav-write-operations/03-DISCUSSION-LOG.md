# Phase 3: CardDAV Write Operations - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md -- this log preserves the alternatives considered.

**Date:** 2026-03-27
**Phase:** 03-carddav-write-operations
**Areas discussed:** Address book targeting, Contact resolution for writes, Write method signatures, Testing approach
**Mode:** Auto (all areas auto-selected, recommended defaults chosen)

---

## Address Book Targeting

| Option | Description | Selected |
|--------|-------------|----------|
| Accept addressbook_href parameter | Caller provides address book URL, consistent with list_contacts() | ✓ |
| Auto-discover default address book | Method calls list_addressbooks() internally and picks first | |
| Accept optional addressbook_href with fallback | Optional param, auto-discover if not provided | |

**User's choice:** [auto] Accept addressbook_href parameter (recommended default)
**Notes:** Consistent with existing list_contacts() API pattern. Keeps write methods simple. Phase 4 CLI handles address book selection logic.

---

## Contact Resolution for Writes

| Option | Description | Selected |
|--------|-------------|----------|
| Require href and etag as parameters | Caller passes href/etag from a prior fetch; method stays stateless | ✓ |
| Accept UID, method resolves internally | Method calls list_contacts + filter to find href/etag | |
| Accept Contact struct reference | Pass entire Contact struct, extract href/etag from it | |

**User's choice:** [auto] Require href and etag as parameters (recommended default)
**Notes:** Follows the data model where Contact already carries href/etag. Keeps write methods simple and avoids implicit network calls. Caller (Phase 4) already has the Contact from a prior operation.

---

## Write Method Signatures

| Option | Description | Selected |
|--------|-------------|----------|
| Return new href + etag from create, new etag from update, () from delete | Gives callers enough for follow-up operations without re-fetch | ✓ |
| Return full Contact from create/update, () from delete | Requires parsing the server response as vCard | |
| Return just Result<()> for all | Simple but callers must re-fetch for updated etag | |

**User's choice:** [auto] Return href+etag / etag / () (recommended default)
**Notes:** Returns server-assigned metadata (etag) from response headers. Avoids unnecessary re-fetch. Delete has nothing meaningful to return.

---

## Testing Approach

| Option | Description | Selected |
|--------|-------------|----------|
| Unit tests validating request construction and error mapping | No live API; test logic paths with expected inputs/outputs | ✓ |
| Integration tests with Fastmail sandbox | Live API calls against test account | |
| Mock HTTP server (wiremock/mockito crate) | Full HTTP mock server for realistic testing | |

**User's choice:** [auto] Unit tests with request/response validation (recommended default)
**Notes:** Per project memory constraint: no live API calls without explicit permission. Mock HTTP server crate would add a test dependency -- simpler to test logic directly.

---

## Claude's Discretion

- Return type naming (struct vs tuple) for create_contact
- Content-Type header value for PUT requests
- ETag extraction strategy from 412 responses
- Internal helper patterns for authenticated requests
- Test fixture structure

## Deferred Ideas

None -- discussion stayed within phase scope
