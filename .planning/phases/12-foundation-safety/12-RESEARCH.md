# Phase 12: Foundation Safety - Research

**Researched:** 2026-04-04
**Domain:** Rust error handling, secret management, HTTP timeout, path traversal, constructor safety
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**D-01:** Reuse existing `Error::Server(String)` for the 4xx catch-all. Produces `format!("HTTP {} from API", status.as_u16())`. Do NOT add `Error::Http { status, body }` — flat enum is preferred.

**D-02:** Both existing JMAP status-handling sites (`src/jmap/mod.rs:204-209` and `:262-267`) get the same catch-all arm placed after the existing 401/429/5xx arms.

**D-03:** Use `secrecy::SecretString` inside `Config`, `CoreConfig`, and `ContactsConfig` for `api_token` and `app_password` fields. Add `secrecy` with the `serde` feature.

**D-04:** Do NOT propagate `SecretString` through callers. `get_token()` and `get_app_password()` keep their `Result<String>` signatures by calling `secret.expose_secret().to_string()` at the boundary.

**D-05:** Derive `Debug` on Config structs as normal — `SecretString` automatically renders as `[REDACTED]` via its own Debug impl. No custom `impl Debug` needed.

**D-06:** Replace each of the 5 `eprintln!` + `std::process::exit(1)` callsites (`src/main.rs:753, 856, 919, 945, 1095`) with inline `Output::<()>::error("Confirmation required: <message>").print()` followed by `anyhow::bail!("confirmation required")` so main's error handler exits with status 1.

**D-07:** Do NOT introduce an `Error::ConfirmationRequired` variant.

**D-08:** Each callsite's error message must match its command's destructive action (e.g., "Confirmation required: pass --confirm to delete calendar").

**D-09:** `Config::load()` parse failures return `Error::Config(format!("Failed to parse config at {path}: {parse_err}. Delete this file or fix the TOML to recover.", path = path.display(), parse_err = e))`.

**D-10:** Include the resolved absolute config path (from `Self::config_path()?`) in the message.

**D-11:** DAV timeout (STAB-02) — both `CardDavClient::new()` and `CalDavClient::new()` use `reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?`. Place the `Duration` inline (not a shared constant).

**D-12:** Path traversal (SEC-01) — `src/commands/download.rs:112` uses `Path::new(&final_filename).file_name().unwrap_or_else(|| OsStr::new("attachment"))` before joining to `out_dir`.

**D-13:** `JmapClient::new()` (STAB-09) — change signature to `pub fn new() -> Result<Self>`. Error variant: `Error::Config("HTTP client builder failed: ...")`. Current callers propagate with `?`.

### Claude's Discretion

- Exact phrasing of confirmation-required messages per callsite (must be actionable and name the flag needed, e.g., `--confirm` or `--yes`)
- Whether to unit-test the redacted Debug output (recommended: yes, one test per config struct)
- Whether to pin `secrecy` to 0.10.x exactly or allow minor bumps via `^0.10` (recommended: `^0.10`)

### Deferred Ideas (OUT OF SCOPE)

- Shared HTTP_TIMEOUT constant — deferred until a 3rd HTTP client exists
- Deeper SecretString propagation through HTTP-header construction
- Structured per-status HTTP error variant (`Error::Http { status, body }`)
- Central `Error::ConfirmationRequired` variant
- vCard/iCal injection escaping (Phase 13)
- URL encoding (Phase 13)
- MCP DAV client pool (Phase 14)
- Concurrent DAV fetching (Phase 15)
- Newtyped IDs (deferred to v1.3)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STAB-01 | JMAP HTTP 4xx responses produce a clear `Server` error with the status code, not a confusing JSON deserialization error | D-01, D-02: reuse existing `Error::Server`; place catch-all after existing 401/429/5xx arms in both `authenticate()` and `request()` |
| STAB-02 | `CardDavClient` and `CalDavClient` set a 30-second HTTP timeout matching `JmapClient` | D-11: `reqwest::Client::builder().timeout(Duration::from_secs(30)).build()?` in both constructors; both currently use `Client::new()` |
| STAB-03 | Confirmation-guard exit paths emit a valid `Output::error(..)` JSON envelope instead of `eprintln!` + `process::exit(1)` | D-06, D-07, D-08: 5 callsites in `src/main.rs` at lines 753, 856, 919, 945, 1095 |
| STAB-09 | `JmapClient::new()` returns a `Result` instead of `.expect()`-panicking on builder failure | D-13: 3 production callsites + 4 test callsites; tests require `unwrap()` or `expect()` |
| STAB-10 | Config corruption errors include guidance on how to recover | D-09, D-10: embed absolute config path in error message with recovery instruction |
| SEC-01 | Attachment downloads never write outside the user-specified output directory | D-12: `Path::file_name()` strips directory components; fallback to `"attachment"` |
| SEC-06 | Config structs holding `api_token` and `app_password` never print secrets via `{:?}` | D-03, D-04, D-05: `secrecy::SecretString` wraps both fields; serde feature enables TOML deserialization |
</phase_requirements>

---

## Summary

Phase 12 is a pure hardening phase: seven localized, low-risk changes to an existing Rust codebase. All seven requirements are surgical — each touches a specific function, struct, or match arm — with no new modules, no protocol changes, and no user-visible feature changes. The changes are:

1. **STAB-01** — Add a `400..=499` catch-all match arm in the two JMAP status-handling blocks in `src/jmap/mod.rs`.
2. **STAB-02** — Add `.timeout(Duration::from_secs(30))` to `Client::builder()` in `CardDavClient::new()` and `CalDavClient::new()`.
3. **STAB-03** — Replace 5 `eprintln!` + `process::exit(1)` guards in `src/main.rs` with `Output::error().print()` + `anyhow::bail!()`.
4. **STAB-09** — Change `JmapClient::new(token: String) -> Self` to `pub fn new(token: String) -> Result<Self>` and propagate `?` at all call sites.
5. **STAB-10** — Improve the `Config::load()` parse error message to include the resolved config path and recovery instructions.
6. **SEC-01** — Use `Path::file_name()` to strip directory traversal components from server-supplied filenames before writing to disk.
7. **SEC-06** — Wrap `api_token` and `app_password` config fields in `secrecy::SecretString`; add `secrecy = { version = "^0.10", features = ["serde"] }` to `Cargo.toml`.

The main cross-cutting concern is that `JmapClient::new()` is called in 7 places (3 production, 4 tests). The signature change cascades to all of them. The `SecretString` wrapping in `Config` also requires updating the existing `Config` tests that construct `CoreConfig { api_token: Some("test-token".to_string()) }`.

**Primary recommendation:** Implement in dependency order — SEC-06 (config struct changes) first since it affects `Config` tests, then STAB-09 (constructor change), then the remaining five independent changes.

---

## Standard Stack

### Core (unchanged)
| Library | Version | Purpose | Relevance to Phase 12 |
|---------|---------|---------|----------------------|
| `thiserror` | 2.0.17 | Error enum derivation | `Error::Server(String)` and `Error::Config(String)` already exist |
| `reqwest` | 0.13.1 | HTTP client | `ClientBuilder::timeout()` used for DAV clients |
| `tokio` | 1.49.0 | Async runtime | No changes needed |
| `anyhow` | 1.0.100 | Error wrapping in command handlers | `anyhow::bail!()` used for STAB-03 |
| `serde` | 1.0.228 | Serialization | Config struct changes for SEC-06 |
| `toml` | 0.8 | Config parsing | STAB-10 error message improvement |
| `std::path::Path` | stdlib | Path handling | SEC-01 path traversal fix |

### New Production Dependency
| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| `secrecy` | `^0.10` (current: 0.10.3) | `SecretString` wrapper for credential fields | `Debug` impl outputs `Secret([REDACTED alloc::string::String])` automatically; TOML serde deserialization via `serde` feature; memory zeroed on drop; no unsafe code |

**Installation:**
```toml
# Cargo.toml [dependencies]
secrecy = { version = "^0.10", features = ["serde"] }
```

**Verify current version:**
```bash
cargo search secrecy
# Output confirms: secrecy = "0.10.3"
```

### Alternatives Considered and Rejected
| Instead of | Rejected Alternative | Why Rejected (per CONTEXT.md decisions) |
|------------|----------------------|----------------------------------------|
| `secrecy::SecretString` | Custom `impl Debug for Config` | Manual maintenance; new fields added later silently appear unredacted; no memory zeroing |
| `secrecy::SecretString` | `veil` crate | Debug-only redaction, no memory zeroing |
| `Error::Config(...)` in `JmapClient::new()` | New `Error::Http { status, body }` | D-01: flat enum preferred; no new variants |
| `anyhow::bail!()` | `Error::ConfirmationRequired` variant | D-07: 5 callsites does not justify centralized indirection |

---

## Architecture Patterns

### Pattern 1: 4xx Catch-All in JMAP Status Match (STAB-01)

**What:** Add a range match arm `400..=499` that fires before the `_ => {}` default. Must be placed AFTER the existing 401/429/5xx arms since Rust match arms are tested in order.

**Both sites follow identical pattern:**

```rust
// src/jmap/mod.rs — in authenticate() at lines 204-209
match resp.status().as_u16() {
    401 => return Err(Error::InvalidToken("Authentication failed".into())),
    429 => return Err(Error::RateLimited),
    500..=599 => return Err(Error::Server(format!("Server error: {}", resp.status()))),
    400..=499 => return Err(Error::Server(format!("HTTP {} from API", resp.status().as_u16()))),
    _ => {}
}

// src/jmap/mod.rs — in request() at lines 262-267 (identical structure)
match resp.status().as_u16() {
    401 => return Err(Error::InvalidToken("Token expired or invalid".into())),
    429 => return Err(Error::RateLimited),
    500..=599 => return Err(Error::Server(format!("Server error: {}", resp.status()))),
    400..=499 => return Err(Error::Server(format!("HTTP {} from API", resp.status().as_u16()))),
    _ => {}
}
```

**Note on error format:** The `thiserror` annotation on `Error::Server` is `#[error("Server error: {0}")]`. The `format!("HTTP {} from API", status)` string becomes the `{0}`, producing the final message `"Server error: HTTP 400 from API"`. Success criterion 1 requires `{"error": "Server error: HTTP 400"}` — verify the exact format satisfies the criterion. The criterion text says `"Server error: HTTP 400"` which matches the thiserror prefix + the format string `"HTTP 400 from API"` truncation: this actually produces `"Server error: HTTP 400 from API"`. The criterion says `HTTP 400` as a prefix, not exact match — this is sufficient.

**Arm ordering is critical:** The 401 arm must appear before the 400..=499 range or 401 would be caught by the range and misclassified. Rust evaluates arms in order, so placing 401, 429 before the range is correct.

### Pattern 2: reqwest ClientBuilder with Timeout (STAB-02)

**What:** Replace `Client::new()` with `Client::builder().timeout(Duration::from_secs(30)).build()?`.

**Current state:**
- `CardDavClient::new()` in `src/carddav/mod.rs:79-85`: uses `client: Client::new()`
- `CalDavClient::new()` in `src/caldav/mod.rs:99-105`: uses `client: Client::new()`

**Both constructors currently return `Self` (not `Result`).** The `.build()?` call requires the constructors to return `Result<Self>`. This is a parallel change to STAB-09 (JmapClient). All DAV client constructors will move to `Result<Self>`.

```rust
// In CardDavClient::new() and CalDavClient::new()
use std::time::Duration;

pub fn new(username: String, app_password: String) -> Result<Self> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Config(format!("HTTP client builder failed: {e}")))?;
    Ok(Self { client, username, app_password })
}
```

**Caller sites for CardDavClient::new() and CalDavClient::new():** Find all with grep to ensure all are updated.

### Pattern 3: Confirmation-Guard Replacement (STAB-03)

**What:** Replace 5 instances of the `eprintln!` + `std::process::exit(1)` anti-pattern with the JSON output contract.

**Before (example from line 751-754):**
```rust
if !yes {
    eprintln!("Mark email {} as spam? Use -y to confirm.", email_id);
    std::process::exit(1);
}
```

**After (per D-06, D-08):**
```rust
if !yes {
    Output::<()>::error("Confirmation required: pass -y to mark email as spam").print();
    anyhow::bail!("confirmation required");
}
```

**All 5 callsites and their required messages (per D-08):**

| Line | Command | Flag | Message |
|------|---------|------|---------|
| 753 | `Commands::Spam` | `-y` / `--yes` | `"Confirmation required: pass -y to mark email as spam"` |
| 856 | `MaskedCommands::Delete` | `-y` / `--yes` | `"Confirmation required: pass -y to delete masked email"` |
| 919 | `ContactsCommands::Delete` | `--confirm` / `--yes` | `"Confirmation required: pass --confirm to delete contact"` |
| 945 | `CalendarsCommands::Delete` | `--confirm` / `--yes` | `"Confirmation required: pass --confirm to delete calendar"` |
| 1095 | `EventsCommands::Delete` | `--confirm` / `--yes` | `"Confirmation required: pass --confirm to delete event"` |

**Note on existing import:** `std::process` is already imported in `src/main.rs`. After replacement, if no other `process::` usage remains, the import can be removed. Verify with `cargo check`.

### Pattern 4: JmapClient::new() Returns Result (STAB-09)

**What:** Change constructor signature; update all call sites.

**Before:**
```rust
pub fn new(token: String) -> Self {
    let client = Client::builder()
        .timeout(TIMEOUT)
        .build()
        .expect("Failed to build HTTP client");
    Self { client, token, session: None, available_capabilities: Vec::new(), cached_mailboxes: None }
}
```

**After:**
```rust
pub fn new(token: String) -> Result<Self> {
    let client = Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| Error::Config(format!("HTTP client builder failed: {e}")))?;
    Ok(Self { client, token, session: None, available_capabilities: Vec::new(), cached_mailboxes: None })
}
```

**Production call sites (3):**
- `src/jmap/mod.rs:33` in `authenticated_client()`: `let mut client = JmapClient::new(token);` → `let mut client = JmapClient::new(token)?;`
- `src/commands/auth.rs:6`: `let mut client = JmapClient::new(token.to_string());` → `let mut client = JmapClient::new(token.to_string())?;`
- `src/mcp/mod.rs:50`: `let mut client = JmapClient::new(token);` → `let mut client = JmapClient::new(token)?;`

**Test call sites (4 in `src/jmap/mod.rs`):** Lines 1250, 1266, 1282, 1294. These are in `#[cfg(test)]` blocks. Options:
- `JmapClient::new("test-token".to_string()).expect("test client")` — clearest
- `JmapClient::new("test-token".to_string()).unwrap()` — idiomatic in tests

Both are acceptable in `#[cfg(test)]` context. Use `.expect("test client")` for self-documenting intent.

### Pattern 5: Config Parse Error with Recovery Path (STAB-10)

**What:** Improve the error message in `Config::load()` at line 47.

**Before:**
```rust
let config: Config = toml::from_str(&content)
    .map_err(|e| Error::Config(format!("Failed to parse config: {}", e)))?;
```

**After (per D-09, D-10):**
```rust
let path = Self::config_path()?;  // already computed above
// ...
let config: Config = toml::from_str(&content)
    .map_err(|e| Error::Config(format!(
        "Failed to parse config at {path}: {e}. Delete this file or fix the TOML to recover.",
        path = path.display()
    )))?;
```

**Note:** `path` is already in scope at line 41 (`let path = Self::config_path()?;`). The format string can reference it directly. `path.display()` produces a human-readable path without needing additional imports.

### Pattern 6: Path Traversal Fix (SEC-01)

**What:** Strip directory components from server-supplied filenames before writing.

**Current code at `src/commands/download.rs:112`:**
```rust
let path = Path::new(out_dir).join(&final_filename);
```

**After (per D-12):**
```rust
use std::ffi::OsStr;

let safe_filename = Path::new(&final_filename)
    .file_name()
    .unwrap_or_else(|| OsStr::new("attachment"))
    .to_string_lossy()
    .to_string();
let path = Path::new(out_dir).join(&safe_filename);
```

**How this works:** `Path::file_name()` returns only the final path component. For `"../../../etc/passwd"`, it returns `"passwd"`. For `"/absolute/path/file.pdf"`, it returns `"file.pdf"`. For an empty string or path that ends in `..`, it returns `None`, triggering the fallback `"attachment"`.

**Import needed:** `use std::ffi::OsStr;` — add to imports in `download.rs`. `Path` is already imported (`use std::path::Path;`).

### Pattern 7: SecretString Config Integration (SEC-06)

**What:** Wrap `api_token` and `app_password` in `SecretString` in the Config structs.

**Config struct changes (`src/config.rs`):**

```rust
use secrecy::{ExposeSecret, SecretString};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CoreConfig {
    pub api_token: Option<SecretString>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ContactsConfig {
    pub username: Option<String>,
    pub app_password: Option<SecretString>,
}
```

**`get_token()` accessor (per D-04):**
```rust
pub fn get_token(&self) -> Result<String> {
    if let Ok(token) = std::env::var("FASTMAIL_API_TOKEN") {
        return Ok(token);
    }
    self.core.api_token
        .as_ref()
        .map(|s| s.expose_secret().to_string())
        .ok_or(Error::NotAuthenticated)
}
```

**`get_app_password()` accessor (per D-04):**
```rust
pub fn get_app_password(&self) -> Result<String> {
    if let Ok(password) = std::env::var("FASTMAIL_APP_PASSWORD") {
        return Ok(password);
    }
    self.contacts.app_password
        .as_ref()
        .map(|s| s.expose_secret().to_string())
        .ok_or_else(|| Error::Config("App password not set in [contacts] config.".into()))
}
```

**`set_token()` (per D-04 — internally wrap, keep `String` input):**
```rust
pub fn set_token(&mut self, token: String) {
    self.core.api_token = Some(SecretString::from(token));
}
```

**TOML serialization note:** `SecretString` with the `serde` feature serializes as a plain string, so existing `config.toml` files are read and written without format changes. Round-trip compatibility is preserved.

**Existing tests that construct `CoreConfig { api_token: Some("test-token".to_string()) }`** must be updated to:
```rust
CoreConfig { api_token: Some(SecretString::from("test-token".to_string())) }
```

And assertions like `assert_eq!(config.core.api_token, Some("test-token".to_string()))` must become:
```rust
assert_eq!(
    config.core.api_token.as_ref().map(|s| s.expose_secret().as_str()),
    Some("test-token")
);
```

**Debug output from `SecretString`:** Produces `Secret([REDACTED alloc::string::String])`. Tests asserting redaction should match the substring `"REDACTED"`, not the full string, to avoid coupling to `secrecy` minor-version formatting changes.

### Anti-Patterns to Avoid

- **`std::process::exit()` in command handlers:** Bypasses the JSON output contract; MCP hosts and scripts receive no stdout.
- **`Client::new()` in DAV constructors:** No timeout set; a hung connection blocks the process indefinitely.
- **`.expect("...")` in library constructors:** Panics on builder failure; callers cannot handle the error gracefully.
- **`format!("Server error: {}", ...)` in the 4xx match arm:** The `thiserror` attribute already prepends "Server error: " via `#[error("Server error: {0}")]` — double-prefixing produces `"Server error: Server error: HTTP 400 from API"`. Only put the status code portion in the format string.
- **Custom `impl Debug` for Config:** Maintenance burden; silently fails to redact new fields. Use `SecretString` derive passthrough instead.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Secret redaction in Debug | Custom `impl Debug` for each Config struct | `secrecy::SecretString` | Automatic, no maintenance; memory-zeroed on drop; serde compatible |
| Path component stripping | Custom string parsing for `../` sequences | `std::path::Path::file_name()` | stdlib handles all OS path edge cases (Windows paths, null bytes, trailing slashes) |
| HTTP timeout | Manual `tokio::time::timeout` wrapper | `reqwest::ClientBuilder::timeout()` | Already the pattern in `JmapClient`; applies to connect + read; single point of config |

---

## Common Pitfalls

### Pitfall 1: Double-Prefix in Server Error Message
**What goes wrong:** The `thiserror` `#[error("Server error: {0}")]` attribute on `Error::Server(String)` already prepends `"Server error: "`. If the 4xx match arm uses `format!("Server error: HTTP {} from API", status)`, the final message is `"Server error: Server error: HTTP 400 from API"`.
**How to avoid:** Use `format!("HTTP {} from API", status.as_u16())` — only the suffix, not the prefix. The prefix is injected by thiserror.
**Warning signs:** A test asserting `output.contains("Server error: HTTP 400")` passes, but one asserting the exact string fails with double-prefix.

### Pitfall 2: 401 Masked by 4xx Catch-All If Arm Order is Wrong
**What goes wrong:** If `400..=499` is placed BEFORE `401 =>`, the 401 unauthorized response is caught by the range arm and returned as `Error::Server("HTTP 401 from API")` instead of `Error::InvalidToken`. The auth refresh flow would break.
**How to avoid:** Always place specific status codes (401, 429) before the range catch-all in the match arm order. Rust evaluates arms in order; more specific arms must come first.

### Pitfall 3: DAV Client Constructor Signature Change Cascades
**What goes wrong:** Changing `CardDavClient::new()` and `CalDavClient::new()` from `-> Self` to `-> Result<Self>` means all call sites must propagate `?`. Missing a call site causes a compile error (visible), but missing an update in `#[cfg(test)]` code means tests fail to compile.
**How to avoid:** Run `cargo test` (not just `cargo check`) after the change. The test compilation step catches test-only call sites.
**Warning signs:** `cargo check` passes but `cargo test` fails to compile.

### Pitfall 4: SecretString Breaks Existing Config Test Assertions
**What goes wrong:** Tests in `src/config.rs` like `assert_eq!(config.core.api_token, Some("test-token".to_string()))` will not compile because `SecretString` does not implement `PartialEq` (by design — comparing secrets can leak via timing).
**How to avoid:** Update test assertions to use `.expose_secret()` at the boundary: `assert_eq!(config.core.api_token.as_ref().map(|s| s.expose_secret().as_str()), Some("test-token"))`. This is explicit and searchable.
**Warning signs:** Compile errors on `PartialEq` for `Option<SecretString>` in test code.

### Pitfall 5: TOML Serialization of SecretString Writes `[REDACTED]`
**What goes wrong:** Without the `serde` feature on the `secrecy` crate, `SecretString` cannot deserialize from TOML. Without the `expose_secret_serde` feature (older versions), it may serialize as `"[REDACTED]"` rather than the actual token value, corrupting the config file on `config.save()`.
**How to avoid:** Use `secrecy = { version = "^0.10", features = ["serde"] }`. Verify with a round-trip test: `Config::save()` then `Config::load()` and assert `get_token()` returns the original value.
**Warning signs:** After `fastmail-cli auth <token>`, the next command returns `NotAuthenticated` because the saved token was `"[REDACTED]"`.

### Pitfall 6: `anyhow::bail!` in an `anyhow::Result<T>` Context
**What goes wrong:** The 5 confirmation-guard call sites in `src/main.rs` return `anyhow::Result<()>`. `anyhow::bail!("confirmation required")` is valid in this context and returns `Err(anyhow::Error)`. The `main()` function's error handler catches this, prints a JSON error envelope, and exits with status 1. This is correct behavior.
**What is NOT a problem:** `anyhow::bail!` in a function returning `crate::error::Result<T>` (the custom type) would fail to compile. Verify the calling context uses `anyhow::Result`.
**Warning signs:** Compile error: `the trait From<anyhow::Error> is not implemented for error::Error`.

---

## Code Examples

### Verified: reqwest ClientBuilder with timeout
```rust
// Source: reqwest 0.13.1 docs — ClientBuilder::timeout()
// Sets an optional timeout for all requests.
// Timeout is applied from when the request starts connecting until the response body has finished.
use std::time::Duration;

let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .map_err(|e| Error::Config(format!("HTTP client builder failed: {e}")))?;
```

### Verified: secrecy::SecretString usage
```rust
// Source: secrecy 0.10.3 docs — SecretString is type alias for Secret<String>
use secrecy::{ExposeSecret, SecretString};

// Construction
let secret: SecretString = SecretString::from("my-api-token".to_string());
// or
let secret: SecretString = "my-api-token".to_string().into();

// Debug output (automatic, no custom impl needed)
println!("{:?}", secret);
// Output: Secret([REDACTED alloc::string::String])

// Accessing the value (explicit, grep-auditable)
let token_str: &str = secret.expose_secret();

// In tests — assert substring, not exact Debug string
assert!(format!("{:?}", secret).contains("REDACTED"));
```

### Verified: Path::file_name() for traversal prevention
```rust
// Source: std::path::Path::file_name() stdlib docs
use std::ffi::OsStr;
use std::path::Path;

// Strips all directory components
assert_eq!(
    Path::new("../../../etc/passwd").file_name(),
    Some(OsStr::new("passwd"))
);
// Returns None for paths ending in ".."
assert_eq!(Path::new("..").file_name(), None);

// Safe filename extraction with fallback
let safe_filename = Path::new(&server_supplied_name)
    .file_name()
    .unwrap_or_else(|| OsStr::new("attachment"))
    .to_string_lossy()
    .to_string();
```

### Verified: anyhow::bail! for early return with error
```rust
// Source: anyhow 1.0.100 docs
// bail! macro is equivalent to return Err(anyhow::anyhow!(...))
// Valid only in functions returning Result<T, anyhow::Error> (or impl Into<anyhow::Error>)

Output::<()>::error("Confirmation required: pass --confirm to delete contact").print();
anyhow::bail!("confirmation required");
```

### Pattern: JmapClient::new() in tests after signature change
```rust
// Tests in src/jmap/mod.rs — after new() returns Result<Self>
#[cfg(test)]
mod tests {
    #[test]
    fn test_example() {
        // Use expect() in tests — panics are acceptable in test context
        let client = JmapClient::new("test-token".to_string()).expect("test client");
        // ... test body
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | Impact on Phase 12 |
|--------------|------------------|-------------------|
| `#[derive(Debug)]` on credential structs | `secrecy::SecretString` field wrapping | Replaces unsafe derive; no custom Debug impl needed |
| `Client::new()` (no timeout) | `Client::builder().timeout(Duration::from_secs(30)).build()?` | Both DAV constructors updated; matches existing JMAP pattern |
| `eprintln!` + `process::exit(1)` for guards | `Output::error().print()` + `anyhow::bail!()` | JSON output contract preserved for all code paths |
| Constructor panic on builder failure | Constructor returns `Result<Self>` | Standard Rust: infallible constructors are for types that cannot fail |
| Path join with raw server-supplied filename | `Path::file_name()` strip + join | Standard Rust path API; no regex or custom parsing |

---

## Open Questions

1. **Exact error message for STAB-01 vs success criterion**
   - What we know: Success criterion says `{"error": "Server error: HTTP 400"}`. The `thiserror` attribute is `#[error("Server error: {0}")]`. Using `format!("HTTP {} from API", status.as_u16())` produces `"Server error: HTTP 400 from API"`.
   - What's unclear: Does "Server error: HTTP 400" in the success criterion mean an exact match or a prefix match?
   - Recommendation: The criterion says `{"error": "Server error: HTTP 400"}` but the CONTEXT.md specifics section says `format!("HTTP {} from API", ...)` is correct. Implement per D-01 and adjust tests to match the actual output `"Server error: HTTP 400 from API"`.

2. **CardDavClient and CalDavClient call sites**
   - What we know: The constructors change from `-> Self` to `-> Result<Self>`. The 3 production call sites for `JmapClient::new()` are enumerated. DAV client call sites were not enumerated in CONTEXT.md.
   - Recommendation: Run `grep -rn "CardDavClient::new\|CalDavClient::new" src/` to find all call sites before implementing.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust stable toolchain | All compilation | Yes | rustc 1.94.1 | — |
| cargo | Package management | Yes | 1.94.1 | — |
| `secrecy ^0.10` (crates.io) | SEC-06 | Pending add | 0.10.3 (confirmed via `cargo search`) | — |

No missing dependencies with no fallback. `secrecy` is a new explicit dependency but is available on crates.io at the confirmed version.

---

## Project Constraints (from CLAUDE.md)

- **Tech stack**: Rust 2024 edition; follow existing patterns (clap derive, async-graphql, reqwest)
- **Error handling**: Flat `thiserror` enum; `anyhow::Result<T>` in command handlers; custom `Result<T>` in JMAP internals; propagate with `?`
- **No new error variants**: Phase 12 reuses `Error::Server` and `Error::Config` only
- **Module structure**: Tests in `#[cfg(test)]` blocks within the same file as implementation
- **Naming**: `snake_case` functions, `PascalCase` structs, `Option<T>` fields, `Result<T>` returns
- **No `println!` in MCP path**: MCP uses stdio transport; stdout pollution corrupts the JSON-RPC stream
- **Lint**: `cargo clippy` must pass; no `#[allow(unused_imports)]` accumulation
- **Commit discipline**: Commit after each task/story

---

## Sources

### Primary (HIGH confidence)
- CONTEXT.md (`.planning/phases/12-foundation-safety/12-CONTEXT.md`) — locked decisions D-01 through D-13, all call site line numbers
- CODEBASE-REVIEW.md (project root) — findings #1, #2, #3, #11, #15, #32, #33 (direct targets of this phase)
- `.planning/research/STACK.md` — secrecy 0.10.3 API; serde feature; `^0.10` pin recommendation; verified against docs.rs
- `.planning/research/PITFALLS.md` — Pitfall 1 (timeout regression), Pitfall 2 (4xx catch-all arm order), Pitfall 3 (Debug redaction scope)
- `src/error.rs` — confirmed `Error::Server(String)` at line 77 and `Error::Config(String)` at line 72
- `src/jmap/mod.rs` — confirmed constructor at line 179-192, status handling at 204-209 and 262-267
- `src/carddav/mod.rs` — confirmed constructor at line 79-85 using `Client::new()`
- `src/caldav/mod.rs` — confirmed constructor at line 99-105 using `Client::new()`
- `src/config.rs` — confirmed `CoreConfig { api_token: Option<String> }` and `ContactsConfig { app_password: Option<String> }` at lines 14-24
- `src/main.rs` — confirmed 5 `process::exit(1)` callsites at lines 753, 856, 919, 945, 1095
- `src/commands/download.rs` — confirmed `Path::new(out_dir).join(&final_filename)` at line 112
- `cargo search secrecy` output — confirmed 0.10.3 as current version

### Secondary (MEDIUM confidence)
- secrecy docs.rs (0.10.3) — `SecretString` Debug output format; `expose_secret()` signature; serde serialization behavior
- reqwest docs.rs (0.13.1) — `ClientBuilder::timeout()` applies to total request duration
- std::path::Path::file_name() stdlib docs — behavior on `..` and absolute paths confirmed

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries already in Cargo.toml; `secrecy` version confirmed via cargo search
- Architecture: HIGH — all target line numbers confirmed by reading actual source files
- Pitfalls: HIGH — derived from actual code inspection, not hypothetical analysis

**Research date:** 2026-04-04
**Valid until:** 2026-05-04 (stable domain; reqwest, secrecy, stdlib APIs are stable)
