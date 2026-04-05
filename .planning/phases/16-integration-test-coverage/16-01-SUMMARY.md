---
phase: 16-integration-test-coverage
plan: 01
subsystem: test-infrastructure
tags: [testing, wiremock, carddav, caldav, jmap, lib-split]
dependency_graph:
  requires: []
  provides: [lib-crate, url-override-constructors, test-harness, fixtures]
  affects: [src/jmap/mod.rs, src/carddav/mod.rs, src/caldav/mod.rs, tests/common/mod.rs]
tech_stack:
  added: [wiremock 0.6]
  patterns: [url-override-constructor, lib+bin crate split, tests/common/mod.rs harness]
key_files:
  created:
    - src/lib.rs
    - tests/common/mod.rs
    - tests/fixtures/jmap_session.json
    - tests/fixtures/caldav_calendars_propfind.xml
    - tests/fixtures/carddav_addressbooks_propfind.xml
  modified:
    - Cargo.toml
    - src/main.rs
    - src/jmap/mod.rs
    - src/carddav/mod.rs
    - src/caldav/mod.rs
decisions:
  - "lib/bin split uses explicit [lib] and [[bin]] Cargo.toml sections; package name fastmail-cli, lib crate name fastmail_cli"
  - "Production new() constructors delegate to new_with_*_url() — zero callsite changes in commands/"
  - "search_contacts async closure clones base_url alongside username/app_password for move capture"
  - "extract_location_path free function in carddav/caldav retains CARDDAV_BASE/CALDAV_BASE const reference — acceptable for production fallback path"
metrics:
  duration: 387s
  completed: "2026-04-05T01:42:42Z"
  tasks: 3
  files: 10
---

# Phase 16 Plan 01: Integration Test Infrastructure Summary

**One-liner:** Lib+bin crate split, wiremock 0.6 dev-dep, per-client URL-override constructors, and shared tests/common/mod.rs harness with 3 baseline fixtures.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 0 | Split crate into lib + bin | d0dc99e | src/lib.rs, src/main.rs, Cargo.toml |
| 1 | Add wiremock dev-dep and URL-override constructors | 3985ff7 | Cargo.toml, Cargo.lock, src/jmap/mod.rs, src/carddav/mod.rs, src/caldav/mod.rs |
| 2 | Create tests/common/mod.rs harness and baseline fixtures | 8344589 | tests/common/mod.rs, tests/fixtures/*.json, tests/fixtures/*.xml |

## What Was Built

### Task 0: Lib + Bin Split

`src/lib.rs` created with 9 `pub mod` declarations covering all internal modules. `src/main.rs` updated to import from `fastmail_cli` crate via `use fastmail_cli::{caldav, commands, jmap, mcp, models, util}`. Six `crate::caldav::` references in main.rs updated to `caldav::`. Cargo.toml received explicit `[lib]` (name = "fastmail_cli") and `[[bin]]` (name = "fastmail-cli") sections.

### Task 1: Wiremock + URL Override Constructors

- `JmapClient` gains `session_url: String` field and `new_with_session_url(token, session_url)` constructor. `new()` delegates. `authenticate()` reads from `self.session_url` instead of the `SESSION_URL` const.
- `CardDavClient` gains `base_url: String` field and `new_with_base_url(username, app_password, base_url)`. `new()` delegates. All 6 `CARDDAV_BASE` usages inside `impl CardDavClient` methods replaced with `self.base_url`.
- `CalDavClient` gains `base_url: String` field and `new_with_base_url(username, app_password, base_url)`. `new()` delegates. All 10 `CALDAV_BASE` usages inside `impl CalDavClient` methods replaced with `self.base_url`.
- `wiremock = "0.6"` added to `[dev-dependencies]`.

### Task 2: Test Harness + Fixtures

`tests/common/mod.rs` exports 6 helpers: `start_mock_server`, `load_fixture`, `jmap_session_response`, `test_jmap_client`, `test_carddav_client`, `test_caldav_client`.

Three fixtures created:
- `tests/fixtures/jmap_session.json` — realistic Session response with 4 `{{BASE_URL}}` placeholders (apiUrl, downloadUrl, uploadUrl, eventSourceUrl).
- `tests/fixtures/caldav_calendars_propfind.xml` — multistatus with 3 calendars (Default, Work, Personal) using correct DAV:/caldav:/apple namespaces.
- `tests/fixtures/carddav_addressbooks_propfind.xml` — multistatus with Default addressbook using DAV:/carddav namespaces.

## Verification

- `cargo build` — clean, no warnings
- `cargo build --lib` — clean
- `cargo build --tests` — clean
- `cargo test --lib` — 155 passed, 0 failed
- `grep -rn "wiremock" src/` — empty (wiremock not in production code)
- `grep -c "{{BASE_URL}}" tests/fixtures/jmap_session.json` — 4

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED

- src/lib.rs exists: FOUND
- tests/common/mod.rs exists: FOUND
- tests/fixtures/jmap_session.json exists: FOUND
- tests/fixtures/caldav_calendars_propfind.xml exists: FOUND
- tests/fixtures/carddav_addressbooks_propfind.xml exists: FOUND
- Commit d0dc99e: FOUND
- Commit 3985ff7: FOUND
- Commit 8344589: FOUND
