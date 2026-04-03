# Phase 2: vCard Serialization - Research

**Researched:** 2026-03-27
**Domain:** vCard 3.0 serialization (RFC 2426 / RFC 6350), Rust string processing, UUID v4
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** N property derived from single `name` (FN) field by splitting on whitespace: first token = given name, last token = family name, middle tokens joined as middle name. No prefix/suffix handling.
- **D-02:** FN written verbatim from `Contact.name`. N is the split decomposition. Both always present per vCard 3.0 requirement.
- **D-03:** Address is a single freeform string. Serialized into ADR street-address component with all other structured components left empty. Format: `ADR:;;{street};;;;;\r\n`
- **D-04:** `address: Option<String>` field added to Contact struct in this phase. GqlContact must be updated to match.
- **D-05:** Full rewrite strategy — always generate complete vCard from Contact struct fields. Unknown/unmodified properties from original vCard are not preserved.
- **D-06:** `serialize_vcard()` function lives in `src/carddav/mod.rs` alongside `parse_vcard()`.

### Claude's Discretion

- Exact line folding implementation (fold at 75 octets per RFC 6350, using CRLF + space continuation)
- Character escaping strategy for semicolons, commas, backslashes in property values
- UUID v4 generation approach (use `uuid` crate or manual implementation)
- Whether to add the `address` field to Contact in this phase or defer to Phase 3
- Test structure and specific test fixtures beyond the success criteria

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| VCARD-01 | Generate valid vCard 3.0 with FN, N, EMAIL, ORG, TEL, ADR, NOTE properties | RFC 2426 property format verified; TYPE parameter patterns from existing parse_vcard |
| VCARD-02 | Line folding at 75 octets with CRLF line endings per RFC 6350 | RFC 6350 §3.2 text verified; fold_line() implementation pattern documented |
| VCARD-03 | UUID v4 generation for new contact UIDs | uuid crate 1.23.0 with `v4` feature confirmed; already in transitive deps at 1.22.0 |
</phase_requirements>

## Summary

Phase 2 implements a pure-function vCard 3.0 serializer in Rust. The serializer accepts a `Contact` struct and returns a `String` containing a valid vCard. No network access is required and all behavior is fully testable with inline fixtures. The function signature will be `pub fn serialize_vcard(contact: &Contact) -> String` (infallible because all inputs are valid UTF-8 strings and vCard generation cannot fail given the constraints in CONTEXT.md).

The three main technical concerns are: (1) line folding at 75 octets per RFC 6350 §3.2 — measuring bytes not chars for UTF-8 correctness; (2) character escaping per RFC 2426 §5 — backslash-escaping semicolons, commas, backslashes, and newlines in property values; (3) UUID v4 generation using the `uuid` crate (1.23.0, `v4` feature). The `uuid` crate is already present transitively in Cargo.lock at 1.22.0, so adding it explicitly to `[dependencies]` with `v4` feature is a minor, safe change.

The serializer is the inverse of the existing `parse_vcard()` function. Output from `serialize_vcard()` must round-trip through `parse_vcard()` for all Contact fields. The existing `unfold_vcard()` function provides the reference implementation for the inverse operation (unfolding), which informs the folding algorithm.

**Primary recommendation:** Implement `serialize_vcard()` as a pure infallible function in `src/carddav/mod.rs`. Add the `uuid` crate with `v4` feature to Cargo.toml. Add `address: Option<String>` to Contact and GqlContact. Cover all RFC behavior with `#[cfg(test)]` unit tests in the same file.

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| uuid | 1.23.0 | UUID v4 generation | Standard Rust UUID crate; already transitively present at 1.22.0; `v4` feature gives `Uuid::new_v4()` |
| std (Rust stdlib) | - | String building, UTF-8 byte counting | `str::len()` counts bytes (UTF-8 octets), which is exactly what RFC 6350 requires for fold measurement |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| No additional libraries needed | - | - | All serialization is pure string manipulation using Rust stdlib |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| uuid crate | Manual UUID via random bytes | uuid crate is the standard; manual requires getrandom/rand dependency and error-prone formatting |
| Inline string builder | `vcard` / `vobject` crate | No established maintained vCard crate matches the project's Fastmail-specific needs; hand-rolling the small subset needed is simpler |

**Installation:**
```bash
# In Cargo.toml [dependencies]:
uuid = { version = "1.23.0", features = ["v4"] }
```

**Version verification:** uuid 1.23.0 confirmed via `cargo search uuid` on 2026-03-27. Transitive dependency already at 1.22.0 in Cargo.lock — adding explicitly will resolve to 1.23.0 (semver compatible).

## Architecture Patterns

### Recommended Project Structure

No new files. All changes are in existing files:

```
src/
├── carddav/
│   └── mod.rs     # Add: address field to Contact struct
│                  # Add: serialize_vcard() pub function
│                  # Add: fold_line() private helper
│                  # Add: escape_value() private helper
│                  # Add: unit tests in #[cfg(test)] block
└── mcp/
    └── graphql/
        └── types.rs  # Add: address field to GqlContact and From<Contact> impl
```

### Pattern 1: Line Folding at 75 Octets

**What:** Split a single vCard property line (e.g., `FN:Long Name...`) into 75-octet physical lines with CRLF + space continuation.

**When to use:** Applied to every output line longer than 75 octets. The 75-octet limit excludes the trailing CRLF. Measurement must use byte length (`str::len()`) not character count, since UTF-8 multi-byte characters each count as multiple octets.

**Critical constraint from RFC 6350 §3.2:** Multi-byte UTF-8 characters MUST NOT be split across a fold boundary. The fold algorithm must check that splitting at byte position N does not land mid-character. The safe approach: fold at or before 75 bytes, walking backward from position 75 to find the last valid UTF-8 character boundary using `str::is_char_boundary()`.

**Example:**
```rust
// Source: RFC 6350 §3.2 + existing unfold_vcard() pattern in src/carddav/mod.rs
fn fold_line(line: &str) -> String {
    // First physical line: 75 octets max (excluding CRLF)
    // Continuation lines: 74 octets max (1 octet is the leading space)
    const FIRST_MAX: usize = 75;
    const CONT_MAX: usize = 74; // 75 total - 1 leading space

    if line.len() <= FIRST_MAX {
        // No folding needed
        return format!("{}\r\n", line);
    }

    let mut result = String::new();
    let mut pos = 0;
    let bytes = line.as_bytes();
    let max = if pos == 0 { FIRST_MAX } else { CONT_MAX };

    // First chunk
    let mut end = FIRST_MAX.min(line.len());
    while !line.is_char_boundary(end) { end -= 1; }
    result.push_str(&line[..end]);
    result.push_str("\r\n ");
    pos = end;

    // Continuation chunks
    while pos < line.len() {
        let remaining = &line[pos..];
        if remaining.len() <= CONT_MAX {
            result.push_str(remaining);
            break;
        }
        let mut end = CONT_MAX.min(remaining.len());
        while !remaining.is_char_boundary(end) { end -= 1; }
        result.push_str(&remaining[..end]);
        result.push_str("\r\n ");
        pos += end;
    }
    result.push_str("\r\n");
    result
}
```

### Pattern 2: Character Escaping

**What:** Escape special characters in property values before serialization.

**When to use:** Applied to all text property values (FN, N components, ORG, NOTE, ADR street). NOT applied to structured delimiters (the semicolons separating N components or ADR components — those are structural).

**Characters requiring escaping (RFC 2426 §5 / RFC 6350 §3.4):**
- `\` (backslash) → `\\` (must be escaped FIRST to avoid double-escaping)
- `;` (semicolon) → `\;` (appears in compound property field values)
- `,` (comma) → `\,` (appears in multi-value fields)
- `\n` (newline U+000A) → `\n` literal (as two chars: backslash + 'n')

**Example:**
```rust
// Source: RFC 2426 §5, RFC 6350 §3.4
fn escape_value(s: &str) -> String {
    s.replace('\\', "\\\\")   // Must be first
     .replace(';', "\\;")
     .replace(',', "\\,")
     .replace('\n', "\\n")
}
```

**Note:** Semicolons in ADR structural positions (the `;;` separators between components) are NOT escaped — they are the structural delimiters. Only semicolons within a component value are escaped.

### Pattern 3: TYPE Parameter Serialization

**What:** Map the `label` field of ContactEmail/ContactPhone to a vCard `TYPE=` parameter.

**When to use:** When label is `Some(value)`. When label is `None`, omit the parameter.

**Format:**
```
EMAIL;TYPE=work:user@example.com\r\n
TEL;TYPE=cell:+15551234567\r\n
EMAIL:user@example.com\r\n      (no TYPE when label is None)
```

**Compatibility note:** The existing `parse_vcard()` splits on `TYPE=` and takes everything up to the next `:` as the label value. The serializer must produce output compatible with this parsing — `EMAIL;TYPE={label}:{email}` is correct.

### Pattern 4: N Property Decomposition

**What:** Split `Contact.name` (the FN string) into N property components per D-01.

**Algorithm:**
1. Split `name` on ASCII whitespace
2. `given_name` = tokens[0] (or empty string if name is empty)
3. `family_name` = tokens[last] if tokens.len() > 1, else empty string
4. `middle_name` = tokens[1..last-1].join(" ") if tokens.len() > 2, else empty string
5. Format: `N:{family};{given};{middle};;`

**Example:**
```
Contact.name = "John"           → N:;John;;;\r\n   (single token: given only)
Contact.name = "John Smith"     → N:Smith;John;;;\r\n
Contact.name = "John Q Smith"   → N:Smith;John;Q;;\r\n
Contact.name = "John Q R Smith" → N:Smith;John;Q R;;\r\n
```

### Pattern 5: Full vCard Structure

**What:** The complete output format for a serialized Contact.

**Template (CRLF line endings, folding applied per-line after construction):**
```
BEGIN:VCARD\r\n
VERSION:3.0\r\n
UID:{contact.id}\r\n
FN:{escape(contact.name)}\r\n
N:{escape(family)};{escape(given)};{escape(middle)};;\r\n
[EMAIL;TYPE={label}:{email}\r\n  for each email with label]
[EMAIL:{email}\r\n               for each email without label]
[TEL;TYPE={label}:{number}\r\n   for each phone with label]
[TEL:{number}\r\n                for each phone without label]
[ORG:{escape(org)}\r\n           if organization is Some]
[TITLE:{escape(title)}\r\n       if title is Some]
[ADR:;;{escape(street)};;;;\r\n  if address is Some]
[NOTE:{escape(notes)}\r\n        if notes is Some]
END:VCARD\r\n
```

**Property ordering:** Fixed order above. vCard 3.0 does not require specific ordering beyond BEGIN first and END last, but a fixed order makes tests deterministic.

### Anti-Patterns to Avoid

- **Measuring characters instead of bytes for folding:** `chars().count()` gives Unicode code points, not octets. RFC 6350 says 75 octets. Use `str::len()` (byte count). A single emoji may be 4 bytes and must not be split.
- **Escaping structural delimiters:** The semicolons in `N:Smith;John;;;` are structural — do NOT escape them. Only escape semicolons inside the component values (e.g., a name containing a literal semicolon).
- **Escaping ADR structural semicolons:** `ADR:;;{street};;;;` uses 6 semicolons as structural delimiters. Escape only semicolons within the `{street}` value itself.
- **Using `\n` instead of `\r\n` for line endings:** vCard 3.0 and 4.0 both specify CRLF. The existing `unfold_vcard()` handles both, but `serialize_vcard()` must produce `\r\n` for full spec compliance.
- **Escaping backslash after other characters:** The backslash replacement must come first in `escape_value()` or it will double-escape previously escaped characters.
- **Making serialize_vcard() return Result:** Serialization is infallible given the Contact struct's fields are valid UTF-8 strings. Returning `String` directly is the right signature.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UUID v4 generation | Custom random bytes + format | `uuid` crate with `v4` feature | Correct RFC 4122 byte layout, proper hyphen placement, variant bits — easy to get wrong manually |

**Key insight:** Line folding and character escaping are simple enough to implement directly with `std::str` methods. The only external dependency needed is `uuid` for cryptographically-correct random UUID generation.

## Common Pitfalls

### Pitfall 1: Splitting Multi-Byte UTF-8 Characters During Folding
**What goes wrong:** Byte-position fold point lands in the middle of a multi-byte UTF-8 sequence, producing an invalid byte sequence in the output.
**Why it happens:** Naive `&s[..75]` slice panics or corrupts if position 75 is mid-character.
**How to avoid:** After selecting candidate fold position N, walk backward while `!s.is_char_boundary(N)` to find the nearest valid boundary.
**Warning signs:** Test with names/notes containing non-ASCII characters (e.g., accented characters, CJK) and assert the output is valid UTF-8.

### Pitfall 2: Continuation Line Budget Miscalculation
**What goes wrong:** Continuation lines use 75 bytes total including the leading space, so only 74 bytes of content per continuation line.
**Why it happens:** RFC 6350 says "75 octets excluding the line break" but the leading space on continuation lines counts against the 75.
**How to avoid:** First line max = 75 bytes; continuation line content max = 74 bytes (75 - 1 for the leading space).
**Warning signs:** A 76-byte line that folds correctly, then a continuation line that folds incorrectly at 75 instead of 74.

### Pitfall 3: N Property With Single-Token Name
**What goes wrong:** If `Contact.name = "Alice"`, splitting gives one token. Naively assigning tokens[0] to given AND family produces `N:Alice;Alice;;;`.
**Why it happens:** Off-by-one in the decomposition logic.
**How to avoid:** Per D-01, first token = given name, last token = family name. With a single token, they are the same token. The correct output is `N:;Alice;;;` (treat single token as given name, family name empty) — confirm this against the test in CONTEXT.md's success criteria "FN and N always present."
**Warning signs:** `parse_vcard()` round-trip test failing for single-word names.

### Pitfall 4: Round-Trip Incompatibility With parse_vcard
**What goes wrong:** `serialize_vcard()` output passes RFC compliance but `parse_vcard()` cannot read it back because of subtle formatting differences.
**Why it happens:** `parse_vcard()` uses `starts_with("EMAIL")` etc. — any whitespace or unexpected parameter ordering could confuse it.
**How to avoid:** Include explicit round-trip tests: `parse_vcard(serialize_vcard(&contact))` should yield a Contact with the same field values.
**Warning signs:** A test that checks raw output bytes passes, but a round-trip test fails.

### Pitfall 5: Adding address Field Without Updating GqlContact
**What goes wrong:** `Contact.address` is added but `GqlContact` is not updated, causing a compile error in `From<Contact> for GqlContact`.
**Why it happens:** The `From<Contact>` impl in `src/mcp/graphql/types.rs` maps every field. Adding a field to `Contact` without updating the impl breaks the build.
**How to avoid:** Per CONTEXT.md D-04, update GqlContact in the same wave/task as Contact struct change. The compiler will catch this — treat any compile error in types.rs as a reminder.
**Warning signs:** `cargo check` fails with "missing field `address` in initializer of `GqlContact`."

## Code Examples

Verified patterns from the existing codebase and RFC specs:

### unfold_vcard Reference (Existing — Inverse of fold_line)
```rust
// Source: src/carddav/mod.rs:267
fn unfold_vcard(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    for line in raw.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            result.push_str(&line[1..]);
        } else {
            if !result.is_empty() { result.push('\n'); }
            result.push_str(line);
        }
    }
    result
}
```

### UUID v4 Generation
```rust
// Source: uuid crate docs (confirmed current API)
use uuid::Uuid;
let uid = Uuid::new_v4().to_string(); // → "550e8400-e29b-41d4-a716-446655440000"
```

### Cargo.toml Addition
```toml
# Add to [dependencies]:
uuid = { version = "1.23.0", features = ["v4"] }
```

### serialize_vcard Signature (per D-06 and CONTEXT.md integration point)
```rust
// src/carddav/mod.rs — public, infallible
pub fn serialize_vcard(contact: &Contact) -> String {
    // Returns complete vCard 3.0 string with CRLF line endings and folding applied
}
```

### Example Expected Output (for test fixtures)
```
BEGIN:VCARD\r\n
VERSION:3.0\r\n
UID:550e8400-e29b-41d4-a716-446655440000\r\n
FN:Alice Smith\r\n
N:Smith;Alice;;;\r\n
EMAIL;TYPE=work:alice@example.com\r\n
TEL:+15551234567\r\n
ORG:Acme Inc\r\n
NOTE:Meeting at 3pm\r\n
END:VCARD\r\n
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| vCard 2.1 QUOTED-PRINTABLE encoding | vCard 3.0: UTF-8 with backslash escaping | RFC 2426 (1998) | Project already handles QP decoding in parse_vcard; serializer only needs to write UTF-8 with backslash escaping |
| vCard 3.0 (RFC 2426) | vCard 4.0 (RFC 6350) | RFC 6350 (2011) | Fastmail uses vCard 3.0; RFC 6350 line-folding rules are consistent with RFC 2426 for practical purposes |

**Deprecated/outdated:**
- QUOTED-PRINTABLE encoding: Only needed for reading legacy vCards (already handled in `decode_qp()`). Never write QP — write UTF-8 directly.
- `\n` line endings in vCard output: Spec requires `\r\n`. The `unfold_vcard()` function tolerates both, but the serializer must write `\r\n`.

## Open Questions

1. **Single-token name N property behavior**
   - What we know: D-01 says first token = given, last token = family. For one token, first == last.
   - What's unclear: Should a single-token name produce `N:Alice;;;\r\n` (Alice as given, family empty) or `N:;Alice;;;\r\n` (Alice as given, nothing as family)?
   - Recommendation: Use `N:;Alice;;;` (single token is given name only, family empty) to match the spirit of "first token = given name, last token = family name" — when there's only one token it is the given name. Confirm with a test that round-trips through `parse_vcard()`.

2. **address field placement in serialized output**
   - What we know: D-04 says add `address: Option<String>` to Contact. ADR format is `ADR:;;{street};;;;;\r\n` per D-03. The 7-component ADR property has 6 semicolons.
   - What's unclear: ADR format in RFC 2426 is `PO Box;Extended;Street;Locality;Region;PostalCode;Country` — 7 components = 6 semicolons. `ADR:;;{street};;;;` has only 4 trailing semicolons after street (positions 4-7). Confirm: `ADR:;;{street};;;;;\r\n` has exactly 6 semicolons total.
   - Recommendation: Count carefully. `ADR:` + `;;` (PO box empty, extended empty) + `{street}` + `;;;;` (locality, region, postal, country all empty) = 6 semicolons total. That is correct.

## Environment Availability

Step 2.6: SKIPPED (no external dependencies — this is a pure code/Cargo.toml change phase, no running services or CLI tools required beyond the Rust toolchain already confirmed working).

**Note:** `cargo check` runs clean (1 warning, 0 errors) as of 2026-03-27.

## Sources

### Primary (HIGH confidence)
- RFC 6350 (datatracker.ietf.org) — line folding §3.2, character escaping §3.4
- RFC 2426 (rfc-editor.org) — vCard 3.0 N property structure, ADR structure, escaping rules
- `src/carddav/mod.rs` — existing parse_vcard(), unfold_vcard(), Contact struct, test patterns
- `src/mcp/graphql/types.rs` — GqlContact structure, From<Contact> impl
- `Cargo.toml` / `Cargo.lock` — confirmed uuid 1.22.0 already transitive; uuid 1.23.0 latest
- uuid crate docs (docs.rs/uuid) — `Uuid::new_v4()` API with `v4` feature

### Secondary (MEDIUM confidence)
- cargo search output (2026-03-27) — confirmed uuid latest = 1.23.0

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — uuid crate API verified via docs.rs; already in Cargo.lock transitively
- Architecture: HIGH — serializer is the exact inverse of existing parse_vcard(); RFC specs verified
- Pitfalls: HIGH — UTF-8 boundary issue is a standard Rust concern; structural vs. value escaping confirmed from RFC text

**Research date:** 2026-03-27
**Valid until:** 2026-09-27 (stable RFC-based domain; uuid crate API is stable)
