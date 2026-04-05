# Phase 17: Quality Polish - Context

**Gathered:** 2026-04-04
**Status:** Ready for planning
**Mode:** Auto-context (small polish phase, no grey areas)

<domain>
## Phase Boundary

Small quality improvements: replace fragile unwrap() patterns with let-else guards, switch contact fallback ID hasher from DefaultHasher to a stable hasher, remove stale `#[allow(unused_imports)]` annotations.

Requirements: STAB-05, STAB-08, QUAL-01.

</domain>

<decisions>
## Implementation Decisions

### STAB-05: download.rs let-else guard

- **D-01**: Replace triple-unwrap pattern in `src/commands/download.rs` (lines 31, 40, 72) with `let Some(attachments) = &email.attachments else { return Ok(()) }` guard at the top of the attachment processing block. Eliminates `.unwrap()` calls entirely.

### STAB-08: Stable hasher for contact fallback IDs

- **D-02**: Replace `DefaultHasher` at `src/carddav/mod.rs:866` with `SipHasher13` from the `siphasher` crate (de facto stable Rust hashing). Alternative: `twox_hash::XxHash64`. Choosing `siphasher` for cryptographic stability and stdlib-compat API.
- **D-03**: Add `siphasher = "1"` to Cargo.toml `[dependencies]`.
- **D-04**: Fallback IDs must be byte-identical across Rust versions — verify with a golden-value test.

### QUAL-01: Remove stale allow attributes

- **D-05**: Audit every `#[allow(unused_imports)]` annotation in src/. For each, check if the import is actually used; if yes, remove the attribute. If the import is truly unused, remove the import instead.
- **D-06**: Also run `cargo clippy --all-targets --all-features -- -D warnings` and fix any warnings that surface after cleanup.

### Claude's Discretion

- Exact siphasher key/seed (two u64 values) — use deterministic public values
- Whether to also audit `#[allow(dead_code)]` etc. (scope could creep — stick to unused_imports per QUAL-01)

</decisions>

<code_context>
## Existing Code Insights

### Target sites

- `src/commands/download.rs:31,40,72` — triple unwrap
- `src/carddav/mod.rs:866` — DefaultHasher
- `src/jmap/mod.rs:10` — `#[allow(unused_imports)]` (noted as "used in PERF-07 15-02")
- `src/carddav/mod.rs:14` — `#[allow(unused_imports)]`

### Established Patterns

- Rust idiomatic error handling (`?`, `Result<T>`)
- Deterministic hashing for stable artifact generation

</code_context>

<specifics>
## Specific Ideas

- Unit test: hash the same input via DefaultHasher vs SipHasher13 — document the new stable value
- Unit test: hashing twice returns the same ID (obvious but confirms stability)
- CI-like check: `cargo clippy --all-targets --all-features -- -D warnings` returns exit 0

</specifics>

<deferred>
## Deferred Ideas

- Broader unwrap audit across codebase (out of scope for QUAL-01)
- dead_code / unused_mut annotation audit (not required by QUAL-01)

</deferred>

---

*Phase: 17-quality-polish*
*Context gathered: 2026-04-04*
