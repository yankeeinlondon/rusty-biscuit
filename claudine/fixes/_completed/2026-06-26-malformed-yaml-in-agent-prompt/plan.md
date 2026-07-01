---
agent: open_code/zai-coding-plan/glm-5.2
phases: 3
created: 2026-06-26
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/frontmatter.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/composition/resolve.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/frontmatter_excerpt.rs
  - claudine/lib/src/composition/error.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/mod.rs
  - claudine/cli/src/output/error_walker.rs
  - claudine/lib/src/composition/resolve.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/SKILL.md
source_code:
  - darkmatter/lib/src/markdown/types.rs
  - darkmatter/lib/src/markdown/frontmatter.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/mod.rs
  - claudine/lib/src/composition/resolve.rs
  - claudine/lib/src/composition/prepare.rs
  - claudine/lib/src/composition/frontmatter_excerpt.rs
  - claudine/lib/src/composition/error.rs
  - claudine/cli/src/output/error_walker.rs
documentation: []
packages:
  - darkmatter
  - claudine
---

# Execution Plan — Malformed Frontmatter Fence Silently Leaks YAML Into the Agent Prompt

Source spec: `claudine/fixes/2026-06-26-malformed-yaml-in-agent-prompt/spec.md`

## Summary

Convert the silent near-miss-frontmatter leak (`----` fences wrapping YAML) into
a precise, actionable typed error. Detection lives in **Darkmatter**
(`parse_frontmatter`); **Claudine** classifies and enriches the error at its
existing frontmatter render boundary so the offending fence line is highlighted
in the user's own file.

## Key findings from code reconnaissance

These were verified against the current tree and affect how the tasks below are
scoped:

1. **The shipped reproduction no longer exists.** `prompts/cross-platform.md`
   opens with `---\n` (byte-verified: three dashes), not `----`. The spec's
   "Immediate workaround" has already been applied to the shipped file.
   Consequences:
   - Acceptance criterion #5 ("once fixed to `---`, composes with frontmatter
     stripped") is satisfied today by a **regression** assertion, not by editing
     the shipped prompt.
   - All positive/negative detection tests must use **synthetic `----`
     fixtures**, not the shipped prompt.
2. **Detection site confirmed.** `darkmatter/lib/src/markdown/frontmatter.rs:220`
   is the exact-match guard (`lines[0].trim() != "---"`) that silently returns
   the whole document as body. The closing-fence scan at
   `frontmatter.rs:228` has the same exact-match constraint.
3. **`MarkdownError` is a non-exhaustive-feeling enum but is matched exhaustively**
   in two places that the compiler will force updates to: the `BlockError`
   impl at `darkmatter/lib/src/markdown/types.rs:195` and (transitively) the
   block helpers in `darkmatter/lib/src/markdown/errors/blocks.rs`.
4. **Claudine resolve mapping is a single match arm.**
   `claudine/lib/src/composition/resolve.rs:18` (`map_load_error`) only routes
   `MarkdownError::FrontmatterParse` to `CompositionError::FrontmatterParse`;
   the new variant currently falls through to the flat `MarkdownLoad` string and
   must be added.
5. **`prepare.rs::map_compose_error`** (`claudine/lib/src/composition/prepare.rs:19`)
   already funnels every non-shell `MarkdownError` through `ComposeFailed(other)`
   while keeping the typed source via `#[source]`. Acceptance criterion #2
   (inline-compose / sequence surfaces the error) is largely satisfied for free
   once Darkmatter raises the typed error; this task is a **verify + test**,
   not a structural change.
6. **The excerpt enrichment has a real design gap to close.** Two interlocking
   limitations in `claudine/lib/src/composition/frontmatter_excerpt.rs` block the
   "highlight the fence line" goal:
   - `capture_frontmatter_block` (line 96) only recognizes `---` openings
     (`opening.trim() != "---"` → `None`), so a `----` block is never captured.
   - `FrontmatterExcerpt::capture` (line 47) takes a dotted **property** path,
     not a line number, so it cannot target the delimiter on line 1.
   - `frontmatter_block_spec` (`claudine/lib/src/composition/error.rs:1413`)
     returns `Option<Option<String>>` (property-or-none), which cannot express
   "highlight line N". This return type (or the enrichment dispatch) must be
   extended. See Phase 2, Task 3.
7. **`serde_yaml_ng::Value` probe API is already in use** in the darkmatter
   test suite (`errors/blocks.rs:391,412,436`) and the crate is re-exported via
   `biscuit_file::serde_yaml_ng` (`frontmatter.rs:7`). The spec's recommended
   probe (`serde_yaml_ng::from_str::<serde_yaml_ng::Value>`) is consistent with
   existing usage.

## Dependencies and parallelism at a glance

- **Phase 1 → Phase 2** is a hard dependency: Claudine maps and renders a
  Darkmatter-defined variant.
- Within Phase 1, the variant definition (Task 1) gates Tasks 2–3; the compiler
  enforces the new match arms. Tasks 2 and 3 touch disjoint files and can be
  developed in parallel once Task 1 lands.
- Within Phase 2, Tasks 1, 2, and 3 are independent of each other and can be
  parallelized; Task 4 depends on Task 3 (uses the new capture helper).
- Phase 3 is validation only and runs after Phases 1–2.

---

## Phase 1 — Darkmatter: near-miss detection, typed error, and block rendering

**Goal:** `parse_frontmatter` raises `MarkdownError::FrontmatterFenceMismatch`
for a matched dash-only (`----`+) fence pair wrapping a non-empty YAML mapping,
and the `BlockError` render shows the offending fence, the fix, and the source
path with a line-1 excerpt. Legitimate leading thematic breaks and non-mapping
content are unchanged.

### Task 1.1 — Add the `FrontmatterFenceMismatch` variant

- [x] Add variant to `MarkdownError` in `darkmatter/lib/src/markdown/types.rs` (insert near the existing `FrontmatterParse` arm, ~line 27):
  ```rust
  FrontmatterFenceMismatch {
      ctx: SourceContext,
      found: String,
      line: usize,
  }
  ```
  with a `#[error(...)]` message naming the offending fence and the fix, e.g.
  `"frontmatter fence must be exactly `---`, found `{found}` on line {line} in {}"`.

**Validation:** `cargo check -p darkmatter` surfaces the two non-exhaustive
match sites (types.rs `status_block`, blocks.rs callers) that the next tasks
fill. No new logic yet.

### Task 1.2 — Implement the conservative detection heuristic

*(Parallelizable with Task 1.3 after Task 1.1 lands — disjoint file.)*

- [x] In `darkmatter/lib/src/markdown/frontmatter.rs::parse_frontmatter` (the early-return guard at line 220), before returning "no frontmatter", detect a near-miss fence pair and raise `FrontmatterFenceMismatch`. Heuristic (all must hold):
  1. `lines[0].trim()` is a dash-only run (`^-+$`) of length `>= 4`.
  2. A later line exists whose trimmed content is the **same** dash run (exact-match closing fence; do not normalize `----` ↔ `-----`).
  3. The strict interior between the fences is non-empty, parses as a YAML **mapping** via `serde_yaml_ng::from_str::<serde_yaml_ng::Value>`, and has `>= 1` key. Scalar / sequence / empty-map / parse-failure → treat as body (no error, unchanged behavior).
  - Do **not** reuse `parse_yaml_with_fallbacks` for this probe (keep it a conservative shape check).
  - Preserve all existing `---` behavior, including "missing closing delimiter ⇒ body text".
- [x] Add focused unit tests in `frontmatter.rs` (`mod tests`):
  - `----`…`----` wrapping a YAML map → `Err(FrontmatterFenceMismatch { found: "----", line: 1 })`.
  - `-----`…`-----` wrapping a YAML map → `Err(... found: "-----" ...)`.
  - Leading `----` thematic break followed by prose (no matched closing dash run) → `Ok`, empty frontmatter, content unchanged.
  - Matched `----` fences around a scalar / sequence / empty map → `Ok`, no error (treated as body).
  - Mismatched pair (`----` open, `-----` close) → `Ok`, no error (not a matched pair).
  - Correct `---` document parses as before (regression — existing tests stay green).

**Validation:** `just test` in `darkmatter/` (or `just test darkmatter` from
repo root). All new detection tests pass; existing frontmatter tests unchanged.

### Task 1.3 — Add the `BlockError` render arm

*(Parallelizable with Task 1.2 after Task 1.1 lands — disjoint file.)*

- [x] Add a `MarkdownError::FrontmatterFenceMismatch { ctx, found, line }` arm to `status_block` in `darkmatter/lib/src/markdown/types.rs:195` that delegates to a new `blocks::frontmatter_fence_mismatch_block(ctx, found, line)` helper.
- [x] Add `frontmatter_fence_mismatch_block` to `darkmatter/lib/src/markdown/errors/blocks.rs` with:
  - header: `MarkdownError` / `frontmatter fence mismatch`;
  - body: source path (when `ctx.display != "unknown"`, reuse `ctx.linked_path_prose()`), the offending fence, and the document line;
  - hint: `Use exactly three dashes (---) for Markdown frontmatter fences.`;
  - excerpt: highlight `line` (currently always 1) using the existing `frontmatter_excerpt_prose(ctx, line, context)` helper — note it already reads the **full document** from `ctx.content`, which is correct here because the offending token is the delimiter itself, not the YAML interior.
- [x] Add unit tests in `blocks.rs` (`mod tests`):
  - The mismatch block names the offending fence (`----`) and suggests `---`.
  - The block includes the source path when `ctx` carries one.
  - The block highlights document line 1 (gutter marker `> 1 │ ----`) without relying on a YAML parser location.

**Validation:** `just test` in `darkmatter/`. New block tests pass.
**Phase 1 exit checkpoint:** `just test darkmatter && just lint darkmatter` green;
a `----`-fenced synthetic document loaded via `Markdown::try_from_content`
returns `Err(MarkdownError::FrontmatterFenceMismatch)`.

---

## Phase 2 — Claudine: error mapping and excerpt enrichment

**Goal:** The new typed error is classified as frontmatter-rooted (not a flat
`MarkdownLoad` string), and the CLI renders a syntax-highlighted, line-numbered
excerpt with the offending fence highlighted — TTY-gated, stripped at
`ColorDepth::None`.

### Task 2.1 — Map the variant in `resolve.rs`

*(Parallelizable with Tasks 2.2 and 2.3 — independent file/change.)*

- [x] In `claudine/lib/src/composition/resolve.rs::map_load_error` (line 18), add a `MarkdownError::FrontmatterFenceMismatch { .. }` arm routing to `CompositionError::FrontmatterParse(err)` (reuse is acceptable per spec — same user-facing category).
- [x] Add a test in `resolve.rs` (`mod tests`): a temp file fenced with `----`…`----` around a YAML map resolves to `Err(CompositionError::FrontmatterParse(_))`, **not** `MarkdownLoad`/`FileNotFound`/`PromptPropertyMissing`.

**Validation:** `just test` in `claudine/` for the resolve module.

### Task 2.2 — Verify the compose-time path preserves the typed error

*(Parallelizable with Tasks 2.1 and 2.3 — independent file/change.)*

- [x] Confirm `claudine/lib/src/composition/prepare.rs::map_compose_error` (line 19) already funnels `FrontmatterFenceMismatch` through `ComposeFailed(other)` with the typed source intact (it should, since it's a catch-all). No structural change expected.
- [x] Add a test that a transclusion/reload surface producing a `----` fence surfaces the typed `MarkdownError` in the `ComposeFailed` source chain (assert the error string names the offending fence). If no realistic transclusion fixture is reachable, document why and cover via the `try_from_content` path in Phase 3 instead.

**Validation:** `just test` in `claudine/` for the prepare module.

### Task 2.3 — Extend `FrontmatterExcerpt` for line-target capture

*(Parallelizable with Tasks 2.1 and 2.2 — independent file/change. **Task 2.4
depends on this one.**)*

This closes the design gap identified in finding #6.

- [x] Add a near-miss-aware block capture helper in `claudine/lib/src/composition/frontmatter_excerpt.rs` (sibling to `capture_frontmatter_block`) that recognizes a matched dash-only (`----`+) fence pair and returns the block **including** both delimiter lines (so line 1 in the block equals line 1 in the source file).
- [x] Add a line-target capture constructor to `FrontmatterExcerpt`, e.g. `FrontmatterExcerpt::capture_line(source_text, line: usize, stderr_is_tty: bool) -> Option<Self>`, which uses the near-miss block capture and sets `highlight_line = Some(line)`. Do not invent a separate renderer — reuse the existing `render_appendix` (it already honors `highlight_line` and the TTY / `ColorDepth::None` gating).
- [x] Add unit tests:
  - `capture_line` on a `----`…`----` document returns `Some` with `highlight_line == Some(1)`.
  - `capture_line` on a plain-prose document (no near-miss pair) returns `None`.
  - `render_appendix` of the captured excerpt highlights the fence line in TTY output and is empty when `stderr_is_tty == false`.

**Validation:** `just test` in `claudine/` for the excerpt module.

### Task 2.4 — Wire enrichment to highlight the fence line

*(Depends on Task 2.3.)*

- [x] Extend the enrichment dispatch in `claudine/lib/src/composition/error.rs` so `FrontmatterFenceMismatch` highlights the fence line. Two viable shapes (pick the more surgical):
  - **(a)** Broaden `frontmatter_block_spec`'s return (line 1413) from `Option<Option<String>>` to a small enum `{ Property(String), Line(usize), BlockOnly }`, and have the `FrontmatterParse` arm detect a wrapped `FrontmatterFenceMismatch` → `Line(err.line)`. Update `enrich_frontmatter` (line 1376) to dispatch `Line(n)` → `FrontmatterExcerpt::capture_line`, `Property(p)` → existing `capture`, `BlockOnly` → `capture(.., None, ..)`. This is the spec's stated preference ("`frontmatter_block_spec` must highlight the fence line").
  - **(b)** Special-case in `enrich_frontmatter`: detect `CompositionError::FrontmatterParse(MarkdownError::FrontmatterFenceMismatch { line, .. })` before the generic `frontmatter_block_spec` path and call `capture_line` directly.
  - Prefer (a) if the call-site churn is small; otherwise (b). Either way the user sees line 1 highlighted.
- [x] Ensure the wrapped `MarkdownError::FrontmatterFenceMismatch` is reachable through `CompositionError::FrontmatterParse(err)` for the `frontmatter_block_spec`/`enrich_frontmatter` match (the inner `err` is the typed `MarkdownError`).
- [x] Add unit tests in `error.rs`:
  - `enrich_frontmatter` on a `FrontmatterParse(FrontmatterFenceMismatch)` error produced from a `----` source attaches a `WithFrontmatter` excerpt (i.e. `frontmatter_excerpt()` is `Some`).
  - The excerpt's `highlight_line` is `Some(1)` (assert via the captured block / a render snapshot).

**Validation:** `just test` in `claudine/`.
**Phase 2 exit checkpoint:** `just test claudine && just lint claudine` green;
the CLI error walker (`claudine/cli/src/output/error_walker.rs`) already renders
`WithFrontmatter` excerpts and needs no change — verify by inspecting
`try_render_block_report` / `find_frontmatter_wrapper` (lines 28–70) which
already append `excerpt.render_appendix`.

---

## Phase 3 — Cross-package validation and regression

**Goal:** All seven acceptance criteria from the spec are demonstrably met.

### Task 3.1 — Verify acceptance criteria #3 and #4 (no false positives / no regression)

- [x] Confirm (via the Phase 1 tests) that a legitimate leading `----` thematic break + prose renders unchanged (criterion #3), and that a correctly-fenced `---` document is unaffected (criterion #4). Add an explicit darkmatter regression test if Phase 1 did not already cover the `---` round-trip end-to-end through `Markdown::try_from_content`.

### Task 3.2 — Verify acceptance criterion #6 (both load paths return the typed error)

- [x] Add/confirm tests that a `----`-fenced YAML-mapping document returns `Err(MarkdownError::FrontmatterFenceMismatch)` through **both** `Markdown::try_from(path)` (`darkmatter/lib/src/markdown/mod.rs:973`) and `Markdown::try_from_content(content)` (`mod.rs:285`).
- [x] Confirm the infallible `From<String>` path (`mod.rs:948`) swallows the error by design (returns a `Markdown::new(content)` with the raw text) and is **not** used by Claudine's prompt-loading path (`resolve.rs:69` uses `Markdown::try_from`). Document this in a test comment so the asymmetry is not mistaken for a bug.

### Task 3.3 — Verify acceptance criterion #7 (non-TTY output)

- [x] Add/confirm a test that the typed error + actionable hint render in non-TTY / `ColorDepth::None` output **without** ANSI styling and **without** the TTY-only frontmatter appendix (unless `FORCE_COLOR=1`). The existing `render_appendix` gating (`stderr_is_tty == false` → empty string; `ColorDepth::None` → `strip_escape_codes`) should already enforce this — assert it holds for the fence-mismatch path.

### Task 3.4 — Verify acceptance criterion #5 (shipped prompt composes cleanly)

- [x] Add a regression test that composing `prompts/cross-platform.md` (which is already `---`-fenced — see finding #1) yields non-empty frontmatter and a body beginning `# Ensuring Cross Platform Support`, with **no** `name:`/`description:` YAML leaking into the body. This locks in the already-applied workaround.

### Task 3.5 — L2 real-terminal capture (optional, per spec)

- [x] *(Optional)* Add an L2 real-terminal test proving the highlighted excerpt renders and that no YAML appears in the Agent Prompt section for a synthetic `----` fixture. Gate behind the existing L2 harness/tier conventions (`rust-testing` skill). Skip if the L1 unit coverage is judged sufficient.

### Task 3.6 — Full lint and test sweep

- [x] Run `just test darkmatter && just test claudine` from the repo root (or `just test` within each package area).
  - `claudine`: 1764/1764 passed.
  - `darkmatter`: 4604/4605 passed; one pre-existing unrelated failure in `darkmatter::expression_regression regression_page_block_with_has_skill` (`darkmatter/lib/tests/expression_regression.rs:712`) — outside the frontmatter-fence blast radius.
- [x] Run lint for both package areas: `just -f darkmatter/justfile lint && just -f claudine/justfile lint`.
- [x] Run doctests if touched: `just doctest darkmatter` (the `try_from_content` doc comment may be referenced).

**Final checkpoint:** all acceptance criteria #1–#7 from the spec are satisfied
and demonstrable via the test suite; `just test` + `just lint` are green for both
`darkmatter` and `claudine`.
