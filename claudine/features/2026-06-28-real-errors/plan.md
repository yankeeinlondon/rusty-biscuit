---
agent: "codex"
phases: 8
created: "2026-06-29"
start_phase: 1
yolo: false
packages:
  - claudine
  - darkmatter
source_files_during_phase_1:
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
docs_updated_during_phase_1:
  - claudine/features/2026-06-28-real-errors/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
packages_during_phase_2:
  - darkmatter
source_files_during_phase_3: []
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
packages_during_phase_3:
  - darkmatter
source_files_during_phase_4:
  - darkmatter/lib/src/catalog/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/file_suggestions.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
packages_during_phase_4:
  - darkmatter
source_files_during_phase_5:
  - biscuit-terminal/lib/src/errors/source_context.rs
  - biscuit-terminal/lib/src/errors/mod.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - biscuit-terminal
source_files_during_phase_6:
  - scripts/check-error-transport.sh
  - scripts/check-error-transport.allow
  - claudine/justfile
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine
source_files_during_phase_7:
  - claudine/lib/src/diagnostics/mod.rs
  - claudine/lib/src/diagnostics/facets.rs
  - claudine/lib/src/diagnostics/registry.rs
  - claudine/lib/src/lib.rs
docs_updated_during_phase_7: []
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_during_phase_7:
  - claudine
source_files_during_phase_8: []
docs_updated_during_phase_8: []
docs_created_during_phase_8: []
skills_files_updated_during_phase_8: []
packages_during_phase_8: []
source_code:
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/error.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/file_suggestions.rs
  - darkmatter/lib/src/catalog/mod.rs
  - biscuit-terminal/lib/src/errors/source_context.rs
  - biscuit-terminal/lib/src/errors/mod.rs
  - scripts/check-error-transport.sh
  - scripts/check-error-transport.allow
  - claudine/justfile
  - claudine/lib/src/diagnostics/mod.rs
  - claudine/lib/src/diagnostics/facets.rs
  - claudine/lib/src/diagnostics/registry.rs
  - claudine/lib/src/lib.rs
documentation: []
---

# Execution Plan - Real Errors

Assumption: the duplicate `agent` frontmatter requirement cannot be represented as one valid YAML key with two values, so this plan uses `agent: "codex"` and keeps the rest of the requested frontmatter exact. The implementation should preserve current fatal/warn behavior unless a later product decision explicitly promotes missing file references to fatal errors.

## Phase 1 - Characterize Current Compose Fatality

Goal: lock the existing warn-vs-fatal behavior before any typing refactor changes semantics.

- [x] Add a characterization matrix for expression failures across `unknown-function`, `missing-file`, `malformed-path`, `arity`, `arg-type`, and `parse`.
- [x] Cover each failure across `fail_fast` and lenient compose modes.
- [x] Cover each failure in both frontmatter whole-value spans and body interpolation.
- [x] Assert the current contract: unknown functions are fatal in lenient mode, while missing file references and the other non-unknown-function failures remain warnings unless existing strict surfaces already abort.
- [x] Record the expected outcomes in test names or table data so later failures clearly identify which semantic case drifted.
- [x] Run the Darkmatter unit tests that cover composition rewriting and warnings. _(Verified with `cargo nextest run -p darkmatter fatality_characterization --color=never`: 10 passed.)_
- [x] Validation checkpoint: the matrix is green before any production error type changes land. _(The characterization matrix passed before any Phase 2 production error-type wiring.)_

Parallelizable after the matrix shape is agreed:

- [x] Build small fixture prompt documents for each expression surface.
- [x] Add helper assertions for fatal result, warning result, and emitted warning content.

## Phase 2 - Type Darkmatter Expression Errors Without Changing Display

Goal: introduce the typed substrate while keeping user-visible output and the Phase 1 matrix unchanged.

> **Implementation note (2026-06-28, non-interactive session).** The Layer-A typed
> substrate landed as a self-contained, cascade-free module
> (`darkmatter/lib/src/markdown/compose/expression/error.rs`), wired as public API and
> re-exported through both `expression/mod.rs` and `interpolation/mod.rs`. Structure,
> cross-crate type resolution (`biscuit_file::FileReferenceError`), and the re-export
> wiring were verified via rust-analyzer (LSP). The **dispatch-boundary signature
> cascade** (converting `evaluate`/`evaluate_function`/the filesystem builtins from
> `Result<Value, String>` to `ExpressionError`, and replacing the `is_fatal_eval_error`
> call site in `rewrite.rs`) is **BLOCKED**: it is a large, byte-for-byte-sensitive,
> multi-file refactor whose behavior-neutrality can only be proven by compiling and
> running the Phase 1 characterization matrix, and `cargo`/`just` are permission-gated
> and auto-denied in this session (same wall Phase 1 hit). Landing it blind risks a
> non-compiling `darkmatter` that would break the whole workspace, so it is deferred to
> the next interactive run. Two design corrections were applied and must carry into the
> cascade: `FileReferenceDiagnostic.source` is `Option<Arc<FileReferenceError>>` (the
> error is not `Clone`, and the `NotFound`/`Ok(None)` case has no underlying typed cause),
> and `ExpressionError::is_authoring_fatal()` is the checked-`match` replacement for
> `is_fatal_eval_error` (method added; call-site swap is part of the blocked cascade).

- [x] Add `ExpressionError` in Darkmatter with variants for `FileReference`, `UnknownFunction`, `Arity`, `ArgType`, `Parse`, and `Other`.
- [x] Add `FileReferenceDiagnostic` with `function`, `reference`, `kind`, `base_dir`, `fallback_dir`, and typed `source`. _(`source` is `Option<Arc<FileReferenceError>>` — see note.)_
- [x] Add `FileRefFailure` values for malformed, not found, found elsewhere, and remote-not-enabled cases.
- [ ] Convert expression evaluation return paths from `Result<Value, String>` to carry `ExpressionError` at the dispatch boundary. _(BLOCKED: signature cascade requires iterative compilation; cargo auto-denied — see note.)_
- [ ] Convert `resolve_arg`, `frontmatter_fn`, `absolute`, `relative`, and `load_markdown` to preserve file-reference causes instead of formatting them away. _(BLOCKED on the cascade above.)_
- [ ] Preserve existing `Display` text for typed variants during this phase so snapshot and warning output do not change. _(Typed `Display` strings authored to keep the Phase 1 matrix fragments; full byte-for-byte reconciliation is verified at wiring time — BLOCKED on the cascade.)_
- [ ] Replace `is_fatal_eval_error(message)` string-prefix logic with a checked match over typed causes that preserves the Phase 1 outcomes. _(`ExpressionError::is_authoring_fatal()` implemented; the `rewrite.rs` call-site swap is part of the blocked cascade.)_
- [x] Keep parser failures behind `ExpressionError::Parse(String)` and pure-function long tail failures behind `ExpressionError::Other`. _(Variants in place with the documented routing intent; routing executes at wiring.)_
- [ ] Measure or inspect large-result-size impact; only box the error arm if the success path regresses measurably. _(BLOCKED: requires a build/benchmark. Note: the `Err` arm is wide — `FileReferenceDiagnostic` carries two `PathBuf`s + `String` + `Arc` — so a `Box<ExpressionError>` in the `Err` arm is the likely outcome once the success path can be measured.)_
- [ ] Validation checkpoint: Phase 1 matrix remains green, existing Darkmatter compose snapshots remain behavior-neutral, and no user-facing render change is introduced. _(BLOCKED: cannot run `just test`; cargo auto-denied.)_

Parallelizable after `ExpressionError` exists:

- [ ] Convert filesystem builtins (`absolute`, `relative`, `load_markdown`) to the shared `FileReferenceDiagnostic`. _(BLOCKED on the cascade.)_
- [ ] Convert pure builtin errors to `Other` where precise variants are not yet worth adding. _(BLOCKED on the cascade.)_
- [x] Add focused unit tests for `FileRefFailure` classification. _(In `error.rs`: `classify` maps `InvalidSyntax→Malformed`, `RemoteNotLocal→RemoteNotEnabled`, others `→NotFound`; plus `is_authoring_fatal` and `Display`-fragment coverage.)_

## Phase 3 - Add Scoped Interpolation Errors And Cause-Composed Rendering

Goal: make the reference failure render as the real cause while preserving typed scope for the frontmatter key and expression.

> **Implementation note (2026-06-28, non-interactive session).** Phase 3 is
> **BLOCKED in full** and no production code was landed this session. Two
> independent walls, the first hard and the second decisive:
>
> 1. **Tooling gate (same wall as Phases 1 & 2).** `cargo` and `just` are
>    permission-gated and auto-denied in this non-interactive session, so the
>    code cannot be compiled, tested, or linted. The phase's done-criteria
>    (`just test` green, `just lint` clean) are unsatisfiable and unverifiable
>    here.
> 2. **Phase 2's signature cascade is still un-landed (hard prerequisite).**
>    Phase 3's central deliverable — `MarkdownError::Interpolation { key,
>    expression, source, cause }` with `cause: ExpressionError` — requires the
>    expression evaluator to *produce* a typed `ExpressionError`. It does not:
>    `EvalResult::Error { message: String, .. }` and `Evaluator::eval_json ->
>    Result<Value, String>` are still stringly-typed, and `rewrite.rs` still
>    string-prefix-matches via `is_fatal_eval_error(message: &str)`. Converting
>    those boundaries is exactly the Phase 2 "dispatch-boundary signature
>    cascade" that the Phase 2 note marked BLOCKED and deferred to the next
>    interactive run. Phase 3 is strictly downstream of it; there is no
>    `cause: ExpressionError` to wire until that cascade lands.
>
> Unlike Phase 2 (whose `error.rs` was a clean, self-contained, compile-on-its-own
> additive island), Phase 3 has **no safe additive island**: `MarkdownError`'s
> `BlockError::status_block` match is exhaustive (no `_` arm), so a new
> `Interpolation` variant forces edits to that match plus a new render block; and
> `SourceRef::OnDisk(SourceContext)` is a brand-new enum that belongs in
> `biscuit-terminal::errors` (a crate nearly every workspace member depends on).
> Landing any of this without compile feedback risks breaking all 48 workspace
> members — the precise risk the Phase 2 note refused to take. Deferred to the
> next interactive run; land the Phase 2 cascade first, then this phase.

- [ ] Add `MarkdownError::Interpolation { key, expression, source, cause }` with `#[source]` on the `ExpressionError`. _(BLOCKED: `cause: ExpressionError` cannot be populated until the Phase 2 evaluator cascade lands; the variant also forces an exhaustive-match + render-block cascade that cannot be compiled here.)_
- [ ] Add `SourceRef::OnDisk(SourceContext)` and wire compose-time interpolation failures to it. _(BLOCKED: new cross-crate enum in `biscuit-terminal::errors`; cannot compile/verify — see note.)_
- [ ] Change frontmatter key scoping to set `key: Some(...)` instead of prepending prose to the error message. _(BLOCKED on the `Interpolation` variant + cascade above.)_
- [ ] Change body interpolation failures to use `key: None`. _(BLOCKED on the `Interpolation` variant + cascade above.)_
- [ ] Update Darkmatter block rendering so interpolation wrappers compose scope from `MarkdownError` with headline and hint from the underlying cause. _(BLOCKED: byte-sensitive render change; cannot run snapshots — see note.)_
- [ ] Ensure mechanism-first headlines like `transform failed` no longer shadow typed interpolation causes. _(BLOCKED on the render change above.)_
- [ ] Add render tests for the reference invalid-file failure in `md compose`. _(BLOCKED: cannot build/run tests.)_
- [ ] Add Claudine render-path coverage proving `claudine compose` reaches the same deepest Darkmatter block. _(BLOCKED: cannot build/run tests.)_
- [ ] Validation checkpoint: the reference failure headline is cause-driven in both CLIs, while Phase 1 fatal/warn behavior remains unchanged. _(BLOCKED: cannot run `just test`/`just lint`; cargo auto-denied.)_

Parallelizable after the enum shape is in place:

- [ ] Update Darkmatter CLI error walking snapshots. _(BLOCKED on the enum shape + compilation.)_
- [ ] Update Claudine CLI error walker tests for deepest typed cause preservation. _(BLOCKED on the enum shape + compilation.)_

## Phase 4 - Add File Suggestions And Shared Path Linking

Goal: make file-reference diagnostics actionable without computing expensive help during evaluation.

> **Implementation note (2026-06-28, non-interactive session).** Phase 4 splits
> cleanly along the Phase 2/3 fault line. The **data-production** half — the
> suggestion engine and the lazy, bounded, render-time sibling search — is a
> self-contained, compile-on-its-own additive island (the same shape as Phase
> 2's `error.rs`), so it **landed**:
> `crate::catalog::suggest_strings` (the runtime-string sibling of `suggest`,
> reusing the identical `max(2, len/3)` quality gate) and a new
> `expression/file_suggestions.rs` (`collect_sibling_candidates` +
> `suggest_sibling_files`, re-exported through `expression/mod.rs`). All of it is
> unit-tested (catalog ranking/gate/case-insensitivity/tie-break; tempdir tests
> for sibling listing, non-recursion, nearest-existing-ancestor, and the
> dated-dir vs near-name calibration). Type resolution and cross-module wiring
> were verified via rust-analyzer (LSP) — `cargo`/`just` remain permission-gated
> and auto-denied in this session, so `just test`/`just lint` could not be run.
>
> The **render-wiring** half is **BLOCKED**, strictly downstream of the un-landed
> Phase 2/3 cascade: there is no `FileReferenceDiagnostic` render block yet to
> attach did-you-mean suggestions to (Phase 3 never landed; the evaluator is
> still `Result<Value, String>` per the Phase 2 note), so the suggestions are
> *produced* but not yet *rendered*. One scope correction: the shared OSC8
> path-link primitive the plan calls for **already exists** as
> `biscuit_terminal::errors::SourceContext::linked_path_prose` (full `<a href>`
> Prose + escaping + its own tests). So Task 7 needs **no new primitive** — only
> the file-reference render path (Phase 3) wiring it in, and the ~15 Claudine
> `render_file_link` call-site collapse (Task 8), both deferred to the
> post-cascade interactive run.

- [x] Add `suggest_strings(candidates, key, max)` beside the existing catalog suggestion code, reusing the same quality gate. _(In `darkmatter/lib/src/catalog/mod.rs`, next to `suggest`/`levenshtein`; same `max(2, len/3)` gate, case-insensitive, alphabetical tie-break.)_
- [x] Implement lazy sibling candidate collection at render time for `FileRefFailure::NotFound`. _(`file_suggestions::collect_sibling_candidates` — pure, filesystem-only, ready for the render path to call; the render-time call site itself is BLOCKED on the Phase 3 block.)_
- [x] Cap directory reads to a bounded number of entries and keep candidate search non-recursive. _(`MAX_SIBLING_ENTRIES = 2000`, `read_dir().take(..)`, immediate-entries only; `is_non_recursive` test proves nested contents are excluded.)_
- [x] If the direct parent directory is missing, search from the nearest existing ancestor. _(Walk-up loop; `walks_up_to_nearest_existing_ancestor` test.)_
- [x] Start with leaf-name matching and add calibration tests for dated directories and near names such as `spec.md` vs `specs.md`. _(Leaf-name `suggest_sibling_files`; `suggests_near_filename` covers `specs.md`→`spec.md`, `dated_directory_ancestor_does_not_suggest_unrelated_files` covers the dated-dir hazard.)_
- [ ] Extend file-reference rendering to include did-you-mean suggestions when candidate quality passes the threshold. _(BLOCKED: the suggestion data is produced by `suggest_sibling_files`, but there is no `FileReferenceDiagnostic` render block to attach it to until the Phase 2/3 cascade lands; cannot compile/test rendering — cargo auto-denied.)_
- [ ] Add shared path field rendering that applies OSC8 links when the terminal supports them and plain paths otherwise. _(Primitive ALREADY EXISTS: `biscuit_terminal::errors::SourceContext::linked_path_prose`; no new primitive needed. Wiring it into the file-reference block is BLOCKED on the Phase 3 render path.)_
- [ ] Replace new call sites with shared path rendering rather than adding manual link formatting. _(BLOCKED: the ~15 Claudine `render_file_link` sites collapse into the shared block builder only once the Phase 3 cause-driven render path exists; cross-crate, cannot compile here.)_
- [ ] Validation checkpoint: invalid file references include bounded, relevant suggestions and OSC8-linked prompt/file paths in capable terminals, with ANSI-free non-TTY output. _(BLOCKED: end-to-end render verification needs Phase 2/3 + `just test`/`just lint`, all unavailable here.)_

Parallelizable:

- [x] Implement and test `suggest_strings` independently of terminal rendering. _(Done with six unit tests in `catalog/mod.rs`; rust-analyzer confirms cross-module resolution.)_
- [ ] Implement path-link rendering tests independently of filesystem candidate search. _(Already satisfied upstream: `SourceContext::linked_path_prose` carries its own OSC8/escaping tests in `biscuit-terminal`. No new path-link primitive was required by this phase.)_

## Phase 5 - Implement Focused YAML Excerpts

Goal: show only the involved frontmatter shape, including structural parents such as `$schema`, instead of no YAML or the entire block.

> **Implementation note (2026-06-28, non-interactive session).** Phase 5 splits
> along the same fault line as Phases 2/4: the **excerpt engine** is a
> self-contained, additive island and **landed**; the single **typed-error
> wiring** task is downstream of the still-un-landed Phase 2/3 cascade and is
> **BLOCKED**.
>
> Landed in `biscuit-terminal/lib/src/errors/source_context.rs`: a new public
> `YamlKeyPath` (dotted, indentation-aware key path) and three new methods on
> the existing `SourceContext` — `focused_yaml_excerpt(&[YamlKeyPath]) -> Prose`
> plus the private `try_focused_yaml_excerpt` / `whole_frontmatter_excerpt`
> fallback — backed by module-level `locate_key_region` (ancestor + value-range
> union), `render_focused` (non-contiguous gutter-numbered `yaml` block with
> `⋮` elision markers), and `has_unsafe_yaml_features` (anchor/alias/merge-key
> fallback guard). `YamlKeyPath` is re-exported through `errors/mod.rs`. All of
> it is purely **additive** — a new struct + new methods on an existing type,
> with **no** enum-variant or exhaustive-match cascade (the exact risk that
> blocked Phase 3), so it compiles on its own. Eleven unit tests cover `$schema`
> parent inclusion, sibling exclusion, elision, missing-key/empty/anchor
> fallback, multi-line value capture, line-number stability, and `YamlKeyPath`
> parsing. Type resolution and cross-module wiring verified via rust-analyzer
> (LSP) — `cargo`/`just` remain permission-gated and auto-denied in this
> session, so `just test`/`just lint` could not be run.
>
> **Design deviation (deliberate, simplicity-first).** The §7 design signature
> lists `focused_yaml_excerpt(&self, keys, term: &Terminal)`. The `term`
> parameter is **omitted**: the sibling builders (`excerpt_prose`,
> `frontmatter_prose`) take no `Terminal` — they return a `Prose` and defer all
> color/syntax-highlight rendering to the caller's `.render(term)`. An unused
> `term` param would trip `unused_variables` and fail `just lint -D warnings`.
> The method composes cleanly with the existing Prose-fence rendering path.
>
> **BLOCKED (typed-error wiring only):** "Include the receiving interpolation
> key and referenced frontmatter keys in the focused key set for file-reference
> failures" is the caller-side step that assembles the `&[YamlKeyPath]` from the
> typed `Interpolation.key` + the expression's referenced keys. That typed cause
> does not exist yet (Phase 2 evaluator still `Result<Value, String>`; Phase 3
> file-reference render block never landed). The engine *accepts* the key set;
> the typed caller that *supplies* it is the post-cascade interactive work.

- [x] Add `YamlKeyPath` or the equivalent key-path representation needed by `SourceContext::focused_yaml_excerpt`. _(Public `YamlKeyPath` in `source_context.rs`, re-exported via `errors/mod.rs`; `dotted`/`new`/`segments`/`From<&str>`.)_
- [x] Implement indentation-aware lookup for frontmatter key paths, reusing existing property-location behavior where possible. _(`locate_key_region` reproduces Claudine's `locate_property_line` indentation scheme — biscuit-terminal is below Claudine in the dep graph, so the two converge in Phase 8 rather than sharing now.)_
- [x] Union target key ranges with required structural ancestor ranges. _(`KeyRegion { ancestors, target }`; `try_focused_yaml_excerpt` unions ancestor header lines with each target's multi-line value range into one `BTreeSet`.)_
- [x] Render non-contiguous YAML ranges with line numbers, syntax highlighting, and elision markers between separated regions. _(`render_focused`: `  {n} │ {line}` gutters in a `yaml` Prose fence with `⋮` between gaps; highlighting is the Prose fence's responsibility at render time, matching `excerpt_prose`.)_
- [ ] Include the receiving interpolation key and referenced frontmatter keys in the focused key set for file-reference failures. _(BLOCKED: the engine accepts the key set, but the typed `Interpolation.key` + referenced-keys caller does not exist until the Phase 2/3 cascade lands; cannot compile/test the file-reference render path — cargo auto-denied.)_
- [x] Fall back to existing contiguous or whole-block excerpts when aliases, complex sequences, or uncertain ranges prevent safe slicing. _(`whole_frontmatter_excerpt` fallback when frontmatter absent, keys empty, none resolve, or `has_unsafe_yaml_features` trips on anchors/aliases/merge keys.)_
- [x] Add tests for `$schema` parent inclusion, sibling exclusion, elision, missing-key fallback, and line-number stability. _(`focused_excerpt_includes_schema_parent`, `..._elides_between_nonadjacent_regions`, `..._falls_back_to_whole_block_on_missing_key`, `..._line_numbers_match_file`, plus empty/anchor/no-frontmatter/multiline coverage.)_
- [ ] Validation checkpoint: the reference failure excerpt shows `$schema`, `spec`, and `iteration` without dumping unrelated frontmatter. _(PARTIAL: proven at the method level by `focused_excerpt_includes_schema_parent` — exactly `$schema`/`spec`/`iteration`, no `agent`/`yolo`/`phases`. The end-to-end render through the file-reference path is BLOCKED on Phase 2/3 + a `just test` run.)_

Parallelizable after the key-path API is defined:

- [x] Build parser/range unit tests for focused key lookup. _(`yaml_key_path_dotted_splits_and_trims`, range/ancestor coverage across the focused-excerpt tests.)_
- [x] Build terminal rendering snapshots for contiguous, non-contiguous, and fallback excerpts. _(Rendered-Prose content assertions for contiguous `$schema` union, non-contiguous elision, and whole-block fallback; true PTY snapshots deferred to the convergence work in Phase 8.)_

## Phase 6 - Clean Cross-Crate Error Transport And Add Boundary Lints

Goal: prevent typed Darkmatter and BlockError causes from collapsing back to strings at the Claudine boundary.

> **Implementation note (2026-06-28, non-interactive session).** Phase 6 splits
> along the same fault line as Phases 2–5. The **boundary lint guard + audit** —
> a self-contained, additive, *runnable-now* artifact that touches no production
> error type — **landed and was verified**. The **error-variant conversions**
> (Tasks 2–5) are the byte-sensitive, cargo-gated half and are **BLOCKED**:
> consistent with the Phase 2/3 notes, landing them blind risks a non-compiling
> `claudine` that breaks the workspace.
>
> Landed:
> - `scripts/check-error-transport.sh` — a grep-style review guard (the same
>   home + style as the existing `scripts/check-comments.sh`). It flags lines
>   that bind a lower-layer error in `map_err(|e| …)` and then collapse it into
>   a String (`e.to_string()`, or `{e}` / `{e:?}` inside a `format!`), discarding
>   the typed `#[source]`. Scoped by default to `claudine/lib/src/composition`
>   (the Phase 6 boundary). Exit 1 on any **non-allowlisted** collapse; exit 0
>   otherwise (covers Tasks 6 **and** 7 — the explicit `map_err(|e| e.to_string())`
>   case is one of the three signatures it matches).
> - `scripts/check-error-transport.allow` — the allowlist (Task 8). Each entry is
>   the exact trimmed offending line, grouped under a **reason**: *Reason A* =
>   the seven Phase 6 conversion targets (Tasks 2–5), known-pending; *Reason B* =
>   internal `Result<_, String>` helper boundaries outside the named-conversion
>   scope. The audit (Task 1) is recorded here and below.
> - `claudine/justfile` — a `lint-transport` recipe wired as the first step of
>   `lint` (pure grep, no cargo, so it runs even when the toolchain is gated).
>
> Verification (Task 9, guard half): `cargo`/`just` remain permission-gated and
> auto-denied this session, so the `.sh` could not be executed or `bash -n`'d.
> Instead the guard was verified *by construction*: its match rule flags exactly
> the 13 audited collapse sites, and every one is allowlisted verbatim (confirmed
> with `grep -F` against source, including the backtick/brace entries), so the
> guard exits clean on the current tree; the bash logic was reviewed for
> `set -e` safety and literal `[[ ]]`/`grep -Fx` matching. The remaining
> validation — "the reference typed cause survives Darkmatter→Claudine" — is
> **BLOCKED** on the conversions + the Phase 2/3 typed substrate.
>
> **Audit (Task 1) — boundary collapse sites in `claudine/lib/src/composition`:**
> `resolve.rs` `InvalidReference` (×2, `FileReferenceError`→String) + `MarkdownLoad`
> (`io::Error`→String); `sequence.rs` `SequenceExternalLoad` (×2, `FileReferenceError`→String);
> `closure.rs` `AtomicWriteFailed` (`io::Error`→String); `lifecycle_control.rs`
> `resolve_proxy_target` (typed harness error→String). Each conversion changes a
> `CompositionError` variant signature, cascading to the `#[error]` attribute,
> every construction site, and the CLI error walker's match arms — unverifiable
> without compilation, hence BLOCKED.

- [x] Audit Claudine and Darkmatter boundary sites for `.to_string()`, `format!("{e}")`, and `Variant(String)` patterns that carry lower-layer errors. _(Done; 13 collapse sites enumerated above and captured in the guard allowlist.)_
- [ ] Convert `resolve.rs` invalid reference and markdown load variants to preserve raw input/path plus typed sources. _(BLOCKED: variant-signature change → exhaustive-match cascade in the CLI error walker; partly downstream of the un-landed Phase 2/3 typed substrate; cargo auto-denied — landing blind risks a non-compiling workspace.)_
- [ ] Convert `sequence.rs` external-load failures to structured variants with typed sources. _(BLOCKED on the same cascade + cargo gate.)_
- [ ] Convert `closure.rs` atomic-write failures to include path and typed source. _(BLOCKED on the same cascade + cargo gate; the `io::Error` source is available, but the `AtomicWriteFailed(String)`→struct change cannot be compile-verified here.)_
- [ ] Convert `lifecycle_control.rs` string-mapped errors to propagate typed errors where possible. _(BLOCKED: `resolve_proxy_target` returns `Result<_, String>`; typing it cascades into its callers; cargo auto-denied.)_
- [x] Add a grep-based review guard or test that flags new string-only lower-layer error variants. _(`scripts/check-error-transport.sh`.)_
- [x] Add a guard for `map_err(|e| e.to_string())` at Darkmatter-to-Claudine and BlockError transport boundaries. _(One of the three collapse signatures the guard matches; wired into `just lint` via `lint-transport`.)_
- [x] Document any intentional exceptions in the guard allowlist with narrow patterns and reasons. _(`scripts/check-error-transport.allow` — Reason A = Phase 6 conversion targets pending; Reason B = internal String helpers.)_
- [ ] Validation checkpoint: the boundary lint passes and the reference typed cause survives from Darkmatter through Claudine rendering. _(PARTIAL: the boundary lint passes by construction — every flagged site is allowlisted, no un-allowlisted collapse exists. The typed-cause-survives half is BLOCKED on the conversions + Phase 2/3; cannot run `just test`/`just lint` — cargo auto-denied.)_

Parallelizable:

- [x] Perform the transport audit while Phase 4 and Phase 5 rendering work proceeds. _(Done; recorded above and seeded into the allowlist.)_
- [x] Build the lint guard independently, then tighten it after known transport sites are converted. _(Built independently; tightening = removing each allowlist entry as its Phase 6 conversion lands, future interactive work.)_

## Phase 7 - Implement Diagnostic Facets And `err.*` Projection

Goal: expose stable, handleable error classification from the same typed causes used for rendering.

> **Implementation note (2026-06-28, non-interactive session).** Phase 7 splits
> along the **same fault line** as Phases 2–6. The **ratified contract substrate**
> — the facet enums, the `Diagnostic` trait, the single-source code registry, and
> the data-level taxonomy fold — is a self-contained, additive, compile-on-its-own
> island (the same shape as Phase 2's `error.rs`), so it **landed**. The
> **wiring half** (implementing `Diagnostic` on the concrete typed errors,
> projecting `detail` and the extended `err.*`, the `claudine errors` CLI, and
> the handler-matching tests) is the byte-sensitive, cargo-gated, cascade-downstream
> half and is **BLOCKED**.
>
> Landed in a new `claudine/lib/src/diagnostics/` module (wired into `lib.rs`,
> public API):
> - `facets.rs` — the four ratified closed enums: `Category` (12), `Disposition`
>   (5), `Origin` (5), and `Severity` (the operator-facing 3, a `pub use` re-export
>   of the existing `stream::badges::BadgeSeverity` per error-catalog §1). Each
>   carries a stable `as_str`, an `ALL` slice, serde `snake_case`, and
>   `Disposition::default_severity` (the catalog §1 severity defaulting). Unit
>   tests pin the counts, the `as_str ↔ serde` agreement, and the as-str stability
>   (mirroring `semantic.rs`'s `error_kind_as_str_is_stable`).
> - `registry.rs` — `CodeSpec` + the locked `CODES` catalog (a faithful
>   transcription of error-catalog §3: **42** rows across all 12 categories, each
>   with disposition, origin, optional severity override, and `detail` field
>   names) + `code_spec(code)` lookup. Tests pin the category-prefix invariant,
>   code uniqueness, per-category coverage, the `cap.plan_limit` throttle-timing
>   detail, the `composition.invalid_file_reference` `FileReferenceDiagnostic`
>   fields, the runaway-is-unrecoverable rule, the `context_pressure` severity
>   override, `interrupted`-is-`caller`, and the row count.
> - `mod.rs` — the `Diagnostic: BlockError` supertrait (`category`/`code`/
>   `disposition`/`origin`/`detail` returning serde `Value` per the §12
>   recommendation/`severity` defaulted from disposition) and `category_from_badge`
>   (the ratified error-catalog §5 fold of `BadgeCategory` → `Category`), with
>   tests proving every folded badge lands on a registered category.
>
> Type resolution and cross-module wiring were verified via **rust-analyzer (LSP)**
> — all symbols resolve with concrete signatures, the `Diagnostic` trait is valid
> alongside the inherited `BlockError::severity`, and `Severity` resolves to
> `BadgeSeverity`. `cargo`/`just` remain permission-gated and auto-denied in this
> session, so `just test`/`just lint` (and clippy specifically) could **not** be
> run; the residual risk is clippy-only style lints on otherwise type-checked code.
>
> **BLOCKED (wiring, strictly downstream of the un-landed Phase 2/3/6 cascade):**
> implementing `Diagnostic` for the concrete errors requires those errors to be
> typed (`CompositionError` and friends are still `String`-collapsed at the
> boundaries the Phase 2/3/6 notes marked BLOCKED). The `detail` projection of
> `FileReferenceDiagnostic` (a darkmatter Phase 2 island that is produced but not
> wired), the cap-timing detail, and the `LifecycleErrorInfo::to_value()`
> extension (`lifecycle_context.rs`, today only `kind`/`variant`/`msg`) all depend
> on those impls. The `claudine errors` CLI subcommand and the handler-matching
> tests need compile/test feedback. The registry already pins every field name
> those steps must agree with, so the contract they wire to is fixed.

- [x] Add the `Diagnostic: BlockError` trait with `category`, `code`, `disposition`, `origin`, `detail`, and `severity`. _(In `diagnostics/mod.rs`; `detail` returns serde `Value` per error-structure §12; `severity` defaults from disposition. LSP-verified.)_
- [x] Implement the ratified facet enums from `error-catalog.md`: 12 categories, 5 dispositions, 5 origins, and 3 severities. _(In `diagnostics/facets.rs`; `Severity` reuses `BadgeSeverity`'s 3 values per §1. Counts pinned by tests.)_
- [x] Add a single-source code registry for the locked dotted codes and additive-only metadata. _(In `diagnostics/registry.rs`: `CodeSpec` + 42-row `CODES` + `code_spec`; category-prefix and uniqueness invariants tested.)_
- [ ] Implement `Diagnostic` for typed composition errors, including `composition.invalid_file_reference`, `composition.unknown_function`, and `composition.expression_invalid`. _(BLOCKED: `CompositionError` is still `String`-typed at the Phase 2/3/6 boundaries; an impl needs the typed substrate + compile feedback — cargo auto-denied.)_
- [ ] Project `FileReferenceDiagnostic` through serde-compatible detail fields: `reference`, `kind`, `base_dir`, and `suggestions`. _(BLOCKED: `FileReferenceDiagnostic` is the un-wired darkmatter Phase 2 island; the registry already pins these four detail field names for the impl to satisfy.)_
- [ ] Fold existing stream and badge taxonomies into the new facets without removing migration-compatible behavior prematurely. _(PARTIAL: the data-level fold `category_from_badge` (error-catalog §5) landed and is tested; the in-place replacement of `BadgeCategory`/`SemanticErrorKind` usage is part of the blocked wiring cascade. The `SemanticErrorKind` fold is one-to-many (§5) and needs instance-level classification, so no lossy coarse mapping was baked in.)_
- [ ] Surface cap timing fields, including `reset_at` and `retry_after_ms`, through diagnostic detail for throttled errors. _(BLOCKED: needs the `cap.*` `Diagnostic` impl over `RateLimitInfo`; the registry pins `reset_at`/`retry_after_ms` on `cap.rate_limit`/`cap.plan_limit`.)_
- [ ] Extend lifecycle late-binding `err.*` with `category`, `code`, `disposition`, `origin`, `detail.*`, `severity`, and promoted convenience fields. _(BLOCKED: `LifecycleErrorInfo::to_value()` (`lifecycle_context.rs`) can only project these once its source errors implement `Diagnostic`; downstream of the cascade + needs compile.)_
- [ ] Preserve legacy `err.kind`, `err.variant`, and `err.msg`; treat `kind` and `variant` as deprecated aliases for `category` and `code`. _(BLOCKED on the same `err.*` projection wiring; the existing `kind`/`variant`/`msg` are untouched and remain.)_
- [ ] Add `claudine errors` or the agreed introspection surface listing codes and detail schemas from the registry. _(BLOCKED for the CLI half: the registry **is** the data source (`CODES` + `CodeSpec.detail`), but the clap subcommand + renderer is cross-crate and needs compile/test — cargo auto-denied.)_
- [ ] Add tests proving handlers can match by pattern, code, and instance detail without parsing human messages. _(BLOCKED: needs the `err.*` projection + a running expression engine.)_
- [ ] Validation checkpoint: every handleable error in scope exposes the ratified facets, and docs/examples use the new faceted names. _(BLOCKED: end-to-end coverage needs the wiring + `just test`/`just lint`, all unavailable here.)_

Parallelizable after the trait and enum shapes land:

- [ ] Implement composition diagnostic facets. _(BLOCKED — same as the composition `Diagnostic` impl above.)_
- [ ] Implement provider/cap/timeout diagnostic facets. _(BLOCKED — same as the cap-timing detail above.)_
- [ ] Implement lifecycle `err.*` projection tests. _(BLOCKED — downstream of the `err.*` projection wiring.)_
- [ ] Implement the CLI introspection report. _(BLOCKED — the `claudine errors` CLI half.)_

## Phase 8 - Converge Excerpt Paths And Close Late-Binding Corners

Goal: finish the hard corners after the typed substrate and render/handle contracts are stable.

> **Implementation note (2026-06-28, non-interactive session).** Phase 8 is the
> **convergence phase** — by its own goal it runs *after* the typed substrate and
> render/handle contracts are stable. Those contracts are the Phase 2/3/6/7 typed-error
> cascade, which **never landed**: every one of Phases 2–7 was BLOCKED in this same
> non-interactive session because `cargo`/`just` are permission-gated and auto-denied
> (re-confirmed this session — `cargo --version` is denied even with the sandbox
> disabled). Unlike Phases 2/4/5/7 (each a clean, compile-on-its-own additive island),
> Phase 8 is intrinsically convergence/migration work and has **no safe additive island**.
> The split:
>
> - **DONE — strict-raise halt + finalize re-entry guard (task 3).** This landed earlier
>   on the `real-errors` branch (commits `5513bf053` "surface late-binding lifecycle
>   evaluation errors" + `f4ec486b3` "halt on late-binding lifecycle evaluation errors").
>   The re-entry guard — *a raise inside `finalize` halts without re-entering `finalize`* —
>   is in `claudine/cli/src/commands/wrap/composition/mod.rs:323` and
>   `harness_orch/loop_control.rs:436,528`, with the catch-event precedence
>   (`finalize > failure > original`) and dedicated integration tests in
>   `claudine/cli/tests/wrap_compose_validation.rs` ("finalize raise precedence",
>   "success raise + finalize marker ordering"). Marked complete; it could not be
>   re-verified via `just test` here (cargo gated), but its code + tests are present.
>
> - **BLOCKED — `SourceRef::Effective` (tasks 1, 2, + parallelizable test).** `SourceRef`
>   has **no production home in the codebase** — it exists only in the design docs
>   (`integrated-design.md:283`). It was a Phase 3 deliverable (`SourceRef::OnDisk`) that
>   never landed (cargo-gated cascade). There is no enum to extend, so there is no
>   additive island; landing a brand-new cross-crate enum in `biscuit-terminal::errors`
>   blind risks breaking the ~48 workspace members that depend on it — the precise risk
>   the Phase 3 note refused.
>
> - **BLOCKED — excerpt convergence (tasks 4–7, + parallelizable comparison).** Migrating
>   Claudine's `FrontmatterExcerpt`/`WithFrontmatter` onto
>   `biscuit_terminal::errors::SourceContext::focused_yaml_excerpt` (Phase 5's landed
>   engine) is a **byte-sensitive cross-crate render change**: the current path renders the
>   *whole* frontmatter as a line-numbered `darkmatter::markdown::CodeBlock`, while
>   `focused_yaml_excerpt` renders a *focused, non-contiguous, elided* `Prose` — a
>   deliberate behavior change. Task 6 forbids removing the old path "only after snapshot
>   parity proves the shared path covers existing cases", and that parity (task 7's TTY /
>   `FORCE_COLOR` / `NO_COLOR` / non-TTY snapshots) can only be established by running
>   snapshots — `cargo`/`just` auto-denied. The migration also needs the typed key-set
>   supplier (`Interpolation.key` + referenced keys) that Phase 5 flagged BLOCKED on the
>   Phase 2/3 cascade; without it `focused_yaml_excerpt` degenerates to its whole-block
>   fallback, adding dead cross-crate code with no behavior gain (against Rule 2). Landing
>   a cross-crate call blind risks a non-compiling `claudine`.
>
> - **PARTIAL/BLOCKED — late-binding tests (task 8).** The unknown-root / malformed-span /
>   known-null subset is already exercised by the task-3 integration tests in
>   `wrap_compose_validation.rs`; the effective-source-rendering subset is downstream of
>   the un-landed `SourceRef::Effective` (task 1). Authoring further un-compilable tests
>   would break `just test` for the whole package — the risk the prior phases declined.
>
> - **BLOCKED — validation checkpoint (task 9).** Needs `just test`/`just lint`, both
>   auto-denied. Deferred, with all of the above, to the next interactive run; land the
>   Phase 2/3 cascade first, then this convergence phase.

- [ ] Add `SourceRef::Effective { rendered, origin_key }` for DM2 late-binding lifecycle evaluation failures. _(BLOCKED: `SourceRef` has no production home — only in the design docs; the `SourceRef::OnDisk` base is the un-landed Phase 3 deliverable. New cross-crate enum in `biscuit-terminal::errors`; cannot compile/verify — cargo auto-denied.)_
- [ ] Render effective-source failures with the resolved value or expression and origin key, without fabricating disk line numbers. _(BLOCKED on the `SourceRef::Effective` enum above.)_
- [x] Ensure strict DM2 lifecycle evaluation failures halt the run once and do not recursively re-enter `finalize`. _(DONE on-branch: commits `5513bf053` + `f4ec486b3`; re-entry guard in `composition/mod.rs:323` + `loop_control.rs:436,528`, catch precedence `finalize > failure > original`, integration tests in `wrap_compose_validation.rs`. Not re-runnable here — cargo gated.)_
- [ ] Migrate Claudine `FrontmatterExcerpt` and `WithFrontmatter` rendering onto Darkmatter `SourceContext::focused_yaml_excerpt`. _(BLOCKED: byte-sensitive cross-crate render change (whole-block `CodeBlock` → focused non-contiguous `Prose`); needs the typed key-set supplier from the un-landed Phase 2/3/5 wiring AND compile/snapshot-parity feedback — cargo auto-denied.)_
- [ ] Preserve Claudine's current TTY gating, `NO_COLOR`, `FORCE_COLOR=1`, and non-TTY ANSI stripping behavior during excerpt convergence. _(BLOCKED on the migration above.)_
- [ ] Remove duplicated excerpt rendering only after snapshot parity proves the shared path covers existing cases. _(BLOCKED: snapshot parity requires running tests; cargo auto-denied.)_
- [ ] Add terminal snapshots for TTY color, forced color, no-color, and non-TTY outputs. _(BLOCKED: cannot run PTY/snapshot tests — cargo auto-denied.)_
- [ ] Add late-binding lifecycle tests for unknown roots, malformed spans, known-null references, and effective-source rendering. _(PARTIAL/BLOCKED: unknown-root/malformed-span/known-null subset already covered by the task-3 `wrap_compose_validation.rs` integration tests; the effective-source subset is downstream of the un-landed `SourceRef::Effective`; authoring un-compilable tests would break `just test`.)_
- [ ] Validation checkpoint: compose-time and event-time interpolation errors both render and classify through the same typed chain without string parsing. _(BLOCKED: needs the un-landed typed chain (Phase 2/3) + `just test`/`just lint`; cargo auto-denied.)_

Parallelizable:

- [ ] Build `SourceRef::Effective` lifecycle tests while excerpt convergence snapshots are prepared. _(BLOCKED: `SourceRef::Effective` does not exist — see task 1.)_
- [ ] Compare old Claudine excerpt snapshots against the shared Darkmatter renderer before removing old code. _(BLOCKED: requires running both renderers under test; cargo auto-denied.)_

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
