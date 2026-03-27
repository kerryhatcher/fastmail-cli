<!-- GSD:project-start source:PROJECT.md -->
## Project

**Contact CRUD via CardDAV**

Adding contact create, update, and delete operations to fastmail-cli, extending the existing CardDAV read-only integration. Exposes these as both CLI commands (`contacts create/update/delete`) and GraphQL mutations in the MCP server. Implements radiosilence/fastmail-cli#17.

**Core Value:** Users can manage contacts (create, update, delete) without leaving the terminal or AI assistant, building on the existing CardDAV plumbing.

### Constraints

- **Tech stack**: Rust, must follow existing patterns (clap derive, async-graphql, reqwest)
- **Protocol**: CardDAV (WebDAV + vCard) — PUT for create/update, DELETE for delete
- **Compatibility**: Must work with Fastmail's CardDAV server specifically
- **Auth**: Reuse existing authentication mechanism (app-specific password in config)
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 2024 edition - CLI application, JMAP client, HTTP handlers
## Runtime
- Rust toolchain (stable)
- Tokio async runtime (1.49.0)
- Cargo (Rust package manager)
- Lockfile: Present (`Cargo.lock`)
## Frameworks
- Tokio 1.49.0 - Async runtime with full feature set for concurrent operations
- Reqwest 0.13.1 - HTTP client with TLS/rustls support and JSON serialization
- Clap 4.5.54 - Command-line argument parsing with derive macros
- Clap_complete 4.5.65 - Shell completion generation (bash, zsh, fish, powershell)
- async-graphql 7 - GraphQL server implementation for MCP
- rmcp 0.12 - Model Context Protocol server framework with transport-io support
- Serde 1.0.228 - Serialization framework with derive macros
- Serde_json 1.0.149 - JSON support
- Toml 0.8 - TOML configuration parsing
- Kreuzberg 4.4 - Multi-format text extraction (56 formats)
- Image 0.25 - Image manipulation (gif, jpeg, png, webp formats)
- roxmltree 0.21.1 - XML parsing for vCard/CardDAV responses
- Tracing 0.1.44 - Structured logging framework
- Tracing-subscriber 0.3.22 - Tracing backend with env-filter support
## Key Dependencies
- Reqwest 0.13.1 - Why it matters: Handles all HTTP communication with Fastmail JMAP API and CardDAV endpoints
- Tokio 1.49.0 - Why it matters: Enables concurrent async operations for email operations, downloads, and server mode
- async-graphql 7 - Why it matters: Powers the MCP server's GraphQL schema for composable queries
- Serde 1.0.228 - Polymorphic JSON/TOML serialization for API responses and config
- Schemars 0.8 - JSON Schema generation for MCP tool parameter validation
- Thiserror 2.0.17 - Error type derivation for custom error handling
- Anyhow 1.0.100 - Error context and wrapping
- Dirs 6.0.0 - Platform-aware config directory paths (`~/.config/fastmail-cli`)
- Base64 0.22 - Base64 encoding/decoding for email content
## Configuration
- Configuration file: `~/.config/fastmail-cli/config.toml`
- File permissions: 0600 (Unix) for secure credential storage
- Env vars take precedence over config file
- Release profile optimizations:
- Targets: x86_64-linux-gnu, x86_64-darwin, aarch64-darwin
## Platform Requirements
- Rust stable toolchain
- Cargo for building
- Unix-like environment preferred (directory handling, permissions)
- Deployment: Standalone binary or via Mise version manager
- Installation: Prebuilt releases on GitHub or `cargo install --git`
- Platforms: Linux (x86_64), macOS (x86_64, aarch64)
## Feature Flags
- Enables text extraction from 56 file formats
- Disables default features, adds specific image format support
- JSON serialization for API requests
- Rustls for TLS (no OpenSSL dependency)
- Complete feature set including io-util, rt, sync, time, macros
- Runtime log level filtering via `RUST_LOG` environment variable
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Lowercase with underscores: `send.rs`, `get.rs`, `config.rs`, `carddav.rs`
- Reserved keyword escaping when needed: `r#move.rs` (uses raw identifier prefix)
- Module structure uses `mod.rs` as entry point for submodules
- snake_case for all functions: `parse_addresses()`, `authenticated_client()`, `get_email()`, `extract_text()`
- Async functions use `async fn` keyword: `async fn authenticate()`, `async fn send_email()`
- Private helper functions with leading underscore for internal use
- snake_case throughout: `email_id`, `mailbox_id`, `token`, `params`, `body`, `content_type`
- Short loop variables accepted: `e` for element in filter, `f` for first in address parsing
- Boolean variables use clear names: `is_addressbook`, `is_image`, `has_attachment`, `is_unread`
- PascalCase for all structs: `JmapClient`, `EmailAddress`, `Session`, `Mailbox`, `CardDavClient`
- PascalCase for enums: `Commands`, `ListCommands`, `Error`
- Type aliases for result types: `pub type Result<T> = std::result::Result<T, Error>;`
- snake_case with serde rename for API conversions:
- Optional fields use `Option<T>`: `pub parent_id: Option<String>`
- Boolean flags use explicit naming: `is_personal`, `is_read_only`, `may_delete`
## Code Style
- No explicit formatter config (uses Rust defaults)
- Standard Rust formatting conventions
- Line continuation with proper indentation in match statements
- Comments use `//` for single line, `///` for documentation comments
- No explicit `.clippy.toml` or `.rustfmt.toml` (uses Rust defaults)
- Code follows Clippy conventions and passes clippy checks (visible in CI)
## Import Organization
- No custom path aliases configured in `Cargo.toml`
- All imports use standard absolute crate paths
- Re-exports in `mod.rs` files make internal modules available: `pub use commands::*`
## Error Handling
- Custom error enum using `thiserror` crate in `src/error.rs`:
- Type alias `pub type Result<T> = std::result::Result<T, Error>;` used throughout
- Function signatures use `-> anyhow::Result<T>` in command handlers (lighter-weight for CLIs)
- Internal JMAP operations use `-> Result<T>` (custom error type)
- Error conversion via `#[from]` for automatic impl in error enum
- Match patterns for error cases: `if result.is_err() { ... }`
- Error propagation with `?` operator throughout async functions
- JMAP errors include method name, error type, and description:
- HTTP status codes mapped to specific errors:
## Logging
- `#[instrument]` macro on async functions for automatic span creation:
- `debug!()` for detailed diagnostic info: `debug!("Fetching JMAP session")`
- `debug!()` with structured fields: `debug!(username = %session.username, "Session established")`
- Failures logged with `tracing::debug!()` for deserialization errors
- No info/warn/error logging in library code — let callers decide
## Comments
- Public API methods: Brief description of purpose
- Non-obvious logic: Complex parsing, threading header construction, state management
- MIME type mappings: Extensive list of file extensions in `util.rs` mime_from_filename()
- Security-sensitive code: File permission setting with comments about Unix mode
- Use `///` for module-level and public struct/function documentation
- Example from `carddav/mod.rs`:
- Short doc comments on public types:
## Function Design
- Command handlers (e.g., `send()`, `get_email()`): 10-30 lines
- Public API methods: 40-150 lines (JMAP operations with complex logic)
- Helper functions: 5-25 lines
- Largest function in codebase: `JmapClient::send_email()` at ~150 lines (acceptable for complex API interaction)
- Prefer passing references for large types: `&str`, `&Email`, `&[EmailAddress]`
- Use builder-like structs for multiple related parameters: `ComposeParams`, `SearchFilter`
- Slice types for collections: `&[EmailAddress]` instead of `Vec<EmailAddress>`
- Optional parameters as `Option<T>`: `from: Option<&str>`
- Explicit `Result<T>` type for error handling
- No implicit Option returns — use Result for operations that may fail
- Struct wrappers for related return values: `SendResponse { email_id, status }`
- Internal Option uses for chaining: `Option::and_then()`, `Option::or_else()`
## Module Design
- Private by default: `mod commands;` (internal)
- Public modules re-export contents: `pub mod util;` with `pub fn parse_addresses()`
- Submodule re-exports in `mod.rs`: `pub use auth::*;`, `pub use send::*;`
- Entry point `src/main.rs` imports from modules: `use models::Output`, `use commands::*`
- `src/commands/mod.rs` re-exports all command functions
- `src/mcp/graphql/mod.rs` (parent) re-exports submodules
- `src/mcp/mod.rs` exports public GraphQL types via barrel
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
- Functions marked with `async fn`
- Main runtime: `#[tokio::main]` on main function
- Client operations use `Client` from `reqwest` with async methods
- Awaiting: `client.get_email().await?`
- `tokio` with `full` features for full async runtime
- `reqwest` with `json` and `rustls` features for HTTP
## Testing Infrastructure in Code
- All test functions in `#[cfg(test)]` blocks at module end
- Unit tests in same file as implementation (not separate test directory)
- Example locations: `src/util.rs`, `src/models/mod.rs`, `src/jmap/mod.rs`, `src/config.rs`
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Async Rust with tokio runtime
- Clear separation between CLI commands, API clients, and data models
- Dual interface: command-line and Model Context Protocol (MCP) server
- Authentication-first pattern with config file storage
- JSON-based output for all operations
## Layers
- Purpose: Parse command-line arguments and route to appropriate handlers
- Location: `src/main.rs`, `src/commands/`
- Contains: Clap-based command definitions, command handlers for each operation
- Depends on: JMAP client, models, utilities, config
- Used by: Main entry point, MCP server
- Purpose: Abstract Fastmail JMAP API protocol and provide high-level operations
- Location: `src/jmap/mod.rs`
- Contains: HTTP client wrapper, session management, email operations, threading
- Depends on: reqwest, serde, models
- Used by: All commands, MCP GraphQL schema
- Purpose: Handle contact management via WebDAV/vCard
- Location: `src/carddav/mod.rs`
- Contains: Contact discovery, vCard parsing, contact search
- Depends on: reqwest, roxmltree (XML parsing)
- Used by: Contact commands (`src/commands/contacts.rs`)
- Purpose: Expose Fastmail functionality via Model Context Protocol for AI integration
- Location: `src/mcp/mod.rs`, `src/mcp/graphql/`
- Contains: GraphQL schema builder, tool definitions, query/mutation resolvers
- Depends on: async-graphql, JMAP client, models
- Used by: `Commands::Mcp` entry point
- Purpose: Serialize/deserialize JMAP responses and represent domain entities
- Location: `src/models/mod.rs`
- Contains: Session, Email, Mailbox, Identity, MaskedEmail, EmailAddress structures
- Depends on: serde
- Used by: All layers
- Purpose: Manage authentication tokens and user settings
- Location: `src/config.rs`
- Contains: Config file I/O (~/.config/fastmail-cli/config.toml), env var fallback
- Depends on: toml, dirs crates
- Used by: JMAP client, CardDAV client, commands
- Purpose: Unified error types for all operations
- Location: `src/error.rs`
- Contains: Thiserror-based error enum with domain-specific variants
- Depends on: thiserror, reqwest
- Used by: All layers
- Purpose: Common operations across commands
- Location: `src/util.rs`
- Contains: Email address parsing, image processing, document text extraction, size parsing
- Depends on: image, kreuzberg (document extraction)
- Used by: Commands, MCP layer
- Purpose: Standardize JSON output across all commands
- Location: `src/models/mod.rs` (Output struct)
- Contains: Success/error JSON wrapper
- Depends on: serde
- Used by: All commands
## Data Flow
- **Session**: Created once on first auth, cached in `JmapClient.session`
- **Mailboxes**: Cached in `JmapClient.cached_mailboxes` for performance
- **Config**: Loaded from disk, held in memory during execution
- **No database**: All state is transient or persistent in Fastmail's servers
## Key Abstractions
- Purpose: Abstract JMAP protocol complexity, provide typed methods
- Examples: `send_email()`, `get_email()`, `search_emails()`, `get_thread()`
- Pattern: Builder pattern for search filters, async methods return `Result<T>`
- Purpose: Bundle email composition metadata (cc, bcc, identity, draft flag)
- Examples: `src/jmap/mod.rs` lines 38-68
- Pattern: Separate user input (ComposeParams) from resolved context (ComposeContext)
- Purpose: Wrap command results as standardized JSON
- Examples: `Output::success(data).print()`, `Output::error(msg).print()`
- Pattern: Generic struct with serde serialization
- Purpose: Abstract WebDAV/vCard protocol for contacts
- Examples: `list_addressbooks()`, `search_contacts()`
- Pattern: Separate HTTP concerns from business logic
- Purpose: Typed representation of JMAP filter criteria
- Examples: `src/commands/search.rs`, `SearchFilter` struct
- Pattern: Option-based fields map to JMAP filter operators
## Entry Points
- Location: `src/main.rs`
- Triggers: User runs `fastmail-cli <command> [args]`
- Responsibilities: Parse arguments, dispatch to command handler, handle errors, exit with status
- Location: `src/main.rs` → `Commands::Mcp` → `mcp::run_server()`
- Triggers: User runs `fastmail-cli mcp`
- Responsibilities: Initialize GraphQL schema, listen for tool calls, serve schema and execute queries
- Location: `src/commands/<command>.rs`
- Triggers: User invokes specific subcommand (e.g., `send`, `get`, `search`)
- Responsibilities: Validate input, call JMAP client, format and print output
## Error Handling
- All functions return `Result<T>` (alias for `std::result::Result<T, Error>`)
- Command handlers propagate errors to main(), which prints JSON error and exits(1)
- JMAP errors mapped to domain-specific variants (MailboxNotFound, EmailNotFound, etc.)
- HTTP/IO errors preserved via `#[from]` attribute for debugging
- Config errors caught early before attempting API calls
- Authentication required guard in `authenticated_client()` returns NotAuthenticated error
```json
```
## Cross-Cutting Concerns
- Tracing crate with `#[instrument]` macro on key methods
- EnvFilter allows runtime control via `RUST_LOG` env var
- Spans track async task execution across await points
- No logging in prod by default; enable with `RUST_LOG=debug fastmail-cli ...`
- Email addresses: parsed by `util::parse_addresses()` with optional name support
- Mailbox names: matched against list from JMAP session (case-sensitive)
- Dates: ISO 8601 format (e.g., 2024-01-01) validated by JMAP server
- File paths: constructed from user input, validated before filesystem operations
- Sizes: human-readable format (e.g., 500K, 1M) parsed by `util::parse_size()`
- Bearer token stored in config file (mode 0600 on Unix)
- Env var `FASTMAIL_API_TOKEN` overrides config file
- Session established on first API call, cached for reuse
- Token validation happens at first authenticated_client() call
- 401 response prompts user to re-authenticate
- Models deserialized from JMAP JSON-RPC responses
- EmailAddress parsed from "name <email>" format
- Document attachments extracted using kreuzberg library
- Images resized using image crate for MCP context window limits
- CardDAV vCard parsed using roxmltree for XML structure
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
