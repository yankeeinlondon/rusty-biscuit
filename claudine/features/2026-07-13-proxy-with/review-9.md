---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-19T17:33:20-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-9.md
previous: 2026-07-13-proxy-with/review-8.md
next: 2026-07-13-proxy-with/review-10.md
---

# Review 9: Proxy With

## Verdict

The feature is **not ready for production**. Review 8's implementation closes
the previously reported loop/schema-ordering gap and makes the rebuilt retry
identity choose the binary, profile, session mode, permission mode, and MCP
injection used for the next spawn. However, the value described as the complete
launch bundle is still only part of the attempt's provider-dependent state.

Three high-severity runtime gaps remain. A retry that changes provider keeps
provider-shaped execution adapters and dispatch metadata from the opening
provider; the launch-plan replay carries the opening provider's rendered output
and sandbox flags and cannot remove stale environment values; and a resume
compatibility refusal occurs after `start` but returns without the required
`failure`/`finalize` lifecycle tail. In addition, the specification's required
Level 2 case for a control-plane overlay that installs lifecycle/shell
configuration exists only at Level 1.

## Findings

### 1. High: a provider-switch retry still runs through the opening provider's execution adapters

R6 and R8 require a retry's refreshed provider, launch plan, dispatch context,
and execution behavior to come from one coherent prepared document
(`spec.md:481-506`, `spec.md:524-550`). `execute_attempt_phase` now reads the
provider/profile/binary/argv from `rebuilt`, but immediately before doing so it
copies several provider-dependent values from invocation-fixed `state.run`:

- the original `structured_codex_output` artifact;
- the original profile's stdout/stderr noise prefixes;
- the original profile's structured-stderr suppression policy; and
- the original document's composition dispatch context, including
  `resolved_model` and selection reasons.

Those values are then passed unchanged to `execute_harness_attempt`
(`cli/src/commands/wrap/harness_orch/loop_control.rs:1441-1449,1612-1620`). This
contradicts the adjacent claim that no invocation-fixed launch state remains.

The failure is user-observable. For example, a non-Codex opening attempt that
refreshes to Codex can receive a rebuilt `--output-last-message` argv path while
`structured_codex_output` remains `None`; `attempt.rs:249-265,416-421` therefore
cannot load/render the final response from that path. Other provider switches
can filter stdout/stderr with the wrong provider's prefixes and publish stale
model/provider dispatch metadata.

The new Level 2 provider-switch retry row proves only that `goose` is followed
by the `gemini` binary (`level2_lifecycle_control.rs:813-859`). It does not
exercise Codex output recovery, noise policy, stderr policy, or dispatch
context. The existing proxy-target launch-equivalence rows do not cover this
same-document retry path because a proxy target re-enters the outer command
pipeline.

Make the per-attempt rebuilt value own every provider-dependent execution
adapter, not only spawn inputs. Derive noise/suppression policy from the rebuilt
profile, carry a rebuilt Codex output artifact/path as one value, and recompute
the dispatch context from the refreshed target/model. Add Level 2 retry rows
that switch into and out of Codex and assert final output, pane filtering, and
the dispatched provider/model metadata.

**Verification level:** Level 2 for the selected binary only; none for the
provider-dependent execution adapters. This is a wrong-level and implementation
gap. Level 3 is not applicable.

### 2. High: launch-plan replay preserves stale provider-shaped argv and environment

The re-entrant builder records `output_format_args` and `sandbox_args` after the
opening profile renders them (`composition/pipeline.rs:761-771,878-886`). On a
provider change, `replay` appends those byte slices verbatim
(`wrap/launch_plan.rs:387,421`) even though its own field documentation says the
output flag shape is provider-owned and replayable only while the provider
holds still (`launch_plan.rs:121-125`).

This breaks immutable CLI intent when its provider-specific encoding changes.
For example:

- Goose renders `--output json` as no argv, while Gemini requires
  `--output-format json`; a Goose-to-Gemini retry silently loses the requested
  format.
- Gemini-to-Goose can pass Gemini's unsupported `--output-format` bytes.
- Goose does not implement `--sandbox`, while Codex does; replaying the opening
  slice either loses the requested sandbox on Goose-to-Codex or leaks Codex's
  flag in the reverse direction.

The environment has the same stale-layer problem. `build_harness_launch`
clones the opening `base_env` and only inserts rebuilt overlays
(`harness_orch/launch.rs:54-60`); there is no removal operation. When a refresh
removes `model:`, `launch_env_overrides` simply omits `MODEL`
(`target_launch.rs:376-387`), so the opening document's `MODEL` remains in the
actual child environment. The Level 1 test
`rebuild_omits_model_when_target_pins_none` checks only the overlay vector and
incorrectly treats omission as clearing the base environment. Provider-specific
base values such as an old `OPENCODE_CONFIG_CONTENT` or shadow `HOME` can leak
for the same reason.

Store semantic invocation intent (`OutputFormat`, sandbox requested) and render
it through the rebuilt profile. Represent environment changes as a typed patch
that supports both set and remove, or rebuild the provider-owned environment
from an invocation-neutral base. Add Level 2 rows for provider-switch retry with
`--output`, `--sandbox`, model removal, and provider-specific environment
cleanup; assert the complete actual argv/environment, not only flag-shaped
substrings selected by the current recorder.

**Verification level:** Level 2 for provider binary, session mode/YOLO, and MCP
addition; none for output/sandbox re-encoding or environment removal. This is a
wrong-level and implementation gap. Level 3 is not applicable.

### 3. High: resume incompatibility skips the lifecycle tail after `start`

A resumed attempt re-enters at `start`. The compatibility comparison then runs
in `execute_attempt_phase`; when it detects changed facets it returns
`LifecycleResumeIncompatible` directly
(`loop_control.rs:1564-1573`). It does not call the same
`emit_failure_finalize_with_err` path used by later pre-spawn/attempt failures.

The behavior is already observable and deliberately locked in by Level 2:
`level2_lifecycle_resume_refuses_when_refresh_changes_model` expects the second
`start` and explicitly asserts that neither `failure` nor `finalize` fires
(`level2_lifecycle_control.rs:1408-1417`). The acceptance map and authoritative
composition/lifecycle documentation call this an open question.

It is not safe to leave open for production. The ratified lifecycle contract
routes an error after `start` through `failure` and then exactly one `finalize`
(`features/_completed/2026-05-12-lifecycle/spec.md:624-628,665-669`). Skipping
that tail prevents cleanup and `err`-aware recovery authored for the active
document. It also makes compatibility refusal the only post-`start`, pre-spawn
failure with this lifecycle shape.

Route `LifecycleResumeIncompatible` through the shared typed catch protocol,
keeping the incompatibility diagnostic as the active error and preserving the
no-second-spawn guarantee. Update all five Level 2 refusal rows to assert
`start -> failure -> finalize`, exactly once, with the expected `err` facets.

**Verification level:** Level 2 currently verifies the wrong lifecycle order.
Level 3 is not applicable.

### 4. High: the required control-plane overlay shell-policy case is only Level 1

AC25 states that a `proxy.with` value may install lifecycle/shell configuration
but cannot bypass target-side policy. The Test Strategy explicitly requires a
Level 2 case where a control-plane overlay adds target lifecycle/shell
configuration and proves target validation and approval still run
(`spec.md:1020-1023,1079-1081`).

The acceptance map marks AC25 complete using only in-process tests. In
particular,
`a_shell_command_installed_by_the_overlay_stays_subject_to_target_side_policy`
constructs and audits the parsed lifecycle directly. The Level 2
`proxy_shell_approved_bytes_equal_executed_bytes` row covers a different case:
the target authors the shell action and the overlay supplies only a data value
interpolated into it. Likewise, the Level 2 later-event denial row uses a
target-authored shell action.

Add Level 2 allow and deny cases where `proxy.with` itself installs the target's
`initialize` or later-event shell stack. Assert captured-pane diagnostics,
provider non-launch on denial, and the exact executed bytes on approval. Until
then the user-observable policy boundary is verified at the wrong level.

**Verification level:** Level 1 only for overlay-installed control-plane shell
configuration; related but non-equivalent Level 2 coverage exists. Level 3 is
not applicable.

### 5. Medium: the shipped motivating route has no automated regression row

The Test Strategy requires the shipped `prompts/implement.md` ->
`prompts/_implement/implement-plan.md` route to execute all phases exactly as a
direct invocation does (`spec.md:1057-1060`). The Level 2 loop-equivalence test
explicitly states that this shipped route remains a manual smoke case
(`level2_lifecycle_control.rs:3121-3141`).

The synthetic loop fixture is valuable and exercises the underlying
coordinator, so this is not a missing verification level for generic loop
ownership. It does not protect the actual prompt files, their routing
conditions, or their multi-phase loop schema from drifting independently. Add
the specified self-contained fake-provider Level 2 regression using the shipped
documents (or checked-in fixture copies mechanically held in sync).

## Verification-level audit

| User-observable requirement | Strongest present | Assessment |
|---|---:|---|
| Proxy handoff ownership, loop equivalence, inline closure, sequence containment | Level 2 | Appropriate for the covered fixtures |
| Fresh target provider/model/MCP/argv/CWD/system-prompt equivalence | Level 2 | Appropriate on coordinator-owned proxy entry |
| Loop target `initialize` before schema verdict, including invalid converse | Level 2 | Review 8 gap closed |
| Retry selects the refreshed provider binary | Level 2 | Appropriate but too narrow |
| Retry rebuilds output/sandbox encoding, provider adapters, dispatch context, and environment removals | None | **Mismatch / implementation gap** |
| Resume incompatibility detection and rendered facets | Level 2 | Detection appropriate; lifecycle order asserted incorrectly |
| Overlay-installed lifecycle/shell configuration remains policy-bound | Level 1 | **Mismatch; specification requires Level 2** |
| Approved bytes equal executed bytes for target-authored shell using overlay data | Level 2 | Appropriate, but does not cover overlay-installed shell configuration |
| Typed diagnostic rendering and overlay-value non-disclosure | Level 2 | Appropriate for covered paths |
| Pure parsing, typed interpolation, precedence, shallow/null semantics | Level 1 | Appropriate |
| Shipped implement-route multi-phase regression | Manual only | Coverage gap; generic behavior has Level 2 |
| OS keyboard/input encoder behavior | None | Not applicable; no Level 3 requirement |

## Validation performed

- Read the complete specification, review 8, the current acceptance map,
  authoritative Claudine composition/lifecycle docs, the review-8 implementation
  commits, and the production/test paths for schema staging, launch-plan replay,
  retry/resume execution, compatibility refusal, control-plane overlays, and the
  Level 2 equivalence matrix.
- Used GitNexus to locate the canonical handoff/retry flows and inspect
  `build_launch_plan` callers and tests. The refreshed index found no named
  execution process for this path, so source/call-site tracing supplied the
  decisive evidence.
- Started the canonical `just test` area recipe. `claudine-catalog-types` ran
  21 tests successfully; the command then exceeded the non-interactive
  approximately 60-second ceiling while compiling `claudine` dependencies and
  was stopped with exit 130 before any Claudine feature test executed.
- Started a focused nextest run for the launch-plan and target-launch tests. It
  likewise exceeded the command ceiling during compilation and was stopped
  with exit 130 before tests executed. No fresh L1/L2/lint pass is claimed by
  this review.
- Started the canonical `just test-l2` area recipe in its parallel self-spawn
  mode. It also reached the command ceiling while compiling dependencies and
  was stopped with exit 130 before any Level 2 test executed; the recipe left
  no shared broker pane running.
- The acceptance map records earlier full `just test`, `just test-l2`, and
  `just lint` passes at this HEAD. Those runs show that the existing suite is
  green; they do not exercise the missing branches above.

## Production readiness

Production readiness requires the retry bundle to be genuinely provider- and
document-coherent, resume refusal to honor the lifecycle contract, and AC25's
policy boundary to have its required Level 2 proof. Because those conditions
are not met, `ready` remains `false`.
