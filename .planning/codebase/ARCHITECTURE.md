# Architecture

**Analysis Date:** 2026-03-27

## Pattern Overview

**Overall:** Command-driven async CLI with layered client architecture

**Key Characteristics:**
- Async Rust with tokio runtime
- Clear separation between CLI commands, API clients, and data models
- Dual interface: command-line and Model Context Protocol (MCP) server
- Authentication-first pattern with config file storage
- JSON-based output for all operations

## Layers

**CLI Layer (Routing & Commands):**
- Purpose: Parse command-line arguments and route to appropriate handlers
- Location: `src/main.rs`, `src/commands/`
- Contains: Clap-based command definitions, command handlers for each operation
- Depends on: JMAP client, models, utilities, config
- Used by: Main entry point, MCP server

**JMAP Client Layer (API Integration):**
- Purpose: Abstract Fastmail JMAP API protocol and provide high-level operations
- Location: `src/jmap/mod.rs`
- Contains: HTTP client wrapper, session management, email operations, threading
- Depends on: reqwest, serde, models
- Used by: All commands, MCP GraphQL schema

**CardDAV Client Layer (Contacts Integration):**
- Purpose: Handle contact management via WebDAV/vCard
- Location: `src/carddav/mod.rs`
- Contains: Contact discovery, vCard parsing, contact search
- Depends on: reqwest, roxmltree (XML parsing)
- Used by: Contact commands (`src/commands/contacts.rs`)

**MCP Server Layer (Claude Integration):**
- Purpose: Expose Fastmail functionality via Model Context Protocol for AI integration
- Location: `src/mcp/mod.rs`, `src/mcp/graphql/`
- Contains: GraphQL schema builder, tool definitions, query/mutation resolvers
- Depends on: async-graphql, JMAP client, models
- Used by: `Commands::Mcp` entry point

**Models & Data Layer:**
- Purpose: Serialize/deserialize JMAP responses and represent domain entities
- Location: `src/models/mod.rs`
- Contains: Session, Email, Mailbox, Identity, MaskedEmail, EmailAddress structures
- Depends on: serde
- Used by: All layers

**Configuration Layer:**
- Purpose: Manage authentication tokens and user settings
- Location: `src/config.rs`
- Contains: Config file I/O (~/.config/fastmail-cli/config.toml), env var fallback
- Depends on: toml, dirs crates
- Used by: JMAP client, CardDAV client, commands

**Error Handling Layer:**
- Purpose: Unified error types for all operations
- Location: `src/error.rs`
- Contains: Thiserror-based error enum with domain-specific variants
- Depends on: thiserror, reqwest
- Used by: All layers

**Utilities & Helpers:**
- Purpose: Common operations across commands
- Location: `src/util.rs`
- Contains: Email address parsing, image processing, document text extraction, size parsing
- Depends on: image, kreuzberg (document extraction)
- Used by: Commands, MCP layer

**Output Formatting:**
- Purpose: Standardize JSON output across all commands
- Location: `src/models/mod.rs` (Output struct)
- Contains: Success/error JSON wrapper
- Depends on: serde
- Used by: All commands

## Data Flow

**Authentication Flow:**

1. User calls `fastmail-cli auth <token>`
2. `commands::auth()` saves token to `~/.config/fastmail-cli/config.toml` (mode 0600)
3. On subsequent commands, `Config::load()` reads token (env var `FASTMAIL_API_TOKEN` preferred)
4. `authenticated_client()` creates JMAP client with token
5. Client calls `JmapClient::authenticate()` to fetch session, validate, cache capabilities

**Email Retrieval Flow:**

1. CLI command (e.g., `list emails`) parsed in `main.rs`
2. Command handler in `src/commands/` calls `authenticated_client()`
3. JMAP client executes JSON-RPC style call against https://api.fastmail.com/jmap/session
4. Response deserialized into model struct (Email, Mailbox, etc.)
5. `Output::success()` wraps result as JSON, printed to stdout

**Compose & Send Flow:**

1. User calls `fastmail-cli send --to ... --subject ... --body ...`
2. `commands::send()` parses addresses using `util::parse_addresses()`
3. JMAP client builds `ComposeContext` (resolves identity, draft folder, threading)
4. `create_and_submit_email()` helper creates Email and EmailSubmission objects in JMAP
5. Response returned as `{email_id, status: "sent"|"draft"}`

**MCP Server Flow:**

1. User runs `fastmail-cli mcp` to start GraphQL server
2. `FastmailMcp::new()` initializes JMAP client, builds GraphQL schema
3. Client sends GraphQL query to MCP tools: `schema_sdl` or `graphql`
4. `schema_sdl` returns full SDL for introspection
5. `graphql` executes query against schema, returns typed results

**State Management:**
- **Session**: Created once on first auth, cached in `JmapClient.session`
- **Mailboxes**: Cached in `JmapClient.cached_mailboxes` for performance
- **Config**: Loaded from disk, held in memory during execution
- **No database**: All state is transient or persistent in Fastmail's servers

## Key Abstractions

**JmapClient:**
- Purpose: Abstract JMAP protocol complexity, provide typed methods
- Examples: `send_email()`, `get_email()`, `search_emails()`, `get_thread()`
- Pattern: Builder pattern for search filters, async methods return `Result<T>`

**ComposeParams & ComposeContext:**
- Purpose: Bundle email composition metadata (cc, bcc, identity, draft flag)
- Examples: `src/jmap/mod.rs` lines 38-68
- Pattern: Separate user input (ComposeParams) from resolved context (ComposeContext)

**Output<T>:**
- Purpose: Wrap command results as standardized JSON
- Examples: `Output::success(data).print()`, `Output::error(msg).print()`
- Pattern: Generic struct with serde serialization

**CardDAV Client:**
- Purpose: Abstract WebDAV/vCard protocol for contacts
- Examples: `list_addressbooks()`, `search_contacts()`
- Pattern: Separate HTTP concerns from business logic

**Email Search Filter:**
- Purpose: Typed representation of JMAP filter criteria
- Examples: `src/commands/search.rs`, `SearchFilter` struct
- Pattern: Option-based fields map to JMAP filter operators

## Entry Points

**CLI Main:**
- Location: `src/main.rs`
- Triggers: User runs `fastmail-cli <command> [args]`
- Responsibilities: Parse arguments, dispatch to command handler, handle errors, exit with status

**MCP Server:**
- Location: `src/main.rs` → `Commands::Mcp` → `mcp::run_server()`
- Triggers: User runs `fastmail-cli mcp`
- Responsibilities: Initialize GraphQL schema, listen for tool calls, serve schema and execute queries

**Individual Commands:**
- Location: `src/commands/<command>.rs`
- Triggers: User invokes specific subcommand (e.g., `send`, `get`, `search`)
- Responsibilities: Validate input, call JMAP client, format and print output

## Error Handling

**Strategy:** Error propagation with type-safe error variants

**Patterns:**
- All functions return `Result<T>` (alias for `std::result::Result<T, Error>`)
- Command handlers propagate errors to main(), which prints JSON error and exits(1)
- JMAP errors mapped to domain-specific variants (MailboxNotFound, EmailNotFound, etc.)
- HTTP/IO errors preserved via `#[from]` attribute for debugging
- Config errors caught early before attempting API calls
- Authentication required guard in `authenticated_client()` returns NotAuthenticated error

**Error Output Example:**
```json
{"status":"error","message":"Email not found: invalidId123"}
```

## Cross-Cutting Concerns

**Logging:**
- Tracing crate with `#[instrument]` macro on key methods
- EnvFilter allows runtime control via `RUST_LOG` env var
- Spans track async task execution across await points
- No logging in prod by default; enable with `RUST_LOG=debug fastmail-cli ...`

**Validation:**
- Email addresses: parsed by `util::parse_addresses()` with optional name support
- Mailbox names: matched against list from JMAP session (case-sensitive)
- Dates: ISO 8601 format (e.g., 2024-01-01) validated by JMAP server
- File paths: constructed from user input, validated before filesystem operations
- Sizes: human-readable format (e.g., 500K, 1M) parsed by `util::parse_size()`

**Authentication:**
- Bearer token stored in config file (mode 0600 on Unix)
- Env var `FASTMAIL_API_TOKEN` overrides config file
- Session established on first API call, cached for reuse
- Token validation happens at first authenticated_client() call
- 401 response prompts user to re-authenticate

**Data Transformation:**
- Models deserialized from JMAP JSON-RPC responses
- EmailAddress parsed from "name <email>" format
- Document attachments extracted using kreuzberg library
- Images resized using image crate for MCP context window limits
- CardDAV vCard parsed using roxmltree for XML structure

---

*Architecture analysis: 2026-03-27*
