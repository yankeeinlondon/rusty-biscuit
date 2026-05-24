---
status: proposed
date: 2026-05-09
supersedes: review-2.md (recovery actions)
spec: spec.md
branch: darkmatter
---

# Recovery Plan: Good Errors

This plan recovers the `darkmatter` "Good Errors" feature from its current non-compiling state and lands the spec's Phase 1 quality bar without rewinding the work already invested in `biscuit-terminal`.

## Context

The implementation regressed during the Phase 2 sweep (commits `8bdb00db`, `93b05534`). The `biscuit-terminal` foundations from the spec — `StatusBlock::body: Vec<Prose>`, the `SourceContext` value type, fenced-code Prose grammar — were landed cleanly in earlier commits (`a2ce3195`, `aec8000b`) and are not the source of breakage. The breakage is in `darkmatter`: 86 compile errors caused by partially-applied edits during the multi-file sweep.

Review-2 captures the symptoms accurately but is wrong on one point: the `error_snapshots` test directory **already contains ~50 baseline `.snap` files** (see `darkmatter/lib/tests/error_snapshots/snapshots/`). The work is to make the test crate compile so those snapshots can be re-run and either confirmed or accepted with intentional diffs.

## Strategy

**Fix forward, in disciplined increments.** Each step ends with a green checkpoint (`cargo check -p darkmatter` or `cargo test -p darkmatter`) before the next begins. Do not introduce new functionality until the baseline is restored.

- **Do not revert.** The biscuit-terminal foundation work is correct and not the source of breakage. Reverting throws away ~5 commits of real work.
- **Do not press on with the spec as written.** The original spec bundles too much into one push. After baseline is green, address review-2's substantive findings as separate, individually-green commits.
- **No new features until green.** Snapshot reviews, ANSI-preserving helpers, docs, and skill updates all wait until the workspace compiles and tests run.

## Step-by-Step Plan

### Step 1 — Stop the bleeding (mechanical compile fix)

**Goal.** `cargo check -p darkmatter` reports zero errors.

**Estimated effort.** 2–3 hours focused.

**Sub-steps, in this order** (each followed by `cargo check -p darkmatter` to track progress):

1. **Delete duplicate `MergeStrategy`.**
   - File: `darkmatter/lib/src/markdown/frontmatter.rs`
   - Delete the second definition at `frontmatter.rs:178-192` (the canonical definition lives at line 12 and is referenced by `merge_with` at line 80+).
   - Move the `use biscuit_terminal::errors::SourceContext;` line at `frontmatter.rs:176` to the top-of-file imports.
   - Resolves: 1× E0428, 6× E0119.

2. **Fix `SourceContext` import paths.**
   - The type now lives at `biscuit_terminal::errors::SourceContext`. Replace any remaining references to `crate::markdown::compose::transclusion::SourceContext`.
   - Files (from compiler output):
     - `darkmatter/lib/src/markdown/reference/graph.rs:18` — unresolved import.
     - `darkmatter/lib/src/markdown/compose/parse_utils.rs:26` — type reference in fn signature.
     - `darkmatter/lib/src/markdown/compose/transclusion/parser.rs:16,63,74,115,196,241,333` — multiple in-scope references.
     - `darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs:29,140,257` — in-scope references.
   - For each file: add `use biscuit_terminal::errors::SourceContext;` near the existing `biscuit_terminal::errors::*` imports if any, otherwise top-of-file with the other `use` statements.
   - Resolves: 1× E0432, 10× E0425.

3. **Fix `SourceContext` private import.**
   - `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs:3` — the import statement points at a path where `SourceContext` is no longer `pub`. Replace with `use biscuit_terminal::errors::SourceContext;`.
   - Resolves: 1× E0603.

4. **Add missing `PathBuf` import.**
   - `darkmatter/lib/src/markdown/compose/conditions.rs:238` references `PathBuf` without importing it.
   - Add `use std::path::PathBuf;` at top of file (or with the other `std::path` imports if present).
   - Resolves: 1× E0433.

5. **Fix `frontmatter_parse_block` call site.**
   - `darkmatter/lib/src/markdown/types.rs:115` calls `blocks::frontmatter_parse_block(ctx, source)` passing `&SourceContext`, but the fn signature at `darkmatter/lib/src/markdown/errors/blocks.rs:61` takes `SourceContext` by value.
   - **Decision:** change the call site to `frontmatter_parse_block(ctx.clone(), source)`. `SourceContext` clones cheaply (`Arc<str>` content + `PathBuf` paths); changing the signature to `&SourceContext` would require touching the body which already calls `ctx.linked_path_prose()` etc. via methods that don't consume.
   - Resolves: 1× E0308.

6. **Fix `parse_frontmatter_refs` call site.**
   - `darkmatter/lib/src/markdown/reference/mod.rs:119` calls with 1 argument; signature now requires 2.
   - The caller has a `Markdown` self in scope. Construct a `SourceContext` from `self`'s known path/content (look at how the surrounding code already builds one for similar diagnostic flows) and pass it.
   - Resolves: 1× E0061 here, plus the E0061 cluster at the same callsite pattern in `discovery.rs`.

7. **Fix remaining E0061 signature drifts.**
   - All 10 are call sites where a `ctx: SourceContext` parameter was added to the callee but not threaded through the caller.
   - Locations from compiler output:
     - `darkmatter/lib/src/markdown/compose/shell_expansion/discovery.rs:181, 342, 348, 354, 387, 401`
     - `darkmatter/lib/src/markdown/compose/page_blocks/engine.rs:32, 75`
     - `darkmatter/lib/src/markdown/compose/conditions.rs:127`
     - `darkmatter/lib/src/markdown/compose/transclusion/conditions.rs:12`
     - `darkmatter/lib/src/markdown/compose/transclusion/parser.rs:14, 61`
     - `darkmatter/lib/src/markdown/compose/transclusion/resolver.rs:11`
   - Strategy per call site: trace upward to find a `SourceContext` already in scope (parsers and engines are constructed with one) and forward it. If genuinely no `ctx` is in scope, the caller's signature also needs to gain a `ctx` parameter — propagate up until a constructor with the original `(path, content)` is reached.
   - Resolves: 10× E0061.

8. **Fix `ShellExpansionError` constructors.**
   - 41× E0063: variants gained a required `ctx` field but constructors don't supply it.
   - Files:
     - `darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs:322`
     - `darkmatter/lib/src/markdown/compose/shell_expansion/executor.rs:179, 202, 269, 287, 301, 429, 440, 469, 484, 495, 587, 602, 616, 717`
     - `darkmatter/lib/src/markdown/compose/shell_expansion/tokenize.rs:75, 103, 123, 136, 153, 180, 188, 195, 203, …`
   - Each function in `executor.rs` and `tokenize.rs` must accept (or already has in scope) a `&SourceContext` from its caller. Add a `ctx: ctx.clone()` field initializer to every `ShellExpansionError { … }` literal.
   - For functions that don't currently take a `ctx`: thread it through their signatures from the caller. Most callers in `executor.rs`/`tokenize.rs` are reached from `shell_expansion::parser` which already (per step 2) has `ctx` in scope.
   - Resolves: 41× E0063.

9. **Sweep residual errors.**
   - The remaining ~3× E0599, 1× E0027 will mostly fall out as cascading damage repaired by the steps above. Address each individually by reading the compiler diagnostic.
   - Resolves: ~5 remaining errors.

**Step 1 exit criteria.**
- `cargo check -p darkmatter` exits 0.
- `cargo check -p darkmatter --tests` exits 0 (compile, not run, the test binaries).
- Commit: `fix(darkmatter): restore compilation after Good Errors Phase 2 sweep`.

### Step 2 — Test crate compiles

**Goal.** `cargo test -p darkmatter --no-run` succeeds.

**Estimated effort.** 1–2 hours.

The test files under `darkmatter/lib/tests/error_snapshots/` construct error variants with **old field names**. They must be updated to match the current variant signatures. Snapshots themselves are preserved; only the test code needs editing.

**Sub-steps:**

1. **Inventory failing test files.** Run `cargo check -p darkmatter --tests 2>&1 | grep "tests/error_snapshots"` to enumerate. Per review-2 the affected files are:
   - `condition.rs` — `ConditionError::Parse` needs a `span: Range<usize>` field.
   - `image_ref.rs`, `link.rs`, `reference.rs`, `stylesheet.rs` — variants need `ctx: SourceContext` (drop now-removed `source_file` if present).
   - `page_block.rs:13-17` — `ParseDirective { line, message }` → add `ctx`.
   - `transclusion.rs:12-17` — `ParseDirective { line, message, caret_col }` → add `ctx`.
   - `transclusion.rs:33-39` — `InvalidReference { reference, line, source_file, directive_kind }` → replace `source_file` with `ctx`.
   - Other files (`deferred_set.rs`, `editor.rs`, `file_tree.rs`, `markdown_error.rs`, `mermaid_theme.rs`, `normalization.rs`, `shell_expansion.rs`, `toc_linking.rs`) — verify and update as compiler reports.

2. **Construct test `SourceContext` values via a shared helper.**
   - Add (or reuse) a helper in `darkmatter/lib/tests/error_snapshots/helpers.rs`:
     ```rust
     pub fn test_ctx(content: &str, display: &str) -> SourceContext {
         SourceContext {
             absolute: PathBuf::from("/tmp/test").join(display),
             display: PathBuf::from(display),
             content: Arc::from(content),
             frontmatter: None,
         }
     }
     ```
   - Use this helper in every test that needs a `ctx`. Tests that exercise frontmatter rendering should pass a `Some(0..N)` range pointing at the YAML head of `content`.
   - This keeps the diff small and gives one place to evolve later (e.g., when frontmatter byte-range extraction is added).

3. **Mechanical update of constructors.** For each test file, add the `ctx` field (use the helper) and remove obsolete fields. Do not adjust the `payload` or assertion logic.

**Step 2 exit criteria.**
- `cargo test -p darkmatter --no-run` exits 0.
- Commit: `test(darkmatter): update error_snapshots constructors to current variant signatures`.

### Step 3 — Snapshot baseline review

**Goal.** All snapshot tests pass, with any drifted snapshots intentionally re-accepted after visual review against the spec §3.5 quality bar.

**Estimated effort.** 1–2 hours — the manual review is the bottleneck, not the running.

**Sub-steps:**

1. Run `cargo insta test -p darkmatter` once to see which baselines drift.
2. For each drift, run `cargo insta review` and inspect the diff:
   - **Accept** when the new output matches spec §3.5 (no bare `<dim>`, linked path, frontmatter snapshot, excerpt with `>` gutter, hint).
   - **Reject and fix** when the new output regresses (literal tag leak, missing excerpt, malformed fence).
3. The canonical reference is `error_snapshots__page_block__unterminated_block.snap`. If it doesn't show a linked file path, frontmatter block, and gutter-marked excerpt, **stop** — there's a code bug, not a snapshot bug.
4. Commit accepted snapshots in a separate commit per related variant cluster (one for `page_block`, one for `transclusion`, etc.) so review is digestible.

**Step 3 exit criteria.**
- `cargo test -p darkmatter` exits 0 with all snapshot tests included.
- Each accepted snapshot has been visually reviewed against the spec §3.5 bar.
- Commits: one per variant cluster, e.g. `test(darkmatter): accept good-errors snapshots for page_block variants`.

### Step 4 — Address review-2 substantive findings

These were already partially scoped in review-2's "Short-Term" recommendations but were blocked by compilation. With baseline restored, each becomes its own focused, individually-green change.

#### 4a — ANSI-preserving render helper

**Why.** Review-2 §6 (correctly) flags that the existing `render()` test helper strips ANSI codes before assertion, so it cannot verify OSC 8 hyperlinks, color, or `<inverse>` styling — exactly the styling the spec §3.1 requires.

**Action.**
- In `darkmatter/lib/tests/error_snapshots/helpers.rs`, add a sibling helper:
  ```rust
  pub fn render_with_ansi(err: &dyn BlockError) -> String {
      // Same as render(), without strip_escape_codes.
  }
  ```
- Add at least one assertion per styling-sensitive variant verifying the expected escape sequence is present:
  - OSC 8 `\x1b]8;;file://` for linked paths.
  - SGR `\x1b[7m` (inverse) inside the hint that contains `::end-block`.
- Do **not** snapshot ANSI-laden output — escape sequences are noisy and the byte-level form drifts. Use `assert!(out.contains(...))` for the specific escape codes.
- Commit: `test(darkmatter): add ANSI-preserving assertions for styled error output`.

#### 4b — Audit `.body(format!(...))` call sites for `Vec<Prose>` idiom

**Why.** Review-2 §8 — `DeferredSetError`, `StylesheetError`, and others still pass single multi-line `format!` strings instead of `Vec<Prose>` paragraphs. They render acceptably (Prose now parses the markup) but don't get paragraph separation between conceptual chunks.

**Action.**
- Grep workspace-wide: `rg "\.body\(format!" darkmatter/lib/src` to enumerate.
- For each multi-line format string, split into a `vec![ Prose::new(...), Prose::new(...) ]` where the original `\n\n` separators were intentional paragraph breaks.
- Re-run snapshots; accept the cosmetic diffs.
- Commit per file (or per error type) so each diff is reviewable.

#### 4c — Author `darkmatter/docs/errors/README.md`

**Why.** Spec §3.7. Review-2 §3 confirms it doesn't exist.

**Action.**
- Author the doc covering: the body-is-`Vec<Prose>` contract, `SourceContext` requirement for file-origin errors, standard structure (linked header → frontmatter → excerpt → hint), and the snapshot test requirement.
- Cross-reference from `darkmatter/README.md` (drift-maintenance per spec §7).
- The existing skill at `darkmatter/.claude/skills/darkmatter/errors.md` should remain authoritative for AI contributors and link to this doc as the canonical human reference.
- Commit: `docs(darkmatter): add error rendering conventions guide`.

#### 4d — Update `biscuit-terminal/README.md`

**Why.** Spec §7 drift-maintenance. The `StatusBlock::body` signature change and new `SourceContext` type are public API and should be discoverable.

**Action.**
- Add a short "Errors" section to `biscuit-terminal/README.md` describing `SourceContext`, `body: Vec<Prose>`, and `body_line` shortcut. Link to the darkmatter doc for end-to-end usage.
- Commit: `docs(biscuit-terminal): document SourceContext and StatusBlock body API`.

### Step 5 — Defer (out of scope for recovery)

The following items from spec §6 ("Open Questions") and review-2 ("Medium-Term") are deliberately **not** part of recovery:

- **Syntax highlighting in fenced code blocks** (spec §6.1) — visual-quality enhancement, gated on Phase 1 baseline.
- **Composed-document line-number caveat phrasing** (spec §6.2) — needs real usage data first.
- **Frontmatter truncation rule** (spec §6.4) — wait for a document that triggers the issue.
- **Level 2 / Level 3 terminal-emulator tests** (review-2 §6) — meaningful only after step 4a is in place. Track as a follow-up feature.
- **`SourceContext` adoption outside darkmatter** (spec §2.2 non-goal) — explicitly out of scope.

Open a follow-up issue for each before closing this feature.

## Workflow Discipline

Per `darkmatter/.claude/skills/darkmatter/errors.md` and the project conventions:

- **No `cargo build` at workspace root.** Use `-p darkmatter` and `-p biscuit-terminal` only.
- **One green checkpoint per commit.** Steps 1–4 each break into multiple commits. No commit lands red.
- **Snapshot acceptance is a manual gate.** `cargo insta accept --all` is forbidden during recovery — review each diff against spec §3.5.
- **No subagent commits.** Subagents may implement and test in worktrees but commits stay with the human/orchestrator.

## Risk & Rollback

- **Step 1 risk.** A signature thread (e.g., `parse_directives` ctx propagation) might force changes deeper than expected. Mitigation: keep each sub-step in its own working-tree state and `git diff --stat` before proceeding. If a sub-step balloons beyond ~10 files touched, pause and re-evaluate scope.
- **Step 2 risk.** Test helper signature may need broader change than `test_ctx(...)`. If multiple test files need divergent helpers, fold them into a builder. Don't proliferate one-off helpers.
- **Step 3 risk.** A cascade of "wrong" snapshots could indicate a real regression in `StatusBlock::body` or `Prose` rather than acceptable drift. If more than ~5 snapshots show literal tag leakage (`<dim>`, `<cyan>` in plain output), **stop and investigate** — it points at a Phase 1 bug, not a Phase 2 sweep gap.
- **Rollback.** All work is in commits on the `darkmatter` branch. `git reset --hard f37e4ef9` restores the broken state if the recovery itself goes sideways. (Note: this requires explicit user authorization — do not execute autonomously.)

## Verification Summary

| Step | Gate | Command |
|---|---|---|
| 1 | Library compiles | `cargo check -p darkmatter` |
| 2 | Tests compile | `cargo test -p darkmatter --no-run` |
| 3 | All snapshots pass | `cargo test -p darkmatter` |
| 4a | ANSI assertions pass | `cargo test -p darkmatter` |
| 4b | Idiom audit complete | `rg "\.body\(format!" darkmatter/lib/src` returns 0 results |
| 4c–4d | Docs land | manual review |

## Total Estimate

- Step 1: 2–3 hours
- Step 2: 1–2 hours
- Step 3: 1–2 hours (review-bound, not edit-bound)
- Step 4: 4–6 hours total across sub-steps
- **Total: ~10–13 hours of focused work** to take darkmatter from non-compiling to fully spec-compliant Phase 1, with all of review-2's findings addressed.
