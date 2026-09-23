---
$schema: feature-review.yaml
ready: false
agent: codex/default
created: 2026-07-16T20:59:13-07:00
spec: 2026-07-13-proxy-with/spec.md
implemented: false
description: A **feature** review of `2026-07-13-proxy-with/spec.md`
feature: 2026-07-13-proxy-with/review-1.md
previous: /
---

# Review 1: Proxy With

## Verdict

The feature is **not ready for production**. The tree contains a reviewed
specification and a detailed 14-phase execution plan, but the production
implementation has not started: all 120 plan tasks remain unchecked. The
current runtime still has the exact split ownership, reduced re-composition,
and out-of-band proxy channels that the specification is intended to replace,
and a mapping-valued `proxy.with` is rejected by the existing action parser.

Existing Level 1 and Level 2 tests establish useful baseline behavior for the
legacy proxy implementation. They do not verify the new authoring surface or
the required direct-versus-proxy equivalence contract.

## Findings

### 1. Critical: `proxy.with` is not implemented and valid specified syntax is rejected

`LifecycleControlAction::Proxy` carries only `target: Expr`
(`claudine/lib/src/composition/lifecycle/actions.rs:125-130`). The key/value
parser rejects every direct object-valued parameter before verb-specific action
construction (`claudine/lib/src/composition/lifecycle/parse.rs:791-805`), and
the proxy long-form builder consumes only `target`
(`claudine/lib/src/composition/lifecycle/action_shape.rs:511-521`). Therefore
the specification's required form:

```yaml
- action: proxy
  target: prompts/next.md
  with:
      spec: "{{ spec }}"
```

fails as unsupported object data instead of producing a typed proxy overlay.
There is no `EvaluatedProxyRequest`, resolved `ProxyHandoff`, overlay evaluator,
source-time recursive interpolation, shallow merge/null-removal behavior, or
immediate-target overlay lifetime in production code.

This leaves acceptance criteria 18–27 unimplemented, including schema
satisfaction, caller-override precedence, typed whole-value interpolation,
atomic evaluation failure, retry/resume/loop retention, downstream replacement,
control-plane policy enforcement, and the no-write/no-hash contract.

**Required change:** implement the typed proxy descriptor as the exact exception
to the generic object-parameter rejection, preserve the mapping as structured
expressions, evaluate it once through Darkmatter's existing subtree-composition
path, and transport the resulting JSON-compatible values in a typed request and
resolved handoff. Add the specified Level 1 parser/evaluator/state tests and
Level 2 end-to-end overlay cases before treating the surface as available.

**Verification level:** none for `proxy.with`. Level 1 is the minimum for parser,
typing, interpolation, precedence, atomicity, and lifetime invariants; Level 2
is required for the observable CLI handoff/schema/provider behavior. Both are
absent.

### 2. Critical: Proxy still changes document identity inside the attempt harness instead of using one active-document coordinator

The provider-neutral transition still carries only an authored string as
`LifecycleTransitionDecision::ProxyHandoff { target: String }`
(`claudine/lib/src/composition/lifecycle/runtime.rs:66-97`). Initialize routing
then resolves that string and returns it through a separate
`Option<PathBuf>` channel (`claudine/cli/src/commands/wrap/composition/pipeline.rs:1127-1215`),
which is threaded into the harness at `pipeline.rs:1393-1408`. Terminal proxy
routing independently resolves the target and directly mutates
`prompt_state.source_path`, `original_ref`, prompt/session overrides, and guard
state (`claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs:176-227`).

Retry and proxy re-entry also continue to use
`materialize_harness_prompt`, which captures a new ambient `ComposeContext`,
builds a private option set, and calls Darkmatter `compose_with` directly
(`claudine/cli/src/commands/wrap/harness_orch/prompt.rs:60-100` and
`:190-260`). That is the reduced second composer forbidden by R3. The feature
plan records the same current-state facts and notes that loop ownership is
still chosen from the router before its initialize proxy fires
(`claudine/features/2026-07-13-proxy-with/plan.md:64-123`).

Consequently, acceptance criteria 1–17 are not met: no single coordinator owns
identity, no four-layer state model exists, direct/proxy/retry/resume do not use
one preparation service, target initialize is not stabilized before loop
selection, target launch state is not rebuilt canonically, and retry/resume do
not implement the specified compatibility and budget contracts. The motivating
multi-phase routed-document defect remains structurally present.

**Required change:** complete the coordinator/state/preparation refactor before
adding route-specific patches. All proxy producers must return one typed
transition; only the coordinator may resolve and commit it. Direct, proxy,
retry, and resume must enter the same staged preparation service with explicit
entry reasons, while loop iterations reuse the prepared structural plan.

**Verification level:** existing Level 1 transition tests and Level 2 proxy
tests cover pieces of the legacy routes, but no test proves the new ownership
or canonical-preparation contract. The required Level 1 state model and Level 2
direct/proxy equivalence matrix are absent.

### 3. High: Handoff resolution and failures are neither atomic nor consistently typed

Initialize proxy resolution calls `resolve_proxy_target` but immediately wraps
its typed error in `eyre!` (`pipeline.rs:1157-1162`). Terminal proxy recovery
uses the lower-level `resolve_harness_path` directly and also stringifies its
error (`control_dispatch.rs:176-184`). This bypasses the existence check in
`resolve_proxy_target` (`claudine/lib/src/composition/lifecycle/control.rs:224-252`),
so the same missing target can fail at different stages depending on which
lifecycle signal produced the proxy.

The current transition has no overlay or provenance, and resolution/cycle
handling occurs independently in multiple CLI paths. It cannot guarantee the
specified evaluated-request-versus-resolved-handoff boundary, source/event/key
diagnostics, no-partial-commit rule, or identical typed failure identity across
direct, initialize-proxy, and terminal-recovery-proxy routes. This violates
acceptance criteria 4, 21, and 28–29.

**Required change:** preserve concrete errors to the render boundary, attach
proxy provenance, resolve exactly once through the shared file-reference
authority, and validate overlay/target/cycle/hop state before committing the
active document. Route pre-finalize failures through the existing catch/finalize
protocol without duplicating terminal events; surface post-finalize failures
directly.

**Verification level:** legacy typed-error and rendered-block cases have partial
Level 1/Level 2 coverage, but there is no cross-route identity assertion and no
`with.<key>` diagnostic. Level 1 typed identity plus Level 2 rendered diagnostic
comparison is required and absent.

### 4. High: The required Level 2 equivalence matrix is missing

`claudine/cli/tests/level2_lifecycle_control.rs` has useful Level 2 coverage for
legacy initialize/failure proxy handoffs, caller `--set` forwarding, cycle
handling, target initialize, and selected error routes. None of those tests
invokes the same target both directly and through a router and compares the
full observable result. There is no motivating
`implement.md -> _implement/implement-plan.md` multi-phase fixture, loop-count
comparison, target-dependent launch/MCP/workspace comparison, inline closure
ownership case, sequence-step containment case, retry/resume overlay case, or
any `proxy.with` case.

This is a verification-level mismatch, not a test-count concern. The spec
explicitly requires Level 2 for the CLI-visible equivalence matrix. Passing
route-specific Level 2 tests cannot prove that direct and routed execution are
observationally equivalent.

**Required change:** add the platform-neutral fake-provider matrix from the
specification and run it through the real tmux/terminal harness. Compare prompt,
effective frontmatter, prepared context/environment, target selection and
launch plan, lifecycle order, loop iterations, closure/output routing, approved
and executed shell bytes, and typed rendered failures. Run the same fixtures on
macOS, Linux, and Windows where platform behavior is involved.

**Verification level:** strongest current verification is Level 2 for isolated
legacy proxy behaviors. The required comparative Level 2 evidence is absent,
so every user-observable equivalence requirement remains a high-severity gap.

## Requirement Verification Levels

| User-observable requirement | Acceptance criteria | Strongest verification present | Assessment |
|---|---:|---|---|
| Direct and proxied targets prepare, initialize, select launch state, loop, and close identically | 1–17 | Level 2 isolated legacy proxy tests | **Gap:** no direct/proxy comparison; implementation retains split orchestration. |
| Key/value `proxy.with` parsing and typed source-time interpolation | 18–21 | None | **Gap:** valid mapping syntax is rejected. Needs Level 1 plus Level 2 CLI execution. |
| Overlay precedence, schema/policy participation, lifetime, and no file/hash mutation | 22–27 | None | **Gap:** no overlay exists. Needs Level 1 invariants plus Level 2 target behavior. |
| Typed failure identity and event-aware failure/finalize routing across entry routes | 28–29 | Level 2 legacy route-specific error cases | **Gap:** errors are stringified and no cross-route identity comparison exists. |
| Redacted status/diagnostics rendered through terminal components | 30 | Level 2 legacy proxy status/error rendering only | **Gap:** no overlay-aware status or `with` diagnostic exists. Needs Level 1 redaction and Level 2 capture. |

Level 3 is not applicable. This feature has no requirement involving physical
keyboard/mouse events, terminal input encoding, paste, or IME behavior.

## Verification Performed

- `just test` from `claudine/`: `claudine-catalog-types` **21 passed**;
  `claudine` **3,423 passed, 7 skipped** (two retry-classified flaky passes);
  `claudine-contract` **47 passed, 5 skipped**. The CLI run was stopped after
  exceeding the non-interactive time ceiling: **431 passed, 1 failed, 157
  skipped, 1,530 not run**. The failure was an existing harness prerequisite:
  `inline_compose_writes_hash_that_passes_md_diff` could not find
  `target/debug/md`.
- `just test-l2`: **130 passed, 1 failed, 1,988 skipped**. All existing proxy
  cases passed. The unrelated failure was
  `level2_stalled_generation_renders_in_tmux`, whose captured Agent Error block
  lacked the expected red SGR after four attempts.
- `just lint`: error-transport and lifecycle-doc guards passed;
  `claudine-catalog-types` and `claudine` completed successfully. The command was
  stopped after exceeding the non-interactive time ceiling while checking later
  package-area crates.
- Static inspection covered the action parser/model, lifecycle transition,
  initialize and terminal proxy routes, reduced harness composer, existing L2
  proxy inventory, feature plan, and required documentation. The lifecycle and
  composition topics and Claudine skill contain no `proxy.with` contract.

The passing baseline tests do not alter the verdict because the production
feature and its required verification matrix are absent.

## Closure Criteria

Complete the plan's phases 1–14, including the dependency gates for shared file
resolution and typed error propagation; map all 30 acceptance criteria to
passing tests; land the Level 2 equivalence matrix; update lifecycle,
composition, and skill documentation; and obtain green `just test`,
`just test-l2`, and `just lint` results before requesting another production
readiness review.
