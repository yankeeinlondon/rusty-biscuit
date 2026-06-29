---
ready: false
agent: "codex/default"
created: "2026-06-28T20:11:13"
---

# Review 1

## Findings

### High - Reference failure still renders through `MarkdownError::Transform`, so the core user-visible requirement is not implemented

The spec requires the invalid file reference to render with a root-cause headline, the receiving frontmatter key as structured scope, a focused excerpt, prompt-file link, and suggestions in both `md compose` and `claudine compose`. The implementation adds `ExpressionError` and `FileReferenceDiagnostic`, but the live interpolation error path still flattens typed evaluation errors into `MarkdownError::Transform(format!(...))` at `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs:109`, and frontmatter key scoping still prepends prose at `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs:216`.

`MarkdownError` also has no `Interpolation { key, expression, source, cause }` variant; `Transform(String)` remains the rendered variant at `darkmatter/lib/src/markdown/types.rs:72`. The feature plan confirms the same state: Phase 3 tasks for the interpolation variant, cause-composed rendering, `md compose` render tests, and `claudine compose` render tests are unchecked/blocked at `claudine/features/2026-06-28-real-errors/plan.md:188`.

Impact: the example that motivated the feature can still surface as a mechanism-first transform failure, and the typed file-reference cause cannot drive the requested report. This blocks production readiness.

Verification level mismatch: the user-observable terminal report requirement currently has no completed end-to-end render verification. Text/report content can start at Level 1 CLI snapshot coverage, but OSC8/color/TTY behavior and focused terminal rendering need Level 2 capture. The plan explicitly marks both render tests blocked at `plan.md:194`.

### High - The handleability axis is scaffolded but not wired to concrete errors or lifecycle `err.*`

The spec requires every handleable error to expose `category`, `code`, `disposition`, `origin`, `detail`, and for lifecycle handlers to consume those through `err.*`, while keeping legacy aliases. The new `claudine::diagnostics::Diagnostic` trait exists, but its own status note says concrete implementations and lifecycle projection are deferred at `claudine/lib/src/diagnostics/mod.rs:24`.

The live lifecycle global still exposes only `err.kind`, `err.variant`, and `err.msg` from `LifecycleErrorInfo::to_value()` at `claudine/lib/src/composition/lifecycle_context.rs:113`, and injects exactly that object at `lifecycle_context.rs:293`. The plan marks implementation of `Diagnostic` for typed composition errors, `FileReferenceDiagnostic` detail projection, cap timing detail, lifecycle `err.*` facets, and deprecated alias wiring as blocked at `claudine/features/2026-06-28-real-errors/plan.md:427`.

Impact: prompt authors cannot write the required `when: err.code == ...` or `err.detail.*` handlers without string parsing. This misses one of the feature's two primary axes.

Verification level: this is mostly Level 1/API behavior and needs focused unit/integration tests over lifecycle expression evaluation. Those tests are absent because the projection is not implemented.

### High - The cross-crate transport guard permits known string-collapsing boundaries, so the "no string-only lower-layer errors" criterion is not met

The new guard runs and passes, but it passes because several known typed-error collapses are allowlisted. The allowlist documents unresolved conversions at `scripts/check-error-transport.allow:15`, including `InvalidReference(String)`, `MarkdownLoad(String)`, `SequenceExternalLoad(String)`, and `AtomicWriteFailed(String)`. The actual string-only variants are still present in `claudine/lib/src/composition/error.rs:34`, `:46`, `:204`, and `:814`, with call sites such as `claudine/lib/src/composition/resolve.rs:43` and `claudine/lib/src/composition/sequence.rs:106`.

Impact: the lint is useful as a regression guard, but it does not prove the success criterion. Existing lossy boundaries remain and can still prevent Darkmatter/Claudine from preserving typed causes across crate boundaries.

Verification: I ran `just -f claudine/justfile lint-transport`; it passed, but this only verifies no new unallowlisted collapses.

### High - Shared file-reference diagnostics do not yet cover all required filesystem functions

`absolute()` and `relative()` now use `FileReferenceDiagnostic` through `resolve_arg`, and `frontmatter()` reaches it through `load_markdown`. However, `load_markdown` still converts unreadable local files and Markdown parse errors into `ExpressionError::Other` string messages at `darkmatter/lib/src/markdown/compose/expression/functions.rs:1615`, rather than preserving a shared file-reference diagnostic or a typed IO/parse cause. The plan also marks the final acceptance item for `absolute()` / `relative()` / `load_markdown()` shared diagnostic coverage unchecked at `claudine/features/2026-06-28-real-errors/plan.md:524`.

Impact: the win does not fully generalize across the required `load_markdown` family, and detail fields for `composition.invalid_file_reference` cannot be projected consistently.

### Medium - Focused excerpts and suggestions are implemented as utilities but not connected to the rendered diagnostic

`SourceContext::focused_yaml_excerpt` and sibling file suggestion helpers exist, but the file-reference render block that would call them is not present. The plan states suggestions are produced but not rendered because there is no `FileReferenceDiagnostic` render block at `claudine/features/2026-06-28-real-errors/plan.md:223`, and the final checklist still has focused excerpt and did-you-mean unchecked at `plan.md:519`.

Impact: users do not get the focused `$schema` / `spec` / `iteration` excerpt or bounded suggestions promised by the spec, even though some building blocks are available.

Verification level mismatch: focused terminal rendering and OSC8 behavior should get Level 2 coverage because they depend on the terminal render path, not just string construction. The relevant tests are still blocked.

### High - `$schema` grammar/convert errors collapse into `SchemaLoad` and render path-resolution remediation for a syntax mistake

This is the same defect class the feature exists to eliminate (typed cause discarded → wrong-category, mechanism-first message), in a code path neither the plan nor the other findings cover. A bad SimplifiedSchema constraint separator (e.g. `file(required, match(...))`, where `,` separates *args* but constraints are separated by `;`) makes Darkmatter produce a fully-typed `SchemaError::Grammar { property, message, span }` at `darkmatter/lib/src/markdown/schemas/errors.rs` and preserve it on `MarkdownError::SchemaValidationFailed { source: Some(...), problems: [], summary }` at `darkmatter/lib/src/markdown/compose/schema_validation.rs:98` and `:156`.

Claudine then throws the typed cause away. `translate_schema_failure` discriminates on `problems.is_empty()` alone at `claudine/lib/src/composition/schema_validation.rs:203` and funnels *every* empty-problems case — genuine reference-load failures **and** grammar/convert/shape errors — into `CompositionError::SchemaLoad { message: String }`; the `SchemaValidationFailed` destructure at `schema_validation.rs:186` drops `source` via `..`. `SchemaLoad` carries only `message: String` (no typed `#[source]`) at `claudine/lib/src/composition/error.rs:991`, and its render block hardcodes reference-resolution remediation ("Verify the `$schema` path is correct… Remote `http://` references are not supported") at `error.rs:2151`, which is wrong and misleading for a syntax error in an inline schema body.

Impact: the motivating user experience (a precise, root-cause-first report) is not delivered for the entire schema surface. A `spec: file(required, match(**/*spec*.md))` typo renders as "schema load failed / verify the path" instead of "invalid schema constraint syntax for `spec`: `;` separates constraints, `,` separates args", with no focused excerpt highlighting the offending line — even though `FrontmatterExcerpt` enrichment and the typed `span` are both already available.

Note on the transport guard (finding #3): this collapse is invisible to `scripts/check-error-transport.sh` and is not on `scripts/check-error-transport.allow`. The type is lost one layer up inside Darkmatter; at the claudine call site `SchemaLoad { message: summary }` is built from an already-`String` `summary`, so the heuristic sees no typed→String collapse to flag. Clearing finding #3 will not surface or fix this.

Required: capture the typed `SchemaError` from `SchemaValidationFailed.source`, branch `Grammar | Convert | FrontmatterShape` to a new typed variant (e.g. `CompositionError::SchemaParse { source_path, property, message, span }`) with constraint-syntax remediation and the property as structured scope, reuse `FrontmatterExcerpt` to highlight the offending line, and keep `SchemaLoad` only for `Unresolved | RemoteUnsupported | AmbiguousReferenced | Io`.

Verification level: Level 1 CLI snapshot for the headline/remediation/scope text; Level 2 capture for the focused-excerpt highlight and OSC8 prompt-file link, consistent with finding #5.

## Summary

This is a partial implementation. The fatality characterization matrix and some typed substrate work are valuable, but the feature's production contract is not met: the root-cause renderer is not wired, typed errors still collapse at known boundaries (the interpolation path *and* the `$schema` grammar/convert path), `Diagnostic` facets are not implemented for concrete errors, and lifecycle `err.*` still exposes only the legacy fields.

Production ready: **no**.
