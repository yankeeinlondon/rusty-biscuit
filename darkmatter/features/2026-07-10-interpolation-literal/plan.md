---
agent: opencode/kimi-for-coding/k2p7
total_phases: 7
created: 2026-07-10
phase: 7
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/dmls/src/graph/substrate.rs
  - darkmatter/dmls/src/providers/dsl.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/code_actions.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - .claude/skills/darkmatter/compose.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_7: []
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
source_code:
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
  - darkmatter/lib/src/markdown/compose/remote.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/graph/substrate.rs
documentation:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - .claude/skills/darkmatter/compose.md
packages:
  - darkmatter
  - dmls
---

# Interpolation Literals Plan

## Success Criteria

- `{{{ name }}}` in body prose composes to the literal text `{{ name }}` with zero warnings and zero replacements counted for it.
- `` `{{{ … }}}` `` in an inline code span composes to `` `{{ … }}` `` and produces no DMLS diagnostic.
- `` `var_{{ phase }}` `` (no literal) continues to interpolate.
- Tight and empty forms work: `{{{x}}}` → `{{x}}`, `{{{}}}` → `{{}}`, `{{{ }}}` → `{{ }}`.
- Adjacency: `{{ a }}{{{ b }}}` evaluates `a` and emits `{{ b }}` literally.
- Four-brace opener `{{{{ x }}}}` and unclosed `{{{ x }}` reproduce today's behavior byte-for-byte.
- A literal containing a valid expression `{{{ {{ x }} }}}` emits `{{ {{ x }} }}` with `x` unevaluated.
- A literal inside a fenced code block is untouched source text after compose.
- Frontmatter: `key: "{{{ x }}}"` resolves to the string `{{ x }}`; the literal survives both interpolation passes.
- Body rescan loop: a replacement value introducing `{{{ y }}}` composes to literal `{{ y }}`.
- Demand-driven capture: `{{{ ctx.hardware }}}` does not trigger the hardware probe; a remote URL inside a literal does not appear in remote-discovery output.
- DMLS: no `dm.expression.*` diagnostic on a literal; hover renders the literal block; no `NodeKind::Interpolation` node or `uses_variable` edge is indexed for it.
- `fail_fast` compose over a document whose only `{{ }}`-shaped content is literals succeeds.

## Phase 1: Orientation & API Design

- [x] Read `darkmatter/features/2026-07-10-interpolation-literal/spec.md` and confirm the six scanner recognition rules, the conversion-after-final-scan ordering, and the DMLS inertness requirements.
- [x] Inspect `darkmatter/lib/src/markdown/compose/expression/lexer.rs` to understand the current `ExpressionFinder`, `ExpressionLocation`, `find_all`, and `find_all_plain` implementation and code-region exclusion.
- [x] Inspect `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs` to understand the body rescan loop, `interpolate_text`, and `interpolate_value`.
- [x] Inspect `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs` to understand the two-pass frontmatter flow, the `contains_interpolation` check, and the deferred-key path.
- [x] Inspect `darkmatter/lib/src/markdown/compose/context/capture/groups.rs` to understand how `scan_needed_groups` derives `ContextGroup` from raw `ctx.KEY` matches.
- [x] Inspect `darkmatter/lib/src/markdown/compose/remote.rs` to understand how `discover_remote_urls_from_expressions` uses `ExpressionFinder`.
- [x] Inspect `darkmatter/dmls/src/overlay/expressions.rs` and `darkmatter/dmls/src/providers/dsl.rs` to understand DMLS interpolation hover, diagnostics, and completion boundaries.
- [x] Inspect `darkmatter/dmls/src/graph/substrate.rs` to confirm how `VariableUseFact` is extracted from `expressions::interpolations` and how it becomes `NodeKind::Interpolation` / `uses_variable` edges.
- [x] Design the scanner API: add `InterpolationLiteral` (start, end, content) and `ExpressionScanResult` (expressions, literals) to `lexer.rs`, expose a new `scan()` method, and keep `find_all()` / `find_all_plain()` as convenience wrappers that return only expressions while delegating to `scan()`.
- [x] List every call site of `ExpressionFinder::new(...).find_all()`, `ExpressionFinder::find_all_plain(...)`, and any function that internally calls them; record how each must behave for literals.
- [x] Validation checkpoint: produce a written API contract and the expected file-change list before writing any implementation code.

## Phase 1 API Contract

### Scanner API (to be implemented in Phase 2)

- Add `InterpolationLiteral` to `darkmatter/lib/src/markdown/compose/expression/lexer.rs`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct InterpolationLiteral {
      /// Byte offset of the first `{` in the source.
      pub start: usize,
      /// Byte offset after the last `}` in the source.
      pub end: usize,
      /// The literal content between `{{{` and `}}}`, preserved verbatim.
      pub content: String,
  }
  ```
- Add `ExpressionScanResult` to `lexer.rs`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct ExpressionScanResult {
      pub expressions: Vec<ExpressionLocation>,
      pub literals: Vec<InterpolationLiteral>,
  }
  ```
- Add `ExpressionFinder::scan(&self) -> ExpressionScanResult` implementing the six recognition rules from the spec.
- Keep `find_all()` and `find_all_plain()` as convenience wrappers returning `Vec<ExpressionLocation>` by delegating to `scan()` and dropping `literals`.
- Re-export `InterpolationLiteral` and `ExpressionScanResult` from `darkmatter/lib/src/markdown/compose/expression/mod.rs`.

### Recognition rules (implemented in `scan()`)

1. Check for `{{{` before `{{` at every scan position.
2. Recognize a literal opener only when there are exactly three consecutive `{` characters (pos+3 is out of bounds or not `{`).
3. Skip openers inside fenced/indented code regions (same exclusion as `{{`).
4. Close a recognized literal at the first subsequent `}}}`.
5. On an unclosed `{{{` (no subsequent `}}}`), fall back to legacy `{{` scanning at the same byte position.
6. Runs of four or more `{` fall through to the existing `{{` nested-depth scanner byte-for-byte.

### Call-site behavior for literals

| Consumer | Current API | Literal behavior |
|----------|-------------|------------------|
| `rewrite.rs` `interpolate_text` | `find_all()` | Phase 3 adds `convert_literals` after the rescan loop; literals are not counted as replacements. |
| `rewrite.rs` `interpolate_value` / `whole_value_span` | `find_all_plain()` | Phase 3 treats a trimmed whole literal as a string, not a typed whole-value expression. |
| `frontmatter_interpolation.rs` `contains_interpolation` | `find_all_plain()` | Phase 3 ensures a pure literal returns `false`. |
| `frontmatter_interpolation.rs` key-ref extraction | `find_all_plain()` | Literals produce no frontmatter-key dependencies because they are not expressions. |
| `remote.rs` `discover_remote_urls_from_expressions` | `find_all()` | Already ignores literals because `find_all()` returns only expressions. |
| `groups.rs` `scan_needed_groups` | raw byte scan | Phase 4 switches to `ExpressionFinder::scan()` and skips `ctx.KEY` matches inside literal spans. |
| `dmls/overlay/expressions.rs` `interpolations` | `find_all()` | Already ignores literals. |
| `dmls/providers/dsl.rs` diagnostics/hover | `interpolations()` | Phase 5 adds `literal_hover`; diagnostics already ignore literals. |
| `dmls/graph/substrate.rs` variable indexing | `interpolations()` | Already ignores literals. |
| `claudine/*` | `find_all_plain()` | No change required; literals do not affect expression discovery. |

### Expected file-change list (Phases 2–7)

- `darkmatter/lib/src/markdown/compose/expression/lexer.rs`
- `darkmatter/lib/src/markdown/compose/expression/mod.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs`
- `darkmatter/lib/src/markdown/compose/context/capture/groups.rs`
- `darkmatter/dmls/src/overlay/expressions.rs`
- `darkmatter/dmls/src/providers/dsl.rs`
- `darkmatter/dmls/src/providers/code_actions.rs`
- `darkmatter/dmls/src/graph/substrate.rs`
- `darkmatter/docs/inline/interpolation.md`
- `darkmatter/docs/topics/darkmatter-expressions.md`
- `.claude/skills/darkmatter/compose.md`

## Phase 2: Scanner Foundation

- [x] Add `InterpolationLiteral` to `darkmatter/lib/src/markdown/compose/expression/lexer.rs` with `start`, `end`, and `content` fields.
- [x] Add `ExpressionScanResult` to `lexer.rs` carrying `expressions: Vec<ExpressionLocation>` and `literals: Vec<InterpolationLiteral>`.
- [x] Implement `ExpressionFinder::scan()` returning `ExpressionScanResult`, applying the recognition rules in order:
  - Check for `{{{` before `{{`.
  - Recognize only an exact run of three `{` (pos+3 is out of bounds or not `{`).
  - Skip openers inside fenced/indented code regions.
  - Close at the first subsequent `}}}`.
  - On unclosed `{{{`, fall back to legacy `{{` scanning at the same byte position.
  - Four-or-more `{` runs fall through to the existing `{{` nested-depth scanner byte-for-byte.
- [x] Update `ExpressionFinder::find_all()` to call `scan()` and return only `expressions`, preserving its current signature so existing callers compile unchanged.
- [x] Update `ExpressionFinder::find_all_plain()` to call `scan()` with empty code regions and return only `expressions`.
- [x] Re-export `InterpolationLiteral` and `ExpressionScanResult` from `darkmatter/lib/src/markdown/compose/expression/mod.rs`.
- [x] Add unit tests in `lexer.rs` covering:
  - Simple literal `{{{ name }}}`.
  - Literal inside inline code `` `{{{ name }}}` ``.
  - Literal skipped inside fenced code block.
  - Tight form `{{{x}}}` and empty forms `{{{}}}`, `{{{ }}}`.
  - Adjacent expression and literal `{{ a }}{{{ b }}}`.
  - Four-brace opener `{{{{ x }}}}` preserves legacy behavior.
  - Unclosed `{{{ x }}` preserves legacy behavior.
  - Literal containing an expression `{{{ {{ x }} }}}`.
  - `find_all()` returns no expression for a pure literal.
- [x] Validation checkpoint: run the new scanner tests and the existing `expression_finder` tests; confirm no regressions.

## Phase 3: Compose Integration

- [x] Add a `convert_literals(input: &str) -> String` helper in `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs` that uses `ExpressionFinder::scan()` to find valid literals and replace each `{{{ content }}}` span with `{{ content }}` from end to start to preserve byte offsets.
- [x] Integrate `convert_literals` into `interpolate_text` so it runs once after the rescan loop terminates, before returning `InterpolationRewrite::output`.
- [x] Ensure `interpolate_text` counts and warnings are unaffected by literal conversion (the replacement count must not include literals).
- [x] In `interpolate_value`, add a guard that treats a frontmatter value whose trimmed content is exactly one `{{{ … }}}` as a string (not a typed whole-value expression), routing it through `interpolate_text` and conversion.
- [x] Add a `convert_frontmatter_literals(frontmatter: &mut Frontmatter)` helper in `frontmatter_interpolation.rs` that walks every `Value::String` and applies literal conversion after the final interpolation pass.
- [x] Invoke `convert_frontmatter_literals` at the end of `interpolate_frontmatter_impl` after the fallback pass (i.e., after pass 2 when shell expansion is enabled, or after pass 1 when it is disabled).
- [x] Ensure `contains_interpolation` and the seed/templated classification remain based on real `{{ }}` expressions only; a pure literal frontmatter value must not be classified as templated.
- [x] Add unit tests in `rewrite.rs` for body conversion: `{{{ name }}}`, inline code, fenced code blocks, empty forms, adjacency, and rescan loop introduction.
- [x] Add unit tests in `frontmatter_interpolation.rs` for frontmatter literal conversion, two-pass survival with shell expansion, and the string-path (not typed whole-value) behavior.
- [x] Validation checkpoint: run the interpolation and frontmatter interpolation unit tests; confirm the acceptance-criteria compose cases pass.

## Phase 4: Inert-Consumer Guarantees

- [x] Update `scan_needed_groups` in `darkmatter/lib/src/markdown/compose/context/capture/groups.rs` to use `ExpressionFinder::scan()` and skip `ctx.KEY` matches that fall inside `InterpolationLiteral` spans, ensuring `{{{ ctx.hardware }}}` does not trigger `ContextGroup::Hardware`.
- [x] Add unit tests in `groups.rs` proving that `ctx.hardware` inside a literal is ignored while `ctx.hardware` outside a literal still triggers `Hardware`.
- [x] Verify that `discover_remote_urls_from_expressions` in `darkmatter/lib/src/markdown/compose/remote.rs` naturally ignores literal content because it uses `ExpressionFinder::new(...).find_all()`; add a regression test that a URL inside `{{{ ... }}}` is not discovered.
- [x] Verify that `darkmatter/dmls/src/graph/substrate.rs` naturally ignores literals because `expressions::interpolations` uses `ExpressionFinder::find_all()`; add a substrate test that `{{{ title }}}` produces no `VariableUseFact` and therefore no `NodeKind::Interpolation` / `uses_variable` edge.
- [x] Verify that `darkmatter/dmls/src/providers/dsl.rs` naturally emits no `dm.expression.*` diagnostic for a literal because `expression_diagnostics` uses `expressions::interpolations`; add a diagnostic test for `{{{ > invalid }}}`.
- [x] Validation checkpoint: run the context-capture, remote, and DMLS unit tests affected by the new behavior; confirm no unintended side effects.

## Phase 5: DMLS Integration

- [x] Add `Literal` (re-exporting `InterpolationLiteral`) and helper functions `literals(text, body_base)` and `literal_at(text, body_base, offset)` to `darkmatter/dmls/src/overlay/expressions.rs`, mirroring the existing `interpolations` / `interpolation_at` helpers.
- [x] Add unit tests in `overlay/expressions.rs` for `literals` and `literal_at`, including frontmatter-base filtering and inline-code spans.
- [x] Implement `literal_hover` in `darkmatter/dmls/src/providers/dsl.rs` that renders a Markdown block identifying the span as an interpolation literal, showing the composed output `{{ content }}`, and noting that the content is not interpolated.
- [x] Wire `literal_hover` into the `dsl::hover` dispatcher before the fallback `frontmatter_shell_hover` so a cursor inside a literal gets the literal hover.
- [x] Add L2 or unit tests in `dsl.rs` for literal hover on `{{{ name }}}` and `{{{ {{ x }} }}}`.
- [x] (Optional / deferrable) Implement a diagnostic-driven code action in `darkmatter/dmls/src/providers/code_actions.rs` that wraps an offending `{{ ... }}` span from `dm.expression.malformed` into `{{{ ... }}}`.
- [x] (Optional / deferrable) Add a test for the wrap-in-literal code action if implemented; otherwise document it as a follow-up item in the phase handoff.
- [x] Validation checkpoint: run DMLS tests and confirm hover renders the literal block and no expression diagnostic is emitted for literal spans.

## Phase 6: Documentation Updates

- [x] Update `darkmatter/docs/inline/interpolation.md`:
  - Add an **Interpolation Literals** section documenting `{{{ ... }}}`, first-`}}}` termination, the unclosed-opener fallback, and the fenced-code-block alternative.
  - Update the **Implementation** section's scanner description to mention literal recognition alongside code-block exclusion.
- [x] Update `darkmatter/docs/topics/darkmatter-expressions.md` to add the literal syntax to the expression-surface documentation and note that it is inert on every scanning surface.
- [x] Update `.claude/skills/darkmatter/compose.md` to mention interpolation literals and regenerate the skill file's `hash:` frontmatter with `md hash <file>`.
- [x] Validation checkpoint: run `md hash` on the updated skill file and confirm the frontmatter hash is refreshed; verify that the markdown docs render without errors.

## Phase 7: Final Verification & Handoff

- [x] Run `just test` from the `darkmatter` package area to execute the full unit test suite through nextest.
- [x] Run `just test-l2` from the `darkmatter` package area if DMLS integration tests are present and applicable.
- [x] Run `just lint` from the `darkmatter` package area; do not run `cargo fmt` unless explicitly requested.
- [x] Review `git diff` for the implementation and documentation files; reject any unrelated formatting churn, drifted comments, or behavior changes outside the spec.
- [x] Confirm every acceptance criterion from the spec has a passing test or an explicit parity/docs check.
- [x] Handoff note: summarize changed files, tests run, any skipped validation, and the status of the optional wrap-in-literal code action.

**Handoff note:** All 14 acceptance criteria from `spec.md` are covered by passing tests in `darkmatter` and `dmls`. `just test` passed (545 darkmatter-cli, 399 dmls), `just test-l2` passed (19 darkmatter + 69 darkmatter-cli + 0 dmls), and `just lint` passed. The optional "wrap in interpolation literal" code action was implemented in `dmls/src/providers/code_actions.rs` and has a passing unit test. The skill hash for `.claude/skills/darkmatter/compose.md` was verified with `md hash` and matches the existing frontmatter. No unrelated formatting or behavior churn was introduced.
