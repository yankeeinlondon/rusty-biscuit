---
agent: "codex"
phases: 8
created: "2026-06-29"
start_phase: 1
yolo: false
---

# Execution Plan - Real Errors

Assumption: the duplicate `agent` frontmatter requirement cannot be represented as one valid YAML key with two values, so this plan uses `agent: "codex"` and keeps the rest of the requested frontmatter exact. The implementation should preserve current fatal/warn behavior unless a later product decision explicitly promotes missing file references to fatal errors.

## Phase 1 - Characterize Current Compose Fatality

Goal: lock the existing warn-vs-fatal behavior before any typing refactor changes semantics.

- [ ] Add a characterization matrix for expression failures across `unknown-function`, `missing-file`, `malformed-path`, `arity`, `arg-type`, and `parse`.
- [ ] Cover each failure across `fail_fast` and lenient compose modes.
- [ ] Cover each failure in both frontmatter whole-value spans and body interpolation.
- [ ] Assert the current contract: unknown functions are fatal in lenient mode, while missing file references and the other non-unknown-function failures remain warnings unless existing strict surfaces already abort.
- [ ] Record the expected outcomes in test names or table data so later failures clearly identify which semantic case drifted.
- [ ] Run the Darkmatter unit tests that cover composition rewriting and warnings.
- [ ] Validation checkpoint: the matrix is green before any production error type changes land.

Parallelizable after the matrix shape is agreed:

- [ ] Build small fixture prompt documents for each expression surface.
- [ ] Add helper assertions for fatal result, warning result, and emitted warning content.

## Phase 2 - Type Darkmatter Expression Errors Without Changing Display

Goal: introduce the typed substrate while keeping user-visible output and the Phase 1 matrix unchanged.

- [ ] Add `ExpressionError` in Darkmatter with variants for `FileReference`, `UnknownFunction`, `Arity`, `ArgType`, `Parse`, and `Other`.
- [ ] Add `FileReferenceDiagnostic` with `function`, `reference`, `kind`, `base_dir`, `fallback_dir`, and typed `source`.
- [ ] Add `FileRefFailure` values for malformed, not found, found elsewhere, and remote-not-enabled cases.
- [ ] Convert expression evaluation return paths from `Result<Value, String>` to carry `ExpressionError` at the dispatch boundary.
- [ ] Convert `resolve_arg`, `frontmatter_fn`, `absolute`, `relative`, and `load_markdown` to preserve file-reference causes instead of formatting them away.
- [ ] Preserve existing `Display` text for typed variants during this phase so snapshot and warning output do not change.
- [ ] Replace `is_fatal_eval_error(message)` string-prefix logic with a checked match over typed causes that preserves the Phase 1 outcomes.
- [ ] Keep parser failures behind `ExpressionError::Parse(String)` and pure-function long tail failures behind `ExpressionError::Other`.
- [ ] Measure or inspect large-result-size impact; only box the error arm if the success path regresses measurably.
- [ ] Validation checkpoint: Phase 1 matrix remains green, existing Darkmatter compose snapshots remain behavior-neutral, and no user-facing render change is introduced.

Parallelizable after `ExpressionError` exists:

- [ ] Convert filesystem builtins (`absolute`, `relative`, `load_markdown`) to the shared `FileReferenceDiagnostic`.
- [ ] Convert pure builtin errors to `Other` where precise variants are not yet worth adding.
- [ ] Add focused unit tests for `FileRefFailure` classification.

## Phase 3 - Add Scoped Interpolation Errors And Cause-Composed Rendering

Goal: make the reference failure render as the real cause while preserving typed scope for the frontmatter key and expression.

- [ ] Add `MarkdownError::Interpolation { key, expression, source, cause }` with `#[source]` on the `ExpressionError`.
- [ ] Add `SourceRef::OnDisk(SourceContext)` and wire compose-time interpolation failures to it.
- [ ] Change frontmatter key scoping to set `key: Some(...)` instead of prepending prose to the error message.
- [ ] Change body interpolation failures to use `key: None`.
- [ ] Update Darkmatter block rendering so interpolation wrappers compose scope from `MarkdownError` with headline and hint from the underlying cause.
- [ ] Ensure mechanism-first headlines like `transform failed` no longer shadow typed interpolation causes.
- [ ] Add render tests for the reference invalid-file failure in `md compose`.
- [ ] Add Claudine render-path coverage proving `claudine compose` reaches the same deepest Darkmatter block.
- [ ] Validation checkpoint: the reference failure headline is cause-driven in both CLIs, while Phase 1 fatal/warn behavior remains unchanged.

Parallelizable after the enum shape is in place:

- [ ] Update Darkmatter CLI error walking snapshots.
- [ ] Update Claudine CLI error walker tests for deepest typed cause preservation.

## Phase 4 - Add File Suggestions And Shared Path Linking

Goal: make file-reference diagnostics actionable without computing expensive help during evaluation.

- [ ] Add `suggest_strings(candidates, key, max)` beside the existing catalog suggestion code, reusing the same quality gate.
- [ ] Implement lazy sibling candidate collection at render time for `FileRefFailure::NotFound`.
- [ ] Cap directory reads to a bounded number of entries and keep candidate search non-recursive.
- [ ] If the direct parent directory is missing, search from the nearest existing ancestor.
- [ ] Start with leaf-name matching and add calibration tests for dated directories and near names such as `spec.md` vs `specs.md`.
- [ ] Extend file-reference rendering to include did-you-mean suggestions when candidate quality passes the threshold.
- [ ] Add shared path field rendering that applies OSC8 links when the terminal supports them and plain paths otherwise.
- [ ] Replace new call sites with shared path rendering rather than adding manual link formatting.
- [ ] Validation checkpoint: invalid file references include bounded, relevant suggestions and OSC8-linked prompt/file paths in capable terminals, with ANSI-free non-TTY output.

Parallelizable:

- [ ] Implement and test `suggest_strings` independently of terminal rendering.
- [ ] Implement path-link rendering tests independently of filesystem candidate search.

## Phase 5 - Implement Focused YAML Excerpts

Goal: show only the involved frontmatter shape, including structural parents such as `$schema`, instead of no YAML or the entire block.

- [ ] Add `YamlKeyPath` or the equivalent key-path representation needed by `SourceContext::focused_yaml_excerpt`.
- [ ] Implement indentation-aware lookup for frontmatter key paths, reusing existing property-location behavior where possible.
- [ ] Union target key ranges with required structural ancestor ranges.
- [ ] Render non-contiguous YAML ranges with line numbers, syntax highlighting, and elision markers between separated regions.
- [ ] Include the receiving interpolation key and referenced frontmatter keys in the focused key set for file-reference failures.
- [ ] Fall back to existing contiguous or whole-block excerpts when aliases, complex sequences, or uncertain ranges prevent safe slicing.
- [ ] Add tests for `$schema` parent inclusion, sibling exclusion, elision, missing-key fallback, and line-number stability.
- [ ] Validation checkpoint: the reference failure excerpt shows `$schema`, `spec`, and `iteration` without dumping unrelated frontmatter.

Parallelizable after the key-path API is defined:

- [ ] Build parser/range unit tests for focused key lookup.
- [ ] Build terminal rendering snapshots for contiguous, non-contiguous, and fallback excerpts.

## Phase 6 - Clean Cross-Crate Error Transport And Add Boundary Lints

Goal: prevent typed Darkmatter and BlockError causes from collapsing back to strings at the Claudine boundary.

- [ ] Audit Claudine and Darkmatter boundary sites for `.to_string()`, `format!("{e}")`, and `Variant(String)` patterns that carry lower-layer errors.
- [ ] Convert `resolve.rs` invalid reference and markdown load variants to preserve raw input/path plus typed sources.
- [ ] Convert `sequence.rs` external-load failures to structured variants with typed sources.
- [ ] Convert `closure.rs` atomic-write failures to include path and typed source.
- [ ] Convert `lifecycle_control.rs` string-mapped errors to propagate typed errors where possible.
- [ ] Add a grep-based review guard or test that flags new string-only lower-layer error variants.
- [ ] Add a guard for `map_err(|e| e.to_string())` at Darkmatter-to-Claudine and BlockError transport boundaries.
- [ ] Document any intentional exceptions in the guard allowlist with narrow patterns and reasons.
- [ ] Validation checkpoint: the boundary lint passes and the reference typed cause survives from Darkmatter through Claudine rendering.

Parallelizable:

- [ ] Perform the transport audit while Phase 4 and Phase 5 rendering work proceeds.
- [ ] Build the lint guard independently, then tighten it after known transport sites are converted.

## Phase 7 - Implement Diagnostic Facets And `err.*` Projection

Goal: expose stable, handleable error classification from the same typed causes used for rendering.

- [ ] Add the `Diagnostic: BlockError` trait with `category`, `code`, `disposition`, `origin`, `detail`, and `severity`.
- [ ] Implement the ratified facet enums from `error-catalog.md`: 12 categories, 5 dispositions, 5 origins, and 3 severities.
- [ ] Add a single-source code registry for the locked dotted codes and additive-only metadata.
- [ ] Implement `Diagnostic` for typed composition errors, including `composition.invalid_file_reference`, `composition.unknown_function`, and `composition.expression_invalid`.
- [ ] Project `FileReferenceDiagnostic` through serde-compatible detail fields: `reference`, `kind`, `base_dir`, and `suggestions`.
- [ ] Fold existing stream and badge taxonomies into the new facets without removing migration-compatible behavior prematurely.
- [ ] Surface cap timing fields, including `reset_at` and `retry_after_ms`, through diagnostic detail for throttled errors.
- [ ] Extend lifecycle late-binding `err.*` with `category`, `code`, `disposition`, `origin`, `detail.*`, `severity`, and promoted convenience fields.
- [ ] Preserve legacy `err.kind`, `err.variant`, and `err.msg`; treat `kind` and `variant` as deprecated aliases for `category` and `code`.
- [ ] Add `claudine errors` or the agreed introspection surface listing codes and detail schemas from the registry.
- [ ] Add tests proving handlers can match by pattern, code, and instance detail without parsing human messages.
- [ ] Validation checkpoint: every handleable error in scope exposes the ratified facets, and docs/examples use the new faceted names.

Parallelizable after the trait and enum shapes land:

- [ ] Implement composition diagnostic facets.
- [ ] Implement provider/cap/timeout diagnostic facets.
- [ ] Implement lifecycle `err.*` projection tests.
- [ ] Implement the CLI introspection report.

## Phase 8 - Converge Excerpt Paths And Close Late-Binding Corners

Goal: finish the hard corners after the typed substrate and render/handle contracts are stable.

- [ ] Add `SourceRef::Effective { rendered, origin_key }` for DM2 late-binding lifecycle evaluation failures.
- [ ] Render effective-source failures with the resolved value or expression and origin key, without fabricating disk line numbers.
- [ ] Ensure strict DM2 lifecycle evaluation failures halt the run once and do not recursively re-enter `finalize`.
- [ ] Migrate Claudine `FrontmatterExcerpt` and `WithFrontmatter` rendering onto Darkmatter `SourceContext::focused_yaml_excerpt`.
- [ ] Preserve Claudine's current TTY gating, `NO_COLOR`, `FORCE_COLOR=1`, and non-TTY ANSI stripping behavior during excerpt convergence.
- [ ] Remove duplicated excerpt rendering only after snapshot parity proves the shared path covers existing cases.
- [ ] Add terminal snapshots for TTY color, forced color, no-color, and non-TTY outputs.
- [ ] Add late-binding lifecycle tests for unknown roots, malformed spans, known-null references, and effective-source rendering.
- [ ] Validation checkpoint: compose-time and event-time interpolation errors both render and classify through the same typed chain without string parsing.

Parallelizable:

- [ ] Build `SourceRef::Effective` lifecycle tests while excerpt convergence snapshots are prepared.
- [ ] Compare old Claudine excerpt snapshots against the shared Darkmatter renderer before removing old code.

## Final Acceptance Checklist

- [ ] The reference invalid-file failure renders with a root-cause headline in both `md compose` and `claudine compose`.
- [ ] The report names the receiving frontmatter key and links the prompt file when OSC8 is supported.
- [ ] The focused excerpt contains `$schema`, `spec`, and `iteration` without unrelated frontmatter.
- [ ] Did-you-mean suggestions appear for likely filesystem typos and are bounded.
- [ ] Fatal-vs-warn behavior remains provably unchanged through the typing refactor.
- [ ] `absolute()`, `relative()`, and `load_markdown()` failures use the same `FileReferenceDiagnostic` path.
- [ ] No new string-only lower-layer error variants cross the Darkmatter-to-Claudine boundary.
- [ ] Every in-scope handleable error exposes `category`, `code`, `disposition`, `origin`, `severity`, and `detail`.
- [ ] Lifecycle `err.*` supports the new faceted fields while keeping deprecated compatibility aliases.
- [ ] New user-facing terminal output uses `TerminalRenderable`/`BlockError`/`StatusBlock` paths and preserves TTY/color behavior.
