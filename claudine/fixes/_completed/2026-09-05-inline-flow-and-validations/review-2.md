---
$schema: feature-review.yaml
ready: false
agent: opencode/zai-coding-plan/glm-5.3
created: 2026-09-07T13:01:35-07:00
spec: 2026-09-05-inline-flow-and-validations/spec.md
implemented: true
implemented_by: claude/default
log: claudine/fixes/2026-09-05-inline-flow-and-validations/log.md
next: 2026-09-05-inline-flow-and-validations/review-3.md
description: A **fix** review of `2026-09-05-inline-flow-and-validations/spec.md`
fix: 2026-09-05-inline-flow-and-validations/review-2.md
previous: 2026-09-05-inline-flow-and-validations/review-1.md
---

# Review 2: Inline Flow and Completion Validation

## Verdict

**Not ready for production — by one test.**

All twelve review-1 resolutions are genuinely implemented, and eleven of them
are verified at the appropriate level. The independent-axes ruling (#2/#5) was
carried through completely: the projection, the classifier, the shipped
prompt, the drift pin, the DMLS surfaces, and the spec all agree, with the
Claudine-side header fix currently sitting uncommitted but green. The shared
status renderer (#1) is real — `schema_status_report_prose` lives in the
library, the CLI launch report delegates to it, and the L2 tmux capture
asserts satisfied rows stay visible next to the failures. The write-grant
smoke (#7), the L2 error captures (#6), the single-validation verdict (#9),
and the guardrail warning (#8) all check out against the code and the run.

The one blocker is narrow: the rewritten interrupt messaging from finding #3 —
the operator-safety fix that tells a user their document was restored after
Ctrl+C — has **zero test coverage at any level**. Review-1 flagged the old
text as actively misinforming; the new text is correct today only because I
read it, not because anything would fail if it drifted back. Per the
test-rigor mandate (a user-observable requirement whose strongest test is
absent is a gap at severity ≥ high), that alone keeps `ready: false`. It is a
single L1 assertion on a scenario the suite already stages.

## Review-1 resolution verification

| # | Disposition (2026-09-06 rulings) | Status | Evidence |
|---|---|---|---|
| 1 | Carry `SchemaStatusReport` into `CompletionSchemaFailed`, render via shared renderer | **Done** | `CompletionVerdict::into_error()` carries `status` (`completion.rs:124-135`); the schema-family renderer emits `schema_status_report_prose(status)` (`error/render/schema.rs:115`); the CLI launch report calls the same library function (`schema_interactive/status.rs:36`). L2 capture asserts "was defined correctly" rows render beside failures. |
| 2 | Separate the axes; restore `spec: file(eager; …)`; delete the lint; rewrite §D1 | **Done** | `prompts/_implement/implement-plan.md:6` restored; `shipped_prompts_never_declare_eager_without_required` no longer exists; drift-pin suite green (31/31); spec §D1 states the independent axes; the follow-through lives in `darkmatter/fixes/2026-09-07-required-vs-eager` plus the uncommitted `inline_prompt.rs` change (`def_is_required_at_completion` now `Required`-only, with a full matrix test). |
| 3 | Rewrite both branches of `report_inline_agent_status` | **Implemented, untested** | `wrap/inline.rs:75-97` now states the document "was restored to its pre-run state" and labels the response "Agent summary:". No test asserts either string (see Finding 1). |
| 4 | Keep synthesized `ToolCall`; suppress render via `DisplayPolicy` facet | **Done** | `suppress_synthetic_tool_calls` on the policy (`catalog-types/src/display_policy.rs:22`), honored in `event_renderer/mod.rs:175-177`, generated `true` for OpenCode through `claudine-gen` (`provider/opencode/data.rs:651`); two tests pin the silent arrow. |
| 5 | Launch stops forcing `Eager`→`Required`; classifier gains validated-at-launch | **Done** | `phase.rs:126` — Launch requires only `property_eager && authored_required`; `classify.rs:51-55` makes a present-but-invalid eager value hard at launch; `inline_launch_allows_absent_eager_but_rejects_present_invalid_eager` pins both halves. |
| 6 | Plain L2 captures for the two new codes, after #1 | **Done** | `level2_completion_schema_failure_renders_the_full_plain_status_report` and `level2_unchanged_inline_body_renders_the_plain_typed_error` (`level2_typed_error_render_capture.rs:1211`, `:1230`); both re-run green in tmux for this review. |
| 7 | `real-tests` smoke for Claude and Codex | **Done** | `real_inline_write_grant.rs` — temp document outside the workspace, one-byte body assertion, gated on `CLAUDINE_CONTRACT_REAL=1` + the `real-tests` feature, wired into `just test-real`. |
| 8 | Warn (never refuse) on retired guardrail phrasing | **Done** | `retired_custom_guardrails_path` detects the retired contract structurally (`guardrails.rs:106-113`); `prepare.rs:696-712` pushes a `ComposeWarning` naming the file and migration; run proceeds. |
| 9 | Validate once, derive problems and rows from one report | **Done** | `evaluate_completion` calls `validate_for_phase` once and feeds `&report` into `status_report_from_validation` (`completion.rs:268-283`), which derives rows without re-validating. |
| 10 | Dotted nested property paths; literal-key-first anchoring | **Done** | `convert.rs:505-508` qualifies nested failures as `context.prop`; DMLS tries the whole string as a literal key, then the split path, then the block (`frontmatter.rs:167-177`); `nested_schema_conversion_error_points_to_the_nested_value` pins the range. |
| 11 | Rewrite accumulator comment; typed `SchemaError` for phase validation | **Done** | `event_sink.rs:45-50` states the current model; `validate_for_phase_with_positions` returns `Result<_, SchemaError>` with no `expect` (`schemas/mod.rs:849-908`). |
| 12 | Positive summary assertion in the AC5 test | **Done** | `wrap_inline_compose.rs:562-565` asserts the summary reaches stdout. |

## Findings

### 1. High — the rewritten interrupt messaging has no verification at any level

Requirement: after Ctrl+C on an inline run, the operator is told the document
was restored to its pre-run state and the trailing text is labeled as the
agent's summary (review-1 #3 resolution; the spec's AC8 covers the restore
*behavior*, the resolution covers the *reporting*).

What exists: `report_inline_agent_status` (`claudine/cli/src/commands/wrap/inline.rs:36-98`)
prints the corrected text, but no test in the repository asserts
"restored … to its pre-run state" or "Agent summary" — I grepped every test
tree. The AC8 tests (`inline_completion_lifecycle.rs`, `wrap_inline_compose.rs`)
assert the document is byte-identical; they never look at the messaging. The
only interrupt-text test in the CLI asserts a different string
("interrupt received", `level2_interrupt_feedback_capture.rs`), and the
`error_report.rs` test covers exit-code classification, not this prose.

Why it matters: review-1 #3 was High precisely because this output lied to
operators. The class of defect is "user-facing prose drifting from behavior,"
and prose with no assertion is how that drift recurs silently.

Secondary, same function: the message is printed from
`harness_orch/attempt.rs:454`, which runs *before* `rollback_inline_document`
(`loop_control.rs:2018`) — the past-tense "restored" precedes the restore. On
the normal path the claim is true milliseconds later; if the rollback itself
fails, the operator sees "restored to its pre-run state" followed by a typed
rollback-failure diagnostic — contradictory output in the one scenario where
accuracy matters most.

**Fix:** extend the existing AC8 exit-130 test with stderr assertions for the
restored-message and the summary label (Level 1 is the appropriate tier —
subprocess stderr substring). While there, either move the print after the
rollback seam or word it prospectively ("Claudine will restore …") so a failed
rollback cannot contradict it.

### 2. Low — assertion-free debug test left in the L1 suite

`composition/schema/tests.rs:1361` — `scratch_dump_file_array_problems` contains
only `eprintln!` loops and zero assertions. The name says what it is: scratch
scaffolding from investigating array/file-match problems. It runs in the
default L1 filter set (unlike the `scratch_` tests some areas exclude) and
fails never, proving nothing. Delete it, or convert the three cases into
assertions on `report.problems`.

### 3. Low — AC9 L2 equivalence test fails on any host that exports `MODEL`

`level2_lifecycle_equivalence_ac9_context_facets_match_direct_run`
(`level2_lifecycle_control.rs:5850`) authors
`model: llamacpp/ac9-probe-model` in the probe target but does not scrub the
ambient `MODEL` environment variable, which outranks frontmatter. On this
host — an agent session exporting `MODEL=zai-coding-plan/glm-5.3` — the test
hard-fails (not skips) with the fixture model replaced by the host model. I
verified it passes with `env -u MODEL`. Phase 8 recorded it green because that
run had no `MODEL` in scope.

This is an environment-hygiene defect, not a regression, but this repo's
reviews and CI increasingly run inside agent sessions that export `MODEL`.
**Fix:** pin or unset `MODEL` in the staged environment, the way the suite
already pins `HOME`/`PATH`. The same audit is worth doing for the other
facet-equivalence tests that read `env.MODEL`.

### 4. Low — no collision regression test for the nested-anchoring fix

Review-1 #10's exact scenario — a nested leaf whose *name* collides with a
valid top-level property (`$schema.meta.prompt` invalid beside a valid
top-level `prompt`) — is fixed by the dotted qualification but not pinned: the
nearest test (`nested_schema_conversion_error_points_to_the_nested_value`)
has no same-named top-level sibling. One fixture closes it; the mechanism
(`convert.rs` dotted paths + literal-first, split-path-second lookup in
`frontmatter.rs:167-177`) is implemented and otherwise covered.

### 5. Note — the independent-axes Claudine-side change is uncommitted

`claudine/lib/src/composition/inline_prompt.rs` (completion column of the
inline prompt header: `eager` no longer implies required-at-completion, plus
the four-cell/both-array-placements/union matrix test) and its two
shared-resources doc updates are green in the worktree but not yet committed.
They belong to the resolution of findings #2/#5; land them with this cycle so
the committed tree is not left between two rulings.

## Test-rigor classification (user-observable requirements)

| Requirement | Strongest evidence | Level | Assessment |
|---|---|---|---|
| AC1 — eager accepted on every type; screenshot document clean | `eager_schema_fixture_is_clean_and_catalog_driven` (in-memory LSP session through the real diagnostics pipeline) | L1 in-process | Appropriate — no terminal encoder in scope for editor squiggles |
| AC2 — per-property anchoring; valid neighbor hover isolated | `schema_definition_errors_are_independent_and_property_ranged` (LSP session: two ranged diagnostics + hover isolation) | L1 in-process | Appropriate |
| AC3/AC13 — launch collection, deferred inline gaps | `level2_schema_prompt_pty` + L1 unit matrix | L2 (PTY) + L1 | Appropriate |
| AC4 — completion failure status block | L1 lifecycle suite + `level2_completion_schema_failure_renders_the_full_plain_status_report` (tmux; asserts satisfied rows render) | L2 | Appropriate |
| AC5 — summary on CLI, not in document | L1 subprocess with positive stdout assertion and negative body assertions | L1 | Appropriate — file/stdout routing, no terminal-encoder dependence |
| AC6/AC7/AC8 — restore warnings, unchanged body, rollback | L1 subprocess byte-for-byte document assertions | L1 | Appropriate |
| Interrupt *messaging* (review-1 #3) | none | — | **Gap — Finding 1** |
| AC9/AC9a/AC9b — compose completion, launch refusal, kept artifact | L1 subprocess suite (`inline_completion_lifecycle`, `inline_compose_hash`) | L1 | Appropriate; one test host-fragile (Finding 3) |
| AC11 — accumulator resets on any tool activity | per-adapter replay fixtures incl. the saved voip transcript | L1 | Appropriate — parser contract |
| AC19 — write-grant posture | construction table (3 OS path shapes) + opt-in real-tier smoke for Claude/Codex | L1 + real | Matches resolution #7; more providers additive |
| Typed error renders for `composition.completion_schema` / `composition.body_unchanged` | two tmux captures, plain variant | L2 | Matches resolution #6 (shared renderer carries SGR/OSC8 evidence) |

## What is solid (carried from review-1, re-confirmed)

- One verdict call site serves both modes (`loop_control.rs:2194`; the
  `inline.rs:208` site is a `#[cfg(test)]` helper), and rollback seams fire at
  every case §D4 names.
- The verdict validates once per composition and derives problems and status
  rows from that single pass.
- Phase projection is passive, recursive, and now truly orthogonal: Launch
  requires only `required; eager`, Completion only `required`, and a present
  eager value is hard at launch through the classifier, not through a
  presence lie.
- The DMLS surfaces consume Darkmatter's descriptor catalog for `eager`
  wording (the catalog-verbatim fixture group), so editor and validator
  cannot paraphrase apart.

## Verification performed for this review

| Command | Result |
|---|---|
| `BISCUIT_TEST_FILTER="test(completion) or test(closure) or test(write_grant) or test(inline) or test(shipped_prompt)" just test` (claudine/) | 585 passed, 6236 skipped, 0 failed — includes the uncommitted `inline_prompt.rs` matrix tests |
| `BISCUIT_TEST_FILTER="binary(/shipped_prompt/) or binary(/inline_compose_hash/) or binary(/inline_completion_lifecycle/)" just test` (claudine/) | 31 passed, 0 failed — drift pin with the restored eager `spec` declaration |
| `BISCUIT_TEST_FILTER="binary(schema_phase_validation) or binary(lsp_session) or binary(inline_document_text) or test(hash::write)" just test` (darkmatter/) | 142 passed, 7613 skipped, 0 failed |
| `just _test_l2 claudine-cli --features terminal-tests level2_completion_schema_failure… level2_unchanged_inline_body…` | 2 passed (real tmux, `MODEL` scrubbed) |
| Same recipe, `level2_lifecycle_equivalence_ac9_context_facets_match_direct_run` | fails with ambient `MODEL`; **passes** with `env -u MODEL` — environment leak, not a regression (Finding 3) |
| `just lint` in claudine/ and darkmatter/ | clean, including the `wasm32-wasip2` Zed extension leg |
| Manual code verification of all twelve resolution sites | all present as tabulated above |

A broader L2 sweep was not completed: with the ambient `MODEL` leak it halts
at Finding 3's failure, and a WezTerm chooser test
(`level2_wezterm_operation_file_multi_match_uses_choose_one`) is blocked by
the same Atuin onboarding dialog in the WezTerm login shell that Phase 8
documented as host-blocked. Neither is attributable to this fix's code.

## Recommendation

Add the interrupt-messaging assertions (Finding 1) — one test extension, the
scenario is already staged — then delete the scratch dump (Finding 2), scrub
`MODEL` in the AC9 equivalence tests (Finding 3), and commit the uncommitted
axis follow-through (Finding 5). With Finding 1 landed I would consider this
production ready: every other user-observable requirement in the spec is now
verified at the level the brief requires, and the two remaining low items
(Findings 4) can ride along in any later pass.
