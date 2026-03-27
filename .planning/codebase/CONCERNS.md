# Codebase Concerns

**Analysis Date:** 2026-03-27

## Tech Debt

**Unwrap/Expect Usage:**
- Issue: Multiple instances of `.unwrap()`, `.unwrap_or()`, and `.expect()` that could panic if assumptions are violated
- Files:
  - `src/jmap/mod.rs` (lines 183, 219, 287, 290, 294, 298, 343, 407, 437, 479, 507, 615, 633, 668, 676, 680, 698, 702, 822, 830, 834)
  - `src/commands/download.rs` (lines 18, 27, 36, 38, 56, 59, 68, 73, 80, 96)
  - `src/models/mod.rs` (lines 394, 408, 426)
- Impact: Program could panic in edge cases rather than gracefully handle errors. Production crashes from API response variations.
- Fix approach: Replace `.unwrap()` calls with proper error handling using `?` operator or `.ok_or()`. Most critical: `src/jmap/mod.rs:183` (HTTP client builder), `src/commands/download.rs:18,27,59` (attachment handling).

**Potential Panics from XML Parsing:**
- Issue: In `src/carddav/mod.rs`, vCard parsing with `rfind()` on split iterator could fail silently or panic with `.unwrap()`
- Files: `src/carddav/mod.rs` (lines 135-140)
- Impact: Malformed CardDAV responses or vCard data could cause crashes instead of graceful degradation
- Fix approach: Use proper error handling for XML parsing and vCard line extraction. Return `Result<Contact>` from `parse_vcard()` instead of `Option`.

**Double Unwrap Pattern:**
- Issue: Pattern of checking then immediately unwrapping (`attachments.is_none() || attachments.unwrap()`) in download command
- Files: `src/commands/download.rs` (lines 18, 27, 59)
- Impact: Poor code quality, potential panic if attachment state changes between checks
- Fix approach: Use `if let Some(atts) = attachments` pattern or refactor to eliminate the redundant check

## Security Considerations

**Confirmation Token in GraphQL Mutations:**
- Risk: The confirmation token implementation uses simple hash-based token (`confirmation_token()` from `to`, `subject`, `body`). While this provides basic protection against accidental re-submissions, it may not be cryptographically secure for truly sensitive operations.
- Files: `src/mcp/graphql/mutation.rs` (lines 33, 47), `src/mcp/graphql/types.rs`
- Current mitigation: Token validation prevents reuse without explicit user confirmation
- Recommendations: This is acceptable for CLI usage but document the limitations. Consider using cryptographic tokens (HMAC-SHA256) if this becomes a service endpoint.

**Credentials Stored in Plain Text:**
- Risk: API tokens and app passwords stored in `~/.config/fastmail-cli/config.toml` without encryption
- Files: `src/config.rs` (lines 51-72)
- Current mitigation: File permissions set to 0o600 on Unix systems; environment variable fallback available
- Recommendations: Add optional encryption support for sensitive values in config file. Document the security implications clearly.

**CardDAV App Password Requirements:**
- Risk: CardDAV doesn't support API tokens and requires an "app password", which must be stored separately from the main API token
- Files: `src/carddav/mod.rs`, `src/config.rs`
- Current mitigation: Separate config section, clear documentation needed
- Recommendations: Provide setup guidance emphasizing the security implications of app passwords vs API tokens.

## Performance Bottlenecks

**N+1 Contact Lookup in Search:**
- Problem: `search_contacts()` fetches all contacts from all address books to perform client-side filtering
- Files: `src/carddav/mod.rs` (lines 220-245)
- Cause: No server-side filtering available in CardDAV; fetches entire address book list then all contacts in each
- Improvement path: Cache address book contents with TTL, implement local search index, or document limitation

**No Request Batching for Multiple Email Fetches:**
- Problem: Email operations may result in sequential HTTP requests to JMAP server
- Files: `src/jmap/mod.rs` (various get/fetch methods)
- Cause: Individual method calls not grouped into batch requests
- Improvement path: JMAP supports multiple method calls per request. Consider batching related operations like fetching multiple emails or mailbox lists.

**XML Parsing on Every vCard Access:**
- Problem: Each contact list operation re-parses XML responses instead of caching
- Files: `src/carddav/mod.rs` (lines 193-216)
- Cause: No caching layer between CardDAV responses and Contact parsing
- Improvement path: Implement optional caching with TTL for address book lists and contact data

**Kreuzberg Async Extraction Unbounded:**
- Problem: Text extraction from attachments using kreuzberg can be slow and unbounded for large files
- Files: `src/util.rs` (lines 38-63)
- Cause: No timeout, no file size limits, no streaming
- Improvement path: Add timeouts, implement file size checks before extraction, consider streaming for very large files

## Known Bugs

**Recent Fix: XML Parsing with roxmltree:**
- Symptoms: XML parsing failures in CardDAV PROPFIND/REPORT responses
- Files: `src/carddav/mod.rs` (lines 107, 194)
- Trigger: Certain XML namespace handling or malformed responses
- Status: Fixed in commit 1d4e499 (roxmltree XML parsing improvements)
- Workaround: No longer needed; upgrade to latest version

**Recent Fix: vCard Line Folding:**
- Symptoms: Multi-line vCard properties not parsed correctly
- Files: `src/carddav/mod.rs` (lines 248-263)
- Trigger: vCard entries with wrapped lines (RFC 6350 compliance)
- Status: Fixed in commit 1d4e499 with proper `unfold_vcard()` implementation
- Workaround: No longer needed; upgrade to latest version

**Recent Fix: MCP Confirmation Token Guard:**
- Symptoms: MCP mutations could be executed without proper confirmation
- Files: `src/mcp/graphql/mutation.rs` (lines 47-58)
- Trigger: Missing or invalid confirmation_token check
- Status: Fixed in commit 1d4e499 with stricter validation
- Workaround: No longer needed; upgrade to latest version

## Fragile Areas

**JMAP Response Parsing:**
- Files: `src/jmap/mod.rs` (lines 277-315)
- Why fragile: Complex JSON path navigation with multiple unwraps; assumes specific response structure. If JMAP API response format changes slightly, parsing could fail.
- Safe modification: Add comprehensive tests for various JMAP error responses (malformed, missing fields, wrong method name). Consider using a more robust JSON validation library.
- Test coverage: Basic tests exist but should expand error cases

**Email Creation and Submission:**
- Files: `src/jmap/mod.rs` (lines 666-723, 727-773)
- Why fragile: Two-stage process (create email, submit for sending) with complex error handling. Email creation can succeed while submission fails.
- Safe modification: The `parse_email_create_response()` logic is complex with nested Option chains. Add integration tests confirming both success and partial failure paths.
- Test coverage: Critical path should have end-to-end tests

**CardDAV Contact Parsing:**
- Files: `src/carddav/mod.rs` (lines 298-350+, not fully visible)
- Why fragile: vCard parsing relies on pattern matching with specific field names and formats. Malformed vCard data could silently skip fields without error.
- Safe modification: Validate that required fields (UID, FN) are present before returning Contact
- Test coverage: Test with edge cases (missing fields, duplicate values, encoding issues)

**Attachment Handling in Download Command:**
- Files: `src/commands/download.rs` (lines 6-135)
- Why fragile: Multiple unwraps, assumes attachment structure; image mime type inference is hardcoded
- Safe modification: Refactor to use proper error handling. Test with various attachment types and missing fields.
- Test coverage: Missing; needs tests for various content types and edge cases

## Test Coverage Gaps

**JMAP Client Integration:**
- What's not tested: Actual HTTP requests to JMAP API, authentication token refresh, rate limiting behavior
- Files: `src/jmap/mod.rs` (entire module)
- Risk: Silent failures in production; error handling paths not validated
- Priority: High

**CardDAV Parsing:**
- What's not tested: Malformed XML responses, incomplete vCard data, special characters in names
- Files: `src/carddav/mod.rs`
- Risk: Production crashes with unusual contact data
- Priority: High

**Email Composition:**
- What's not tested: Reply/forward threading headers, CC/BCC handling, draft vs send distinction
- Files: `src/jmap/mod.rs` send/reply/forward methods
- Risk: Incorrect email headers or wrong mailbox placement
- Priority: Medium

**Configuration:**
- What's not tested: File permission validation, corrupt TOML, missing directories
- Files: `src/config.rs` (has some tests but incomplete)
- Risk: Silent configuration failures
- Priority: Medium

**Utility Functions:**
- What's not tested: Email address parsing, image detection, MIME type inference
- Files: `src/util.rs`, `src/commands/download.rs`
- Risk: Incorrect parsing with unusual input formats
- Priority: Medium

## Scaling Limits

**In-Memory Contact Lists:**
- Current capacity: All contacts loaded into memory for searching
- Limit: Fastmail accounts with thousands of contacts could cause memory issues
- Scaling path: Implement pagination and server-side filtering in CardDAV, or use streaming deser

**Large File Handling:**
- Current capacity: Downloaded blobs held entirely in memory
- Limit: Very large attachments (>100MB) could exhaust available memory
- Scaling path: Implement streaming download to disk with progress reporting

**JMAP Query Result Size:**
- Current capacity: Email lists limited by JMAP response size constraints
- Limit: Searching large mailboxes could return thousands of results
- Scaling path: Implement cursor-based pagination, add result limiting

## Dependencies at Risk

**Kreuzberg (Text Extraction):**
- Risk: Large, complex dependency for document text extraction. Bundled PDFium adds 50+ MB to binary.
- Impact: Binary size, compilation time, potential security vulnerabilities in PDF parsing
- Migration plan: Could be made optional feature, or replaced with more minimal extraction library for common formats

**async-graphql:**
- Risk: GraphQL framework dependency for MCP server. Large surface area.
- Impact: Adds complexity to MCP server. If GraphQL implementation has vulnerabilities, affects all users.
- Migration plan: Stable, widely-used library. Monitor for security updates.

**roxmltree:**
- Risk: XML parsing for CardDAV. Recently fixed issues with namespace handling.
- Impact: CardDAV operations fail if XML parsing breaks
- Migration plan: Current version stable after recent fixes (0.21.1). Monitor for updates.

## Missing Critical Features

**No Offline Support:**
- Problem: All operations require live JMAP connection; no local caching of emails or contacts
- Blocks: Offline email viewing, local search

**No Encryption for Stored Config:**
- Problem: Credentials stored in plaintext on disk (with filesystem permissions only)
- Blocks: Secure use on shared systems

**No Email Attachment Preview Before Processing:**
- Problem: Text extraction happens on download but can't preview before large operations
- Blocks: Users can't review attachment contents before processing

---

*Concerns audit: 2026-03-27*
