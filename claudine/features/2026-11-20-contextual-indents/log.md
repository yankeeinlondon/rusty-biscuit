---
implementation_1: "2026-11-20T22:51:51-08:00"
---

# Contextual Indents Review Log

## Implementation of Review Findings #1

> **started at:** 2026-11-20T22:51:51-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/home/build/coding/rusty-biscuit/claudine/features/2026-11-20-contextual-indents/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false` at start
- no local OS testing rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); cross-OS proof is left to CI

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | No-op Markdown saves rewrite authored text and resolve indentation | fixed | every required child fixed or clean; see children |
| R1-F1.1 | Markdown frontmatter editor via `md set --save`, equal quoted scalar → original bytes | fixed | `frontmatter_path_edit::setting_a_quoted_scalar_to_its_own_value_keeps_the_source_and_resolves_nothing` (exact bytes, 0 fallback calls); CLI `get_set_rm::unchanged_saves::setting_a_quoted_scalar_to_its_own_value_leaves_the_file_unwritten` (bytes + mtime unchanged); pass |
| R1-F1.2 | Same editor, equal whole mapping (block, quoted, comment) → original bytes | fixed | `frontmatter_path_edit::setting_a_block_mapping_to_its_own_value_keeps_its_quotes_and_inner_comment`; CLI `unchanged_saves::setting_a_block_mapping_or_sequence_to_its_own_value_keeps_its_comments`; pass |
| R1-F1.3 | Same editor, equal whole sequence (block, quoted, comment) → original bytes | fixed | `frontmatter_path_edit::setting_a_block_sequence_to_its_own_value_keeps_its_quoted_item_and_comment`; same CLI test; pass |
| R1-F1.4 | CLI destination lookup before the editor on equal value → no indentation lookup | fixed | `md set`/`md rm` pass a lazy `|| destination_indentation(..)`; the library asks it only after the no-op check (counting-closure tests show 0 calls); unchanged `--save` skips the write and reports `saved: false` (`unchanged_saves::an_unchanged_save_reports_that_nothing_was_saved`) |
| R1-F1.5 | `EffectEngine::set_frontmatter` equal title → original bytes, no lookup | fixed | verb returns before `save` (sole `destination_indentation` caller in `verbs.rs`); `markdown_save_indentation::setting_a_property_to_the_value_it_holds_leaves_the_file_unwritten` (exact bytes + mtime, two-space `.editorconfig` repo); pass |
| R1-F1.6 | `EffectEngine::merge_frontmatter` existing title → original bytes, no lookup | fixed | `markdown_save_indentation::merging_values_that_already_hold_leaves_the_file_unwritten`; pass |
| R1-F1.7 | `EffectEngine::delete_frontmatter` absent property → original bytes, no lookup | fixed | `markdown_save_indentation::deleting_an_absent_property_leaves_the_file_unwritten`; pass |
| R1-F1.8 | Plain-YAML editor equal scalar → original bytes (control) | clean | `yaml_edit.rs` guard `if expected == original { return Ok(source.to_string()) }`; `yaml_edit::indentation_of_new_values` pass |
| R1-F1.9 | `ConfigTransaction` incl. Claudine, equal second save → unchanged, no resolver call (control) | clean | `config_write_indentation::the_preference_is_resolved_once_across_batches_and_never_when_nothing_renders` (0 resolver calls, `written: false`); claudine `config_indentation::create_edit_and_an_unchanged_save_round_trip`; pass |
| R1-F1.10 | Hash save, unchanged baseline → no fallback call, no write (control) | clean | `stamp_baseline_with_indentation` calls `fallback()` only inside `if decision.new_stored.is_some()`; `hash.rs` writes only `if let Some(written) = stamp.text`; `baseline::tests::unchanged_content_needs_no_write` |
| R1-F1.11 | Claudine compatibility writer, no changes → not called (control) | clean | `classify.rs` `if changed { write_markdown_document(..) }`; new `linking::compatibility::tests::a_skill_needing_no_upgrade_is_left_unwritten` (exact bytes with quoted, commented `name`); pass |
| R1-F1.12 | Removal CLI; increment/decrement, append/prepend → not a no-op path (control) | clean | `md rm` requires ≥1 prop and errors on a missing key; `bump` always writes n±1 or errors; `array_mutate` always grows or errors; `rm` now resolves lazily too |
| R1-F1.13 | Sibling: DMLS frontmatter providers/code actions → no save path | clean | `dmls/src` calls none of the editor/save APIs; code actions emit LSP insertions of missing keys only |
| R1-F1.14 | Sibling: `claudine/gen` agent-errors check, Schematic OpenAPI export → whole-document regeneration | clean | regenerate from a value with no authored text to preserve; output is deterministic; the no-op rule is under “Edits to existing text” |
| R1-F2   | In-memory document editing ignores explicit indentation | pending | |
| R1-F2.1 | Source-less branch of `serialized_into_source`, two-space argument → two spaces | pending | |
| R1-F2.2 | Source-present serialization branch → caller preference (control) | pending | |
| R1-F2.3 | Source-preserving node edit branch → caller preference (control) | pending | |
| R1-F2.4 | `Markdown::as_string_with_indentation` → caller preference (control) | pending | |
| R1-F2.5 | Default wrappers → four spaces (control) | pending | |
| R1-F3   | Four schematic L1 tests retain failing package timings | pending | |
| R1-F3.1 | `generated_docs.rs` generated documentation test → ≤ 5 s alone and in package | pending | |
| R1-F3.2 | `artifact_drift.rs` generated Rust artifact test → ≤ 5 s alone and in package | pending | |
| R1-F3.3 | `fresh_output_compile.rs` generated REST source test → ≤ 5 s alone and in package | pending | |
| R1-F3.4 | `definition_bundles.rs` definition-bundle test → ≤ 5 s alone and in package | pending | |
| R1-F3.5 | Feature tests rerun → none at 5 s (control) | pending | |

### Work Log

- starting the work on 'R1-F1 no-op Markdown saves' at 22:52:45
        - defect class: a save path renders and resolves a layout for edits whose resulting values equal the values already stored, instead of returning the source bytes untouched
        - Markdown editor (`edit_frontmatter_paths_with_indentation`) now returns the source bytes when the source reloads to the expected values, before any `Layout` is built
        - decision: the fallback indentation is now a lazy `impl Fn() -> Indentation` (cached per call) on both `edit_frontmatter_paths_with_indentation` and `edited_document_text_with_indentation`, matching `stamp_baseline_with_indentation`; alternative (a third lazy variant) rejected because both functions are new on this branch
        - decision: `md set --save` with an unchanged value skips the write and reports `saved: false` (matches `md hash --save`); documented in `darkmatter/docs/cli/set.md`
        - lifecycle `set_frontmatter`/`merge_frontmatter`/`delete_frontmatter` return before `save` when no value changes; consequence: with auto-rehash, a no-op no longer bumps `last_updated` (documented in `darkmatter/docs/topics/side-effects.md`)
        - docs: `darkmatter/docs/cli/set.md`, `darkmatter/docs/topics/frontmatter.md`, `darkmatter/docs/topics/side-effects.md`, `.claude/skills/darkmatter/frontmatter.md`
        - subagent focused runs: darkmatter 55/55, 253/253, 34/34, 15/15; claudine 42/42; no SLOW; darkmatter clippy clean, `check-zed` fails for missing `wasm32-wasip2` (environment)
- work completed for 'R1-F1 no-op Markdown saves' at 23:06:08
- starting the work on 'R1-F2 in-memory document indentation' at 23:06:08
        - defect class: a serialization branch drops the caller's explicit formatting argument and uses the default layout
