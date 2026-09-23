---
source_files_during_phase_1:
  - darkmatter/lib/tests/dasherized_identifier_corpus.rs
  - prompts/_reviews/review-spec-inline.md
  - prompts/_docs.md
  - prompts/documentation.md
  - prompts/_add/add-expressions.md
docs_updated_during_phase_1:
  - darkmatter/features/2026-09-15-dasherized-identifiers/spec.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
docs_created_during_phase_1:
  - darkmatter/features/2026-09-15-dasherized-identifiers/rulings.md
skills_files_updated_during_phase_1:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/tests/dasherized_identifier_corpus.rs
  - darkmatter/lib/tests/dasherized_identifier_compose.rs
  - darkmatter/cli/tests/compose_dasherized_identifiers.rs
docs_updated_during_phase_2:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/tests/compose_diagnostic_identity.rs
  - darkmatter/cli/tests/compose_diagnostic_identity.rs
docs_updated_during_phase_3:
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/compose/tests/rendering.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/tests/expression_regression.rs
  - darkmatter/lib/tests/context_functions.rs
  - darkmatter/lib/tests/nested_composition.rs
  - darkmatter/lib/tests/compose_diagnostic_identity.rs
  - darkmatter/lib/tests/compose_expression_failure_contract.rs
  - darkmatter/lib/tests/feature_review_incident.rs
  - darkmatter/cli/tests/compose_diagnostic_identity.rs
  - darkmatter/cli/tests/compose_terminal_detection.rs
  - darkmatter/cli/tests/compose_expression_failures.rs
docs_updated_during_phase_4:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_5:
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/tests/unknown_identifier_warning.rs
  - darkmatter/lib/tests/feature_review_incident.rs
  - darkmatter/cli/tests/compose_unknown_identifiers.rs
docs_updated_during_phase_5:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_6:
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_6:
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/dmls/docs/features.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/darkmatter/SKILL.md
  - .claude/skills/darkmatter/dmls.md
source_files_during_phase_7:
  - darkmatter/lib/src/markdown/compose/subtree.rs
docs_updated_during_phase_7:
  - darkmatter/docs/topics/parsing/index.md
  - darkmatter/docs/topics/parsing/lexing.md
  - darkmatter/docs/topics/parsing/grammar.md
  - darkmatter/docs/topics/parsing/scanning.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/lsp/features.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/darkmatter/compose.md
source_files_during_phase_8: []
docs_updated_during_phase_8:
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
docs_created_during_phase_8: []
skills_files_updated_during_phase_8:
  - .claude/skills/os/build-hosts.md
source_code:
  - darkmatter/lib/tests/dasherized_identifier_corpus.rs
  - prompts/_reviews/review-spec-inline.md
  - prompts/_docs.md
  - prompts/documentation.md
  - prompts/_add/add-expressions.md
  - darkmatter/lib/src/markdown/compose/expression/lexer.rs
  - darkmatter/lib/src/markdown/compose/expression/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/parser.rs
  - darkmatter/dmls/src/overlay/expressions.rs
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/tests/dasherized_identifier_compose.rs
  - darkmatter/cli/tests/compose_dasherized_identifiers.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
  - darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
  - darkmatter/lib/tests/compose_diagnostic_identity.rs
  - darkmatter/cli/tests/compose_diagnostic_identity.rs
  - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
  - darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
  - darkmatter/lib/src/markdown/compose/directive_targets.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
  - darkmatter/lib/src/markdown/compose/subtree.rs
  - darkmatter/lib/src/markdown/compose/preflight/collect.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/errors/blocks.rs
  - darkmatter/lib/src/markdown/compose/tests/rendering.rs
  - darkmatter/lib/src/markdown/compose/tests/provider_network.rs
  - darkmatter/lib/tests/expression_regression.rs
  - darkmatter/lib/tests/context_functions.rs
  - darkmatter/lib/tests/nested_composition.rs
  - darkmatter/lib/tests/compose_expression_failure_contract.rs
  - darkmatter/lib/tests/feature_review_incident.rs
  - darkmatter/cli/tests/compose_terminal_detection.rs
  - darkmatter/cli/tests/compose_expression_failures.rs
  - darkmatter/lib/src/markdown/compose/expression/absence.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/mod.rs
  - darkmatter/lib/src/markdown/compose/expression/functions/predicates.rs
  - darkmatter/lib/src/markdown/compose/context/effective_state.rs
  - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
  - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
  - darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
  - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
  - darkmatter/lib/src/markdown/compose/conditions.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/unknown_identifiers.rs
  - darkmatter/lib/src/markdown/compose/mod.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/tests/unknown_identifier_warning.rs
  - darkmatter/cli/tests/compose_unknown_identifiers.rs
  - darkmatter/dmls/src/providers/dsl.rs
  - darkmatter/dmls/src/providers/code_actions.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
documentation:
  - darkmatter/features/2026-09-15-dasherized-identifiers/spec.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/plan.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/rulings.md
  - darkmatter/features/2026-09-15-dasherized-identifiers/implementation-log.md
  - darkmatter/docs/inline/interpolation.md
  - darkmatter/dmls/docs/diagnostics.md
  - darkmatter/dmls/docs/features.md
  - darkmatter/docs/topics/parsing/index.md
  - darkmatter/docs/topics/parsing/lexing.md
  - darkmatter/docs/topics/parsing/grammar.md
  - darkmatter/docs/topics/parsing/scanning.md
  - darkmatter/docs/topics/darkmatter-expressions.md
  - darkmatter/docs/topics/context-variables.md
  - darkmatter/docs/inline/fm-interpolation.md
  - darkmatter/docs/lsp/features.md
completed_phase: "8"
implemented: true
packages:
  - darkmatter
human_review: false
message_to_agent: >-
  Phase 8 was the final phase: verification only, no source changes. Status:
  implementation complete, ready for review; the feature was not moved to
  _completed. Darkmatter L1 8269/0/14, lint green, L2 green on re-run (one
  cold-WezTerm harness spawn failure, unrelated). Claudine: 7032 passed, 4
  failed. All four are caused by an unrelated uncommitted edit to
  prompts/_implement/implement-plan.md that turns positional `set:` actions
  into mappings, which claudine's action_shape.rs rejects
  (LifecycleObjectDataThroughInterpolationPositional) and the shipped-prompt
  hash pin flags. With the committed prompt, all four pass alongside every
  Darkmatter change. The owner of that prompt edit must resolve it. No remote
  OS evidence was produced: build-win-native's W: disk is full, build-linux's
  cross-check lock has been held by nightly-reward-spike since 2026-09-14, and
  WSL ssh resets. Cross-OS risk was reviewed by reading the tests (sorted
  walk, canonicalize parity, Path::ends_with), so CI is the first multi-OS
  evidence.
implementation_1: "2026-09-18T09:20:26-07:00"
implementation_2: "2026-09-18T14:01:25-07:00"
implementation_4: "2026-09-18T18:43:39-07:00"
implementation_5: "2026-09-18T20:23:59-07:00"
---

# Dasherized Identifiers — Implementation Log

## Phase 1

Phase 1 produced rulings, spike findings, one new permanent test, and fixes
to four shipped prompts. It changed no lexer, evaluator, or DMLS code. The
full record is in [rulings.md](./rulings.md); this log keeps the facts that
matter operationally.

### Rulings (all ten recorded in `rulings.md`)

- **R-1 corrects the plan.** `FunctionBinding` *does* exist and already
  registers aliases (`is_null`/`isnull`, `is_empty`/`isempty`). The
  absence-predicate rule therefore resolves through that registry: add a
  typed marker on the binding and one `is_absence_predicate(name)` resolver.
- **R-2:** write a new shared classifier. `collect_variable_roots` and
  `ctx.*` warnings stay unchanged.
- **R-3:**
  - Strict: the body, frontmatter mixed text, shell expansion (all three
    sites already pass `true`), and directive targets.
  - Lenient: preflight's best-effort frontmatter pass (approval discovery)
    and subtree.
- **R-4:** a typed `ExpressionFailurePolicy { Lenient, Strict }` replaces
  `fail_fast: bool` on the `pub(crate)` helper.
- **R-5:** fold candidates per unit on `ComposeReport`. There is no lock on
  `ComposeOptions`.
- **R-6:** update the fatality matrix in place and give it a policy axis.
- **R-7:** walk the four directories recursively, covering `.md` and `.yaml`.
- **R-8:** carry the quick-fix replacement in `Diagnostic.data`, with a
  range-plus-reparse fallback.
- **R-9:** every body-stage lookup answers "known" today. Phase 5 must
  override `is_known_variable_root` on `EffectiveState`. The effective schema
  is applied at reconciliation.
- **R-10:** raising the severity changes one unit test and one doc row.

### S-1: the corpus audit and collisions

The new test is `darkmatter/lib/tests/dasherized_identifier_corpus.rs`,
with 9 tests: 2 corpus gates, 1 coverage guard, 1 schema pin, and 5
classifier/extractor tests.

- **Gate A** (`shipped_corpus_has_no_unspaced_identifier_subtraction`) was red
  on the first run. It found one real breaking-form use:
  `::block when="depends-on"` in `prompts/_reviews/review-spec-inline.md`.
  - Probing with `md compose` showed it **already fails** ("Subtraction
    requires numeric operands", exit 1) whenever it is reached. The author
    meant the spec's `depends-on` key.
  - The pre-approved `x - 1` rewrite would have preserved a guaranteed
    failure, so I fixed it to the evident intent instead:
    `when='frontmatter(spec, "depends-on")'`.
  - The sibling `when="parent"` / `when="peers"` blocks had the same mistake
    (they rendered silently empty) and got the same fix.
- **Gate B** (`shipped_corpus_expressions_all_parse`) found three prompts
  that already fail. I fixed each at the syntax level:
  - `_docs.md`: a missing opening quote in `when=`.
  - `documentation.md`: `{{doc.doc}}` inside `when=`, which a condition never
    interpolates.
  - `_add/add-expressions.md`: a `{{ … }}` in the frontmatter description.
    It is now the literal `{{{ … }}}`.
- **Gate B is enabled now, not deferred to Phase 2.** It passes on both
  grammars, and keeping it on catches regressions earlier.
- **Surfaces covered:**
  - Body `{{ }}` and `when=` options.
  - Frontmatter mixed text, recursively.
  - Claudine lifecycle and loop conditions (`when`, `while`, `until`),
    pinned against `claudine-types.yaml`.
  - `$()` ternary conditions and value branches.
- **Excluded by construction:** `{{{ }}}`, fenced examples, and `${{ }}`.

### S-2 and S-4: read-only spikes

Both were read-only code investigations; the findings are in `rulings.md`.

- **S-2** rejects an `Arc<Mutex>` accumulator on `ComposeOptions` for two
  reasons:
  - It would put a cross-document lock on the rayon path.
  - It would lose the candidates of the second parent on a single-flight
    cache hit (`engine.rs:1486` replays `cached.report`).
- **S-4** chooses one lexer-owned `identifier_prefix_start` helper shared by
  both DMLS completion scans. Hover and go-to-definition already use AST
  spans.

### S-3: fatality blast radius

The probe made the lenient branches fatal. It was reverted by restoring
`rewrite.rs` from a byte copy (`cmp` verified).

- Darkmatter: 26 failures caused by the probe.
  - 22 are lenient-asserting tests to invert.
  - 3 are fixtures that use broken expressions as scaffolding:
    `nested_composition::self_composition_reaches_the_depth_limit` and two
    `cli/tests/compose_terminal_detection.rs` tests.
  - 1 is the subtree-lenient pin that must stay green.
- Claudine: 0 failures caused by the probe.
- No failure needs the lexer fix first.

### Gates run

| Gate | Result |
| --- | --- |
| `cargo nextest run -p darkmatter --test dasherized_identifier_corpus` | 9/9 pass |
| `just test` (darkmatter), final state | 8127 run, 0 failed, 29 timed out |
| Re-run of those 29 at `--test-threads 2` (`persistent_cache_tests` + `provider_network`, 55 tests) | 55/55 pass. The timeouts are the documented load cluster; load average was 17–37 |
| `just lint` (darkmatter, clippy `--all-targets`, zed-dmls wasm) | pass |
| `just test` (claudine), final state | 7036 run, 4 failed. **Pre-existing:** all four come from the uncommitted, pre-session edit to `prompts/_implement/implement-plan.md`, and fail identically with and without the probe |

### OS considerations

The new test only reads files and runs pure lexer/parser/extractor calls.
Paths are built with `Path::join`, and no process or terminal is involved.
Line endings: frontmatter and directive scanning already handle CRLF, and the
breaking-form check reads byte spans from `lex_spanned`. Risk is low, so no
cross-OS run was made. CI covers it.

### Open items passed forward

- `prompts/documentation.md` calls `file_empty(...)`, which is not a catalog
  function. The block still fails at evaluation when it is reached. The
  intent is unknown, so I left it for the author. This is not a
  dasherized-identifier issue.
- **Phase 4 hazard:** a `{{{ … }}}` literal in a frontmatter value that is
  interpolated into the body gets re-scanned as `{{ … }}` on the rescan
  pass. That is a warning today and would become fatal under Strict.
- **Phase 7:** `claudine/docs/topics/composition.md:430` and
  `.claude/skills/claudine/composition.md:545` use `{{review-file}}` as a
  motivating error. Under R1 that no longer errors.

## Phase 2

Phase 2 changed the grammar: `-` now continues an identifier. It also brought
the DMLS cursor scans into line with the lexer in the same change, as the spec
requires. Every task in Work-groups 2A and 2B is done.

### What changed

- **Lexer (2.1).** A new `Lexer::dash_continues_identifier` returns true when
  the current character is `-` and the next one satisfies
  `is_identifier_char`. Both `read_variable` loops call it: the leading name
  and each dotted segment.
  - Being inside `read_variable` is what makes the scan mid-identifier.
    `read_number` never calls the check, so `4-2` stays subtraction.
  - The post-scan `true`/`false` reclassification is untouched, which is why
    `false-1` and `true-value` join.
  - `is_identifier_char` is unchanged, per Resolved Decision 1.
- **Parser (2.2).** No code change was needed. The unquoted-object-key guard
  (`parser.rs:592`) accepts `{ spec-name: 1 }`, and new tests pin that.
  - A key the guard rejects still needs quotes: a dotted key, a non-ASCII
    first character, a leading digit, or `foo--bar`.
  - `{ spec-name: 1, "spec-name": 2 }` is reported as a duplicate key.
- **DMLS cursor scans (2.3).** Per S-4, a new pure public helper
  `expression::identifier_prefix_start(prefix)` sits next to
  `read_variable`. `completion_partial` and `value_completion_partial` both
  call it in place of the old backward scan over `[alnum _ .]`.
  - It does a backward candidate scan, then a forward walk that tracks three
    states: identifier, number, or none.
  - A trailing `-` or `.` directly after an identifier joins, for
    completion filtering only (`spec-`, `ctx.`).
  - The old `rfind(..) + 1` could slice inside a multibyte non-identifier
    character. The helper uses `char_indices`, and
    `multibyte_separator_does_not_split_a_char_boundary` covers it.
  - Hover and go-to-definition already worked from AST spans. They follow
    the lexer automatically; the new tests prove it.
- **`--set` keys.** No change needed, as the plan expected. Option keys
  already accept `-` (`parse_utils.rs:348`), and `--set`/`set.NAME` keys
  are unvalidated strings.
- **Corpus gate (2.7).** Gate A is retired, per the Phase 1 message and
  Resolved Decision 12.
  - `breaking_form_offsets` became `dashed_variables`.
  - The two classifier tests were restated:
    `former_breaking_forms_now_lex_as_one_variable` (same inputs, now one
    `Variable` and no `Minus`) and `subtraction_and_literal_dashes_never_join`.
  - The ternary-extraction test's final loop now asserts that each fixture
    contains a joined identifier.
  - Gate B took over the corpus-size guard (≥ 50 files, ≥ 100 expressions).
    It passes on the new grammar.

### Requirement-to-test mapping

| Behavior | Test(s) |
| --- | --- |
| Every row of the R1 behavior table, as a token stream | `lexer::tests::dash_continuation::*` (11 tests): kebab names, dotted kebab segment, breaking rows (`iteration-1`, `false-1`, `true-value`), `a - b`/`a -b`/`a- b`, `4-2`/`f(x)-1`/`arr[0]-1`/`(a)-1`/`"x"-1`, unary minus, `foo--bar`, trailing dash (end of input and before whitespace), `café-name`, full-name spans, condition mode |
| Parser lowering | `parser::tests::dasherized_identifiers::*` (8 tests): one `Variable` spanning the whole name; every subtraction form still `Binary Sub`; `foo--bar` = `foo - (-bar)`; `spec-` is a parse error; kebab as operand and argument; unquoted and quoted object keys; guard rejections |
| Cursor helper | `lexer::tests::identifier_prefix::*` (7 tests), including agreement with `lex_spanned` on complete names |
| SC1, three spellings resolve identically | `lib/tests/dasherized_identifier_compose.rs` (7 tests): bare/`doc.`/`doc['']`; native vs quoted YAML key; string, number, and boolean values; frontmatter mixed text; `when=` condition; object-literal key; `_loop_count - 1`, `4-2`, `length(items)-1`, `items[0]-1`, `(phase)-1`, `phase - 2` vs `phase-2` |
| Round trip | `kebab_key_resolves_in_frontmatter_mixed_text_and_survives_a_round_trip`: compose, `as_string`, compose again; the frontmatter values and the serialized document are stable |
| Normal invocation path (`md compose`) | `cli/tests/compose_dasherized_identifiers.rs` (3 tests): stdin kebab document with `--frontmatter`; `--set '{"spec-name":…}'` overrides every spelling; the shipped `prompts/_implement/review-findings-plan.md` (dashes in string literals) still composes to `review-plan-2.md`, with no `{{` left |
| Passive corpus over all shipped artifacts | `lib/tests/dasherized_identifier_corpus.rs` gate B (`shipped_corpus_expressions_all_parse`) |
| SC7 navigation, every cursor position including the dash | `dmls/tests/lsp_session.rs::kebab_identifier_navigation_resolves_at_every_cursor_position` (protocol level): hover, definition, and completion `textEdit` start at all 10 columns of `spec-name`; negatives for `a - b` and `foo--bar` (separate hovers, separate completion starts, no definition jump) |
| Both scanners and both hover dialects | `overlay::expressions::tests::dasherized_identifiers::*` (6 tests) |

Each positive kebab test would fail on the old lexer, because `spec-name`
lexed as `spec`, `Minus`, `name`.

### Gates run

| Gate | Result |
| --- | --- |
| Targeted lexer/parser filter (`--lib`) | 490/490 pass |
| `dasherized_identifier_corpus` | 8/8 pass |
| `dasherized_identifier_compose` | 7/7 pass |
| CLI `compose_dasherized_identifiers` + `spawn_site_guard` | 26/26 pass |
| DMLS overlay unit filter + `lsp_session` kebab test | 8/8 and 1/1 pass |
| `just test` (darkmatter) | **8169/8169 pass**, 14 skipped, no timeouts (load average about 13) |
| `just lint` (darkmatter: clippy `--all-targets`, zed-dmls wasm) | pass |
| `just test --no-fail-fast` (claudine) | 7032/7036. The **4 failures are pre-existing** and match Phase 1 exactly: `shipped_prompt_route_drift`, `shipped_implement_plan_launches_without_a_sibling_spec`, and two `shipped_implement_plan_*` schema tests. All four come from the uncommitted edit to `prompts/_implement/implement-plan.md` (fixture hash drift and `LifecycleObjectDataThroughInterpolationPositional` on `initialize`), not from the lexer. |

Nothing was skipped. I did not run a cross-OS check. Every change is a pure
string or char operation with no filesystem, path, process, or terminal
behavior. The CLI tests assert only OS-neutral file-name substrings, never
full paths, and reuse the `CliProcessFixture` pattern that
`compose_schema.rs` already runs on every CI leg.

### Observations

- **Silent breaking row.** `{{ iteration-1 }}` with only `iteration` defined
  now renders empty and warns nothing until Phase 5 adds the
  unknown-identifier warning. It is pinned by
  `unspaced_identifier_subtraction_now_names_a_kebab_key`, which Phase 5
  should extend with the warning assertion.
- **No shipped kebab identifiers yet.** No shipped prompt uses a kebab
  identifier. Every `-` inside a shipped `{{ }}` sits in a string literal.
- **Skill update.** `.claude/skills/darkmatter/SKILL.md` gained one
  paragraph: where the joined-dash rule lives, and that any cursor-side scan
  must call `identifier_prefix_start` rather than widen a character class.
- **Spec frontmatter.** `implemented: true` and
  `implemented_by: claude/opus` were already set, so there was nothing to
  change.


## Phase 3

Phase 3 implements Requirement 5: source-aware diagnostic identity and
deduplication, so each issue is reported once. Work-groups 3A and 3B are done.

### What changed

- **Typed identity (3.1).** `ComposeWarning` has a new crate-private field,
  `identity: Option<WarningIdentity>`. Only a family's constructor sets it.
  - `WarningIdentity { subject: WarningSubject }` is keyed together with the
    warning's existing `source` and `code` fields.
  - `WarningSubject::Path(path)` is the schema-advisory family, set in
    `from_schema_advisory`. Its key is exactly today's `(source, code, path)`.
  - `WarningSubject::Root { document, name }` is the unresolved-root family.
    Today that family is only the `ctx.*` typo warning, now built by
    `ComposeWarning::unknown_context_variable` with code
    `dm.expression.unknown_context_variable`. The name is `ctx.<group>`, so
    `ctx.toady` and `ctx.toady.deeper` are one issue. Phase 5's
    `dm.expression.unknown_identifier` should add a sibling constructor that
    returns the same `Root` subject.
  - `WarningSubject::Expression { document, scope, origin }` is the
    expression-failure family, built by `ComposeWarning::expression_failure`
    with codes `dm.expression.parse_failure` and
    `dm.expression.evaluation_failure`.
    - `origin` is `ExpressionOrigin::Authored(span)` for a span in the text
      the caller supplied.
    - It is `Generated { pass, span }` for an expression that a replacement
      produced, taken at the first pass where the rescan observes it. The
      separate variant means a generated origin never aliases an authored
      one.
  - **Scope (a decision beyond the plan's key).** A document's frontmatter is
    scanned once per string value. A bare `(document, code, span)` key would
    therefore alias failures at the same offsets in two keys. `scope` names
    the scanned text: `rewrite_value` threads the key path (`key`,
    `key.child`, `key[0]`) and tags warnings with `in_scope`. Body scans leave
    `scope` as `None`.
  - Codes are not rendered by the CLI, so adding them changes no stderr text.
- **Rescan tracking (3.1).** `interpolate_text_located` keeps a `reported`
  list of `(current range in output, origin)`.
  - A location whose range matches an entry is skipped: it is not evaluated
    again and not reported again.
  - `shift_reported` moves the ranges after each replacement. Replacements go
    end to start and never overlap, so a range is always wholly before or
    wholly after the replaced span.
  - This is the mechanism behind the duplicate: a failing span stays in the
    output, a sibling replacement forces another pass, and that pass rescans
    the failure.
  - Warnings within a pass are now emitted in **document order**. Locations
    are still visited end to start, and the pass buffer is reversed
    afterwards. Without that, "keep the first authored location" would keep
    the last one.
- **One dedup mechanism (3.2).**
  - `ComposeReport::add_warning` drops a warning whose identity key matches
    one already in the report. `merge` routes through the same path through
    `add_warnings`, and `add_schema_advisory` now just calls `add_warning`.
  - `same_schema_advisory` and `is_schema_advisory` are deleted. Warnings
    without an identity are never collapsed, and
    `compose_report_merge_keeps_duplicate_non_schema_warnings` still passes
    unchanged.
  - **Document attribution.** `run_compose_pipeline_node` calls
    `report.attribute_to_document(source_path(self, &options))` before
    returning. It stamps the document onto every `Root`/`Expression` identity
    that has no document yet, then re-dedups.
    - A child's report is stamped before the transclusion engine merges it,
      so the same root in `a.md` and `b.md` stays two warnings.
    - A cached replay of `a.md` carries the same stamp and collapses.
    - `Path` subjects are never document-scoped. A schema advisory still
      collapses across consumers, as before.
  - The stage sites that received these families now call `add_warnings`
    instead of `warnings.extend`. Those are the body interpolation targets and
    result, both frontmatter interpolation passes, and frontmatter shell
    expansion. Page-block and transclusion `when=` already used
    `add_warning`.

### Requirement-to-test mapping

| Behavior | Test(s) |
| --- | --- |
| Original two-span rescan fixture: one replacement plus one bad expression gives exactly one issue (parse and eval variants) | `rewrite::tests::rescan_identity::one_replacement_and_one_parse_failure_report_exactly_one_issue`, `…_evaluation_failure_…`; compose level `lib/tests/compose_diagnostic_identity.rs::one_replacement_and_one_bad_body_expression_report_exactly_one_issue`; binary `cli/tests/compose_diagnostic_identity.rs::compose_reports_a_bad_expression_beside_a_replacement_once` |
| Tracked ranges shift across a replacement that changes length; document order | `rescan_identity::failures_around_a_length_changing_replacement_stay_one_each_in_document_order` |
| A generated expression is reported once and never aliases an authored one | `rescan_identity::a_generated_failure_is_reported_once_and_distinct_from_an_authored_one`; `report::tests::a_generated_origin_never_aliases_an_authored_one_at_the_same_offsets` |
| One unknown root referenced ten times gives one warning | `rescan_identity::ten_unknown_context_references_share_one_identity`; `compose_diagnostic_identity::one_unknown_root_referenced_ten_times_warns_once` (asserts all ten still render) |
| Frontmatter and body of one document are one issue, with the first authored location kept | `one_unknown_root_in_frontmatter_and_body_of_one_document_warns_once` (the kept warning is `key 'label'`) |
| The same root in two transcluded documents gives two; one of them transcluded twice is still one | `one_root_in_two_transcluded_documents_warns_once_per_document`; binary `compose_reports_an_unknown_root_once_per_source_document` (root plus `a.md` twice plus `b.md` gives exactly 3 on stderr) |
| Two different roots give two, in document order | `two_different_unknown_roots_warn_twice` |
| Identity is not the message, in both directions | `report::tests::identical_messages_with_different_identities_stay_distinct`, `the_add_path_keeps_the_first_of_two_same_issue_warnings` |
| Negative: same offsets in two frontmatter keys do not alias | `identical_bad_expressions_in_two_frontmatter_keys_warn_once_each`; `report::tests::expression_failures_in_different_scopes_do_not_alias` |
| Add path, merge path, and attribution | `report::tests::merge_collapses_one_documents_repeats_but_not_two_documents_issues`, `attribution_keeps_child_documents_and_collapses_newly_equal_repeats`, `scoping_leaves_root_identities_per_document`, `warnings_without_an_identity_are_never_collapsed` |
| Schema-advisory dedup unchanged | Existing pins pass unedited: `compose/tests/frontmatter.rs::schema_advisory_is_not_duplicated_when_transclusion_reports_merge`, `transcluded_schema_advisory_uses_root_document_as_consumer`, `compose_report_merge_keeps_duplicate_non_schema_warnings`, `bare_sidecar_*`. The new `report::tests::schema_advisory_identity_is_source_code_and_path` pins the triple: consumer and message excluded |

**Proof the regressions fail on the old behavior.** Each proof disabled one
fix temporarily. Each file was restored from a byte copy, with `cmp`
verified.

- Disabling the rescan skip turned 4 of 5 `rescan_identity` tests red. The
  fifth, the ctx test, is owned by the report dedup.
- Disabling the `add_warning` dedup turned 4 of 6 compose-level tests red.
  Of the other two, one is the rescan fixture, which the skip owns, and one
  is the negative no-alias test.

There is no persisted value in this phase, so no round trip applies.
Warnings are stderr-only and are never written. The only shipped-artifact
surface is the expression grammar, which this phase did not change. The
Phase 1/2 corpus gate stayed green inside `just test`.

### Gates run

| Gate | Result |
| --- | --- |
| Targeted lib units (`rescan_identity`, `context::report::tests`) | 14/14 pass |
| `lib/tests/compose_diagnostic_identity.rs` | 6/6 pass |
| CLI `compose_diagnostic_identity` plus `spawn_site_guard` | 25/25 pass |
| `just test` (darkmatter) | **8191/8191 pass**, 14 skipped, no timeouts (load average about 5) |
| `just lint` (darkmatter: clippy `--all-targets`, zed-dmls wasm) | pass |
| `just test --no-fail-fast` (claudine) | 7032/7036. The **4 failures are pre-existing** and identical to Phases 1 and 2: `shipped_prompt_route_drift`, `shipped_implement_plan_launches_without_a_sibling_spec`, and two `shipped_implement_plan_*` schema tests. All four come from the uncommitted `prompts/_implement/implement-plan.md` edit |
| GitNexus `detect-changes --scope all` | **Not a clean check.** The worktree carries about 525 files of unrelated uncommitted work (Claudine changes and a benchmarks move), so the report is `critical` for the whole tree and lists 15 of 662 symbols ("… and 647 more"). This phase's symbols cannot be isolated from it. Pre-edit `impact` was run: `ComposeReport::merge` LOW, `add_schema_advisory` LOW, `interpolate_text_located` MEDIUM, `collect_context_warnings` MEDIUM, and `add_warning` **CRITICAL** (50 upstream callers). The mitigation for `add_warning` is that it dedups only warnings carrying an identity. Every other warning, which is every one those callers build with `ComposeWarning::new`, is added exactly as before |

No cross-OS run was made. The change is in-memory bookkeeping: byte ranges,
`PathBuf` equality on paths computed once by `source_path`, and string keys.
It adds no filesystem, process, terminal, or path-spelling behavior. The
tests build paths with `Path::join` and `CliProcessFixture::write_file`, and
assert only warning counts and fixed body lines.

### Observations

- **Warnings are now in document order within a pass.** They used to be in
  reverse order, a side effect of end-to-start replacement. No existing test
  depended on the old order.
- **Two same-message stderr lines are now possible and correct**, for
  example one per transcluded child. They look identical because
  `ComposeWarning` rendering shows neither `path` nor the source document.
  Phase 5's unknown-identifier warning is specified with `source`, `path`,
  and `line_number` set. It should also get the document into the rendered
  line, or the two warnings are hard to act on.
- **Document identity is `source_path(markdown, options)`.** It is a
  `PathBuf` as spelled by the loader, not canonicalized. Two spellings of
  one child that are not served from the run-local cache would count as two
  documents. This did not arise in any test.
- `ComposeWarning` gained a crate-private field. The only struct-literal
  constructions outside `report.rs` are in-crate (`remote_fetch.rs` uses
  `..ComposeWarning::new`). Claudine uses only `ComposeWarning::new` and
  field reads, and it compiles and runs unchanged.
- The spec frontmatter already had `implemented: true` and
  `implemented_by: claude/opus`, so nothing changed there.

## Phase 4

Phase 4 implements Requirement 2: a body or mixed-frontmatter expression
that cannot be parsed or evaluated fails full-document composition,
whatever `fail_fast` says.

### Pre-edit impact (GitNexus, upstream)

- `interpolate_text` **HIGH** (47 impacted). `interpolate_frontmatter`
  **CRITICAL** (45; every composition goes through it).
- `interpolate_text_located`, `interpolate_value`, and `rewrite_value` are
  MEDIUM. `rewrite_directive_targets` and `compose_string` are LOW.
- Mitigation: this change is intentional (R2). Each call site names its
  policy explicitly, the lenient subtree contract is pinned by an unchanged
  regression, and the full darkmatter and Claudine suites run.

### What changed

- **Typed policy (4.1, R-4).** `rewrite.rs` has a new
  `ExpressionFailurePolicy { Lenient, Strict }`. It replaces `fail_fast: bool`
  on `interpolate_text`, `interpolate_text_located`, and `interpolate_value`.
  `Strict` behaves exactly like the old `fail_fast = true`: every parse or
  evaluation failure aborts, including one found by a rescan. `Lenient` is the
  old `false` path. Each call site names its policy:

  | Site | Policy |
  | --- | --- |
  | Body stage (`inline/interpolation.rs`) | `options.expression_failure_policy()`, which is `Strict` except in discovery |
  | Directive targets (`directive_targets.rs`) | `Strict`; its `fail_fast` parameter is gone |
  | Frontmatter pass (`interpolate_frontmatter`, both pipeline passes) | `options.expression_failure_policy()`, now an explicit parameter replacing `fail_fast` |
  | Preflight best-effort frontmatter (`interpolate_frontmatter_best_effort`) | `Lenient`, hard-coded; its `fail_fast` parameter is gone |
  | Shell ternaries (`frontmatter_shell_expansion.rs`, three sites) | `Strict`; they already passed `true` |
  | Subtree (`subtree.rs`) | Mapped from `SubtreeStrictness`, unchanged |

  `ComposeOptions::fail_fast` no longer reaches expressions. It still governs
  TOC linking and non-structural transclusion.
- **A departure from R-3: the discovery pass stays lenient.** Preflight
  discovery does more than call `interpolate_frontmatter_best_effort`. It also
  runs a real `compose_with` pass (frontmatter interpolation, text
  replacement, body interpolation) **without page blocks**, so it is
  condition-blind.
  - Made strict, it rejected a document whose bad expression sits inside a
    `::block when="false"` region that the real compose removes. Discovery
    would then have been stricter than execution. `md compose` runs preflight
    first, so this was user-visible.
  - Fix: a crate-private `ComposeOptions::defer_expression_failures`, the
    sibling of `defer_missing_runtime_context`. Only
    `preflight/collect.rs` sets it. It is encoded into the compose cache key.
    `expression_failure_policy()` reads it.
  - The terminal pass is always strict, so a real failure still stops the
    run, with its own error.
- **Rescan hazard (S-3), decided: strict.** A failure the rescan finds in
  replacement output is fatal under `Strict`. It has no authored line, so the
  error names only the file.
  - Why: keeping rescan failures lenient would have changed the three shell
    ternary sites, which have always been fatal there. The rescan also already
    *executes* literal-derived text: `description: "{{{ name }}}"` rendered
    through `{{ description }}` prints the value of `name`, not `{{ name }}`
    (verified with `md`).
  - A malformed literal-derived span failing is therefore consistent with the
    existing semantics, not a new class of surprise.
  - The root bug (a rescan re-executing `{{{ … }}}` output) predates this
    feature and is out of scope. It is recorded for follow-up.
  - Pinned by
    `compose_expression_failure_contract::a_failure_in_replacement_output_is_fatal_without_an_authored_line`.
- **Typed error rendering (4.2).** The body stage already projected the
  scanner span through `anchor_authored_failure`, giving `OnDiskLine` for an
  authored span and `OnDisk` otherwise. But `interpolation_block`'s catch-all
  arm (parse, arity, arg-type, and anything else that is not a file reference
  or missing context) ignored `source`.
  - Every error that Phase 4 made fatal therefore printed with **no file and
    no line**. This was found by running `md compose`.
  - Fix: a shared `push_on_disk_locus` renders the link and focused YAML
    excerpt (keyed) or the authored line and numbered excerpt (body) for every
    cause. The file-reference arm now uses it too.
  - No partial output: `run_stage` returns before `content_mut()` is
    assigned, and the CLI prints nothing on stdout (asserted).
- **Matrix (4.3, R-6).** `fatality_characterization.rs` now runs on the
  policy axis, with the `//!` policy prose rewritten. The new
  `document_body_is_strict_whatever_fail_fast_says` composes a real document
  under both `fail_fast` values, pinning that the document entry passes
  `Strict`, not only that the helper honors it.
- **Docs touched because behavior changed** (authoring discipline):
  - The `with_fail_fast` and `fail_fast` field docs state the narrowing.
  - The `ComposeWarning` doc no longer says "when `fail_fast = false`".
  - The doc comments on `interpolate_value`, `interpolate_frontmatter`,
    `rewrite_directive_targets`, and `run_stage` were updated.
  - `darkmatter/docs/**` is left for Phase 7.
    `docs/topics/darkmatter-expressions.md:1002` and
    `docs/inline/file-links.md:124` still mention `fail_fast: false`. Check
    them there.

### Blast-radius triage (4.4)

S-3's 26 failures reproduced as predicted, plus two Phase 3 tests that
asserted lenient behavior. None was a latent breakage in a shipped artifact.

| Test(s) | Disposition |
| --- | --- |
| `fatality_characterization` (4) | R-6 rewrite on the policy axis |
| `frontmatter_interpolation::…mixed_malformed_interpolation_stays_warning_without_fail_fast` | Now `mixed_malformed_interpolation_is_fatal`. Added `mixed_unevaluatable_interpolation_is_fatal` and `best_effort_keeps_mixed_text_failures_lenient` |
| `rendering::test_interpolation_{parse_error_preserves_original,bare_pipe_produces_parse_error}` | Now `…_is_fatal_without_fail_fast`, asserting a typed `Interpolation` error |
| `provider_network::generic_expression_failures_still_warn_on_the_body_surface` | Now `…_are_fatal_on_the_body_surface`. The provider-versus-generic split is still pinned under the lenient helper by the matrix |
| `expression_regression` (11) | One `assert_fatal_without_fail_fast` helper per cause. `regression_division_by_zero_error_in_interpolation` became an exact duplicate and was removed |
| `context_functions::recent_commits_rejects_invalid_counts_in_the_body` | The type-error half now expects a fatal error |
| `nested_composition::empty_and_non_string_arguments` | Non-string and null arguments are fatal |
| `nested_composition::nested_diagnostics_keep_their_provenance` | Stage provenance is kept through a `ctx.toady` warning. The inner `length(1, 2)` is now fatal with `as_markdown(): nested composition failed` |
| `nested_composition::self_composition_reaches_the_depth_limit` | Fixture: `again` is self-referencing mixed-text state, now `with_exclude_keys(["again"])` (DM1) so the raw template reaches each level |
| CLI `compose_terminal_detection` (2) | `DOC_WITH_WARNING` uses `{{ ctx.toady }}`, a warning that survives R2 |
| Phase 3 `compose_diagnostic_identity` (lib 2, CLI 1) | The rescan fixture asserts **one fatal error** (lib: typed parse error, `> invalid`; CLI: exit 1, empty stdout, one `MarkdownError:`, `Expression at line: 4`). The two-frontmatter-keys test now fails on one key; scope non-aliasing stays pinned by `report::tests::expression_failures_in_different_scopes_do_not_alias` |
| `subtree::tests::dm2_lenient_tolerates_malformed_span_in_mixed_string` | Unchanged and green (the Lenient regression) |

### Requirement-to-test mapping

| Behavior | Test(s) |
| --- | --- |
| Body parse and evaluation failure fatal with `fail_fast` false and true; typed error, `OnDiskLine` path and line, source untouched | `lib/tests/compose_expression_failure_contract.rs::a_body_{parse,evaluation}_failure_is_fatal_whatever_fail_fast_says` |
| Unprovable line still names the file | `…::a_body_failure_after_a_rewritten_prefix_still_names_the_file` |
| Rescan-generated failure fatal, no authored line | `…::a_failure_in_replacement_output_is_fatal_without_an_authored_line` |
| Mixed frontmatter parse, evaluation, and nested value fatal; key plus on-disk file | `…::a_mixed_frontmatter_{parse,evaluation}_failure_is_fatal_whatever_fail_fast_says`, `…::a_nested_mixed_frontmatter_failure_is_fatal`; unit `frontmatter_interpolation::…::mixed_{malformed,unevaluatable}_interpolation_is_fatal` |
| Already-strict surfaces stay strict | `…::a_whole_value_frontmatter_failure_stays_fatal`, `a_page_block_condition_failure_stays_fatal`, `a_transclusion_condition_failure_stays_fatal`, `a_directive_target_failure_stays_fatal`, `shell_ternary_condition_and_branch_failures_stay_fatal` |
| Subtree Lenient stays lenient, Strict stays strict | `…::subtree_strictness_keeps_its_contract`; unchanged `subtree::tests::dm2_*` |
| Public condition API keeps `Result` errors | `…::the_public_condition_api_still_returns_failures_as_results`. Claudine: `looping::expression::tests::{parse,evaluate}_errors_carry_the_typed_*_cause` (fatal loops) and `dispatch::runner::tests::when_invalid_expression_skips_action_non_fatally` (warn-and-skip hooks), both green |
| Preflight stays lenient where it must | `…::preflight_discovery_tolerates_what_composition_rejects`, `…::preflight_discovery_does_not_fail_on_a_region_composition_removes`; unit `best_effort_keeps_mixed_text_failures_lenient` |
| Document entry passes `Strict` (not only the helper) | `fatality_characterization::document_body_is_strict_whatever_fail_fast_says` |
| Rendered error shows file and line for generic causes | `errors::blocks::tests::interpolation_block_generic_cause_renders_the_{authored_body_line,frontmatter_key}`, `…_without_a_file_has_no_locus` |
| CLI: exit 1, source line, empty stdout, exactly one error | `cli/tests/compose_expression_failures.rs` (4 tests: body parse, body eval, mixed frontmatter, no partial document); CLI `compose_diagnostic_identity::compose_reports_a_bad_expression_beside_a_replacement_once` |
| Criterion 8: shipped `feature-review.md` composes; the incident typo stops the build | `lib/tests/feature_review_incident.rs` (3 tests; see below) |

**Criterion 8 design.** The typo landed in `7658203f8` as `{{spec-name}}`.
The grammar of the time lexed that as `spec - name`.
- The regression starts from the **shipped** prompt and reinstates that
  reading as `{{spec - name}}`. It asserts exactly one substitution, so the
  test fails loudly if the prompt drifts.
- It composes the prompt as Claudine's prepare path does (lifecycle event
  keys excluded, demand-driven context, `spec` set) and requires a typed
  `Subtraction requires numeric operands` error at the prompt's line under
  both `fail_fast` values.
- A sibling test shows that today's grammar reads `{{spec-name}}` as one
  identifier: no subtraction error and no verbatim span.
- `md` alone cannot compose this prompt. Its Claudine lifecycle `success:`
  key reads a review file that does not exist yet, which was a fatal
  file-reference error before this feature. Claudine defers those keys.

**Proof that the regressions fail on the old behavior.** Each file below was
changed temporarily and then restored from a byte copy, checked with `cmp`.
- Forcing `expression_failure_policy()` to `Lenient` turned 9 tests red: 7
  contract tests, the matrix's document-entry pin, and the incident test.
- Removing the discovery flag turned both preflight tests red.

Passive corpus: this phase changes the failure policy, not the grammar. The
Phase 1 corpus gate (every shipped executable expression parses) stays green
inside `just test`, so no shipped prompt hits an R2 parse failure. Evaluation
failures cannot be checked passively. The S-3 probe and this phase's Claudine
run show that no shipped prompt fails on them. Nothing is persisted, so no
round trip applies.

### Gates run

| Gate | Result |
| --- | --- |
| Targeted: `compose_expression_failure_contract` (16), `feature_review_incident` (3), CLI `compose_expression_failures` (4) plus `spawn_site_guard` | all pass |
| `just test` (darkmatter) | **8219/8219 pass**, 14 skipped. Load average was about 13 on this host, with no timeouts |
| `just lint` (darkmatter) | pass after removing three needless borrows in new test code |
| `just test --no-fail-fast` (claudine) | 7032/7036. The **4 failures are pre-existing** and identical to Phases 1–3 (`shipped_implement_prompts_have_not_drifted_from_their_fixture`, `shipped_implement_plan_launches_without_a_sibling_spec`, and two `shipped_implement_plan_*` schema tests from the uncommitted `prompts/_implement/implement-plan.md` edit) |
| Cross-OS: native Windows (`build-win-native`) | **Not run: the host is out of disk.** Patch sync failed with `fatal: write error: No space left on device`. It is a shared host, so no cleanup was attempted |
| GitNexus `detect-changes` | Not a clean check, as in Phase 3. The worktree carries hundreds of unrelated uncommitted files. Pre-edit `impact` results are above; `interpolation_block` was LOW |

## Phase 5

Phase 5 implements Requirement 4. A well-formed identifier that resolves to
nothing warns at `dm.expression.unknown_identifier`. Composition still
succeeds and the value still renders empty. It stays silent when the author
handled the absence, or when the root is known to the final state, a caller
input, or the effective schema. Work-groups 5A, 5B, and 5C are done.

### Pre-edit impact (GitNexus, upstream)

- **CRITICAL:** `Evaluator::eval` (129 impacted).
- **HIGH:** `evaluate_expr` (145) and `evaluate_value_branch`.
- **MEDIUM:** `interpolate_text_located` (49), `interpolate_value` (20),
  `rewrite_value` (50), and `evaluate_condition` (27).
- **LOW:** `run_compose_pipeline_node`, `render_page_blocks`, and
  `evaluate_ternary_condition`.
- **UNKNOWN (no resolvable callers):** `EffectiveState`, `ResolvingLookup`,
  and `FrontmatterSeedState`. A text search confirmed that the only caller of
  `is_known_variable_root` was subtree strict mode, and its `LayeredLookup`
  does not delegate to the state.
- **Mitigation.** No existing entry point changes signature or output.
  - `evaluate`, `evaluate_condition`, `Evaluator::eval`/`eval_json`/
    `eval_value`, and `interpolate_value` behave exactly as before.
  - Observation is a separate crate-private path: `evaluate_observed`,
    `evaluate_condition_observed`, and `Evaluator::observing_missing_roots`.
  - The `()` observer's `OBSERVES = false` compiles the check away.

### What changed

- **Shared classifier (5.4–5.6, R-2).** The new `expression/absence.rs`
  holds `AbsenceScope`, a `Copy` value threaded through `evaluate_expr`.
  - It carries a `handles_absence` flag plus a stack-borrowed linked list of
    `RootGuard`s, so descending allocates nothing.
  - Because the scope rides the evaluator's own recursion, observation
    follows evaluation. An unchosen ternary branch, a short-circuited
    fallback, or an `and`/`or` operand is never read and never warns. There
    is no pre-evaluation sweep.
  - Handled positions:
    - A fallback primary, plus the operands of a fallback chain that is
      itself a primary.
    - A ternary condition.
    - The condition's own root inside the branches, when the condition is a
      bare variable (through parentheses).
    - A direct argument of an absence predicate.
    - Path or index access on a handled value (`x.deep || "d"`,
      `x[0] || "d"`).
  - Every other node resets to an ordinary operand. So `x == 1 ? …`, `!x`,
    and `is_empty(lower(x))` warn.
  - `collect_variable_roots` (subtree strict mode) and
    `walk_context_variables` (`ctx.*` typo warnings) are unchanged, per R-2.
- **Absence predicates (5.5, R-1).** `predicates.rs` splits `is_null` and
  `is_empty` into a new `ABSENCE_PREDICATES` binding group, registered in
  `BINDING_GROUPS`. Group membership is the typed marker.
  - `functions::is_absence_predicate(name)` resolves the canonical name or an
    alias, case-insensitively, as dispatch does. The walker never spells the
    names. The rule is marked interim (Resolved Decision 9).
  - **Deviation from R-1:** R-1 suggested a field on `FunctionBinding`. That
    would have edited every binding literal across 16 group files. A group
    gives the same typed, alias-aware lookup, following the existing
    `LAZY_BINDINGS` precedent. Removing it later means deleting one group
    and one function.
  - Pinned by
    `registration_tests::exactly_is_null_and_is_empty_are_absence_predicates_with_their_aliases`.
- **Known roots (5.2, R-9).** Three lookups now override
  `is_known_variable_root`: `EffectiveState`, `ResolvingLookup` (which
  forwards), and `FrontmatterSeedState`.
  - A root is known when it is:
    - a key of the state, even when its value is `null` or `""`;
    - a reserved root: `ctx`, `env`, `doc`, `current`, or `current_env`;
    - a bare runtime-context name, which covers `when="repo"`;
    - `null`.
  - `DeferrableLookup` already forwarded. The trait default `true` still
    keeps third-party lookups silent. Its doc now names the second consumer.
  - **`null` is a reserved root, a correction found in testing.** The grammar
    has no `null` literal. `null` lexes as a variable that resolves to
    nothing, and 7+ shipped prompts use it as the literal
    (`{{ ok ? path : null }}`, `{{ ctx.x || null }}`). Without this, the
    shipped `feature-review.md` warned `unknown identifier 'null'`.
- **Accumulator (5.1, R-5).** Candidates travel on `ComposeReport`, per unit,
  and are never behind a lock.
  - `ComposeReport` has a new crate-private `unknown_root_candidates` field,
    which `merge` folds.
  - Stage-local recording: the `Evaluator` owns a
    `RefCell<Vec<MissingRoot>>` only when built with
    `.observing_missing_roots()`. One evaluator serves one stage, so the cell
    is never shared or contended.
  - `interpolate_text_located` attributes each expression's reads to its
    span. A rescan-generated read gets no span.
  - `add_unknown_root_candidates` keeps only the first read per root and
    computes a locus only for that read.
- **Deferred reconciliation (5.3, 5.8).**
  `unknown_identifiers::reconcile` runs once per document, inside
  `run_compose_pipeline_node` just before `attribute_to_document`. By then
  every stage has run, both frontmatter passes included, and the child
  reports are already reconciled.
  - It drops a root known to the final `EffectiveState`, or a key of
    `caller_input_records`. That closes R-9's unverified item: a record alone
    counts as a caller input layer.
  - It also drops a root that the effective schema declares.
    `EffectiveSchema::declares_top_level_property` reads the merged schema
    (document, baseline, and triggers): `properties`, `patternProperties`,
    `anyOf`/`oneOf`/`allOf`/`if`/`then`/`else` arms, and local `$ref`, with a
    depth guard.
  - `PreparedSchemas::effective_for` resolves the schema once, and only when
    a candidate survives the state check.
  - `required` is never consulted. An unset required root fails schema
    validation (step 2) before reconciliation, so the schema's message is
    the only one (5.8).
  - Candidates from a discovery pass (`defer_expression_failures`) are
    dropped.
- **Surfaces (5.7).**
  - The body stage (`inline/interpolation.rs`) observes and sorts reads back
    into document order, because the scanner visits spans end to start.
  - Frontmatter interpolation observes in both passes, per top-level key.
    Best-effort (discovery) runs do not observe.
  - Page-block and transclusion `when=` go through
    `conditions::evaluate_condition_observed`. The public
    `evaluate_condition` delegates with `()`. The whole condition is an
    ordinary position, so a bare `when="x"` warns.
  - `$()` ternaries: the condition, the selected branch's value evaluation,
    and pipeline interpolation.
    - Both branches are prepared before the condition is known, so each
      branch records into its own vector and only the selected one is kept.
    - A ternary condition is a gate like `when=`, not an absence check.
    - `directive_reachable_pipelines` (discovery) passes a throwaway vector.
- **Warning (5.9).** `ComposeWarning::unknown_identifier` sets:
  - code `dm.expression.unknown_identifier`;
  - source `darkmatter.expression` (new const `EXPRESSION_SOURCE`);
  - `path` (the document) and `line_number`;
  - identity `Root { document, name: root }`.
  - Phase 3's dedup then gives one warning per root per source document.
  - The message carries the location, because `md` and Claudine render only
    the message:
    `unknown identifier 'spec-name' at prompts/x.md:73: no frontmatter key,
    caller input, or schema property defines it, so it resolves to null`.
  - Warnings stay stderr-only.
- **Provable lines only.** This follows Phase 4's rule against guessed lines.
  - Body spans use `authored_line`. When an earlier stage rewrote the prefix,
    they fall back to `unique_authored_line`: the exact `{{ … }}` text must
    occur once in the file.
  - `::block` and `::file` directive lines are body-relative. They are
    converted by `unknown_identifiers::directive_locus`, which accepts the
    same text at the offset line, or a unique line match.
    - The page-block stage converts before it replaces the body, because the
      public `render_page_blocks` has no `Markdown`.
    - Found by `md compose`: a `::file when=` below a removed page block was
      reported at line 7 instead of 10.
  - Frontmatter reads locate at their top-level key through the existing
    `frontmatter_key_line`.
  - An unprovable read names only the file.
- **Prose escaping (found with the real CLI).** Both `md` and Claudine's
  `log::warn` render `ComposeWarning.message` as Prose. So
  `prompts/_add/_workflow.md` rendered as `prompts/add/workflow.md` (the
  underscores became emphasis). The root, path, and key are now escaped with
  `Prose::escape_text`.
- **Directive targets are not observed.** A whole-value `::file {{ x }}`
  that is null already warns
  `nullable target `x` … evaluated to null; directive skipped`. A second
  warning would report one issue twice (Requirement 5).

### Decisions beyond the plan (for review)

1. **"Direct" is strict everywhere.** Only a variable directly in a handled
   position is handled. Parentheses and path or index access on it count as
   direct.
   - A ternary condition that is a comparison or negation still warns
     (`{{ status == "done" ? … }}` with a typo'd `status`).
   - The spec's examples all use bare variables, and warning here catches the
     motivating typo class.
2. **A fallback chain that is itself a primary handles every operand.** In
   `a || b || "d"`, `b` is handled, because the parser is left-associative
   and `b` sits on the RHS of the inner node.
3. **`$()` ternary conditions warn like `when=`.** Guarded-root suppression
   does not apply across a shell ternary's branches. The branches are
   separate strings, not a `Ternary` node.
4. **`is_empty(trim(x))` cannot be written.** `trim` is not a catalog
   function; the spec's row fails with `Unknown function: trim`. The test
   uses `is_empty(lower(x))` for the same nested-predicate rule. Phase 7
   docs should use a real function.
5. **Frontmatter `{{ … }}` inside a `$()` value** is interpolated by pass 1
   and so is really read, whichever ternary branch runs. It warns from the
   frontmatter stage. That is correct, but it surprised the first fixture.

### Requirement-to-test mapping

All in `lib/tests/unknown_identifier_warning.rs` (23 tests) unless noted.

| Behavior | Test(s) |
| --- | --- |
| 5.10 undeclared: warning, renders empty, composes; code, source, path, line, message location; no other diagnostic | `an_undeclared_kebab_root_warns_renders_empty_and_composes` |
| 5.10 fallback silent, renders `fallback` | `a_handled_absence_is_silent_and_renders_the_fallback` |
| 5.10 declared optional silent | `a_declared_optional_root_is_silent_and_renders_empty` |
| 5.10 declared required: the schema's own failure, nothing else | `a_declared_required_root_fails_with_the_schema_message_only`; CLI `compose_reports_only_the_schema_failure_for_an_unset_required_root` (exit 1, empty stdout, no "unknown identifier") |
| 5.11 every suppression row, with render and warned roots together | `every_suppression_row_warns_exactly_where_evaluation_reads_an_unhandled_root` (26 rows). Covers every spec row, both branches of `a ? x : b`, both outcomes of `a \|\| x`, the aliases `isnull`/`isEmpty`, the chain, comparison and negation conditions, path and index handling, a known root with a missing member, and `null` |
| 5.11 bare `when="x"` warns at its line | `a_bare_unknown_page_block_gate_warns_at_its_line` |
| 5.11 absence-predicate registry | unit `exactly_is_null_and_is_empty_are_absence_predicates_with_their_aliases`; unit `absence::tests` (6) |
| 5.12 explicit `null` / `""` known | `explicit_null_and_empty_frontmatter_values_are_known` |
| 5.12 `--set` (null, `""`, value) | `set_overrides_are_known_even_when_null_or_empty`; CLI `compose_is_silent_for_a_root_supplied_with_set_even_when_null` |
| 5.12 inherited / external state | `external_state_is_known`, `a_frontmatter_read_of_inherited_state_is_silent`, `a_transcluded_child_knows_inherited_state_and_warns_for_itself` |
| 5.12 caller input layer (R-9 unverified item) | `a_caller_input_record_is_known_even_without_an_override` |
| 5.12 baseline schema | `a_baseline_schema_property_is_known` |
| 5.12 matched trigger schema, with a control that warns without triggers | `a_matched_trigger_schema_property_is_known` |
| 5.3 pass-1 candidates reconciled by the final state and schema | `pass_one_candidates_known_by_the_final_state_or_schema_are_dropped` |
| 5.13 body, mixed and whole-value frontmatter (key and line) | `an_undeclared_…`, `mixed_and_whole_value_frontmatter_warn_at_their_key` |
| 5.13 page-block and transclusion `when=` (line survives a removed block) | `a_bare_unknown_page_block_gate_warns_at_its_line`, `a_transclusion_gate_warns_at_its_authored_line` |
| 5.13 `$()` ternary: gate plus selected branch warn; unselected branch silent | `a_shell_ternary_warns_for_its_gate_and_its_selected_branch_only` |
| Discovery pass never warns about a false-block root | `a_root_read_only_inside_a_false_block_never_warns_even_with_preflight_discovery` |
| Requirement 5: one per root per document, first read kept | `one_root_read_by_every_surface_warns_once_at_its_first_read`, `ten_body_reads_warn_once_at_the_first_line`, `one_root_in_two_transcluded_documents_warns_once_per_document` |
| Criterion 8 extension: the shipped prompt is clean; the typo warns once at its line | `lib/tests/feature_review_incident.rs` (`the_shipped_…` and `the_typo_is_one_identifier_…` extended) |
| CLI: exit 0, empty render, one stderr warning with file:line, stdout clean; `--output json` document only; Prose-escaped path | `cli/tests/compose_unknown_identifiers.rs` (5 tests) |
| Message shape and escaping; unique-line proof | unit `unknown_identifiers::tests` (3) |

**Proof that the tests are load-bearing.** Each probe temporarily disabled
one mechanism. Each file was restored from a byte copy, checked with `cmp`.
- `AbsenceScope::handles` returning false: the suppression table and the
  fallback test go red (2 tests).
- Schema reconciliation disabled: the baseline, trigger, and pass-1 tests go
  red (3).
- Discovery-candidate drop disabled: nothing goes red. The discovery report
  never reaches the caller, so the drop is defense-in-depth. The test pins
  the user-visible behavior anyway.
- Final-state recheck at reconciliation disabled: nothing goes red.
  - External state and `--set` merge into frontmatter before pass 1.
  - Materialized optional bindings are also schema-declared.
  - So no current path is known only to the final state. The recheck stays,
    because the spec mandates it and it costs O(candidates).

**Shipped-artifact sweep.** Every `.md` in the four corpus directories was
composed with `md compose`.
- Before the `null` fix, the only false-positive class was `null`.
- The remaining warnings are real:
  - Claudine lifecycle `success:`/`failure:` values read event-time
    `error`/`err`/`timing`. Claudine excludes those keys, so under Claudine
    they are never evaluated.
  - Caller inputs (`{{context}}` in `brainstorm.md`).
  - `_`-prefixed partials composed standalone.
- The passive corpus gate (`dasherized_identifier_corpus.rs`) is unaffected:
  this phase changed no grammar. The warning is runtime-only, so the
  end-to-end coverage is the real `feature-review.md` composed as Claudine
  does, plus the sweep.

No value is persisted, so no round trip applies.

### Performance posture (by inspection, no new SLOs)

- **Invariant 2 (O(1) per variable evaluation).** Observation runs only when
  a read misses.
  - It is one `AbsenceScope::handles` check, bounded by the expression's
    ternary nesting.
  - Then one `is_known_variable_root`: a hash probe plus a fixed-size
    catalog check.
  - Unobserved evaluation (`()`) skips it at compile time.
- **Invariant 3 (no cross-document lock).** No lock was added.
  - The recorder is a per-stage `RefCell` inside a non-`Sync` `Evaluator`,
    never shared with rayon workers.
  - Shell ternaries are evaluated sequentially, before the parallel
    execution step.
  - Candidates ride on each document's own `ComposeReport` and are reconciled
    before `merge`. A cached child replays its already-reconciled warnings.

### Gates run

| Gate | Result |
| --- | --- |
| Targeted: `unknown_identifier_warning` (23), `feature_review_incident` (3), lib units `unknown_identifiers`/`absence`/registry (12), CLI `compose_unknown_identifiers` (5) plus `spawn_site_guard` | all pass |
| `just test --no-fail-fast` (darkmatter) | 8256 run: **8195 passed, 0 failed**, 14 skipped, 61 timed out. Every timeout is in the documented HTTP-client cluster (`remote_fetch`, `provider_network`, `remote_transclusion_tests`, `preflight::collect` remote, `fn_remote_tests`, `resolve_ctx` fetch), at a load average of 30 |
| HTTP cluster re-run at `--test-threads 2` | **155/155 pass** |
| `just lint` (darkmatter: clippy `--all-targets -D warnings` for `darkmatter`, `darkmatter-cli`, `dmls`, `zed-dmls-cli`, plus the zed-dmls wasm32-wasip2 check) | pass, after removing one needless borrow in the new CLI test |
| `just test --no-fail-fast` (claudine) | 7032/7036. The **4 failures are pre-existing** and identical to Phases 1–4, with the same causes: two `LifecycleObjectDataThroughInterpolationPositional` errors, the route-drift hash pin, and `object value not allowed here`, all from the uncommitted `prompts/_implement/implement-plan.md` edit. None involves an unknown-identifier warning |
| Shipped-prompt sweep with `md compose` (4 corpus dirs) | no false positives after the `null` fix; the remaining warnings are real (see above) |
| GitNexus `detect-changes` | Not a clean check, as in Phases 3–4. The worktree carries hundreds of unrelated uncommitted files (a benchmarks move and Claudine work). Pre-edit `impact` results are listed above |

- **An accidental repo-wide lint was stopped.** The first `just lint` ran
  from the repo root after the shell's cwd reset, so it ran the root
  orchestrator across every area. It was killed (only this session's process
  tree), and the darkmatter recipe was re-run from `darkmatter/`.
- **Cross-OS: no remote run.**
  - This phase adds no filesystem, process, terminal, or path-spelling
    behavior.
  - Path equality in the new tests uses `std::fs::canonicalize` on both
    sides, exactly as the transclusion resolver and its own tests do. That
    matches Windows `\\?\` spelling.
  - Line proofs use `str::lines` and `\n` counts, which are CRLF-safe.
  - Prose escaping also keeps Windows backslash paths literal in the
    rendered warning.
  - CI covers Linux, Windows, and WSL. Phase 4 found `build-win-native` out
    of disk.

## Phase 6

Phase 6 implements Requirement 3 in DMLS. `dm.expression.unknown_identifier`
now checks every `Variable` in any operand position, at its own span, in both
body `{{ … }}` and Expression-typed frontmatter values. It suppresses what the
runtime suppresses, reports at `WARNING`, and adds a dash-separated-key
diagnostic with a structured quick-fix. Work-groups 6A and 6B are done.

### Pre-edit impact (GitNexus, upstream)

- **HIGH:** `is_unknown_root` (6 impacted). Every impacted symbol is in the
  DMLS diagnostic path. Its two direct callers, `is_unknown_identifier` and
  `expression_root_is_unknown`, are the two sites this phase rewrites. Its
  signature is unchanged.
- **LOW:** both `expression_diagnostics` functions (the frontmatter one has 21
  impacted), `is_unknown_identifier`, `expression_root_is_unknown`, and
  `code_actions`.
- **Not in the index:** `AbsenceScope` (added in Phase 5, index stale). A text
  search found it only in `expression/mod.rs` and `absence.rs`. This phase
  adds to it and changes no existing method.
- **`root_identifier` was not edited.** `git diff` shows no change to its body.
  Its five consumers (hover, definition, graph indexing, and the old two
  diagnostic sites) keep the one-root contract. The diagnostic sites no longer
  call it.

### What changed

- **Library: the static twin of the runtime classifier (6.1, 6.3).**
  `expression/absence.rs` gains `static_variable_reads(&SpannedExpr)`. It
  yields a `StaticVariableRead { path, span, handles_absence }` for every
  `Variable`.
  - `walk_static` mirrors `evaluate_expr`'s scope transitions node for node.
  - A `SpannedExpr` twin of `condition_guard` keeps every rule in
    `absence.rs`, as the Phase 5 hand-off asked. DMLS never re-derives them.
  - The one deliberate difference: both ternary branches and every fallback
    operand are visited (Resolved Decision 6).
  - It is single-pass, linear in AST nodes, and works on the
    already-parsed AST.
  - `is_statically_known_root(root)` is the state-independent half of
    `EffectiveState::is_known_variable_root`: reserved roots (including
    `null`, `current`, `current_env`) plus bare runtime-context names
    (`repo`, `today`).
  - All three are re-exported from `expression`. The additions are purely
    additive, and no runtime path changed.
- **DMLS classification (6.2).** `is_unknown_root` stays the shared
  authority. Its hard-coded `ctx|env|doc` check is replaced by
  `is_statically_known_root`, so the editor no longer flags `null`,
  `current`, or `repo`. Callers now pass each read's first dotted segment.
- **`KnownRoots` (dsl.rs).** One type is shared by both diagnostic sites.
  - It holds the frontmatter AST plus `known_shape`, computed once per
    diagnostics pass. The old code rebuilt the shape for every expression.
  - It is `None` without frontmatter, which preserves the frontmatter-less
    editor exception.
  - The old `is_unknown_identifier` / `expression_root_is_unknown` wrappers
    were deleted. Their doc comments moved to `KnownRoots`.
- **Severity (6.4).** Both sites emit through
  `dsl::unknown_identifier_diagnostic` at `WARNING`.
  - R-10 inventory: the one severity-asserting unit test was renamed to
    `unknown_expression_root_is_a_warning_frontmatter_diagnostic` and
    flipped.
  - The `dmls/docs/diagnostics.md` row now says **Warning**.
  - No snapshot or editor configuration depended on the old severity.
- **Dash-separated keys (6.5).** `overlay::expressions::unknown_identifier_findings`
  looks for subtraction chains that contain an unknown operand. A chain holds
  only variables, unary minus, and `-`, such as `foo--bar`, `a- b`, or the
  `a- b` inside `a- b - x`.
  - When the chain's whitespace-free source is a present or declared
    top-level key, it emits **one** key-level finding over the subtraction.
    That finding replaces the operands' generic findings.
  - A literal operand keeps it arithmetic (`iteration - 1` against a key
    `iteration-1`).
  - When both operands are known, nothing is reported, since that is
    intentional arithmetic.
- **Quick-fix (6.6, R-8).**
  - `KeyReferenceFix { key, replacement }` (serde) rides in `Diagnostic.data`.
  - The fix is proven: the edited expression must reparse in the same dialect
    with a node at exactly the replacement's span that references the key.
    The bare key is tried first, then `doc['key']`.
  - The bracket quote avoids the frontmatter value's YAML quote style, so
    `'foo--bar'` becomes `doc["foo--bar"]`.
  - In frontmatter, the fix is withheld when the projection crosses a YAML
    escape, because the authored bytes differ from the decoded text.
  - `code_actions` gains a `reference-dash-separated-key` category. It reads
    `data`. When a client drops `data`, it recomputes the producing provider's
    diagnostics (chosen by `source`) and takes the one at the same range. It
    never reads the message.

### Decisions beyond the plan (for review)

1. **Dotted reads classify by root.** `{{ user.name }}` with an unknown `user`
   now warns in DMLS. Before, any dotted path was silent. The spec and runtime
   both classify by the first segment.
2. **`null`, `current`, `current_env`, and bare runtime-context names are
   known in DMLS.** Before, a bare `{{ today }}` or `{{ null }}` got the
   INFORMATION squiggle. The runtime knows them, so the editor now agrees
   (Phase 5 hand-off item 2).
3. **Frontmatter `||` is `or()`.** Expression-typed frontmatter values use the
   condition dialect, where `||` lowers to `or(...)`. Its operands are
   ordinary, so `known || fm_rhs` flags `fm_rhs`. It is the same AST and
   classifier the runtime uses, so the two stay in agreement.
4. **The editor reports every occurrence.** Requirement 5's one warning per
   root binds a compose run. An editor squiggle belongs on each occurrence.
5. **A dash-key finding requires an unknown operand.** Without that, valid
   arithmetic between two known keys would gain a new warning just because a
   hyphenated key also exists.

### Requirement-to-test mapping

| Behavior | Test(s) |
| --- | --- |
| 6.1/6.3 static walk: every suppression row, read statically (21 rows incl. aliases, chain, comparison/negation conditions, path/index, `foo--bar`, arrays) | lib unit `absence::tests::static_reads_apply_the_suppression_rules_to_every_branch` |
| Editor/runtime parity: every runtime warning (all-missing lookup) is a static warning; the observer must actually fire | lib unit `absence::tests::every_runtime_warning_is_also_a_static_warning` |
| Each read carries its own span (kebab, dotted, call arg, branch) | lib unit `absence::tests::static_reads_carry_their_own_spans` |
| Reserved roots, `null`, runtime-context names known; typos not | lib unit `absence::tests::statically_known_roots_match_the_runtime_reserved_and_context_names` |
| 6.7 operand positions (binary, ternary branches, call arg, fallback RHS, dotted) at own span | dmls unit `overlay::expressions::tests::every_operand_position_is_checked_at_its_own_span` |
| Suppressions and known roots silent; nested predicate warns | dmls unit `handled_absence_and_known_roots_are_silent` |
| Dash key: both replacement forms, YAML-quote avoidance, inner chain only | dmls unit `a_subtraction_spelling_a_key_is_one_finding_with_the_right_replacement` |
| Ambiguous arithmetic: generic findings, no fix; both-known silent | dmls unit `ambiguous_arithmetic_keeps_generic_findings_and_no_fix` |
| Unprovable replacement: no fix | dmls unit `a_key_no_replacement_can_reference_gets_no_fix` |
| Structured payload JSON round trip | dmls unit `the_fix_payload_round_trips_through_json` |
| 6.4 severity at the frontmatter site (R-10 item 1) | dmls unit `unknown_expression_root_is_a_warning_frontmatter_diagnostic` |
| **Protocol (real server):** every operand position in body **and** frontmatter, exact ranges, source, `WARNING` (severity 2), messages; `S*` suppressions and `null`/`repo`/`doc['typo-key']` silent | `tests/lsp_session.rs::unknown_identifier_fires_in_every_operand_position_at_warning_severity` |
| **Protocol:** `data` for `doc['foo--bar']`, `a-b`, frontmatter `doc["foo--bar"]`, `a-b`; code action edit equals `data` and targets the diagnostic range, both with `data` echoed and with `data` stripped; ambiguous arithmetic has no data and no action | `tests/lsp_session.rs::dash_separated_key_carries_its_replacement_and_quick_fix` |
| **Protocol:** frontmatter-less exception stays silent | `tests/lsp_session.rs::a_frontmatter_less_document_never_reports_unknown_identifiers` |
| Success Criterion 7 (kebab navigation) still holds | existing `kebab_identifier_navigation_resolves_at_every_cursor_position` |
| `root_identifier` unchanged | no diff to the function; existing hover/definition/graph tests (`test_root_identifier_*`, `kebab_identifier_navigation_…`, `interpolation_ctx_hover_*`) green |

**Proof that the protocol tests are load-bearing.** Each probe temporarily
disabled one mechanism. Every file was restored from a byte copy and checked
with `cmp`.

| Probe | Test that went red |
| --- | --- |
| Absence handling ignored | the operand-position test |
| `data` fallback removed | the dash-key test (the stripped-`data` branch) |
| `data` never attached | the dash-key test |
| Severity back to `INFORMATION` | the operand-position test |
| Frontmatter-less exception removed | the frontmatter-less test |

**Shipped-artifact sweep.** A throwaway protocol test, since deleted, opened
all 77 `.md` files in the four corpus directories through the real server.
- It produced 15 findings and 0 dash-key findings.
- Every finding matches the runtime's rules:
  - caller inputs of `_`-prefixed partials (`_fm.md`, `_workflow.md`,
    `implement-suggestions.md`'s `perf`, `brainstorm.md`'s `context`);
  - `i` inside a `::loop` body in `_agent-skills.md`. `::loop` is not
    implemented in compose or DMLS, so `i` really resolves to nothing.
- There is no `null` or reserved-root noise.
- The grammar did not change, so the passive corpus gate is unaffected.
- The editor side has no persisted values, so no round trip applies.

### Gates run

| Gate | Result |
| --- | --- |
| Targeted: lib `absence` (13 incl. registry), dmls lib `expressions::`/`frontmatter::` (150), `lsp_session` new three plus kebab/schema-declared | all pass |
| `just test` (darkmatter) | **8269 passed, 0 failed**, 14 skipped. No timeouts this run (load average about 7) |
| `just lint` (darkmatter: clippy `-D warnings` for all four crates, plus zed-dmls wasm32-wasip2) | pass on the first run |
| Claudine | not run. The library change is additive (new public functions) and changes no runtime path |
| GitNexus `detect-changes --scope all` | **Not a clean check.** It is truncated ("… and 851 more"), and the worktree carries 541 changed files of unrelated uncommitted work. Pre-edit `impact` results are listed above |
| Cross-OS | no remote run. The phase adds no filesystem, process, path, or terminal behavior. LSP ranges go through the existing `SourceMap` (UTF-16/CRLF-aware), and the new tests use ASCII columns |

## Phase 7

Phase 7 brings the documentation and the Darkmatter skill up to date with the
behavior shipped in Phases 2–6. It is documentation-only apart from doc
comments.

### What changed

- **7.1 Parsing topic docs.**
  - `lexing.md`: the identifier section states the `-` rule and has a
    row-by-row table (`spec-name`, `doc.spec-name`, `false-1`, `a - b`, `4-2`,
    `foo--bar`, `spec-`). The kebab workaround paragraph is replaced by the
    whitespace rule for subtraction and the remaining bracket-access cases. It
    also points cursor-side code at `identifier_prefix_start`.
  - `grammar.md`: "What the language server sees" is rewritten, not just
    trimmed (Phase 6 hand-off). It covers every operand at its own span, the
    `static_variable_reads` walk, suppression parity with the one reachability
    difference, the frontmatter-less exemption, the dash-key quick-fix, and a
    severity column.
  - `index.md`: the known-deviation callout is gone. The failure table adds
    mixed-text frontmatter and `$()` rows, states the `fail_fast` narrowing and
    the one lenient path, separates "missing value warns" from "failure", and
    links the registry rules.
  - `scanning.md` (not named by the spec): "fails to parse and warns" became
    "fails composition".
- **7.2 Expression and interpolation docs.**
  - `darkmatter-expressions.md`: the identifier principle is rewritten, a
    "missing value warns" principle is added, kebab keys are listed under
    Variable Access, and the unsupported-forms list swaps the
    `spec-name`-is-subtraction entry for the unspaced-subtraction and
    bracket-only-keys entries.
  - `inline/interpolation.md`: the fallback paragraph now mentions the
    warning. New sections: "Kebab-case Keys", "Missing Variables" (known-root
    table, real warning text, suppression table), and "Failures". The
    Implementation bullet about failing expressions now scopes "left in place"
    to the lenient policy.
- **7.3 Registry note.** `dmls/docs/diagnostics.md` gains "`dm.*` registry
  rules" (ownership, one-condition-one-code, severity-per-surface, adding a
  code), linked from `parsing/index.md`. It cites `unknown_identifier` as a
  Warning on both surfaces. No shared constant module was built (Resolved
  Decision 15).
- **7.4 Public API docs.** `with_fail_fast`, the `fail_fast` field,
  `ComposeWarning`, and the fatality matrix were already correct from Phase 4.
  `subtree.rs` was not:
  - The module docs gain an "Expression Failures" section. It separates
    expression authoring failures (always fatal in documents; degraded only by
    an explicit `Lenient` subtree call) from recoverable non-expression stage
    failures.
  - `SubtreeStrictness`'s "Lenient — Darkmatter's existing body and
    mixed-string use cases" was drift. It now says document composition never
    uses Lenient.
- **7.5 Skill.** `compose.md`: two Variable Resolution rows (kebab key,
  bracket form) and a rewritten parsing pointer, which no longer implies some
  surfaces warn, and now points at the registry rules. Its Error Handling and
  unknown-identifier sections were already current from Phases 4–5. Hash
  re-run: the value is unchanged, because `last_updated` is outside the hashed
  keys. No README described the changed behavior.

### Drift found outside the spec's list (code assumed correct)

1. **`docs/inline/fm-interpolation.md`** said mixed-text failures are lenient
   when `fail_fast` is off, and called the whole-value case the "one exception".
   Both were stale after Phase 4. The section now says failures are fatal
   everywhere. The heading was renamed to "Whole Values Keep Their Type", and
   its two inbound anchors (same file and `docs/topics/context-variables.md`)
   were repointed.
2. **The same file's "Important Limitation"** said chained templated keys do
   not resolve (`plan: ".plan.md"`). `md compose` resolves them to
   `/root/spec.md.plan.md` (dependency-order resolution; see
   `frontmatter_interpolation.rs` module docs,
   `chained_reference_resolved_incrementally`). This predates this feature. I
   replaced it with "Chained References", which describes the observed
   behavior and the cycle termination: `a: "{{b}}x"` / `b: "{{a}}y"` gives
   `x` / `xy` (`mutual_cycle_terminates_without_hang`). I fixed it because it
   contradicted the new missing-variable text.
3. **`docs/lsp/features.md`** listed "left-in-place expressions" as a
   fail-fast=false fallback. Removed.
4. **`docs/inline/interpolation.md`** linked `./darkmatter-compose-pipeline.md`,
   which does not exist. It now points at `../darkmatter-compose-pipeline.md`.

### Requirement-to-test mapping

No runtime behavior changed. The only Rust edit is doc comments in
`subtree.rs`. Each documented claim is pinned by an existing test. I also
checked each claim against the real `md` binary.

| Documented claim | Pinned by | Observed via `md compose` |
| --- | --- | --- |
| Lexing table (`spec-name`, `doc.spec-name`, `false-1`, `true-value`, `iteration-1` join; `a - b`, `a -b`, `a- b`, `4-2`, `f(x)-1` subtract; `foo--bar` = `foo - (-bar)`; `spec-` parse error) | `parser.rs` `dasherized_identifiers` module; lexer unit tests; `lib/tests/dasherized_identifier_compose.rs`; `cli/tests/compose_dasherized_identifiers.rs` | every row: `[KEY][3][3][3][2][1][10][IT][2][F1][TV][SN][DD]`, `spec-` → "Expected expression" exit 1 |
| Unknown root warns once per document with that message; handled absence silent | `lib/tests/unknown_identifier_warning.rs`, `cli/tests/compose_unknown_identifiers.rs` | `colour` twice → one warning; `shade \|\| "none"` silent |
| Body and mixed-text frontmatter parse/eval failures fatal, exit 1, file and line | `lib/tests/compose_expression_failure_contract.rs`, `cli/tests/compose_expression_failures.rs`, `fatality_characterization.rs` | `upper(` body and `title: "a {{ upper( }}"` → exit 1; `nosuchfn` → exit 1 |
| DMLS every operand, Warning, dash-key quick-fix, frontmatter-less exemption | `dmls/tests/lsp_session.rs` (three Phase 6 protocol tests) | — |
| `dm.expression.malformed` is Warning in the editor | `dsl.rs:728`, `frontmatter.rs` emit `WARNING` (read, not newly tested) | — |
| Chained references and cycle termination | `chained_reference_resolved_incrementally`, `wide_and_deep_graph_resolves_in_dependency_order`, `mutual_cycle_terminates_without_hang` | `/root/spec.md.plan.md`; `[x][xy]` |

The passive corpus gate and round-trip requirements do not apply: no parser,
schema, template, or persisted value changed.

### Gates run

| Gate | Result |
| --- | --- |
| `just lint` (darkmatter, all four crates + zed-dmls wasm32-wasip2) | pass |
| `just test` (darkmatter L1) | **7947 passed, 0 failed**, 7 skipped (load average about 7.6). Phase 6 logged 8269/14. This phase changed no tests, so the count moved with the shared worktree; not investigated |
| Relative-link and anchor check over the nine edited docs | clean after the anchor fixes above. Two hits were prose (`[relative](absolute)`), not links |
| `cargo doc` with `-D rustdoc::broken_intra_doc_links` | 58 pre-existing errors crate-wide, none in `subtree.rs`/`options.rs`/`report.rs`. Not a lint gate; left alone |
| Cross-OS | not needed: docs and doc comments only |
| GitNexus impact on `SubtreeStrictness` | `UNKNOWN` (enum, no edges). A text search found its users (darkmatter lib/tests, claudine conformance tests). The edit is doc-only |

## Phase 8

Phase 8 is integration verification and handoff: full package-area gates,
downstream (Claudine) validation, cross-OS review, the Definition-of-Done
audit, and graph change analysis.

No source code changed in this phase. The one file edit outside the plan and
log is an `os` skill note (see "Skill updates").

### Gates run

| Gate | Result |
| --- | --- |
| `just test` (darkmatter L1) | **8269 passed, 0 failed**, 14 skipped (186 s, load about 13–20). This matches Phase 6's 8269/14, so Phase 7's 7947 was a shared-worktree artifact |
| `just lint` (darkmatter, four crates + zed-dmls wasm32-wasip2) | pass, exit 0. Not run concurrently with tests |
| `just test-l2` (darkmatter) | First run: `level2_tree_dim_renders_dim_sgr_in_real_terminal` panicked at `send_command_with_env failed` after 16 s. That was the cold WezTerm harness spawn under load, and nextest fail-fast then skipped the other 17. The test passed alone (0.8 s). A full re-run passed all three crates: lib 18/18, cli 69/69, dmls 3/3. Render-tree SGR code is not touched by this feature |
| `just test` (claudine L1, `--no-fail-fast`) | **7032 passed, 4 failed**, 9 skipped. All four failures come from an uncommitted, unrelated edit to `prompts/_implement/implement-plan.md` (see below) |
| Claudine's four failing tests against the committed `implement-plan.md` | **4/4 pass** with every Darkmatter change in place. The working copy was restored byte-identical (checksum verified) |
| GitNexus `detect_changes` `all` | 974 symbols / 548 files, `partial` null, `truncated` null, risk `critical` |
| GitNexus `detect_changes` `compare main` | 1902 symbols / 668 files, `partial` null, `truncated: true`. Re-ran once and got the same result: the listing cap is 1000, and the count is exact because the result is not partial |

### Claudine failures (not caused by this feature)

The shared worktree has an uncommitted edit to
`prompts/_implement/implement-plan.md`. It rewrites the `initialize` actions
`set: ["epilog", ""]` and friends from positional lists into mappings
(`set: { epilog: null }`). Claudine's `lifecycle/action_shape.rs` (unmodified)
rejects a mapping there with `LifecycleObjectDataThroughInterpolationPositional`,
and the hash pin in `shipped_prompt_route_drift` sees the byte change. The four
tests:

- `claudine composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`
- `claudine composition::schema::tests::shipped_implement_plan_preserves_supplied_commit_message_in_preflight_command`
- `claudine-cli::shipped_prompt_contract shipped_implement_plan_launches_without_a_sibling_spec`
- `claudine-cli::shipped_prompt_route_drift shipped_implement_prompts_have_not_drifted_from_their_fixture`

To prove the attribution, I swapped in `git show HEAD:` of the prompt, ran the
four tests (all four passed), and restored the edited working copy. I did not
touch the prompt edit. Its owner must either keep positional `set` or teach
`action_shape` the mapping form, then re-derive the route fixture and pin.

### Downstream consumers (8.2)

`grep` of every `Cargo.toml` depending on `darkmatter` (the Sniff package list
does not expose reverse dependencies) plus a `darkmatter::` symbol scan:

- **Claudine** (lib, cli, gen) is the only consumer of the changed surfaces
  (parser, evaluator, `compose_subtree`, compose pipeline, `evaluate_condition_against`).
- **research, biscuit-icon-cli, playa-cli, sniff-cli, biscuit-speaks-cli** use
  only `Markdown`/`TerminalOptions`/`render::Link` rendering. They do not
  compose, so they are unaffected.

The three Resolved Decision 10 behaviors all passed in the Claudine run:

| Behavior | Pinning test |
| --- | --- |
| Loop conditions stay fatal | `looping/expression.rs` `parse_errors_carry_the_typed_parse_cause`, `evaluate_errors_carry_the_typed_evaluate_cause` |
| Hook `when=` still warns and skips | `dispatch/runner/tests.rs` `when_invalid_expression_skips_action_non_fatally` |
| Explicit `SubtreeStrictness` choices unchanged | `composition/interpolation_conformance.rs` (Strict and Lenient rows); Darkmatter side `compose_expression_failure_contract.rs` `subtree_strictness_keeps_its_contract`, `the_public_condition_api_still_returns_failures_as_results` |

### Cross-OS review (8.3)

Every new test was reviewed by reading it:

- **`dasherized_identifier_corpus.rs`**
  - It sorts the collected files explicitly (`files.sort()`), like `yaml_files_in`.
  - It joins `".claude/commands"` with `/`, which `Path::join` accepts on Windows.
  - The only symlink in the corpus dirs is `prompts/.markdownlint.jsonc`, which the extension filter drops. Windows checkouts that materialize symlinks as text are therefore irrelevant.
  - `.gitattributes` sets `* text=auto eol=lf`, so a Windows checkout cannot introduce CRLF.
- **`unknown_identifier_warning.rs`** compares warning paths against `child.canonicalize()`. The transclusion resolver (`transclusion/resolver.rs:167`) also uses `std::fs::canonicalize`, so both sides carry the same `\\?\` verbatim spelling on Windows.
- **Suffix checks**
  - `"doc.md:4"` / `"_part.md:1"` are separator-independent.
  - `feature_review_incident.rs`'s `context.display.ends_with("prompts/_reviews/feature-review.md")` is on a `PathBuf`, a component-wise comparison that accepts `/` on Windows.
- **No `#[cfg]` surface** was added by the feature.

Remote evidence was attempted and blocked on all three rigs. Each blocker is
infrastructure, not a test failure:

- **windows (`build-win-native`)**: `W:` is out of space (`scp ... Failure`,
  `fatal: write error: No space left on device`). I did not clean a shared
  host; see the `storage-strategy` skill.
- **linux (`build-linux`)**: `.cross-check.lock` is held by
  `nightly-reward-spike` (owner `reward-20260914-c3e60d0`, started
  2026-09-14T18:25:30Z). The script gave up after 1800 s. It is likely stale,
  but it is not mine to remove.
- **wsl (`build-win`)**: `kex_exchange_identification: Connection reset by
  peer`. This is the same physical Windows machine as the full disk.

CI will provide the Linux, Windows, and WSL evidence.

### Definition-of-Done audit (8.4)

| # | Criterion | Proven by |
| --- | --- | --- |
| 1 | darkmatter test+lint green; Claudine green | Gates table above (Claudine: green except four failures proven to come from the unrelated prompt edit) |
| 2 | Permanent L1 corpus audit over four dirs, library extractors, both gates | `lib/tests/dasherized_identifier_corpus.rs`: `shipped_corpus_expressions_all_parse`, `corpus_walk_reaches_every_present_surface`, `condition_keys_match_the_shipped_claudine_schema`, `former_breaking_forms_now_lex_as_one_variable`, `gate_b_rejects_the_corpus_failures_found_in_phase_one` |
| 3 | Bare / dotted / bracket kebab parity; arithmetic preserved | `dasherized_identifier_compose.rs` `kebab_key_resolves_the_same_bare_dotted_and_bracketed`, `native_and_quoted_yaml_keys_and_scalar_types_all_resolve`, `subtraction_still_evaluates_where_the_left_operand_cannot_continue_a_name` (`_loop_count - 1`, `4-2`, `length(items)-1`, `items[0]-1`); CLI `compose_resolves_a_kebab_key_three_ways_through_the_binary` |
| 4 | Body and mixed-frontmatter failures fatal regardless of `fail_fast`, typed path+span, no partial output; Lenient subtree unchanged | `compose_expression_failure_contract.rs` `a_body_parse_failure_is_fatal_whatever_fail_fast_says`, `a_body_evaluation_failure_is_fatal_whatever_fail_fast_says`, `a_mixed_frontmatter_*_is_fatal_whatever_fail_fast_says`, `subtree_strictness_keeps_its_contract`; CLI `compose_expression_failures.rs` `compose_emits_no_partially_rewritten_document`; `fatality_characterization.rs` |
| 5 | One message per bad expression; one warning per root per source doc | `compose_diagnostic_identity.rs` `one_replacement_and_one_bad_body_expression_fail_with_one_error`, `one_unknown_root_referenced_ten_times_warns_once`, `one_root_in_two_transcluded_documents_warns_once_per_document`; CLI `compose_reports_an_unknown_root_once_per_source_document` |
| 6 | `dm.expression.unknown_identifier` warns, exit 0, renders empty; every suppression row pinned | `unknown_identifier_warning.rs` `an_undeclared_kebab_root_warns_renders_empty_and_composes`, `every_suppression_row_warns_exactly_where_evaluation_reads_an_unhandled_root`, `set_overrides_are_known_even_when_null_or_empty`, `external_state_is_known`, `a_caller_input_record_is_known_even_without_an_override`, `a_baseline_schema_property_is_known`, `a_matched_trigger_schema_property_is_known`, `explicit_null_and_empty_frontmatter_values_are_known`; CLI `compose_warns_once_for_an_unknown_identifier_and_still_succeeds` |
| 7 | DMLS operand positions at WARNING (body and frontmatter), frontmatter-less exemption; cursor on every kebab position incl. dash, no merge of `foo--bar` / trailing dash / spaced subtraction | `dmls/tests/lsp_session.rs` `unknown_identifier_fires_in_every_operand_position_at_warning_severity`, `a_frontmatter_less_document_never_reports_unknown_identifiers`, `kebab_identifier_navigation_resolves_at_every_cursor_position` (columns 6..=15, `a - b`, `foo--bar`); trailing dash through the shared scan both DMLS sites call: lexer `identifier_prefix::trailing_dash_joins_for_filtering_only_after_an_identifier` |
| 8 | `feature-review.md` would have failed to compose when the typo landed | `lib/tests/feature_review_incident.rs` `the_original_typo_would_have_failed_to_compose`. It uses the shipped prompt bytes with the typo restored in its historical reading, `spec - name`. For both `fail_fast` values it asserts a typed `MarkdownError::Interpolation` naming the file and authored line. `the_typo_is_one_identifier_under_the_current_grammar` shows today's reading warns instead of shipping `{{…}}` |

Note on criterion 7: the DMLS protocol test does not place the cursor on a
trailing dash. That case is pinned where the behavior lives:
`identifier_prefix_start`, which both `completion_partial` and
`value_completion_partial` call. No new test was added.

### Graph change analysis (8.5)

I refreshed the index first (`just gitnexus`: 148,834 nodes, 84.9 s) because
`gitnexus status` reported it stale. Neither scope was partial.

The `critical` risk reflects breadth. This worktree also carries unrelated
in-flight Claudine lifecycle, preflight, and sequence work plus skill edits.

- Symbols: 446 of the 974 `all`-scope changed symbols fall in the 51 `.rs`
  files this plan lists. Some of those files are shared with the other work.
- Processes: 60 of the 77 affected processes touch those symbols. Every one is
  a Darkmatter compose entry point: `Run_stage`, `Rewrite_directive_targets`,
  `Execute_frontmatter_shell_expansion`, `Resolve_prepared_transclusion`,
  `Run_compose_pipeline(_node)`, `Get`/`Get_checked`, and `Classify_options`.
- Coverage: the 8269-test L1 suite and the Claudine suite cover all of them.

The behavior change on these paths (fatal body failures, new warning) is the
spec'd change. It is not an unintended regression.

### Rulings consistency

I spot-checked the rulings against the code:

- R-1: `FunctionBinding` at `functions/mod.rs:73`. Absence predicates resolve
  through `is_absence_predicate`.
- R-4: `pub(crate) enum ExpressionFailurePolicy` at `rewrite.rs:44`.
- R-7: four corpus directories.
- R-8: the quick-fix reads `diag.data` (`code_actions.rs:408`).

The record's "Corrections to the plan's ground truth" section already
reconciles the plan's stale `FunctionBinding` claim. Nothing is open.

### Skill updates

- `.claude/skills/os/build-hosts.md`: `just cross-check` re-splits its
  arguments, so a quoted `-E` filterset dies with a shell syntax error. Call
  `./scripts/cross-check.sh` directly. This cost a run here.
- The `darkmatter` skill needed no change. Phases 1–7 already record the
  grammar, failure, dedup, and unknown-identifier contracts.

### Terminal state (8.6)

Implementation complete, ready for review. The feature was not moved to
`_completed`, `just complete` was not run, and nothing was staged or
committed.

## Implementation of Review Findings #1

> **started at:** 2026-09-18T09:20:26-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-15-dasherized-identifiers/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'High — Typed interpolation errors discard the required authored expression span' at 09:21:09
    - discovery: `anchor_authored_failure` reduced the scanner's byte range to a line through a prefix-equality proof (`authored_line`), so any earlier rewrite (replacement, page block, directive target) dropped even the line; frontmatter mixed-text failures only ever got `SourceRef::OnDisk` (file, no position)
    - discovery: Claudine only constructs/matches `SourceRef::Effective` (`claudine/lib/src/composition/error/tests.rs`, `claudine/cli/tests/error_guards.rs`); `OnDiskLine` consumers are darkmatter-internal (renderer + three lib test files)
    - GitNexus upstream impact: `render_page_blocks` HIGH (18), `scan_and_replace` HIGH (37), `rewrite_value` HIGH (52), `interpolate_frontmatter` CRITICAL (46, 44 direct — almost all in-module unit tests), `interpolate_value` MEDIUM, `interpolation_block` MEDIUM, `anchor_authored_failure`/`rewrite_directive_targets`/`push_on_disk_locus` LOW, `SourceRef`/`with_authored_line`/`run_inline_pre_operation` UNKNOWN (confirmed by grep: darkmatter-internal plus Claudine `Effective` only)
        - mitigation: public signatures of `render_page_blocks`, `apply_replacements`, and `interpolate_frontmatter` are kept; their output bytes are unchanged and the edit record is produced by `pub(crate)` siblings
    - design: replaced `SourceRef::OnDiskLine { context, line }` with `SourceRef::OnDiskSpan { context, span: AuthoredSpan }`; `AuthoredSpan` (new, exported from `darkmatter::markdown`) holds the byte range of the whole `{{ … }}` in the loaded text plus one-based line and character column, computed once by `AuthoredSpan::locate` so the three cannot disagree (private fields, read accessors); `MarkdownError::with_authored_line` became `with_authored_span` and now also anchors keyed (frontmatter) errors
    - design (body, source-map projection): new `compose/body_origin.rs` — `TextEdit { range, replacement_len }` and `BodyOrigin`, a segment map from the current body back to the loaded text, captured once per pipeline node (CRLF-aware, stops at the first other divergence) and carried through text replacement (`apply_replacements_with_edits`), page blocks (`render_page_blocks_with_edits`: false region, true block's open/close lines), and directive targets (`DirectiveTargetRewrite::edits`); a span projects only when every byte was copied from the file, so inserted/replacement text never gets a position; the map is keyed (length + hash) to the exact text it describes, so an untracked body rewrite stops projection instead of mis-projecting; this replaces the prefix-equality `authored_line` (deleted with its three unit tests; the unknown-root candidate locus now uses the same projection before its unique-occurrence fallback)
    - design (frontmatter): `rewrite_value` now returns `LocatedFrontmatterError` (key/index path + scanned string + span within it, built on unwind); new `interpolate_frontmatter_located` is what the pipeline calls; `into_anchored` walks `locate_schema_value` over the loaded frontmatter to the authoring scalar, requires it to decode to exactly the scanned string, and projects the span through quoting/escapes with `yaml_scalar::DecodedScalar`; `interpolate_frontmatter` is now a `#[cfg(test)]` convenience wrapper for the in-module unit tests
    - boundary (documented in the darkmatter skill): body stages that rewrite the body before interpolation must report edits through `body_origin::advance`; frontmatter block scalars (`|`, `>`), multi-line plain scalars, and values changed after loading (schema coercion, shell expansion) stay file-only `OnDisk`; rescan-generated expressions have no authored span by design; in-memory (non-file) documents keep the `Effective` fallback as before
    - rendering: the located block now says `Expression at line: N, column: C`; a keyed frontmatter error with a span shows link + position + the existing focused YAML excerpt; body excerpt unchanged
    - files changed: `darkmatter/lib/src/markdown/types.rs`, `markdown/mod.rs` (export), `compose/body_origin.rs` (new), `compose/mod.rs`, `compose/replacement.rs`, `compose/page_blocks/engine.rs`, `compose/directive_targets.rs`, `compose/inline/{replacement,page_blocks,interpolation}.rs`, `compose/pipeline/{mod,phases}.rs`, `compose/interpolation/{rewrite,mod}.rs`, `compose/frontmatter_interpolation.rs`, `errors/blocks.rs`; tests `lib/tests/{compose_expression_failure_contract,missing_ctx_capture,feature_review_incident}.rs`, `cli/tests/compose_expression_failures.rs`; docs `.claude/skills/darkmatter/{SKILL,compose,errors}.md`, `darkmatter/docs/inline/interpolation.md`, `darkmatter/docs/topics/parsing/index.md`
    - tests added/strengthened: `compose_expression_failure_contract.rs` asserts exact range + line + column + file content for body parse (40..49), body eval (27..41), mixed-frontmatter parse (13..24) and eval (13..25), escaped quoted frontmatter value (23..35, projection through `\"`/`\t`), nested sequence item (28..37), whole-value frontmatter (11..23), directive target (15..27), after a removed page block (41..53), after literal text replacement (66..78); rescan case keeps asserting `OnDisk`; `missing_ctx_capture.rs` page-block case flipped from "no line" to the exact authored span (8:4), CRLF case asserts column; unit tests for `BodyOrigin` (frontmatter offset, CRLF, shift, inserted text, straddling span, stale map) and a page-block edit-replay test; CLI tests now also assert the column
    - non-vacuity: dropping the page-block origin advance makes `a_body_failure_after_a_removed_page_block_keeps_its_authored_span` fail (checked, then restored)
    - gates: `just test` (darkmatter) — 8274 passed, 14 skipped, 0 failed (201.9 s, host load ~20-33); `just lint` (darkmatter, run after the tests, not concurrently) — clean, exit 0, no warnings, zed-dmls wasm check green
    - downstream: `cargo check -p claudine -p claudine-cli --tests` compiles clean (Claudine only uses `SourceRef::Effective`); no other workspace crate names `SourceRef`
- work completed for 'High — Typed interpolation errors discard the required authored expression span' at 09:55:47
- starting the work on 'Medium — Unknown-root candidate deduplication is quadratic' at 09:56:32
    - discovery: `add_unknown_root_candidates` scanned the whole candidate vector per root, and `add_warning` scanned `warnings` per coded warning; `merge` calls `add_warnings`, so merging k single-warning child reports was also quadratic
    - discovery: `ComposeReport::warnings` is `pub` and is also extended directly inside the crate (`pipeline/mod.rs` remote-cache/ICMP/`current` warnings, context merge diagnostics; `shell_blocks`, `inline/shell_expansion.rs`), and `nested.rs` rewrites `stage` in place; Claudine only clones it (`composition/prepare.rs`); no struct literal of `ComposeReport` exists outside its module (`unknown_root_candidates` was already `pub(crate)`), so adding a private field breaks no caller
    - discovery: `unknown_root_candidates` is touched only by `add_unknown_root_candidates`, `merge`, `reconcile` (`mem::take`), and `inline/page_blocks.rs` (`len` + locus rewrite of the tail)
    - GitNexus upstream impact: `add_warning` CRITICAL (72, all darkmatter compose-internal), `add_unknown_root_candidates` CRITICAL (26), `ComposeReport::merge` 8 (ambiguous name; report.rs candidate), `attribute_to_document` LOW (6), `same_issue` UNKNOWN (text search: only caller was `add_warning`), `reconcile` UNKNOWN/LOW (3)
        - mitigation: no public signature changed; first-occurrence order, the first-kept warning, and the `(source, code, subject)` identity are unchanged
    - design (candidates): new `pub(crate) UnknownRootCandidates` wraps the list with a private `HashSet<String>` of roots, so the set cannot drift from the list; the only mutators are `push_first`, `take`, and `relocate_from` (rewrites a tail's locus, never a root); `merge` now inserts child candidates first-wins rather than appending (identical today, since a merged child report carries none)
    - design (warnings): private `WarningIndex` field (`HashMap<(source, code, WarningIdentity), first position>` + `covered` length), used by `add_warning`; because `warnings` stays `pub`, the index is a self-checking cache: it indexes a directly-pushed tail on the next add, rebuilds when the vector shrinks, and verifies every hit against the position before trusting it (rebuilding on mismatch); its `PartialEq` always returns true so report equality is unchanged; `Hash` derived on `WarningIdentity`/`WarningSubject`/`ExpressionOrigin`; `same_issue` replaced by `issue_key`/`has_issue_key`
        - residual: a warning replaced in place outside `add_warning` (same length, e.g. `warnings[i] = w`) goes unseen until a rebuild, so a later same-issue add could be admitted; no code in the workspace does that
    - files changed: `darkmatter/lib/src/markdown/compose/context/report.rs`, `compose/inline/page_blocks.rs` (`relocate_from`), `compose/unknown_identifiers.rs` (`take()`); docs `.claude/skills/darkmatter/SKILL.md` (warning index and its residual), `.claude/skills/darkmatter/compose.md` (hash-backed candidates)
    - tests added (`report.rs`, against a `#[cfg(test)]` thread-local identity-probe counter, not wall-clock): `distinct_unknown_roots_do_linear_identity_work` (1000 distinct roots read by 3 stages, ≤ 4n probes), `distinct_coded_warnings_do_linear_identity_work` (1000 distinct + 1000 repeat adds + 1000 single-warning child merges, ≤ 6n probes), `unknown_root_candidates_keep_first_read_order_and_locus` (interleaved distinct + repeated roots across passes and a merge), `the_warning_index_follows_direct_changes_to_warnings` (direct push, clear, replaced vector)
    - non-vacuity: with the linear scans put back (each comparison counted), the candidate regression reports 2,000,000 probes and the warning regression 2,499,500, and both fail; restored afterward
    - gates: `just test` (darkmatter, `--no-fail-fast`) — 8278 run: 8205 passed, 14 skipped, 73 timed out, all in the known HTTP-client cluster (`remote_fetch`, `provider_network`, remote transclusion, preflight remote, `fn_remote`), host load ~15-23 on the load average; re-running exactly that cluster at `--test-threads 2` passed 119/119; `just lint` (run after the tests, not concurrently) — exit 0, no warnings, zed-dmls wasm check green
    - downstream: `cargo check -p claudine -p claudine-cli --tests` compiles clean
- work completed for 'Medium — Unknown-root candidate deduplication is quadratic' at 10:28:04
- starting the work on 'Medium — The permanent corpus gate does not discover schema-typed expressions' at 10:28:41
    - discovery: DMLS classified Expression-typed values in `dmls/src/providers/frontmatter.rs` (`known_shape` → `def_at_path_ctx` → `expression_atom`), all DMLS-private and bound to `DocumentContext`; the library had no equivalent, only the compiled `darkmatter-expression` format check
    - discovery: no shipped schema declares an `expression` property today; Claudine's `when`/`while`/`until` are typed `string` in `claudine-types.yaml`, so the hard-coded keys protect lifecycle semantics the schema cannot see
    - discovery: DMLS's effective shape is base + configured extension shapes + the document `$schema`; trigger payloads layer into the compiled JSON Schema only, so neither DMLS nor the gate sees trigger-declared properties (unchanged, documented as a residual)
    - GitNexus upstream impact: `known_shape` MEDIUM (49, 14 direct), `def_at_path_ctx` MEDIUM (27), `overlay_root_union` MEDIUM (36), `nested_shape_for_completion`/`expression_values`/`expression_atom`/`merged_inline_object_shape`/`discriminated_arm_shape` LOW, `merge_defs` UNKNOWN (ambiguous name; text search: the moved copy was used only by the two merge helpers, `overlay/schema.rs` keeps its own unrelated copy); all callers DMLS-internal, no HIGH/CRITICAL
    - design: new passive library module `darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs` exporting `effective_property_shape`, `property_def_at_path`, `nested_property_shape`, `expression_atom`, and `frontmatter_expression_values` (+ `FrontmatterExpressionValue { path, expression }`); the shape/arm-selection/merge code moved out of DMLS verbatim in behavior, and DMLS's `known_shape`, `def_at_path_ctx`, and `nested_shape_for_completion` are now thin wrappers over it, so DMLS and the gate share one classification
    - design (gate): `dasherized_identifier_corpus.rs` now has a per-file `classify_file` and `gate_b_failures`; each Markdown artifact resolves its effective schema through `DarkmatterSchemas::with_darkmatter_baseline_json_schema().effective_for` (the same passive resolution DMLS uses, no triggers) and adds `Surface::SchemaExpression` values in condition mode; values holding `$(`/`{{` are skipped as pending, matching the format validator; `CONDITION_KEYS` kept with a comment scoping it to Claudine lifecycle semantics
    - files changed: `darkmatter/lib/src/markdown/schemas/frontmatter_shape.rs` (new), `lib/src/markdown/schemas/mod.rs` (module + exports), `dmls/src/providers/frontmatter.rs`, `lib/tests/dasherized_identifier_corpus.rs`, `.claude/skills/darkmatter/SKILL.md`, `.claude/skills/darkmatter/library-surfaces.md`
    - tests added: lib unit tests (nested + union + native-bool discovery; sequences/nulls/`$schema` excluded; discriminated arm governs a nested key), a doctest, and corpus fixtures `schema_typed_expressions_of_any_name_are_discovered` / `a_malformed_schema_typed_expression_fails_gate_b` (temp-dir prompt whose referenced `$schema` file types `review-gate` as `expression`); `corpus_walk_reaches_every_present_surface` now also requires at least one resolved document `$schema` (no shipped schema-typed expressions exist to count)
    - non-vacuity: disabling the `SchemaExpression` push makes both fixture tests fail (checked, then restored)
    - real corpus: gate stays green; no shipped expression needed changing
    - gates: DMLS `cargo nextest run -p dmls` 681/681; `just test` (darkmatter, `--no-fail-fast`) — 8283 run: 8252 passed, 14 skipped, 31 timed out, all in the known HTTP-client cluster (load ~15-18); that cluster re-run at `--test-threads 2` passed 99/99; `just lint` (after tests, not concurrently) — exit 0; new doctest passes
- work completed for 'Medium — The permanent corpus gate does not discover schema-typed expressions' at 10:58:05
- cross-OS check (orchestrator) at 11:07:41
        - ran `./scripts/cross-check.sh darkmatter --os windows` filtered to the affected binaries (`compose_expression_failure_contract`, `missing_ctx_capture`, `dasherized_identifier_corpus`, `compose_expression_failures`, `body_origin`, `identity_work`)
        - **blocked:** the native Windows host (`build-win-native`) ran out of disk space (`fatal: write error: No space left on device` during fetch; the patch upload failed too), so no tests ran there
        - this is not a code failure; the host is shared, so it was not cleaned up as part of this task. The CRLF handling in the new `body_origin` projection is covered by L1 unit/integration fixtures that ran on macOS, and CI's Windows leg will run the same tests

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1h 47m. During this implementation all 3 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 3 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- residual limitations recorded by the subagents (not deferrals):
        - authored spans are not attached to frontmatter block scalars (`|`, `>`), to multi-line plain scalars, or to values changed after load (coercion, shell expansion); these fall back to file-only `SourceRef::OnDisk`
        - failures found by a rescan of replacement output have no authored span, by design
        - trigger-only schema properties are not visible to the shared `frontmatter_expression_values` extractor, which matches DMLS's behavior from before this change
        - `WarningIndex` cannot detect an equal-length, in-place replacement of an element of the public `warnings` vector. Nothing in the workspace does this today
- the local Windows cross-check was blocked because the host was out of disk space (see above)

The files changed across this cycle are recorded in each finding's entry above; nothing was staged or committed.

## Implementation of Review Findings #2

> **started at:** 2026-09-18T14:01:25-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-15-dasherized-identifiers/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- starting the work on 'High — Block-scalar frontmatter expressions lose required provenance and DMLS analysis' at 14:01:40
    - discovery: the frontmatter parser is `serde_yaml_ng` (libyaml via `unsafe-libyaml`), with tab-normalization and expression-protection fallbacks; DMLS parses with `rlsp-yaml-parser`, whose `FmEntry::value_span` for a block scalar starts at the `|`/`>` header and whose `FmEntry::scalar` already holds the YAML-decoded value (probed for `|`, `>-`, `|+`, multi-line double-quoted with an escaped break, multi-line plain with a trailing comment, and CRLF: all decode as libyaml does)
    - discovery: the library's structural locator (`locate_schema_value`, `schemas/simplified/source.rs`) returned `None` for the **whole** frontmatter as soon as any value was a block scalar (`locate_inline` rejects the header), and silently truncated a mapping at a multi-line plain/quoted continuation line, so compose could not anchor any expression in such a frontmatter, not just the block-scalar one
    - discovery: the existing `DecodedScalar` map records one raw offset per decoded boundary (the end of the previous source run); that is exact while raw bytes are contiguous, but a multi-line scalar skips indentation and trimmed whitespace, so a range start needs the raw *start* of the next decoded byte, or a span would begin in the indentation
    - GitNexus upstream impact: `expression_values` LOW (9, 3 direct, DMLS diagnostics/providers), `locate_inline` LOW (11), `push_mapped` LOW (10), `DecodedScalar` LOW (11; ambiguous name, resolved candidate), `DecodedScalar::project` LOW (9), `BlockLocator` LOW (6), `locate_schema_value` UNKNOWN (0 resolved; text search: DMLS `diagnostics/frontmatter.rs` `smallest_invalid_definition_span`/`standalone_payload_node` and compose `FrontmatterExpressionLocation::project`), `locate_yaml_value` UNKNOWN (ambiguous; text search: the three source-aware schema parsers in `source.rs` plus `locate_schema_value`), `into_anchored` UNKNOWN (text search: the two frontmatter passes in `compose/pipeline/mod.rs`); no HIGH/CRITICAL
    - design (decoder): new public `yaml_scalar::decode_scalar_node(source, start, parent_indent) -> Option<(DecodedScalar, end)>` (re-exported from `darkmatter::markdown::schemas`) decodes plain, single-, and double-quoted scalars on one line or several (line folding, escaped line breaks, escapes) and literal `|` / folded `>` block scalars with `-`/`+`/clip chomping and explicit indentation indicators, porting libyaml's scanner rules (`scan_block_scalar`, `scan_flow_scalar`, `scan_plain_scalar`); `\r\n` and `\n` both decode to `\n`; `parent_indent` is the column of the holding key or `-`; tags, aliases, anchors, and flow collections return `None`
    - design (map): `DecodedScalar` gained a private per-byte `decoded_starts` beside the existing end map, so `project` starts a range at its first authored byte instead of in skipped indentation; for the existing single-line decoders the two maps coincide (`DecodedScalar::contiguous`), so `raw_offset`, `decoded_offset`, and every existing projection are unchanged
    - design (locator): `BlockLocator::inline_value` sends block headers, quoted values, and plain values with a more-indented continuation line through `decode_scalar_node` and consumes the continuation lines; one-line plain values and flow collections keep the old `locate_inline` path, so schema source maps are unchanged for them
    - design (compose): `FrontmatterExpressionLocation::project` tracks the parent key/`-` column while walking the located tree and decodes with `decode_scalar_node` over the YAML extent the parser read (the parser joins the YAML lines without the last terminator, so a clipped/kept block scalar ending the frontmatter has no final newline there; decoding the whole loaded text instead broke three cases and was caught by the new tests); the decoded==scanned check still gates every projection
    - design (DMLS): `expression_values` decodes each value with the same `decode_scalar_node` at `value_span.start` (rlsp's span starts at the block header) with the key's column as parent indent, and keeps the value only when the decoded text equals the parser's `FmEntry::scalar`; the map now carries document offsets, so `ExpressionValue::project`/`decoded_offset` no longer add `value_span.start`; there is no DMLS-side decoder
    - files changed: `darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs`, `schemas/simplified/{mod,source}.rs`, `schemas/mod.rs` (export), `lib/src/markdown/compose/frontmatter_interpolation.rs`, `darkmatter/dmls/src/providers/frontmatter.rs`; tests `lib/tests/compose_expression_failure_contract.rs`, `cli/tests/compose_expression_failures.rs`, `dmls/tests/lsp_session.rs`; docs `.claude/skills/darkmatter/{SKILL,compose,errors,dmls}.md` (surgical edits on top of the earlier unstaged ones)
    - tests added: 9 `yaml_scalar` unit tests, each comparing the decoded text with `serde_yaml_ng`'s value for the same YAML (literal and folded × clip/strip/keep with empty and more-indented lines, a folded expression spanning lines, explicit indentation indicators including a nested `>1-`, CRLF for four headers, multibyte text before the expression, multi-line double-quoted with `\t`/`é`/escaped break/empty line plus CRLF, multi-line single-quoted and plain with a trailing comment plus CRLF, unmodeled nodes return `None`); 6 `compose_expression_failure_contract` tests asserting exact `OnDiskSpan` range + line + character column under both `fail_fast` settings (`|` parse failure 39..50 at 5:5, `>-` eval failure after `café —` 33..45 at 4:10, CRLF `|+` 24..33 at 4:5, nested `|2` 26..38 at 4:6, multi-line plain 23..32 at 3:8 and double-quoted 26..38 at 5:3, and a single-line value after a block scalar 30..39 at 4:10, which previously lost its span because the locator gave up on the whole frontmatter); CLI `compose_fails_on_a_block_scalar_frontmatter_failure_with_its_locus` asserts `Expression at line: 5, column: 5`; LSP session `unknown_identifiers_in_block_and_multi_line_expression_values_warn_at_their_own_ranges` (LF and CRLF documents: `|`, `>-`, multi-line double-quoted, and `|2-` with `"naïve café"` before the identifier; six identifiers in binary, fallback, call-argument, and both ternary-branch positions, all WARNING at exact UTF-16 ranges)
    - non-vacuity (each broken, run, then restored and diffed clean): DMLS back on `decode_scalar` over `value_span` → the LSP test fails; compose back on `decode_scalar_at` → 5 of the 6 new contract tests fail; locator back on `locate_inline` → all 6 fail; decoder folding a break to `\n` and range starts taken from the end map → 5 of the 9 new unit tests fail
    - first `just test` run found one real regression: `meta_schema_phase6::source_aware_v1_presentation_boundary_is_closed` pins that source-aware **schema** parsing rejects block scalars (the frozen v1 grammar in `docs/topics/schema-definition.md`), and the shared locator change had widened it; fixed by making multi-line location opt-in: `BlockLocator` carries a `multi_line_scalars` flag, `locate_schema_value` and the three source-aware schema parsers keep it off (unchanged behavior, including for DMLS's `smallest_invalid_definition_span`), and a new crate-private `locate_frontmatter_value` turns it on for the compose anchor only; `compose.md` names the new locator
    - gates: `just test` (darkmatter, `--no-fail-fast`, after the schema-boundary fix): 8301 run, 8221 passed, 14 skipped, 0 failed, 80 timed out, all in the known HTTP-client cluster (`remote_fetch` integration/persistent-cache, `provider_network`, remote transclusion, preflight remote collection, `effects`), host load 26 to 39 on 16 cores; re-running exactly those 80 at `--test-threads 2` passed 74 with 6 persistent-cache tests still timing out (load ~26); those 6 at `--test-threads 1` passed 5, and the last one (`strict_failure_without_fallback_errors`) passed alone in 3.3 s. `just lint` (run after the tests, not concurrently): exit 0, clippy `-D warnings` clean for darkmatter, darkmatter-cli, dmls, zed-dmls-cli, and the zed-dmls wasm32-wasip2 check is green
    - downstream: `cargo check -p claudine -p claudine-cli --tests` compiles clean; public API change is additive only (new `darkmatter::markdown::schemas::decode_scalar_node`; `DecodedScalar` gained a private field and had no public constructor)
    - residual limitations: values in a flow collection spread over several lines, aliases, tags, and values changed after loading (coercion, shell expansion) keep the file-only `OnDisk` fallback, and DMLS skips them rather than mis-projecting; DMLS reads the value as rlsp parses it, so a clipped/kept block scalar that ends the frontmatter carries a trailing newline in DMLS but not in composition (the serde parser reads the YAML without its last terminator); a trailing newline does not change how an expression parses. Windows and Linux were not run; CRLF is covered by L1 fixtures at the decoder, compose, and LSP-session levels
- work completed for 'High — Block-scalar frontmatter expressions lose required provenance and DMLS analysis' at 15:39:05
- verification by the orchestrator at 15:49:19
        - `just lint` (darkmatter) re-run after touching `lib/src/lib.rs` and `dmls/src/lib.rs`, so clippy actually re-checked the edited crates rather than reporting cached results: clean for darkmatter, darkmatter-cli, dmls, zed-dmls-cli, and the zed-dmls wasm32-wasip2 check
        - native Windows cross-check (`./scripts/cross-check.sh darkmatter --os windows`, filtered to `compose_expression_failure_contract`, `compose_expression_failures`, `lsp_session`, and the `yaml_scalar` unit tests) **blocked**: `build-win-native` is still out of disk space (`No space left on device` during fetch; the patch upload failed), as in cycle 1, so no tests ran; the shared host was not cleaned up as part of this task
        - OS risk assessment: the only OS-sensitive behavior is CRLF, and the fixtures build CRLF bytes explicitly in the test source, so it is covered by the L1 tests that ran on macOS; CI's Windows leg runs the same tests

### Successful Completion

The implementation of review cycle 2 has completed successfully in 1h 48m. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- residual limitations (not deferrals):
        - values in a multi-line flow collection, aliases, tags, and values changed after loading (coercion, shell expansion) keep the file-only `SourceRef::OnDisk` fallback; DMLS skips them rather than mis-projecting
        - a clipped or kept block scalar that ends the frontmatter carries a trailing newline in DMLS but not in composition; this does not change how the expression parses
        - source-aware **schema** parsing still rejects block scalars (the frozen v1 grammar); multi-line location is enabled only for the frontmatter compose anchor through `locate_frontmatter_value`
- the local Windows cross-check was blocked because the host is out of disk space (see above)

The files changed in this cycle are listed in the finding's entry above. Nothing was staged or committed.

## Implementation of Review Findings #3

> **started at:** 2026-09-18T15:59:09-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-15-dasherized-identifiers/review-3.md'
- this is iteration 3 of the review-to-implement cycle
- starting the work on 'High — Valid tagged expression scalars still lose provenance and DMLS diagnostics' at 16:00:41
        - discovery (probed both parsers in a scratch crate outside the repo): `serde_yaml_ng` delivers `!!str`, `!<tag:yaml.org,2002:str>`, and `!!binary` values as ordinary strings, turns `!!int "5"` / `!!float` / `!!bool` into non-strings, and **rejects the whole frontmatter** for a local tag (`!custom "x"`) or the non-specific `!` (`invalid type: enum, expected any valid JSON value`), so a local tag never reaches composition as a string; aliases arrive resolved, and a redefined anchor resolves to the last definition before the alias
        - discovery: `rlsp-yaml-parser` (DMLS) reports a tagged or anchored scalar's `loc` **after** its tag and anchor (`title: !!str "x"` → `loc` starts at the quote, `!!str |` → at the `|`), so DMLS's direct `decode_scalar_node` call never sees a node property; an alias is a separate `Node::Alias { name, loc }` that DMLS lowers to `FmValueKind::Alias` with no value, which `expression_values` skipped
        - discovery: the library locator (`BlockLocator::value`) already strips a leading `&anchor` but resets the node's span start to the `&`, and leaves a tag inside a plain scalar, so composition's `decode_scalar_node(node.span.start)` started on `&` or `!` and returned `None` for anchored and tagged values alike
        - GitNexus upstream impact: `expression_values` LOW (9 impacted, 3 direct), `expression_value_at` LOW (8, 1 direct), `lower_mapping` LOW (5, 1 direct), `FmEntry` LOW (4, 1 direct), DMLS `expression_diagnostics` LOW (23, 1 direct; ambiguous name, resolved by file); `decode_scalar_node` and `inline_value` UNKNOWN (added in cycle 2, not in the index); text search: `decode_scalar_node` is called by `BlockLocator::inline_value`, compose `FrontmatterExpressionLocation::project`, DMLS `expression_values`, and the `yaml_scalar` unit tests; `inline_value` by `BlockLocator::value` and `BlockLocator::sequence`; no HIGH/CRITICAL
        - discovery (corrects the review): a probe LSP session before any DMLS change showed DMLS **already** reported exact-range warnings and the malformed diagnostic for tagged (`!!str`, verbose, block) and anchored Expression values, because rlsp's span starts after the properties; the only DMLS gap was the alias, which `expression_values` skipped (`FmValueKind::Alias`). The composition half of the finding reproduced as written: tagged, anchored, and aliased values all lost their `OnDiskSpan`
        - GitNexus (continued): `anchor_value_start` LOW (5 impacted, 1 direct: `BlockLocator::value`); this one was run after the edit that replaced it, not before
        - **alias decision: project through the defining scalar, when the anchor is provably unique; otherwise fall back (compose: file-only `OnDisk`, still fatal and typed with its key; DMLS: the alias is skipped).** Why: an aliased expression is authored in exactly one place, so the anchor's scalar is the only honest span, and a permanent "cannot analyze" warning on valid YAML would be a diagnostic the author cannot act on. Soundness without a YAML tokenizer in the library: the decoder resolves `*name` only when the text `&name` (not continuing into a longer name) occurs **exactly once** before the alias. The parser accepted the alias, so a definition exists before it and is itself such an occurrence; if there is only one, it is the definition. A redefinition, or the name repeated in a comment or string, makes two occurrences and resolves nothing, so "last definition wins" can never be mis-projected. The decoded==parser-value gate still decides every projection on top of that. Trade-off accepted: redefined anchors get no span and no DMLS analysis (pinned by tests below) rather than a second behavior (a new `dm.*` code); no code was added, so `codes.rs` and `dmls/docs/diagnostics.md` are unchanged
        - design (decoder): `decode_scalar_node` now skips a tag and an anchor, in either order, before the scalar (`skip_node_properties`), so the map starts at the scalar's own bytes; properties whose scalar starts on a later line return `None`. It never interprets a tag: `!!int "5"`, `!!bool`, `!!float` are non-strings to the parser and a local tag fails the frontmatter parse, so the existing gate rejects them. `*name` goes to `decode_alias_target` (unique-occurrence rule above), which reads the defining node's parent column from its own line (`holder_start`: the key, or the last `-`) and returns the alias token's end; a wrong column can only make the decoded text differ, which the gate rejects
        - design (locator): `BlockLocator::value` already stripped `&anchor`; `anchor_value_start` became `node_properties_end`, which also strips a tag when the existing `multi_line_scalars` opt-in is on, so a tagged block collection (`list: !!seq` + items, `map: &a !!map`) no longer truncates the located frontmatter. Schema parsing keeps the flag off and is unchanged (`meta_schema_phase6` green). The flag was not renamed; its doc names the tag behavior
        - design (compose): no code change in `FrontmatterExpressionLocation::project` beyond its rustdoc; it decodes from the located node's start (the `&`, `!`, or `*`) and the decoder does the rest
        - design (DMLS): `FmEntry` gained `alias_target` (the text of the last scalar anchor defined before the alias, tracked in document order through mappings, sequences, and keys by `record_anchors`); `expression_values` accepts alias entries and gates the decoder's text against it; `expression_diagnostics` reports each authored expression span once, so an Expression-typed anchor and its Expression-typed alias do not double-report (Requirement 5); `expression_value_at` ignores alias entries, so hover/completion on the `*name` token do not read a map that points elsewhere; `yaml_quote` now reads the character before the expression's first authored byte instead of the entry's first character, because for an alias the entry is the `*name` token and the quick-fix edits the anchor's quoted scalar (without this the fix offered `doc['foo--bar']` inside a single-quoted scalar)
        - files changed: `darkmatter/lib/src/markdown/schemas/simplified/{yaml_scalar,source}.rs`, `lib/src/markdown/compose/frontmatter_interpolation.rs` (rustdoc only), `darkmatter/dmls/src/overlay/frontmatter.rs`, `dmls/src/providers/frontmatter.rs`, `dmls/src/diagnostics/frontmatter.rs`; tests `lib/tests/compose_expression_failure_contract.rs`, `cli/tests/compose_expression_failures.rs`, `dmls/tests/lsp_session.rs`; docs `.claude/skills/darkmatter/{SKILL,compose,errors,dmls}.md` (surgical edits on top of the earlier unstaged ones)
        - tests added (`yaml_scalar` unit, each comparing with `serde_yaml_ng`): `a_tag_stays_outside_the_projection_of_the_scalar_behind_it` (`!!str` quoted with `café —` before the expression, plain with a trailing comment, verbose tag single-quoted, `!!str |`, `!!str >-` folded across lines, multi-line quoted, `!!binary`; LF and CRLF; exact start and end offsets), `an_anchor_and_a_tag_in_either_order_stay_outside_the_projection` (incl. a nested `&name |2`), `a_tag_that_changes_the_value_disagrees_with_the_parser` (`!!int`, `!!float`, `!!bool`, `!custom` against the real `FrontmatterMap` parse), `an_alias_projects_into_the_scalar_defining_its_anchor` (quoted, anchor+tag plain, a `- key: &x |-` block, a multi-line plain sequence item, a longer `&x-long` anchor beside `&x`; LF and CRLF; alias token end), `an_alias_whose_anchor_is_not_provably_unique_yields_none` (redefinition, the name in a string, the name in a comment; a definition after the alias is ignored); `nodes_the_decoder_does_not_model_yield_none` updated (undefined alias, tagged collection, doubled tag, property with its scalar on the next line)
        - tests added (`compose_expression_failure_contract`, exact `OnDiskSpan` range + line + character column under both `fail_fast` settings, source never rewritten): `a_tagged_quoted_parse_failure_…` (`!!str "café — {{ upper( }}"` → 28..40 at 2:22, `Parse`), `a_verbose_tagged_plain_evaluation_failure_…` (42..54 at 2:38, `min() requires 2 arguments`), `a_tagged_crlf_block_scalar_failure_…` (33..42 at 4:5), `anchored_failures_…` (`&shared "…"` 25..37 at 2:21; `&shared !!str` plain 26..35 at 2:23; sequence item `!!str &item |-` 43..55 at 5:5), `a_non_string_tag_elsewhere_does_not_hide_a_failing_values_span` (`!!int "5"`, `!!seq [a, b]`, a tagged block sequence, an anchored+tagged block mapping, then the failing value at 92..101, 8:10), `an_alias_failure_spans_the_scalar_defining_its_anchor` (the anchored value fails first in document order at 41..53, 3:25; with the anchor's key deferred through `with_exclude_keys` the **alias key** `title` reports the same span; CRLF block anchor 33..42 at 4:3 reported by `alpha`), `an_alias_of_a_redefined_anchor_claims_no_span` (typed `Interpolation` for key `title`, `SourceRef::OnDisk`); helper `failures_with` added
        - tests added (CLI, through `CliProcessFixture`): `compose_fails_on_a_tagged_frontmatter_failure_with_its_locus` (`title: !!str "prefix {{ upper( }}"` → exit 1, empty stdout, one `MarkdownError:`, `Expression at line: 2, column: 22`), `compose_fails_once_on_an_aliased_frontmatter_failure_at_its_anchor` (`Expression at line: 3, column: 29`, exactly one locus line)
        - tests added (DMLS): LSP session `tagged_anchored_and_aliased_expression_values_are_diagnosed_once_where_authored` (LF and CRLF documents; `!!str` plain, verbose-tag quoted, `!!str |-`, `&reused !!str '…'` with `t5: *reused` also Expression-typed, `!!str &late "\"…\""`, an alias whose anchor sits in an untyped `defaults:` sequence item as a folded block, a redefined `&dup`, and a malformed `!!str '"naïve café" || lower('`; seven identifiers at WARNING, source `darkmatter.frontmatter`, exact UTF-16 ranges with `"naïve café"` before four of them; `anc_both` exactly once; nothing for `dup_*`; one `dm.expression.malformed` WARNING ranged over the whole authored expression inside the quotes), `an_aliased_dash_separated_key_fix_avoids_the_defining_scalars_quote` (range 5:11-19 inside the anchor's scalar, replacement `doc["foo--bar"]`); overlay unit `test_an_alias_resolves_to_the_last_scalar_anchor_defined_before_it`
        - non-vacuity (each broken, run, restored, and compared byte-for-byte with the saved copy): decoder back to returning `None` for `!`/`&`/`*` → 5 of the new/updated unit tests, 5 contract tests, both CLI tests, and the LSP session test fail (the session test only on the alias identifier, consistent with the discovery above: its tagged and anchored assertions pin behavior that already held); locator tags off → `a_non_string_tag_elsewhere_…` fails; DMLS dedupe removed → the session test fails with `anc_both` reported twice; `yaml_quote` reverted → `an_aliased_dash_separated_key_fix_…` fails with `doc['foo--bar']`
        - follow-up found while reviewing the locator change: a tagged empty value (`empty: !!str`) had no deeper line to descend into and would have failed location for the whole frontmatter once tags were stripped; with the opt-in on it now locates as the inline token (the fixture of `a_non_string_tag_elsewhere_…` gained that line, range 105..114 at 9:10; removing the new arm makes that test fail, restored byte-for-byte)
        - gates: `just test --no-fail-fast` (darkmatter, on the final code; host load 34 to 46 on 16 cores): 8318 run, 8244 passed, 14 skipped, 0 failed, 74 timed out, all HTTP-client tests (`remote_fetch` persistent-cache 26 and integration 5, `provider_network` 23, remote transclusion 14, preflight remote collection 4, `fn_remote_tests` 1, `resolve_ctx` on-demand remote fetch 1); re-running exactly those 74 at `--test-threads 2` passed 74 of 74. `meta_schema_phase6` and `dasherized_identifier_corpus` are green in the full run. An earlier full run, before the tagged-empty-value follow-up, had the same totals. A behavior-neutral tidy of the occurrence filter in `decode_alias_target` came after the full run; the `yaml_scalar` unit tests (20) and the contract suite (31) were re-run after it and the constrained rerun and lint compiled it
        - `just lint` (after the tests, not concurrently, with `lib/src/lib.rs`, `dmls/src/lib.rs`, and `cli/src/main.rs` touched first so the crates were really re-checked; no content change left in them): exit 0, clippy `-D warnings` clean for darkmatter, darkmatter-cli, dmls, zed-dmls-cli, and the zed-dmls wasm32-wasip2 check is green
        - downstream: no public darkmatter signature changed (`decode_scalar_node` keeps its signature and accepts more inputs); a text search found no use of `decode_scalar_node`, `locate_schema_value`, or `FmEntry` outside `darkmatter/`, so `cargo check -p claudine -p claudine-cli --tests` was **not run**. `dmls::overlay::frontmatter::FmEntry` gained the public field `alias_target`; its only constructor is `lower_mapping`
        - residual limitations: an alias whose anchor name occurs more than once before it (a redefined anchor, or the name in a comment or string) keeps the file-only `OnDisk` fallback in composition and is skipped by DMLS, by design, rather than mis-projected; an alias of a collection (including `<<: *base` merge keys) and a value reached through one are not projected; a tag or anchor whose scalar starts on the next line (`key: !!str` then an indented value) keeps the fallback; hover and completion do not fire on an alias token, nor inside an anchored scalar whose own key is not Expression-typed; DMLS analyzes a scalar behind a tag the frontmatter parser rejects (`!custom`) or retypes (`!!int`) as text, where composition fails the YAML parse instead; an anchored or tagged **collection** as a sequence item still truncates the located tree (pre-existing). Windows and Linux were not run; CRLF is covered by L1 fixtures built as explicit bytes at the decoder, compose, and LSP-session levels
- work completed for 'High — Valid tagged expression scalars still lose provenance and DMLS diagnostics' at 16:59:55

## Implementation of Review Findings #4

> **started at:** 2026-09-18T18:43:39-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-15-dasherized-identifiers/review-4.md'
- this is iteration 4 of the review-to-implement cycle
- starting the work on 'Medium — DMLS alias provenance performs quadratic document rescans' at 18:45:06
        - discovery (the log's cycle-3 entry lags the code): an Expression-typed alias whose anchor is not provably unique is **not** skipped by DMLS today; `expression_values` keeps it as `AuthoredExpression::AliasToken`, every range collapses onto the `*name` token, identical diagnostics collapse, and no quick-fix is attached. `lsp_session.rs` pins this for **redefined** anchors only (`tagged_anchored_and_aliased_…`: two distinct `*dup` warnings and one `*mal` malformed warning at UTF-16 columns; `a_dash_separated_key_behind_an_unprovable_alias_offers_no_fix`: no `data`, no code action). The "name in a comment or string" form is pinned only for the library's text decoder (`yaml_scalar` unit test), not for DMLS
        - discovery (confirms the review): the prefix search is `decode_alias_target`'s `source[..start].match_indices("&name")`, reached from DMLS through `decode_scalar_node(ctx.text, entry.value_span.start, …)` once per Expression-typed alias, and `diagnostics()` reaches `expression_values` twice per publication (`schema_problem_diagnostics` for the suppression set, `expression_diagnostics` for the walk). Everything else on the per-entry path is line-local (`source_column`, `column`, `holder_start` stop at the previous line break) or O(depth) (`key_path_at`)
        - composition evaluated and left alone: the library path is `LocatedFrontmatterError::into_anchored` → `FrontmatterExpressionLocation::project` → `decode_scalar_node`, called only inside the two `map_err(…)?` arms of `compose/pipeline/mod.rs` (frontmatter passes 1 and 2). A frontmatter failure is fatal and returns, so a composition performs at most one prefix search, O(document), on its way out; `BlockLocator::inline_value` reaches the decoder only through `locate_frontmatter_value`, the same failure path. No hot path rescans, so the text-only unique-occurrence rule stays as the library's alias contract
        - GitNexus upstream impact (index at HEAD `583ea7c`, so symbols added by the uncommitted cycles 2 and 3 are absent): `expression_values` LOW (9 impacted, 3 direct: `expression_diagnostics`, `schema_problem_diagnostics`, `expression_value_at`; then `diagnostics`, `expression_hover`, `hover`, `key_reference_fix`, the provider `diagnostics`, and the unit-test helper `diagnostics_for`), `schema_problem_diagnostics` LOW (23, 1 direct), `lower_mapping` LOW (5, 1 direct), `FmEntry` LOW (4, 1 direct); `decode_scalar_node`, `decode_alias_target`, `holder_start`, `record_anchor`, `record_anchors` UNKNOWN (not indexed), confirmed by text search: `decode_scalar_node` ← `BlockLocator::inline_value`, compose `project`, DMLS `expression_values`, unit tests; `decode_alias_target` ← `decode_scalar_node` only; `holder_start` ← `decode_alias_target` only; `record_anchor`/`record_anchors` ← `lower_mapping` and each other only. No HIGH/CRITICAL
        - design (library): new public `yaml_scalar::decode_alias_definition(source, definition_start) -> Option<DecodedScalar>` (re-exported from `darkmatter::markdown::schemas`), the direct-projection entry point for a caller whose YAML parser already knows the defining node. It reads the parent column from the node's own line (`holder_start`, bounded by that line) and calls `decode_scalar_node`; it never searches and returns `None` on an alias token. `decode_alias_target` keeps its unique-occurrence search and now ends in the same function, so `decode_scalar_node`'s contract for composition and the locator is unchanged and both paths share one holder rule. `holder_start` was generalized so `at` may be the node's first property **or** its scalar text (rlsp reports a node after its tag and anchor): properties running up to `at` are the node's own, anything else after the last `-` is its key. For the old call shape the result is identical except a tagged key (`- !t key: &x …`), which now reads the key's column instead of the `-`'s; the decoded==parser-value gate decides either way
        - design (DMLS overlay): `lower_mapping`'s anchor state became `AnchorDefinition { text, start, redefined }`; `FmEntry` gained `alias_target_start: Option<usize>` (document offset of the defining scalar, after its tag and anchor), set only when the anchor names a scalar defined **once** before the alias. `alias_target` is unchanged
        - design (DMLS provider): `expression_values` decodes an alias with `decode_alias_definition(ctx.text, alias_target_start)` and never calls the decoder on a `*name` token; without a start, or when the decoded text differs from `alias_target`, it keeps the existing `AliasToken` fallback. The gate still decides every projection
        - design (once per publication): `diagnostics()` computes `expression_values` once, next to the single validation report, and passes `&[ExpressionValue]` to `schema_problem_diagnostics` (suppression set) and `expression_diagnostics` (which no longer takes the AST). No other duplicate on that path shares the same data; `KnownRoots::for_document` already computes `known_shape` once. Hover (`expression_value_at`) and the code action's recompute are separate requests and were left alone
        - **alias-behavior decision: keep the pinned fallback for redefined anchors, let the YAML tree decide everything else.** The tree resolves every alias exactly, redefined anchors included, so the text-only unique-occurrence rule is no longer needed for soundness in DMLS. This finding is about work, not behavior, so the two pinned LSP-session behaviors stay byte-for-byte: an anchor redefined before the alias keeps the alias-token range, collapsed duplicates, and no quick-fix (`redefined` withholds the start). Lifting that is a behavior change for a later cycle, not a requirement of this one
        - **observable changes (both unpinned before, both strictly toward exact ranges, both a direct consequence of not searching the text):** (1) an anchor whose name is merely repeated in a comment or a string is no longer "ambiguous" to DMLS; its aliases are diagnosed inside the anchor's scalar, with quick-fixes, instead of on the alias token. Keeping the old outcome would have needed a second, linear text-occurrence index whose only purpose is to degrade a projection the tree proves. (2) an alias of a scalar that starts on the line after its anchor (`key: &x` then an indented value) now projects when the decoded text equals the parser's; the parent column read from that line is the scalar's own, so a multi-line plain scalar in that position still disagrees and keeps the alias token. Composition is unchanged for both (it has no tree): unique-occurrence rule, alias-token span otherwise
        - work counters (the established `IDENTITY_WORK` pattern: thread-local `Cell`, `#[inline]` note function with a cfg'd body, before/after deltas): the library counts source bytes searched for alias anchors (`ALIAS_SEARCH_WORK`, read through `alias_search_work()`); DMLS counts `expression_values` computations (`EXPRESSION_VALUE_SETS`, `cfg(test)` only). The search lives in the library and the regression lives in DMLS, and `cfg(test)` does not cross a crate boundary, so the library counter compiles under `cfg(any(test, feature = "work-counters"))`; dmls enables that feature on a **dev-dependency** only (resolver 2 keeps it out of every non-test dmls build), following the `effects-instrumentation` precedent for counters a downstream suite reads. No justfile, CI metadata, or recipe change is needed: any `cargo nextest run -p dmls` gets it
        - files changed (source): `darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs`, `lib/src/markdown/schemas/simplified/mod.rs` and `lib/src/markdown/schemas/mod.rs` (exports), `lib/Cargo.toml` (`work-counters` feature), `dmls/Cargo.toml` (dev-dependency feature), `dmls/src/overlay/frontmatter.rs`, `dmls/src/providers/frontmatter.rs`, `dmls/src/diagnostics/frontmatter.rs`; tests in `yaml_scalar.rs`, `dmls/src/overlay/frontmatter.rs`, `dmls/src/diagnostics/frontmatter.rs`, `dmls/tests/lsp_session.rs`; skills `.claude/skills/darkmatter/{dmls,compose}.md` (surgical edits on top of the earlier unstaged ones)
        - tests added (`yaml_scalar` unit): `a_known_definition_decodes_like_the_searched_one` (seven fixtures × LF/CRLF, each entered at the node's first property and at its scalar text, equal to the searched map and to `serde_yaml_ng`: quoted with `café` before the expression, tag-then-anchor plain, `- key: &x |-`, a multi-line plain `- &x` item, `- &x !!str |1-` and nested `- - !!str &x >2-` whose explicit indentation needs the `-` column, and `&k key: &x |2-` under an anchored key), `a_known_definition_needs_no_unique_anchor_name` (redefined, name in a string, name in a comment; an alias token yields `None`), `known_definitions_are_decoded_without_searching_the_document` (200 and 400 distinct anchors: the searched path costs at least N × the anchor section in bytes, the known-definition path exactly 0, and both produce equal maps)
        - tests added (DMLS): unit `distinct_expression_aliases_publish_without_searching_the_document` (150 and 300 distinct anchors in an untyped sequence, one Expression-typed alias each, through the real overlay and `diagnostics()`: one warning per alias on its anchor's line, `alias_search_work` delta exactly 0, `EXPRESSION_VALUE_SETS` delta exactly 1); overlay units extended/added (`alias_target_start` is the defining scalar for a single definition, `None` for a redefinition, a collection, and a non-alias; it skips tag and anchor in either order, with a multibyte key, LF and CRLF); LSP session `aliases_are_analyzed_in_the_definition_the_yaml_parser_resolved` (LF and CRLF: `- &seq |2-` with `"naïve café"` before the identifier, a multi-line plain scalar in a nested mapping, an anchor whose name a comment and a string repeat, a scalar on the line after its anchor, an anchor redefined only **after** its alias, an alias of a flow mapping reporting nothing, and an alias read after the redefinition staying on its token with the later definition's identifier; WARNING, source `darkmatter.frontmatter`, exact UTF-16 ranges). The existing session tests for tags, anchors, redefinitions, the quote-aware fix, and the no-fix fallback pass unchanged
        - non-vacuity (each broken, run, restored with `cp` from a saved copy and `cmp`-verified byte-for-byte): DMLS back on `decode_scalar_node` at the alias token → the work-count test fails with `1101800` bytes searched for 150 aliases (expected 0) and the new session test fails (the comment/string-mentioned and next-line anchors fall back to their alias tokens), while the three pre-existing alias session tests still pass, so nothing else would have caught it; a second `expression_values` call restored in `diagnostics()` → the work-count test fails with 2 sets (expected 1); a search put back inside `decode_alias_definition` → the library work test fails with `588295` bytes for 200 definitions; `holder_start` back on its old rule → `a_known_definition_decodes_like_the_searched_one` fails on the `- &x` multi-line item entered at its scalar text
        - follow-up found while reviewing the diff: the alias-token duplicate check in `expression_diagnostics` compared each finding with **every** diagnostic published so far (`out.contains`), which is quadratic in the diagnostic count for a document with many redefined-anchor aliases. Only the value's own findings can be identical (the token range belongs to one entry), so the check now reads `out[first..]`; behavior is unchanged and the pinned `*dup` collapse still passes. GitNexus: `expression_diagnostics` (dmls) LOW (23 impacted, 1 direct)
        - gates: `just test --no-fail-fast` (darkmatter; host load rose from 7 to 96 on 16 cores during the run, another session was also building): 8327 run, 8256 passed (17 slow), 14 skipped, **0 failed**, 71 timed out, all HTTP-client tests (`remote_fetch` persistent-cache 26 and integration 5, `provider_network` 23, remote transclusion 12, preflight remote collection 4, `resolve_ctx` 1); re-running exactly those 71 at `--test-threads 2` (load still ~70): 71 of 71 passed. The `out[first..]` follow-up is dmls-only and came after the full run; the whole dmls package was re-run on the final code (`cargo nextest run -p dmls --features effects-instrumentation --no-fail-fast`): 689 run, 689 passed. `dasherized_identifier_corpus`, `meta_schema_phase6`, `compose_expression_failure_contract`, and the CLI `compose_expression_failures` are green in the full run
        - `just lint` (after the tests, not concurrently; the edited crates were re-checked, not served from cache): exit 0, clippy `-D warnings` clean for darkmatter, darkmatter-cli, dmls, zed-dmls-cli, and the zed-dmls wasm32-wasip2 check; the library lints without `work-counters`, so the feature-off shape of `note_alias_search` is covered. `git diff --check` is clean and `Cargo.lock` did not change (the dev-dependency adds a feature, not a package)
        - GitNexus `detect_changes` (scope `all`): 2073 changed symbols in 627 files, risk `critical`, listing `truncated: true`, `partial` unset; that is the whole branch's uncommitted work, not this cycle. The 345 KB result was filtered with `jq` rather than read in full: in the files this cycle touched it lists `diagnostics`, `expression_diagnostics`, `schema_problem_diagnostics`, `yaml_quote`, `lower_mapping`, `FmEntry`, `expression_values`, `ExpressionValue`, and neighbors shifted by line (the index is at HEAD, so `yaml_scalar.rs`'s cycle-2/3 symbols are not attributed). Scope `unstaged`: 326 symbols in 63 files, 25 affected flows, all from earlier cycles' compose changes (`status_block`, `interpolate_value`, `run_compose_pipeline_node`); the pre-edit impact runs reported 0 affected processes for every symbol edited here
        - downstream: the public darkmatter change is additive (`decode_alias_definition`; `alias_search_work` only under the off-by-default `work-counters` feature); no existing signature changed. A text search found no use of `decode_scalar_node`, `decode_alias_definition`, or DMLS's `FmEntry`/`alias_target` outside `darkmatter/` (tree-hugger's `alias_target` is its own symbol), so no downstream `cargo check` was run. `FmEntry` gained the public field `alias_target_start`; its only constructor is `lower_mapping`
        - residual limitations: an alias read after its anchor was redefined stays on the alias token with no quick-fix, by decision (see above), although the tree could place it exactly; a multi-line plain scalar that starts on the line after its anchor keeps the alias token, because the parent column cannot be read from that line; the work counter measures declared search work (bytes handed to the anchor search), as `IDENTITY_WORK` does, not every byte the decoder reads, so a new scan added elsewhere without a note would not move it; `expression_value_at` still computes the whole set per hover or completion request (one request, no aliases decoded twice, not a publication path); composition keeps one O(document) anchor search on its fatal failure path. No `cfg(windows)` or path code was touched; Windows and Linux were not run; CRLF is covered by L1 fixtures built in the test source at the decoder, overlay, and LSP-session levels, and `holder_start` stops at either `\n` or `\r`
- work completed for 'Medium — DMLS alias provenance performs quadratic document rescans' at 19:26:45
- orchestrator verification on the final code at 19:35:21 (host load back to 6 on 16 cores, nothing else running): `cargo nextest run -p dmls` 689 of 689 passed; the library's `yaml_scalar` unit tests, `compose_expression_failure_contract`, and `dasherized_identifier_corpus` 65 of 65 passed; the CLI `compose_expression_failures` binary 8 of 8 passed; `just lint` (with `lib/src/lib.rs` and `dmls/src/lib.rs` touched first so both crates were really re-checked) exited 0, including the `wasm32-wasip2` check; `git diff --check` is clean

### Successful Completion

The implementation of review cycle 4 has completed successfully in 53m. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- no performance measurement was deferred: the regression asserts counted work (bytes searched for anchors, and expression-value sets computed per publication) at two sizes, so it needs no wall-clock timing and is not sensitive to host load
- one observable DMLS change needs the author's attention (it is not pinned by any earlier test and composition is unchanged):
        - an anchor whose name is merely repeated in a comment or a string is no longer treated as ambiguous by DMLS, because the YAML tree proves which definition the alias reads; its aliases are now diagnosed inside the anchor's scalar, with quick-fixes, rather than on the alias token. Composition has no YAML tree and keeps the text-only unique-occurrence rule, so the two surfaces differ for this one shape
        - an alias of a scalar that starts on the line after its anchor now projects when the decoded text equals the parser's
        - an anchor **redefined** before its alias deliberately still lands on the alias token with no quick-fix, as cycle 3 pinned
- residual limitations recorded by the subagent (not deferrals):
        - `expression_value_at` still computes the whole expression-value set for each hover or completion request; that is one request, not a diagnostics publication
        - composition keeps one O(document) anchor search, on its fatal failure path only
        - the work counter measures declared search work, as `IDENTITY_WORK` does; a new scan added elsewhere without a counter note would not move it
- Windows, WSL2, and Linux were not run locally: no `cfg(windows)`, path, or process code changed, and CRLF is covered by L1 fixtures built as explicit bytes at the decoder, overlay, and LSP-session levels, which CI's other legs run unchanged
- drift noticed and left alone: the cycle 3 section above has no `### Successful Completion` entry, this file has no `implementation_3` frontmatter key, and `review-3.md` has no `log` / `implemented_by` keys

The files changed in this cycle are listed in the finding's entry above. Nothing was staged or committed.

## Implementation of Review Findings #5

> **started at:** 2026-09-18T20:23:59-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/features/2026-09-15-dasherized-identifiers/review-5.md'
- this is iteration 5 of the review-to-implement cycle
- starting the work on 'Medium — Shared alias targets are still cloned and decoded per alias' at 20:24:30
        - impact (GitNexus upstream): `FmEntry` LOW (4 impacted, direct: `Overlay`), `lower_mapping` LOW (5), `expression_values` LOW (9; direct: `expression_diagnostics`, `schema_problem_diagnostics`, providers), `ExpressionValue` LOW (6); `record_anchor`, `AnchorDefinition`, `AuthoredExpression` UNKNOWN (private, not indexed) — text search confirms they are private to `dmls/src/overlay/frontmatter.rs` / `dmls/src/providers/frontmatter.rs`, and `alias_target` has no consumer outside those two files
        - design: `AnchorDefinition::text` and `FmEntry::alias_target` become `Option<Arc<str>>`; the anchor's scalar is copied once when the anchor is recorded and every alias entry holds an `Arc` clone of it; `expression_values` keeps a per-computation `HashMap<alias_target_start, Option<Arc<DecodedScalar>>>`, so each defining offset is decoded and text-checked once and each alias gets an `Arc` clone; `AuthoredExpression::{Exact(Arc<DecodedScalar>), AliasToken(Arc<str>)}`
        - files changed: `darkmatter/dmls/src/overlay/frontmatter.rs` (`FmEntry::alias_target` and `AnchorDefinition::text` are `Option<Arc<str>>`; doc notes the sharing), `darkmatter/dmls/src/providers/frontmatter.rs` (per-call `definitions` map keyed by `alias_target_start`; `AuthoredExpression::Exact(Arc<DecodedScalar>)` / `AliasToken(Arc<str>)`; new `#[cfg(test)]` `ALIAS_TARGET_DECODES` counter beside `EXPRESSION_VALUE_SETS`; doc sentence on once-per-start decoding), `darkmatter/dmls/src/diagnostics/frontmatter.rs` (new test), `.claude/skills/darkmatter/dmls.md` (two surgical additions)
        - test added: `diagnostics::frontmatter::tests::many_aliases_of_one_long_expression_share_its_text_and_one_decode` — 200 aliases of one anchored Expression scalar at 100 and 400 `||` terms; asserts every alias entry's `alias_target` is `Arc::ptr_eq` to the first, exactly 1 `ALIAS_TARGET_DECODES` per publication, and exactly one unknown-identifier warning on the anchored scalar's line
        - non-vacuity: (1) forcing a cache miss per alias (`definitions.remove(&start)` before lookup) -> FAIL `left: 200, right: 1` on decodes; (2) cloning per alias (`text.as_deref().map(Arc::from)`) -> FAIL "every alias of one anchor shares one stored target"; both files restored from `/tmp/nv-alias/` copies and `cmp`-verified identical
        - gates: `cargo nextest run -p dmls --features effects-instrumentation` 690/690 passed; darkmatter `just test --no-fail-fast` 8328 run: 8261 passed, 67 timed out, 14 skipped — all 67 timeouts were the `darkmatter` lib HTTP-client cluster (remote_fetch, provider_network, preflight remote, remote transclusion, effects catalog) under load average ~14 on 16 cores; re-running exactly those 67 with `--test-threads 2` -> 67/67 passed; `just lint` exit 0; `git diff --check` clean
        - residual: `expression_diagnostics` still calls `parse_condition` on each alias's expression before its `reported` dedup check, so N aliases of one scalar still parse it N times (O(N×L)); the reorder would change which property reports a union-rejected malformed value, so it was left out of this surgical fix and needs an author decision
        - follow-up (coordinator): closed the per-alias parse gap in `expression_diagnostics` (`darkmatter/dmls/src/diagnostics/frontmatter.rs`); the coordinator's behavior-preservation argument was verified against the code (values sharing an `expression_span` decode the same authored bytes, so text and parse result are identical; `known_roots` is per-document; `DecodedScalar::project` is O(1))
        - design: `reported` became `settled` (span skipped before parsing); an `Ok` parse settles its span even when `known_roots` is `None`; an `Err` on a property whose union accepts the value is kept in `unreported_errors: HashMap<span, ParseError>` and reused, not re-parsed, by a later alias whose property rejects it; comment at the set rewritten; new `#[cfg(test)]` `EXPRESSION_DIAGNOSTIC_PARSES` counter at the diagnostics call site (not inside `parse_condition`, which hover and finding generation also call)
        - tests: `many_aliases_of_one_long_expression_share_its_text_and_one_decode` now also asserts exactly 1 parse; new `a_malformed_alias_target_is_parsed_once_and_reported_by_the_rejecting_alias` — 200 aliases under `[expression, string]` properties plus `last: *shared`, at 100/400 terms, with `last` typed `expression` (exactly one `dm.expression.malformed`, on the anchored line) and `[expression, string]` (none); exactly 1 parse in every case
        - non-vacuity: (A) re-parsing on every settled span -> FAIL parses `200` vs `1`; (B) discarding the kept error so it re-parses -> FAIL parses `201` vs `1`; (C) settling the span instead of keeping the error -> FAIL malformed lines `[]` vs `[205]`; restored from `/tmp/nv-alias/diagnostics-fixed.rs` and `cmp`-verified after each
        - skill: `.claude/skills/darkmatter/dmls.md` sentence on `expression_diagnostics` now says it parses once and names the new counter and test
        - gates: `cargo nextest run -p dmls --features effects-instrumentation` 691/691 passed; `just lint` exit 0; `git diff --check` clean
        - residual: none known for the review's quadratic concern; `KnownRoots::findings` still runs once per reported expression, which is inherent
- work completed for 'Medium — Shared alias targets are still cloned and decoded per alias' at 20:47:30
- orchestrator verification on the final code at 20:47:24 (host load ~16 on 16 cores): `cargo nextest run -p dmls --features effects-instrumentation` 691 of 691 passed; `just lint` completed through its final `wasm32-wasip2` check; `git diff --check` is clean

### Successful Completion

The implementation of review cycle 5 has completed successfully in 24m. During this implementation all 1 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 1 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- no performance measurement was deferred: the regressions assert counted work (one stored target via `Arc::ptr_eq`, one target decode, and one expression parse per publication, at two scalar lengths with 200 aliases), so they need no wall-clock timing and are not sensitive to host load
- scope note: the finding named cloning and decoding; the orchestrator also had the subagent close the remaining per-alias **parse** in `expression_diagnostics`, which kept the N × L cost alive after the decode fix. The early skip is behavior-preserving (a span already settled always hit `continue` before), and a parse error seen under a union-accepting property is cached so a later rejecting alias still reports it exactly once
- Windows, WSL2, and Linux were not run locally: only DMLS overlay/provider/diagnostics code changed, with no `cfg(windows)`, path, or process code

The files changed in this cycle are `darkmatter/dmls/src/overlay/frontmatter.rs`, `darkmatter/dmls/src/providers/frontmatter.rs`, `darkmatter/dmls/src/diagnostics/frontmatter.rs`, and `.claude/skills/darkmatter/dmls.md`. Nothing was staged or committed.
