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
message_to_agent: |-
  Phase 1 left one test that pins the intermediate state and must change in
  Phase 2: `interpolate_false_is_accepted_and_composes_like_the_plain_directive`
  in darkmatter/lib/src/markdown/compose/tests/transclusion.rs asserts that
  `::file ./child.md interpolate=false` composes byte-identically to the plain
  directive ("Hello World." from `{{ name }}`). Once prose suppression lands,
  rewrite it to assert the suppressed output (`Hello {{ name }}.`) instead.
  The skill note in .claude/skills/darkmatter/compose.md
  ("`interpolate=` on `::file` / `::url` (internals)") has a **Planned**
  bullet saying suppression is not wired yet; remove it when Phase 2 lands.
  The folded setting is `ComposeOptions.prose_interpolation`
  (`context::options::ProseInterpolation`); read it from the child's options.
  Unrelated: claudine-cli's
  `document_environment_routes::a_prompt_task_and_its_transclusion_read_their_legs_map`
  fails in this worktree with and without this feature's changes (the
  worktree carries uncommitted Claudine sequence work); do not chase it.
  `git diff` here uses an external diff tool; pass `--no-ext-diff` to get a
  patch, and quote zsh path lists as arrays.
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
