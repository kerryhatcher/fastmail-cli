# Milestones

## v1.3 Contact Groups (Shipped: 2026-04-14)

**Phases completed:** 2 phases, 6 plans, 1 tasks

**Key accomplishments:**

- 1. [Rule 1 - Bug] Removed invalid `#[graphql(desc = ...)]` method-level attributes
- Task 1 — `src/main.rs`:
- `addGroupMember` and `removeGroupMember` mutations added to MCP GraphQL schema, returning full GqlContactGroup with resolved member contacts via ETag-guarded CardDav transport

---

## v1.2 Hardening & Quality (Shipped: 2026-04-05)

**Phases completed:** 6 phases, 23 plans, 29 tasks

**Key accomplishments:**

- api_token and app_password wrapped in secrecy::SecretString with custom TOML serde helpers, Debug output redacts credentials, Config::load() parse errors include absolute config path and recovery guidance
- 30-second HTTP timeouts added to CardDavClient and CalDavClient constructors; server-supplied attachment filenames sanitized via Path::file_name() to prevent path traversal
- HTTP 400-499 responses now surface as Error::Server (not deserialization panics), and JmapClient::new() returns Result<Self> eliminating the last panic path in construction
- 5 confirmation-guard callsites in src/main.rs now emit Output::error JSON via anyhow::bail instead of eprintln+process::exit
- One-liner:
- JMAP blob download URLs now percent-encode {blobId} and {name} template segments using NON_ALPHANUMERIC set via the percent-encoding 2.3 crate, closing SEC-09
- One-liner:
- AppContext struct with HMAC-SHA256 confirmation tokens (OsRng key), OnceCell lazy DAV init, and GraphQL depth/complexity limits replacing bare JmapContext
- All MCP GraphQL resolvers migrated from JmapContext shim to AppContext; DAV clients shared via OnceCell; all confirmation gates use HMAC tokens; JmapContext shim eliminated
- HMAC confirmation-token gate added to markAsSpam mutation; token validated before JMAP acquisition; SpamAction enum reused; SEC-08 complete
- Concurrent CardDAV/CalDAV fetches via join_all with partial-failure tolerance, plus UID-targeted CalDAV REPORT replacing full event-history sweep in get_event_by_id
- bytes::Bytes for blob downloads, Arc<Vec<T>> for mailbox cache and capabilities, and owned-Value parse_response eliminate the major Vec clone hot paths in the JMAP client
- One-liner:
- Gate kreuzberg behind optional `extract` cargo feature (default-on), switch image resize from Lanczos3 to Triangle, and narrow tokio from `full` to an explicit minimum feature list
- One-liner:
- One-liner:
- One-liner:
- One-liner:
- One-liner:
- SipHasher13 with fixed zero seed replaces DefaultHasher in hash_id(), pinned by golden-value test asserting exact u64 output 17102779196494968154 for "John Doe"
- Deleted two stale lint suppression annotations from src/jmap/mod.rs and src/carddav/mod.rs so clippy's output is trustworthy and future unused-import warnings cannot be silently masked

---

## v1.1 Calendar Access and Management (Shipped: 2026-04-04)

**Phases completed:** 7 phases, 7 plans, 0 tasks

**Key accomplishments:**

- CalDAV foundation with Fastmail calendar-home discovery and collection listing (Phase 5)
- iCalendar parsing/serialization for events, attendees, recurrence, and reminders (Phase 6)
- Full calendar & event CRUD transport with ETag-safe writes (Phase 7)
- CLI calendar experience with default-today, --week, and explicit range controls (Phase 8)
- MCP GraphQL surface for AI agent calendar operations, validated against live Fastmail (Phase 9)
- Explicit range contract hardening and CLI attendee clearing parity (Phases 10-11)

---

## v1.0 milestone (Shipped: 2026-04-03)

**Phases completed:** 4 phases, 4 plans, 9 tasks

**Key accomplishments:**

- Contact struct extended with server-assigned href/etag fields for CardDAV write operations, plus ContactNotFound and ContactConflict error variants for Phase 2-4 write operation error handling
- CardDAV write operations now support create, update, and delete with correct conditional header handling and error mapping.
- CLI and MCP now expose contact create, update, and delete flows with shared partial-update logic and explicit delete confirmation.

---
