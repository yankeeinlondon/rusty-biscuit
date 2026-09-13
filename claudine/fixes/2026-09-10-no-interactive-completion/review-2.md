---
$schema: feature-review.yaml
description: A **fix** review of `2026-09-10-no-interactive-completion/spec.md`
fix: 2026-09-10-no-interactive-completion/review-2.md
spec: 2026-09-10-no-interactive-completion/spec.md
previous: 2026-09-10-no-interactive-completion/review-1.md
reviewed_by: claude/opus
created: 2026-09-10T16:05:56-07:00
log: claudine/fixes/2026-09-10-no-interactive-completion/log.md
implemented: false
ready: true
human_review: true
human_review_items:
    - |-
        **Is a pseudo-terminal test acceptable where the specification asked for the shared terminal harness?**

        The specification for this fix says the new interactive coverage should
        run "the shared terminal harness". The delivered tests instead spawn the
        command inside a pseudo-terminal (a software terminal that the test
        process itself drives), which is the pattern every neighbouring test file
        in this area already uses. The previous review recorded the same
        deviation and chose not to block on it; this review reaches the same
        conclusion, so the decision has now been deferred twice and is worth a
        one-time ruling from you.

        What the difference means in practice:

        - **Pseudo-terminal (what was built).** Proves the ordering and the data
          flow: the file-selection question is asked *before* the prompt document
          starts running, the file the user picks is the file that is read, and
          the choice survives being handed to a second document. It does not
          prove how a real terminal application (WezTerm, Kitty, tmux) draws the
          chooser or encodes keypresses.
        - **Shared harness (what the specification named).** Runs the command
          inside a real terminal application and reads back what is actually
          drawn on screen. It is the right tool when the thing under test is
          *appearance* — colors, box drawing, column widths, scrolling.

        My assessment: nothing about *appearance* changed in this fix. The
        confirmation dialog and the multi-file chooser are pre-existing widgets
        that were reused unmodified, and the keys the tests send (`y`, `n`,
        `Enter`, `Esc`, down-arrow) are all encoded identically by every
        terminal. So the pseudo-terminal level fits what is being verified.

        Your options:

        1. **Accept as-is** (my recommendation). Record the deviation and move
           on; the specification text is stricter than this change needs.
        2. **Add one real-terminal capture** of the accept-the-file path, so at
           least one test proves the dialog renders correctly inside a real
           terminal application. Roughly one new test; the patterns exist in
           `level2_dry_run_metadata_capture.rs`.
        3. **Loosen the specification's wording** for future fixes, so
           "pseudo-terminal is sufficient when no new rendering is introduced"
           becomes the written rule rather than a per-review judgement call.

        Related, and part of the same decision: the new terminal tests only run
        on macOS and Linux (the file that holds them is compiled out on
        Windows). On native Windows this flow is therefore covered only by the
        non-interactive tests, which check that an unresolvable file fails with
        a clear error rather than crashing. If you want the interactive path
        proven on Windows too, that is a separate piece of work and needs a
        Windows-capable terminal test approach that does not exist in this
        repository yet.
---

# Review 2: Restore Interactive Completion Before Initialize Consumes Caller File Inputs

## Summary

The two blockers from review 1 are fixed, and the fix for each one holds up
under independent checking rather than only under the author's own assertions.
The launch-area anchoring row of the specification — previously asserted in a
way that could not fail — is now pinned by a decoy candidate placed outside the
launch directory plus a companion test that launches from the repository root
and proves the decoy *is* discoverable from there. I mutated the production
scope derivation to anchor at the repository root and the anchoring test failed,
so the assertion is doing real work. The vacuous transcript assertion was
replaced with two assertions that both key on strings the code actually emits,
one of which additionally counts interactive widgets and would fail on any extra
prompt.

The four optional findings that touched the same files were also implemented: the
redundant second resolution pass is gated off for the caller's own document (and
that gate is safe — the only other entry into that function hard-codes the
non-first case), swallowed chooser errors now leave a debug trace, four drifted
comments were corrected, and a sequence-proxy regression test pins the typed
diagnostic that entry must produce. The two findings review 1 placed outside this
fix's scope were correctly left alone.

No functional defect was found. **This fix is production ready.** The three
items in Findings are non-blocking: one comment overstates a cost in one branch,
one is an efficiency observation whose remedy would cost more than it saves
today, and one is carried forward from review 1 as prompt content rather than
code.

## Verification performed

| Command (in `claudine/`) | Result |
| --- | --- |
| `just test supplied_ shipped_review_router proxy_target_partial sequence_proxy_target` | 27 passed, 0 failed |
| `just test-l2 provided_partial review_router proxy_target_schema` | 9 passed, 0 failed |
| `just lint` | clean — no errors, no warnings across the area's packages |
| Mutation check (see below) | the anchoring test fails when the production scope is mis-anchored |

**Mutation check.** Review 1's finding 1 was that a mis-anchored candidate scope
would pass every assertion. To confirm the new test really discriminates, I
temporarily changed the scope in
`claudine/cli/src/commands/schema_interactive/supplied.rs` from
`origin.base_dir()` to `origin.repository_root()` — exactly the mis-anchoring the
specification warns about — and ran
`level2_review_router_partial_yolo_confirms_before_initialize_and_survives_proxy`.
It failed: the single-file confirmation never appeared, because the root-anchored
scope found the out-of-area decoy and rendered the chooser instead. The mutation
was reverted and the file is byte-identical to the committed state.

The working tree also carries the unrelated in-flight silent-audio changes.
Nothing in those files intersects this fix, and the runs above are all scoped to
this fix's tests.

## Contract coverage

Every row of the specification's acceptance table is paired with its strongest
test below. Two level labels are given because this repository and this review's
rubric draw the line differently: the `rust-testing` skill counts a
pseudo-terminal test as L2 ("real terminal / PTY"), while the rubric in the
review prompt counts it as Level 1 because the test process manufactures the
input bytes rather than a terminal emulator's encoder producing them.

| Specification row | Strongest verification | Level (repo / rubric) | Assessment |
| --- | --- | --- | --- |
| Router shape, one match, TTY, `-y`: confirmation before the guard, proxy succeeds | `level2_review_router_partial_yolo_confirms_before_initialize_and_survives_proxy` | L2 / Level 1 | Adequate. Dialog precedes provider launch, the guard read the chosen file (`SELECTED=alpha`), the proxy prompt carries the chosen path, zero extra widgets. |
| Several matching specs: chooser, guard and proxy consume the selection | `level2_review_router_partial_chooser_keeps_selected_identity_in_proxy` | L2 / Level 1 | Adequate. Down-arrow picks the second candidate; `SELECTED=beta` proves the guard read that file; exactly one widget. |
| Only `spec` supplied while the router also declares required `plan` and `review`: no unrelated prompt | `assert_review_router_completed` now asserts the file-collection sentence never appears **and** that the interactive-widget count is exactly what the resolution needs; plus `shipped_review_router_literal_does_not_collect_absent_route_inputs` for the non-TTY path | L1 + L2 / Level 1 | Adequate. Review 1's vacuous assertion is gone; both replacements key on strings the code emits. |
| Valid literal file, with and without `initialize`: no dialog | `shipped_review_router_literal_does_not_collect_absent_route_inputs`; pre-existing literal tests in `compose_caller_file_provenance.rs` | L1 | Adequate. |
| Supplied partial without `initialize`: existing behavior intact | Four pre-existing `level2_pty_provided_partial_*` tests, all still green through the new early pass | L2 / Level 1 | Adequate. |
| No match / decline / cancel / non-TTY / disabled config / silent: typed failure, no guard dereference, no provider spawn | `level2_review_router_partial_decline_and_cancel_stop_before_initialize`; `shipped_review_router_non_tty_partial_fails_before_initialize`; `supplied_resolution_gates_and_cancellation_leave_both_maps_unchanged` (all four gates) | L2 + L1 | Adequate. |
| `initialize` supplies or repairs a value: lifecycle still precedes the verdict | Pre-existing `level2_lifecycle_initialize_precedes_schema_verdict_*` | L2 | Adequate, pre-existing. |
| Multiple supplied partials; required and eager-optional; file-array input; no stale values | `supplied_files_separate_resolution_from_schema_verdict`; `supplied_selections_preserve_array_slots_and_caller_provenance`; two file-array PTY tests | L1 + L2 | Adequate. Array slot indices, scalar shorthand, unrelated overrides, and record origins are all asserted. |
| Launch from a package area, then change runtime CWD or proxy elsewhere: candidates and identity stay anchored to launch context | `level2_review_router_partial_yolo_confirms_before_initialize_and_survives_proxy` (out-of-area decoy must stay invisible) + `level2_review_router_partial_repo_root_launch_widens_candidates_to_the_chooser` (same fixture from the root must see the decoy) | L2 / Level 1 | **Now adequate.** The pair is a genuine discriminator, confirmed by the mutation check above. `level2_proxy_target_schema_resolves_partial_once_before_its_initialize` extends the same proof across a proxy hop: the target lives at the repository root beside the decoy, yet only the launch-area candidate is offered. |
| Proxy / fresh preparation after selection: no duplicate prompt | `level2_proxy_target_schema_resolves_partial_once_before_its_initialize`; the one-occurrence assertion on the "did not match a file directly" line in every router test; the unit test's second call panics on any prompt | L2 + L1 | Adequate. |
| Root-union arm selection (implementation record) | `supplied_files_select_shipped_review_router_union_arm`; `supplied_files_leave_ambiguous_and_mismatched_union_arms_untouched` (4 shapes) | L1 | Adequate. |
| Sequence-owned proxy entry uses a denied policy (implementation record) | `sequence_proxy_target_partial_fails_typed_before_the_target_initialize` | L1 | **New.** Asserts the typed schema surface naming the adopted target and its own property, and asserts the absence of both the lifecycle evaluation error and the raw path error — the two symptoms that would appear if the pass were skipped for the adopted target. |

### Review 1 findings — disposition

| Finding | Disposition |
| --- | --- |
| 1. Launch-area anchoring asserted nowhere that could fail (high) | **Fixed and independently confirmed.** Decoy seeded at the repository root; companion root-launch test proves the decoy is reachable; mutation check proves the assertion fails on a mis-anchored scope. |
| 2. Vacuous `MissingProperties` assertion (medium) | **Fixed.** Replaced with `!contains("requires a valid file reference")` (a string `collect_file` really prints, verified at `schema_interactive/mod.rs:640`) plus an exact count of keyboard-enhancement pushes, which fails on any additional widget. |
| 3. Redundant early pass on the first document (low) | **Fixed, and the gate is safe.** The second call is now `if !first`. The only other entry into `prepare_and_run_active_document` is the sequence step's proxy loop, which passes `false` literally (`wrap/sequence/iterate.rs:699`), and the sole `first == true` caller always runs the pass beforehand. The new `proxy_target_partial_is_resolved_before_the_target_initialize` test locks in that the gate did not remove the coverage that matters. |
| 4. PTY rather than the shared terminal harness (medium, judgment call) | **Carried forward unchanged** as the single human-review item, with a recommendation. |
| 5. Comment drift around the old chooser branch (low) | **Fixed** at all four named sites. |
| 6. Chooser I/O errors reported as "no match" (low, pre-existing) | **Fixed.** Both swallow points now emit `tracing::debug!` with the property, the provided value, and the underlying error; the user-facing surface is unchanged. |
| 7. Sequence proxy entry untested (low) | **Fixed** by the new sequence test, which also documents honestly that the denied policy itself is not observable without a TTY. |
| 8. Shipped prompts that would still crash on a partial (informational) | **Correctly left alone**; restated in Findings below. |

## Findings

### 1. Perf-span comment overstates the cost in the `initialize` branch (low)

`claudine/cli/src/commands/compose/prep.rs:257-258` says the schema-validation
span "holds two effective-schema loads whenever the caller set a value." That is
true only for a document that does *not* declare `initialize`, where the supplied
pass loads the effective schema and the pre-validator loads it again. A document
that *does* declare `initialize` skips the pre-validator entirely, so the span
holds one load — and documents with `initialize` are exactly the ones this fix
exists for. Reword to "…two loads whenever the caller set a value on a document
without `initialize`", or drop the count and keep the first sentence.

The remaining double load in the non-`initialize` branch is inherent to
separating supplied-file resolution from the schema verdict. Removing it would
mean threading a loaded `EffectiveSchema` through the library's pre-validation
API, which is a wider change than the saving justifies; it is correctly not
attempted here.

### 2. Root-union arm selection runs before any cheap disqualifier (low, efficiency)

`unresolved_supplied_files` →
`supplied_file_shape` (`claudine/lib/src/composition/schema/supplied.rs:117`)
clones each union arm, relaxes eagerness on caller-owned file properties,
rebuilds a `DarkmatterSchemas` baseline, projects it, and validates the caller's
instance against it — once per arm — before anything has checked whether any
supplied property could be an eager `file` in *any* arm. For a union document
where the caller supplied only ordinary strings, that whole loop runs and then
finds nothing to do.

A cheap pre-filter would skip it: if no arm declares any supplied property as a
`file` carrying `eager`, return `None` immediately. That is a few lines over
metadata already in hand, with no behavior change. It is listed as low because
the loop only runs when the caller supplied at least one value, the single-shape
path (the common case) already short-circuits at the first match, and the
per-invocation cost has not been shown to be user-visible. Worth doing if this
path ever shows up in a `--perf` profile; not worth blocking on.

### 3. Shipped prompts declaring input files without `eager` (informational, outside this fix)

Carried forward from review 1's finding 8 so the audit result stays written down.
The early pass deliberately reaches only properties marked `eager`, because a
lazy file property may legitimately name a file the run is about to create. Two
shipped prompts declare *input* files without `eager`, so a partial supplied for
them is never offered a chooser:

- `prompts/_implement/implement-suggestions.md` — `spec: file(required; match(**/*spec*.md))`
- `prompts/clarify.md` — `design: file(required; match(**/*design*.md))`

`prompts/_implement/implement-suggestions.md` was edited in this working tree for
unrelated reasons during this cycle and still lacks `eager`, so the gap is
unchanged. This is prompt content, not code, and adding `eager` to shipped
prompts is a separate, cheap change that belongs outside this fix.

## Design observations

- **The anchoring test pair is the right shape.** One test proves the decoy is
  invisible from the launch area; its companion proves the decoy is visible when
  the launch area moves. Either assertion alone could pass for the wrong reason;
  together they isolate the launch origin as the cause. The proxy-target test
  adds a third angle, because there the router itself sits in the same directory
  as the decoy and still does not pull it in.
- **The widget count is a good proxy for "asked exactly one question".** It
  works because the two dialogs differ in how they take over the terminal: the
  multi-candidate chooser goes through the shared prompt runner and announces
  itself with the keyboard-enhancement push, while the single-candidate
  confirmation just enables raw mode. Verified against
  `completion/autocomplete_ui.rs:123-154`. Any additional prompt of either kind
  changes the count or trips the file-collection assertion beside it.
- **The `!first` gate is load-bearing in one direction only.** It can only ever
  *remove* a duplicate pass over the caller's own document, never the first pass
  over an adopted target, because the sequence coordinator hard-codes the
  non-first case and the compose coordinator always runs the pass before its
  loop. The new proxy-target test would catch a future caller that violated that.
- **The debug traces are placed where triage needs them.** Both swallow points
  log the property and the provided value, so a support report of "it said no
  file matched but a file exists" is now distinguishable from a genuine
  zero-candidate result without changing the user-facing text.
- **Cross-platform.** The new terminal tests inherit the file's `#![cfg(unix)]`
  gate, so Windows runs the non-interactive process tests and the unit tests
  only. Per the review instructions, cross-OS evidence is CI's responsibility and
  is not a readiness input; the practical consequence is folded into the human
  review item instead.
