---
phases: 3
created: 2026-04-24
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/frontmatter.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/frontmatter.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase2: []
packages:
  - darkmatter
---

# UTF-8-Safe Byte Scanning in Frontmatter Fallback Helpers — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the multi-byte UTF-8 panic in `protect_shell_expressions()` and `protect_interpolation_expressions()` so frontmatter containing emoji or accented characters round-trips through the YAML fallback path without panicking or substituting `'?'`.

**Architecture:** Both helpers in `darkmatter/lib/src/markdown/frontmatter.rs` keep their existing structure (sentinel detection on raw ASCII bytes via `bytes[pos]`, depth tracking, placeholder generation, `Vec<(String, String)>` replacements) but replace the single-byte slice `yaml[pos..pos + 1].chars().next().unwrap_or('?')` with a char-aware advance that reads the next full scalar from `yaml[pos..]` and increments `pos` by `char.len_utf8()`. Sentinels (`$(`, `{{`, `}}`) are ASCII, so byte-level matching remains correct; only the non-matching branch needs the fix. A targeted audit of the rest of `darkmatter/lib/src/markdown/*` confirms the anti-pattern is unique to `frontmatter.rs` and is recorded as a negative finding.

**Tech Stack:** Rust 2024 edition, `serde_yaml_ng` via `biscuit-file`, existing `#[cfg(test)] mod tests` block in `frontmatter.rs`.

---

## Audit Pre-Findings (informational, no work item)

A `Grep` for the anti-pattern `result.push(...[pos..pos + 1].chars().next().unwrap_or(...))` across `darkmatter/lib/src/markdown/**/*.rs` returned exactly two matches, both in `frontmatter.rs:328` and `frontmatter.rs:371`. Sibling scanners that also walk `as_bytes()` (e.g., `compose/interpolation/lexer.rs`) only perform ASCII byte comparisons and never slice the source string by an arbitrary `pos`, so they are not affected. This negative finding is captured as a rustdoc-adjacent comment in Phase 3, Step 4. **No remediation work is required outside `frontmatter.rs`.**

---

## Phase 1: Fix `protect_shell_expressions()`

**Files:**
- Modify: `darkmatter/lib/src/markdown/frontmatter.rs:287-334` (`protect_shell_expressions` body and rustdoc)
- Test: `darkmatter/lib/src/markdown/frontmatter.rs` (existing `#[cfg(test)] mod tests` block, ~line 459+)

### Task 1.1: Write the failing helper test

- [ ] **Step 1: Add a failing test for non-ASCII input outside the sentinel span**

Append this test inside `mod tests` in `darkmatter/lib/src/markdown/frontmatter.rs` (just after `test_protect_shell_expressions_multiple`, so all shell-helper tests stay grouped):

```rust
#[test]
fn test_protect_shell_expressions_unicode_outside_sentinel_preserved() {
    // Pre-fix: this panics with "byte index N is not a char boundary"
    // because the non-matching branch sliced `yaml[pos..pos + 1]` while
    // `pos` was inside the multi-byte scalar for the emoji.
    let yaml = "key: \"\u{1F5A5}\u{FE0F} prefix $(echo hi) suffix\"";
    let (protected, replacements) = protect_shell_expressions(yaml);

    // Sentinel still detected and replaced.
    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].1, "$(echo hi)");
    assert!(protected.contains("__DM_SHELL_0__"));

    // Non-ASCII characters outside the sentinel are preserved byte-identically.
    assert!(protected.contains('\u{1F5A5}'));
    assert!(protected.contains('\u{FE0F}'));
    assert!(protected.contains("prefix"));
    assert!(protected.contains("suffix"));

    // No lossy `'?'` substitution survives in the output.
    assert!(!protected.contains('?'));
}
```

- [ ] **Step 2: Run the test and confirm it panics (or fails)**

Run from the repo root:

```bash
cargo test -p darkmatter --lib \
  markdown::frontmatter::tests::test_protect_shell_expressions_unicode_outside_sentinel_preserved \
  -- --nocapture
```

Expected: FAIL with a panic similar to `byte index 1 is not a char boundary; it is inside '🖥' (bytes 0..4) of ...` originating at `frontmatter.rs:328`.

If the panic instead originates somewhere else (e.g., a different line number after edits), stop and re-read the file before continuing — the test must reproduce the exact bug the spec describes.

### Task 1.2: Fix `protect_shell_expressions`

- [ ] **Step 3: Replace the byte-slice copy with a UTF-8-safe scalar advance**

In `darkmatter/lib/src/markdown/frontmatter.rs`, replace exactly these two lines inside `protect_shell_expressions` (currently at line 328-329):

```rust
            result.push(yaml[pos..pos + 1].chars().next().unwrap_or('?'));
            pos += 1;
```

with:

```rust
        } else if let Some(ch) = yaml[pos..].chars().next() {
            // Non-matching byte: advance by a full UTF-8 scalar so `pos`
            // always lands on a char boundary. Slicing `yaml[pos..pos + 1]`
            // here would panic when `pos` falls inside a multi-byte scalar.
            result.push(ch);
            pos += ch.len_utf8();
        } else {
            break;
        }
```

(Note: replace the original `} else {` opening of the branch as well — the new form folds the branch into `} else if let Some(ch) = ...` and adds a defensive `else { break; }`. The defensive `break` is unreachable because the `while pos < bytes.len()` loop guard guarantees at least one char remains, but it keeps the function panic-free under any future edit to the loop guard.)

- [ ] **Step 4: Update the rustdoc on `protect_shell_expressions` with the UTF-8-boundary note**

Add one line to the existing rustdoc block above `fn protect_shell_expressions`. Insert it as the final paragraph of the doc comment (before the `fn` line), so the resulting tail of the doc reads:

```rust
/// ...
/// The scanner is quoting-agnostic: it does not attempt to understand shell
/// quoting rules, because any valid shell `$(...)` must have balanced
/// parentheses regardless of inner quoting. Backtick substitutions and process
/// substitutions (`<(...)`, `>(...)`) are not protected — add them when a
/// real case emerges.
///
/// Non-matched content advances by full UTF-8 scalar boundaries (via
/// `yaml[pos..].chars().next()` + `char.len_utf8()`) so `pos` never lands
/// inside a multi-byte scalar.
fn protect_shell_expressions(yaml: &str) -> (String, Vec<(String, String)>) {
```

- [ ] **Step 5: Run the new test and verify it passes**

```bash
cargo test -p darkmatter --lib \
  markdown::frontmatter::tests::test_protect_shell_expressions_unicode_outside_sentinel_preserved \
  -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run the full frontmatter test module to verify no regressions**

```bash
cargo test -p darkmatter --lib markdown::frontmatter::tests:: -- --nocapture
```

Expected: every test in the module passes (the pre-existing tests `test_protect_shell_expressions_basic`, `test_protect_shell_expressions_nested_parens`, `test_protect_shell_expressions_multiple`, `test_parse_frontmatter_with_nested_quotes_in_shell_expression`, and the interpolation-side tests still pass; the new unicode test also passes).

- [ ] **Step 7: Commit Phase 1**

```bash
git add darkmatter/lib/src/markdown/frontmatter.rs
git commit -m "fix(darkmatter): advance protect_shell_expressions by UTF-8 scalars

Replace the single-byte slice copy in the non-matching branch of
protect_shell_expressions with a char-aware advance using
yaml[pos..].chars().next() and char.len_utf8(). Multi-byte scalars
(emoji, accented characters) outside a \$(...) span no longer panic
the helper, and the lossy '?' fallback is removed. Sentinel detection,
nested-paren depth tracking, unbalanced-tail behavior, and placeholder
generation are unchanged."
```

---

## Phase 2: Fix `protect_interpolation_expressions()`

**Files:**
- Modify: `darkmatter/lib/src/markdown/frontmatter.rs:339-377` (`protect_interpolation_expressions` body and rustdoc)
- Test: `darkmatter/lib/src/markdown/frontmatter.rs` (existing `#[cfg(test)] mod tests` block)

### Task 2.1: Write the failing helper test

- [ ] **Step 1: Add a failing test for non-ASCII input outside the sentinel span**

Append this test inside `mod tests`, grouped near the other interpolation helper tests (e.g., just after `test_protect_multiple_expressions`):

```rust
#[test]
fn test_protect_interpolation_expressions_unicode_outside_sentinel_preserved() {
    // Pre-fix: this panics with "byte index N is not a char boundary"
    // because the non-matching branch sliced `yaml[pos..pos + 1]` while
    // `pos` was inside the multi-byte scalar for the emoji.
    let yaml = "key: \"\u{1F5A5}\u{FE0F} prefix {{var}} suffix\"";
    let (protected, replacements) = protect_interpolation_expressions(yaml);

    // Sentinel still detected and replaced.
    assert_eq!(replacements.len(), 1);
    assert_eq!(replacements[0].1, "{{var}}");
    assert!(protected.contains("__DM_EXPR_0__"));

    // Non-ASCII characters outside the sentinel are preserved byte-identically.
    assert!(protected.contains('\u{1F5A5}'));
    assert!(protected.contains('\u{FE0F}'));
    assert!(protected.contains("prefix"));
    assert!(protected.contains("suffix"));

    // No lossy `'?'` substitution survives in the output.
    assert!(!protected.contains('?'));
}
```

- [ ] **Step 2: Run the test and confirm it panics (or fails)**

```bash
cargo test -p darkmatter --lib \
  markdown::frontmatter::tests::test_protect_interpolation_expressions_unicode_outside_sentinel_preserved \
  -- --nocapture
```

Expected: FAIL with a panic similar to `byte index 1 is not a char boundary; it is inside '🖥' ...` originating at `frontmatter.rs:371`.

### Task 2.2: Fix `protect_interpolation_expressions`

- [ ] **Step 3: Replace the byte-slice copy with a UTF-8-safe scalar advance**

In `darkmatter/lib/src/markdown/frontmatter.rs`, replace exactly these two lines inside `protect_interpolation_expressions` (currently at line 371-372):

```rust
            result.push(yaml[pos..pos + 1].chars().next().unwrap_or('?'));
            pos += 1;
```

with:

```rust
        } else if let Some(ch) = yaml[pos..].chars().next() {
            // Non-matching byte: advance by a full UTF-8 scalar so `pos`
            // always lands on a char boundary. Slicing `yaml[pos..pos + 1]`
            // here would panic when `pos` falls inside a multi-byte scalar.
            result.push(ch);
            pos += ch.len_utf8();
        } else {
            break;
        }
```

(Same shape as Phase 1 Step 3: replace the original `} else {` opening with the `} else if let Some(ch) = ...` form plus a defensive `else { break; }`.)

- [ ] **Step 4: Update the rustdoc on `protect_interpolation_expressions` with the UTF-8-boundary note**

Add one line to the existing rustdoc block above `fn protect_interpolation_expressions`. Insert it as the final paragraph of the doc comment, so the resulting doc reads:

```rust
/// Replaces `{{ }}` expression bodies with safe placeholders.
///
/// Returns the modified string and a map of placeholder → original expression.
///
/// Non-matched content advances by full UTF-8 scalar boundaries (via
/// `yaml[pos..].chars().next()` + `char.len_utf8()`) so `pos` never lands
/// inside a multi-byte scalar.
fn protect_interpolation_expressions(yaml: &str) -> (String, Vec<(String, String)>) {
```

- [ ] **Step 5: Run the new test and verify it passes**

```bash
cargo test -p darkmatter --lib \
  markdown::frontmatter::tests::test_protect_interpolation_expressions_unicode_outside_sentinel_preserved \
  -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Run the full frontmatter test module to verify no regressions**

```bash
cargo test -p darkmatter --lib markdown::frontmatter::tests:: -- --nocapture
```

Expected: every test in the module passes (both Phase 1 and Phase 2 unicode tests pass, all pre-existing tests still pass).

- [ ] **Step 7: Commit Phase 2**

```bash
git add darkmatter/lib/src/markdown/frontmatter.rs
git commit -m "fix(darkmatter): advance protect_interpolation_expressions by UTF-8 scalars

Apply the same UTF-8-safe scalar advance to the non-matching branch of
protect_interpolation_expressions that Phase 1 applied to
protect_shell_expressions. Multi-byte scalars outside a {{...}} span
no longer panic the helper, and the lossy '?' fallback is removed.
Sentinel detection, nested {{ / }} depth tracking, and placeholder
generation are unchanged."
```

---

## Phase 3: End-to-End Reproduction Test, Audit, and Final Verification

**Files:**
- Test: `darkmatter/lib/src/markdown/frontmatter.rs` (existing `#[cfg(test)] mod tests` block)
- (Read-only audit) `darkmatter/lib/src/markdown/**/*.rs`

### Task 3.1: Add the end-to-end reproduction test

- [ ] **Step 1: Add the spec's reproduction case as an end-to-end test**

Append this test at the end of `mod tests` in `darkmatter/lib/src/markdown/frontmatter.rs`:

```rust
#[test]
fn test_parse_frontmatter_with_emoji_value_and_unquoted_interpolation() {
    // Reproduction from the spec: an unquoted `{{...}}` value forces
    // parse_yaml_with_fallbacks() into the byte-scanning fallback, and
    // the multi-byte emoji in a quoted value used to panic the scanner.
    // Both must round-trip verbatim through the fallback path.
    let content = concat!(
        "---\n",
        "area: {{ctx.current_package_area}}\n",
        "success_message: \"\u{1F5A5}\u{FE0F} Build succeeded\"\n",
        "---\n",
        "# Body\n",
    );

    let (fm, remaining) = parse_frontmatter(content).unwrap();
    let area: Option<String> = fm.get("area").unwrap();
    let message: Option<String> = fm.get("success_message").unwrap();

    assert_eq!(area, Some("{{ctx.current_package_area}}".to_string()));
    assert_eq!(message, Some("\u{1F5A5}\u{FE0F} Build succeeded".to_string()));
    assert!(remaining.starts_with("# Body"));
}
```

- [ ] **Step 2: Run the end-to-end test and verify it passes**

```bash
cargo test -p darkmatter --lib \
  markdown::frontmatter::tests::test_parse_frontmatter_with_emoji_value_and_unquoted_interpolation \
  -- --nocapture
```

Expected: PASS. The unquoted `{{...}}` line forces the YAML parser to fail, the fallback runs through both protect helpers (now UTF-8 safe), the resulting placeholder-substituted YAML parses cleanly, and the restoration step puts the original `{{ctx.current_package_area}}` back into the `area` value while leaving the emoji in `success_message` untouched.

If the test fails because the YAML parser accepts the unquoted `{{...}}` form (i.e., the fallback path is never entered), edit the input so the unquoted token is unambiguously YAML-invalid (for example, change the line to `area: {{ctx.current_package_area || "default"}}`, which contains a `||` and quoted string that the raw parser will reject) and re-run.

### Task 3.2: Audit the rest of `darkmatter/lib/src/markdown/*` for the same anti-pattern

- [ ] **Step 3: Confirm the negative audit finding with a fresh grep**

Run from the repo root:

```bash
rg -n 'push\([^)]*\[[^]]*\.\.[^]]*\+ 1\]' darkmatter/lib/src/markdown
```

Expected output: zero matches after Phases 1 and 2 land. (Pre-fix this command returned exactly the two lines that the spec calls out: `frontmatter.rs:328` and `frontmatter.rs:371`. Post-fix both are gone.)

Also confirm sibling scanners that walk `as_bytes()` are not affected:

```bash
rg -n '\.as_bytes\(\)' darkmatter/lib/src/markdown
```

Expected: matches in files like `compose/interpolation/lexer.rs`, `compose/transclusion/parser.rs`, `compose/page_blocks/parser.rs`, `compose/toc_linking/parser.rs`, `compose/cache/hashing.rs`, `compose/cache/runtime.rs`, `compose/cache/store.rs`, `compose/context/capture.rs`, `cleanup.rs`, `output/terminal.rs`, `reference/types.rs`. Spot-check that none of them slice the source string by an arbitrary `pos` to copy a single byte into a `String`; their `as_bytes()` use is limited to ASCII byte comparisons (`bytes[pos] == b'{'`, etc.) where non-matching positions only advance `pos` and never produce a slice. No remediation work is required.

- [ ] **Step 4: Record the negative audit finding as a one-line in-source note**

Add this single comment line immediately above the rustdoc on `parse_yaml_with_fallbacks` (currently at line 219) in `darkmatter/lib/src/markdown/frontmatter.rs`. Insert it as the very first line of the function block, right before `/// Attempts to parse YAML with progressive fallback strategies.`:

```rust
// UTF-8-boundary audit (2026-04-24): the byte-indexed scanners in this file
// were the only `markdown/*` sites that copied `yaml[pos..pos + 1]` into a
// String. Sibling scanners under `markdown/` walk `as_bytes()` only for
// ASCII sentinel checks and never slice the source by an arbitrary `pos`.
```

(This is the spec-mandated record of the negative finding. It lives in source rather than in a CHANGELOG or fix-log per the spec's "Out of scope" list.)

### Task 3.3: Final verification across the darkmatter package

- [ ] **Step 5: Run the entire darkmatter test suite**

```bash
cargo test -p darkmatter
```

Expected: all tests in `darkmatter` pass. This includes the `frontmatter` module tests added in Phases 1, 2, and 3, plus every other pre-existing test in the package, including doc tests for the `Frontmatter` type.

- [ ] **Step 6: Run clippy on the darkmatter library to confirm no new warnings**

```bash
cargo clippy -p darkmatter --lib --all-targets -- -D warnings
```

Expected: clean exit (no warnings, no errors). If clippy flags `manual_let_else` or similar suggestions on the new `if let Some(ch) = ... else { break; }` form, accept the suggestion only if it preserves panic-free behavior; otherwise keep the explicit form.

- [ ] **Step 7: Commit Phase 3**

```bash
git add darkmatter/lib/src/markdown/frontmatter.rs
git commit -m "test(darkmatter): cover emoji + unquoted interpolation reproduction case

Add the spec's exact reproduction input — an unquoted area: {{...}}
line that forces parse_yaml_with_fallbacks into the byte-scanning
fallback alongside a quoted value beginning with a multi-byte emoji
— as an end-to-end test against parse_frontmatter. Record the negative
audit finding (no other byte-indexed-copy sites under
darkmatter/lib/src/markdown/) as an in-source note above
parse_yaml_with_fallbacks."
```

---

## Acceptance Criteria Checklist

Run before declaring the fix complete:

- [ ] `cargo test -p darkmatter --lib markdown::frontmatter::tests::` — all tests pass, including the three new ones.
- [ ] `cargo test -p darkmatter` — full package tests pass.
- [ ] `cargo clippy -p darkmatter --lib --all-targets -- -D warnings` — clean.
- [ ] `rg -n 'unwrap_or\(.\?.\)' darkmatter/lib/src/markdown/frontmatter.rs` — zero matches (the lossy `'?'` fallback is gone from both helpers).
- [ ] `rg -n 'push\([^)]*\[[^]]*\.\.[^]]*\+ 1\]' darkmatter/lib/src/markdown` — zero matches (the byte-slice copy anti-pattern is gone).
- [ ] Both `protect_shell_expressions` and `protect_interpolation_expressions` carry a one-line rustdoc note about UTF-8 scalar advance.
- [ ] The end-to-end reproduction test parses successfully and preserves both the emoji and the `{{...}}` template expression byte-identically.
- [ ] Three commits on the branch, one per phase.

## Out of Scope (per spec)

- No CHANGELOG or fix-log entry beyond this plan and the spec.
- No edits outside `darkmatter/lib/src/markdown/`.
- No changes to the YAML parser or to public frontmatter API surface.
- No changes to backtick substitution or process substitution handling (`<(...)`, `>(...)`).
