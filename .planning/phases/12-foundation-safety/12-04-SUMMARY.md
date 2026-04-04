---
phase: 12-foundation-safety
plan: "04"
subsystem: cli
tags: [rust, anyhow, output-contract, confirmation-guard, json-output]

requires:
  - phase: 12-foundation-safety/12-03
    provides: main.rs with correct match arm structure (bail!-compatible Result type)

provides:
  - All 5 confirmation-guard callsites in src/main.rs emit Output::error JSON before exiting
  - anyhow::bail!("confirmation required") replaces std::process::exit(1) at all guard sites
  - MCP hosts and scripting consumers receive valid JSON envelope on confirmation-required exits

affects: [mcp, scripting-consumers, all-delete-commands]

tech-stack:
  added: []
  patterns:
    - "Confirmation guard pattern: Output::<()>::error(msg).print() + anyhow::bail!(\"confirmation required\")"

key-files:
  created: []
  modified:
    - src/main.rs

key-decisions:
  - "Used inline Output::<()>::error().print() + anyhow::bail!() at each callsite (D-07: no Error::ConfirmationRequired variant)"
  - "Completions arm changed from return; to Ok(()) to unify match expression type as anyhow::Result<()>"

patterns-established:
  - "Confirmation required: Output::<()>::error(\"Confirmation required: pass <flag> to <action>\").print() then anyhow::bail!()"

requirements-completed: [STAB-03]

duration: 15min
completed: 2026-04-04
---

# Phase 12 Plan 04: Confirmation-Guard JSON Output Contract Summary

**5 confirmation-guard callsites in src/main.rs now emit Output::error JSON via anyhow::bail instead of eprintln+process::exit**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-04-04T22:36:00Z
- **Completed:** 2026-04-04T22:51:00Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Replaced all 5 `eprintln!` + `std::process::exit(1)` confirmation guards with the JSON-contract pattern
- All destructive commands (spam, masked delete, contacts delete, calendars delete, events delete) now emit `{"error": "Confirmation required: ..."}` on stdout
- Fixed `Commands::Completions` arm to return `Ok(())` instead of `return;` to unify match type
- Build passes, clippy passes with zero warnings, 116 tests pass

## Task Commits

The changes were applied as a deviation fix in the 12-03 plan execution (Rule 3 - blocking):

1. **Task 1: Replace all 5 confirmation-guard exits with JSON-contract pattern** - `1860e29` (fix applied as part of 12-03 Rule 3 auto-fix)

## Files Created/Modified
- `src/main.rs` - Replaced 5 eprintln+process::exit confirmation guards with Output::error().print() + anyhow::bail!()

## Decisions Made
- Applied D-07: No `Error::ConfirmationRequired` variant — kept explicit inline pattern at each callsite
- Applied D-08: Each message names the required flag (e.g., "pass -y to mark email as spam", "pass --confirm to delete calendar")
- Fixed `Commands::Completions` arm: `return;` became `Ok(())` to unify match expression return type as `anyhow::Result<()>`

## Deviations from Plan

### Pre-applied by Parallel Agent

The 12-03 plan agent applied these changes as a Rule 3 (blocking issue) auto-fix before this plan executed. The 12-03 agent needed the `anyhow::bail!()` pattern to work because it changed `CardDavClient::new()` to return `Result<Self>`, which broke the match type inference in `main()`. The agent fixed all 5 guards as part of unblocking the match expression type.

This plan verified all changes are in place, all acceptance criteria pass, and committed the SUMMARY documentation.

---

**Total deviations:** 0 (work was pre-applied by 12-03 agent as Rule 3 fix)
**Impact on plan:** No scope issue — the exact required changes were applied. STAB-03 is satisfied.

## Issues Encountered
- Parallel execution caused file modification contention: stash/pop operations from another agent agent reverted partial edits made by this executor. Resolved by using atomic Python-based replacement and verifying via bash grep rather than relying on Read tool cache.
- Build failures from other parallel plan changes (config.rs SecretString, carddav/caldav, download.rs) were transient — resolved by the time this plan's cargo build ran.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- STAB-03 complete: all destructive commands emit JSON confirmation errors
- Phase 12 STAB plan set fully complete (01-04)
- Ready to proceed to Phase 13 (vCard/iCal injection escaping) or SEC-06 verification

---
*Phase: 12-foundation-safety*
*Completed: 2026-04-04*
