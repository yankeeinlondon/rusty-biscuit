---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-10-no-interactive-completion/spec.md`
fix: 2026-09-10-no-interactive-completion/review-1.md
spec: 2026-09-10-no-interactive-completion/spec.md
reviewed_by: claude/fable
created: 2026-09-10T14:02:57-07:00
log: claudine/fixes/2026-09-10-no-interactive-completion/log.md
implemented: true
implemented_by: claude/opus
next: 2026-09-10-no-interactive-completion/review-2.md
ready: false
human_review: false
---

# Review 1: Restore Interactive Completion Before Initialize Consumes Caller File Inputs

## Summary

The implementation delivers the contract the spec asked for. A caller-supplied
partial for an eager `file(match(...))` property is now resolved through the
existing confirmation/chooser flow before a document's `initialize` stack can
dereference it, the selection is written into both the effective overrides and
the caller provenance records so a proxy target consumes the same identity
without a second dialog, and missing-value collection plus the full schema
verdict still wait for `initialize`. The reported invocation shape (the
shipped review router, a root-level schema union, `-y`, proxy to
`feature-review.md`) is exercised end to end by a PTY test that fails on the
old code and passes on the new.

No functional defect was found in the production code. The review is marked
**not ready** for one reason: a spec acceptance row about launch-area anchoring
has no test that can distinguish correct from incorrect behavior, and the
production code that derives the candidate-search scope has no direct coverage
at all. One further transcript assertion is vacuous. Both are small test
changes, not code changes.

## Verification performed

| Command (in `claudine/`) | Result |
| --- | --- |
| `just test supplied_ shipped_review_router` | 25 passed (5 library unit tests, 2 CLI unit tests, 2 CLI process tests, plus pre-existing name matches) |
| `just test-l2 provided_partial review_router proxy_target_schema` | 8 passed (4 pre-existing partial-file PTY tests, 4 new router/proxy PTY tests) |

The unrelated working-tree modifications from the in-flight silent-audio fix
(`PLAYA_DRY_RUN` env in the provenance test helpers) were present during these
runs and do not interact with this fix.

## Contract coverage

Each requirement from the spec is paired with the strongest test that verifies
it. In this repo the `rust-testing` skill classifies expectrl PTY tests as L2
("Real terminal / PTY"); the review rubric in the prompt would call the same
tests Level 1 because the test process manufactures the input bytes. Both
labels are given below so the reader can apply either standard.

| Spec row | Strongest verification | Level (repo / rubric) | Assessment |
| --- | --- | --- | --- |
| Router shape, one match, TTY, `-y`: confirmation before guard, proxy succeeds | `level2_review_router_partial_yolo_confirms_before_initialize_and_survives_proxy` | L2 / Level 1 | Adequate. Asserts dialog precedes provider launch, guard read the selected file (`SELECTED=alpha`), proxy prompt carries the selected path, no lifecycle error. |
| Several matching specs: chooser, guard and proxy consume the selection | `level2_review_router_partial_chooser_keeps_selected_identity_in_proxy` | L2 / Level 1 | Adequate. Down-arrow selects the second candidate and `SELECTED=beta` proves the guard read that file. |
| Only `spec` supplied, router also requires `plan`/`review`: no unrelated prompt | `shipped_review_router_literal_does_not_collect_absent_route_inputs` (non-TTY success proves no missing-value error) and the router PTY tests | L1 + L2 / Level 1 | Adequate for the non-TTY path. The PTY-side assertion is vacuous (finding 2). |
| Valid literal, with and without `initialize`: no dialog | `shipped_review_router_literal_does_not_collect_absent_route_inputs`; pre-existing literal tests in `compose_caller_file_provenance.rs` | L1 | Adequate. |
| Supplied partial without `initialize`: existing behavior intact | Four pre-existing `level2_pty_provided_partial_*` tests still pass through the new early pass | L2 / Level 1 | Adequate. Note these now exercise the new code path, not the old chooser branch. |
| No match / decline / cancel / non-TTY / disabled config / silent: typed failure, no guard dereference, no spawn | `level2_review_router_partial_decline_and_cancel_stop_before_initialize`; `shipped_review_router_non_tty_partial_fails_before_initialize`; `supplied_resolution_gates_and_cancellation_leave_both_maps_unchanged` (all four gates) | L2 + L1 | Adequate. |
| `initialize` supplies or repairs a value: lifecycle still precedes verdict | Pre-existing `level2_lifecycle_initialize_precedes_schema_verdict_*` (4 routes) | L2 | Adequate, pre-existing. |
| Multiple partials, required + eager-optional, file-array shape, no stale values | `supplied_files_separate_resolution_from_schema_verdict`; `supplied_selections_preserve_array_slots_and_caller_provenance`; two file-array PTY tests | L1 + L2 | Adequate. Array slot indices, scalar shorthand, unrelated overrides and provenance origin are all asserted. |
| Launch from a package area, CWD switch or proxy elsewhere: candidates and identity anchored to launch context | Router PTY tests launch from `packages/example` with the router at repo root | L2 / Level 1 | **Gap (finding 1).** The fixtures contain no candidate outside the launch area, so a scope anchored at the repo root would pass identically. |
| Proxy/fresh preparation after selection: no duplicate prompt | `level2_proxy_target_schema_resolves_partial_once_before_its_initialize`; `matches("did not match a file directly").count() == 1` in every router test; the unit test's second `resolve_supplied_file_inputs_with` call panics on any prompt | L2 + L1 | Adequate. |
| Root-union arm selection (implementation record) | `supplied_files_select_shipped_review_router_union_arm`; `supplied_files_leave_ambiguous_and_mismatched_union_arms_untouched` (4 shapes) | L1 | Adequate. |

The spec also asked for the L2 coverage to use the shared terminal harness.
The delivered tests use the expectrl PTY pattern of the sibling
`level2_schema_prompt_pty.rs` suite instead. See finding 4 for why this is
noted but not treated as blocking.

## Findings

### 1. Launch-area anchoring is asserted nowhere that could fail (high)

Spec: "Use the frozen caller launch origin for caller-owned file inputs and
candidate discovery. Do not recapture ambient process CWD after the wrapper
changes directories." Acceptance row: "Launch from a package area, then change
runtime CWD or proxy to another directory: candidates and selected caller
identity remain anchored to launch context."

The production scope derivation lives in the closure inside
`resolve_supplied_file_inputs` at
`claudine/cli/src/commands/schema_interactive/supplied.rs:27-34`, which builds a
`ScopeContext` from `origin.base_dir()`. That closure is only reached through
process-level tests. The unit test
`supplied_selections_preserve_array_slots_and_caller_provenance` asserts the
origin handed to a *fake* selector equals the launch context, which proves the
plumbing up to the closure boundary but not the closure itself.

The router PTY fixture (`review_router_fixture`) seeds specs only under
`packages/example/fixes/...`, and the process launches from `packages/example`.
If the closure walked from the repository root, from the router's directory, or
from the ambient CWD after the switch to the repo root, every candidate would
still be found and every assertion would still pass.

Required change: seed a decoy that matches the substring outside the launch
area, for example `fixture.cwd().join("fixes/2026-09-10-local-decoy/spec.md")`
at the repository root, in the single-match variant. Then assert that the
dialog is the single-file confirmation (`Use this file`) and not the chooser,
and that the provider prompt contains `2026-09-10-local-a/spec.md` and not
`local-decoy`. A second variant should switch the launch directory to the
repository root and assert the chooser now appears with both candidates. This
is one fixture line plus two assertions in the existing PTY file.

### 2. Vacuous assertion in the router PTY helper (medium)

`assert_review_router_completed` at
`claudine/cli/tests/level2_provided_partial_file_pty.rs:377` asserts
`!plain.contains("MissingProperties")`. That string is a Rust enum variant name.
The renderer maps it to the kind string `composition.missing_properties` and
the human text never contains the variant name, so the assertion can never
fail and does not verify the "no unrelated `plan`/`review` prompt" row on the
TTY path.

Replace it with an assertion on something the missing-value flow actually
emits. The missing-property chooser prints
`The <name> requires a valid file reference; choose from the files below:`
(`schema_interactive/mod.rs:634`), so asserting the transcript does not contain
`requires a valid file reference` is sufficient. Asserting exactly one raw-mode
entry before `provider reached` would be stronger still.

### 3. Redundant early pass on the first document (low, efficiency)

`run_composition_inner` runs `resolve_supplied_file_inputs` at
`claudine/cli/src/commands/compose/prep.rs:212` and then
`prepare_and_run_active_document` runs it again at line 453 for the same
document when `first` is true. The second call loads the effective schema a
second time (`load_effective_schema_in_context`, which parses and validates the
schema) and, for a root-level union, re-runs Darkmatter validation per arm. It
always finds nothing because the first call already installed every selection.

The second site is the right one for proxy targets, which can be the first
document to declare the input's schema. For the caller's own document it is pure
cost. Gate the call at line 453 with `!first`, or move the first-document call
out of `run_composition_inner` entirely if the non-initialize branch does not
need it before `pre_validate_with_interactive_collection` (it does need it, so
gating on `!first` is the smaller change). This is also worth a one-line note in
the perf substage comment since "schema validation" timing now includes the
schema load twice.

### 4. L2 coverage uses a raw PTY rather than the shared terminal harness (medium, judgment call)

The spec says: "Add L2 terminal coverage that reaches the real initialize/proxy
pipeline ... Use the shared terminal harness without bringing terminal or
browser windows into focus." The delivered tests use `expectrl` PTY sessions,
matching the sibling `level2_schema_prompt_pty.rs` and the pre-existing tests in
the same file.

This is not treated as blocking because (a) the repo's own taxonomy in the
`rust-testing` skill counts PTY as L2, (b) the requirements verified here are
ordering and data flow, not glyph rendering or the terminal's key encoder, and
(c) the confirmation and chooser widgets are pre-existing and unchanged. It is
recorded so the deviation from the spec text is an explicit decision rather
than an oversight. If Ken wants a tmux capture for the acceptance path, the
harness patterns in `level2_dry_run_metadata_capture.rs` apply directly.

### 5. Comment drift around the old chooser branch (low)

Behavior moved but three comments still describe the old ownership:

- `claudine/cli/src/commands/schema_interactive/mod.rs:114-119` says the
  pre-validator's "interactive collection below still runs here, and its own
  outcomes (a resolved partial, a zero-match failure) are never deferred." That
  branch is now the fallback for shapes the early pass defers (root unions
  with zero or several applicable arms, referenced arms). Say so.
- `claudine/cli/src/commands/schema_interactive/mod.rs:188-197` documents
  `resolve_unresolved_file_reference` as the way a partial is resolved. Add one
  sentence that supplied partials are normally completed earlier by
  `resolve_supplied_file_inputs` and this is the residual path.
- `claudine/cli/src/commands/compose/prep.rs:235-237` says interactive
  collection "may have ... replaced a `file(match)` partial." Still true for the
  residual path but misleading as the primary description. Reword.
- `claudine/cli/tests/level2_provided_partial_file_pty.rs:1-29` module doc
  describes only the Phase 3 tests and does not mention the router/proxy tests
  appended by this fix or which fixture they use.

### 6. Chooser I/O errors are reported as "no match" (low, pre-existing)

The production closure at
`claudine/cli/src/commands/schema_interactive/supplied.rs:34-40` uses
`.ok().flatten()` and `.ok()??`, so a terminal I/O failure in the chooser or a
resolution error on the chosen path surfaces as the "no existing file matched
reference" diagnostic. `resolve_unresolved_file_reference` already behaved this
way (`Ok(None) | Err(_) => downgrade`), so this is not a regression. If the
chooser ever fails for a reason other than the user's choice, the message will
mislead. A `tracing::debug!` of the swallowed error at that point would make
support triage possible without changing the user-facing surface.

### 7. Sequence proxy entry has no test for the denied policy (low)

`run_step_proxy_loop` passes `InteractiveSchemaOptions::default()` (all gates
false) at `claudine/cli/src/commands/wrap/sequence/iterate.rs:704`, which the
spec permits ("This fix does not introduce new sequence interaction modes").
No test drives a sequence step's proxied target with a supplied partial. The
expected result is the typed schema diagnostic before the target's
`initialize`, not a lifecycle evaluation error. One L1 process test in the
sequence suite would pin that.

### 8. Shipped prompts that would still crash on a partial (informational, outside the spec)

The early pass only handles properties carrying `eager`, and the docs now
state that lazy file properties may name future output files. Two shipped
prompts declare input files without `eager`:

- `prompts/_implement/implement-suggestions.md:3` `spec: file(required; match(**/*spec*.md))`
- `prompts/clarify.md:7` `design: file(required; match(**/*design*.md))`

`clarify.md`'s `initialize` only tests truthiness, so it will not crash, but a
partial passed for `design` will not be offered a chooser either. This is
prompt content, not code, and is not a readiness item for this fix. It is
recorded so the shipped-prompt audit the spec asked for has a written result.

## Design observations

- **Root-union arm selection is conservative and correct.** Relaxing only
  `eager` on caller-owned file properties, keeping every other constraint, and
  deferring when zero or several arms validate means the pass never guesses a
  file type. The four ambiguous-shape cases in
  `supplied_files_leave_ambiguous_and_mismatched_union_arms_untouched` are the
  right ones.
- **Atomic install of both maps.** `resolve_supplied_file_inputs_with` builds
  cloned maps and writes them back only after every selection succeeds, so a
  cancellation midway through several partials leaves overrides and provenance
  untouched. The gate test asserts this for each of the four gates.
- **Cheaper than the old chooser branch.** The new scope avoids
  `ScopeContext::discover()`, which forks `sniff` repo detection; the old
  fallback path still pays that when reached. Labels may differ slightly
  between the two paths when the sniff repo root differs from the git root, but
  both produce repo-relative labels in the common case.
- **Fixture coupling to the shipped router is intentional.** Both the library
  test and the CLI tests `include_str!` `prompts/review.md`. The spec asked for
  a shipped-router regression fixture, so this coupling is the point. Anyone
  reshaping that prompt's schema will need to revisit these tests, which is
  desirable.
- **Windows.** The PTY file is `#![cfg(unix)]`, matching sibling suites, so
  Windows runs only the two non-TTY process tests and the unit tests. Native
  Windows and WSL2 evidence remain pending per the plan and are a CI matter,
  not a readiness input for this review.

## Required before `ready: true`

1. Finding 1: add the out-of-launch-area decoy and assertions.
2. Finding 2: replace the vacuous `MissingProperties` assertion.

Findings 3, 5, 6, and 7 are recommended in the same pass since they are small
and touch the same files. Finding 4 is a documented judgment call. Finding 8 is
outside this fix's scope.
