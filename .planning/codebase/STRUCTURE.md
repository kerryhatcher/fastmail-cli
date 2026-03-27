# Codebase Structure

**Analysis Date:** 2026-03-27

## Directory Layout

```
fastmail-cli/
├── src/                           # All Rust source code
│   ├── main.rs                    # CLI entry point, command routing
│   ├── commands/                  # Command handlers (one per operation)
│   │   ├── mod.rs                # Module exports and SearchFilter type
│   │   ├── auth.rs               # Authentication handler
│   │   ├── send.rs               # Email sending
│   │   ├── reply.rs              # Reply to email
│   │   ├── forward.rs            # Forward email
│   │   ├── get.rs                # Fetch single email
│   │   ├── read.rs               # Mark email read/unread
│   │   ├── search.rs             # Full-text and filter-based search
│   │   ├── list.rs               # List mailboxes, emails, identities
│   │   ├── move.rs               # Move email to mailbox
│   │   ├── spam.rs               # Mark email as spam
│   │   ├── thread.rs             # Fetch entire thread/conversation
│   │   ├── download.rs           # Download attachments, extract text
│   │   ├── contacts.rs           # List/search contacts
│   │   └── masked.rs             # Manage masked email addresses
│   ├── jmap/                      # JMAP protocol client
│   │   └── mod.rs                # JmapClient struct, API methods, session handling
│   ├── carddav/                   # CardDAV protocol client
│   │   └── mod.rs                # CardDavClient, vCard parsing, contact operations
│   ├── mcp/                       # Model Context Protocol server
│   │   ├── mod.rs                # MCP server setup, GraphQL tools
│   │   └── graphql/              # GraphQL schema and resolvers
│   │       ├── mod.rs            # Schema builder, context setup
│   │       ├── types.rs          # GraphQL type wrappers (GqlEmail, GqlMailbox, etc.)
│   │       ├── query.rs          # Query resolvers (list, get, search, etc.)
│   │       └── mutation.rs       # Mutation resolvers (send, move, etc.)
│   ├── models/                    # Data structures
│   │   └── mod.rs                # Session, Email, Mailbox, Identity, Output types
│   ├── config.rs                  # Configuration file handling
│   ├── error.rs                   # Error types and Result alias
│   └── util.rs                    # Utilities (parsing, image processing, extraction)
├── Cargo.toml                     # Project manifest, dependencies
├── Cargo.lock                     # Dependency lock file
├── README.md                      # User documentation
├── CHANGELOG.md                   # Version history
├── LICENSE                        # MIT license
└── .github/                       # GitHub Actions workflows
    └── workflows/
        ├── ci.yml                # Test, lint, format checks
        └── release.yml           # Build and publish releases

(Generated during build):
target/                           # Compiled artifacts (gitignored)
.planning/                        # GSD planning documents
  └── codebase/
      ├── ARCHITECTURE.md         # This document
      ├── STRUCTURE.md            # This document
      ├── CONVENTIONS.md          # (Optional, not yet written)
      ├── TESTING.md              # (Optional, not yet written)
      ├── CONCERNS.md             # (Optional, not yet written)
      ├── STACK.md                # (Optional, not yet written)
      └── INTEGRATIONS.md         # (Optional, not yet written)
```

## Directory Purposes

**src/**
- Purpose: All Rust source code
- Contains: CLI setup, command handlers, protocol clients, data models
- Key files: `main.rs` (entry), `commands/` (routing), `jmap/` (core API client)

**src/commands/**
- Purpose: Implement each CLI command as a separate module
- Contains: 14 command modules, one handler function per module
- Key files: `mod.rs` (exports and types), command files (handler functions)
- Pattern: Each command module is minimal (~30-60 lines), calls JMAP client or utilities

**src/jmap/**
- Purpose: Encapsulate JMAP protocol and Fastmail API interaction
- Contains: HTTP client wrapper, session negotiation, email operations
- Key files: `mod.rs` (main client, 500+ lines of methods)
- Pattern: Single `JmapClient` struct, impl block with many async methods

**src/carddav/**
- Purpose: WebDAV/vCard protocol for contact management
- Contains: HTTP client setup, CardDAV operations, vCard parsing
- Key files: `mod.rs` (CardDavClient struct and operations)
- Pattern: Similar to JMAP client but simpler (no session caching)

**src/mcp/**
- Purpose: Expose Fastmail as MCP server for Claude/LLMs
- Contains: Tool registration, GraphQL schema, resolvers
- Key files: `mod.rs` (server setup), `graphql/mod.rs` (schema builder), `graphql/query.rs` and `graphql/mutation.rs` (resolvers)
- Pattern: Wraps existing JMAP client functionality in GraphQL interface

**src/models/**
- Purpose: Data structures for serialization/deserialization
- Contains: JMAP types, output wrapper, domain entities
- Key files: `mod.rs` (60+ type definitions)
- Pattern: Derives serde, uses camelCase for JMAP compatibility

**Cargo.toml**
- Purpose: Project metadata and dependency declarations
- Key sections:
  - `[package]`: Name, version, edition (2024), keywords
  - `[dependencies]`: Core (clap, reqwest, tokio, serde), API (async-graphql, rmcp), Utilities (image, kreuzberg, roxmltree)
  - `[profile.release]`: Optimizations (LTO, stripping)

## Key File Locations

**Entry Points:**
- `src/main.rs`: CLI argument parsing and command dispatch (570 lines)
  - Defines Cli struct (clap Parser), Commands enum (all subcommands)
  - Dispatches to command handlers or MCP server

**Configuration:**
- `src/config.rs`: Config file I/O and env var handling (100+ lines)
  - Location: `~/.config/fastmail-cli/config.toml`
  - Permissions: 0600 (user-read-write only)
  - Env vars: `FASTMAIL_API_TOKEN`, `FASTMAIL_USERNAME`, `FASTMAIL_APP_PASSWORD`

**Core Logic:**
- `src/jmap/mod.rs`: JMAP client (800+ lines, read with limit)
  - Methods: `send_email()`, `get_email()`, `search_emails()`, `get_thread()`, `move_email()`, etc.
  - Session management and capability negotiation
- `src/commands/`: Individual command handlers (15 modules, ~30-60 lines each)
  - Pattern: Read input → Call JMAP/CardDAV → Print output
  - No duplicate code: shared via JMAP client layer

**Testing:**
- `src/util.rs`: Unit tests for utility functions (lines 256-314)
  - Tests: Email address parsing (5 test cases), size parsing, image detection
  - Test pattern: Simple assertions in test modules at file end

**API Integration:**
- `src/jmap/mod.rs`: Fastmail JMAP API client
- `src/carddav/mod.rs`: Fastmail CardDAV API client
- `src/mcp/graphql/`: GraphQL schema builders and resolvers

## Naming Conventions

**Files:**
- Command files: `src/commands/{operation}.rs` (e.g., `send.rs`, `get.rs`)
- Module files: Named `mod.rs` at directory root (e.g., `src/commands/mod.rs`)
- Utilities: `util.rs` for cross-cutting helpers
- Clients: Named after protocol/service (jmap/, carddav/, mcp/)

**Modules:**
- Directories: Lowercase, plural when grouping similar items (`commands/`, `models/`)
- Special case: `r#move` (escape keyword) for move operation

**Functions:**
- Command handlers: Imperative verbs (`send()`, `get_email()`, `search_contacts()`)
- JMAP methods: CamelCase mirroring JMAP objects (`send_email()`, `get_email()`)
- Utilities: Verb+object pattern (`parse_addresses()`, `extract_text()`, `resize_image()`)
- Types: PascalCase (`JmapClient`, `CardDavClient`, `Email`, `Mailbox`)

**Variables:**
- Configuration: `config`, `token`, `username`
- API responses: `response`, `session`, `client`
- Email data: `email`, `emails`, `mailbox`, `mailboxes`
- GraphQL: `query`, `mutation`, `schema`

**Types:**
- Structures: PascalCase (`Email`, `Session`, `Mailbox`, `Identity`)
- Enums: PascalCase for variants (`Commands`, `Error::NotAuthenticated`)
- Result alias: `Result<T>` (standard Rust pattern)
- GraphQL wrappers: Prefix `Gql` (e.g., `GqlEmail`, `GqlMailbox`)

## Where to Add New Code

**New Command:**
1. Create `src/commands/{name}.rs` with handler function
   ```rust
   pub async fn {name}(args...) -> anyhow::Result<()> {
       let client = authenticated_client().await?;
       // Call JMAP client methods
       Output::success(result).print();
       Ok(())
   }
   ```
2. Add module to `src/commands/mod.rs` and export
3. Add Subcommand variant to `Commands` enum in `src/main.rs`
4. Add match arm in main dispatch (lines 361-563)
5. Tests: Add unit tests in handler file

**New JMAP Operation:**
1. Add async method to `JmapClient` impl in `src/jmap/mod.rs`
   - Pattern: Call `self.call()` with JMAP method name and params
   - Return `Result<T>` where T is a model from `src/models/mod.rs`
2. Add corresponding GraphQL resolver in `src/mcp/graphql/query.rs` or `mutation.rs`
3. Update GraphQL schema in `src/mcp/graphql/mod.rs` if new types needed

**New Model Type:**
1. Define struct in `src/models/mod.rs` with serde derives
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct MyType { ... }
   ```
2. Use camelCase serde attribute to match JMAP field names
3. Add GraphQL wrapper in `src/mcp/graphql/types.rs` with `#[derive(SimpleObject)]`
4. Add From<Model> conversion for GraphQL type

**New Utility:**
1. Add function to `src/util.rs`
   - Pattern: Pure functions or standalone helpers
   - Tests: Add test module if needed
2. Export from module if not internal

**New Integration (CardDAV-like):**
1. Create new module `src/{service}/mod.rs`
2. Define client struct with reqwest Client
3. Implement methods following pattern from `CardDavClient`
4. Call from new command handler or MCP resolver

## Special Directories

**target/**
- Purpose: Compiled artifacts and build outputs
- Generated: Yes (by cargo)
- Committed: No (in .gitignore)
- Contents: Debug and release builds, dependencies

**.github/workflows/**
- Purpose: Continuous integration and release automation
- Generated: No (manually maintained)
- Committed: Yes
- Contents: GitHub Actions YAML configs for testing and publishing

**.planning/codebase/**
- Purpose: GSD mapping documents
- Generated: Yes (by mapper agent)
- Committed: Yes (but generated, not edited manually)
- Contents: ARCHITECTURE.md, STRUCTURE.md, etc.

## Configuration Files

**Cargo.toml:**
- Format: TOML
- Purpose: Package manifest and dependency specification
- Modified when: Adding dependencies, updating versions
- Key sections: [package], [dependencies], [profile.release]

**Cargo.lock:**
- Format: TOML
- Purpose: Lock file for exact reproducible builds
- Modified by: `cargo add`, `cargo update`
- Committed: Yes (ensures all developers build same versions)

**.gitignore:**
- Format: Standard gitignore patterns
- Purpose: Exclude build artifacts and secrets
- Contains: `target/`, compiled binaries
- Does NOT exclude: `Cargo.lock` (intentionally committed)

**config.toml (user home directory):**
- Location: `~/.config/fastmail-cli/config.toml`
- Format: TOML
- Purpose: Store API token and contacts config
- Permissions: 0600 (Unix) — user read/write only
- Example:
  ```toml
  [core]
  api_token = "..."

  [contacts]
  username = "user@fastmail.com"
  app_password = "..."
  ```

---

*Structure analysis: 2026-03-27*
