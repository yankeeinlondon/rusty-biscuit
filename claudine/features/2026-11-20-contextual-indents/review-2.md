---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: "2026-11-21T00:13:56-08:00"
spec: 2026-11-20-contextual-indents/spec.md
implemented: false
description: "A **feature** review of `2026-11-20-contextual-indents/spec.md`"
feature: 2026-11-20-contextual-indents/review-2.md
previous: 2026-11-20-contextual-indents/review-1.md
---

# Review of Contextual Indents

The feature is production ready. All three previous findings are verified
fixed, including their required instances and the additional controls in the
repair ledger. This review found no new defect requiring repair. No author
decision or later human review is requested.

## Previous Finding Verification

I read `review-1.md` and the repair log's Finding Ledger before reviewing
the changes. There are no earlier reviews or author rulings in this directory.
The table below records the review's assessment independently of the repair's
completion flags. The instance checks following it cover every ledger child.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — No-op Markdown saves rewrite authored text and resolve indentation | R1-F1.1–R1-F1.12 from the review; additional ledger controls R1-F1.13–R1-F1.14 | verified fixed | Read the editor's equality check, lazy CLI resolver, lifecycle early returns, and control guards; reran public editor, lifecycle, CLI, transaction, Claudine persistence, compatibility, and plain-YAML tests. Details below. | Nothing |
| R1-F2 — In-memory document editing ignores explicit indentation | R1-F2.1–R1-F2.5 from the review; additional ledger sweep R1-F2.6 | verified fixed | The source-less branch now calls `as_string_with_indentation(indentation)`; the explicit two-space test and source-present controls executed successfully. | Nothing |
| R1-F3 — Four schematic L1 tests retain failing package timings | R1-F3.1–R1-F3.5 from the review; additional ledger guard R1-F3.6 | verified fixed | A fresh Schematic package L1 run executed 3,192 tests successfully in 10.900 s, with no individual test at or above five seconds. All replacement cases and the coverage guard ran. | Nothing |

### Unchanged saves: every required instance

The public Markdown editor is in
[Darkmatter's path editor](/home/build/coding/rusty-biscuit/darkmatter/lib/src/markdown/hash/path_edit.rs).
Its equality check precedes `Layout` construction. The CLI supplies a closure
for destination indentation and writes only when the resulting text differs
from the loaded source. The lifecycle verbs compare values before entering
their whole-document save function.

| Instance | Site and input checked | Verification evidence and result |
| --- | --- | --- |
| R1-F1.1 | Markdown editor, equal quoted scalar | `frontmatter_path_edit::setting_a_quoted_scalar_to_its_own_value_keeps_the_source_and_resolves_nothing` passed: exact bytes and zero resolver calls. CLI equivalent also passed, checking modification time. |
| R1-F1.2 | Markdown editor, equal block mapping with quoted child and inner comment | `setting_a_block_mapping_to_its_own_value_keeps_its_quotes_and_inner_comment` passed; CLI mapping/sequence case passed. |
| R1-F1.3 | Markdown editor, equal block sequence with quoted item and comment | `setting_a_block_sequence_to_its_own_value_keeps_its_quoted_item_and_comment` passed; CLI mapping/sequence case passed. |
| R1-F1.4 | CLI indentation discovery and publication on equal values | `frontmatter.rs` passes a lazy closure; the editor's equality return precedes layout. Counting-closure tests returned zero calls. CLI `an_unchanged_save_reports_that_nothing_was_saved` passed with `saved: false`; byte and modification-time checks passed. |
| R1-F1.5 | Lifecycle `set_frontmatter`, equal title | Public lifecycle test passed with exact authored bytes and unchanged modification time. Code returns before `save`, the destination lookup site. |
| R1-F1.6 | Lifecycle `merge_frontmatter`, existing title | Public lifecycle test passed with exact authored bytes and unchanged modification time; code calls `save` only if an incoming value differs. |
| R1-F1.7 | Lifecycle `delete_frontmatter`, absent property | Public lifecycle test passed with exact authored bytes and unchanged modification time; missing-key branch returns before `save`. |
| R1-F1.8 | Plain-YAML editor, equal authored scalar | Reran `yaml_edit::indentation_of_new_values`: seven cases passed, including unchanged-source preservation. Its equality guard precedes rendering. |
| R1-F1.9 | Transaction and Claudine, identical second save | Eight transaction indentation tests and nine Claudine persistence tests passed. The transaction control asserts zero resolver calls and `written: false`; Claudine's create/edit/repeated-save control preserves bytes. |
| R1-F1.10 | Hash save, unchanged stored baseline | Read `stamp_baseline_with_indentation`: the resolver is called only when `decision.new_stored.is_some()`. The CLI writes only `Some(stamp.text)`; unchanged content takes the no-write branch. This is an explicit code invariant. |
| R1-F1.11 | Claudine compatibility writer, no upgrade | `a_skill_needing_no_upgrade_is_left_unwritten` passed with authored quotes and comment intact; `classify.rs` retains its `if changed` write guard. |
| R1-F1.12 | Removal CLI and lifecycle numeric/list mutations | Read the callers: removal requires an existing target; numeric mutation changes the number or errors; append/prepend grows the collection or errors. These are actual mutations, rather than equal-value saves. Removal also uses lazy indentation. |
| R1-F1.13 | DMLS code actions | No Markdown save API is called; actions return text insertions for missing keys, so the equal-value file-save defect does not apply. |
| R1-F1.14 | Claudine generator and Schematic OpenAPI output | These generate whole documents from values, without authored source to preserve. Their deterministic regeneration contract is distinct from an edit to existing text. |

The lifecycle tests retain a changed-value control: adding a nested mapping
still writes with the destination's two-space preference. No-op protection
has not turned the mutation API into an unconditional no-op.

### Caller indentation: every required instance

| Instance | Site and input checked | Verification evidence and result |
| --- | --- | --- |
| R1-F2.1 | Source-less document, explicit two-space preference | `a_document_built_in_memory_is_serialized_with_the_callers_indentation` passed with two-space nesting and one resolver call. The implementation forwards the argument at the formerly failing branch. |
| R1-F2.2 | Source-present whole-frontmatter fallback | `control_flow_style_frontmatter_falls_back_to_the_callers_indentation` passed with two-space nesting and one call across refusal and fallback. |
| R1-F2.3 | Source-preserving replacement of an empty mapping | `control_an_empty_mapping_replaced_in_place_uses_the_callers_indentation` passed with two-space nesting and one call. |
| R1-F2.4 | Direct `Markdown::as_string_with_indentation` | Read its forwarding path; the public `a_not_yet_existing_destination_resolves_its_worktrees_preference` test passed with two-space nested output. |
| R1-F2.5 | Default wrappers | Default in-memory serialization and `output_with_no_destination_nests_four_spaces` both passed. Wrappers pass `Indentation::default`. |
| R1-F2.6 | Other indentation-bearing writer/editor callers | Read the YAML writer, node layout, transaction staging, hash save, compatibility writer, and Schematic export paths. Supplied preferences reach rendering; deliberate JSON/no-write branches need no YAML layout. Destination and fallback controls passed in the writer, transaction, and Claudine suites. |

### Test duration: every required instance

| Instance | Required coverage | Verification evidence and result |
| --- | --- | --- |
| R1-F3.1 | Generated documentation | Four replacement cases ran in the package run; all passed below five seconds. Reviewed the per-group documentation checks and crate-level case. |
| R1-F3.2 | Generated Rust artifact drift | Four replacement cases ran below five seconds. Group cases compare generated bytes; the crate inventory case checks missing/unregistered modules and split-directory contents. |
| R1-F3.3 | Generated REST runtime ownership | Four replacement cases ran below five seconds. Every selected module is scanned; each API client must occur once in its owning group and zero times elsewhere, including the crate-level control. |
| R1-F3.4 | Definition-bundle round trip and exports | Three replacement cases ran below five seconds. Each group's APIs still round-trip through files and compare both canonical JSON and exported OpenAPI/Postman results. |
| R1-F3.5 | Feature-test timing controls | All selected feature tests in this review stayed below five seconds; individual nextest `PASS` timings were checked, rather than relying solely on a recipe banner. |
| R1-F3.6 | Bounded group coverage guard | `every_rest_module_is_in_exactly_one_bounded_generation_group` ran in Schematic L1. It compares the grouped modules with registrations, rejects duplicates/omissions, and limits endpoints per group. The current three groups each have an explicit case in all four divided tests. |

The production performance changes also received review: component caches are
invalidated when registrations/compositions change, and registration errors
are checked before returning cached components. The new invalidation test
ran. Repeated nested schemas containing root-relative references are excluded
from the comparison shortcut. Module assembly reuses parsed pieces without
changing their order; shipped artifact drift tests passed. The faster export
gates omit only warning-producing work on success and retain full diagnostics
on error. Public validation still computes warnings.

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None. The previous defect classes are repaired. No unchanged instance,
repair regression, or missed sibling remains to carry forward.

## Spec coverage and verification level

The review covered the spec, formatter reference, previous review, both
implementation records, repaired code and tests, resolver configuration
loaders/validators, YAML rendering, destination-aware transaction staging,
writer callers, and affected documentation. Previously accepted implementation
choices remain accepted, including structure-based YAML rendering and the
bounded formatter-discovery policy.

| User-facing requirement | Appropriate level and evidence checked in this review |
| --- | --- |
| Default four-space YAML, tabs converted to spaces, compact collections, tags, non-string keys, multiline content, wide widths, mapping order, semantic equivalence | L1 public writer tests: `biscuit-file: just test yaml_writing::`, 21 passed; the recipe's minimal-feature follow-up also passed six cases. Reviewed layout and equivalence code. |
| Ranking, defaults, consensus, partial opinions, EditorConfig matching, formatter overrides, file-type constraints, discovery, invalid configuration diagnostics, single reads | L1 public resolver and output tests: `sniff: just test indentation::`, 160 passed. |
| CLI plain/JSON output, aggregate payload, extension errors, fallback outside a repository, loader wiring | L1 subprocess tests: `sniff: just test repo_indent::`, 22 passed. Direct built-CLI run in this checkout printed `4 spaces` from `.editorconfig` and `consensus: yes`. |
| Authored local indentation, comments, indentless sequences, multiline edits, CRLF, equal-value preservation | L1 public editor tests: `darkmatter: just test frontmatter_path_edit`, 58 passed; `just test yaml_edit::indentation_of_new_values`, seven passed. |
| Markdown destination preferences, lifecycle no-op saves, CLI no-op saves and reported saved status | L1 public API/filesystem and subprocess tests: `just test markdown_save_indentation`, nine passed; `just test unchanged_saves`, four passed. |
| Lazy resolution once per target, effective symlink destination, fallback warning, generated documents, validation/refused/raced creation | L1 public transaction tests: `just test config_write_indentation`, eight passed. |
| Claudine setup, legacy user/repository conversion, read-back, backup/retry, comments, repeated saves | L1 public persistence/setup tests: `claudine: just test config_indentation`, nine passed; compatibility no-upgrade control, one passed. |
| Schematic export behavior, exact numbers, artifact stability, repaired test duration, cache invalidation | L1 full Schematic package run: 3,192 passed, 23 excluded-tier tests skipped, no test at or above five seconds. Includes the YAML export and serializer guard cases. |
| Serializer restrictions and unchanged hashes/fingerprints | Reviewed retained no-write/hash guards and the earlier serializer-guard evidence. Schematic's guard executed again in the full L1 run. Existing hash/fingerprint encoding was not changed by the repair. |

These requirements concern text generation, file contents, and reported data.
The spec explicitly assigns CLI verification to L1 subprocess tests; it adds
no keyboard, input-encoder, interactive scrolling, or terminal-specific visual
requirement needing L2/L3. The relevant test modules are compiled by declared
targets and their selected L1 cases actually ran. Cross-OS evidence remains
CI work, as requested; it does not block readiness.

The full Schematic recipe also completed its separately declared Cargo-build
checks: 17 fresh-output cases passed in 173.105 s and the compile-fail case
passed in 1.218 s. Those existing `slow_` cases exercise generated crates
through Cargo outside the ordinary L1 timing gate; the repaired four test
groups remain in ordinary L1 and all stayed below five seconds. No test was
moved to that separate tier to satisfy the repair.

### Configuration input robustness

The matrix applies to Sniff's formatter readers. Invalid fields are diagnosed
with their file and key, ignored, and resolved using remaining applicable
opinions or formatter defaults. Invalid documents are diagnosed and skipped
so the next candidate can apply. Absence takes formatter defaults; an empty
declaring mapping is valid. Empty/comment-only YAML declares defaults, while
explicit null settings are invalid and diagnosed. This preserves the spec's
best-effort lookup contract rather than imposing a new fail-all policy.

Reviewed the shared width, boolean, style, mapping, glob, and override
validators and their public-API tests. The suite exercises absent values,
explicit null, wrong types, invalid collection elements, and applicable empty
forms, with positive controls and per-field wiring cases. Strict JSON,
JSON5, TOML, single-document YAML, and YAML streams have loader cases for
duplicates, trailing content, and format-specific syntax. CLI tests prove
loader/diagnostic wiring. All executed successfully in the resolver and CLI
runs above.

The requested code-smell search found no additional defect: optional
serialized result fields are output data, not configuration validators;
width conversion's `.ok()` is behind an integer range check; missing target
file types are deliberate; swift-format validates its mapping before reading
entries. YAML's non-string mapping keys are deliberately excluded from the
string-named setting lookup. No silent filtering of a load-bearing list was
found. The rendering and Markdown text paths do not consume configuration
fields and do not need this matrix.

### Implementation records and limits

No newly verified false behavior claim required an in-place record correction.
The original implementation record's failures describe its earlier runs; the
repair log records their subsequent resolution. They are historical evidence,
not a claim that those failures persist. The new Schematic L1 run independently
confirms the repaired timing result. This review did not repeat the full
Darkmatter/Claudine suites or their lint recipes; the repair records those
passing gates, and this review reran the affected public paths above.
