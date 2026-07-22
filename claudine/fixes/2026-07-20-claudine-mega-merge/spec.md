---
created: 2026-07-21
status: ready for review
reviewed: false
area: claudine
integration_seed: d2d0a8fc4467230ed78e2a1d3146b7c336cc17fd
feature_tips:
    error-prop-and-file-resolution: 43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97
    proxy-with: e348486c810969abe87a6b7209979034f5454b07
rulings:
    - merge error-prop-and-file-resolution before proxy-with
    - retain one request-scoped FileReference resolution model
    - retain one canonical preparation service and one command-owned coordinator
    - require fresh merged-tree acceptance evidence
---

# Claudine Mega-Merge Integration Specification

## Status

Ready for review. This specification defines how to integrate the `claudine`,
`error-prop-and-file-resolution`, and `proxy-with` histories without reducing
the work to a textual conflict exercise.

The merge is architecturally high risk. The reviewed feature tips have 36
direct content conflicts, but the larger risk is that automatically merged or
one-sided files can leave two internally coherent designs connected at the
wrong seams. Success therefore means preserving one combined execution model,
not merely producing a compiling tree without conflict markers.

## Inputs and Revision Boundary

This specification is derived from:

- [`claudine-log.md`](./claudine-log.md)
- [`error-prop-and-file-resolution-log.md`](./error-prop-and-file-resolution-log.md)
- [`proxy-with-log.md`](./proxy-with-log.md)
- [`conflict-report.md`](./conflict-report.md)

The conflict report analyzed these tips:

| Branch | Reviewed tip |
|---|---|
| `claudine` | `dc4cdebde57897516aa836c69d96cb3b9e062186` |
| `error-prop-and-file-resolution` | `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97` |
| `proxy-with` | `e348486c810969abe87a6b7209979034f5454b07` |

The current `claudine` integration seed is
`d2d0a8fc4467230ed78e2a1d3146b7c336cc17fd`. Its only changes after the
reviewed trunk tip are the committed mega-merge research documents and a
GitNexus count refresh. The feature tips and merge bases still match the
report:

- `claudine` ↔ `error-prop-and-file-resolution`:
  `8fc8711434b01327479297af9b40a67685409d00`
- `claudine` ↔ `proxy-with`:
  `6cdb8bf56321c3747d5ea16a1241e47c2bff7fce`

The pre-merge preview MUST be refreshed against the exact integration seed
because the four-input conflict report predates it. If any branch moves again,
the new SHA becomes a reviewed input only after the conflict inventory and
baseline evidence are refreshed.

At authoring time, the integration worktree also contains modified
`.claudine/memory/commits.md` and `CLAUDE.md`, an untracked
`.claude/settings.local.json`, and this new specification. They are user-owned
inputs and MUST be preserved or committed intentionally before Phase 0 declares
the worktree clean.

## Goal

The final `claudine` branch MUST contain all three histories and preserve the
complete behavior of four workstreams:

1. end-to-end typed error propagation;
2. unified, request-scoped file resolution;
3. Sequence Plus;
4. canonical proxy handoffs and `proxy.with`.

The merged design MUST make these workstreams mutually reinforcing:

- typed diagnostics carry file-resolution and handoff provenance;
- `FileReference` resolves direct, proxy, sequence, and nested Darkmatter
  references from one captured request context;
- Sequence Plus executes through the canonical preparation and lifecycle
  model;
- proxy handoffs activate a new document through that same preparation model;
- launch, schema, lifecycle, rendering, and terminal behavior agree across
  direct, proxied, retried, resumed, looped, and sequenced execution.

## Non-Goals

- Rewriting either feature branch from scratch.
- Squashing or cherry-picking the combined 369 unique feature commits.
- Preserving obsolete private path resolvers, reduced preparation paths, or
  optional proxy-return channels for compatibility.
- Broad cleanup, formatting, renaming, or unrelated refactoring during the
  merge.
- Treating cross-compilation, skipped tests, backend setup failure, or
  pre-merge branch evidence as merged runtime acceptance.
- Completing or rescoping lifecycle `defer` or other unrelated planned work.

## Architectural Responsibility Map

The final implementation MUST have one owner for each responsibility.

| Responsibility | Final owner | Required boundary |
|---|---|---|
| Reference grammar, kind classification, candidate planning, and probes | `biscuit-file::FileReference` | No Claudine or Darkmatter syntax rewrite may compete with it. |
| Request-scoped resolution state | `FileResolutionContext` | Captured once per request; child documents derive a context from source provenance. |
| Markdown composition, interpolation, transclusion, and schema mechanics | Darkmatter | Receives explicit context and schema stage; does not recapture ambient process state. |
| Provider-neutral document preparation | Claudine library canonical preparation service | Direct, proxy, retry, resume, and loop entry use explicit entry/stage policy through one service. |
| Active document and handoff transition ownership | Claudine command coordinator | Every surfaced transition is atomically consumed or explicitly rejected. |
| Lifecycle state and diagnostic projection | Claudine library lifecycle protocol | One effective `DiagnosticSnapshot` drives `err.*`, rendering, and transport. |
| Sequence state, task/group semantics, and deterministic merge | Claudine library | The sequence command retains step/task ownership while the active target owns document execution. |
| Provider launch, process execution, and terminal orchestration | `claudine-cli` | One rebuilt launch bundle is both compatibility-keyed and spawned. |
| Terminal presentation | `TerminalRenderable` components | No duplicate or ad hoc error/status emission at lower boundaries. |

Any resolution that leaves two owners in a row above is invalid even if it
compiles.

## Mandatory Merge Invariants

### I1 — Typed diagnostic fidelity

Known errors MUST retain their concrete causes inside the process. Erased,
wire, and persistence boundaries MUST carry a versioned
`DiagnosticSnapshot`. Terminal rendering, lifecycle `err.*`, and serialization
MUST select the same effective diagnostic, structured detail, concise message,
and one registered cause. Exit status, retry policy, and lifecycle routing MUST
not be inferred from rendered strings.

### I2 — One file-reference authority

All production document references MUST be parsed and planned through
`FileReference`:

- bare references: repository root first, then source directory;
- explicit `./` and `../`: source-relative only, with no fallback;
- magic, package, vault, home, URL, recursive, interpolation, and native
  absolute forms: kind-specific behavior from Biscuit File;
- rooted payloads after a magic prefix: rejected on every host.

Resolution failures MUST preserve ordered candidates, probes, reference kind,
and source provenance as typed detail.

### I3 — No ambient recapture in nested execution

CWD, repository, package area, HOME, environment, and configured roots MUST be
captured as explicit request state. A nested document MUST derive its child
context from its own source provenance. It MUST NOT reread ambient process
state or use diagnostic-only `launch_area` metadata as a second execution
anchor.

### I4 — One preparation service

Direct execution, handoff adoption, retry, resume, and loop refresh MUST use
one canonical preparation service with explicit entry reasons and schema
stages. No reduced proxy composer or sequence-only materializer may claim
equivalent behavior outside this service.

### I5 — Atomic coordinator-owned handoff

A proxy result is a typed document transition. The command coordinator MUST be
the only owner allowed to resolve, prepare, validate, and commit it. Every
producer MUST receive a consumed or explicitly rejected result. Failed
handoffs MUST NOT partially activate a target, lose the triggering lifecycle
event, synthesize source completion, or emit closure twice.

### I6 — Sequence containment

A proxy selected inside a sequence step transfers active-document ownership
without restarting, duplicating, or advancing the containing step. Sequence
state, ordered outputs, JIT visibility, task/group identity, preflight graph,
and deterministic state merge remain owned by the sequence invocation. The
adopted target owns its document context, loop, lifecycle, launch, and closure.

### I7 — One coherent launch rebuild

After a handoff, target-dependent provider, profile/binary, model,
interactivity, permission mode, structured output, MCP, credentials, argv,
environment, child CWD, system-prompt delivery, dispatch, closure, and loop
facets MUST be rebuilt as one plan. The session compatibility key MUST be
computed from the plan that is actually spawned. Retry and resume MUST retain
and refresh only their documented facets and budgets.

### I8 — `proxy.with` semantics

`with:` is accepted only on key/value proxy actions. It MUST:

- evaluate once in the source lifecycle context using one captured fallback
  context;
- recursively preserve typed whole values while mixed strings remain strings;
- apply the precedence target frontmatter < overlay < caller overrides;
- remove a target-authored top-level key when the overlay value is null;
- fail atomically;
- survive refreshes of its immediate target but not leak to a downstream hop;
- remain transient, file-neutral, and Darkmatter-hash-neutral;
- redact values from status, diagnostics, and tracing; and
- remain subject to every target-side schema, effect, permission, shell,
  filesystem, network, messaging, and provider policy.

### I9 — Initialize and schema ordering

Darkmatter's deferred schema-verdict support MUST survive. Target bootstrap may
coerce values before `initialize`; the authoritative schema verdict occurs
after the stabilized reread. Direct, initialize-proxy, recovery-proxy, retry,
resume, and looping paths MUST apply the same selected stage policy.

### I10 — Exactly-once terminal behavior

Typed diagnostics, lifecycle status, and Sequence Plus task frames MUST retain
their stdout/stderr ownership and ordering. No boundary may render the same
failure twice. `NO_COLOR`, non-TTY output, Unicode fallback, OSC 8 links,
concurrent attribution, idle flush, and close behavior MUST remain
informationally equivalent.

### I11 — Trunk work survives

The merge MUST preserve the trunk's transient system-prompt `.gitignore` fix,
lifecycle-schema scaffolding, local-runner schemas/research, shared prompt
changes, and topic documentation. Generated counts or branch-local prose are
not allowed to overwrite the final merged architecture.

### I12 — Cross-platform contract

Production code MUST compile on macOS, Linux, and Windows. Behavior involving
native paths, Windows named pipes, console control, process trees, keyboard
injection, or real terminals requires the evidence level stated in the
acceptance ledger. A Windows cross-check is useful but is not native Windows
runtime evidence.

## Risk Register

| ID | Risk and likely failure | Impact | Required control | Detection / exit evidence |
|---|---|---|---|---|
| R1 | Two preparation designs are combined line by line, leaving route-dependent state. | Critical | Adopt one stage-aware preparation service carrying `FileResolutionContext`. | Direct/proxy/retry/resume/loop equivalence tests and responsibility audit. |
| R2 | Proxy handoff failures flatten diagnostics or route closure through the wrong source/target. | Critical | Resolve lifecycle/transition conflicts as a state machine; retain snapshots through erased boundaries. | Combined handoff-failure matrix proving diagnostic identity, event retention, and exactly-once closure. |
| R3 | Sequence Plus and proxy both own step advancement or active-document closure. | Critical | Keep sequence invocation state outside the coordinator-owned active document. | Proxy-in-step, proxy-in-group, looped-target, teardown, and output-order seam tests. |
| R4 | Compatibility key and spawned launch are assembled from different state. | Critical | Make the complete launch bundle the sole source of both key and spawn inputs. | Per-facet launch tests plus provider/model/MCP/credential retry-resume cases. |
| R5 | Nested documents recapture ambient CWD/HOME/repository state. | High | Thread and derive the explicit request snapshot at every adapter. | Source/repository re-anchoring and single-capture tests on direct, proxy, sequence, and Darkmatter routes. |
| R6 | Overlay typing, precedence, null removal, or schema timing changes during conflict resolution. | High | Preserve immutable pre-schema overlay and deferred verdict ordering. | Typed recursive values, all precedence combinations, null deletion, immediate refresh, downstream-hop isolation, and schema-order tests. |
| R7 | Auto-merged files hide semantic incompatibility. | High | Review all files changed by both branches plus listed one-sided responsibility modules. | Base/ours/theirs/caller/test review recorded in conflict checklist. |
| R8 | Error/status output is duplicated or reordered under concurrent tasks. | High | Preserve one effective selector and `TerminalRenderable`/task-stream ownership. | Plain/color/OSC 8 and parallel task failure captures at L2. |
| R9 | Generated inventories, fixtures, skills, or docs describe one branch rather than merged source. | High | Settle behavior first; regenerate from merged tree; reconcile prose last. | Generator/drift/source-scan guards and manual grammar/stage-matrix review. |
| R10 | L2 infrastructure fails before assertions and is counted as acceptance. | High | Require fail-closed L2 execution through `just test-l2`; record backend failures separately. | Test logs prove the expected rows reached assertions. |
| R11 | Windows behavior is inferred from macOS or cross-compilation. | High | Schedule native Windows runtime/terminal evidence for relevant paths. | Native CI or attended run artifacts, plus `just check-windows` as a separate compile gate. |
| R12 | Supporting-package changes are lost because review focuses only on `claudine/`. | High | Gate Biscuit File, Darkmatter, Biscuit Test Harness, and Rendezvous independently. | Package-area tests/lints and affected-caller audit. |
| R13 | Dirty or untracked integration inputs are overwritten or omitted. | High | Preserve changes before merge; start from a clean integration worktree; never destructively reset. | Clean `git status --short --branch` and recoverable copy/commit of this directory. |
| R14 | Branch refs move after analysis. | High | Freeze exact SHAs and repeat merge-tree preview if any ref changes. | Recorded SHA ledger and refreshed conflict manifest. |
| R15 | Broad `ours`/`theirs` selection deletes a valid architecture. | Critical | Resolve by responsibility and invariant, never by branch preference alone. | Per-conflict rationale and post-resolution responsibility audit. |

## Conflict Review Scope

The 36 predicted feature-branch content conflicts are the minimum review set,
grouped below.

| Cluster | Conflicted paths / surfaces | Resolution owner |
|---|---|---|
| Architecture and records | `.claude/skills/claudine/{SKILL,architecture}.md`, `.claudine/memory/commits.md`, `CLAUDE.md` | Reconcile after code; keep history factual and final architecture authoritative. |
| CLI preparation and composition | `compose/prep.rs`, `wrap/composition/{mod,pipeline,runner}.rs`, `wrapper_stages.rs` | Canonical preparation plus explicit request context. |
| Harness and proxy routing | `loop_control.rs`, `control_dispatch.rs`, `proxy.rs`, `prompt.rs`, `types.rs`, `overlay.rs`, related tests | Coordinator/transition state machine plus typed diagnostics. |
| Sequence integration | `wrap/sequence/{iterate,mod,phase1c}.rs` and requeue/proxy tests | Sequence invocation ownership surrounding active-document execution. |
| Library composition/lifecycle | `composition/error`, lifecycle `context`/`executor`, looping `engine`, `composition/{mod,preflight,prepare,types}.rs` | Provider-neutral combined contract. |
| Darkmatter | `markdown/compose/context/options.rs` | Explicit resolution context, name coercion, and deferred schema verdict in one option model. |
| Generated/docs | dispatch inventory, `claudine-gen` drift fixture, composition topic | Regenerate or rewrite from settled merged source. |

The following automatically merged or one-sided areas MUST also be reviewed:

- composition `target.rs`, launch adapters, environment sanitation, attempt
  classification, wrapper entry points, system-prompt lifetime, and session
  reporting;
- Darkmatter schema validation, transclusion, reference graph, expression, and
  schema-resolution paths;
- new `composition::coordinator`, preparation-stage, diagnostic registry,
  restored-diagnostic, Sequence Plus task/group, task-stream, and launch-plan
  modules;
- completion adapters and all remaining private `@/` or relative-path rewrite
  candidates;
- test harness backends, nextest configuration, CI filters, and fail-closed
  tier behavior;
- the Rendezvous Windows `Connected` adapter and dependency gating.

## Merge Strategy

### Phase 0 — Freeze, protect, and baseline

1. Freeze the exact integration seed and both feature-tip SHAs. Stop feature
   work on those refs until integration completes.
2. Preserve all modified and untracked files, including this merge directory,
   before starting. The actual integration worktree MUST be clean.
3. Create a recoverable integration branch from the frozen `claudine` seed.
   Keep all three source branches unchanged and retain their ancestry through
   merge commits.
4. Refresh non-mutating merge previews against the frozen SHAs and export the
   exact conflict list into a checklist.
5. Record independent baseline results at both feature tips. Red, timed-out,
   skipped, and backend-blocked results remain labeled as such.
6. Build the acceptance ledger before editing. Import every criterion from the
   four feature specs, the Sequence Plus validation matrix, and the proxy-with
   acceptance map. Each row needs a merged-code test, tier, platform, owner,
   status, and evidence location.
7. Confirm the GitNexus index is current. Before editing a conflicted symbol,
   run upstream impact analysis and record direct callers, affected processes,
   and risk. Stop and warn before editing a HIGH or CRITICAL symbol until the
   resolution is reviewed.
8. Enable `git rerere` if repeated attempts are expected. Reused resolutions
   still require review against this specification.

Phase 0 exits only when the inputs are recoverable, refs are frozen, baselines
are recorded, and the acceptance ledger exists.

### Phase 1 — Integrate the foundation branch

Merge `error-prop-and-file-resolution` into the integration branch using a
non-fast-forward, no-commit merge.

This branch is first because its internal dependency order is already:

```text
typed diagnostics → FileReference/request context → Sequence Plus
```

Resolve the predicted trunk conflicts in repository records and shared
prompts. Manually audit the auto-merged composition pipeline, wrapper stages,
composition/system-prompt topics, and the trunk `.gitignore` fix.

Before recording the first merge commit:

- prove the diagnostic registry, snapshots/restoration, and source/transport
  guards;
- prove Biscuit File grammar, precedence, candidate detail, context derivation,
  and completion round trips;
- prove Darkmatter composition, schema, reference, and transclusion paths;
- prove Sequence Plus retained behavior, JIT state, tasks/groups, deterministic
  merges, and process ownership at their applicable tiers;
- run supporting-package and Claudine L1/lint gates; and
- inspect staged changes and scan both worktree and staged blobs for conflict
  markers.

Phase 1 exits with a known-good foundation merge commit. Failures originating
here MUST be resolved before proxy-with is introduced so later regressions
remain attributable.

### Phase 2 — Integrate proxy-with by dependency order

Merge `proxy-with` with `--no-commit`. Resolve conflicts in this order:

1. shared types, diagnostic shapes, and transition errors;
2. Darkmatter options and the schema-stage contract;
3. canonical preparation and request/file-resolution context;
4. coordinator, lifecycle protocol, loop engine, and transition ownership;
5. CLI preparation, launch bundle, harness orchestration, and retry/resume;
6. Sequence Plus containment and task/group integration;
7. terminal rendering, task framing, and error emission;
8. tests, generated inventories, skills, docs, CI, and repository guards.

The proxy branch supplies the command-owned coordinator and complete
target-launch rebuild. The foundation supplies typed diagnostics,
`FileReference`, request-scoped context, Sequence Plus, and task-stream
behavior. The resolution MUST combine those responsibilities rather than
selecting one complete file side.

For each conflict:

1. inspect the merge base and both branch versions;
2. identify the final responsibility owner from this specification;
3. inspect callers, callees, and affected execution flows;
4. identify existing acceptance tests from both branches;
5. resolve the smallest coherent behavior unit;
6. run its focused tests before moving to the next cluster;
7. record the rationale and evidence in the conflict checklist.

Phase 2 exits only when there is one preparation service, one transition path,
one resolution grammar, one diagnostic selector, one launch bundle, and no
conflict markers in either unstaged or staged content.

### Phase 3 — Regenerate and reconcile derived material

After runtime behavior is settled:

1. regenerate or intentionally refresh dispatch inventories and generated
   drift fixtures with their owning tools;
2. run source-scan, generated-artifact, dispatch, and test-placement guards;
3. reconcile the Claudine skill and architecture docs against the merged
   responsibility map;
4. reconcile composition, lifecycle, file-reference, schema, Sequence Plus,
   system-prompt, completion, and error-architecture documentation;
5. preserve trunk lifecycle schemas and local-runner research;
6. correct the stale proxy-with acceptance-map description of the Linux L2
   workflow; and
7. verify that comments describe the merged behavior, deleting or correcting
   drift while treating the code as authoritative unless an acceptance
   invariant proves the code wrong.

Do not copy branch-local line-number inventories, hashes, or status claims into
the merged tree unchanged.

### Phase 4 — Layered merged-tree verification

Run focused tests after each subsystem, then complete area gates. Canonical
commands include:

```sh
cd biscuit-file
just test
just test-l2
just lint

cd ../darkmatter
just test
just test-l2
just lint

cd ../biscuit-test-harness
just test
just lint

cd ../claudine/rendezvous
just check
just test
just lint

cd ..
just test
just test-l2 --no-fail-fast
just lint
just check-windows
```

All unit and integration tests use nextest through the `just` recipes. Do not
substitute `cargo test`. Real-terminal tests run only through `just test-l2`,
which owns terminal resources and fail-closed tier configuration. Do not run
`cargo fmt` or `rustfmt` in write mode.

Focus-stealing L3 keyboard tests run only through guarded, attended/native or
designated CI workflows. Their opt-in and platform requirements MUST remain
visible in the ledger.

### Phase 5 — Final audit and closeout

1. Run GitNexus change detection against `main` and review unexpected affected
   symbols, execution flows, deleted guards, or duplicate paths before each
   merge commit.
2. Review the final diff for unintended production, test, generated, and
   documentation changes.
3. Verify generators and drift/source-scan guards from a clean merged checkout.
4. Repeat full L1, L2, and lint gates from that checkout.
5. Attach native macOS, Linux, and Windows evidence at the level required by
   each ledger row.
6. Update all four feature records with the final merged SHA and fresh results.
7. Move feature/fix records to `_completed` only after all required evidence is
   green or the owner explicitly accepts a precisely documented residual debt.

## Acceptance Assurance

### Workstream matrix

| Workstream | Required merged contracts | Minimum fresh evidence |
|---|---|---|
| Error propagation | No typed in-process flattening; complete registry; renderer/`err.*`/snapshot parity; registered cause; lossless excerpts and resolution detail; exactly-once output; unchanged decisions | Registry, chain, facets, snapshots/restoration, source scans, boxed/transport guards at L1; direct/proxy/terminal and color/plain/OSC 8 captures at L2; Claudine test/lint gates |
| File resolution | Sole `FileReference` grammar; repository-first bare and strict explicit-relative behavior; ordered probes; one request snapshot; child re-anchoring; completion/execution and cross-surface parity | Biscuit File grammar/context/resolution/completion; Darkmatter composition/reference/schema/transclusion; Claudine direct/proxy/sequence L1 and L2; caller audit; Unix and Windows path cases |
| Sequence Plus | Retained behavior; typed invalid cases; source/list grammar; JIT visibility; exact shell bytes; serial/parallel ordering; deterministic merge; teardown; task attribution; process-tree interruption | Re-anchored validation matrix; Claudine/Biscuit File/Darkmatter L1; task-stream L2; guarded L3 interruption evidence; compile and native Windows evidence; lints |
| Proxy-with | Canonical preparation; atomic coordinator transitions; direct/proxy equivalence; target-owned state; launch rebuild; retry/resume compatibility; deferred schema verdict; typed/redacted failures; complete overlay semantics | All 30 criteria mapped to merged tests; coordinator/overlay/launch/session L1; full real-CLI proxy and diagnostic matrices at L2; Linux L2 reaches assertions; Claudine gates |
| Cross-feature seams | Shared request context, snapshot-preserving handoffs, proxy containment in sequence, non-leaking overlays/state, shared schema/preflight order, keyed plan equals spawned plan | Combined seam suite below; isolated feature suites are insufficient |

### Mandatory combined seam cases

The merged suite MUST prove at least these interactions:

1. A bare proxy target resolves repository-first and a missing target exposes
   the ordered candidate/probe diagnostic identically in terminal, `err.*`, and
   snapshot output.
2. An explicit relative proxy target remains source-local without fallback;
   its nested references derive from the target's source and repository.
3. A `proxy.with` overlay enters a Sequence Plus step whose target owns a loop;
   the step runs once, the loop completes, and JIT state/output remain visible.
4. A proxied target's typed schema failure matches direct execution while the
   source and target each receive exactly the lifecycle events they are owed.
5. Initialize-time and terminal-time handoff failures retain their distinct
   triggering events, closure ownership, and diagnostic identity.
6. Overlay values survive retry, resume, and immediate-target loop refresh but
   are absent from a downstream proxy unless explicitly supplied again.
7. Provider/model/MCP/credential changes across retry or resume rebuild the
   launch plan and produce typed incompatibility when a live session cannot be
   reused.
8. The session compatibility key is derived from the exact launch bundle that
   is spawned.
9. Sequence/task preflight approves the exact shell bytes later executed after
   target handoff and context re-anchoring.
10. A failure from a proxied target inside a parallel group remains attributed
    to the correct task, preserves stdout/stderr ordering, settles all children,
    merges state deterministically, and tears down process descendants.

### Evidence classification

Every acceptance-ledger row MUST use one of these statuses:

- `passed`: the merged test reached and passed its intended assertion;
- `failed`: a merged product assertion failed;
- `blocked`: infrastructure prevented the assertion from running;
- `skipped`: the test intentionally did not run under its tier contract;
- `compile-only`: the target compiled but received no runtime proof;
- `accepted-debt`: the owner explicitly accepted a named residual gap.

Only `passed` satisfies a required evidence row. `accepted-debt` may allow a
code-integration commit to proceed only when the owner records scope,
consequence, mitigation, and follow-up. It does not satisfy a required row or
the completion definition. The other statuses never become implicit passes.

The ledger SHOULD use this shape:

| Criterion ID | Contract | Test or audit | Tier | Platform | Status | Merged SHA | Evidence |
|---|---|---|---|---|---|---|---|
| Example | Direct/proxy schema parity | named test/matrix row | L2 | Linux | blocked | `<sha>` | tmux setup failed before assertion |

## Platform Evidence Matrix

| Evidence | macOS | Linux | Windows |
|---|---|---|---|
| L1 library/CLI behavior | Required | Required in CI | Required in CI where supported |
| L2 real-terminal behavior | Required on an available backend | Dedicated tmux-backed job MUST reach assertions | Required for Windows-specific terminal claims |
| L3 keyboard/process interruption | Guarded attended run where macOS behavior is claimed | Guarded native CI/run | Guarded native CI/run for console-control and descendant termination |
| Cross-compile/type-check | Useful | Useful | `just check-windows` required, but classified `compile-only` |
| File path/runtime semantics | Native Unix cases | Native Unix cases | Native drive, UNC/named-pipe, HOME, and console cases |

Two evidence gaps are known before integration:

- proxy-with review 18 did not obtain current L2 product evidence because the
  managed macOS environment denied tmux setup before assertions;
- Sequence Plus still requires the recorded native Windows runtime and some L3
  keyboard-interruption evidence.

These gaps do not forbid starting the merge. They do forbid declaring the
merged result fully accepted until discharged or explicitly accepted as debt.

## Stop Conditions and Recovery

Pause the merge and return to the last recoverable checkpoint if any of these
conditions occurs:

- branch tips no longer match the frozen SHA ledger;
- impact analysis reports HIGH or CRITICAL risk without a reviewed resolution;
- a conflict cannot be assigned to one owner in the responsibility map;
- two preparation, transition, resolution, diagnostic-selection, or launch
  paths remain after resolution;
- typed detail must be converted to text to cross a new in-process boundary;
- a proxy-in-sequence path cannot identify one owner for step advancement and
  one for document closure;
- generated/drift guards can pass only by weakening their scope;
- L2 or L3 infrastructure repeatedly fails before assertions and no compliant
  alternate backend or CI route is available; or
- unrelated dirty work prevents a reliable diff or abort.

Before any merge abort or retry, preserve the current conflict-resolution patch
and ledger. Use recoverable Git operations only; never use a destructive reset
against a broad path or dirty worktree.

## Completion Definition

The mega-merge is complete only when:

- `claudine` contains both feature histories through reviewed merge commits;
- all mandatory invariants and the responsibility map hold in the final source;
- no duplicate legacy execution path or private resolution grammar remains;
- every conflict and semantic hotspot has a recorded resolution rationale;
- all four feature specifications are mapped to fresh merged-code evidence;
- every required L1 and L2 test reached its assertion and passed;
- required L3 and native Windows evidence is attached and passed;
- package-area lints, drift guards, generators, source scans, and test-placement
  checks pass;
- documentation and skills describe the merged behavior; and
- a clean-checkout final audit finds no conflict markers, stale generated data,
  unexpected changes, or unowned acceptance rows.

Compilation alone, a clean textual merge, skipped tests, backend denial, or
green pre-merge branch results do not satisfy this definition.
