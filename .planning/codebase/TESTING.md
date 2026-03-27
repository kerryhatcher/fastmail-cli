# Testing Patterns

**Analysis Date:** 2026-03-27

## Test Framework

**Runner:**
- Standard Rust `cargo test`
- Edition 2024 (from `Cargo.toml`)

**Assertion Library:**
- `assert!()`, `assert_eq!()`, `assert_ne!()` from stdlib
- `unwrap()` for panicking on errors in tests

**Run Commands:**
```bash
cargo test                    # Run all tests
cargo test --lib             # Run library tests only
cargo test -- --test-threads=1  # Run tests sequentially
cargo test 2>&1 | grep -E "test result"  # View summary only
```

## Test File Organization

**Location:**
- Co-located with implementation (not separate test directory)
- Tests at end of each module file in `#[cfg(test)]` block

**Naming:**
- Test functions prefixed with `test_`: `fn test_parse_single_email()`
- Test helper functions without prefix: `fn test_identity()`, `fn create_test_session()`
- Descriptive names matching what's being tested: `test_require_capability_succeeds_when_present()`

**Structure:**
Tests organized in `#[cfg(test)]` modules at bottom of implementation files:

```
src/
├── util.rs                    # 20 lines of test code
├── config.rs                  # 14 lines of test code
├── models/mod.rs              # 70 lines of test code
├── jmap/mod.rs                # 117 lines of test code
└── carddav/mod.rs             # Tests present but co-located
```

## Test Structure

**Suite Organization:**
Flat structure within `#[cfg(test)]` blocks — no nested describe/context functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_email() {
        // Arrange
        let result = parse_addresses("test@example.com");

        // Assert
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].email, "test@example.com");
        assert!(result[0].name.is_none());
    }
}
```

**Patterns:**
- Arrange-Act-Assert: Create test data, call function, assert results
- No explicit setup/teardown (functions are pure and side-effect free)
- Test isolation: Each test creates independent data structures
- Named test results: `let result = ...` then `assert_eq!(result...)`

## Mocking

**Framework:** No explicit mocking library used (no `mockall`, `mock_derive`, etc.)

**Patterns:**
Manual test double construction by setting struct fields directly:

```rust
#[test]
fn test_require_capability_succeeds_when_present() {
    let mut client = JmapClient::new("test-token".to_string());
    client.session = Some(create_test_session(vec![
        "urn:ietf:params:jmap:core",
        "urn:ietf:params:jmap:mail",
    ]));

    assert!(client
        .require_capability("urn:ietf:params:jmap:submission", "Email sending")
        .is_ok());
}
```

Helper functions build test instances with required state:
```rust
fn test_identity(id: &str, email: &str, name: &str) -> Identity {
    Identity {
        id: id.to_string(),
        name: name.to_string(),
        email: email.to_string(),
        reply_to: None,
        bcc: None,
        html_signature: None,
        text_signature: None,
        may_delete: true,
    }
}

fn create_test_session(capabilities: Vec<&str>) -> Session {
    // Returns Session with specified capabilities
}
```

**What to Mock:**
- JMAP client behavior: Set `session` field directly with test data
- API responses: Parse JSON strings directly via `serde_json::from_str()`
- Collections: Build Vec/HashMap manually with test data

**What NOT to Mock:**
- Serialization/deserialization: Use actual `serde_json::from_str()` (tests JSON round-tripping)
- Email address parsing: Test actual parsing logic with `parse_addresses()`
- Display implementations: Test actual `impl Display` formatting

## Fixtures and Factories

**Test Data:**
Helper functions create test instances:

```rust
// From jmap/mod.rs tests
fn test_identity(id: &str, email: &str, name: &str) -> Identity {
    // Returns fully constructed Identity
}

fn create_test_session(capabilities: Vec<&str>) -> Session {
    // Returns Session with HashMap of capabilities
}
```

Inline JSON strings for deserialization tests:
```rust
#[test]
fn test_mailbox_deserialize() {
    let json = r#"{
        "id": "mb1",
        "name": "Inbox",
        "role": "inbox",
        "totalEmails": 100,
        "unreadEmails": 5
    }"#;
    let mailbox: Mailbox = serde_json::from_str(json).unwrap();
    assert_eq!(mailbox.id, "mb1");
}
```

**Location:**
- In same file as tests: `#[cfg(test)]` block at bottom of module
- Organized as helper functions in test module
- No separate fixtures directory or files

## Coverage

**Requirements:** No explicit coverage enforcement

**View Coverage:**
Not currently configured — would use:
```bash
cargo tarpaulin --out Html  # If tarpaulin were added
```

## Test Types

**Unit Tests:**
- Scope: Individual functions and small components
- Approach: Testing pure functions and simple logic in isolation
- Examples:
  - Email address parsing: `test_parse_single_email()`, `test_parse_mixed_formats()`
  - Boolean model methods: `test_email_is_unread()`, `test_email_is_flagged()`
  - Serialization: `test_session_deserialize()`, `test_mailbox_deserialize()`
  - Identity selection: `test_pick_identity_*()` family of tests
  - Configuration: `test_config_set_token()`, `test_config_serialize_deserialize()`

**Integration Tests:**
- Not explicitly separated or tested
- JMAP client methods would need HTTP mocking to test (currently no mock server)
- CLI commands use real authenticated client (integration-style in practice)

**E2E Tests:**
- Not present in codebase
- CLI would require Fastmail API credentials to test end-to-end
- Manual testing is primary E2E approach

## Common Patterns

**Async Testing:**
Tests use synchronous assertions on synchronous helper functions:
```rust
// Most tests are sync because they test data types and pure functions
#[test]
fn test_output_success() {
    let output: Output<&str> = Output::success("test data");
    assert!(output.success);
}
```

Async functions in JMAP client are not directly unit tested (require HTTP mocking).

**Error Testing:**
Test error cases by checking `is_err()` or `unwrap_err()`:

```rust
#[test]
fn test_require_capability_fails_when_missing() {
    let mut client = JmapClient::new("test-token".to_string());
    client.session = Some(create_test_session(vec!["urn:ietf:params:jmap:core"]));

    let result = client.require_capability("urn:ietf:params:jmap:submission", "Email sending");
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("urn:ietf:params:jmap:submission"));
    assert!(err_msg.contains("read-only"));
}
```

Verify error messages contain specific context using string assertions.

**Display/Format Testing:**
Test custom `impl Display` for types:

```rust
#[test]
fn test_email_address_display_with_name() {
    let addr = EmailAddress {
        name: Some("John Doe".to_string()),
        email: "john@example.com".to_string(),
    };
    assert_eq!(format!("{}", addr), "John Doe <john@example.com>");
}

#[test]
fn test_email_address_display_without_name() {
    let addr = EmailAddress {
        name: None,
        email: "john@example.com".to_string(),
    };
    assert_eq!(format!("{}", addr), "john@example.com");
}
```

## Test Coverage by Module

**`src/util.rs`** (59 total lines):
- 8 tests for `parse_addresses()` covering:
  - Single email
  - Multiple emails
  - Emails with display names
  - Mixed formats
  - Empty strings
  - Whitespace handling
  - Angle brackets without names
- No tests for image/text processing (would require file I/O or binary data)

**`src/config.rs`** (158 total lines):
- 4 tests:
  - `test_config_default()`: Default config has no token
  - `test_config_get_token_some()`: Token present in config
  - `test_config_set_token()`: Token can be set
  - `test_config_serialize_deserialize()`: TOML round-trip works

**`src/models/mod.rs`** (434 total lines):
- 10 tests:
  - Display formatting: `test_email_address_display_*()` (3 variants)
  - Email state: `test_email_is_unread()`, `test_email_is_flagged()`
  - Output struct: `test_output_success()`, `test_output_error()`
  - Deserialization: `test_session_deserialize()`, `test_mailbox_deserialize()`, `test_masked_email_deserialize()`

**`src/jmap/mod.rs`** (1367 total lines):
- 8 tests organized in two groups:
  - Capability checking: `test_require_capability_*()` (4 variants)
  - Identity selection: `test_pick_identity_*()` (4 variants)
- Helper functions: `test_identity()`, `create_test_session()`

**`src/carddav/mod.rs`** (463 total lines):
- Tests present but details not examined in full detail
- Likely tests for XML parsing and CardDAV response handling

## Gaps and Considerations

**Not Tested:**
- Async/await functions in JMAP client (would need HTTP mocking)
- CLI command execution (would need integration test framework)
- Actual HTTP requests (integration tests only)
- File I/O operations in util.rs
- Error cases in async code paths
- CardDAV XML parsing (if not extensively covered)

**Why:**
- Unit tests focus on pure, testable functions
- Async code requires infrastructure (tokio runtime, HTTP mocks) not set up in current test suite
- CLI testing would require full environment setup

---

*Testing analysis: 2026-03-27*
