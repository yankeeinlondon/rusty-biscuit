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
    - claudine
    - claudine-cli
    - dmls
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
source_files_during_phase_3:
    - Cargo.lock
    - darkmatter/lib/Cargo.toml
    - darkmatter/lib/src/markdown/literal_token.rs
    - darkmatter/lib/src/markdown/mod.rs
    - darkmatter/lib/src/markdown/frontmatter.rs
    - darkmatter/lib/src/markdown/compose/expression/lexer.rs
    - darkmatter/lib/src/markdown/compose/expression/mod.rs
    - darkmatter/lib/src/markdown/compose/expression/error.rs
    - darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs
    - darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/errors/blocks.rs
    - darkmatter/lib/tests/l1/literal_token.rs
    - darkmatter/lib/tests/l1/main.rs
    - darkmatter/cli/tests/l1/compose_value_provenance.rs
docs_updated_during_phase_3:
    - docs/dependencies.md
    - darkmatter/docs/dependencies.md
    - darkmatter/docs/inline/interpolation.md
    - claudine/fixes/2026-09-27-agent-text-is-data/plan.md
    - claudine/fixes/2026-09-27-agent-text-is-data/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/darkmatter/SKILL.md
source_files_during_phase_4:
    - claudine/lib/src/composition/runtime_state.rs
    - claudine/lib/src/composition/runtime_state/tests.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/tests.rs
    - claudine/lib/src/composition/types.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/interpolation_conformance.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/lifecycle.rs
    - claudine/lib/src/composition/error/tests.rs
    - claudine/lib/src/composition/lifecycle/action_shape.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
    - claudine/lib/src/composition/lifecycle/context.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs
    - claudine/lib/src/composition/lifecycle/executor/tests/proxy_with_evaluation.rs
    - claudine/lib/src/composition/lifecycle/validate.rs
    - claudine/lib/src/composition/looping/actions.rs
    - claudine/lib/src/composition/looping/seed.rs
    - claudine/lib/src/composition/looping/types.rs
    - claudine/lib/src/composition/looping/engine/tests/iteration_actions.rs
    - claudine/lib/src/composition/looping/engine/tests/seed_state.rs
    - claudine/lib/src/composition/sequence/task/mod.rs
    - claudine/lib/src/composition/sequence/task/tests.rs
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/overlay.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs
    - claudine/cli/src/commands/wrap/sequence/iterate.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/src/commands/wrap/sequence/jit/tests.rs
    - claudine/cli/src/commands/wrap/sequence/phase1c.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/override_boundary_guard.rs
    - claudine/cli/tests/l1/main.rs
    - claudine/cli/tests/l1/wrap_compose_validation.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/tests/l1/data_origin.rs
docs_updated_during_phase_4:
    - claudine/docs/topics/flow-control/lifecycle.md
    - claudine/docs/topics/flow-control/flow-control-reference.md
    - claudine/docs/topics/flow-control/looping.md
    - claudine/docs/topics/composition.md
    - claudine/fixes/2026-09-27-agent-text-is-data/plan.md
    - claudine/fixes/2026-09-27-agent-text-is-data/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
source_files_during_phase_5:
    - darkmatter/lib/src/markdown/hash/mod.rs
    - darkmatter/lib/src/markdown/hash/write.rs
    - darkmatter/lib/src/markdown/literal_token.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/schema_validation.rs
    - darkmatter/lib/src/markdown/compose/tests/schema.rs
    - darkmatter/lib/src/markdown/schemas/format.rs
    - darkmatter/lib/src/markdown/schemas/mod.rs
    - darkmatter/lib/src/markdown/schemas/rewrite.rs
    - darkmatter/lib/src/markdown/schemas/simplified/source.rs
    - darkmatter/lib/src/markdown/schemas/simplified/yaml_scalar.rs
    - darkmatter/lib/src/markdown/schemas/tests/mod.rs
    - darkmatter/lib/tests/l1/data_origin.rs
    - darkmatter/dmls/src/diagnostics/frontmatter.rs
    - darkmatter/dmls/src/diagnostics/frontmatter/severity_tests.rs
    - claudine/lib/src/composition/closure.rs
    - claudine/lib/src/composition/closure/persist.rs
    - claudine/lib/src/composition/closure/persist/tests.rs
    - claudine/lib/src/composition/closure/tests.rs
    - claudine/lib/src/composition/error/mod.rs
    - claudine/lib/src/composition/error/render/mod.rs
    - claudine/lib/src/composition/error/render/schema.rs
    - claudine/lib/src/composition/file_detail.rs
    - claudine/lib/src/composition/guardrails.rs
    - claudine/lib/src/composition/lifecycle/executor.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/lib/src/composition/schema/classify.rs
    - claudine/lib/src/composition/schema/mod.rs
    - claudine/lib/src/composition/schema/supplied.rs
    - claudine/lib/src/composition/schema/tests.rs
    - claudine/lib/src/composition/sequence/grammar.rs
    - claudine/lib/src/composition/sequence/mod.rs
    - claudine/lib/src/composition/sequence/tests.rs
    - claudine/lib/src/composition/sequence/preflight/mod.rs
    - claudine/lib/src/composition/sequence/preflight/tests.rs
    - claudine/cli/src/commands/wrap/sequence/jit.rs
    - claudine/cli/tests/l1/agent_text_is_data.rs
    - claudine/cli/tests/l1/error_guards.rs
    - claudine/cli/tests/l1/inline_completion_lifecycle.rs
    - claudine/cli/tests/l1/wrap_inline_compose.rs
docs_updated_during_phase_5:
    - claudine/docs/topics/composition.md
    - claudine/docs/topics/state-management/side-effects.md
    - darkmatter/docs/inline/interpolation.md
    - darkmatter/docs/topics/schemas/definition.md
    - claudine/fixes/2026-09-27-agent-text-is-data/plan.md
    - claudine/fixes/2026-09-27-agent-text-is-data/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/SKILL.md
    - .claude/skills/claudine/timeline.md
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

## Phase 3

- Started and finished 2026-09-28. All changes are in `darkmatter` (the lib,
  plus two new CLI tests). Claudine is unchanged; its L1 suite gives the same
  result as at the end of Phase 2.

### What was built

- **Codec** (`darkmatter/lib/src/markdown/literal_token.rs`, new, public).
  - `encode` (the bare token), `encode_yaml_scalar` (always double-quoted),
    `decode`, and `decode_leaf`, which classifies one string leaf as no token,
    a whole token, or a malformed or embedded token.
  - `TokenError` has these variants: `NotAToken`, `Unterminated`,
    `MissingVersion`, `UnsupportedVersion`, `InvalidPayload`, `InvalidUtf8`,
    and `Embedded`.
  - `decode_literal_tokens(&Value)` returns a `LiteralTokenError` that carries
    the dotted path. `Frontmatter::decoded_literal_tokens()` is the per-key
    convenience (N6).
  - `base64 0.22` is a new direct dependency, using unpadded URL-safe base64
    with canonical decoding. It was already in the lockfile, and both
    dependency docs are updated.
- **Scanner.** `ExpressionFinder::scan` classifies a two-brace opener
  followed by `!data:` as a `LiteralTokenLocation` in a new
  `ExpressionScanResult::tokens` list, and never as an expression. A token
  ends at the first `}}`, or at the end of the text when unclosed. Triple
  braces, backslash escapes, and code regions are checked first, so they
  still win.
- **Error.** The new `ExpressionError::MalformedLiteralToken(TokenError)` is
  authoring-fatal, so a lenient caller cannot keep a token. `interpolation_block`
  gives it a dedicated "malformed literal token" block, with the file,
  line, column, and excerpt.
- **Frontmatter pass 1.** `decode_authored_tokens` runs before literal
  conversion and before classification. Each authored whole-leaf token, at
  any depth, is replaced by its text and marked as a data path. A malformed
  or embedded token fails at its located span. The check runs in a pre-pass
  so that it is fatal in best-effort (preflight) runs too; those runs swallow
  per-key rewrite errors. Excluded (event-time) keys keep their raw text.
- **Body and mixed text.** `interpolate_text_in` rejects any token its
  masked view still shows. Tokens inside data are masked, so they are not
  errors. It and the whole-value path in `interpolate_value_located` also
  reject an expression whose source holds `{{!data:`, such as a token inside
  a string literal (N5, "inside an expression").
- **Pre-approval gate fix** (`pipeline/mod.rs`). `validate_pre_approved` now
  collects from a snapshot of the document taken before frontmatter pass 1.
  Before this fix it re-collected from the document *after* pass 1. It then
  re-prepared the provenance, so a produced whole-value `$( … )` read as
  authored text, and the run failed with "Command 'echo X' … was not
  pre-approved … bug in the pre-flight scanner".
  - This was a latent Phase 2 defect. `cmd: "{{ '$(echo X)' }}"` failed
    under `md compose` the same way before Phase 3. A decoded token exposed
    it.
  - The shell stage never ran the command; only the gate misfired.
  - The snapshot is taken only when the gate applies (the root, with
    `pre_approved_commands`).

### Departures and decisions

- **Unterminated and trailing text.** `{{!data:v1:YQ}} ` (trailing space)
  and two adjacent tokens are `Embedded`, not `Unterminated`: the scanner
  finds a complete token that is not the whole leaf. `{{!data:v1:YQ` with no
  closer is `Unterminated`.
- **The nested-span lint no longer flags a token.** Before this phase,
  `lint_expression` reported `{{!data:…}}` inside a string literal as a
  nested span, because it was an expression. Now it is a scanner token and
  not an expression. The runtime check above reports it as
  `MalformedLiteralToken` instead. No `ExpressionLintKind` variant was
  added, because Claudine and DMLS match on that enum. S3 row 5 suggested
  reporting from the lint; the runtime check covers the same input on every
  compose surface.
- **Excluded keys are not decoded.** Claudine's lifecycle keys stay raw for
  the caller that evaluates them at event time. A token in one therefore
  reaches Darkmatter interpolation later, and there it fails as
  `MalformedLiteralToken`, which is fail-closed. Phase 4/5 must decide
  whether an agent can own a lifecycle key at all.
- **Error rendering.** A token failure in the body says "A document
  expression holds …". That is the existing scope phrase, which was kept.
- **Out of this phase, owned by Phase 5:**
  - the N6 "pending" classifiers (`schemas/format.rs:265`,
    `schemas/mod.rs:1565`, `schema_validation.rs:1253`,
    `schemas/rewrite.rs:374`, DMLS, and Claudine `value_needs_composition`)
  - Claudine `sequence/grammar.rs:122`

  None of them decode yet.

### Requirement → test mapping

| Requirement | Tests |
| ----------- | ----- |
| `decode(encode(s)) == s` for arbitrary Unicode and tricky strings (braces, `$(`, backslashes, CR/LF, quotes, token look-alikes, empty) | `literal_token::tests::decode_inverts_encode`, `decode_inverts_encode_on_tricky_strings` (proptest) |
| Encoded scalar is YAML-safe and loads back to the same string | `literal_token::tests::the_yaml_scalar_round_trips` (proptest), `the_yaml_scalar_loads_back_as_the_bare_token` |
| A token is exactly one scanner token, never an expression | `literal_token::tests::a_token_is_one_scanner_token_and_never_an_expression` (proptest) |
| Canonical payload, empty string, each `TokenError` | `the_empty_string_has_an_empty_payload`, `the_payload_is_unpadded_url_safe_base64`, `malformed_tokens_name_what_is_wrong` |
| Whole-leaf only; escapes are text | `a_leaf_is_a_token_only_when_it_is_the_whole_value`; L1 `literal_token::escaped_spellings_are_text` |
| Public decode API (N6), paths, shape | `decoding_a_tree_replaces_whole_leaves_and_keeps_shape`, `a_malformed_leaf_in_a_tree_reports_its_path`; L1 `loaders_keep_tokens_and_readers_decode_them` |
| Decoded `{{ area }}` / `{{…}}` / `{{{ area }}}` / `$(echo X)` / mixed / token look-alike / empty: not evaluated, exact at top level, nested leaf, dependent key, body, and transcluded child; no approval; file unchanged | L1 `literal_token::a_decoded_token_is_data_everywhere_it_flows` |
| Unchanged token read again on a later run (repeated read) | L1 `loaders_keep_tokens_and_readers_decode_them` (composes twice) |
| Decoded `$(echo X)` is not a shell candidate for collection, approval, or the pre-approval gate; authored `$(echo Y)` control still is; produced `$( … )` passes the gate | L1 `literal_token::a_decoded_command_is_not_a_shell_candidate` (fails if the gate fix is reverted; checked by mutation) |
| Malformed tokens fail with typed cause, key, and 1-based line/column under both `fail_fast` values: bad version, unterminated, embedded, leading whitespace, nested bad payload, body, inside an expression | L1 `literal_token::malformed_tokens_report_their_location` |
| Lenient preflight cannot keep a token | L1 `literal_token::preflight_fails_on_a_body_token` |
| End to end through `md compose` | CLI `compose_value_provenance::literal_tokens_compose_to_their_exact_text`, `malformed_literal_token_fails_with_its_location` |

The input-robustness matrix does not apply to this phase as a config
reader, but the token grammar's shapes are all covered:

- absent: an ordinary string
- empty payload
- a wrong version
- an invalid alphabet or length
- non-canonical trailing bits
- non-UTF-8 bytes
- trailing and leading content
- two tokens
- unterminated

### Gates

- macOS, `cd darkmatter`: `just test` passed 8565 tests with 12 skipped.
  `just lint` passes. `just test-l2` passed 18 + 69 + 3.
- macOS, `cd claudine`: `just test` passed 7481 tests with 25 skipped,
  unchanged from Phase 2. `just lint` passes. The only warning is the
  existing macOS linker note `__eh_frame section too large`.
- `just cross-check` with `literal_token` and `pre_approved` name filters:
  - `darkmatter`: Linux 22/22, native Windows 22/22.
  - `darkmatter-cli`: Linux 2/2, native Windows 2/2.
- No new `#[cfg]`. No pre-existing failures were seen.

## Phase 4

- Started and finished 2026-09-28. Changes are in `claudine` (lib),
  `claudine-cli`, and one function in `darkmatter` (lib). Phase 3 had been
  committed (`fa9c7b953`, `882592f0e`, `c20f0c0e0`) by the time this phase
  started; nothing from Phase 3 was touched.

### What was built

- **One override boundary** (`lib/src/composition/runtime_state.rs`).
  - New public `LayeredOverrides`: the effective top-level override values
    plus the set of keys that are data. `push`/`insert`/`extend` give a key the
    origin of the layer that last supplied it. `apply_to(ComposeOptions)` is
    the only place Claudine calls Darkmatter's `with_override_layers`; it
    splits the keys into one authored and one data layer.
  - `layered_set_overrides(base, runtime, overlay)` now takes a
    `LayeredOverrides` base and returns one. Runtime mutations, `outputs`,
    and the reserved overlay are always data. `with_initialized_outputs` was
    replaced by `LayeredOverrides::initialize_outputs` (an absent `outputs` is
    seeded as data).
  - `PrepareOptions` and `CallerInputLayers` gain `data_override_keys`, with
    `layered_overrides()` / `set_layered_overrides()` accessors (and a
    `with_layered_overrides` builder on `PrepareOptions`). Every existing
    reader of `set_overrides` (schema pre-validation, `prompt` overlay,
    override keys) still sees the effective values unchanged.
  - `canonical_compose_options` layers `proxy_overlay` (data) <
    `layered_overrides()` and applies them through `apply_to`.
  - Guard: `cli/tests/l1/override_boundary_guard.rs` fails on any
    `with_set_overrides` / `with_data_overrides` / `with_override_layers` call
    outside `runtime_state.rs`, and on any `literal_token` reference in the
    runtime-layer modules (runtime values stay raw; no token at runtime).
- **Call sites.**
  - Loop iteration (`cli/compose/prep.rs`):
    `ctx.as_layered_overrides(&caller)` replaces `as_set_overrides()`.
  - Harness re-entry (`cli/wrap/harness_orch/prompt.rs`): re-layers the
    input layers plus the live runtime snapshot, and passes
    `proxy_overlay: state.overlay`. The old fold of `proxy.with:` into the
    user setters (I2 B16) is gone.
  - Sequence JIT (`cli/wrap/sequence/jit.rs`): `step_set_overrides` returns
    `LayeredOverrides`, and `compose_step` / `build_template_preflight_options`
    take it. Pre-validation's dropped optionals keep the other keys'
    origins (`LayeredOverrides::from_parts`). Interactively collected values
    are authored (`iterate.rs`).
  - Task prompts (`sequence/task/mod.rs`): `PromptTaskRequest::set_overrides`
    is a `LayeredOverrides`: evaluated `params` are data beneath authored user
    setters (I2 B12). Group `variables` ride the reserved overlay, so they are
    data too.
- **Loop values (N11, I2 B8, B9).** `LoopIterationContext::as_layered_overrides`
  marks a frontmatter key authored only while it still equals the value the
  caller typed as authored. Control variables lifted from the composed seed,
  action results, and every `_loop_*` ambient are data.
  - `lift_seed` carries only authored caller keys.
  - `render_string_with_lookup` no longer re-parses mixed text as JSON; a
    whole-span leaf keeps its type.
- **`proxy.with:` (I2 B16).** New `PrepareOptions::proxy_overlay`, applied as
  the lowest data layer. `prepare_and_run_active_document` sets it for the
  loop seed, pre-flight, and staged reads. The harness sets it from
  `state.overlay` on every attempt. The eager shell pre-flight builds the
  same layering with `overlay::proxy_caller_overrides`. The overlay is still
  merged into the target's frontmatter map, so direct readers still see it.
- **Lifecycle executor (N10, I2 B3, B15).**
  - New `evaluate_operand`: an authored literal body is interpolated once;
    any other expression's result is data. `render_message`,
    `dispatch_side_effect`, and `resolve_typed_value` (`set:`, `with:`) use it.
  - `reject_surviving_spans(_deep)`, `LifecycleExprError::SurvivingSpan`, and
    `LifecycleEvaluationReason::SurvivingSpan` (with its renderer hint) are
    deleted.
  - `ShellAction::pre_resolved` marks commands stamped at pre-flight (C3). The
    executor runs their bytes as approved (`render_shell_text`).
    `validate_no_err_in_no_error_events` skips stamped text, which is data;
    C3 already refuses an authored late-binding `err`.
  - `reject_control_plane_template`: a `with:` value for a lifecycle key
    (`LIFECYCLE_EVENT_KEYS`, which includes `loop`) that still holds a
    `{{ … }}` span fails with `LifecycleProxyWithEvaluationFailed`.
- **Darkmatter** (`schema_validation.rs::caller_input_records`). Without
  explicit caller records, the fallback now includes data overrides as well
  as `set_overrides`, so a schema-selected file value supplied by the run
  resolves from the caller base exactly as it did when runtime values were
  authored overrides.

### Departures and decisions

- **The surviving-span guard is deleted, not moved to authored text.**
  Darkmatter's strict subtree compose evaluates every span
  `find_all_plain` finds in authored text (`validate_strict_roots` and
  `interpolate_value` both use `ScanMode::Plain`). So an authored-side check
  after evaluation could only ever fire on inserted data, which is exactly
  what N10 removes. A token in authored text is already the fatal
  `MalformedLiteralToken`. The nested-span-in-literal guard and strict
  unknown-root failure are unchanged and tested.
- **The `proxy.with:` overlay stays merged into the target's frontmatter
  map** as well as becoming a data layer. The plan said "move out of
  `merge_frontmatter_overlay`", but loop recognition, `prompt:`, `$schema`,
  provider selection, and the lifecycle parse read that map directly (the
  `a_control_plane_overlay_is_reparsed_by_the_target` test installs a
  `success:` stack this way). The data layer supersedes the merged copy for
  compose, which is what makes the values inert.
- **The overlay lives on `PrepareOptions`, never on `CallerInputLayers`.**
  My first attempt put it into the caller layers and stripped data keys on
  in-place adoption. That broke
  `a_sequence_reserved_overlay_shadows_a_same_named_caller_file_input`: the
  sequence step overlay also rides the caller layers as data and must
  survive an in-place proxy. The overlay is per-document, so it now has its
  own field that the harness re-derives from `state.overlay`.
- **Lifecycle-key overlay values refuse template text (new guard).**
  Without the old guard, run-time data carrying `{{ … }}` could otherwise
  become a target's `success:` or `loop:` configuration, which Claudine
  evaluates outside compose. The check reads resolved values only for those
  keys. That answers Phase 3's open question for `proxy.with:`: an agent's
  data cannot own a lifecycle key. The persistence side, whether an
  agent-written lifecycle key in an inline document is owned, is still
  Phase 5's.
- **`LifecycleEvaluationReason` keeps one variant** (`Expression`). It stays
  a typed hook for the renderer rather than being collapsed in this fix.
- **Out of scope, handed to Phase 5.** The "pending composition" classifiers
  still treat a data value that contains `{{` as pending (Darkmatter
  `schema_validation.rs` `value_pending_composition` /
  `caller_classification_instance`, and the other N6 sites). A trial fix in
  `caller_classification_instance` alone was not enough: a data file
  reference named `{{x}}.md` still failed. So it was reverted, and the row
  was dropped from the new Darkmatter test.
- **Docs drift fixed now** (Phase 6 still owns the full write-up):
  - `lifecycle.md`: the surviving-span error section is replaced by "Template
    text in a value is data, not an error".
  - `flow-control-reference.md`: `with:` evaluation, the lifecycle-key
    exception, and precedence.
  - `looping.md` and `composition.md`: no JSON re-parse; the loop and
    lifecycle renderers now differ in two ways, not three.
  - `interpolation_conformance.rs`: the former "divergence 1" is now an
    agreement test.

### Requirement → test mapping

| Requirement | Tests |
| ----------- | ----- |
| Loop `_loop_last_output` raw in the prompt, no approval, `x` survives (spec row 1) | `agent_text_is_data::loop_last_output_with_template_syntax_stays_raw`, `…_whole_value_shell_stays_raw`, `…_template_and_shell_stays_raw` (un-ignored) |
| Loop predicate sees the raw text | `agent_text_is_data::loop_predicate_reads_the_raw_output` (new; pins the contract, it was not red before) |
| Sequence `outputs` / `last(outputs)` raw, including a parallel group's nested entry (row 2) | `sequence_outputs_with_template_syntax_stay_raw`, `sequence_parallel_group_nested_output_stays_raw` |
| Lifecycle stack message verbatim (row 5) | `lifecycle_stack_message_from_agent_written_file_is_sent_verbatim`; lib `a_frontmatter_value_holding_template_text_is_sent_verbatim`, `a_top_level_field_inserts_template_text_once`; CLI `wrap_compose_validation::a_template_text_value_is_sent_verbatim_at_event_time` (migrated) |
| `set:` inert on the next preparation (row 4) | `lifecycle_set_from_agent_data_stays_inert_on_next_preparation` |
| `proxy.with:` inert in the target: harness route, retry (B16), coordinator/loop route and eager pre-flight | `lifecycle_proxy_with_from_agent_data_stays_inert_in_target`, `lifecycle_proxy_with_from_agent_data_stays_inert_on_a_retry`, `initialize_proxy_with_from_file_data_stays_inert_in_a_looping_target`; lib `a_raw_span_stored_in_frontmatter_reaches_the_overlay_as_data` (migrated) |
| Lifecycle-key overlay refuses run-time template text | lib `run_time_template_text_cannot_become_the_targets_lifecycle_configuration` |
| B3 lifecycle shell runs the approved bytes | `lifecycle_shell_from_file_data_runs_approved_bytes` |
| B8 loop seed | `loop_seed_from_composed_frontmatter_stays_raw` |
| B9 action result not re-parsed | `loop_action_result_is_not_reparsed_as_json`; lib `interpolation_conformance::mixed_string_stays_a_string_in_both_engines` |
| B10 step `state` | `sequence_state_from_jsonl_item_stays_raw` |
| B12 params / group variables | `task_params_and_group_variables_from_outputs_stay_raw`; lib `sequence::task::tests` precedence test (now asserts origins) |
| B15 positional side-effect argument | `set_frontmatter_argument_from_agent_data_is_not_reresolved` |
| Regression: `--set '{"x":"{{ title }}"}'` fills in on every loop iteration | `set_value_template_still_fills_in_on_every_iteration` |
| Regression: nested-span guard, strict unknown root | `nested_span_in_a_lifecycle_literal_is_still_refused`, `an_unknown_root_in_an_authored_lifecycle_span_still_fails`, lib `an_authored_span_with_an_unknown_root_still_fails_before_dispatch`; existing `wrap_compose_validation` nested-span suite unchanged |
| Layer precedence and origin (N3) | lib `runtime_state::tests::layer_precedence_is_setters_then_mutations_then_overlay`, `only_user_setters_are_authored_and_every_runtime_layer_is_data`, `a_later_layer_gives_a_key_its_own_origin`, `from_parts_ignores_data_keys_that_are_no_longer_present`, `apply_to_hands_authored_keys_as_templates_and_data_keys_verbatim`, `initialize_outputs_seeds_only_when_absent`; CLI `jit::tests` precedence tests |
| Single boundary; no token at runtime | `override_boundary_guard::overrides_reach_darkmatter_only_through_the_layered_boundary`, `runtime_layers_never_encode_a_literal_token`, `the_scan_sees_a_builder_call_but_not_prose_about_it` |
| Data override file reference resolves from the caller base (Darkmatter) | darkmatter `data_origin::a_data_override_file_reference_resolves_from_the_caller_base`; CLI `overlay_layering::a_file_valued_overlay_property_resolves_through_the_targets_own_context` |
| Audio silent | every CLI test spawns through `CliProcessFixture` (child-local `PLAYA_DRY_RUN=1` plus a private spool) |

Mutation checks. I reverted each change temporarily and confirmed the named
tests fail, then restored the code:

- dropping the harness `proxy_overlay` fails both harness-route proxy tests;
- dropping the coordinator `PrepareOptions::proxy_overlay` fails the looping
  `initialize` proxy test;
- building the eager pre-flight from authored setters only fails the same
  test, on its whole-value `$(echo INJECTED)` row.

The input-robustness matrix does not apply: no file format or configuration
reader changed.

### Gates

- macOS, `cd claudine`: `just test` passed 7510 tests with 11 skipped. That
  is 14 red tests un-ignored plus the new tests; the 2 remaining ignored
  tests are marked `red until phase 5`. `just lint` passes; the only warning
  is the existing macOS linker note.
- macOS, `cd darkmatter`: `just test` passed 8566 tests with 12 skipped,
  including the new `data_origin` test. `just lint` passes.
- `just cross-check`:
  - `darkmatter --os windows data_origin`: 16/16.
  - `claudine-cli --os windows override_boundary_guard`: pass.
  - `claudine --os linux runtime_state`: pass.
- No new `#[cfg]`. The new CLI rows live in the existing `#[cfg(unix)]`
  `agent_text_is_data` module (the fake-agent convention);
  `override_boundary_guard` runs on every OS.
- No pre-existing failures were seen.
- The remaining B14 red test
  (`set_frontmatter_persisted_agent_data_survives_next_preparation`) now
  exits 0, but iteration 2 renders `Note: [none]`. Loop iterations prepare
  from the in-memory source, not a fresh disk read, so the test cannot
  observe B14 as written. Handed to Phase 5 to redesign (spec
  `message_to_agent`).

## Phase 5

- Started 2026-09-28. Changes are in `darkmatter` (lib), `dmls`, `claudine`
  (lib), and `claudine-cli`.

### Log (in order)

- **Darkmatter locator** (`schemas/simplified/source.rs`, frontmatter mode
  only): an empty value followed by a same-indent `- ` line is an indentless
  sequence; an empty value with nothing deeper is a null scalar with an empty
  span; a sequence-item map aligns with its first key (wide `-   k:` markers).
  The closed v1 schema grammar is unchanged. `DecodedScalar::empty` added.
- **`locate_frontmatter_leaves`** (`hash/write.rs`, public): per-top-level-node
  location as spike S2 recommended, with `FrontmatterPathSegment`, `LeafSpan`,
  `UnlocatedLeaf{path,line,reason}`, `UnlocatedLeafReason`, and
  `LeafLocateError::{Document, Unlocated}`. The guard compares the decoded
  source text with a parse of the trimmed YAML (what compose reads).
  - Departure from S2: a `|+` block that ends the frontmatter is **located**,
    not rejected. With the trimmed-YAML guard both sides agree, and the payload
    is what compose reads. Tested.
- **N6 pending classifiers (Darkmatter).**
  - New `literal_token::holds_pending_syntax(&str)`: `{{`/`$(` present and the
    leaf is not a whole valid token. Shared by `format.rs`
    (`is_pending_expression_value`), `schemas/mod.rs` (`scan_pending_values`),
    and compose `schema_validation.rs`.
  - `schema_validation.rs`: `value_pending_composition` now takes the key and
    the request's `DataPaths` and judges `authored_view` only. `DataPaths` is
    threaded through `prepare_caller_projection`, `run_with_registry`,
    `verify_projection_stability`, `ensure_projection_stable`,
    `caller_classification_instance`, and `build_validation_instance`
    (`provenance.data()` from the pipeline). Data is judged, never deferred.
  - `schemas/rewrite.rs`: removed the lexical `{{`/`$(` skip in
    `rewrite_file_value`; the key-level `composition_pending` set (checked by
    the caller) is the authority, and it is now origin-aware.
  - `validate_with_options` validates the token-decoded instance and computes
    pending on the raw one.
  - The `expression` format validator parses the decoded text of a whole token.
  - DMLS `expression_diagnostics` skips a whole-token value (data, and its
    encoded bytes cannot anchor a parse error).
  - Finding: a data value like `{{x}}.md` is **not a valid file reference**
    (`biscuit_file` reads `{{NAME}}` as a variable). Phase 4's dropped row
    therefore fails for a real reason once data is judged; the new test uses
    `$(x).md`.
- **Claudine `closure/persist.rs`** (new): `repair_agent_frontmatter`,
  `encode_agent_values`, and `AgentFrontmatterRejection` (one error type for
  both, with `line`, `property`, `reason`, `agent_edit`).
  - Repair implements N12 lexically on top-level nodes. Reading of N12 made
    concrete: a value starting with `"`, `'`, `[`, or `{` is quoted only when it
    does not parse (so valid quoted scalars and flow collections are never
    touched); `& * ! % @` and a backtick always quote; `- `, `? `, `: ` at the
    start also quote (not in N12's list, always a YAML error otherwise); a core
    number/bool/null before a ` #` comment is left alone.
  - A duplicate top-level key is located lexically (the YAML parser only names
    the mapping). `serde_yaml_ng::Value` is used for the reparse because a
    JSON map silently keeps the last duplicate.
  - The encoder compares the delta's raw values (same parser on both sides) to
    decide ownership, so an agent that rewrites a stored token as its decoded
    text is re-encoded to the same token, and a raw look-alike is encoded once
    more. The splice is verified by re-parsing.
- **Closure wiring** (`closure.rs`): repair → restore → encode → decoded delta
  → hash → `atomic_write` (N7). `InlineArtifact::frontmatter_delta` now holds
  what composition reads (tokens decoded), so `inline_completion_instance`
  needs no change. New `CompositionError::InlineAgentFrontmatterRejected`
  (code `document.invalid_frontmatter`, origin Provider, detail `doc`,
  `property`, `problems`); any closure error already rolls back
  (`loop_control.rs`).
  - Behavior change: a duplicate owned key (`prompt` twice) is now this
    agent-attributed rejection instead of `InlineArtifactEditFailed`; the
    existing closure test was updated.
- **Readers (N6 amendment, Claudine).**
  - `schema/mod.rs`: `value_needs_composition` uses
    `holds_pending_syntax`; validation judges the token-decoded instance while
    deferral reads the raw one. New `pre_validate_layered_for_mode` takes
    `LayeredOverrides`, so a data override key is judged, never deferred;
    `drop_invalid_optionals_with_origin` and `is_composition_independent(…,
    data_keys)` carry the same rule. The sequence JIT
    (`cli/.../sequence/jit.rs`) uses the layered entry point. The public
    `pre_validate_schema[_for_mode]` and `drop_invalid_optionals` keep their
    signatures and treat every override as authored (their callers pass
    person-typed setters).
  - `classify.rs`: the status report validates decoded text.
  - `sequence/mod.rs` (`resolve_sequence_plan_with`) and
    `sequence/preflight/mod.rs` (`step_state`) read stored tokens as their
    text, so an expression source and the approved shell bytes see what
    composition sees.
  - `sequence/grammar.rs`: a `sequence:` value that begins a token is
    refused as `SequenceInvalid` (plan option "reject"; decoding would let data
    choose the steps).
  - `file_detail.rs`: `name` and `description` show decoded text.
  - One helper for all of these: `closure::stored_text` (and
    `opens_stored_token`). `override_boundary_guard::runtime_layers_never_encode_a_literal_token`
    forbids any `literal_token` reference in `sequence/`, so the readers go
    through the persistence module instead of the guard being narrowed.
  - `load_prompt`'s stored markdown feeds only the lifecycle nested-span check
    on lifecycle keys, which are author-owned; no change.
- **B14 (effect-engine writes).** `lifecycle/executor.rs::dispatch_side_effect`
  passes the value of `set_`/`merge_`/`append_`/`prepend_frontmatter` through
  `closure::persisted_data` (same N9 gate; keys never encoded). The in-memory
  mirror stays raw. The sequence `side_effect:` path dispatches through the same
  function. Every written value is the product of an evaluation, so all of them
  are data; this includes an authored `{{{ x }}}` literal, whose result is
  `{{ x }}` text (R1.3).
- **Guardrails.** `DEFAULT_GUARDRAILS` gains the plain-scalar rule; the
  2026-09-06 text is `SHIPPED_GUARDRAILS_2026_09_06` in
  `HISTORICAL_SHIPPED_GUARDRAILS`, so materialized copies upgrade.
- **Error guards.** The first cut built a rejection `reason` from a typed
  `MarkdownError` (repair's fence error) and a `LiteralTokenError` (decoded
  delta); `error_guards` refused both. Fixed without allowlist entries: a
  near-miss fence is left for `restore_properties_text` to report (typed
  `InlineArtifactEditFailed`), `encode_agent_values` returns
  `EncodeError::{Rejected, Frontmatter(MarkdownError)}`, and the decoded delta
  keeps a malformed token raw (as every other reader does). The corpus gained a
  `document.invalid_frontmatter` entry.
- **B14 red test redesigned** as the spec message asked: run 1 persists, and
  run 2 (a fresh `compose`, reading the file from disk) renders the text. It
  also asserts the on-disk token.

### Departures and decisions

- **N12 made concrete** (see the repair entry above): the indicator rule
  applies to values that do not already parse, so valid quoted scalars and
  flow collections are never touched, as R4 requires.
- **`|+` at the end of frontmatter is supported**, not rejected (S2 said
  reject). The trimmed-YAML guard agrees with the decoder, and the payload is
  what compose reads. A clipped block that ends the frontmatter is likewise
  stored without its final newline, which is what compose reads there.
- **Ownership compares raw parsed values** (the delta's own parser), not
  decoded ones as I2 Table C suggested. Comparing decoded values would leave an
  agent's rewrite of a token into raw template text un-encoded; comparing raw
  values re-encodes it to the same token.
- **Sequence indexes are compared by position.** An agent inserting at the
  front of a list owns every shifted string; an authored `{{ … }}` item shifted
  that way is encoded and stops being a template. Documented here only; no
  shipped document relies on it.
- **The Claudine pre-validators judge data only on the sequence JIT path.**
  The interactive/harness entry points pass person-typed setters, so they keep
  treating overrides as authored. Darkmatter's compose-time validation is
  origin-aware for every path, so a data value that the pre-validator defers is
  still judged at prepare time.
- **An expression-typed field holding decoded data** that contains `{{` is
  still accepted lexically by the Darkmatter `expression` format validator
  (a string-only callback cannot see origin). A raw token is decoded and
  parsed. Edge case; recorded, not fixed.
- **`{{x}}.md` is not a valid file reference** (biscuit-file variables), so
  Phase 4's dropped row cannot pass for any origin; the test uses `$(x).md`.
- **Behavior changes visible to users:** a duplicate owned key and malformed
  YAML written by the agent now report `InlineAgentFrontmatterRejected`
  (agent-attributed, with the line) instead of `InlineArtifactEditFailed`.
  Three existing tests changed expectations (`closure::tests::reports_a_duplicate_owned_key…`,
  CLI `inline_completion_lifecycle::a_duplicate_owned_key_is_refused_and_rolled_back`,
  `wrap_inline_compose::inline_compose_keeps_agent_frontmatter_and_refuses_a_malformed_document`).

### Requirement → test mapping

| Requirement | Tests |
| ----------- | ----- |
| Leaf locator: every S2 row, indentless sequences, empty values, wide markers, CRLF, flow items, anchors/aliases/tags/merges, nested sequences, per-node isolation, line reporting | darkmatter `hash::write::leaf_tests::*` (9) |
| Repair matrix (N12): control row + one edit per shape (colons, ` #`, tab-`#`, trailing `:`, `*`, backtick, `!`, `- `, broken quote, backslash; valid plain/quoted/number/number+comment/float/bool/null/`~`/empty/flow seq/flow map/block scalar; bad nesting) | claudine `closure::persist::tests::repair_walks_every_value_shape_from_one_fixture` |
| Repair: unchanged key untouched; owned property never repaired; duplicate key line + agent; missing `---`; CRLF; block scalar formatting | `persist::tests::an_unchanged_key_is_never_rewritten…`, `a_changed_key_is_repaired_and_an_owned_property_is_not`, `duplicate_keys_name_the_line_and_the_agent`, `a_missing_closing_delimiter_is_refused`, `crlf_line_endings_survive_a_repair`, `a_block_scalar_keeps_its_formatting` |
| Encoder (N8/N9): gate, authored bytes kept, changed-container leaves only, stored token left alone, rewritten token re-encoded, raw look-alike encoded once, CRLF, block scalar replaced, unlocatable leaves refused with line | `persist::tests::only_owned_values_that_could_instruct_become_tokens`, `a_changed_container_owns_only_its_changed_string_leaves`, `a_stored_token_is_left_alone_and_a_raw_look_alike_is_encoded_once`, `encoding_keeps_crlf_and_replaces_a_changed_block_scalar`, `an_unlocatable_owned_value_is_refused_with_its_line` |
| Closure order (N7), decoded delta, coherent hash, repeated read/write/read, rollback of unrepairable edits (incl. CRLF) | `closure::tests::agent_values_are_repaired_encoded_hashed_and_reported_decoded`, `stored_tokens_survive_a_second_run_byte_for_byte`, `an_unrepairable_edit_is_refused_without_writing` |
| Spec L1 inline (end to end): tokens for `summary`/`cmd`, quoted `note`/`title`, completion schema sees decoded (pattern rejects token spelling), `md hash --diff` agreement, run 2 reads exact text, authored `{{ area }}` fills in, tokens keep bytes, no approval | CLI `agent_text_is_data::inline_agent_values_are_stored_as_data_and_read_back_exactly`, `inline_agent_added_frontmatter_survives_a_second_run` (un-ignored) |
| Spec L1 inline unrepairable: duplicate key / bad nesting name the line and agent; rollback restores bytes | CLI `unrepairable_agent_frontmatter_names_the_line_and_rolls_back` |
| CRLF and block scalar formatting kept end to end | CLI `crlf_and_block_scalar_documents_keep_their_formatting` |
| N6 Darkmatter: data never pending; decoded token judged; file ref from data resolves | darkmatter `data_origin::a_data_value_holding_template_text_is_validated_not_deferred` (mutation-checked), `a_data_file_reference_holding_template_text_resolves`; `schemas::tests::validate_with_options_judges_a_literal_token_by_its_text`; `format` tests `pending_expression_values_are_classified_lexically`, `expression_validation_parses_the_text_a_literal_token_holds` |
| N6 DMLS: a token is not parsed as an expression | dmls `a_literal_token_is_not_parsed_as_an_expression` (mutation-checked) |
| N6 Claudine: token judged by text and dropped when invalid; data override judged, authored deferred | `schema::tests::a_stored_literal_token_is_judged_by_its_text`, `a_data_override_is_judged_and_an_authored_one_is_deferred` |
| Sequence readers: expression source reads token text; token refused as a `sequence:` source; approved shell bytes use decoded text | `sequence::tests::an_expression_source_reads_a_stored_token_as_its_text`, `a_stored_token_is_not_a_sequence_source`; `preflight::tests::shell::a_stored_token_resolves_to_its_text_in_approved_bytes` (mutation-checked) |
| `file_detail` shows decoded text | `file_detail::tests::a_stored_literal_token_shows_its_text` |
| B14: effect writes store data as tokens; next run renders exact text | CLI `set_frontmatter_persisted_agent_data_survives_next_preparation` (un-ignored, redesigned, mutation-checked), `set_frontmatter_argument_from_agent_data_is_not_reresolved` (asserts decoded text); lib `persist::tests::persisted_data_encodes_only_gated_string_leaves_and_never_keys` |
| Guardrail plain-scalar rule and migration | `guardrails::tests` (default contents; every historical text migrates) |
| New error code corpus and guards | CLI `error_guards::*` |

Input-robustness matrix (the repair and encoder read agent-written YAML; the
load-bearing field is each agent-added or changed value):

| Shape | Outcome |
| ----- | ------- |
| absent (key removed) | a `Deletion` in the delta; nothing repaired or encoded |
| explicit null (`k:`, `k: null`, `k: ~`) | untouched; null in the tree; never encoded |
| wrong type, whole field (number where a string is expected) | untouched; the completion schema judges it |
| wrong type, one element (`[a, 3]`) | string items judged by the gate; the number untouched |
| wrong type, every element (`[1, 2]`) | untouched |
| empty (`[]`, `{}`, `""`) | untouched; parses as empty |
| duplicate key | refused with the second key's line, agent-attributed; rollback |
| trailing or invalid content (bad nesting, missing `---`) | refused with the line; rollback |

These rows are covered by `repair_walks_every_value_shape_from_one_fixture`
(one fixture, one edit per row, plus a control row), the duplicate and
missing-delimiter tests, and the encoder tests. Code smells grepped: no
`#[serde(default)]`, `filter_map(.. as_str())`, or `unwrap_or_default()` on a
load-bearing parse in `persist.rs`. The decoders' `unwrap_or_else(|_| raw)`
fallbacks are deliberate: a malformed token stays raw so composition reports
it with its location.

### Gates

- macOS, `cd claudine`: `just test` passed 7538 tests, 9 skipped; no
  `#[ignore = "red until` markers remain anywhere in `claudine/` or
  `darkmatter/`. `just lint` passes (only the existing macOS linker note).
  `just test-l2` passed 251 + 3.
- macOS, `cd darkmatter`: `just test` passed 8580 tests (DMLS included), 12
  skipped. `just lint` passes.
- `just cross-check`:
  - `darkmatter --os windows` (leaf_tests, literal_token, data_origin,
    validate_with_options, pending_expression): 57/57.
  - `claudine --os windows` (closure::, composition::schema::, file_detail,
    guardrails, stored-token tests): 153/154. The one failure,
    `composition::schema::tests::shipped_implement_plan_prepares_with_unset_optional_commit_message`,
    is **pre-existing**: it fails identically on an unmodified HEAD worktree
    (`ctx.repo_root` renders `B:/…` while the test expects `Path::display()`'s
    `B:\…`). Not related to this fix; not fixed here.
  - `claudine-cli --os linux` (agent_text_is_data, wrap_inline_compose,
    inline_completion_lifecycle, error_guards, override_boundary_guard): 81/81.
- No new `#[cfg]`. The new CLI rows live in the existing `#[cfg(unix)]`
  `agent_text_is_data` module (fake-agent convention); every library test runs
  on every OS. CRLF is covered at the locator, repair, encoder, closure, and CLI
  levels.
- Housekeeping: the baseline check used a temporary `p5-baseline` worktree,
  since removed; its Windows clone directory
  (`B:\coding\shazam--p5-baseline`) is left on `build-win-native`.
