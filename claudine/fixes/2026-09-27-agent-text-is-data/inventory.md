---
created: 2026-09-28
---

# Inventory

## I1: fixed-point reliance

The installed `md` binary produced every "Today" value below. It ran on 2026-09-28 against copies in `/tmp`. Repository files were extracted with `git archive HEAD` into a scratch tree, so relative and `{{ctx.repo_root}}` transclusions resolve. Standalone reproductions used minimal documents. Nothing in the worktree was composed in place or modified.

Today, three mechanisms re-read produced text. Every row below comes back to one of them:

- **Body rescan.** `interpolate_text_located` (`interpolation/rewrite.rs:180`, up to `MAX_INTERPOLATION_DEPTH = 10` passes) rescans the text its own replacements produced. This covers the output of a ternary branch, a frontmatter value inserted by `{{ key }}`, and a `frontmatter(...)` read.
- **Frontmatter pass 2.** This pass (`compose/pipeline/mod.rs:351-375`) runs only when a frontmatter `$( … )` expanded. It re-partitions every key with `contains_interpolation`, so a value whose *result* contains `{{ … }}` is evaluated again. Two examples: shell stdout, and a whole-value ternary whose chosen literal holds braces.
- **Transclusion inheritance.** The parent's effective state reaches each child as `external_state` (`transclusion/engine.rs:1441`), and the child interpolates it again. A parent value that a `{{{ … }}}` literal turned into `{{ … }}` is therefore evaluated in every child that does not override the key.

The following cases involve no rescan, and single-pass leaves them unchanged:

- Body `::shell` output. Shell blocks run after interpolation, so `::shell printf '%s%s' 'z {' '{ c }}'` already prints `z {{ c }}`.
- Transcluded child output. The parent does not rescan it: `_prompt.md` composes byte-identically standalone and under a parent, and `{{ title || '' }}` survives even when the parent defines `title`.
- Sequence `template:` values. They are interpolated once, at normalization.

### Migration table

| File | Construct | Today | New form / new expected output |
| ---- | --------- | ----- | ------------------------------ |
| `prompts/_add/add-expressions.md:2-4` | Frontmatter `description` holds `` `{{{ … }}}` ``. The parent's converted `{{ … }}` is inherited by children that lack a `description` key and is re-interpolated there. | `md compose … requirements='add foo'` exits 0 with warnings `interpolation of '…' failed`. The body loses `::file ../_no_formatting.md` and `::file ../_os.md` (`_Could not transclude …_`). A copy with the literal removed from `description` transcludes both. | No source edit. Inherited parent state is data, so both partials transclude and `description` stays `` …callable from `{{ … }}`… ``. This is the only prompt harmed rather than helped by the rescan. |
| `prompts/_add/add-expressions.md` body (`{{{ … }}}` at lines 44, 185) and transcluded `prompts/_add/_workflow.md:33,130` | Body interpolation literals | They render once as `{{ … }}` and are not rescanned by the parent. | Unchanged. |
| `prompts/_add/add-context-variables.md` (body line 76, `_workflow.md`) | Body interpolation literals only; `description` has no literal. | Same literal rendering. | Unchanged. |
| `prompts/_prompt.md:43,77` | Body interpolation literals in a partial, transcluded with `::file ^prompts/_prompt.md` | Standalone and transcluded output are identical: `{{ expr }}` and `{{ title || '' }}` stay literal. | Unchanged. |
| `darkmatter/benchmarks/fixtures/compose_interpolation_heavy.md:61` | Nested span inside a ternary branch literal: `{{ proj ? 'inside {{proj}} now' : 'none' }}` | `Nested: inside Darkmatter now` | Rewrite as `{{ proj ? 'inside ' + proj + ' now' : 'none' }}`, which keeps `Nested: inside Darkmatter now`. `compose_phase6.rs:34` and the `compose_pipeline` bench read this fixture. The fixture's 15-link `chain_*` and 30-key `wide_*` keys resolve without rescan and need no change. |
| `claudine/cli/tests/fixtures/nested_span_regression/commit.md:21-26,57` | Frozen pre-fix defect: whole-value `resides_in` whose ternary literals hold `{{ctx.dirty_package_areas}}`, `{{length(…)}}`, and `{{as_unordered_list(…)}}`, read from the body as `{{resides_in}}` | `…staged files to commit which are spread across 2:` followed by a list of areas | Do **not** migrate. `nested_span_regression_fixtures_preserve_pre_fix_defects` pins the defect text. The composed body becomes the raw literal `which are spread across {{length(ctx.dirty_package_areas)}}:` plus `{{as_unordered_list(ctx.dirty_package_areas)}}`. The live `prompts/commit.md:21-26` already uses `+` concatenation. |
| `claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md` | Nested spans only inside lifecycle `say:` literals, which are already single-pass and refused before launch | Refused by the nested-span validator | Unchanged. |

Areas searched with no rescan reliance found:

- **`prompts/**` other than the rows above.** Searched with:
  - `rg -F '{{{'`;
  - a per-span nested-literal regex, `\{\{(?!\{)[^}\n]*?(['"])[^'"\n}]*\{\{`;
  - the same regex with `-U` for multi-line whole values.

  The only hits were `plan.md:28-33` (false positives: separate spans on one line) and the `add-*` rows above. All roughly 40 `frontmatter(spec|review|report, …)` reads fetch status, boolean, or list data, and none expects the read text to evaluate. The frontmatter `$( … )` commands are only the examples in `_prompt.md:51-52`, which print counts and `yes`/`no`.
- **`claudine/prompts/create-new-provider.md` and `darkmatter/prompts/update-cli-docs.md`.** No `{{` at all.
- **`claudine/docs/research/**`.** The fleet prompts (`*/_fleet.md`, `_TEMPLATE.md`) contain no `{{{` and no nested literal spans. `{{state.desc}}` comes from `claudine/docs/providers.yaml` `template.desc`, which is interpolated once, per item, at sequence normalization.
- **Markdown anywhere with `{{{` in frontmatter.** `rg -l -F '{{{' -g '*.md'` returns 50 files. Apart from the rows above they are specs, plans, logs, and reviews under `features/`, `fixes/`, and `reviews/`, plus docs pages. None is a composed template that references a literal key from its body.
- **Design docs.** `darkmatter/features/2026-09-16-expression-type-system/declarations-design.md:832` is a nested-span example in prose, not a composed template.

#### Out of scope for I1, noted for I2 and the rulings

- **`as_markdown(value)` composes its argument on purpose.** It is an explicit nested composition, not a rescan: see `nested_composition.rs:261` `self_composition_reaches_the_depth_limit` and `:336`. It will still compose data, for example `as_markdown(_loop_last_output)`, unless I2 rules otherwise.
- **`::file … key={{v}}` directive overlays are re-interpolated by the child.** One example is `prompts/_interactive-prompting.md:17`. This is the same inheritance channel as the `add-expressions.md` row above.
- **Two pre-existing breakages unrelated to rescanning:**
  - `add-context-variables.md:62` transcludes the missing `darkmatter/docs/topics/context-variables.md`, and compose exits 1.
  - `claudine/docs/topics/state-management/context-variables.md:8`, `expression-engine.md`, and `darkmatter/docs/topics/darkmatter-expressions.md` each author a bare `{{ … }}` in prose. They fail to compose standalone, so `add-*.md` loses them as `_Could not transclude …_` today and will after the fix too.

### Rust tests asserting rescan behavior

No test asserts the `MAX_INTERPOLATION_DEPTH` warning (`interpolation depth limit … possible infinite loop`). The constant, that warning, and `ExpressionOrigin::Generated` have no single-pass producer and can be deleted together.

| Test (file:line) | Asserts | New expectation |
| ---------------- | ------- | --------------- |
| `rewrite.rs:480` `located_failure_from_replacement_output_has_no_span` | State `template: "{{ > invalid }}"` inserted by `{{ template }}` is rescanned and fails under Strict with `span: None` | Succeeds; output `x\ny {{ > invalid }}`. Delete the test, or invert it to prove inserted text is not parsed. |
| `rewrite.rs:705` `rescans_replacement_text_for_nested_interpolation` | `{{ pkg ? 'in a package directory: {{pkg}}' : … }}` becomes `in a package directory: darkmatter`, with `replacements == 2` | `in a package directory: {{pkg}}`, `replacements == 1` |
| `rewrite.rs:723` `rescans_false_branch_for_nested_interpolation` | The false branch becomes `missing: none`, with 2 replacements | `missing: {{fallback}}`, 1 replacement |
| `rewrite.rs:907` `rescan_loop_converts_introduced_literal` | State `tmpl: "{{{ y }}}"` inserted and converted to `{{ y }}` | Output `{{{ y }}}` verbatim: literal conversion applies only to authored literals. 1 replacement. |
| `rewrite.rs:1049` `a_generated_failure_is_reported_once_and_distinct_from_an_authored_one` | 2 warnings, one of them `ExpressionOrigin::Generated { pass: 1, span: 0..17 }` | 1 warning, `Authored(10..27)`; output unchanged (`{{ > generated }} {{ > generated }}`). Remove the Generated half. |
| `rewrite.rs:986`, `:1006`, `:1024` (`rescan_identity`) | Failures are not re-reported across passes | Unchanged and still pass. The `reported` range tracking they guard becomes redundant with one pass. |
| `context/report.rs:902` `a_generated_origin_never_aliases_an_authored_one_at_the_same_offsets` | `Generated` and `Authored` at the same offsets stay distinct | Delete along with the `Generated` variant. |
| `lib/tests/l1/ternary_integration.rs:66` `test_ternary_branch_with_nested_interpolation` | Body `in a package directory: darkmatter` | `in a package directory: {{pkg}}`, or rewrite the input with `+` to keep the old output |
| `ternary_integration.rs:78` `test_frontmatter_ternary_with_nested_interpolation` | Whole-value `message` keeps `Package: {{pkg}}` (confirmed with `md compose --fm`); body `{{message}}` rescans it to `Package: darkmatter` | Body `Package: {{pkg}}` |
| `ternary_integration.rs:87` `test_false_branch_with_nested_interpolation` | `missing: none` | `missing: {{fallback}}` |
| `lib/tests/l1/compose_phase6.rs:34` `interpolation_heavy_fixture_composes_expected_output` | `Nested: inside Darkmatter now` "which the rescan pass then resolves" | Unchanged once the fixture row above is migrated; update the comment. |
| `lib/tests/l1/missing_ctx_capture.rs:172` `a_generated_expression_is_not_reported_at_an_authored_line` | `template: "{{{ ctx.os }}}"` plus body `value={{ template }}` fails because `ctx.os` was not captured, with no authored line | Composes; body `value={{ ctx.os }}`. Today `md compose` fails with `runtime context not captured … ctx.os`. |
| `lib/tests/l1/compose_expression_failure_contract.rs:158` `a_failure_in_replacement_output_is_fatal_without_an_authored_span` | `note: "call {{{ f( }}}"` plus `see {{ note }}` is a fatal body parse error on `f(` | Composes; body `see call {{ f( }}` |
| `cli/tests/l1/compose_value_provenance.rs:14`, `:33` (`escaped_expression_…`, `escaped_non_expression_in_frontmatter_stays_literal_in_body`) | Already red (`#[ignore = "red until phase 2"]`) | Turn green: `Body: fixed {{ area }}` and `Body: fixed {{…}}` |
| `dmls/src/diagnostics/nested_span/nested_span_tests.rs:145` `the_same_defect_in_a_body_produces_no_diagnostic` | No nested-span diagnostic in a body "because the body rescans" (`nested_span.rs:113-115`) | The premise is gone: a body nested span now renders raw braces. Rule whether the diagnostic extends to every surface. If it does, the test asserts one diagnostic. |
| `nested_span_tests.rs:171` `mixed_strings_and_non_lifecycle_whole_values_are_not_flagged` | `resides_in` and mixed `say` are not flagged | Same ruling as above |
| `claudine/lib/src/composition/lifecycle/tests/nested_span.rs:267` `synthesized_mixed_and_non_lifecycle_surfaces_are_not_rejected`; `:287` `commit_fixture_nested_span_lives_on_a_non_lifecycle_key` | Non-lifecycle and mixed nested spans are accepted because "its owning pipeline may rescan" | The assertion holds only if the validator's scope stays lifecycle-only; either way the comment is stale. Same ruling. |
| `claudine/cli/tests/l1/wrap_compose_validation.rs:865` `synthesized_mixed_and_ordinary_frontmatter_values_still_launch` | Launches once; body `{{resides_in}}` and `success.info: "a {{ area ? 'in {{area}}' : 'x' }} b"` resolve by rescan | Still launches, but emits `Body in {{area}}` and `a in {{area}} b`. Rewrite the inputs with `+`, or assert the literal output. |
| `claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs:374` `post_dm2_surviving_span_fails_before_dispatch` | `tmpl: "{{x}}"` plus message `{{tmpl}}` gives a SurvivingSpan evaluation error and no dispatch | Dispatches the message `{{x}}` verbatim (R1.4). |
| `event_time_interpolation.rs:410` `surviving_span_in_top_level_field_renders_property_reason_and_specific_hint` | `tmpl: "done in {{ctx.repo_name}}"` gives `SurvivingSpan { span: "{{ctx.repo_name}}" }` and the concatenate or `{{{ … }}}` hint | Emits `done in {{ctx.repo_name}}`. Retarget the test to an *authored* surviving span if the reason and hint remain. |
| `claudine/cli/tests/l1/wrap_compose_validation.rs:888` `surviving_span_at_event_time_names_the_property_and_specific_hint` | `tmpl: "{{{ctx.repo_name}}}"` plus `start.info: "{{ tmpl }}"`: exit 1, "still contains … after every interpolation pass", 0 provider runs | Exit 0, one provider run, and `start.info` prints `{{ctx.repo_name}}`. |
| `claudine/lib/src/composition/lifecycle/executor/tests/proxy_with_evaluation.rs:295` `a_raw_span_stored_in_frontmatter_never_reaches_the_overlay` | `payload` holding `{{ target_side }}` (as a scalar, in an object, in an array) aborts `proxy.with` with `LifecycleProxyWithEvaluationFailed` | The handoff proceeds, and `x` carries the inert literal. This mirrors the red `agent_text_is_data.rs:332`. |

Also relevant:

- **Comment-only updates:**
  - `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/overlay_layering.rs:351-356` cites the surviving-span rejection.
  - `claudine/cli/tests/l1/shipped_prompt_contract.rs:536` names the runtime surviving-span guard as its proof. Its `!rendered.contains("{{")` check still holds.
- **Confirm by running:** `claudine/cli/tests/l1/loop_initialize_state.rs:8` expects the authored `derived: "phase {{phase}}"` to render `phase 4` on iteration 2. This should be an authored scan per iteration, not a rescan.
- **Red tests in `claudine/cli/tests/l1/agent_text_is_data.rs`:** the ignored tests at `:100`, `:107`, `:114`, `:126`, `:156`, `:210`, `:258`, `:281`, `:308`, and `:332` turn green with this change.
- **The `render_message` and `resolve_typed_value` calls (`executor.rs:1302`, `:1862`) also do the one authored scan** of a mixed literal such as `"a {{ x }} b"`, which parses to `Expr::StringLiteral`. Restrict them to authored literals instead of deleting them, or `message_interpolates_frontmatter_in_literal` and `mixed_body_resolves_both_spans_at_event_time` break.
- **Unchanged tests that exercise authored literals, not rescans:**
  - `interpolation_literal_pipeline.rs:8`
  - `frontmatter_interpolation.rs:2328`, `:2338`, and `:2349`
  - `rewrite.rs:630` and `:951`
  - `missing_ctx_capture.rs:386`
  - `request_context_epoch.rs:155`

### Key-to-key chains

Frontmatter key-to-key references resolve in **dependency order, not by rescan**. N2 needs no new topological order.

- `interpolate_frontmatter` extracts each templated key's variable roots once (`extract_frontmatter_key_refs`, `frontmatter_interpolation.rs:1295`). `doc.<k>` counts as the dependency `k`. The function then sweeps keys whose dependency count has reached zero, and inserts each resolved value into the seed map before any dependent evaluates.
- A whole-value key never rescans, so it tells the two mechanisms apart. With `a: "{{ b }}"`, `b: "B={{ c }}"`, `c: "{{ d }}"`, `d: D` declared in that order, `md compose --fm` shows `a: B=D`. Had `a` run before `b`, it would hold the raw `B={{ c }}`.
- `a: "{{ doc.b }}"` resolves the same way (`a: B=C`). Bare `doc` is a snapshot of *seed* values only, `{"c":"C"}`, and adds no dependency.
- Existing coverage:
  - `frontmatter_interpolation.rs:1806` `wide_and_deep_graph_resolves_in_dependency_order`
  - `:1967` `chained_reference_resolved_incrementally`
  - `:2302` `doc_root_dependency_orders_like_bare_name`
  - the benchmark fixture's 15-link chain
- Cycles (`:1837` self-reference, `:1849` mutual) fall through to the fallback pass and resolve against empty values. There is no rescan here either.
- **The one frontmatter rescan is pass 2's re-partition.** With a `$( … )` key present, a whole-value ternary result `x {{ c }}` becomes `t: x C` in frontmatter. Without one it stays `t: x {{ c }}`. Shell stdout `x {{ c }}` is likewise re-evaluated to `sh: x C`, and the dependent key becomes `pre x C`. Under single-pass these values stay `x {{ c }}` and `pre x {{ c }}`. Pass 2 must re-evaluate only the authored keys it deferred, from their authored source, and never re-partition by `contains_interpolation` on resolved values.

#### Docs that describe the rescan (for the docs phase)

- `darkmatter/docs/inline/interpolation.md:218`, `:236-245`, and `:265-271`
- `darkmatter/docs/topics/schemas/parsing/scanning.md:86-89`
- `.claude/skills/darkmatter/compose.md:669-680` and `:870`
- `.claude/skills/darkmatter/SKILL.md:261` and `:294`
- `.claude/skills/darkmatter/errors.md:83`
- `claudine/docs/topics/flow-control/lifecycle.md:231` and `:748`: "re-expanded when its message renders"

## I2: entry points and readers

This inventory is read-only analysis of the worktree at `feat/schema-enhancement`
(HEAD `ff7219ffc`) for `2026-09-27-agent-text-is-data`, Phase 1, task I2.
Line numbers are the ones in that tree. Paths are relative to the repository
root. `lib/` means `claudine/lib/src/`, `cli/` means `claudine/cli/src/`, and
`dm/` means `darkmatter/lib/src/`.

No repro was executed. "Confirmed" means the path was traced in code, not run.

### Shared Darkmatter tail

All Claudine override-borne rows end in the same Darkmatter path:

1. `dm/markdown/compose/context/options.rs:951` `with_set_overrides` stores the object.
2. `dm/markdown/compose/pipeline/mod.rs:173` calls `prepare_frontmatter_for_compose`
   (`dm/markdown/compose/util.rs:216`). At `:234-238` every override key is
   inserted into the **authored** frontmatter map. At `:241-247` the
   pre-interpolation snapshot captures those strings as if they were authored
   source. That snapshot is the shell-provenance input.
3. `pipeline/mod.rs:213` frontmatter interpolation pass 1
   (`interpolate_frontmatter_located`).
4. `pipeline/mod.rs:325` frontmatter shell expansion, then `:356` pass 2.
5. Body interpolation `dm/markdown/compose/interpolation/rewrite.rs:148`
   (`interpolate_text_located`), with the fixed-point loop at `:180`
   (`MAX_INTERPOLATION_DEPTH` `:80`) and `convert_literals` at `:89`.

The Claudine entry into that tail is `lib/composition/prepare.rs:289-291`
(`canonical_compose_options`, fn at `:235`):
`with_set_overrides(with_initialized_outputs(options.set_overrides))`. Direct
mode composes at `prepare.rs:506` and inline mode at `prepare.rs:617` (a
temporary document built at `:597` from the frontmatter with `prompt` as its
body).

### Table A: spec rows confirmed

| Spec row | Capture site | Hops to Darkmatter (file:line) | Confirmed? | Owning phase (4 or 5) |
| --- | --- | --- | --- | --- |
| Loop `_loop_last_output` | Engine captures iteration output into `LoopAmbient.last_output`. | `lib/composition/looping/types.rs:63` `LoopIterationContext::as_set_overrides` → `insert_ambient_overrides` `types.rs:275-278` inserts `_loop_last_output` as a raw `Value::String` → `cli/commands/compose/prep.rs:1003-1007` passes the whole object as the **user-setters** layer of `layered_set_overrides` (`lib/composition/runtime_state.rs:235`, `merge_object_into` `:270`) → `PrepareOptions.set_overrides` → `prepare.rs:289` → shared tail. | Yes | 4 |
| Sequence `outputs` and step overlay | `RuntimeState::append_output` / `append_output_entry` (`runtime_state.rs:178/184`) from `task/mod.rs:560-563` (prompt) and the shell/group tasks. | `layered_set_overrides` `runtime_state.rs:246-249` inserts `outputs` → callers `cli/commands/wrap/sequence/jit.rs:96` (step), `lib/composition/sequence/task/mod.rs:835` (prompt task), `cli/commands/wrap/harness_orch/prompt.rs:172` (harness re-entry), `prep.rs:1004` (loop) → `jit.rs:213` / `jit.rs:331` (`with_set_overrides` for the shell-approval audit) / `prepare.rs:289`. The overlay is `SequenceStepOverlay::as_set_overrides` (`sequence/model.rs:309`), `jit.rs:106`, and `sequence/preflight/mod.rs:358`. Recursion: arrays and objects are inserted whole, and the frontmatter interpolator walks every leaf. | Yes. The overlay also carries foreign-data `state` (Table B, B10). | 4 |
| Inline agent-added or changed frontmatter | The agent edits the file. The closure `reconcile_inline_artifact_with_evidence` (`lib/composition/closure.rs:107`) restores owned keys (`restore_properties_text` `:136`), plans the hash (`:146-153`), and calls `atomic_write` (`:157`). Nothing encodes the values. | Next run: `resolve_composition_source_in_context` `lib/composition/resolve.rs:174` reads (`:220`) and parses (`:231`, `Markdown::try_from`) → `prepare_inline` `prepare.rs:570` → temporary document `:597` → compose `:617` → shared tail (from step 3; the value is authored frontmatter, not an override). Within one invocation the same happens through the re-reads `reload_composition_source` `resolve.rs:367` (per sequence step, `cli/.../wrap/sequence/iterate.rs:283`) and `load_overlaid_document` `harness_orch/prompt.rs:113` (retry/resume and the stabilized reread). | Yes | 5 |
| Lifecycle `set:` | `dispatch_runtime_set` `lib/composition/lifecycle/executor.rs:1476`: each value goes through `resolve_with_value` (`:1809`) → `resolve_typed_value` (`:1861`). Then `RuntimeState::set_batch` (`runtime_state.rs:143`) records the mutation. | `layered_set_overrides` `runtime_state.rs:241-245` copies `runtime.mutations` above the user setters → same callers as the `outputs` row → shared tail. A second rescan happens at capture time: `resolve_typed_value` `:1875` re-resolves a `{{`-containing result through `resolve_string_value` (`:926`). | Yes | 4 |
| Lifecycle `proxy.with:` | `resolve_proxy_with` `executor.rs:1757` → `walk_proxy_with` `:1786` → `resolve_with_value` → `resolve_typed_value` (same rescan at `:1875`, then `reject_surviving_spans_deep` `:1878`/`:2067`). | Two routes. (a) `merge_frontmatter_overlay` `cli/commands/wrap/overlay.rs:17` puts the values **into the authored map**. Call sites: `prep.rs:354`, `harness_orch/prompt.rs:131` (inside `load_overlaid_document`), and `sequence/iterate.rs:686`. They then go through the shared tail from step 3. (b) **Not in the plan:** `harness_prepare_options` `harness_orch/prompt.rs:159-166` also folds `state.overlay` into `caller_values`, which is the **user-setters** layer of `layered_set_overrides` at `:172`. On retry/resume the overlay enters twice, once as authored map and once as an authored override. | Yes (route b is new) | 4 |
| Agent-written file read by an expression (`frontmatter(log, 'message_to_agent')`) | Darkmatter `frontmatter_fn` `dm/markdown/compose/expression/functions/mod.rs:2396` returns the stored string. | This row does not reach Darkmatter compose again. It fails inside Claudine. Top-level fields: `emit_top_level` `executor.rs:978-1010` → `resolve_emit` `:1028` → `resolve_string_value` `:926` → `reject_surviving_spans` `:940`/`:2050` rejects the returned data. Stack actions: `render_message` `:1302` → a `{{` in the result is re-resolved at `:1314-1316` (strict parse error on `{{…}}`, or silent evaluation of `{{ ctx.repo }}`), then rejected at `:940`. The same rescan runs for positional side-effect args (`dispatch_side_effect` `:1411-1413`, see B15). In a document **body**, the same function result is rescanned by Darkmatter's fixed point (`rewrite.rs:180`). That is Phase 2. | Yes | 4 (lifecycle); body is Phase 2 |

#### Every `render_message` caller (all are covered by N10's fix)

`executor.rs:1241` (communication `message`), `:1360` (shell `command`, see
B3), `:1370` (shell `on_error`), `:1704` / `:1746` (`reason` via `eval_opt_string`
`:1882`), `:1711` (proxy `target`), `:1730` / `:1745` (`delay`), and `:1738`
(`resume` message).

### Table B: new re-entry points

Severity terms follow the spec: *crash*, *silent corruption*, *command
execution*. "Plan" says whether plan.md already names the site.

| # | New re-entry point | file:line | How data re-enters | Severity | Proposed owning phase (2/4/5) | Needs red test? |
| --- | --- | --- | --- | --- | --- | --- |
| B1 | Body directives created by interpolation | `dm/markdown/compose/mod.rs:17-25` (Interpolation is stage 5, before `::shell` 6, `::shell-block` 7, `::file`/`::url` 9, `::code` 11); `dm/.../transclusion/engine.rs:668-672` registers URLs "produced by an earlier compose phase". | Inserted text that contains a line starting `::shell`, `::file`, `::url`, or `::code` becomes a directive. An unbalanced code fence in data can also hide the authored directives after it. In a sequence, the step's shell audit (`jit.rs:193-200`, `with_set_overrides` `:331`) offers the injected command for approval, and a whitelist rule can pass it with no prompt. `::file` and `::url` need no approval. | Command execution (approval or whitelist gated); unapproved file read or fetch; silent corruption | 2 (Phase 2 Wave 2 must name directive discovery; the plan lists ShellExpansion but not transclusion or `::code` discovery on Data ranges) | Yes. Loop body `Last: {{ _loop_last_output }}`, iteration 1 prints `ok\n::file /etc/hosts`; iteration 2's prompt contains the hosts file. |
| B2 | Data inside a Claudine `sh -c` command | Runners: `lib/composition/lifecycle/executor.rs:297-311` (`SystemShellRunner`) and `lib/composition/sequence/task/shell.rs:320`, `:903-910` (`sh -c`). Approval is argv-based: `dm/.../shell_expansion/tokenize.rs` (backticks and `$VAR` are literal). | Lifecycle `shell:` and sequence shell tasks interpolate data (for example `frontmatter(log,'msg')` or step `state`) into a string that `sh -c` runs. Approval tokenizes it as argv, so shell metacharacters in the data are not seen. A whitelist such as `git commit *` passes it silently. This is shell injection through **authored** interpolation, not a template rescan. R1.2 ("interpolation may supply its arguments") does not cover it. | Command execution | 4, **or a separate fix; the author must decide scope** | Yes. Whitelist `git commit *`, `shell: git commit -m "{{ frontmatter(log,'msg') }}"`, agent writes ``msg: x `touch /tmp/pwned` ``. |
| B3 | Late rescan of an approved lifecycle shell command | `executor.rs:1360` → `render_message` `:1302-1320` | Preflight stamps the resolved command, but at event time a `{{` in the result is resolved again. Data such as `{{ err.msg }}` or `{{ current.x }}` is evaluated late, which bypasses the preflight late-binding rejection. The executed bytes then differ from the approved bytes. | Command execution; approval mismatch | 4 (N10 fixes it; add the call site to the Phase 4 list) | Yes. A data file value `{{ err.msg }}`, `shell: "notify {{ frontmatter(f,'v') }}"` in `failure:`. |
| B4 | Transclusion child inherits the parent's composed state | `dm/.../transclusion/engine.rs:1448`, `:1467`, `:1637-1648`; `dm/markdown/compose/util.rs:224-231` (deep merge into the child's frontmatter); `util.rs:241-247` (the snapshot includes it) | Each `::file` child receives the parent's already-composed values (shell output, file reads, `_loop_last_output`, `outputs`) as frontmatter defaults. The child interpolates them again and counts them as authored for the whole-value `$( … )` check. A root-only fix misses this. | Crash; silent corruption; command execution (`$( … )` shape) | 2 (the `external_state` channel needs the same Data origin as N3's overrides) | Yes. Frontmatter `area: x`, `note: "fixed {{{ area }}}"`, body `::file ./child.md`, and the child uses `{{ note }}`. |
| B5 | `::file … set.key="{{x}}"` directive options (corrected in Phase 1: the value override syntax is `set.NAME=`; a bare `key=` is ignored with an `Unknown option` warning, so `prompts/_interactive-prompting.md:17`'s `actor={{actor}}` never reaches the child today, a separate prompt defect) | `dm/.../transclusion/engine.rs:1403-1414` (`apply_set_overrides`); used in `prompts/_interactive-prompting.md:17` | The option value is interpolated in the parent body, applied to the child's frontmatter, and interpolated again. | Crash; silent corruption | 2 | Yes. `actor` = `see {{…}}`. |
| B6 | Transcluded agent-written files | `prompts/_reviews/suggestion-review.md:34` (`::file @{{review}}`); `prompts/commit.md:140` (`::file {{lessons_learned}}`) | A file an agent wrote is composed as a full document: `{{ }}`, `::shell`, and frontmatter `$( … )`. Inline mode does not compose the body (`prepare.rs:585-610`), but a transclusion or direct `compose` of the same file does. | Crash; command execution (approval gated) | **Spec decision needed.** Does a transclusion target count as authored or data? Default: authored (current behavior), documented in Phase 6. | Only if the author rules "data" |
| B7 | `as_markdown(…)` expression | `dm/markdown/compose/nested.rs:167-202` | Composes any string as a document by design, so `as_markdown(_loop_last_output)` evaluates data. | Command execution (approval gated) | 2 (name it as an explicit R1 exception, or restrict it to authored arguments) | Yes, once the rule is chosen |
| B8 | Loop seed lifts composed control variables | `lib/composition/looping/seed.rs:130-148` (`lift_seed` copies values from `effective_frontmatter`, `:99`/`:122`) → `types.rs:63` → `prep.rs:1003-1007` | Control variables referenced by `while`/`until`/actions are taken **after** composition, so their values can come from `$( … )` output or `frontmatter(plan, …)`. They are fed back as authored overrides on every iteration, iteration 1 included. `implement-plan.md` does this with `phase`/`total_phases` read from the agent-maintained `plan.md`. | Crash; silent corruption; `$( … )` shape | 4 (seed values that differ from the authored source are Data; CLI setters in the seed stay Authored) | Yes. `total_phases: "{{ frontmatter('plan.md','total_phases') }}"`, where plan.md holds `"{{…}}"`, and `until: phase >= total_phases`. |
| B9 | Loop action results reparsed as JSON | `lib/composition/looping/actions.rs:241` (`serde_json::from_str(&output)`) | After a mixed render, a string that happens to be valid JSON becomes a number, bool, null, array, or object. | Silent corruption (type change) | 4 (N11) | Yes. `value: " {{ _loop_last_output }}"` with output `true`. |
| B10 | Sequence step `state` from foreign data | `lib/composition/sequence/mod.rs:196-240` (expression and shell item sources), `sequence/data.rs`, `sequence/formal.rs:149-180` (`template:`) → `model.rs:309-330` → `jit.rs:96-106`, `preflight/mod.rs:343-360` | Items from data files, JSONL, `{{ }}` arrays, and `$( … )` output become `state`/`previous`/`next`. They are rescanned in the step document and interpolated into shell tasks (B2). The spec's row only mentions `outputs`. | Crash; silent corruption; command execution via B2 | 4 (the overlay's `state` leaves derived from item data are Data) | Yes. `sequence: "$(ls specs)"` with a directory named `{{x}}`, or a JSONL item `{"name":"a","note":"{{…}}"}`. |
| B11 | Foreign-data items can define tasks | `lib/composition/sequence/normalize.rs:186-236` with `model.rs:201-205` | Lenient data items accept executable keys (`shell:`, `prompt:`, `task:`, `group:`, `set:`). A data file or `{{ }}` array can define work. (`$( … )` sources produce strings only, `mod.rs:236-238`.) | Command execution (approval gated) | 4, or its own fix (the author must decide scope) | Yes. A JSON data item `{"name":"x","shell":"echo hi"}`. |
| B12 | Task `params:` and group `variables:` | `lib/composition/sequence/task/mod.rs:808-838` (`evaluate_params`, `layered_overrides`); `task/group.rs:445-455`, `:514-523`; consumed as caller input at `cli/commands/wrap/sequence/task_run.rs:406-411` | Evaluated once (for example `{{ last(outputs) }}`), then layered as the **user-setters** layer and rescanned in the member prompt. This carrier is separate from `outputs` and the overlay. | Crash; silent corruption; `$( … )` shape | 4 | Yes. `params: {prev: "{{ last(outputs) }}"}` where the previous task printed `{{…}}`. |
| B13 | MCP `#tag` lexing of the composed prompt | `cli/commands/wrap/composition/pipeline.rs:567` (`lex_tags(&effective_prompt)`), `:639`; `harness_orch/prompt.rs:30`, `:348-352`; `loop_control/target_launch.rs:351-355` | With `--mcp`/`--use`, `#tags` in interpolated agent text or file content enable catalogued MCP servers and are stripped from the prompt, and this is recomputed every iteration. Not a Darkmatter scan, but the same "data becomes instruction" class. | Capability change; silent corruption of the prompt | 4, **or its own fix (outside R1-R6's wording)** | Yes. The agent summary says "fixed #slack integration"; the next iteration attaches the slack server. |
| B14 | Positional lifecycle frontmatter writes persist data unencoded | `executor.rs:1443-1450` (`set_frontmatter`, `merge_frontmatter`, `append_frontmatter`, `prepend_frontmatter` through the Darkmatter effect engine), mirror `:1590-1640`; sequence `side_effect:` tasks reach the same verbs (`task/mod.rs:702`) | Data (for example `_loop_last_output` or `err.msg`) is written raw into a document's frontmatter on disk, including the active document. The next re-read (loop stabilized reread, retry fresh read, sequence JIT `reload_composition_source`, next run) treats it as authored. The executor's own comment at `:2047` cites this case. This is a second persistence path outside the inline closure, and R3 only covers the closure. | Crash; silent corruption; command execution (`$( … )` shape on the next run) | 5 (encode data-origin values at the effect-engine write, the same token rule as the closure), **the author should confirm** | Yes. `success: [set_frontmatter(doc, 'note', _loop_last_output)]` with output `see {{…}}`; the next iteration fails to prepare. |
| B15 | Positional side-effect args re-resolved | `executor.rs:1411-1413` (`dispatch_side_effect`) | Same rescan as `render_message`, but for effect arguments. It is not named in N10's list (`:1302`/`:926`/`:1862`). | Crash; silent corruption; feeds B14 | 4 (add to N10) | Yes, combined with B14 |
| B16 | `proxy.with:` folded into user setters on harness re-entry | `cli/commands/wrap/harness_orch/prompt.rs:159-166` → `:172` | See Table A, proxy row, route (b). Phase 4's move to a Data layer must also cover this fold, or retry/resume keeps an Authored copy. | Same as the proxy row | 4 | Covered by the proxy red test if it exercises retry |

#### Lower-severity observations (same class, no Darkmatter rescan)

- **Hook `bash` params split after substitution.** `lib/dispatch/runner/bash.rs:50-73`:
  hook templates are single-pass (`dispatch/template.rs:443-465`), but the
  result is split with `shell_words::split`, so quotes in a payload value
  (prompt, `tool_input.*`) can add arguments. `call` args are per-element and
  safe (`runner/mod.rs:260-266`). This is outside this spec.
- **Contract prompt as a bare positional argument.**
  `claudine/contract/src/session.rs:145-163`: some providers have empty
  `prompt_flags` and no `--` separator, so a prompt starting with `--` could
  be read as a flag. No Darkmatter is involved. This is outside this spec.
- **Lifecycle text is rendered as markup.** `lib/composition/lifecycle/mod.rs:375-414`
  (`Status::from_prose` / `Prose::new`) styles data with tags or links. This
  is cosmetic.
- **More origins reach the surviving-span guard** (`executor.rs:2050`): `err.msg`
  (provider stderr, `lib/harness/runtime.rs:142-168`, and Darkmatter errors
  that quote `'{{ expr }}'`), and `ctx.dirty_files` with a `{{` in a filename.
  `failure: message: "{{ err.msg }}"` after an interpolation error fails the
  handler itself. The Phase 4 N10 fix covers this; use it as an extra red test.
- **Origin laundering hazard.** `cli/commands/compose/prep.rs:1551-1566` plus
  `lib/composition/types.rs:531-558`: `from_caller_overrides` marks every key of
  the merged override map as caller input. It is harmless today, but Phase 4
  must not let runtime layers pass through it.
- **Supplied file selections.** `cli/commands/schema_interactive/supplied.rs:66-119`
  inserts a chosen file **path** into the setters as caller input. A filename
  containing `{{` would be rescanned. This is low severity and person-selected.
- **Future `defer` queue.** `harness_orch/loop_control/requeue.rs` is not wired
  in yet. When it is, it must not replay runtime values as setters.

#### Checked and not re-entry points

- **Hook templates.** `lib/dispatch/template.rs:443-465` substitutes in a
  single pass with no Darkmatter involvement. Results go to argv, TTS, reports,
  or messaging.
- **System prompt.** `lib/system_prompt/prepare.rs:120-198` composes files only,
  with no `set_overrides`. The only injected value is `env.AGENT`. An
  agent-edited `system-prompt.md` is still authored.
- **`claudine-contract`.** It has no Darkmatter dependency, and the prompt is
  passed as argv.
- **Reaper.** It has no dependency on `claudine` or `darkmatter`. The other
  Darkmatter users (biscuit-icon, playa, sniff, biscuit-speaks, messenger, gen,
  biscuit-tui) render or parse Markdown only.
- **Retry and resume.** The resume message replaces the prompt verbatim
  (`harness_orch/prompt.rs:355-357`), and `prompt_tail` is appended after
  compose (`:334-337`). Retry reuses the known layers plus B8, B13, and B16.
  `retry` has no `with:`.
- **Loop `initialize`.** `initialize_frontmatter` is lookup state only
  (`seed.rs:103`, `:126`). Its `set:` writes are the known mutation row.
- **Loop predicates.** `looping/expression.rs` resolves values by lookup
  (`:250-290`) and never parses data as an expression.
- **Results, logs, completion.** `completion.rs` is pure. There is no template
  rendering of results. `render_guardrails` is one `replace`
  (`lib/composition/guardrails.rs:158`).
- **Inline body in inline mode.** Only `prompt` is composed
  (`prepare.rs:585-610`). The body becomes a re-entry only through B6.
- **Darkmatter `::shell` execution.** It uses argv, not `sh -c`
  (`dm/.../shell_expansion/executor.rs:199`).

### `with_set_overrides` callers, workspace-wide

Only `darkmatter` and `claudine` reference `set_overrides` or
`with_external_state`. Both `grep -rln` over `*.rs` and every other crate's
`Cargo.toml` were checked.

| Caller | file:line | Values | Origin |
| --- | --- | --- | --- |
| Claudine canonical preparation | `lib/composition/prepare.rs:289` | Everything below that reaches `PrepareOptions.set_overrides`: CLI setters, loop seed (B8), loop ambients and action results, runtime `set:` mutations, `outputs`, the step overlay (`state` from B10), `proxy.with` (B16), and `params`/`variables` (B12) | **Mixed: carries data** |
| Sequence step shell audit | `cli/commands/wrap/sequence/jit.rs:331` (options also built at `:213`) | Step setters from `step_set_overrides` `:89-96` | **Mixed: carries data** |
| Compose shell preflight audit | `cli/commands/compose/prep.rs:730` | CLI setters and interactive schema input (`:1545-1566`) | Person |
| `md compose --set` / shorthand | `darkmatter/cli/src/commands/compose.rs:350` | CLI JSON and `key=value` | Person |
| `md compose --state` (`with_external_state`) | `darkmatter/cli/src/commands/compose.rs:322` | CLI JSON | Person. The same field carries **data** internally for transclusion children (B4, `dm/.../pipeline/mod.rs:421`, `dm/markdown/reference/graph.rs:331`). |
| Direct field writes (not the builder) | `prep.rs:1003-1007`, `prep.rs:1566`, `harness_orch/prompt.rs:172`, `jit.rs:213`, `lib/composition/types.rs:576` | As in the first row | Mixed: carries data (except `:1566`, person) |
| Override clears | `dm/markdown/compose/nested.rs:202`, `dm/.../preflight/collect.rs:628` | None | n/a |
| Tests | Darkmatter: `schema_validation.rs:2523-2988`, `tests/frontmatter.rs`, `tests/rendering.rs`, `tests/schema.rs`, `preflight/mod.rs:429-849`, `preflight/collect.rs:1542/1573/1631/2019`, `cache/hashing.rs:499`, `lib/tests/l1/{unknown_identifier_warning,feature_review_incident,frontmatter_surface_projection,nested_composition,set_overlay_integration}.rs`. Claudine: `composition/{preflight,prepare,prepare/bootstrap,prepare/service,schema,coordinator}/tests.rs`, `looping/engine/tests/seed_state.rs`, `cli/.../harness_orch/{prompt/tests.rs,loop_control/tests/*}` | Literal fixtures | Person-analog. `collect.rs:2019` (`{"cmd":"$(echo after)"}` is still a command) must keep passing under ruling N1. |

No crate outside `darkmatter` and `claudine` passes data through
`with_set_overrides`. This confirms the plan's claim. Within Claudine, both
production builder callers (`prepare.rs:289`, `jit.rs:331`) carry data and need
N3's origin-tagged layers. So does the transclusion-internal `external_state`
channel in Darkmatter (B4).

### Table C: inline-frontmatter readers

**Design note.** Per N6, Darkmatter decodes tokens during frontmatter pass 1,
so the **loaders must keep tokens raw**. Decoding at load and then composing
would turn the decoded text back into template syntax. Consumers that read
`source.markdown.frontmatter()` **outside** compose call
`decode_literal_tokens`. `original_text` (`resolve.rs:220`) must stay raw
because it is the closure and rollback baseline.

| Reader | file:line | Reads what | Needs decoded API? | Notes |
| --- | --- | --- | --- | --- |
| `resolve_composition_source_in_context` (loader) | `lib/composition/resolve.rs:174` (read `:220`, parse `:231`) | Whole map, raw | No (keeps tokens) | Compose decodes. Its consumers are the rows below. |
| `load_yaml_document` (loader) | `resolve.rs:269` | YAML sequence root, raw | No (keeps tokens) | A YAML sequence file is not an inline document, so tokens are unlikely there. |
| `reload_composition_source` (loader) | `resolve.rs:367` (`:371`, `:378`) | Whole map, raw | No (keeps tokens) | Per-step live re-read. |
| `load_overlaid_document` (loader) | `cli/commands/wrap/harness_orch/prompt.rs:113` (`:121`, `:127`) | Raw map plus `with:` overlay | No (keeps tokens) | Bypasses `resolve.rs`. |
| `resolve_sequence_source` YAML branch | `cli/commands/sequence.rs:150-152` | Raw YAML map | No | As `load_yaml_document`. |
| `resolve_launch_schema` | `lib/composition/prepare.rs:316` (`:321`) | `$schema` presence | No | Author-owned. |
| `inline_prompt_text` | `prepare.rs:345` (`:354`) | `prompt` | No | Closure-owned, never encoded. |
| `scan_removed_validation_keys` callers | `prepare.rs:472`, `:575-577`, `:846` | Key names only | No | |
| `prepare_inline` `original_hash` | `prepare.rs:654` | Hash of the raw source | No (uncertain) | It must match the closure's hash (`closure.rs:146-148`) and `md hash`. Raw on both sides is consistent. |
| `load_effective_schema_in_context` | `lib/composition/schema/mod.rs:419` (`:431`) | `$schema` | No | Author-owned. |
| `pre_validate_schema_for_mode` → `build_effective_instance` | `schema/mod.rs:527` (`:536`) → `:689-695` | Whole map plus setters, validated | **Yes** | Arbitrary values. |
| `drop_invalid_optionals` | `schema/mod.rs:763` (`:775`, `:791`, `:849`); also `cli/commands/sequence.rs:384` | Whole map plus overrides; drops keys | **Yes** | It mutates the source handed to compose. It must decode for **judgment only** and never write decoded values back. |
| `is_composition_independent` / `value_needs_composition` | `schema/mod.rs:904` / `:923-926` | Raw strings: `contains("{{") \|\| contains("$(")` | **Yes (hazard)** | A token matches `{{`, so pre-validation silently defers agent values. It needs a token-aware rule. |
| `pre_validate_schema` | `schema/mod.rs:502` | Wrapper that delegates to `pre_validate_schema_for_mode` | **Yes** (via `:527`) | Decode once in the `:527` path. |
| `build_schema_status_report_for_mode` | `lib/composition/schema/classify.rs:523` (`:533`, `:560`); used by `cli/commands/wrap/sequence/phase1c.rs:309` | Whole map plus overrides → per-property status | **Yes** | |
| `build_missing_properties_error` | `lib/composition/schema/translate.rs:281` (`:287`) | Schema `description` | No (uncertain) | Author prose. |
| `resolve_loop_config` | `lib/composition/looping/config.rs:61` (`:64`) | `loop` subtree | No | Author control. |
| `build_loop_seed*` / `lift_seed` | `lib/composition/looping/seed.rs:40`, `:84`, `:115`, `:130` | `effective_frontmatter` | No (already composed) | B8 is the origin issue, not decoding. `prepare/service.rs:101` / `:174` only dispatch and do no reads. |
| `resolve_sequence_plan_with` | `lib/composition/sequence/mod.rs:102` (`:106-129`) | `sequence`, `fail_fast`, and the whole map for `{{ expr }}` sources and formal templates | **Yes** | An expression source evaluated against the raw map would see tokens. |
| `is_direct_formal_document` | `sequence/formal.rs:45` (`:51`) | `sequence` type | No | |
| `validate_prompt_lifecycle_literals` | `sequence/preflight/mod.rs:116` (`:121`) | Whole map of each prompt document; nested-span check | Uncertain | Lifecycle keys are author-owned. Check that a token in a non-lifecycle key is not scanned. |
| `reject_non_sequence_kind` | `sequence/preflight/mod.rs:155` | `kind` | No | |
| `step_state` (shell approval state) | `sequence/preflight/mod.rs:344` (`:353`) | Whole map plus step overlay → `EffectiveState` for shell interpolation | **Yes** | The approved bytes must equal the executed bytes, so it must see what compose sees (decoded Data). |
| `source_inline_compose` | `sequence/preflight/mod.rs:756` (`:758`) | `prompt` is a string | No | |
| `load_prompt` (graph leaf) | `sequence/preflight/mod.rs:767` (`:778`, `:785`) | `sequence`, `prompt`, `$schema`; stores the full `Markdown` | Key checks: No. Stored markdown: **Yes** | The stored markdown feeds `validate_prompt_lifecycle_literals`. |
| `with_owning_excerpt` / `enrich_frontmatter_text` | `sequence/task/mod.rs:507-512`; `resolve.rs:560`/`:573` (`:589`/`:602`); `lib/composition/error/mod.rs:3151`/`:3158`; `cli/.../loop_control.rs:1565`; `cli/.../staged_boot.rs:367` | Verbatim frontmatter text for error excerpts | No (by design) | The excerpt shows the on-disk token. That is correct because it matches the file; document it in Phase 6. |
| `is_inline_sequence_mismatch` | `lib/composition/mismatch.rs:22` (`:23`) | `prompt` / `sequence` | No | |
| `parse_selection_hints_from_frontmatter` | `lib/composition/hints.rs:26` (`:29-36`); callers `cli/commands/compose/prep.rs:512`, `cli/.../sequence/mod.rs:278` | `agent`, `model`, `interactive` | No (uncertain) | Author-owned unless an agent edits them. If an agent writes `model:` with `{{`, the hint would see the token. |
| `reject_sequence_interactive` | `cli/commands/sequence.rs:73` (`:78`) | `interactive` | No | |
| Inline prompt pre-check | `cli/commands/compose/mod.rs:479` | `prompt` | No | |
| Sequence inline-mode check | `cli/commands/wrap/sequence/mod.rs:198` | `prompt` | No | |
| `authors_initialize` | `cli/commands/wrap/composition/staged_boot.rs:63` (`:66`) | `initialize` presence | No | |
| Proxy overlay merge | `cli/commands/compose/prep.rs:342-356` | Target raw map plus overlay | No (compose decodes) | The overlay values are runtime Data (Phase 4), not tokens. |
| `reconcile_inline_artifact_with_evidence` → `restore_properties_text` | `lib/composition/closure.rs:113`, `:136`; `dm/markdown/hash/write.rs:85` (`parse_text_frontmatter` `:159`, `semantic_frontmatter_delta` `:210`) | Before/after frontmatter → `FrontmatterDelta` | **Yes** (for ownership) | Compare **decoded** values so that an agent rewriting a stored token as its decoded text is not a change. The encoder still works on raw spans. This is Phase 5 N8 input. |
| `inline_completion_instance` / `apply_delta` | `lib/composition/completion.rs:300` (`:308`) | Delta values over the live effective frontmatter → completion schema | **Yes (critical)** | The delta is raw text from Darkmatter's parse. The live effective map is already decoded by compose. |
| `inline_completion_instance` owned values | `completion.rs:319-322` | `prompt`, `hash`, `last_updated` | No | Never encoded. |
| `parse_inline_stored_hash` / `plan_hash_save` | `closure.rs:146`/`:148` (fn `:249`) | `hash`; whole-document hash | No | Hash the raw bytes, consistent with `md hash --diff` (spec acceptance). |
| `raw_body_of` / `replace_body` | `closure.rs:210` / `:224` | Frontmatter span only | No | |
| `extract_markdown_detail` | `lib/composition/file_detail.rs:48` (`:52-57`) | `name`, `description`, `$schema` | Uncertain (**Yes** per plan Phase 5: "file_detail shows decoded text") | Picker display. |
| `extract_yaml_sequence_detail` | `file_detail.rs:88` (`:93`, `:104`) | `name`, `description`, `$schema` | Uncertain (as above) | |
| `schema_lines_from_markdown` | `file_detail.rs:160` (`:161-170`) | `$schema` (`serde_yaml_ng`) | No | |
| Shell-completion validators | `cli/completion/frontmatter.rs:59`, `:78`, `:105`, `:131` | `prompt` / `sequence` presence | No | |
| `declared_property_order` / `load_effective_schema` | `cli/completion/schema_completion/keys.rs:115` (`:118-124`); `schema_completion/mod.rs:48` (`:50-51`) | `$schema` | No | |
| `wrapper_harness_frontmatter_enabled` | `cli/commands/wrap/wrapper_stages.rs:369` (`:370-371`) | Harness key presence in a provider memory file | No | Not an inline document. |
| `materialize_passthrough_harness_seed` | `cli/commands/wrap/overlay.rs:42` (`:48` read, `:78` compose) | Composed frontmatter | No (compose decodes) | Memory-file passthrough. |
| `collect_auditable_commands` (passthrough) | `cli/commands/wrap/harness_orch/loop_control.rs:1666` | Raw text for the `$()` audit | No | Passthrough only. The audit must not treat a token as a candidate (spike S3). |
| `load_validated_frontmatter` | `cli/commands/signals.rs:658` | Research-document frontmatter | No | Not an active document. |
| Composed-output readers | `prepare.rs:699` (`:716-741`, `effective_surface`), `schema/mod.rs:221` (`post_shell_validate`), `prepare.rs:629` (inline prompt header), `cli/.../composition/dry_run.rs:88`, `cli/.../harness_orch/attempt.rs:119` → `runaway_guard.rs:247` | Composed or effective frontmatter | No | Correct once compose decodes. |
| Darkmatter `frontmatter(path[, key])` | `dm/markdown/compose/expression/functions/mod.rs:2396` (loader `load_markdown` `:2347`) | Any file's map or key via `try_from_content` | **Yes (Darkmatter side)** | Decode the returned value as Data (Phase 3). Otherwise loop or sequence conditions that read the active inline document see tokens. |
| Darkmatter `markdown_title` | `functions/mod.rs:2429` | `title` | Uncertain | Same concern if an agent writes `title`. |

### Summary counts

- **Table A:** 6 spec rows. All 6 are confirmed. The proxy row has one hop the
  plan did not list (B16).
- **Table B:** 16 new re-entry points (B1-B16).
  - 6 are Darkmatter or spec-decision items (B1, B4-B7, and B6's authored-or-data ruling).
  - 10 are Claudine runtime or persistence items.
  - 7 lower-severity observations are outside the spec or cosmetic.
  - 10 surfaces were checked and are clean.
- **`with_set_overrides`:** 3 production builder callers plus 5 direct field
  writes. Only `claudine` and `darkmatter` call them, and only the Claudine
  ones carry data.
- **Table C:** about 50 readers. **11 need the decoded API**:
  - `build_effective_instance`
  - `drop_invalid_optionals`
  - `value_needs_composition`
  - the classify status report
  - `resolve_sequence_plan_with`
  - `step_state`
  - `load_prompt`'s stored markdown
  - the closure delta
  - `inline_completion_instance`
  - `frontmatter_fn`
  - `file_detail` (per the plan)

  About 7 more are uncertain.

### Items for the author

1. Decide whether B2 (data in `sh -c` with argv approval), B11, and B13 are in
   scope for this fix or need their own fix.
2. Decide B6: is a transcluded agent-written file authored or data?
3. Confirm B14: should Phase 5's encoding also apply to effect-engine
   frontmatter writes of data-origin values?
4. Decide B7: is `as_markdown` an explicit exception to R1?

