---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-18T08:12:05-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: true
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-8.md
previous: 2026-07-13-proxy-with/review-7.md
next: 2026-07-13-proxy-with/review-9.md
---

# Review 8: Proxy With

## Verdict

The feature is **not ready for production**. Review 7's fixes materially improve
the non-loop initialization order, expand reachable resume refusals from one
facet to eight, and remove the reduced unowned-handoff path. However, the
implementation and its own acceptance map still identify two release criteria
as partial, and this review found that the retry launch gap is broader than the
map's AC15 classification implies.

Three high-severity blockers remain: a looping document validates its schema
before `initialize`; retry refreshes a document but launches several changed
facets through the invocation's stale provider/profile/argv/MCP bundle; and two
required resume-compatibility facets remain projection-only. The authoritative
composition documentation also describes behavior that the review-7 fix
deleted.

## Findings

### 1. High: looping documents still validate before their own `initialize`

R4 requires every fresh document to run its narrow safety gate and
`initialize` before schema validation and full pre-flight
(`spec.md:430-468`). AC11 is unqualified by document shape and repeats the same
ordering (`spec.md:901-903`). The non-loop branch now honors that contract, but
the loop branch does not. `execute_loop_or_single` receives the selected
`SchemaStage`, then constructs `loop_prepare_options` with
`defer_schema_verdict: false` unconditionally
(`cli/src/commands/compose/prep.rs:1041-1051`). The loop engine can therefore
reject iteration 1 before it emits the document's `initialize`.

This is user-observable for a looping target whose `initialize` supplies or
repairs a schema-required property: the same target succeeds as a non-looping
document but fails before initialization when it owns `loop:`. It also breaks
the specified direct-versus-proxy equivalence whenever the routed target is the
looping shape that motivated this feature.

The current Level 2 rows
`level2_lifecycle_initialize_precedes_schema_verdict_{direct,initialize_proxy,recovery_proxy}`
all exercise single-execution documents. There is no Level 2 row combining a
loop-owning target, initialize-time repair, and the schema verdict. The
acceptance map correctly marks AC11 partial, so this is both an implementation
gap and a wrong-level verification gap.

Thread the chosen `SchemaStage` into the loop preparation options and add Level
2 direct, initialize-proxy, and recovery-proxy rows for a looping document whose
`initialize` satisfies the schema. Include the still-invalid converse so the
captured pane proves `initialize` and the owed closure occur before the typed
diagnostic.

**Verification level:** partial Level 2 for non-looping documents; none for the
failing loop shape. Level 3 is not applicable.

### 2. High: retry refresh computes a new identity but launches a stale bundle

R8 requires retry's refreshed body, lifecycle, context, and launch plan to come
from one coherent prepared document (`spec.md:524-550`), and AC14 requires a
canonical refresh (`spec.md:910-913`). The current implementation splits those
authorities. `execute_attempt_phase` first builds the actual `AttemptLaunch`
from invocation-fixed `provider`, `profile`, `base_args`, interactivity, and
base environment (`loop_control.rs:1378-1391`). Only afterward does it call
`rebuild_launch_identity` and use that separate projection for the compatibility
key (`loop_control.rs:1428-1458`).

The production module documents the resulting limitation explicitly:
`rebuild_launch_identity` does not rebuild provider argv, MCP runtime injection,
or the profile actually spawned, and a retry still launches under the
invocation's argv (`loop_control/target_launch.rs:58-92`). Only the
`AGENT`/`MODEL`/`YOLO` environment projection reaches the child. A document that
changes `agent:`, `interactive:`, permission mode, profile/binary selection, or
MCP tags before a retry can therefore report a refreshed identity while the new
session runs through stale launch machinery.

This is not only an AC15 resume concern: retry opens a fresh session and is
supposed to use the refreshed launch plan. The acceptance map marks AC14
complete using state-slice, overlay, and budget tests, but none asserts which
binary/argv/MCP bundle the retried provider actually receives. Existing Level 2
retry rows cover invocation counts and overlay survival, not launch identity.

Create one typed, final per-attempt launch bundle from the refreshed document
and use that same value for spawning and for the compatibility key. This would
also remove the current double calculation (`rebuild_launch_env`, followed by
`rebuild_launch_identity`) and make the code both less divergent and cheaper.
Add Level 2 retry rows that mutate provider/profile, interactivity/structured
mode, permission mode, and MCP tags before the retry, then assert the actual
stub binary, argv, injected MCP configuration, and child environment.

**Verification level:** Level 2 exists for retry count and overlay lifetime;
the user-observable refreshed launch bundle has no Level 2 coverage. This is a
wrong-level and implementation gap. Level 3 is not applicable.

### 3. High: AC15 still lacks complete reachable resume compatibility

AC15 requires the complete compatibility key to be compared after canonical
refresh, with changed facets named and retry recommended (`spec.md:914-917`).
Review 7 now provides Level 2 refusal rows for model, provider, interactivity,
permission mode, and MCP server set; provider and interactivity rows also name
their coupled binary/protocol/structured-output facets. That is meaningful
progress.

Workspace/child CWD and system-prompt delivery/content remain projection-only
Level 1 checks. `target_launch.rs` explicitly excludes both from the live
rebuild (`target_launch.rs:44-56`), and the acceptance map accordingly leaves
AC15 partial. In particular, system-prompt content is frozen into invocation
argv instead of being rebuilt by the canonical retry/resume preparation service,
even though R3 assigns system-prompt launch planning to that service and R8
requires refusal when the refreshed plan changes.

Either make both facets part of the reachable canonical refresh and add Level 2
refusal/converse rows, or amend the specification before sign-off to define them
as immutable invocation inputs that cannot change during a same-document
resume. The implementation cannot be declared complete while the ratified
criterion requires behavior that only a projection unit test can simulate.

**Verification level:** Level 2 for eight coupled facets; Level 1 projection
only for workspace CWD and system prompt. This is a wrong-level verification
gap. Level 3 is not applicable.

### 4. Medium: authoritative composition documentation describes deleted behavior

The review-7 implementation now refuses an unowned proxy with
`LifecycleProxyWithoutOwningCoordinator`; `surface_or_adopt_terminal_proxy`
contains no env-only adoption arm. Both the repository topic and the required
Claudine skill still say direct provider wrappers adopt the target inside the
harness through an env-only `rebuild_target_launch` fallback
(`docs/topics/composition.md:607,661-665` and
`.claude/skills/claudine/composition.md:607,661-665`). Those same documents also
retain the older statement that only `model` has reachable Level 2 resume
coverage (`composition.md:646-650`), while the current suite has five isolating
rows covering eight coupled facets.

This violates the specification's documentation requirement and the repository
rule that behavior-changing edits update relevant comments and authoritative
docs. Replace the fallback section with the typed-refusal behavior and update
the compatibility discussion to match the exact current Level 1/Level 2 split.

### 5. Medium: direct-wrapper MCP intent is hardcoded off

`wrapper_stages.rs:505-514` constructs `LaunchRebuildIntent` with
`mcp_enabled: false`, even though direct wrappers expose `--mcp` and
`--mcp-use`. Consequently, if lifecycle retry/resume becomes reachable on that
path, refreshed body tags cannot affect the MCP compatibility facet. The
acceptance map already records this as an open latent divergence.

The current direct-wrapper lifecycle is default/empty, so this does not produce
a reachable proxy-with failure today. It should still derive the flag from the
actual wrapper arguments, with a Level 1 contract test, so future lifecycle
wiring cannot silently activate a stale compatibility path.

## Verification-level audit

| User-observable requirement | Strongest present | Assessment |
|---|---:|---|
| Proxy handoff consumption, lifecycle ownership, loop iteration equivalence, inline closure, sequence containment | Level 2 | Appropriate for covered document shapes |
| Dry-run suppresses lifecycle effects and dynamic traversal | Level 2 | Appropriate |
| Context and full launch equivalence for a newly proxied target | Level 2 | Appropriate on `compose`, `inline-compose`, and sequence coordinators |
| Target `initialize` before schema verdict, non-looping document | Level 2 | Appropriate |
| Target `initialize` before schema verdict, loop-owning document | None | **Mismatch / implementation gap** |
| Retry count, fresh session, budgets, and overlay survival | Level 1 plus selected Level 2 | Appropriate for those state/lifetime properties |
| Retry uses the refreshed provider/profile/argv/MCP launch bundle | None | **Mismatch / implementation gap** |
| Resume refusal for model/provider/interactivity/permission/MCP and coupled facets | Level 2 | Appropriate |
| Resume refusal for workspace CWD and system-prompt content/delivery | Level 1 projection only | **Mismatch / incomplete** |
| Shell approval and approved-versus-executed bytes | Level 2 | Appropriate |
| Typed diagnostic identity, lifecycle ordering, and pane rendering | Level 2 | Appropriate for covered paths |
| Overlay-value non-disclosure in terminal status | Level 2 pane capture | Appropriate |
| Pure `proxy.with` parsing, typed interpolation, precedence, shallow/null semantics | Level 1 | Appropriate for deterministic data semantics |
| Overlay persistence/chain behavior and no Markdown mutation | Level 2 backed by Level 1 | Appropriate |
| OS keyboard/input encoder behavior | None | Not applicable; this feature has no Level 3 input requirement |

## Validation performed

- Reviewed the complete specification, review 7, the current acceptance map,
  the review-7 implementation commits, authoritative Claudine skill/topic docs,
  and production/test call sites for loop preparation, retry/resume launch,
  session compatibility, unowned handoff refusal, and MCP intent.
- Used GitNexus to trace `execute_loop_or_single`,
  `materialize_attempt_prompt_phase`, and the unowned-handoff path before
  reading their source and tests.
- Started `just test-l2 initialize_precedes_schema_verdict` through the
  canonical area recipe. The command exceeded the non-interactive session's
  approximately 60-second command limit while compiling dependencies and was
  terminated with exit 130 before any test executed. This review therefore
  claims no fresh local test pass.
- The acceptance map records full-area `just test`, `just test-l2`, and
  `just lint` passes at this HEAD. Those green gates demonstrate that the
  existing suite passes; they cannot close the missing loop/retry rows or the
  projection-only resume facets above.
