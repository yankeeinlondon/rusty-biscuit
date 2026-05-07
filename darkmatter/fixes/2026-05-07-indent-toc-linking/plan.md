---
phases: 4
created: 2026-05-07
start_phase: 1
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/toc_linking/types.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/parser.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/render.rs
  - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
  - darkmatter
---

# Execution Plan: `::toc-linking` Indentation Preservation

## Context

The `::toc-linking` directive emits markdown link lists without respecting the indentation context of the directive line. When nested inside list items or other indented blocks, the output breaks out to column 1, corrupting the document structure.

## Root Cause

1. **`TocLinkingDirective` does not capture leading whitespace** — only `span` (full line byte range) and `line` are stored.
2. **`render_toc_links` emits unindented output** — each line is `- [Title](path#slug)` with no prefix.
3. **The compose replacement logic does a simple byte-range replacement** — no indentation adjustment is performed.

## Execution Phases

### Phase 1 — Write Failing Tests

**Goal:** Establish the bug with concrete test fixtures before touching implementation code.

**Steps:**

1.1. **Add renderer-level unit tests** in `darkmatter/lib/src/markdown/compose/toc_linking/render.rs`
   - Test: `indented_output_two_levels` — headings rendered with a 4-space indent prefix on every line
   - Test: `indented_output_empty_text` — empty_text also indented when provided
   - Test: `no_indent_at_root` — empty indent string produces unchanged behavior
   - *Parallelizable:* Yes (all three tests in one file)

1.2. **Add parser-level unit tests** in `darkmatter/lib/src/markdown/compose/toc_linking/parser.rs`
   - Test: `captures_indent_from_leading_whitespace` — directive at 2 spaces captures `"  "`
   - Test: `captures_indent_from_tabs` — directive with tab prefix captures `"\t"`
   - Test: `inferred_indent_from_previous_line` — directive at column 1 after indented line captures previous line's indent
   - *Parallelizable:* Yes

1.3. **Add integration tests** in `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs`
   - Test fixture: `::toc-linking` nested two levels deep inside a list (4-space continuation)
     - Assert every generated bullet has 4 leading spaces
   - Test fixture: `::toc-linking` at column 1 inside a list item continuation
     - Assert output is indented to match the list item (2+ spaces)
   - Test fixture: `::toc-linking` at document root (no container indentation)
     - Assert output starts at column 1, unchanged from current behavior
   - *Parallelizable:* Yes (all three tests in one file)

**Validation Checkpoint:**
- [x] All new tests compile but fail when run, confirming the bug
- [x] `cargo test -p darkmatter toc_linking` shows 5+ new failures (9 new tests added, 5 failing as expected)

---

### Phase 2 — Core Fix: Types, Parser, and Renderer

**Goal:** Capture indentation at parse time and apply it at render time.

**Steps:**

2.1. **Extend `TocLinkingDirective` struct** in `darkmatter/lib/src/markdown/compose/toc_linking/types.rs`
   - Add field: `pub indent: String` — the leading whitespace of the directive line
   - Add field: `pub inferred_indent: Option<String>` — inferred container indentation when directive is at column 1
   - Update `derive(Debug, Clone)` is already present; no other derives needed

2.2. **Capture indentation in parser** in `darkmatter/lib/src/markdown/compose/toc_linking/parser.rs`
   - In `parse_toc_linking_directives`, when a directive line is found:
     - Extract leading whitespace: `line[..line.len() - line.trim_start().len()]`
     - If leading whitespace is empty, scan backward to the previous non-empty line and capture its leading whitespace as `inferred_indent`
   - Pass both values to `parse_directive_line`
   - Update `parse_directive_line` signature and `TocLinkingDirective` construction
   - *Depends on:* 2.1

2.3. **Update renderer to prefix output** in `darkmatter/lib/src/markdown/compose/toc_linking/render.rs`
   - Change `render_toc_links` signature: add `indent: &str` parameter
   - Compute effective indent: `if indent.is_empty() { inferred_indent.unwrap_or("") } else { indent }`
   - For each generated link line, prefix with effective indent
   - For `empty_text`, also prefix with effective indent (if multi-line, indent each line)
   - Update all call sites and unit tests in this file
   - *Depends on:* 2.1, 2.2

2.4. **Update orchestration function** in `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs`
   - Modify `render_resolved_directive` signature to accept `indent: &str` and `inferred_indent: Option<&str>`
   - Pass both through to `render_toc_links`
   - Update `process_toc_linking` (test-only) to pass indent from directive to renderer
   - Update all unit tests in this file
   - *Depends on:* 2.3

**Validation Checkpoint:**
- [ ] All unit tests in `toc_linking/` pass: `cargo test -p darkmatter toc_linking`
- [ ] No compiler warnings in modified files

---

### Phase 3 — Integration: Compose Pipeline

**Goal:** Wire the indentation through the full compose pipeline so real documents are fixed.

**Steps:**

3.1. **Update compose layer replacement** in `darkmatter/lib/src/markdown/compose/mod.rs`
   - In `resolve_prepared_transclusion` at the `PreparedTransclusion::Toc` arm (lines 1642-1722):
     - Extract `indent` and `inferred_indent` from the `directive` field
     - Pass them to `toc_linking::render_resolved_directive`
   - The `span` replacement logic (lines 982-1011) does not need changes — the rendered content already includes the correct indentation
   - *Depends on:* 2.4

3.2. **Run integration tests**
   - `cargo test -p darkmatter --test reference_integration`
   - `cargo test -p darkmatter --test cli` (if applicable)
   - Verify no regressions in existing toc-linking tests
   - *Depends on:* 3.1

3.3. **Verify `::file` and `::code` directives** (out-of-scope check per spec requirement 5)
   - Read `render_markdown_transclusion` and `render_code_transclusion` in `compose/mod.rs`
   - Confirm they also do not preserve indentation (expected: they share the same bug)
   - If the fix is trivially co-located (same code path), apply it; otherwise, document the finding in a code comment or issue
   - *Parallelizable with:* 3.2

**Validation Checkpoint:**
- [ ] All `darkmatter` tests pass: `cargo test -p darkmatter`
- [ ] Manual verification: create a test markdown file with indented `::toc-linking` and inspect output

---

### Phase 4 — Validation & Documentation

**Goal:** Confirm correctness against acceptance criteria and document the change.

**Steps:**

4.1. **Acceptance criteria verification**
   - [ ] **AC-1:** A test fixture with `::toc-linking` nested two levels deep inside a list (4-space continuation) produces output where every generated bullet has 4 leading spaces
   - [ ] **AC-2:** A test fixture with `::toc-linking` at column 1 inside a list item continuation produces correctly indented output
   - [ ] **AC-3:** A test fixture with `::toc-linking` at the document root produces output starting at column 1, unchanged
   - [ ] **AC-4:** Round-trip the rendered output through a CommonMark parser — inner TOC entries are children of the outer list item, not siblings

4.2. **Edge case testing** (manual or scripted)
   - Directive with tab-based indentation
   - Directive inside a blockquote (`> ` prefix)
   - Multiple `::toc-linking` directives at different indentation levels in the same document
   - `empty_text` with multi-line content

4.3. **Code review checklist**
   - [ ] No `unwrap()` or `expect()` added without justification
   - [ ] Error paths preserve original behavior when indentation is irrelevant
   - [ ] No changes to link generation logic (anchors, text, ordering, `level=` filtering)
   - [ ] All modified functions have doc comments updated if signatures changed

4.4. **Update related documentation**
   - Add a note to `darkmatter/lib/src/markdown/compose/toc_linking/mod.rs` module docs explaining indentation behavior
   - Update `CHANGELOG.md` or equivalent if present

**Validation Checkpoint:**
- [ ] All acceptance criteria pass
- [ ] `cargo test -p darkmatter` is green
- [ ] `cargo clippy -p darkmatter` shows no new warnings
- [ ] Plan marked complete

## Risk & Contingency

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Container indentation inference is unreliable for complex nested structures | Medium | Limit inference to simple heuristic (previous non-empty line). Document limitation. Acceptance criteria AC-2 may need a simplified test fixture. |
| Changing `TocLinkingDirective` struct breaks cache serialization | Low | `indent` fields are `String`/`Option<String>` — serde-compatible. Verify cache keys are based on `directive.options` only, not the struct itself. |
| `::file`/`::code` share the bug and users expect consistency | Low | Per spec, out of scope unless trivial. Document finding in Phase 3.3. |
| Performance regression from backward scan in parser | Very Low | Scan is bounded to a few lines before the directive. Negligible overhead. |

## Parallelizable Work Summary

Within each phase, all steps are independent unless marked with "Depends on." The critical path is:

```
2.1 → 2.2 → 2.3 → 2.4 → 3.1 → 3.2 → 4.1
```

Phase 1 tests can be written in parallel with each other but must land before Phase 2 implementation begins.
