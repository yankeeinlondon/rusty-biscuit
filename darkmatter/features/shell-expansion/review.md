# Shell Expansion Feature Review

**Reviewed:** 2026-03-15
**Scope:** Full implementation review against spec and tech design

## Overall Assessment

The implementation faithfully covers the functional contract from the spec and tech design. All 7 planned module files exist, the pipeline integration is correct, and the CLI approval handler is properly separated from library logic. **85 tests** cover the feature across unit and integration layers.

The feature is production-ready with a handful of suggestions for improvement detailed below.

---

## Findings

### 1. Policy File Naming Deviation

**Severity:** Low (intentional improvement, but diverges from spec)

The spec and tech design specify `.shell-whitelist` and `.shell-blacklist`. The implementation uses `.darkmatter-shell-whitelist` and `.darkmatter-shell-blacklist` (with a `darkmatter-` prefix).

This is likely a deliberate namespacing choice to avoid collisions, but it diverges from both documents. If intentional, update the spec and tech design to match. If unintentional, rename to match the spec.

**Files:** `store.rs:64-65`, `mod.rs:11` (doc comment)

### 2. Missing Non-Interactive Error Guidance

**Severity:** Medium

The spec (section "CLI behavior") and tech design both require that when prompting is not possible, the error should include:

1. The command
2. The whitelist file path
3. The exact `exact ...` and `prefix ...` entries the user can add manually

The current `ApprovalRequired` error includes the command and paths, but the CLI does not format the additional guidance about manual entries when displaying this error in non-interactive mode. Users piping through `md compose -` will get a generic error without actionable instructions.

**Suggestion:** In the CLI error handler for `ApprovalRequired`, format a message like:

```
To approve this command, add one of these lines to <whitelist_path>:
  exact <normalized_command>
  prefix <executable>
```

### 3. Parser: Windows Line Ending Handling

**Severity:** Low (edge case)

In `parser.rs`, the newline offset calculation hardcodes `+1` for the newline byte. With `\r\n` (Windows-style) line endings, the span will be 1 byte short, missing the `\r`. This means replacements on Windows-created files could leave stray `\r` characters.

**Suggestion:** Detect actual newline length (`\r\n` = 2, `\n` = 1) instead of assuming 1.

### 4. Missing Tests for `>>` and `||` Metacharacter Rejection

**Severity:** Low (functionally correct, testing gap)

The tokenizer correctly rejects `>>` and `||` because the single-character checks (`>` and `|`) catch the first character. However, there are no explicit tests for these multi-character operators. Adding tests for `>>` and `||` would improve documentation of intended behavior.

### 5. Missing Doc Comments on Some Public Methods

**Severity:** Low

The following public methods lack `///` doc comments:

- `ShellRuleSet::matches_exact()`
- `ShellRuleSet::matches_prefix()`

These are simple utility methods, but consistent documentation on all public items is a project convention.

### 6. Missing `append_blacklist_prefix()` Function

**Severity:** Low (API asymmetry)

The store module provides `append_whitelist_exact()`, `append_whitelist_prefix()`, and `append_blacklist_exact()`, but not `append_blacklist_prefix()`. The current `BlacklistPersist` approval decision only uses exact matching, so this isn't functionally needed today, but it creates an asymmetric API surface.

**Suggestion:** Either add `append_blacklist_prefix()` for completeness or document why prefix blacklisting is intentionally unsupported.

### 7. Missing Integration Tests

**Severity:** Medium (test coverage gap)

Several integration scenarios from the plan are not tested:

1. **Interpolation feeds into shell expansion** — No test verifying `::shell echo {{ var }}` with frontmatter produces correct output
2. **Allow-once persists across recursive transclusion** — No test with child documents sharing allow-once state
3. **`fail_fast = false` still produces hard failures** — No explicit test that shell errors are hard regardless of fail_fast
4. **Interactive approval handler** — No tests for `CliShellApprovalHandler`'s prompt flow, input parsing, or re-prompting on invalid input
5. **Stderr-only command output** — Executor test for stderr capture is commented out

### 8. Executor Thread Join Safety

**Severity:** Low

In `executor.rs`, stdout/stderr drain threads use `unwrap_or_default()` on join results. If a thread panics during output draining, the error is silently swallowed and treated as empty output. This is defensible but could mask real issues.

### 9. Documentation Updates Not Completed

**Severity:** Low

The plan (Task 3.4) calls for:

- Updating `docs/dependencies.md` with the `which` crate entry
- Updating `darkmatter/docs/darkmatter-pipeline.md` with the new stage ordering

Neither update appears to have been made. The `which` crate is added to `Cargo.toml` but not documented.

### 10. Store: Silent Malformed Line Handling

**Severity:** Low

In `store.rs`, lines in policy files that don't match `exact <cmd>` or `prefix <cmd>` are silently skipped. A typo like `exat echo hello` would be invisible to the user. Consider emitting a `tracing::warn!` for malformed lines.

---

## Separation of Concerns

**Library/CLI boundary is clean.** The library:

- Owns all business logic (parsing, validation, blacklist, execution, policy management)
- Never prompts directly
- Exposes the `ShellApprovalHandler` trait for caller-owned prompting

The CLI:

- Implements `CliShellApprovalHandler` with terminal prompting
- Detects interactive conditions (file input + terminal stdin/stderr)
- Attaches the handler conditionally in `run_compose()`

This is well-architected and matches the design intent.

---

## Test Coverage Summary

| Component | Tests | Quality |
|-----------|-------|---------|
| Tokenizer | 20 | Excellent |
| Parser | 13 | Excellent |
| Policy/Blacklist | 25 | Excellent |
| Store | 9 | Good |
| Executor | 8 | Good (stderr gap) |
| Pipeline Integration | 6 | Good |
| CLI Integration | 4 | Partial |
| **Total** | **85** | |

---

## Prioritized Suggestions

### Should Fix (before merge)

1. **Add non-interactive error guidance** (#2) — Spec explicitly requires manual whitelist instructions in non-interactive errors
2. **Add interpolation-to-shell integration test** (#7.1) — Key pipeline ordering guarantee needs verification

### Should Consider

1. **Add `>>` and `||` tokenizer tests** (#4) — Low effort, improves confidence
2. **Add doc comments to `ShellRuleSet` methods** (#5) — Project convention
3. **Add `tracing::warn!` for malformed policy lines** (#10) — Debugging aid
4. **Update pipeline docs** (#9) — Stage ordering documentation

### Nice to Have

1. **Fix Windows line ending handling** (#3) — Edge case, unlikely on macOS
2. **Add `append_blacklist_prefix()`** (#6) — API symmetry
3. **Add allow-once recursive transclusion test** (#7.2)
4. **Reconcile policy file naming** (#1) — Ensure spec and implementation agree
