---
ready: false
agent: "codex/default"
created: "2026-06-29T01:59:15"
implemented: true
---

# Review 2

## Findings

### High - The reference diagnostic still cannot render the focused prompt context or OSC8-linked source file

The live interpolation path now preserves `MarkdownError::Interpolation`, and the file-reference block now emits a cause-driven `invalid file path` headline with optional sibling suggestions. That closes the first review's most basic `Transform(String)` blocker. However, the spec's user-facing acceptance criterion is broader: the report must name the receiving key, link the prompt file, show a focused `$schema` / `spec` / `iteration` excerpt, and verify the terminal rendering in both `md compose` and `claudine compose`.

That is still not wired. `MarkdownError::Interpolation` carries `source: Box<SourceRef>` at `darkmatter/lib/src/markdown/types.rs:107`, but the render walk discards it at `darkmatter/lib/src/markdown/types.rs:275` and calls `blocks::interpolation_block(key, expression, cause)` with no source context. The block itself only receives `key`, `expression`, and `cause` at `darkmatter/lib/src/markdown/errors/blocks.rs:258`, so it has no way to render an OSC8 prompt-file link or any focused YAML excerpt. I also found no construction site for `SourceRef::OnDisk` or call site for a focused excerpt in the interpolation renderer, so the current implementation cannot meet the reference report shape.

Verification level mismatch: this is user-observable terminal output. The current coverage I found is an in-process unit test for `interpolation_block_file_reference_offers_did_you_mean`; I found no Level 2 `md compose` / `claudine compose` capture for the target invalid-file-reference report. The excerpt, SGR styling, and OSC8 link requirement need Level 2 real-terminal capture before this can be called ready.

### High - The fatal/warn characterization gate was rewritten to a behavior change the spec explicitly says to preserve

The spec and integrated design require the typing refactor to preserve fatal-vs-warn behavior first, then make any missing-file promotion as a separate product decision. `spec.md` recommends preserving current behavior for this implementation, and `integrated-design.md` says the matrix must lock that today only unknown functions are fatal in lenient body interpolation while missing file references remain warnings.

The implementation now does the opposite. `ExpressionError::is_authoring_fatal()` still documents that "every other variant - including a missing file reference - is demoted to a ComposeWarning" at `darkmatter/lib/src/markdown/compose/expression/error.rs:236`, but the match returns `true` for `FileRefFailure::Malformed`, `NotFound`, and `FoundElsewhere` at `darkmatter/lib/src/markdown/compose/expression/error.rs:256`. The characterization matrix has also been rewritten so lenient body `missing-file` and `malformed-path` are expected fatal at `darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs:201`, with tests named `body_missing_file_is_fatal_in_lenient_mode` and `body_malformed_path_is_fatal_in_lenient_mode` at `darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs:260`.

Impact: this removes the design's behavior-neutral correctness gate and turns a documented open product decision into a side effect of the refactor. If the product decision is to promote file references to fatal, that needs to be reflected in the spec/design and covered as an intentional behavior-change phase. As written, this is a spec violation and a migration-risk blind spot.

DECISION: file references should be fatal; spec should be changed to reflect that.

### High - Handleability is only partially wired; several required `err.*` details are missing or unavailable

`CompositionError` now implements `Diagnostic`, and lifecycle `err.*` can project facets for composition errors. That is progress. The production contract, though, says every handleable error exposes `category` / `code` / `disposition` / `origin` / `detail`, with lifecycle `err.*` using those fields and legacy aliases retained.

The implementation still leaves major holes. `LifecycleErrorInfo::from_claudine_error()` and `from_harness_error()` explicitly set `facets: None` at `claudine/lib/src/composition/lifecycle_context.rs:115` and `claudine/lib/src/composition/lifecycle_context.rs:128`, so provider, cap, timeout, runaway, harness, and top-level Claudine errors do not expose the required handleability surface in lifecycle handlers. `claudine/lib/src/diagnostics/mod.rs:24` also still says concrete implementations for composition, provider, cap, and lifecycle projection are "wired in a later step", which is no longer acceptable for a production-ready feature.

Even for the implemented file-reference case, the detail payload does not match the ratified catalog. The registry declares `composition.invalid_file_reference` detail fields as `reference`, `kind`, `base_dir`, and `suggestions` at `claudine/lib/src/diagnostics/registry.rs:166`, but `CompositionError::detail()` emits only `reference`, `kind`, and `base_dir` at `claudine/lib/src/composition/error.rs:2768`. It omits `suggestions`, omits `fallback_dir`, and serializes `kind` with Rust `Debug` (`"NotFound"`) instead of the catalog's snake_case example (`"not_found"`). A handler cannot reliably write `err.detail.suggestions` even though the rendering code computes suggestions separately.

Verification level: this is mostly Level 1/API behavior, plus lifecycle integration tests that execute `when: err.code == ...` and `err.detail.*` in a stack. I found no tests covering `err.detail.suggestions`, snake_case `kind`, or facet projection for non-composition failures.

### Medium - The schema parse fix is typed, but it still lacks the focused excerpt/highlight promised for frontmatter-rooted errors

The review-1 schema collapse appears substantially improved: `translate_schema_failure` now downcasts the typed `SchemaError` and maps grammar/convert/shape errors to `CompositionError::SchemaParse`, while retaining `SchemaLoad` for reference-resolution failures. The render block now gives syntax-oriented remediation for `SchemaParse`.

The remaining gap is the same terminal-context issue as the reference failure. `SchemaParse` carries a `span`, but the render block drops it at `claudine/lib/src/composition/error.rs:2208` and explicitly says the span feeds the focused excerpt while not actually rendering one. The body at `claudine/lib/src/composition/error.rs:2233` names the file/property and message, but it does not highlight the offending frontmatter line. This leaves a frontmatter-rooted syntax failure without the line-focused context expected by the real-errors design.

Verification level: Level 1 tests check classification to `SchemaParse`, but the excerpt/highlight and OSC8 path behavior need Level 2 capture.

## Summary

This iteration made real progress: the interpolation error type exists, the basic root-cause headline is reachable, schema grammar errors are no longer collapsed into path remediation, and `CompositionError` has an initial diagnostic-facet implementation.

The feature is still not production-ready. The target terminal report cannot yet render the focused prompt context or OSC8 source link, the fatality matrix now encodes an undocumented behavior change that contradicts the spec's preservation gate, and the handleability contract remains incomplete for lifecycle `err.*` and file-reference detail payloads.

Production ready: **no**.
