---
spec: /Volumes/coding/wt/rusty-biscuit/fix-sequences/darkmatter/features/2026-11-14-file-interpolate-option/spec.md
plan: darkmatter/features/2026-11-14-file-interpolate-option/plan.md
implemented_by: claude/opus
started_phase: "1"
packages:
    - darkmatter
    - darkmatter-lsp
source_files_during_phase_1:
    - darkmatter/lib/src/markdown/compose/directives_api.rs
    - darkmatter/lib/src/markdown/compose/parse_utils.rs
    - darkmatter/lib/src/markdown/compose/transclusion/types.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
    - darkmatter/dmls/src/overlay/directives.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1:
    - .claude/skills/darkmatter/compose.md
decisions_for_review:
    - |-
      The caret of an invalid `interpolate=` error points at the value's
      first column (for `::file ./a.md interpolate=banana`, column 27), not at
      the `interpolate` key. The plan only said "the authored option
      position". The value is what is wrong, so pointing at it reads best and
      matches where an editor diagnostic over the value span would sit.
      Pointing at the key instead is a one-line change in
      `parse_interpolate_option` (capture the column before reading the key)
      plus the column numbers asserted in the parser tests.
    - |-
      Which text counts as an "active input" (and so keeps evaluating under
      interpolate=false). The plan said: the target span and each option
      value span of a directive line. I instead treat the whole directive
      line from its keyword to the end of the line as active, because the
      directive scanner splits a target on spaces, so `::file ./{{ lang }}.rs`
      would leave `lang }}.rs` outside every span and the target expression
      would not evaluate. Option keys never hold expressions, so nothing that
      was prose becomes active. The exceptions: a `::disclosure` / `::details`
      line's summary text is displayed, so it stays prose (only its option
      values are active), and `::file-links` / `::toc-linking` lines are
      active like every other directive (the plan's list did not name them).
      Going back to per-span ranges means fixing the target tokenization
      first; the change is confined to `active_input_ranges` and its tests.
    - |-
      The pre-flight walk's scan of authored expressions (which plans ICMP
      probes and finds `as_markdown` literals to walk) now skips suppressed
      prose. The plan said Phase 3 needs no pre-flight code change, but
      without this a `ping("…")` sample in suppressed prose was still planned
      as an ICMP probe, which the spec forbids. The alternative was to leave
      it for Phase 3; it would have left a known-failing requirement in this
      phase. Undoing it is a revert of the two `prose_enabled` arguments in
      `preflight/collect.rs`.
    - |-
      Remote-URL discovery for expressions filters the found expressions to
      the active ranges instead of scanning a prose-masked body, as the plan
      suggested. The discovery scan detects code blocks, and a masked body
      can turn a directive line inside a nested list or quote into what
      looks like an indented code block, which would drop a URL that will be
      fetched (losing only its prefetch, but silently). Context-demand and
      capability planning do scan a masked body, because they read it as
      plain text. Switching remote discovery to masking is a one-line change
      in `discover_remote_urls_from_expressions_with`.
    - |-
      A link in suppressed prose whose destination holds interpolation
      syntax, such as `[x](./{{name}}.md)`, now reaches link validation as
      written. Validation reads `{{name}}` as biscuit-file's `{{VAR}}`
      environment-variable syntax; a lowercase name is invalid there, so the
      whole reference check fails with an unlocated `invalid-reference`
      error and `md compose` exits 1. I left this alone: the spec keeps link
      handling unchanged, and this is how any document with such a link
      already behaves. Alternatives: report it as one located broken-link
      issue (a change to the reference validator for every document), or
      skip validating destinations that still hold `{{` in suppressed prose
      (a special case that hides a really broken link). Either is confined
      to `validate_local_path` in `reference/validate.rs` plus a test; the
      Phase 5 docs could also warn authors to keep such examples in code.
    - |-
      Which reference-graph children inherit the disabled setting. The plan
      named only the `::file` fold. I made prologue, epilogue, and
      `::toc-linking` children inherit their parent's setting (composition
      does the same for prologue/epilogue), and a fragment link's target
      file that is not part of the graph is prepared with prose enabled,
      because it is read as its own document, not through an inclusion.
      Changing either is a one-argument edit at the `accepted_child_options`
      call sites in `reference/graph.rs`.
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/interpolation/active_inputs.rs
    - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/call_precheck.rs
    - darkmatter/lib/src/markdown/compose/context/capture/groups.rs
    - darkmatter/lib/src/markdown/compose/context/capture/capabilities.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/remote.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/tests/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/file_interpolate.rs
    - darkmatter/lib/src/markdown/compose/tests/transclusion.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_3:
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
    - darkmatter/lib/src/markdown/compose/tests/file_interpolate.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/compose.md
message_to_agent: |-
  Phase 3 folded the setting into the reference graph
  (`reference/graph.rs::accepted_child_options` takes a `ProseInterpolation`;
  `::file` children fold their directive, prologue/epilogue/`::toc-linking`
  inherit). Preflight needed no code change. Phase 3 tests live in
  `compose/tests/file_interpolate.rs` (preflight, cache, agreement) and
  `reference/graph.rs` tests (graph); `compose/tests/preflight.rs` and
  `caching.rs` are empty placeholders. For Phase 4's CLI fixtures: a link in
  suppressed prose whose destination is `./{{name}}.md` makes `md compose`'s
  reference check fail with `invalid-reference` (biscuit-file reads
  `{{VAR}}` as an environment variable), and `./{{ name }}.md` with spaces
  is not a link at all, so keep interpolation examples out of link
  destinations in the publication fixture unless the test is about that
  (see the log's decisions). `git diff` here uses an external diff tool;
  pass `--no-ext-diff`.
---

# Implementation Log for 2026-11-14-file-interpolate-option (6 phases)

## Phase 1

Implemented directly (no subagents; the two waves were small and shared
files through compile-time checks).

### What changed

- `directives_api::parse_interpolate_value` (public): the one value
  validator. Unquoted `null` → `Ok(None)`; empty or whitespace-only →
  error; boolean words via `simplified_schema::numeric::parse_boolean_word`
  (quoted or not) → `Ok(Some(b))`; everything else, quoted `"null"`
  included → error naming the value.
- `BlockOptions.interpolate: bool` with a hand-written `Default`
  (`interpolate: true`; every other field as the derive produced).
- `parse_directive_line` intercepts `interpolate` for `::file`/`::url` only
  (`parse_interpolate_option`, with the `was_quoted` capture); each
  occurrence is validated in order, so the last valid one wins and an
  earlier invalid one errors. The error is `TransclusionError::ParseDirective`
  with the directive line and `caret_col` at the value. `::code` falls
  through to the unknown-option path. `Cursor::current_column` became
  `pub(crate)` so the parser can record the value's column.
- `context::options::ProseInterpolation` (`Enabled` default,
  `DisabledByInclusion`), field `prose_interpolation`, builder
  `with_prose_interpolation`, `Debug` entry, and both identity encoders
  (graph beside `interpolate_code_blocks`, cache beside
  `replace_parent_wins`) through `prose_interpolation_tag`.
- `transclusion::effective_prose_interpolation` folded in
  `inherited_child_options` (covers local `::file` children, the preflight
  walk, and `as_markdown` children) and by hand on the remote `RemoteFile`
  child path.
- `parameters_identity` keeps `0` for a default site and folds a constant
  component for `interpolate=false`, with or without a `set` overlay.
- The `compose:` child cache key gained a final `:{0|1}` component for the
  child's folded setting.
- DMLS: `option_keys` splits `File | Url` (gain `interpolate`) from `Code`
  (unchanged six keys); `option_values("interpolate")` → `true`/`false`.
- Skill: `.claude/skills/darkmatter/compose.md` gained an internals note
  with a **Planned** marker for the suppression that Phase 2 lands.

### Requirement → test mapping

Boundary: the directive parser's public result (`parse_directives` →
`BlockOptions` / `TransclusionError`), F1 in-process, L1. Wiring through
`Markdown::compose_with` is one control plus one invalid shape, L1.

| Matrix row / rule | Test |
|---|---|
| absent → `true`, equal to explicit `true` and `null` | `parser::tests::interpolate_absent_is_the_default_true_and_equals_explicit_true` |
| unquoted `null` (`::file`, `::url`) | `interpolate_unquoted_null_is_the_default` |
| true words, any case, quoted-with-spaces trimmed | `interpolate_true_words_in_any_case_are_true` |
| false words, any case, `::file` and `::url`, spaced `=` | `interpolate_false_words_in_any_case_are_false` |
| quoted boolean words | `interpolate_quoted_boolean_words_convert_like_bare_ones` |
| other options unaffected (downstream state) | `interpolate_false_leaves_every_other_option_untouched` |
| quoted `"null"` → located error (file, line, caret) | `interpolate_quoted_null_is_text_and_a_located_error` |
| error line inside a longer document | `interpolate_error_reports_the_directive_line_in_the_document` |
| empty `""`, `''`, and `=` at end of line | `interpolate_empty_values_are_located_errors` |
| object, array, other text (`banana`, `True-ish`, `2`, `nil`, `NULL`) | `interpolate_objects_arrays_and_other_text_are_located_errors` |
| repeated: last valid wins | `interpolate_repeated_last_valid_wins` |
| repeated: every occurrence validated (either order) | `interpolate_every_repeated_occurrence_is_validated` |
| `::code`: unknown option, never validated | `interpolate_on_code_is_an_unknown_option_and_never_validated`; compose-level `tests::transclusion_tests::interpolate_on_code_keeps_the_unknown_option_warning` |
| Rule 4–5: both identity products | `context::options::tests::prose_interpolation_is_in_both_identity_products` |
| Rule 6: fold truth table, nested `true` cannot re-enable, default options inherit | `engine::tests::prose_interpolation_fold::disabled_iff_the_parent_is_disabled_or_the_directive_writes_false` |
| Rule 6: single derivation, never escapes to parent or sibling | `engine::tests::prose_interpolation_fold::the_single_child_derivation_folds_without_touching_the_parent` |
| Rule 7: parameter identity | `preflight::parameters_identity_tests::{the_default_site_keeps_its_zero_identity, interpolate_false_is_a_different_child}` |
| Phase-1 checkpoint: accepted, composes byte-identically | `tests::transclusion_tests::interpolate_false_is_accepted_and_composes_like_the_plain_directive` |
| wiring: invalid value fails composition | `tests::transclusion_tests::invalid_interpolate_value_fails_the_composition` |
| Rule 8: DMLS catalog split and values | `darkmatter-lsp overlay::directives::tests::interpolate_is_an_option_of_file_and_url_but_not_code` |

The `compose:` cache-key component has no direct test in this phase: with
suppression not wired the two keys compose identical bytes, so a stale
reuse is not observable yet. The spec's "two uses of one file, both orders,
cache on and off" test belongs to Phase 2.

Code-smell pass: the field is a plain `bool`; null versus quoted-`"null"` is
decided by `was_quoted`; no `unwrap_or_default()`, `.ok()`, or
`filter_map(.. as_str())` on the parse (`value.unwrap_or(true)` maps the
validated null to the documented default).

### Gates

- Targeted: `cargo nextest run -p darkmatter --lib -E 'test(interpolate) |
  test(prose_interpolation) | test(parameters_identity) |
  test(transclusion::parser)'` → 115 passed; `cargo nextest run -p
  darkmatter-lsp -E 'test(overlay::directives)'` → 6 passed.
- `just lint` (darkmatter area): clean, including the zed-dmls wasm check.
- `just test` (darkmatter area): 16807 passed, 10 skipped, 1 slow:
  `markdown::compose::shell_expansion::probe::tests::installed_shells::injection_shaped_names_never_execute`
  at 7.10 s in the package run (load average 8–17 at start on 16 cores),
  1.16 s alone (load average ~47, from other host activity; no subagents
  were running). Under the loaded-host 10 s allowance and ≤5 s alone, so it
  is cleared. This test is untouched by this phase.
- Gates were not skipped (the phase changed compiled source).

### Dependents

- `cargo check --tests` over every package depending on `darkmatter`
  (research, claudine, claudine-cli, claudine-gen, playa-cli,
  sniff-detection-cli, messenger, messenger-cli, biscuit-icon-cli): clean.
  No file outside darkmatter uses `interpolate=` on a directive, and no
  dependent constructs `BlockOptions`.
- Targeted Claudine run (`-E 'test(transclu) | test(file_directive) |
  test(::file)'`, claudine + claudine-cli): 62 passed, 1 failed:
  `claudine-cli::l1 document_environment_routes::a_prompt_task_and_its_transclusion_read_their_legs_map`.
  **Pre-existing:** with this phase's patch reverse-applied it fails the
  same way (the prompt reads `env=launch` instead of the leg's map); the
  worktree carries uncommitted Claudine sequence work. The patch was then
  re-applied and the targeted darkmatter tests re-run green.

### Cross-OS

No path, process, or `cfg` code changed; the work is string parsing and
identity hashing. No cross-OS check was run.

## Phase 2

Implemented directly (no subagents): the four tasks share the new
classifier and one test file, and each step was small enough to verify
inline with targeted filters.

### What changed

- New `compose/interpolation/active_inputs.rs`: `active_input_ranges`,
  `intersects_active`, `mask_prose`. Active input is an authored directive
  line from its keyword on (target, options, callback), the option values
  of a `::disclosure`/`::details` line, and every `::shell-block` body;
  everything else, code blocks and page/collection block bodies included,
  is prose. Directives and blocks are found with the same `data`-aware
  scanners execution uses (`scan_darkmatter_directives_in`,
  `scan_block_pairs_in`), so inserted text can neither open nor hide one.
  `mask_prose` blanks prose bytes to spaces and keeps line breaks and
  offsets.
- `inline::interpolation::run_stage`: under
  `ProseInterpolation::DisabledByInclusion` the target pass runs unchanged;
  the body pass passes a `select` that declines every span not overlapping
  an active range and composes with the existing late-read `defer` closure.
  The scan mode ignores `interpolate_code_blocks` under suppression.
  `resolve_pending` audited: a pending read only comes from an accepted
  span, and its `chosen` closures match exact spans, so no change.
- `interpolation::rewrite::interpolate_text_selected`: an embedded literal
  token fails only when `select` (when present) accepts its span (called
  with `source: None`). Existing selectors: `run_stage`'s defer accepts
  every `None`-source span (unchanged behavior); `resolve_pending`'s plain
  selector declines `None` sources, but its input already passed a full
  pass that fails on any authored token, so nothing reachable changes.
- Passive surfaces take the same classification:
  - `call_precheck::check_body` retains only body expressions overlapping
    an active range (and ignores `interpolate_code_blocks`) when the
    document's prose is disabled; frontmatter, `when=`, page-block
    conditions, and collection gates/callbacks keep their checks. The
    module's surface table gained the exception.
  - `ContextRequirements::for_document_with(document, prose_enabled)`
    (crate-private; `for_document` delegates with `true`), used by
    `extend_context_for` / `needs_extension_for`, the local and remote
    `::file` child context handoff in `engine.rs`, and `nested.rs`.
  - `DeferredCapabilities::for_document_with`, used by the preflight walk.
  - `remote::discover_remote_urls_from_expressions_with` (crate-private)
    filters expression locations to active ranges rather than masking,
    because it scans with Markdown-aware code detection; used by the root
    pipeline registration and the fetched-remote-child registration
    (which now folds the child setting before scanning).
  - Preflight `authored_expressions` (feeding `authored_icmp_probes` and
    `authored_as_markdown_literals`) scans the masked body when the
    document's prose is disabled. This surface was not named in the plan;
    without it, a `ping(...)` sample in suppressed prose was still planned
    as an ICMP probe.
- `ComposeOptions::prose_enabled()` is the one read of the setting outside
  the fold; the `compose:` cache-key component now reuses the folded value
  computed for the child context handoff.
- Phase 1's intermediate test
  `transclusion_tests::interpolate_false_is_accepted_and_composes_like_the_plain_directive`
  became `interpolate_false_keeps_the_childs_prose_literal` (asserts the
  literal output and that only that line differs from the plain directive).
- Skill: `.claude/skills/darkmatter/compose.md` internals note — the
  **Planned** bullet is replaced by the classifier and consumer map.

### Requirement → test mapping

Boundary: the library's public composition result (`compose_with` /
`compose_preflight` output, report, and errors) over real files in a temp
dir, F2 in-process, L1; the classifier and the passive scanners also have
F1 unit tests. All new tests are in the `darkmatter` lib's L1 unit binary
(`cargo nextest -p darkmatter --lib`), selected by `just test`.

| Spec row / rule | Test |
|---|---|
| Rule 10 classifier: directive lines (targets with spaces, every option incl. `set.x=`, `when=`, `exclude=`), `::code`/`::url`, `::shell`, `::shell-block` body, `::map`/`::filter` source and callback, nested directive in a block body, prose/inline/fenced/indented code/table not active, data cannot open a directive, multibyte masking | `interpolation::active_inputs::tests::*` (10 tests) |
| False with known/unknown names, invalid call, malformed text, triple braces, inline code; no warning under `--deny all`; parent prose still interpolates | `tests::file_interpolate::literal_prose_survives_with_no_diagnostic_even_under_deny_all` |
| Control: same examples without the option warn, block under `--deny all`, and a malformed expression fails the inclusion | `without_the_option_the_examples_are_evaluated` |
| Value spellings reach behavior (false words, quoted; true words, null, absent) | `every_false_spelling_suppresses_and_true_spellings_interpolate` |
| `interpolate_code_blocks: true` (child frontmatter and request option) cannot re-enable code; control interpolates code | `code_blocks_stay_literal_even_when_the_child_opts_into_code_interpolation` |
| Child frontmatter interpolation and `set` precedence | `frontmatter_interpolates_and_set_overrides_keep_their_precedence` |
| Conditions, map gate/callback, filter callback execute; selected/repeated prose literal | `conditions_and_collections_execute_while_their_prose_stays_literal` |
| Nested `::file {{ target }}` and option-value expressions evaluate; nested child inherits | `nested_targets_and_option_values_evaluate` |
| `::shell` / `::shell-block` inputs evaluate; approved bytes equal the run bytes | `shell_inputs_evaluate_and_run_with_the_interpolated_bytes` |
| Replacement, exclusion, quotation wrapper still apply; the directive's `replace=` value is the parent's input | `replacement_exclusion_and_wrappers_still_transform_the_literal_prose` |
| Same child normal + false, both orders, cache ReadWrite and Off; parent prose interpolates | `the_same_child_composes_independently_under_each_setting` |
| False ancestor with nested omitted/false/true/null; explicit true no warning under `--deny all`; never reaches a sibling | `a_disabled_ancestor_cannot_be_re_enabled` |
| Prologue/epilogue inherit | `prologue_and_epilogue_of_a_disabled_child_inherit_it` |
| Nested composition from an active input (`as_markdown` in a `when=`) evaluates; prose `as_markdown` stays literal | `nested_composition_from_an_active_input_inherits_it` |
| Permitted remote Markdown via `::file` and `::url` inherits | `file_interpolate::remote::permitted_remote_markdown_inherits_it` |
| Saved output recomposes under normal rules | `a_saved_literal_output_recomposes_under_normal_rules` |
| Read-side sample in prose: no read (`read_set` empty); control reads | `a_read_side_sample_in_suppressed_prose_reads_nothing` |
| Remote sample in prose: never fetched (wiremock `expect(0)`, `fetched == 0`) | `file_interpolate::remote::a_remote_sample_in_suppressed_prose_is_never_fetched` |
| Call check: invalid call in prose not rejected (literal test); same call in a directive input still rejected | `the_call_check_still_rejects_an_invalid_call_in_a_directive_input` |
| ICMP: prose `ping` plans no probe; a directive-input `ping` is planned; nothing sent | `preflight_plans_no_probe_for_a_sample_in_suppressed_prose` |
| Context demand: prose `ctx.*` (inline code and opted-in code blocks too) demands nothing; frontmatter and directive inputs still demand; `true` equals `for_document` | `context::capture::groups::tests::suppressed_prose_demands_only_through_frontmatter_and_directive_inputs` |
| Deferred capabilities: prose `current.*`/`ping` not planned; directive input and frontmatter planned | `context::capture::capabilities::tests::suppressed_prose_plans_only_frontmatter_and_directive_inputs` |
| Remote expression discovery: prose URL not registered; option-value URL registered | `remote::tests::suppressed_prose_registers_no_expression_url` |
| Phase 1 placeholder now asserts suppression | `tests::transclusion_tests::interpolate_false_keeps_the_childs_prose_literal` |

Not covered here, by plan: reference-graph preparation and the shell
preflight approval walk's agreement tests (Phase 3), editor (Phase 4).
Context demand is asserted at the requirement scanner rather than through a
public capture test, because `for_document_with` is crate-private and the
public `ContextCapture::capture_for_document` is root-only (always enabled).

### Gates

- Targeted while working: `cargo nextest run -p darkmatter --lib -E
  'test(file_interpolate) | test(active_inputs) | test(suppressed_prose) |
  test(interpolate) | test(transclusion)'` → 291 passed (rerun after the
  last edit, a pure move of the remote-child fold above its neighboring
  comment).
- `just lint` (darkmatter area): clean, including the zed-dmls wasm check
  (two `single_range_in_vec_init` hits in the new unit tests were fixed).
- `just test` (darkmatter area): 16839 passed, 10 skipped, **none at or
  above 5 s** (load average 18–41 during the run, from this session's
  build). The run started before the final pure-move edit in `engine.rs`;
  lint and the 291 targeted tests were rerun on the final tree.
- Scanner parse-count guards (Rule 17): no existing count changed, since
  the classifier runs only when a document's prose is disabled; no test was
  updated for it.
- Gates were not skipped (the phase changed compiled source).

### Dependents

- `cargo check -p claudine -p claudine-cli -p research --tests`: clean. No
  public signature changed (every new function is crate-private;
  `ContextRequirements::for_document`, `DeferredCapabilities::for_document`,
  and `remote::discover_remote_urls_from_expressions` keep their behavior).
  No file outside `darkmatter/` writes `interpolate=` on a directive
  (only `CLAUDE.md`'s planned note, Phase 5's).
- Targeted Claudine run (`-E 'test(transclu) | test(file_directive) |
  test(::file)'`, claudine + claudine-cli): 62 passed, 1 failed:
  `claudine-cli::l1 document_environment_routes::a_prompt_task_and_its_transclusion_read_their_legs_map`,
  the same pre-existing failure Phase 1 proved independent of this feature
  (it fails with the feature's patch reverse-applied). Not chased.

### Cross-OS

No path, process, or `cfg` code changed: byte-range classification over
text and option plumbing. No cross-OS check was run.

## Phase 3

Implemented directly (no subagents): one code change in the reference graph
plus tests, all small enough to verify inline with targeted filters.

### What changed

- `markdown/reference/graph.rs`: `accepted_child_options` takes the child's
  folded `ProseInterpolation` and sets it on the child request before
  `build_node` recurses, so `prepare_content`'s InlinePre run suppresses the
  same prose composition does. Callers:
  - `::file` children: `effective_prose_interpolation(parent, &directive.options)`
    (the directive is parsed with the transclusion parser, so
    `options.interpolate` is the Phase 1 field);
  - prologue, epilogue, and `::toc-linking` children: inherit the parent's
    setting (they have no `interpolate=` option of their own);
  - `prepare_content_for_validation` for a fragment target outside the graph:
    `Enabled`, because that file is read as its own document, not through an
    inclusion (descendant headings come from the build-time snapshot, which
    was prepared under the folded setting).
- `compose/transclusion/mod.rs`: re-exports `effective_prose_interpolation`
  crate-wide for the graph.
- Preflight: no code change (Rule 18 holds: the walk derives children through
  `inherited_child_options`; Phase 2 already masked its `authored_expressions`).
- Skill: `.claude/skills/darkmatter/compose.md` names the graph fold and why
  the walk needs none.

`compose/tests/preflight.rs` and `compose/tests/caching.rs` are empty
placeholder modules, so the Phase 3 engine tests went into
`compose/tests/file_interpolate.rs` beside the Phase 2 cases (as the Phase 2
handoff suggested), and the graph tests into `reference/graph.rs`'s test
module, next to the graph's existing identity tests.

### Finding: brace-bearing link destinations in suppressed prose

A link written in suppressed prose as `[x](./{{name}}.md)` now survives
into the output, and link validation reads `{{name}}` as biscuit-file's
`{{VAR}}` variable syntax. A lowercase name is invalid there, so the whole
reference check fails with an unlocated `failure: invalid-reference` and
`md compose` exits 1 (checked by hand: `/tmp/fi-*/root.md`). A
`{single}`-brace or plain missing target is a located "file not found"
issue instead. This is existing link-validation behavior (documented in
`docs/errors/file-reference-failures.md`), and the spec keeps link handling
unchanged, so it was left as is and listed in `decisions_for_review`. A
destination with spaces (`./{{ name }}.md`) is not a CommonMark link at all,
so it is never validated.

### Requirement → test mapping

Boundary: the library's public results (`compose_with`, `compose_preflight`,
`reference_graph` / `validate_references*`) over real files in a temp dir,
F2 in-process, L1, in the `darkmatter` lib unit binary (selected by
`just test`).

| Plan item / spec row | Test |
|---|---|
| Rule 19: a `false` child prepares literal prose (link target kept as written), no warning under deny-all | `reference::graph::tests::a_disabled_child_prepares_its_prose_literally_and_keeps_its_links` |
| Control: the same child included normally evaluates prose in preparation (malformed example fails it) | `reference::graph::tests::an_enabled_child_still_evaluates_its_prose_in_preparation` |
| Rule 20: links in suppressed prose still validated, located, with their inclusion trail | `reference::graph::tests::links_in_suppressed_prose_are_still_validated` |
| Graph inheritance: nested child (with `interpolate=true`) and prologue inherit; sibling interpolates | `reference::graph::tests::the_disabled_setting_reaches_descendants_but_not_siblings` |
| Graph options identity differs by setting: a graph built disabled is refused for an enabled request (`Options` mismatch), accepted for the same | `reference::graph::tests::a_graph_built_with_prose_disabled_is_not_reused_with_it_enabled` |
| Preflight agreement: same approval set with/without the option when only prose differs; prose command samples (inline code, fenced `::shell`) never approved; execution asks for exactly the walked set | `tests::file_interpolate::the_approval_walk_approves_exactly_the_commands_execution_runs` |
| A prose `as_markdown("::shell …")` sample is neither walked nor run under `false`; a normal inclusion walks it | `tests::file_interpolate::a_command_sample_in_suppressed_prose_is_never_discovered` |
| Rule 7/18: repeated `::map` copies differing only in `interpolate` are distinct edges (`parameters` 0 vs non-zero), each walked under its own setting; execution accepts both and runs exactly the walked set | `tests::file_interpolate::repeated_copies_differing_only_in_interpolate_are_distinct_sites` |
| Cache identity: a repeat under one setting is reused (control, `cache_stats.hits ≥ 1`); normal + `false` in both orders never share an entry (`hits == 0`) and both outputs are correct | `tests::file_interpolate::a_composed_child_is_reused_only_under_the_same_setting` |
| Validation checkpoint: one tree through execution, the approval walk, and reference-graph preparation agrees on literal prose, no warnings (deny-all), and command bytes | `tests::file_interpolate::execution_preflight_and_reference_graph_agree` |

Phase 2's `the_same_child_composes_independently_under_each_setting`
already covers both orders with the cache on and off for output and
warnings; the new cache test adds the reuse counters.

Regression proof: with the graph fold removed (the child request left
`Enabled`), the three graph behavior tests fail
(`a_disabled_child_…`, `links_in_suppressed_prose_…`,
`the_disabled_setting_reaches_…`) and pass with it. The identity and
control tests pass either way, by design.

### Gates

- Targeted while working: `cargo nextest run -p darkmatter --lib -E
  'test(file_interpolate) | test(preflight) | test(caching) |
  test(reference::) | test(transclusion) | test(active_inputs)'` → 511
  passed.
- `just lint` (darkmatter area): clean, including the zed-dmls wasm check.
- `just test` (darkmatter area): 16849 passed, 10 skipped, **none at or
  above 5 s**, on the final tree.
- Gates were not skipped (the phase changed compiled source).

### Dependents

- `cargo check -p claudine -p claudine-cli -p research --tests`: clean. No
  public signature changed (`accepted_child_options` is private; the
  re-export is crate-private).
- Targeted Claudine run (`-E 'test(transclu) | test(file_directive) |
  test(reference)'`, claudine + claudine-cli, `--no-fail-fast`): 253
  passed, 1 failed:
  `claudine-cli::l1 document_environment_routes::a_prompt_task_and_its_transclusion_read_their_legs_map`,
  the same failure Phase 1 showed also happens without this feature. Not
  chased. The host load average was about 106 during this run, from
  outside this session.

### Cross-OS

No path, process, or `cfg` code changed (one setting passed to an existing
child-request derivation, plus tests over temp-dir fixtures). No cross-OS
check was run.
