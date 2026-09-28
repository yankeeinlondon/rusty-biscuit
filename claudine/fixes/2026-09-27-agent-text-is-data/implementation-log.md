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
    - darkmatter-cli
    - claudine-cli
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
