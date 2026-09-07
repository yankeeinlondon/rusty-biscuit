---
$schema: feature-review.yaml
ready: false
agent: claude/default
created: 2026-09-06T15:39:36-07:00
spec: 2026-09-05-inline-flow-and-validations/spec.md
implemented: false
description: A **fix** review of `2026-09-05-inline-flow-and-validations/spec.md`
fix: 2026-09-05-inline-flow-and-validations/review-1.md
previous: /
---

# Review 1: Inline Flow and Completion Validation

## Verdict

**Not ready for production.**

The bulk of this fix is genuinely well built. The Darkmatter phase projection is
passive and recursive, the file-aware inline flow is coherent, the rollback seams
are wired at *every* place the spec named (including the post-`start` launch
failure, which is easy to miss), and the write-grant planner is grounded in the
per-provider permissions research rather than guessed. Targeted suites are green
on this host: `claudine` 579/579 for `test(completion) or test(closure) or
test(write_grant) or test(inline)`, and `darkmatter`/`dmls` 107/107 for
`schema_phase_validation` + `lsp_session`.

Two things block it.

First, a documented behavior does not exist. Both `claudine/docs/topics/composition.md`
and the Claudine skill state that completion problems "render through the same
`SchemaStatusReport` the launch report uses" and that "a property that is present
and valid prints as satisfied". Neither is true: the report is computed, used only
to sort problem names, and thrown away; the failure block is a bespoke bullet list
of failures only.

Second, the plan escalated a design question to Ken — "`eager` without `required`
is no longer expressible as optional" — and then Phase 7 answered it itself, by
editing a shipped prompt to drop `eager` and adding a repo-wide test that forbids
the old spelling. That is exactly the kind of ruling this repo reserves for Ken,
and it silently removed a capability a shipped artifact depended on.

Everything below is ordered by severity.

Every finding has since been ruled on — see [Resolutions](#resolutions). Three
of the rulings depart from the fix this review proposed; those are called out
there.

## Findings

### 1. High — the completion status block the spec (and the docs) promise is not rendered

Spec §D5 requires a failed verdict to print "a per-property status block using the
same renderer as the launch report (`build_schema_status_report`)", and §D5 step 3
adds "A property that is present and valid prints as satisfied, so the author sees
the whole schema, not only the failures." AC4's observable is "a status block
naming `products` as missing".

What is implemented:

- `claudine/lib/src/composition/completion.rs:254` computes a full
  `SchemaStatusReport` via `status_report_for_instance(...)` and stores it on
  `CompletionVerdict::status`.
- `CompletionVerdict::into_error()` (`completion.rs:112`) drops `status` entirely;
  `CompositionError::CompletionSchemaFailed` carries only `problems`.
- `claudine/lib/src/composition/error/render/schema.rs:105-131` then renders a
  hand-rolled `Problems:` bullet list.
- `render_status_report` — the launch-report renderer in
  `claudine/cli/src/commands/schema_interactive/status.rs:30` — has no call site
  on the completion path. Grepping `SchemaStatusReport` across `claudine/cli/src`
  returns only the interactive-schema module.

So `verdict.status` is dead outside `completion/tests.rs`, satisfied properties
never print, and the two ends of a run deliberately do *not* look alike. The
documentation asserting otherwise is drift shipped in the same change
(`claudine/docs/topics/composition.md:276-279` and
`.claude/skills/claudine/composition.md:366-369`).

**Fix:** either carry the `SchemaStatusReport` into `CompletionSchemaFailed` and
render it through the shared renderer, or — if a full table at failure time was
reconsidered — get that ruling recorded and correct both docs. Do not leave the
docs describing the unbuilt version.

### 2. High — an escalated design question was closed by the implementation, and a shipped prompt lost a capability

`plan.md` Phase 6 records an **open finding** in its own words: "There is no longer
a way to say 'optional, but resolve and check it eagerly when supplied' … Ken's
call is needed on which way it goes — relax the prompt to a non-eager `file`
(losing eager reference resolution for a caller-supplied `spec`), or revisit the
`eager`-implies-`required` ruling."

Phase 7 took the first branch without that call:

- `prompts/_implement/implement-plan.md:6` changed from
  `spec: file(eager; match(**/*spec*.md))` to `spec: file(match(**/*spec*.md))`
  (commit `5bc65559e`), and the drift hash pin was refreshed.
- `claudine/cli/tests/shipped_prompt_contract.rs:215`
  (`shipped_prompts_never_declare_eager_without_required`) now *enforces* that
  workaround across every shipped prompt, with a message instructing authors to
  "drop `eager`" for optional properties.

Two consequences worth surfacing before this is accepted:

1. `spec` in that prompt is a caller-suppliable file input. Dropping `eager`
   removes the eager-file existence check and, per
   `EffectiveSchema::validate_with_positions`' own contract, the
   `normalize_frontmatter` opportunity that rewrites a caller-supplied reference
   to its resolved document-relative path. A caller passing a relative `spec=`
   now behaves differently than before this fix.
2. The lint hard-codes the disputed rule into CI, so the cheap experiment
   ("revisit the ruling") now costs a test change too.

The ratified rule itself is Ken's (spec §D1) and I am not disputing it. What is
not Ken's is the decision about which side of the trade-off the shipped prompt
lands on. **Fix:** put the question back to Ken and gate the lint on the answer.

### 3. High — after Ctrl+C, Claudine tells the user the document was partially filled, then silently restores it

`claudine/cli/src/commands/wrap/inline.rs:71-92` still prints, on an interrupted
inline run:

> User interrupted the agent with CTRL+C; the body of `<path>` has been at least
> partially filled:

…followed by the final response, line by line.

Both halves are now false. `report_inline_agent_status` is called from
`harness_orch/attempt.rs:454`, i.e. *inside* `execute_harness_attempt` and
therefore *before* `classify_attempt_phase` reaches
`rollback_inline_document(...)` (`loop_control.rs:2018`) — so by the time the run
ends the document is byte-identical to the baseline, which is precisely what AC8
asserts. And under §D3 the final response is a summary, not body content, so the
lines printed under that header are not the body at all.

**Failure scenario:** operator interrupts an inline run, reads "the body has been
at least partially filled", opens the file, and finds their original placeholder
body — or worse, trusts the printed text as the partial body and pastes it in.

This is the CLAUDE.md "stale comments/prose past their code" case in
user-facing output. **Fix:** rewrite both branches for the restore-on-interrupt
contract (say the document was restored to its pre-run state; show the summary
labelled as a summary, or not at all).

### 4. Medium — OpenCode now renders two tool lines per tool, and the invariant the old comment protected is gone

§D8's adapter repair was implemented literally
(`claudine/lib/src/stream/providers/opencode.rs:409-421`): the completed
`tool_use` record now emits a synthesized `ToolCall` immediately before its
`ToolResult`. The comment that was deleted in the same hunk said why the
synthesis had been removed: *"The `tool_calls` counter still increments so trailer
metadata matches the rendered line count."*

`EventRenderer` renders both events (`claudine/lib/src/render/event_renderer/mod.rs:174`
and `:181`), so for OpenCode — the one provider whose call and result come from a
single record — every tool now prints a `→` line and a `←` line back-to-back
carrying the same name and input, while `tool_calls` still increments once. The
golden test was flipped to assert the new shape
(`opencode_tool_use_completion_shows_paired_arrows`), so the doubling is now
pinned rather than noticed.

Note that D8's stated goal ("so tool-count and tool-name rollups also see the
call") is already met by D8 change #1 — the accumulator resets on `ToolResult`
too — plus recording the tool name on `ToolResult`. The synthesized call is the
more invasive half.

**Fix:** suppress the render of a synthesized call (a `DisplayPolicy` facet, e.g.
"this provider's call event is synthetic"), or record the name on `ToolResult`
and drop the synthesis. Either way, add an L2 tmux capture of an OpenCode tool
sequence — the current evidence is a single L1 golden-line assertion.

### 5. Medium — `is_required` in the launch classifier ignores `eager`, contradicting the ratified rule

`claudine/lib/src/composition/schema/classify.rs:166` decides "required" purely on
`Constraint::Required`; `Constraint::Eager` is not consulted, even though the
sibling helper `is_eager` sits 70 lines above it and §D1 ratifies "`eager` without
`required` … equivalent to `required; eager` for validation purposes".

Consequences for a property declared `foo: string(eager)`:

- Darkmatter's `SchemaPhase::Launch` projection *does* make it required
  (`darkmatter/lib/src/markdown/schemas/phase.rs:125`).
- Claudine classifies a wrong-typed value for it as `invalid_optional`, which the
  translate path drops-and-retries (`invalid_optional_is_dropped_and_retried`)
  rather than treating as a required-property failure.
- `promote_null_required_to_missing` skips it, so an explicit `null` for an eager
  property does not reach interactive collection.

Finding #2's new lint hides this today, which is exactly why it should be fixed
rather than left latent: the moment the lint is relaxed, the two layers disagree.

**Fix:** `is_required` should return true for `Required | Eager`, matching
`inline_prompt::def_is_required_at_completion` (which already gets this right).

### 6. Medium — the two new typed error renders have no Level-2 verification

Per the tiering rules in the review brief:

| Requirement | Strongest evidence | Level |
|---|---|---|
| Completion-failure status block renders correctly in a real terminal | `error_guards.rs:900-908` corpus entry + `inline_completion_lifecycle` stderr substring assertions | **L1 only** |
| Restored-owned-property warning renders correctly | `wrap_inline_compose` substring assertion | **L1 only** |
| File-aware prompt / summary / rollback / verdict routing | CLI subprocess tests with shell stubs | L1 (appropriate) |
| Interactive eager collection | `level2_schema_prompt_pty.rs` | **L2 (correct)** |
| Proxy closure rewrites only the final target | `level2_lifecycle_control.rs` | **L2 (correct)** |

`claudine/cli/tests/level2_typed_error_render_capture.rs` is the established home
for exactly this — it already captures `level2_schema_failure_renders_status_block_in_tmux`,
`level2_composition_source_lookup_renders_status_block_in_tmux`, and the SGR/OSC8/
`NO_COLOR` variants. The two codes this fix introduces (`composition.completion_schema`,
`composition.body_unchanged`) were not added to it. Since finding #1 will change
what that block emits, adding the capture *after* fixing #1 is the right order.

### 7. Medium — the write-grant posture for ten providers is proven only against itself

`plan_write_grant` (`claudine/cli/src/commands/wrap/write_grant.rs:133`) is a
well-researched table — I verified `--add-dir` (Claude, Codex, Kimi, Antigravity),
`--approval-mode auto_edit` / `--include-directories` (Gemini), `--approval-mode
auto-edit` (Qwen), `GOOSE_MODE=auto`, and OpenCode's `external_directory` against
`claudine/docs/research/agent-permissions/*.md`. Every one is documented.

But `write_grant/tests.rs` is a pure-function table: it asserts that the planner
produces the argv the planner was written to produce. No test — at any tier, on
any host, behind any feature — ever launches a provider with the grant and
confirms the agent can actually write the file. AC19 is worded as "launch-plan
tests prove the minimum writable posture", which the table satisfies literally
while proving nothing about the providers.

**Failure scenario:** a provider renames or removes a flag (Antigravity's `--mode
accept-edits` and Kimi's wire-mode AFK behavior are the least-pinned entries here);
`plan_write_grant` keeps emitting it; every inline run on that provider dies at
spawn with the provider's own usage error, and the whole L1 suite stays green.

**Fix:** add a smoke test behind the existing `real-tests` feature for at least
Claude and Codex (spawn with the planned grant, have the agent write one byte into
a temp document outside the workspace, assert it landed). Cross-OS argv shapes can
stay construction-only.

### 8. Medium — a customized `.claudine/inline-compose.md` breaks every inline run with no warning

`load_or_create_guardrails_with` (`claudine/lib/src/composition/guardrails.rs:112-126`)
migrates the materialized guardrail file only when it is byte-equal to one of the
four historical shipped defaults; anything else is returned unchanged, as §D3
specifies.

This repo's own `.claudine/inline-compose.md` is byte-equal to
`SHIPPED_GUARDRAILS_2026_09_05` (verified), so it migrates cleanly. A user who
*edited* theirs is not so lucky: their file still says "Return the replacement
Markdown body content in your final response / Do not edit the source file
directly", which now directly contradicts the prompt header telling the agent to
write the file. The obedient agent writes nothing, the closure refuses the
unchanged body, and every run fails with "the agent did not update `<path>`" — a
message that points at the agent rather than at the stale guardrail file.

**Fix:** when a customized guardrail template is used, detect the retired
phrasing (or simply the absence of `{document_path}` plus presence of "do not edit
the source file") and emit one warning naming the file and the migration. Cheap,
and it converts a confusing failure into a self-explaining one.

### 9. Low — `evaluate_completion` builds the projected schema and every validator twice per verdict

`EffectiveSchema::validate_for_phase_with_positions`
(`darkmatter/lib/src/markdown/schemas/mod.rs:843-906`) is not cheap: it clones and
re-projects the SimplifiedSchema, re-runs `to_json_schema`, merges baselines,
walks the whole JSON Schema to strip the file format, then constructs a fresh
`Validator` *and* one validator per root-union arm.

`evaluate_completion` calls it twice on the same instance: once directly
(`completion.rs:253`) and once inside `status_report_for_instance`
(`classify.rs:614`). Given finding #1, the second call's product exists only to
produce an ordered list of property *names* for sorting.

The verdict runs once per composition — so once per sequence step and once per
loop iteration (ruling 9). A 30-step sequence pays 60 validator builds where 30
would do.

**Fix:** have `evaluate_completion` validate once and derive both the problems and
the status rows from that single report; or cache the projected `EffectiveSchema`
per (schema, phase) on `LaunchSchema`, since the launch-resolved schema is
immutable for the run by design.

### 10. Low — nested schema-definition errors can anchor on an unrelated top-level property in DMLS

`object_body_from_shape` (`darkmatter/lib/src/markdown/schemas/simplified/convert.rs:503-521`)
aggregates per-property conversion failures, but a failure inside an inline object
carries only the **leaf** property name — the recursion passes the child's own
`prop_name`, not a dotted path.

`schema_prepare_diagnostic_with_origin`
(`darkmatter/dmls/src/diagnostics/frontmatter.rs:160-168`) then anchors on
`entry_by_key_path(["$schema", property])`. For a nested leaf this normally misses
and falls back to the whole `$schema` block — acceptable degradation. But when the
leaf name collides with a real top-level property, the diagnostic lands on that
neighbor:

```yaml
$schema:
    prompt: string(required)          # valid — gets the squiggle
    meta:
        prompt: string(bogus-arg(1))  # invalid — reports property "prompt"
```

That is the exact situation AC2 says must not happen ("hover on a valid neighbor
shows type documentation only"). Narrow, but the AC is explicit.

**Fix:** carry the dotted path in `SchemaError::Convert { property }` for nested
shapes (`format!("{context}.{prop_name}")`), and teach `entry_by_key_path` to walk
it.

### 11. Low — stale comment in the accumulator, and four panic paths in phase validation

Two small items in the same family:

- `claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs:49-50` still
  says "`inline-compose` writes this final turn — never the full accumulated
  narration — into the body." Under §D3 inline-compose writes *nothing* from the
  response into the body; this text is the model the fix exists to retire.
- `validate_for_phase_with_positions` uses four `expect(...)` calls
  (`mod.rs:853`, `:855`, `:868`, `:884`) on the assumption that phase projection
  can never break convertibility. That holds for today's projection (it only adds
  `Required` and removes `Generated`), but it turns any future projection bug into
  a CLI panic inside a *validation* path rather than a typed `SchemaError`.
  Returning `Result` and falling back to the unprojected schema would be strictly
  safer.

### 12. Low — AC5's positive half is unasserted

`inline_compose_writes_the_agents_file_and_reports_only_the_final_summary`
(`claudine/cli/tests/wrap_inline_compose.rs:483`) asserts the narration is absent
from stderr and absent from the document, but never asserts that the summary
*appears* on the CLI — which is half of what AC5 states. The summary does reach
the caller (assistant text streams through `Section::FinalStdout`), so this is a
test gap rather than a behavior gap; one positive assertion closes it.

## What is solid

Worth recording so a second pass does not re-litigate it:

- **Rollback coverage is complete and correct.** Every case §D4 names has a call
  site: provider non-zero exit (`loop_control.rs:2102`), interrupt (`:2018`),
  post-`start` launch-construction failure (`:1771` — the easiest one to omit),
  refused body (`:2222`), and closure read/parse/duplicate-key/hash failure
  (`:2231`). Failed rollback keeps the initiating diagnostic and renders the typed
  cause beside it, exactly as specified.
- **The verdict is genuinely shared and genuinely last.** One call site
  (`loop_control.rs:2194`) serves both modes, `success` cannot fire ahead of it,
  and terminal hooks stay unrestricted per ruling 10.
- **Phase projection is passive.** `phase::make_passive` downgrades the one
  filesystem-probing format, projection never mutates the authored schema or the
  instance, and `Launch` coerces while `Completion` validates raw — matching "no
  coercion at completion". I chased the obvious hazard here (a CLI-supplied
  `total_phases=8` arriving as a string and failing raw completion validation) and
  it does not bite: `darkmatter/lib/src/markdown/compose/schema_validation.rs:260-266`
  writes coerced top-level properties back into the effective map.
- **The transient-input rule is right, including the subtle case.**
  `inline_completion_instance` (`completion.rs:276`) deliberately does *not* treat
  an owned property missing from the file as an absence, so a caller-supplied
  `prompt` that satisfied the launch gate still satisfies completion.
- **Aggregate conversion errors and per-property DMLS anchoring work** (modulo
  finding #10), and the guardrail migration correctly refuses to touch a
  customized file (modulo finding #8).

## Verification performed for this review

| Command | Result |
|---|---|
| `just test` in `claudine/` with `BISCUIT_TEST_FILTER="test(completion) or test(closure) or test(write_grant) or test(inline)"` | 579 passed, 6237 skipped, 0 failed |
| `just test` in `darkmatter/` with `BISCUIT_TEST_FILTER="binary(schema_phase_validation) or binary(lsp_session)"` | 107 passed, 0 skipped, 0 failed |
| Byte-comparison of this repo's `.claudine/inline-compose.md` against `SHIPPED_GUARDRAILS_2026_09_05` | equal — migrates cleanly |
| `plan_write_grant` flags cross-checked against `claudine/docs/research/agent-permissions/*.md` | every flag documented |

Not re-run for this review: the full package `just test` gates, `just lint`, and
`just test-l2` (plan Phase 8 records 6,805 / clean / 235-of-236 with one
host-blocked WezTerm red attributable to an Atuin onboarding dialog, which is
consistent with what I saw in the code and not something this fix touches).

## Recommendation

Address findings 1, 2, and 3 before this is considered done — one is a documented
behavior that does not exist, one is a ruling that belongs to Ken, and one
actively misinforms an operator after Ctrl+C. Findings 4–8 are worth folding into
the same pass; 9–12 can ride along or follow.

## Resolutions

Ruled by Ken, 2026-09-06. Each finding was presented with alternatives; the
selected disposition is below. Three depart from the fix this review proposed
and are marked **(departs)**.

| # | Severity | Disposition |
|---|---|---|
| 1 | High | Carry the `SchemaStatusReport` into `CompletionSchemaFailed` and render it through the shared launch-report renderer. Docs and spec §D5 stand as written. |
| 2 | High | **(departs)** Separate the axes rather than relaxing the prompt: `eager` = *when* a property is validated, `required` = *whether* it must be present. Restore `spec: file(eager; match(**/*spec*.md))` in `prompts/_implement/implement-plan.md`, refresh the drift pin, and delete `shipped_prompts_never_declare_eager_without_required`. Spec §D1 is rewritten, not annotated. |
| 3 | High | Rewrite both branches of `report_inline_agent_status` in place: state that the document was restored to its pre-run state, and label the final response as the agent's summary. The failed-rollback branch already renders its own typed cause. |
| 4 | Medium | Keep the synthesized `ToolCall` in the normalized stream and suppress its *render* via a `DisplayPolicy` facet ("this provider's call event is synthetic"). Event shape stays uniform across providers; the divergence lives in policy, not in the adapter. |
| 5 | Medium | **(departs)** Consequent to #2, the projection is the layer that moves: `SchemaPhase::Launch` stops forcing `Eager` to `Required`. `is_required` correctly keeps ignoring `Eager`; the classifier instead gains a distinct *validated-at-launch* notion so a **present-but-invalid** eager value fails at launch rather than taking the `invalid_optional` drop-and-retry path. An absent eager property is simply absent and defers to completion. |
| 6 | Medium | Add plain captures for `composition.completion_schema` and `composition.body_unchanged` to `level2_typed_error_render_capture.rs`, **after** #1 changes what the block emits. SGR/OSC8/`NO_COLOR` variants are not duplicated per code — the shared renderer already carries that evidence. |
| 7 | Medium | Add a `real-tests` smoke test for Claude and Codex: spawn under the planned grant, have the agent write one byte into a temp document outside the workspace, assert it landed. Cross-OS argv shapes stay construction-only; more providers are additive later. |
| 8 | Medium | Warn (do not refuse) at launch when a materialized guardrail file carries the retired instructions — detected structurally, e.g. no `{document_path}` plus "do not edit the source file". Name the file and the migration; the run proceeds. Never overwrite a customized file. |
| 9 | Low | `evaluate_completion` validates once and derives both the problem list and the status rows from that single report. No projected-schema cache for now. |
| 10 | Low | Qualify the property name in the `object_body_from_shape` loop (`format!("{context}.{prop_name}")`), which accumulates through the recursion for free. `entry_by_key_path` already accepts a path of any depth, so the only editor-side change is the hardcoded two-segment call site. **(departs)** in mechanism: keep the dotted `String` rather than adding a structured path field, and have the diagnostic try the whole string as a literal key first (frontmatter keys may contain `.`), then the split path, then the existing whole-`$schema` fallback. |
| 11 | Low | Rewrite the `event_sink.rs` comment for the current model rather than deleting it. **(departs)** for the panics: `validate_for_phase_with_positions` returns a typed `SchemaError` instead of falling back to the unprojected schema — a silent substitution would validate against rules nobody chose, which is worse in a validation path than either a crash or a loud failure. |
| 12 | Low | Add the missing positive assertion to `inline_compose_writes_the_agents_file_and_reports_only_the_final_summary` rather than splitting a second subprocess test out of it. |

### Sequencing

- **#2 and #5 are one workstream and the largest.** Together they touch
  `phase.rs` projection, `classify.rs`, the shipped prompt plus its drift pin,
  `shipped_prompt_contract.rs`, and spec §D1. Neither is safe to land alone —
  today the lint is the only thing hiding the two layers' disagreement.
- **#1 precedes #6** (the captures pin output #1 changes) **and #9** (once the
  report is consumed, the single-validation change is a simplification rather
  than a deletion of live code).
- **#3, #4, #7, #8, #10, #11, #12** are independent of the above and of each
  other.
