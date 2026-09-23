---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-20T14:50:23-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-13.md
previous: 2026-07-13-proxy-with/review-12.md
next: 2026-07-13-proxy-with/review-14.md
---

# Review 13: Proxy With

## Verdict

The feature is **not ready for production**. The coordinator, canonical
preparation, launch rebuilding, and Level 2 direct/proxy matrix remain broadly
consistent with the specification, but the current tree does not have a green
Level 1 gate for all acceptance evidence. One test cited as the proof for
acceptance criterion 20 reproducibly exceeds nextest's per-test timeout, even
when run alone. The authoritative architecture skill also still states the
opposite of the implemented launch/loop behavior, leaving review 12 only
partially closed.

No requirement in this feature depends on the terminal emulator's keyboard
encoder, so Level 3 is not applicable. The user-observable terminal and process
requirements correctly target Level 2; the new blocker is failing Level 1
evidence for pure recursive `proxy.with` evaluation, not a missing Level 3 row.

## Findings

### 1. High: acceptance criterion 20 is marked complete against a test that times out

The acceptance map marks criterion 20 complete and cites
`proxy_with_evaluation::nested_strings_follow_the_same_interpolation_rule`
(`notes/acceptance-map.md:381`). That test is the only cited row that combines
nested arrays/objects with the top-level whole-value-versus-mixed-string typing
rule required by `spec.md:944-946` and the L1 strategy at `spec.md:1027-1031`.

It does not pass at this HEAD. A focused 98-test run completed 97 tests, then
nextest timed this test out at 30 seconds. Running the exact test by itself
reproduced the same timeout; nextest began its retry, and the review's bounded
55-second command cap terminated the still-running retry. This is not a cold
compile result: the binary finished building in 0.33 seconds before the isolated
test started.

The test exposes an avoidable evaluation shape. Its helper constructs the
public `StackExecutionContext` with `prepared_context: None`. Recursive
`resolve_with_value` evaluates each scalar leaf independently
(`executor.rs:1255-1283`); each leaf calls `build_state`, whose documented
fallback calls `ComposeContext::capture_for_content` again
(`executor.rs:491-544`). The nested fixture has enough string leaves for those
repeated captures to exceed the normal L1 timeout. Other focused interpolation
rows also took multiples of roughly three seconds, which supports the same
per-leaf cost rather than a one-off scheduler delay.

Resolve the contract rather than merely increasing the timeout or adding a
`slow_` prefix:

- If `prepared_context: None` is a supported public-library path, capture one
  event-scoped context/effective state and reuse it while walking the complete
  overlay. This removes repeated host discovery and improves real callers as
  well as the test.
- If canonical preparation is now an invariant for every supported caller,
  encode that invariant in the construction API instead of retaining an
  expensive optional fallback, and make the test supply the same prepared
  snapshot production uses.

Then rerun the complete Claudine Level 1 gate and keep criterion 20 marked
complete only when all cited rows pass under the normal nextest profile.

### 2. Medium: review 12's authoritative architecture correction is still absent

The specification requires stale reduced-harness and transition documentation
to be corrected (`spec.md:890-906`). The acceptance map's AC10/AC15 evidence is
now current, including the renamed prepared-MCP-tag tests and review-11 closure
rows. The architecture half of review 12 is still present verbatim, however.

`.claude/skills/claudine/architecture.md:491-495` says launch inputs and loop
recognition are computed once from the originally invoked document, and that a
proxied target cannot select its model or acquire its loop. The implementation,
the composition topic, the current acceptance map, and the Level 2 matrix all
say the opposite: the command-owned coordinator re-prepares a proxy target and
`rebuild_launch_identity` derives the attempt launch from the freshly
materialized active document.

Remove the obsolete **Known gap** paragraph and replace it with the current
boundary: surfaced composition commands re-enter the command-owned preparation
and launch pipeline, while a direct provider wrapper without an owning
coordinator refuses an otherwise unowned handoff. The branch contains no
post-review-12 implementation change that could implicitly close this item;
the current HEAD is the review-12 documentation commit itself.

## Review-12 closure

Review 12 is only partially closed:

1. The acceptance map now records the review-11 closure evidence for initial
   system-prompt lifetime, unavailable refreshed-provider selection,
   canonically prepared MCP tags, and rebuilt-provider warnings. The obsolete
   disk-lexing test name is gone.
2. The authoritative architecture skill still carries the obsolete **Known
   gap** statement and therefore remains inconsistent with both code and the
   acceptance map.

## Verification-level audit

| User-observable requirement | Strongest current evidence | Required level | Result |
|---|---|---|---|
| Direct/proxy prompt, frontmatter, context, lifecycle order, and loop equivalence | Level 2 tmux direct/proxy matrix | Level 2 | Present |
| Provider/model/profile/binary/argv/environment/interactivity/dispatch/CWD rebuild | Level 2 fake-child launch records and pane capture | Level 2 | Present |
| MCP selection/injection and file-backed system-prompt lifetime across re-entry | Level 2 fake-child argv/file reads, with L1 seams | Level 2 | Present |
| Shell discovery, target policy, approval/denial, and approved bytes equal executed bytes | Level 2 target-policy and shell-byte rows | Level 2 | Present |
| Overlay precedence, schema effects, retry/resume/loop lifetime, and chain forwarding | Level 2 end-to-end rows plus L1 state tests | Level 2 for effects; L1 for pure state | Present |
| Nested recursive `with:` interpolation and type preservation | L1 `nested_strings_follow_the_same_interpolation_rule` | Level 1 | **Failing: times out** |
| Typed diagnostic identity and route-equivalent rendered output | Level 2 three-route diagnostic matrix | Level 2 | Present |
| Inline closure, sequence containment, stream routing, dry-run behavior, and value redaction | Level 2 composition-command/pane rows | Level 2 | Present |
| OS keyboard/modifier encoding | No such requirement | Level 3 | Not applicable |

The Level 2 matrix is correctly Unix/tmux under the repository's platform
policy: Linux CI and macOS opt-in. Windows has platform-neutral Level 1 proxy
coverage but no claimed Level 2 terminal leg. That documented limitation is not
the readiness blocker found here.

## Validation performed

- GitNexus's `proxy-with` index matched HEAD. Symbol context confirmed that
  `materialize_attempt_prompt_phase` rebuilds launch identity from the
  canonically materialized document, and that `prepare_document` is shared by
  direct/proxy entry modes.
- A bounded full `just test-library` attempt reached 673 passing tests with no
  assertion failure before the session's 55-second command cap stopped the
  remaining 2,858 tests. It is not counted as a passing gate.
- Focused Level 1 proxy/coordinator/preparation execution ran 98 tests: 97
  passed; `nested_strings_follow_the_same_interpolation_rule` timed out at 30
  seconds. The exact test timed out again in isolation after a 0.33-second
  build.
- Focused CLI structural/system-prompt tests were attempted, but the cold CLI
  dependency graph was still compiling when the 55-second cap fired; no CLI
  test executed, so this is not counted as passing evidence.
- Level 2 was not rerun after the Level 1 blocker reproduced. The source audit
  verified that the cited suite is genuinely Level 2: it is Unix-gated, uses
  `require_level!(Level::L2, TmuxHarness::available(), "tmux")`, executes the
  shipped binary in tmux, and captures pane text through the terminal harness.
- The error-transport and lifecycle-document guards passed, and GitNexus change
  detection reported no affected execution flow from the documentation-only
  review-chain edits.
- Darkmatter parsed every requested review/spec frontmatter value successfully.
  Full schema validation could not run because the repository's existing
  `schemas/feature-review.yaml` uses the legacy tagged-schema shape rejected by
  the installed Darkmatter CLI; this is schema-infrastructure drift, not a
  `proxy.with` result.

## Production readiness

`ready: false`. Restore a green Level 1 criterion-20 test without papering over
the repeated-capture behavior, correct the authoritative architecture skill,
then rerun the normal Claudine `just test`, `just test-l2`, and `just lint`
gates before production sign-off.
