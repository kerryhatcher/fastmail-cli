---
gsd_state_version: 1.0
milestone: v1.2
milestone_name: Hardening & Quality
status: executing
stopped_at: Completed 16-03-PLAN.md
last_updated: "2026-04-05T01:48:05.930Z"
last_activity: 2026-04-05
progress:
  total_phases: 6
  completed_phases: 4
  total_plans: 20
  completed_plans: 19
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-04-04)

**Core value:** Users can manage Fastmail data without leaving the terminal or AI assistant, with automation-friendly APIs that stay faithful to Fastmail's actual protocol boundaries
**Current focus:** Phase 16 — integration-test-coverage

## Current Position

Phase: 16 (integration-test-coverage) — EXECUTING
Plan: 4 of 4
Status: Ready to execute
Last activity: 2026-04-05

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

*Updated after each plan completion*
| Phase 12-foundation-safety P02 | 25 | 2 tasks | 8 files |
| Phase 12 P04 | 15 | 1 tasks | 1 files |
| Phase 12-foundation-safety P03 | 9 | 2 tasks | 3 files |
| Phase 12-foundation-safety P01 | 10min | 2 tasks | 4 files |
| Phase 13-security-hardening P01 | 3min | 2 tasks | 1 files |
| Phase 13-security-hardening P02 | 8min | 2 tasks | 1 files |
| Phase 13-security-hardening P03 | 15 | 2 tasks | 3 files |
| Phase 13 P04 | 212s | 2 tasks | 4 files |
| Phase 14 P04 | 147s | 1 tasks | 1 files |
| Phase 14 P01 | 212s | 2 tasks | 3 files |
| Phase 14-mcp-layer-refactor P02 | 320s | 2 tasks | 5 files |
| Phase 14-mcp-layer-refactor P03 | 145s | 1 tasks | 1 files |
| Phase 15-performance P03 | 300 | 1 tasks | 3 files |
| Phase 15 P01 | 552s | 2 tasks | 5 files |
| Phase 15-performance P02 | 621 | 2 tasks | 5 files |
| Phase 15-performance P04 | 413 | 2 tasks | 2 files |
| Phase 16-integration-test-coverage P01 | 387s | 3 tasks | 10 files |
| Phase 16 P04 | 70s | 1 tasks | 1 files |
| Phase 16-integration-test-coverage P03 | 275s | 2 tasks | 5 files |

## Accumulated Context

### Decisions

Pending user decision before Phase 12 planning:

- Phase 12: `secrecy` crate vs manual `Debug` impl for SEC-06 (see SUMMARY.md gap — STACK recommends `secrecy`; FEATURES recommends manual to avoid new dep)
- [Phase 12-foundation-safety]: CardDavClient::new() and CalDavClient::new() changed to Result<Self> to surface HTTP builder failures
- [Phase 12-foundation-safety]: safe_filename() in download.rs uses Path::file_name() with 'attachment' fallback to prevent path traversal
- [Phase 12]: No Error::ConfirmationRequired variant (D-07): kept inline Output::error+bail pattern at each of 5 callsites for readability
- [Phase 12]: Commands::Completions arm changed from return; to Ok(()) to unify match expression type as anyhow::Result<()>
- [Phase 12-foundation-safety]: 4xx arm placed after 500..=599 in JMAP match blocks to preserve 401/429 specificity; format 'HTTP {code} from API' avoids double Error::Server prefix
- [Phase 12-foundation-safety]: JmapClient::new() maps reqwest builder failure to Error::Config (no new variant); production callers use ?, test callers use .expect('test client')
- [Phase 12-foundation-safety]: SEC-06: Custom serde helpers required for SecretString TOML serialization — secrecy 0.10 serde feature only provides Deserialize for SecretBox<str>; expose_secret() boundary maintained at Config accessor methods only
- [Phase 13-security-hardening]: Applied escape_value() to EMAIL/TEL value and label on serialize path — matches existing FN/N/ORG/TITLE/ADR/NOTE pattern, no new dependencies
- [Phase 13-security-hardening]: escape_ical_value() reused for attendee email; sanitize_role/sanitize_partstat add RFC 5545 enum validation with uppercase normalization
- [Phase 13-security-hardening]: serialize_rrule drops invalid UNTIL silently; invalid FREQ produces empty FREQ= string (caller responsibility)
- [Phase 13-security-hardening]: encode_blob_url_segment() extracted as free function (not JmapClient method) to enable isolated unit testing via super:: import
- [Phase 13-security-hardening]: SEC-09: NON_ALPHANUMERIC applied to {blobId} and {name} only; {accountId} and {type} left unencoded (accountId always safe, type hardcoded MIME)
- [Phase 13]: Used std::io::stdin().is_terminal() instead of rpassword — terminal hiding via read -rs shell pattern is sufficient per D-10
- [Phase 13]: resolve_token() uses Output::error+bail pattern for non-interactive failure, consistent with existing command handlers
- [Phase 14]: No tokio::time::timeout wrapper around waiting() — rmcp cancellation already drains in-flight requests cleanly
- [Phase 14]: [Phase 14-04]: SIGTERM handler cfg-gated to unix targets; ctrl_c fallback for non-unix cross-platform support
- [Phase 14]: Used rand_core::TryRngCore::try_fill_bytes (not fill_bytes) for rand_core 0.9 OsRng - CONTEXT.md D-06 had wrong method name
- [Phase 14]: Applied #[allow(dead_code)] on AppContext struct+impl to suppress forward-declared API warnings until plans 02-04 use them
- [Phase 14]: Used .limit_depth(5).limit_complexity(200) in build_schema; injected JmapContext shim alongside AppContext for unmigrated resolvers (TODO 14-02)
- [Phase 14-mcp-layer-refactor]: delete_contact/delete_calendar/delete_event ctx parameter added — resolvers originally ctx-less (used free confirmation_token fn), now need AppContext for HMAC
- [Phase 14-mcp-layer-refactor]: reply_to_email and forward_email keep client-before-preview pattern — needed to fetch original email for preview; app_ctx acquired first so token and client from same context
- [Phase 14-03]: Reused SpamAction enum (no MarkAsSpamAction) per RESEARCH.md Open Question 1 resolution — zero GraphQL SDL type churn
- [Phase 14-03]: Token validation placed BEFORE require_jmap() in mark_as_spam — prevents unauthenticated JMAP lock acquisition; enables unit tests without JMAP client
- [Phase 14-03]: PREVIEW surfaces token in GqlStatus.message body (not new field) — avoids ripple changes to other resolvers using GqlStatus
- [Phase 15-performance]: GqlEmail::new() precomputes 5 Arc<Vec<GqlEmailAddress>> fields; resolvers return &[T] (zero allocation per call)
- [Phase 15-performance]: GqlEmail tuple struct replaced with named-fields struct — enables Arc fields alongside inner Email
- [Phase 15]: collect_partial returns Ok(vec) not Err on failures — empty result + warnings is correct DAV partial-failure behavior
- [Phase 15]: get_event_by_id UID REPORT triggers fallback when any calendar returns 400/501 (conservative)
- [Phase 15]: xml_escape_uid escapes &, <, >, quote chars for UID embedding in REPORT XML
- [Phase 15-performance]: Cow<[u8]> used in download.rs to unify bytes::Bytes and Vec<u8> code paths without introducing to_vec() at call sites
- [Phase 15-performance]: parse_response takes Value by value; arr.remove(1) extracts data element; parse_email_create_response takes Vec<Value> to enable owned extraction for two-response cases
- [Phase 15-performance]: list_mailboxes() return type changed to Result<Arc<Vec<Mailbox>>>; callers updated to iterate via .iter().map(|m| GqlMailbox::from(m.clone()))
- [Phase 15-performance]: kreuzberg optional=true with extract feature gate; default=[extract] preserves backward-compat binary; --no-default-features builds without pdfium
- [Phase 15-performance]: tokio narrowed from full to [rt-multi-thread, rt, macros, io-util, sync, time, signal, net]; all builds and 157 tests pass
- [Phase 15-performance]: Triangle filter replaces Lanczos3 for attachment preview resize — lower CPU cost acceptable for MCP context window images
- [Phase 16-integration-test-coverage]: lib/bin split uses explicit [lib] and [[bin]] Cargo.toml sections; production new() constructors delegate to new_with_*_url() with zero callsite changes
- [Phase 16]: Used mailboxes query field (actual name in query.rs) not listMailboxes (plan template name)
- [Phase 16]: Wiremock mounts both GET /jmap/session and POST /jmap/api/ so authenticate() + list_mailboxes() both succeed in test
- [Phase 16-integration-test-coverage]: CalDAV principal discovery XML must use path-only hrefs (not full URLs) because CalDavClient prepends self.base_url
- [Phase 16-integration-test-coverage]: wiremock up_to_n_times(1) used per PUT variant to serve create 201 first then update 204 for CRUD test

### Pending Todos

None yet.

### Blockers/Concerns

- Phase 14: rmcp 0.12 signal handling lifecycle may need a focused research pass before planning (SIGTERM/SIGINT + tokio::select! interaction not fully documented)
- Phase 15: Fastmail CalDAV UID REPORT syntax needs smoke-test against live account before Phase 15 is marked complete (Cyrus IMAP has known quirks)

## Session Continuity

Last session: 2026-04-05T01:48:05.927Z
Stopped at: Completed 16-03-PLAN.md
Resume file: None
