# External Integrations

**Analysis Date:** 2026-03-27

## APIs & External Services

**Fastmail JMAP:**
- Service: Fastmail JSON Mail Access Protocol (RFC 8620)
- What it's used for: Email operations (list, read, search, send, reply, forward, mark spam, move), mailbox management, identity management, masked email management
  - SDK/Client: Custom JMAP client in `src/jmap/mod.rs`
  - Session URL: `https://api.fastmail.com/jmap/session`
  - Auth: Bearer token (env var: `FASTMAIL_API_TOKEN`)
  - Timeout: 30 seconds
  - Capabilities used:
    - `urn:ietf:params:jmap:core`
    - `urn:ietf:params:jmap:mail`
    - `urn:ietf:params:jmap:submission`
    - `https://www.fastmail.com/dev/maskedemail`

**Fastmail CardDAV:**
- Service: Fastmail CardDAV endpoint for contact management
- What it's used for: List and search contacts via vCard
  - Client: Custom CardDAV client in `src/carddav/mod.rs` using raw HTTP
  - Base URL: `https://carddav.fastmail.com`
  - Auth: HTTP Basic Auth (username + app password)
  - App password: Env var `FASTMAIL_APP_PASSWORD` (API tokens don't work for CardDAV)
  - Username: Env var `FASTMAIL_USERNAME`
  - Protocol: WebDAV with vCard (RFC 6352)

## Data Storage

**Databases:**
- Type: None - Stateless CLI application
- State: Transient during command execution only

**File Storage:**
- Local filesystem only - Attachments downloaded to user-specified directories
- Config file: `~/.config/fastmail-cli/config.toml` (TOML format)
- Credentials: Stored with 0600 permissions (Unix) in config file or environment variables

**Caching:**
- In-memory: Session data and mailbox cache during JMAP client lifetime
- No persistent cache layer
- Cached mailboxes: `src/jmap/mod.rs` `cached_mailboxes` field in `JmapClient`

## Authentication & Identity

**Auth Provider:**
- Fastmail native (custom implementation)

**Implementation Details:**
- JMAP: Bearer token authentication
  - Token format: `fmu1-*` (Fastmail API token prefix)
  - Stored in config: `~/.config/fastmail-cli/config.toml` `[core].api_token`
  - Env var takes precedence: `FASTMAIL_API_TOKEN`
  - Retrieval: `config::Config::get_token()` in `src/config.rs` line 76-81

- CardDAV: HTTP Basic Auth
  - Credentials: email + app password
  - Username env var: `FASTMAIL_USERNAME`
  - App password env var: `FASTMAIL_APP_PASSWORD`
  - Retrieval: `config::Config::get_username()` and `config::Config::get_app_password()` in `src/config.rs`
  - Note: App passwords are separate from API tokens (required due to CardDAV protocol restrictions)

## Monitoring & Observability

**Error Tracking:**
- None - Errors logged to stderr via tracing

**Logs:**
- Framework: Tracing with tracing-subscriber
- Configuration: `RUST_LOG` environment variable
- Example: `RUST_LOG=debug fastmail-cli list mailboxes`
- Output: Stderr (instrumented functions in `src/jmap/mod.rs`, `src/carddav/mod.rs`)

## CI/CD & Deployment

**Hosting:**
- GitHub Releases - Prebuilt binaries for Linux/macOS (x86_64, aarch64)
- Mise package manager - Community version management
- Cargo registry - Installable via `cargo install`

**CI Pipeline:**
- GitHub Actions (`.github/workflows/ci.yml`)
  - Runs on: Push to main, pull requests to main
  - Jobs:
    1. **Check**: Formatting (rustfmt), linting (clippy), build, tests
    2. **Release**: Multi-platform builds (Linux x86_64, macOS x86_64, aarch64)
    3. **Publish**: Automatic GitHub release creation when version changes
  - Actions pinned to specific commit SHAs for security
  - Artifacts: Tar archives per platform

**Build Targets:**
- x86_64-unknown-linux-gnu (Ubuntu latest)
- x86_64-apple-darwin (macOS latest)
- aarch64-apple-darwin (macOS latest)

## Environment Configuration

**Required env vars:**
- `FASTMAIL_API_TOKEN` - API token for JMAP (email operations)
- `FASTMAIL_USERNAME` - Email address for CardDAV (contacts only)
- `FASTMAIL_APP_PASSWORD` - App password for CardDAV (contacts only)
- `RUST_LOG` - Log level (optional, default: info)

**Optional env vars:**
- `CARGO_TERM_COLOR` - Always set to "always" in CI

**Secrets location:**
- Config file: `~/.config/fastmail-cli/config.toml`
  - `[core].api_token` for JMAP token
  - `[contacts].username` for CardDAV username
  - `[contacts].app_password` for CardDAV password
  - Alternative: Set via `fastmail-cli auth <token>` command
- Environment variables (take precedence over config file)

**Configuration structure** (`src/config.rs`):
```toml
[core]
api_token = "fmu1-..."

[contacts]
username = "user@fastmail.com"
app_password = "xxxx..."
```

## Webhooks & Callbacks

**Incoming:**
- None - Stateless CLI, no server endpoints

**Outgoing:**
- None - No external webhooks triggered

## API Integration Patterns

**JMAP Client** (`src/jmap/mod.rs`):
- Batch request/response pattern for efficiency
- Session-based state management (account ID, capability negotiation)
- Error handling: Custom JMAP error responses parsed as JSON
- Request/response bodies: JSON via serde_json

**CardDAV Client** (`src/carddav/mod.rs`):
- WebDAV PROPFIND requests for discovery
- vCard parsing via roxmltree XML parser
- HTTP Basic Auth headers
- Raw HTTP (no dedicated CardDAV library)

**MCP Server** (`src/mcp/mod.rs`):
- GraphQL query/mutation interface
- Two tools: `schema_sdl` (introspection), `graphql` (execution)
- Wraps JMAP and CardDAV clients for Claude integration
- Variables passed as JSON strings

## Error Handling

**HTTP Errors:**
- Timeout: 30 seconds per JMAP request
- Connection failures: Wrapped via `anyhow::Context`
- Rate limiting: Not handled (relies on Fastmail service)

**JMAP Errors:**
- Parsed from API response JSON
- Custom error type: `src/error.rs` with thiserror
- Authentication failures: `NotAuthenticated` error type

**CardDAV Errors:**
- XML parsing failures for PROPFIND responses
- Authentication failures: HTTP 401 handling

---

*Integration audit: 2026-03-27*
