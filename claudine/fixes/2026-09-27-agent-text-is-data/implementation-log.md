---
spec: /Volumes/coding/wt/rusty-biscuit/feat-schema-enhancement/claudine/fixes/2026-09-27-agent-text-is-data/spec.md
plan: claudine/fixes/2026-09-27-agent-text-is-data/plan.md
implemented_by: claude/opus
started_phase: "1"
source_files_during_phase_1:
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/main.rs
    - darkmatter/cli/tests/l1/compose_value_provenance.rs
    - darkmatter/cli/tests/l1/main.rs
docs_updated_during_phase_1:
    - claudine/fixes/2026-09-27-agent-text-is-data/plan.md
docs_created_during_phase_1:
    - claudine/fixes/2026-09-27-agent-text-is-data/implementation-log.md
    - claudine/fixes/2026-09-27-agent-text-is-data/inventory.md
    - claudine/fixes/2026-09-27-agent-text-is-data/spike-s1-body-provenance.md
    - claudine/fixes/2026-09-27-agent-text-is-data/spike-s2-yaml-leaf-spans.md
    - claudine/fixes/2026-09-27-agent-text-is-data/spike-s3-token-lexer.md
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
    - darkmatter-cli
    - claudine-cli
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/value_origin.rs
    - darkmatter/lib/src/markdown/compose/body_origin.rs
    - darkmatter/lib/src/markdown/compose/util.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/context/report.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/tests.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion/tests/execution_tests.rs
    - darkmatter/lib/src/markdown/compose/interpolation/mod.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/compose/inline/interpolation.rs
    - darkmatter/lib/src/markdown/compose/inline/page_blocks.rs
    - darkmatter/lib/src/markdown/compose/inline/replacement.rs
    - darkmatter/lib/src/markdown/compose/inline/shell_expansion.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/phases.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/nested.rs
    - darkmatter/lib/src/markdown/compose/parse_utils.rs
    - darkmatter/lib/src/markdown/compose/block_pairs.rs
    - darkmatter/lib/src/markdown/compose/directives_api.rs
    - darkmatter/lib/src/markdown/compose/directive_targets.rs
    - darkmatter/lib/src/markdown/compose/replacement.rs
    - darkmatter/lib/src/markdown/compose/link_resolve.rs
    - darkmatter/lib/src/markdown/compose/page_blocks/engine.rs
    - darkmatter/lib/src/markdown/compose/page_blocks/parser.rs
    - darkmatter/lib/src/markdown/compose/shell_blocks/mod.rs
    - darkmatter/lib/src/markdown/compose/shell_expansion/parser.rs
    - darkmatter/lib/src/markdown/compose/transclusion/engine.rs
    - darkmatter/lib/src/markdown/compose/transclusion/mod.rs
    - darkmatter/lib/src/markdown/compose/transclusion/parser.rs
    - darkmatter/lib/src/markdown/compose/transclusion/types.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/mod.rs
    - darkmatter/lib/src/markdown/compose/toc_linking/parser.rs
    - darkmatter/lib/src/markdown/compose/file_links/mod.rs
    - darkmatter/lib/src/markdown/compose/file_links/parser.rs
    - darkmatter/lib/src/markdown/compose/expression/lint.rs
    - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/src/markdown/reference/graph.rs
    - darkmatter/lib/src/markdown/reference/validate.rs
    - darkmatter/lib/src/markdown/types.rs
    - darkmatter/lib/tests/l1/data_origin.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/lib/tests/l1/benchmark_fixtures.rs
    - darkmatter/lib/tests/l1/compose_expression_failure_contract.rs
    - darkmatter/lib/tests/l1/compose_phase6.rs
    - darkmatter/lib/tests/l1/missing_ctx_capture.rs
    - darkmatter/lib/tests/l1/ternary_integration.rs
    - darkmatter/benchmarks/fixtures/compose_interpolation_heavy.md
    - darkmatter/benchmarks/generate.sh
    - darkmatter/benchmarks/manifest.yaml
    - darkmatter/cli/src/commands/compose.rs
    - darkmatter/cli/tests/l1/compose_interpolation.rs
    - darkmatter/cli/tests/l1/compose_value_provenance.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/interpolation.md
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/fixes/2026-09-27-agent-text-is-data/plan.md
    - claudine/fixes/2026-09-27-agent-text-is-data/inventory.md
    - claudine/fixes/2026-09-27-agent-text-is-data/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/SKILL.md
    - .claude/skills/darkmatter/compose.md
    - .claude/skills/darkmatter/errors.md
---

# Implementation Log for 2026-09-27-agent-text-is-data (6 phases)

## Phase 1

- Started 2026-09-28. Phase 1 is rulings, red reproduction tests, spikes S1–S3,
  and inventories I1–I2. No production code is changed in this phase.

### N1 verification (installed `md`, 2026-09-28)

- `md compose repro.md --set '{"note":"see {{…}} siblings"}'` → `MarkdownError:
  interpolation failed … The note frontmatter property failed to evaluate '…' …
  Defined in: /private/tmp/n1check/repro.md` plus a document excerpt (the false
  attribution R5 removes).
- `--set '{"note":"$(echo INJECTED)"}'` → `Approval required for 'echo INJECTED'`
  (non-interactive, whitelist suggestion printed).
- `esc.md` → `Body: fixed claudine`.
- N1 stands: command-line setters remain templates; only attribution changes.

### Darkmatter red and regression tests

New file `darkmatter/cli/tests/l1/compose_value_provenance.rs`, declared in
`darkmatter/cli/tests/l1/main.rs`. Every test spawns `md` through `CliProcessFixture`.

| Test | State | Observed today |
| ---- | ----- | -------------- |
| `escaped_expression_in_frontmatter_stays_literal_in_body` | `#[ignore = "red until phase 2"]` | exit 0, stdout `Body: fixed claudine` (wants `Body: fixed {{ area }}`) |
| `escaped_non_expression_in_frontmatter_stays_literal_in_body` | `#[ignore = "red until phase 2"]` | exit 1, `interpolation failed … parse error: Unexpected character: '…'` |
| `set_value_with_malformed_template_still_fails` | passing (N1 guard) | fails with `interpolation failed`, names `note` |
| `set_value_template_still_fills_in` | passing (regression) | `X: t` |
| `set_value_shell_command_requires_approval_when_non_interactive` | passing (N1 guard) | `Approval required for 'echo INJECTED'`, never executed |

- Side note: `md` error output stays ANSI-colored under the fixture's `NO_COLOR=1`.
  This is not part of this fix; recorded for follow-up.

### Spike S3 (`spike-s3-token-lexer.md`)

- Today every token placement fails with `Expected end of expression, found ':'
  at position 5`, because `!` is unary-not.
- N5's prefix check is enough **against expression parsing** only if it lives in
  two places: `ExpressionFinder::scan` (`lexer.rs:133`), which DMLS, preflight,
  lint, and Claudine templates all read, and the interpolation entry points in
  `rewrite.rs` together with the nested-span lint (`lint.rs:227`). Both must fail
  on a token even under lenient policy.
- The prefix check does **not** cover:
  - decoded `$(…)` text reaching the resolved-value shell checks. N2/N4 must
    cover this.
  - the `contains("{{")` "pending" checks in schema validation and DMLS. These
    need decoding first (N6), and a Data value must not count as pending.
  - Claudine's `reject_surviving_spans`. Covered by N10.
  - `sequence/grammar.rs:122`.
- Whole-leaf matching must not trim whitespace, so it cannot reuse
  `whole_value_span`.

### Claudine red reproduction tests

New file `claudine/cli/tests/l1/agent_text_is_data.rs`, declared in
`claudine/cli/tests/l1/main.rs` as `#[cfg(unix)] mod agent_text_is_data;` (the
fake-agent convention). Every test is `#[ignore = "red until phase N"]`. Each
spawns through `CliProcessFixture` / `InlineAgentStub` with a fake `goose`, so
fixture audio defaults apply.
`just test` passed (7480 passed, 19 skipped) and `just lint` passed.

| Test | Phase | Observed failure today |
| ---- | ----- | ---------------------- |
| `loop_last_output_with_template_syntax_stays_raw` | 4 | `interpolation failed — The _loop_last_output frontmatter property failed to evaluate '…'`, attributed to the document |
| `loop_last_output_with_whole_value_shell_stays_raw` | 4 | `ShellExpansionError: command not pre-approved — echo INJECTED, Origin: frontmatter._loop_last_output` |
| `loop_last_output_with_template_and_shell_stays_raw` | 4 | same interpolation error; also asserts directory `x` survives |
| `sequence_outputs_with_template_syntax_stay_raw` | 4 | step 2 preflight tries to evaluate `ctx.repo` from the output (`Repo capture group … did not capture`) |
| `sequence_parallel_group_nested_output_stays_raw` | 4 | same, from nested entry `[0]` of a parallel group (a shell task produces the output to avoid a racy shared fake-agent counter) |
| `inline_agent_added_frontmatter_survives_a_second_run` | 5 | second run: `The summary frontmatter property failed to evaluate '…'` |
| `lifecycle_field_from_agent_written_file_is_sent_verbatim` | 4 | surviving-span refusal: `rendered text still contains '{{ title }}' after every interpolation pass` |
| `lifecycle_stack_message_from_agent_written_file_is_sent_verbatim` | 4 | stack action result re-expanded: `interpolation of '…' failed` |
| `lifecycle_set_from_agent_data_stays_inert_on_next_preparation` | 4 | silent corruption: `Carried: [agent said authored-title]` |
| `lifecycle_proxy_with_from_agent_data_stays_inert_in_target` | 4 | silent corruption: `Note: [agent said t]` |

Differences from the brief:

- Lifecycle message rows use `info`, whose output reaches stderr, instead of
  `message:`, which needs a configured route. The two shapes fail differently, so
  each has its own test.
- The `set:` and `proxy.with:` tests read the agent-written log rather than
  `_loop_last_output`, so the loop failure cannot mask them.
- The Darkmatter `esc.md` row is covered in `darkmatter/cli/tests/l1/`.

### Spike S1 (`spike-s1-body-provenance.md`)

- **Chosen: (a)** tracked data byte ranges beside `BodyOrigin`. Every stage that
  scans for instructions already rewrites through span replacements. The stages
  that cannot remap (Cleanup, reflow, Normalization, relevel/wrapper/exclude) run
  after the last scan. Drop the ranges right after the transclusion directive
  parse; they need not survive to Finalization.
- (b) placeholders break link resolution, heading slugs, table widths, relevel,
  and the compose cache, and need their own escaping. Rejected.
- Losing ranges must be an error. Today `BodyOrigin` is dropped silently on
  mismatch.
- Frontmatter key-to-key chains resolve in dependency order, not by rescanning
  (`a: "{{ b * 2 }}"`, `b: "{{ c + 1 }}"`, `c: 5` gives `a: 12` in either
  declaration order). **N2 needs no topological order.** A data value is a seed
  even when it contains `{{`.
- Cycles (`a: "{{ b }}"`, `b: "{{ a }}"`) silently give `null` today. Unchanged
  here; this would be a separate fix.
- Findings that amend rulings: a body clause for N4, and extra N2 carriers
  (reference graph and preflight, the transclusion cache key, and the child
  external state). Both are recorded in the plan. The spike also raises a
  data-link decision (a missing relative link in data text); see the plan's
  N14.

### Spike S2 (`spike-s2-yaml-leaf-spans.md`)

- **API:** `locate_frontmatter_value` (`markdown/schemas/simplified/source.rs`,
  now `pub(crate)`) plus `decode_scalar_node` (`yaml_scalar.rs`, `pub`). Wrap
  them in a new `locate_frontmatter_leaves(document, paths)` in
  `markdown/hash/write.rs`. `parse_text_frontmatter` and `FrontmatterDelta` are
  top-level only. The `biscuit-terminal` locator returns lines, not spans. No
  workspace YAML crate exposes positions, and no parser dependency is added.
- **Supported:** top-level scalars in every quoting style with trailing
  comments, nested maps, indented sequences of scalars and of maps, multi-line
  scalars, every block-scalar header, CRLF, and quoted items in single-line
  flow collections.
- **Unsupported, and failing safely:** anchors, aliases, and tags on an owned
  leaf; `<<` merges; unquoted or multi-line flow collections; a trailing `|+` at
  the end of frontmatter; nested sequences; unindented sequences; empty values.
- **N8 amendments:**
  - locate one top-level property at a time. Locating over the whole frontmatter
    failed on 68 of 2,785 repository files.
  - support unindented sequences and empty values in Phase 5. `serde_yaml_ng`
    emits unindented sequences.
  - encode the decoded text.
  - quoted flow items are allowed.
  - a replaced block scalar loses its header comment.
  - keys are never encoded.
  These are recorded in the plan.

### Inventory I1 (`inventory.md` § I1)

- Migration table (7 rows):
  - live rewrite needed only in
    `darkmatter/benchmarks/fixtures/compose_interpolation_heavy.md:61`;
  - the frozen `nested_span_regression/commit.md` keeps its bytes but gets a new
    expected output;
  - `prompts/_add/add-expressions.md` is broken today by the rescan (transcluded
    children re-evaluate inherited `{{{ }}}` state) and is fixed without an
    edit;
  - the other four were checked and are unchanged.
- Rust tests table: 22 rows. 16 need new expectations, 3 depend on the
  nested-span-reach decision, and 3 are unchanged guards.
  - Nothing tests `MAX_INTERPOLATION_DEPTH` directly. The constant, its warning,
    and `ExpressionOrigin::Generated` can be removed together.
  - Four Claudine surviving-span tests flip to "literal is sent".
- **Third rescan path not named in the plan:** transcluded children
  re-evaluate state inherited from the parent (`::file … key={{v}}` too).
  Assigned to Phase 2's body/transclusion task.
- Confirms S1: frontmatter pass 2 rescans data only when a `$( … )` key is
  present.
- N10 refinement: `render_message` / `resolve_typed_value` must keep the single
  scan of authored mixed text. They only drop the rescan of returned data.
- Unrelated breakage, not fixed here:
  - `add-context-variables.md:62` transcludes the missing
    `darkmatter/docs/topics/context-variables.md`;
  - three docs pages write bare `{{ … }}` in their prose and cannot be
    transcluded.

### Inventory I2 (merged into `inventory.md` § I2)

- The sub-agent wrote `inventory-i2.md`; I merged it into `inventory.md` and
  removed the separate file, as the plan requires.
- Table A: all 6 spec rows confirmed in code, with line-level hops.
- Table B: 16 new re-entry points, B1–B16.
- `with_set_overrides`: only `claudine` and `darkmatter` call it.
  `claudine-contract` and Reaper do not use Darkmatter compose.
- Table C: about 50 readers; 11 need the decoded API, about 7 uncertain.

### Plan amendments (plan § "Phase 1 amendments")

- Amended N2, N4 (body clause), N5, N6, N8, and N10.
- Added N14 (a missing link inside data text warns), N15 (nested-span check
  keeps today's reach), and N16 (scope ruling for B1–B16).
- Added owning tasks:
  - Phase 2: B1/B4/B5, the benchmark fixture migration, and removing
    `ExpressionOrigin::Generated`.
  - Phase 4: B3/B15, B8/B9, and a new "Sequence state, params, and variables"
    task for B10/B12, plus B16.
  - Phase 5: the N6 pending checks, `grammar.rs`, and a new "Effect-engine
    frontmatter writes" task for B14.
- Items raised for the author: N9 (encoding gate), B14 (scope), and B2/B11/B13
  (proposed out of scope).

### Darkmatter red tests for I2 B1/B4/B5

Added to `darkmatter/cli/tests/l1/compose_value_provenance.rs`, all
`#[ignore = "red until phase 2"]`. The data comes from
`frontmatter('data.md', …)` through the new helper `fixture_with_data_file`,
never from `--set`.

| Test | Row | Observed today |
| ---- | --- | -------------- |
| `directive_in_interpolated_file_data_is_not_executed` | B1 | exit 0; the inserted `::file ./secret.md` transcludes `TOP-SECRET-CONTENTS` |
| `fence_in_interpolated_file_data_does_not_hide_authored_directive` | B1 | exit 0; a data fence swallows the authored `::file` |
| `transcluded_child_does_not_reevaluate_inherited_escape` | B4 | `Child: fixed x` (wants `Child: fixed {{ area }}`) |
| `transcluded_child_prints_inherited_file_data_verbatim` | B4 | `_Could not transclude child.md_` plus a `…` parse warning |
| `directive_set_value_from_escape_is_not_reevaluated_in_child` | B5 | `_Could not transclude child.md_` |
| `directive_set_value_from_file_data_reaches_child_verbatim` | B5 | exit 1, `interpolation failed … '…'` (fails in the parent body rescan) |

- B5 syntax correction: the child value override is `set.NAME="…"`. A bare
  `key=` is ignored with an `Unknown option` warning, so
  `prompts/_interactive-prompting.md:17` (`actor={{actor}}`) never passes
  `actor` today. This is a separate prompt defect, noted in `inventory.md` B5
  and not fixed here.
- B1 uses `secret.md` because transclusion accepts only `.md` files.

### Claudine red tests for I2 B3/B8/B9/B10/B12/B14/B15

Added to `claudine/cli/tests/l1/agent_text_is_data.rs`. The file now holds 17
ignored red tests.

| Test | Row | Phase | Observed today |
| ---- | --- | ----- | -------------- |
| `lifecycle_shell_from_file_data_runs_approved_bytes` | B3 | 4 | `out.txt` is `""`, wants `{{ err.msg }}`; a probe with `A{{ title }}B` ran `AtB` |
| `loop_seed_from_composed_frontmatter_stays_raw` | B8 | 4 | exit 1 before iteration 1: `total_phases … failed to evaluate '…'` |
| `loop_action_result_is_not_reparsed_as_json` | B9 | 4 | `Flag: [true]`, wants `Flag: [ true` |
| `sequence_state_from_jsonl_item_stays_raw` | B10 | 4 | `Note: [see authored-title]` |
| `task_params_and_group_variables_from_outputs_stay_raw` | B12 | 4 | `Group: [see ]`, `Param: [see reader-title]` |
| `set_frontmatter_argument_from_agent_data_is_not_reresolved` | B15 | 4 | `note: agent said authored-title` |
| `set_frontmatter_persisted_agent_data_survives_next_preparation` | B14+B15 | 5 | exit 1 after iteration 1: `failed to parse '{{ … }}'` (blocked by B15 first; stays red after Phase 4 until B14) |

- B16 (retry/resume fold of `proxy.with:`) has no separate test. The Phase 4
  `proxy.with:` task owns it; the test for that task must exercise a retry.
- Docs/code drift, not fixed here:
  `claudine/docs/topics/flow-control/looping.md` says loop actions can read
  `state.loop.last_output`, but the action lookup (`looping/expression.rs`)
  resolves only `_loop_last_output`. Handed to Phase 6.

### Phase 1 close-out

- **Behavior changes:** none. This phase added tests and planning artifacts
  only.
- **Requirement → test mapping:** every row of the spec's re-entry table and
  every in-scope I2 point (B1, B3, B4, B5, B8, B9, B10, B12, B14, B15) has an
  ignored red test in the two files above. B16 is covered by the Phase 4
  `proxy.with:` task, whose test must exercise a retry. N1 is pinned by three
  passing Darkmatter regression tests.
- **Gates run on macOS:**
  - `cd claudine && just test`: 7480 passed, 26 skipped. The skips are the 17
    new ignored tests plus existing skips.
  - `cd claudine && just lint`: pass.
  - `cd darkmatter && just lint`: pass.
  - `darkmatter-cli` l1: `compose_value_provenance`, `test_layout`, and
    `spawn_site_guard`: 27 passed; the 8 ignored red tests fail as intended.
  - Claudine red tests under `--run-ignored only`: 17 of 17 fail as intended.
- **Skipped:**
  - `just cross-check`. The Claudine tests are `#[cfg(unix)]` under the
    fake-agent convention. The Darkmatter tests that run today spawn `md` with
    `--set` only, which carries no path or shell OS risk. No production code
    changed.
  - The full `darkmatter` `just test`. Only test files under
    `darkmatter/cli/tests/l1/` changed; that binary's layout gate passes.
- **Pre-existing, unrelated:**
  - `md` error output stays ANSI-colored under `NO_COLOR=1`.
  - `prompts/_interactive-prompting.md:17` passes `actor=` in a form that is
    ignored.
  - `add-context-variables.md:62` transcludes a missing doc.
  - `looping.md` names `state.loop.last_output`, which actions cannot read.
- **Claudine skill:** no update in Phase 1, since no behavior or workflow
  changed. Phase 6 owns the documentation updates.

## Phase 2

- Started and finished 2026-09-28. All changes are in `darkmatter` (lib and
  CLI). Claudine changed only in two test files; its production code is
  untouched.

### What was built

- **Origin types** (`compose/value_origin.rs`, new).
  - `OverrideOrigin::{Authored, Data}` and `OverrideLayer` are public.
  - `ComposeOptions::with_data_overrides` and `with_override_layers` are new.
    `with_override_layers` folds ordered layers so each top-level key takes
    the origin of the layer that supplied it. `with_set_overrides` stays
    Authored.
  - `DataPaths` is a leaf-level set of data paths.
    `FrontmatterProvenance` pairs the authored top-level snapshot with the
    data paths. `authored_shell_candidate` is the single shell predicate.
- **Prepare** (`util.rs`): `prepare_frontmatter_for_compose` always returns
  `FrontmatterProvenance`.
  - Inherited external state from a parent is data, down to each leaf the
    deep merge takes from it.
  - An authored `--set` key clears its data mark; a data override key sets
    one.
  - The snapshot holds authored top-level strings only.
  - New `ComposeOptions` fields `data_overrides` and `inherited_origin`
    (`InheritedOrigin`) are classified in both identity encoders.
- **Frontmatter single pass** (`frontmatter_interpolation.rs`).
  - Classification and dependency analysis read an authored view in which
    data leaves are `null`.
  - `rewrite_value` skips data leaves and records every scanned leaf as data.
    Seeds' `{{{ … }}}` are converted once, up front, and marked data.
  - Pass 2 (`FrontmatterPass::Deferred`) scans only the keys pass 1 deferred.
    Deferred keys are returned in the report. `convert_frontmatter_literals`
    is removed.
- **Shell shape from authored source** (`frontmatter_shell_expansion.rs`,
  `preflight/collect.rs`).
  - `scan_frontmatter`, the deferral mark, preflight `scan_one_frontmatter`,
    `detect_dynamic_frontmatter_command_shape`, and `pending_shell_literals`
    all decide on `is_authored_shell_candidate`.
  - Shell output is marked data. The leak guard skips data keys, so it
    checks authored unresolved values only.
- **Body single pass**.
  - `body_origin.rs` gains `DataRanges`, `EditOrigin`, and `BodyProvenance`.
    `DataRanges` offers `after_edits`, `intersects`, `masked`, and a
    fail-closed `ensure_describes`.
  - `interpolate_text_in` scans once over the masked view and converts
    literals in the same scan. It skips spans that touch data and reports a
    data edit per replacement. `MAX_INTERPOLATION_DEPTH`, the rescan loop,
    `reported`/`shift_reported`, and `ExpressionOrigin::Generated` are
    deleted.
  - Every body stage carries `BodyProvenance`: TextReplacement, PageBlocks,
    directive targets, Interpolation, ShellExpansion, ShellBlocks, and
    LinkResolve. The transclusion phase consumes it after the directive
    parse. An inline-pre-only compose hands it back as
    `ComposeReport::body_data`.
- **Directive scanners**.
  - These scanners take `Option<&DataRanges>`: `parse_directives_in` (shell),
    `scan_block_pairs_in`, `parse_page_blocks_in`,
    `scan_darkmatter_directives_in`, transclusion `parse_directives_in`,
    toc-linking, and file-links.
  - Code regions come from `structural_view`. A directive counts only when
    `authored_directive` holds: its preceding line break, prefix, and keyword
    are authored.
  - `data_changed_shape` compares a command with its masked twin. For
    `::shell` and each shell-block command, data may not supply the
    executable, an action, an operator, or a redirection. For a shell block,
    data may not split or join commands either.
- **TextReplacement**: a `replace` value keeps the origin of its leaf. An
  authored value still writes authored text (the macro use that
  `discovers_directives_introduced_by_text_replacement` pins). Inherited and
  one-off values follow `InheritedOrigin`.
- **Transclusion**.
  - `child_inherited_origin` makes the parent state data. It also makes the
    directive's `set`/one-off `replace` values data when the directive's
    options region held data (`BlockOptions::values_origin`, new field).
  - The overlay origin joins the child cache key. Preflight children receive
    the same origin.
- **Reference graph and N14**. `prepare_content` returns the data ranges. A
  reference inside data carries the `darkmatter-origin: data` attribute, and a
  missing target there is a `Warning`. `md compose` prints those warnings
  after the content and exits 0.
- **R5**. `SourceRef::Supplied { supplier }` is a new public variant.
  `attribute_frontmatter_failure` in `pipeline/mod.rs` attributes a failing
  key that a `--set` override supplied to "a command-line override
  (`--set`)". Blocks render "The value came from …, not from the document."
  in place of "Defined in:" plus an excerpt. The variant is generic, so Phase
  5 can pass "the agent".

### Departures and decisions

- **A replaced directive line keeps its terminator authored**
  (`apply_replacements_with_edits`). Without this, the `\n` ending a
  `::shell` line's output was data, and an authored `::file` on the next line
  stopped being a directive (`preflight_lifecycle_carries_graph_into_transclusion`
  caught it).
- **Data masking maps data line breaks to spaces**, not the spike's "keep
  whitespace". This way data cannot start a line in the masked view, which
  makes "data cannot create or hide a directive" hold for code regions too.
- **Data in a shell-block command that changes its line structure is an
  error** ("splits or joins the block's commands"), not a silent join. This
  is simpler and fail-closed.
- **Directive options**: data counts only when it falls in the options
  region after the target. So `::file {{ path }} set.x="y"` keeps `set.x`
  authored.
- **Not changed, noted**: a `::file … when="…"` condition built from inserted
  data is still evaluated as an expression. `when="{{ cond }}"` is an
  established DMLS-documented pattern. A candidate for a follow-up decision.
- **Pre-existing, unrelated**: `md` only printed error-severity reference
  issues. Warnings now print for the data-link case only.

### Requirement → test mapping

| Requirement | Tests |
| ----------- | ----- |
| R1 data payloads `{{ area }}`, `{{…}}`, `{{{ area }}}`, `$(echo X)` exact in frontmatter/body/transclusion, no approval; adjacent authored span evaluates | `lib/tests/l1/data_origin.rs::data_overrides_stay_exact_in_frontmatter_body_and_transclusion` |
| Control row (N1) | `data_origin::authored_overrides_of_the_same_payloads_stay_templates`; CLI `set_value_template_still_fills_in`, `set_value_with_malformed_template_still_fails`, `set_value_shell_command_requires_approval_when_non_interactive` |
| Layer precedence (N3) | `data_origin::override_layers_take_the_origin_of_the_winning_layer` |
| `esc.md` | `data_origin::escaped_frontmatter_value_stays_literal_in_the_body`; CLI `escaped_expression_in_frontmatter_stays_literal_in_body`, `escaped_non_expression_in_frontmatter_stays_literal_in_body` |
| Pass 2 only deferred (N4) | `data_origin::frontmatter_pass_two_scans_only_deferred_authored_keys` |
| File reads are data (E7, E8) | `data_origin::a_file_read_is_not_rescanned_or_converted` |
| R1.2 authored shell via preflight + approval; data not collected | `data_origin::only_an_authored_frontmatter_command_is_collected_and_approved` |
| Produced `$( … )` shape and shell output stay data | `data_origin::produced_command_shapes_are_data` |
| Interpolated executable / operator rejected (E5c) | `data_origin::a_body_executable_from_interpolation_is_rejected`, `a_body_pipeline_operator_from_interpolation_is_rejected`; existing `discovery_rejects_interpolated_frontmatter_executable` |
| Shell block split/close (E5, E5b) | `data_origin::data_cannot_split_or_close_a_shell_block` |
| Directives in data (E1–E4, B1) | `data_origin::directives_inside_data_are_text`; CLI `directive_in_interpolated_file_data_is_not_executed` |
| Masked view (E6, B1) | `data_origin::a_data_fence_does_not_hide_an_authored_directive`; CLI `fence_in_interpolated_file_data_does_not_hide_authored_directive`; `body_origin::data_ranges::the_masked_view_hides_data_structure_and_keeps_offsets` |
| Replacement leaf origin | `data_origin::replacement_values_keep_the_origin_of_their_leaf` |
| B4/B5 child inheritance | `data_origin::a_child_receives_parent_values_as_data`; CLI `transcluded_child_does_not_reevaluate_inherited_escape`, `transcluded_child_prints_inherited_file_data_verbatim`, `directive_set_value_from_escape_is_not_reevaluated_in_child`, `directive_set_value_from_file_data_reaches_child_verbatim` |
| `DataRanges` model, fail-closed, terminator | `body_origin::data_ranges::after_edits_matches_a_per_byte_model` (proptest), `a_map_for_other_text_fails_closed`, `an_empty_range_inside_data_intersects_it`, `a_replaced_line_keeps_its_terminator_authored` |
| Single-pass rewrite unit | `rewrite.rs`: `replacement_output_is_never_parsed`, `replacements_report_one_data_edit_each`, `data_ranges_are_never_scanned` |
| `DataPaths` | `value_origin::tests::*` (5) |
| R5 | CLI `set_value_with_malformed_template_still_fails` (names the override, no "Defined in:"), `authored_malformed_template_still_names_the_document` |
| N14 | CLI `missing_link_inside_inserted_text_warns` (data link exits 0 and warns; authored link exits 2) |
| Strict whole-value / lenient mixed unchanged | Existing `compose_expression_failure_contract.rs` suite, unchanged and passing |

All 8 Darkmatter `red until phase 2` tests are un-ignored and pass.

### Gates

- macOS, `cd darkmatter`:
  - `just test`: 8546 passed, 12 skipped.
  - `just lint`: pass.
  - `just test-l2`: 3 + 18 + 69 passed.
- macOS, `cd claudine`:
  - `just test`: 7481 passed, 25 skipped. One expectation was migrated
    (`wrap_compose_validation`), and one red test turned green and was
    un-ignored.
  - `just lint`: pass.
- Cross-check, native Windows and Linux: the provenance-related selection
  (`data_origin`, `body_origin`, `value_origin`, `rewrite`,
  `frontmatter_interpolation`, `shell_block_integration`,
  `ternary_integration`), 189 of 189 each.
  - `just cross-check` re-parses parenthesized nextest filtersets through
    `bash -c`, so plain name filters were used.
- The remaining 16 Claudine red tests still fail as intended (Phases 4/5).
- Checkpoint 2 migration check: see `inventory.md` § "Phase 2: changed
  expectations".

### Docs touched now (Phase 6 still owns the full write-up)

- `darkmatter/docs/inline/interpolation.md` no longer describes a fixed-point
  rescan. It gains "Inserted Text Is Data".
- `claudine/docs/topics/flow-control/lifecycle.md` (one sentence) and the
  `darkmatter` skill (`SKILL.md`, `compose.md`, `errors.md`): the rescan
  statements were corrected.
- The Claudine skill is unchanged. Claudine's behavior changes arrive in
  Phases 4–5.
