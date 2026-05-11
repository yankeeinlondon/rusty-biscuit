---
ready: false
agent: open_code
model: ""
---

# Review: Good Errors (2026-05-08) — Review 2

This is the second review of the "Good Errors" feature. The first review identified Phase 2 as incomplete. After examining the current codebase, the situation is substantially worse: **the codebase does not compile** and the implementation is in a broken, intermediate state.

## Executive Summary

**This feature is NOT ready for production.**

The implementation has regressed into a non-compiling state with 85+ compilation errors. While some Phase 1 foundations were laid (`StatusBlock::body` accepting `Vec<Prose>`, `SourceContext` type in `biscuit-terminal`), the mechanical migration of call sites was abandoned mid-stream, leaving the codebase in a broken state where:

1. The code does not compile
2. Test files still use old error variant signatures
3. Multiple modules reference types that no longer exist in their previous locations
4. Documentation was never created
5. Snapshot tests were never generated

## Findings

### Critical Severity

#### 1. Code Does Not Compile (85+ Errors)

The workspace has **85+ compilation errors** in `darkmatter` alone. This is an absolute blocker.

**Key error categories:**

- **`MergeStrategy` defined twice** in `darkmatter/lib/src/markdown/frontmatter.rs` (lines 12 and 180), causing conflicting trait impls for `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`
- **`SourceContext` removed from translocation module** but ~15 files still import `crate::markdown::compose::translocation::SourceContext` (e.g., `reference/graph.rs:18`, `compose/parse_utils.rs:26`, `shell_expansion/parser.rs`)
- **Missing `PathBuf` import** in `darkmatter/lib/src/markdown/compose/conditions.rs:238`
- **Function signature changes without call-site updates**: `parse_frontmatter_refs` now takes `ctx: SourceContext` but `reference/mod.rs:119` calls it with 1 argument; `frontmatter_parse_block` takes `SourceContext` by value but `types.rs:115` passes `&SourceContext`
- **`ShellExpansionError` variants** missing required `ctx` field in constructors across the shell expansion parser
- **Private struct import** — `reference/graph.rs` tries to import `SourceContext` from translocation but it's no longer public there

**Impact:** The package cannot be built, tested, or used. This is a hard blocker.

#### 2. Test Files Use Obsolete Error Variant Signatures

Every test file under `darkmatter/lib/tests/error_snapshots/` constructs error variants using **old field names and signatures** that no longer match the actual enum definitions.

Examples:

- `translocation.rs:12-17` constructs `ParseDirective { line, message, caret_col }` — missing required `ctx: SourceContext`
- `translocation.rs:33-39` constructs `InvalidReference { reference, line, source_file, directive_kind }` — the actual variant uses `ctx: SourceContext` and has no `source_file` field
- `page_block.rs:13-17` constructs `ParseDirective { line, message }` — missing `ctx`
- `reference.rs`, `link.rs`, `image_ref.rs`, `stylesheet.rs` — all have similar mismatches

**Impact:** Even if the library compiled, the test suite would fail with constructor errors.

### High Severity

#### 3. Missing Documentation

Per the spec §3.7 and §5:

- `darkmatter/docs/errors/README.md` — **does not exist**
- `.claude/skills/darkmatter/errors.md` — **does not exist**
- `darkmatter/README.md` — no link to error documentation
- `biscuit-terminal/README.md` — no mention of `StatusBlock::body` signature change or `SourceContext`

**Impact:** Future contributors have no authoritative reference for error rendering conventions.

#### 4. Snapshot Tests Never Generated

Per the spec §3.6, every `BlockError` variant should have an `insta` snapshot test with a checked-in `.snap` file.

- **Zero snapshot files exist** under `darkmatter/lib/tests/error_snapshots/snapshots/`
- The `insta::assert_snapshot!` calls in the test files will fail on first run because there is no baseline to compare against
- The `render()` helper in `helpers.rs` **strips ANSI escape codes**, making it impossible to verify OSC 8 hyperlinks, colors, or other styled output in snapshots

**Impact:** No visual regression protection. The spec's primary gate against bare-markup leakage (snapshot comparison) is absent.

#### 5. SourceContext Name Collision Partially Resolved (But Breaks Imports)

The first review noted a name collision: `translocation/types.rs` defined its own `SourceContext { file, url }`. This local struct was **removed** (good), but the replacement wasn't handled correctly:

- Many files were importing `SourceContext` from `translocation` module rather than `biscuit_terminal::errors::SourceContext`
- When the local struct was deleted, all those imports broke
- The correct fix would have been to update imports to `biscuit_terminal::errors::SourceContext`, but this wasn't done

**Impact:** Compilation failures across the entire compose pipeline.

### Medium Severity

#### 6. Test Rigor Gap — No Level 2 or Level 3 Tests

Per the spec's test strategy (§4.6) and the review instructions:

- **All tests are Level 1 only** (in-process, optimistic terminal, ANSI stripped)
- **No Level 2 tests** running in a real terminal emulator (WezTerm, Kitty, tmux) with escape-code-preserving capture
- **No Level 3 tests** using OS keyboard injection

User-observable behaviors that require higher verification levels:

| Requirement | Spec Section | Strongest Test Level | Gap |
|-------------|--------------|---------------------|-----|
| OSC 8 clickable hyperlinks in headers | §3.1 #2 | Level 1 (ANSI stripped) | **High** — hyperlinks are invisible after `strip_ansi` |
| `>` gutter marker on offending lines | §3.1 #4 | Level 1 | Medium — visible in plain text but width/rendering not verified |
| Dim foreground for fenced code blocks | §4.2 | Level 1 (ANSI stripped) | **High** — color info is stripped |
| `<inverse>::end-block</inverse>` hint | §3.5 | Level 1 (ANSI stripped) | **High** — style info is stripped |

The `render()` helper in `helpers.rs:12-14` calls `strip_escape_codes()` before returning, which **removes exactly the bytes needed to verify styling**. At minimum, a parallel test path should preserve ANSI for some assertions.

#### 7. Prose Fenced Code Block Grammar — Unverified

The spec §4.2 requires Prose to support fenced code blocks (` ```LANG
... ``` `). While the `SourceContext::excerpt_prose()` and `frontmatter_prose()` methods generate strings in this format, there is **no evidence that Prose actually renders fenced code blocks correctly**:

- No unit tests for fenced block rendering in `biscuit-terminal`
- No integration tests verifying that ` ``` ` fences produce visually correct output
- The spec mentions "2-space indent and dim foreground color" but this rendering logic is not visible in the current Prose implementation

**Impact:** The core new grammar may not work; errors with excerpts may render poorly.

#### 8. DeferredSetError Still Uses String Body (Not Vec<Prose>)

In `translocation/types.rs:143-176`, `DeferredSetError`'s `BlockError` impl calls:

```rust
.body(format!("<dim>Line:</dim> {line}\n<dim>Value:</dim> <cyan>{raw}</cyan>\n<dim>Reason:</dim> {reason}"))
```

This passes a single `String` to `.body()`. The `StatusBlock::body` signature now accepts `impl IntoProseVec`. While `String` may convert through `Prose`, the newline-separated multi-line string will be treated as a single Prose item rather than three separate Prose paragraphs. The rendering may be acceptable but it's not using the new API idiomatically.

Similarly, `StylesheetError` in `render/stylesheet.rs:65-161` and several other error types still use `.body(format!(...))` with embedded newlines rather than constructing `Vec<Prose>`.

### Low Severity

#### 9. `frontmatter_parse_block` Uses Manual Snippet Rendering

As noted in Review 1, `darkmatter/lib/src/markdown/errors/blocks.rs:61-79` implements `frontmatter_parse_block` which manually constructs excerpts. This was updated to use `ctx.excerpt_prose()` (good), but the `ctx` parameter type changed from `&SourceContext` to `SourceContext` by value, causing a compilation error at the call site in `types.rs:115`.

#### 10. ConditionError Missing `span` Field in Test

`darkmatter/lib/tests/error_snapshots/condition.rs` constructs `ConditionError::Parse` but the actual variant requires a `span: Range<usize>` field. The test file likely hasn't been updated.

## Ergonomics & Performance

- **Arc<str> in SourceContext**: Correctly implemented; cheap clones
- **StatusBlock::body_line()**: Present and useful for single-line bodies
- **IntoProseVec trait**: Provides ergonomic conversions, though the trait name differs from the spec's `Into<Vec<Prose>>` suggestion

## Recommendations

### Immediate (Blocks Compilation)

1. **Fix `MergeStrategy` duplicate definition** in `frontmatter.rs` — remove the second definition at line 180
2. **Fix all `SourceContext` imports** — replace `crate::markdown::compose::translocation::SourceContext` with `biscuit_terminal::errors::SourceContext` in ~15 files
3. **Add missing `use std::path::PathBuf;`** in `conditions.rs`
4. **Update all function call sites** to pass `SourceContext` where now required:
   - `parse_frontmatter_refs` in `reference/mod.rs`
   - `frontmatter_parse_block` in `types.rs` (pass by value or change signature to `&SourceContext`)
   - `parse_directives` in `shell_expansion/parser.rs` and all its callers
   - All `TransclusionError` and `ShellExpansionError` constructors
5. **Update all test files** to match new error variant signatures (add `ctx` fields, remove obsolete fields like `source_file`)

### Short-Term (Required for "Ready")

6. **Generate snapshot files** — run `cargo insta test --accept` to create baseline `.snap` files for all variants
7. **Create `darkmatter/docs/errors/README.md`** documenting the body-is-`Vec<Prose>` contract, `SourceContext` requirement, standard structure, and snapshot test requirement
8. **Add ANSI-preserving test path** — create a second `render_with_ansi()` helper that does NOT strip escape codes, and add assertions that verify OSC 8 hyperlinks and color codes are present
9. **Add at least one Level 2 test** — spawn the binary in tmux/WezTerm and capture pane text to verify real-terminal rendering of hyperlinks and gutters
10. **Update `.claude/skills/darkmatter/errors.md`** with the error rendering contract

### Medium-Term (Polish)

11. **Audit all `.body(format!(...))` call sites** — convert multi-line format strings to `vec![Prose::new(...), Prose::new(...)]` for proper paragraph separation
12. **Verify Prose fenced code block rendering** — add unit tests in `biscuit-terminal` that verify ` ``` ` fences produce correct visual output with indentation and dimming
13. **Consider Level 3 tests** for key user-interaction requirements (gated behind `RUN_LEVEL3=1`)

## Verification Level Summary

| Requirement | Spec Ref | Test Level Present | Minimum Required | Status |
|-------------|----------|-------------------|------------------|--------|
| No bare markup | §3.1 #1 | N/A (code compiles = enforced) | Level 1 | **Blocked** — code doesn't compile |
| Linked file path | §3.1 #2 | Level 1 (ANSI stripped) | Level 2 | **Gap** — hyperlinks invisible after strip |
| Frontmatter snapshot | §3.1 #3 | Level 1 | Level 1 | OK in principle, blocked by compilation |
| Source excerpt | §3.1 #4 | Level 1 | Level 1 | OK in principle, blocked by compilation |
| Hint with styling | §3.1 #5 | Level 1 (ANSI stripped) | Level 2 | **Gap** — `<inverse>` styling not verified |
| Snapshot regression gate | §3.6 | None | Level 1 | **Gap** — no `.snap` files exist |

## Conclusion

This implementation is **not ready for production**. The feature has regressed from a partial Phase 1 completion into a **non-compiling state** due to incomplete mechanical migration of call sites. Before any further review can be meaningful:

1. The codebase must compile
2. All tests must pass
3. Snapshot baselines must be established
4. Documentation must be authored

Only then can a subsequent review evaluate whether the rendered output meets the visual quality bar defined in the spec.
