# Coding Conventions

**Analysis Date:** 2026-03-27

## Naming Patterns

**Files:**
- Lowercase with underscores: `send.rs`, `get.rs`, `config.rs`, `carddav.rs`
- Reserved keyword escaping when needed: `r#move.rs` (uses raw identifier prefix)
- Module structure uses `mod.rs` as entry point for submodules

**Functions:**
- snake_case for all functions: `parse_addresses()`, `authenticated_client()`, `get_email()`, `extract_text()`
- Async functions use `async fn` keyword: `async fn authenticate()`, `async fn send_email()`
- Private helper functions with leading underscore for internal use

**Variables:**
- snake_case throughout: `email_id`, `mailbox_id`, `token`, `params`, `body`, `content_type`
- Short loop variables accepted: `e` for element in filter, `f` for first in address parsing
- Boolean variables use clear names: `is_addressbook`, `is_image`, `has_attachment`, `is_unread`

**Types:**
- PascalCase for all structs: `JmapClient`, `EmailAddress`, `Session`, `Mailbox`, `CardDavClient`
- PascalCase for enums: `Commands`, `ListCommands`, `Error`
- Type aliases for result types: `pub type Result<T> = std::result::Result<T, Error>;`

**Struct Fields:**
- snake_case with serde rename for API conversions:
  ```rust
  #[serde(rename_all = "camelCase")]
  pub struct Session {
      pub primary_accounts: HashMap<String, String>,  // Maps to primaryAccounts in JSON
      pub api_url: String,                             // Maps to apiUrl in JSON
  }
  ```
- Optional fields use `Option<T>`: `pub parent_id: Option<String>`
- Boolean flags use explicit naming: `is_personal`, `is_read_only`, `may_delete`

## Code Style

**Formatting:**
- No explicit formatter config (uses Rust defaults)
- Standard Rust formatting conventions
- Line continuation with proper indentation in match statements
- Comments use `//` for single line, `///` for documentation comments

**Linting:**
- No explicit `.clippy.toml` or `.rustfmt.toml` (uses Rust defaults)
- Code follows Clippy conventions and passes clippy checks (visible in CI)

## Import Organization

**Order:**
1. Crate imports from local modules: `use crate::jmap::*`, `use crate::models::*`, `use crate::error::*`
2. Standard library: `use std::collections::HashMap`, `use std::fs`
3. Third-party crates: `use serde::{Deserialize, Serialize}`, `use reqwest::Client`, `use tokio`
4. Re-exports at module level: `pub use auth::*`, `pub use send::*`

**Path Aliases:**
- No custom path aliases configured in `Cargo.toml`
- All imports use standard absolute crate paths
- Re-exports in `mod.rs` files make internal modules available: `pub use commands::*`

## Error Handling

**Patterns:**
- Custom error enum using `thiserror` crate in `src/error.rs`:
  ```rust
  #[derive(Error, Debug)]
  pub enum Error {
      #[error("Authentication required...")]
      NotAuthenticated,
      #[error("HTTP error: {0}")]
      Http(#[from] reqwest::Error),
  }
  ```
- Type alias `pub type Result<T> = std::result::Result<T, Error>;` used throughout
- Function signatures use `-> anyhow::Result<T>` in command handlers (lighter-weight for CLIs)
- Internal JMAP operations use `-> Result<T>` (custom error type)
- Error conversion via `#[from]` for automatic impl in error enum
- Match patterns for error cases: `if result.is_err() { ... }`
- Error propagation with `?` operator throughout async functions

**Error Context:**
- JMAP errors include method name, error type, and description:
  ```rust
  Error::Jmap {
      method: "Email/set".into(),
      error_type: error_type.into(),
      description: description.into(),
  }
  ```
- HTTP status codes mapped to specific errors:
  ```rust
  match resp.status().as_u16() {
      401 => return Err(Error::InvalidToken(...)),
      429 => return Err(Error::RateLimited),
      500..=599 => return Err(Error::Server(...)),
  }
  ```

## Logging

**Framework:** `tracing` crate with `tracing-subscriber`

**Setup:** Initialized in main():
```rust
tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .with_target(false)
    .init();
```

**Patterns:**
- `#[instrument]` macro on async functions for automatic span creation:
  ```rust
  #[instrument(skip(self, body, params))]
  pub async fn send_email(&mut self, ...) -> Result<String> {
  ```
- `debug!()` for detailed diagnostic info: `debug!("Fetching JMAP session")`
- `debug!()` with structured fields: `debug!(username = %session.username, "Session established")`
- Failures logged with `tracing::debug!()` for deserialization errors
- No info/warn/error logging in library code — let callers decide

## Comments

**When to Comment:**
- Public API methods: Brief description of purpose
- Non-obvious logic: Complex parsing, threading header construction, state management
- MIME type mappings: Extensive list of file extensions in `util.rs` mime_from_filename()
- Security-sensitive code: File permission setting with comments about Unix mode

**JSDoc/TSDoc:**
- Use `///` for module-level and public struct/function documentation
- Example from `carddav/mod.rs`:
  ```rust
  //! CardDAV client for Fastmail contacts
  //!
  //! Uses raw HTTP with reqwest since CardDAV is just WebDAV with vCard.
  ```
- Short doc comments on public types:
  ```rust
  /// A contact parsed from vCard
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct Contact {
  ```

## Function Design

**Size:** Functions are generally focused:
- Command handlers (e.g., `send()`, `get_email()`): 10-30 lines
- Public API methods: 40-150 lines (JMAP operations with complex logic)
- Helper functions: 5-25 lines
- Largest function in codebase: `JmapClient::send_email()` at ~150 lines (acceptable for complex API interaction)

**Parameters:**
- Prefer passing references for large types: `&str`, `&Email`, `&[EmailAddress]`
- Use builder-like structs for multiple related parameters: `ComposeParams`, `SearchFilter`
- Slice types for collections: `&[EmailAddress]` instead of `Vec<EmailAddress>`
- Optional parameters as `Option<T>`: `from: Option<&str>`

**Return Values:**
- Explicit `Result<T>` type for error handling
- No implicit Option returns — use Result for operations that may fail
- Struct wrappers for related return values: `SendResponse { email_id, status }`
- Internal Option uses for chaining: `Option::and_then()`, `Option::or_else()`

## Module Design

**Exports:**
- Private by default: `mod commands;` (internal)
- Public modules re-export contents: `pub mod util;` with `pub fn parse_addresses()`
- Submodule re-exports in `mod.rs`: `pub use auth::*;`, `pub use send::*;`
- Entry point `src/main.rs` imports from modules: `use models::Output`, `use commands::*`

**Barrel Files:**
- `src/commands/mod.rs` re-exports all command functions
- `src/mcp/graphql/mod.rs` (parent) re-exports submodules
- `src/mcp/mod.rs` exports public GraphQL types via barrel

**Module Structure:**
- `src/main.rs`: CLI entry point with clap command definitions
- `src/error.rs`: Custom error type with thiserror
- `src/config.rs`: Configuration loading/saving with TOML
- `src/models/mod.rs`: Data structures for JMAP/API types
- `src/jmap/mod.rs`: JMAP client implementation and HTTP interaction
- `src/carddav/mod.rs`: CardDAV client for contact operations
- `src/util.rs`: Shared utilities (email parsing, text extraction, image processing)
- `src/commands/`: Individual command implementations
- `src/mcp/`: Model Context Protocol server with GraphQL schema

## Concurrency & Async

**Pattern:** All I/O operations are async:
- Functions marked with `async fn`
- Main runtime: `#[tokio::main]` on main function
- Client operations use `Client` from `reqwest` with async methods
- Awaiting: `client.get_email().await?`

**Dependencies:**
- `tokio` with `full` features for full async runtime
- `reqwest` with `json` and `rustls` features for HTTP

## Testing Infrastructure in Code

Tests are integrated within modules using `#[cfg(test)]`:
- All test functions in `#[cfg(test)]` blocks at module end
- Unit tests in same file as implementation (not separate test directory)
- Example locations: `src/util.rs`, `src/models/mod.rs`, `src/jmap/mod.rs`, `src/config.rs`

---

*Convention analysis: 2026-03-27*
