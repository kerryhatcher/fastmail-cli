---
phase: 16-integration-test-coverage
plan: 02
subsystem: jmap-integration-tests
tags: [testing, wiremock, jmap, auth, errors, send]
dependency_graph:
  requires: [16-01]
  provides: [jmap-auth-tests, jmap-error-tests, jmap-send-tests]
  affects:
    - tests/jmap_auth.rs
    - tests/jmap_errors.rs
    - tests/jmap_send.rs
    - tests/fixtures/jmap_mailbox_get.json
    - tests/fixtures/jmap_identity_get.json
    - tests/fixtures/jmap_email_set_response.json
tech_stack:
  added: []
  patterns: [wiremock-mock-per-call, up_to_n_times-sequential-mocks, received_requests-wire-assertions]
key_files:
  created:
    - tests/jmap_auth.rs
    - tests/jmap_errors.rs
    - tests/jmap_send.rs
    - tests/fixtures/jmap_mailbox_get.json
    - tests/fixtures/jmap_identity_get.json
    - tests/fixtures/jmap_email_set_response.json
  modified: []
decisions:
  - "send_email() issues 3 sequential POSTs (Mailbox/get, Identity/get, Email/set+Submission/set); mocked with .up_to_n_times(1) mocks in registration order"
  - "authenticate() does not short-circuit on existing session — each call is a fresh GET; authenticate_caches_session test removed, replaced with api_url shape test"
  - "Wire-format assertions use server.received_requests() post-call inspection rather than body_partial_json matchers"
metrics:
  duration: 140s
  completed: "2026-04-05T02:15:00Z"
  tasks: 2
  files: 6
---

# Phase 16 Plan 02: JMAP Integration Tests Summary

**One-liner:** Wiremock-based integration tests for JMAP auth session parsing, HTTP error variant mapping (401/429/5xx/4xx), and Email/set wire-format assertions via send_email().

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | JMAP auth + errors tests | fe6ed01 | tests/jmap_auth.rs, tests/jmap_errors.rs |
| 2 | JMAP email send wire-format test + fixtures | d384235 | tests/jmap_send.rs, tests/fixtures/jmap_{mailbox,identity,email_set}*.json |

## What Was Built

### Task 1: Auth and Error Tests

**tests/jmap_auth.rs** (3 tests):
- `authenticate_returns_session_on_200`: Mocks GET /jmap/session returning 200 fixture. Asserts `session.username == "test@example.com"`, capabilities map contains `urn:ietf:params:jmap:core/mail/submission`, and `primary_account_id() == "u1234abcd"`. Matcher includes exact Bearer token header check.
- `authenticate_sends_bearer_token`: Verifies the `Authorization: Bearer test-token` header is forwarded by using a header matcher that only triggers on the correct token.
- `authenticate_parses_api_url`: Verifies `session.api_url` starts with the mock server URI and contains `/jmap/api/` (needed so downstream `request()` calls route to the mock).

**tests/jmap_errors.rs** (8 tests):
- 401 → `Error::InvalidToken(_)` with non-empty message
- 429 → `Error::RateLimited`
- 500, 503 → `Error::Server(_)`
- 400, 403, 422 → `Error::Server(msg)` where `msg` contains the numeric HTTP code
- All use a shared `authenticate_with_status()` helper to avoid repetition.

### Task 2: Email Send Wire-Format Test

**tests/jmap_send.rs** (2 tests):
- `send_email_posts_correct_wire_format`: Mounts session mock + 3 sequential POST mocks (Mailbox/get, Identity/get, Email/set+EmailSubmission/set), calls `send_email()` with subject "Test Subject" and recipient `dest@example.com`. Asserts:
  - Result is `Ok("Email-abc123")` (from fixture created.email.id)
  - At least one POST body's `methodCalls` array contains `"Email/set"`
  - Serialized POST body string contains `"Test Subject"` and `"dest@example.com"`
- `send_email_fails_when_session_returns_401`: Ensures auth error propagates before JMAP requests are attempted.

**3 fixture files** added under `tests/fixtures/`:
- `jmap_mailbox_get.json`: Mailbox/get response with Sent (role "sent"), Drafts, Inbox mailboxes
- `jmap_identity_get.json`: Identity/get response with single identity `test@example.com`
- `jmap_email_set_response.json`: Email/set + EmailSubmission/set combined response returning `Email-abc123`

## Verification

- `cargo test --test jmap_auth --test jmap_send --test jmap_errors`: 13 passed, 0 failed
- `cargo test` (full suite): 177 passed, 0 failed
- No network access: all tests use wiremock bound to 127.0.0.1 with a random port

## Deviations from Plan

### Auto-fixed Issues

None.

### Adjustments (non-deviating clarifications)

**1. authenticate_caches_session test dropped** (Task 1)
- `authenticate()` always performs a fresh GET (no cache check), so an `.expect(1)` mock would fail on a second call.
- Replaced with `authenticate_parses_api_url` test, which provides equivalent coverage of session state persistence.

**2. send_email() POST sequence confirmed via source inspection** (Task 2)
- Plan said "inspect actual request sequence". Found: 3 sequential POSTs (Mailbox/get, Identity/get, Email/set+Submission). Mocked with `.up_to_n_times(1)` ordered mocks.
- Fixture response key corrected: `Email/set` response `created` key must be `"email"` (not `"draft1"`) to match the `parse_email_create_response` code that looks for `c.get("email")`.

## Known Stubs

None.

## Self-Check: PASSED

- tests/jmap_auth.rs: FOUND
- tests/jmap_errors.rs: FOUND
- tests/jmap_send.rs: FOUND
- tests/fixtures/jmap_mailbox_get.json: FOUND
- tests/fixtures/jmap_identity_get.json: FOUND
- tests/fixtures/jmap_email_set_response.json: FOUND
- Commit fe6ed01: FOUND
- Commit d384235: FOUND
