# Claudine Mega-Merge Conflict Report

## Executive assessment

This is a high-risk semantic merge even though the `claudine` trunk has little competing production work. The trunk diverged from `proxy-with` at `6cdb8bf` and from `error-prop-and-file-resolution` at `8fc8711`. At the reviewed tips, the branches are:

- `claudine`: `dc4cdebde57897516aa836c69d96cb3b9e062186`
- `error-prop-and-file-resolution`: `43c23c6535cf6e52a35dbb06ea6f4ccce0c88e97`
- `proxy-with`: `e348486c810969abe87a6b7209979034f5454b07`

Non-mutating `git merge-tree` previews found only three textual conflicts between `claudine` and `error-prop-and-file-resolution`, and one between `claudine` and `proxy-with`. In contrast, the two feature branches have 36 direct content conflicts. Most occur where both branches independently redesigned the same composition, lifecycle, loop, sequence, and wrapper orchestration seams.

The safe integration model is therefore not “choose ours or theirs.” The merged architecture must preserve both sets of invariants:

1. Typed diagnostics and one `FileReference`-owned, request-scoped resolution model are foundational.
2. Sequence Plus must execute through that foundation.
3. Proxy handoffs must be command-owned document transitions that re-enter the same canonical preparation service as direct execution.
4. All three paths must share lifecycle, schema, launch, error, and terminal-rendering behavior.

The recommended order is to merge `error-prop-and-file-resolution` into `claudine` first, validate that foundation, and then merge `proxy-with` while deliberately adapting its coordinator and canonical-preparation design to the already-integrated diagnostic, resolution, and Sequence Plus contracts.

The merge must not be declared complete merely because Git conflicts are resolved and L1 is green. `proxy-with` still lacks current L2 acceptance evidence because its last runs were blocked before feature assertions by the managed host's tmux restrictions. Sequence Plus also records native Windows runtime and some Level 3 keyboard-interruption evidence as outstanding. Those evidence gaps must remain explicit until discharged; a compile-only Windows check is not runtime proof.

## Evidence and pre-merge observations

This report is based on the three branch logs in this directory, the feature specifications and acceptance records at the branch tips, and non-mutating merge previews.

The current `claudine` worktree is not clean. At report time it contains a modified `CLAUDE.md` and untracked `.claude/settings.local.json`, `claudine/fixes/2026-07-20-claudine-mega-merge/`, and `darkmatter/fixes/2026-07-20-mega-merge/`. These changes must be preserved in a commit or otherwise backed up before starting the merge. Do not use a destructive reset, and do not assume the untracked report/spec directories can be recreated.

The `claudine` trunk's only production Rust change during the split is the transient system-prompt `.gitignore` correction in the wrapper composition pipeline. That fix must survive integration. Its lifecycle-schema scaffolding, local-runner schema/docs, prompt changes, and topic documentation also need intentional reconciliation, but they do not establish an alternate implementation of the four features.

## Key conflict areas

### 1. Canonical preparation and file-resolution context

Both feature branches refactor document preparation, but for different reasons:

- File resolution threads a captured `FileResolutionContext` through preparation, preflight, loops, sequence loading, lifecycle lookup, system-prompt resolution, Darkmatter composition, and nested document re-anchoring. It makes `biscuit_file::FileReference` the only grammar and candidate-planning authority.
- Proxy-with introduces entry reasons, schema stages, a canonical preparation service, active-document state, and coordinator-owned handoff adoption so direct, proxy, retry, resume, and loop refreshes do not use divergent preparation paths.

The central merge invariant is one preparation service with both properties. A proxy target must be resolved using the shared `FileReference` resolver, derive a child request context from the target's source provenance, receive its overlay and caller overrides at the specified precedence, and then pass through the correct proxy entry/schema stage. It must not restore the old in-harness path substitution, reread ambient CWD/HOME/repository state, or bypass candidate/probe diagnostics.

The highest-risk files in this area are:

- `claudine/cli/src/commands/compose/prep.rs`
- `claudine/cli/src/commands/wrap/composition/{mod,pipeline,runner}.rs`
- `claudine/lib/src/composition/{mod,prepare,preflight,types}.rs`
- `darkmatter/lib/src/markdown/compose/context/options.rs`

The existing auto-merge of nearby files is not evidence of semantic compatibility. In particular, automatically merged `target.rs`, composition tests, Darkmatter schema validation, and file-resolution adapters must be reviewed against the final preparation-stage matrix.

### 2. Lifecycle diagnostics versus typed proxy transitions

Error propagation changes lifecycle `err.*` from string-derived state to the effective `DiagnosticSnapshot`; proxy-with changes proxy results from optional path-return channels to a typed transition consumed or explicitly rejected by the command coordinator. These changes meet at every failed handoff and recovery route.

The merged behavior must ensure that:

- resolution, overlay, schema, cycle, hop-limit, session-compatibility, and unowned-handoff failures retain their concrete diagnostic identity and source chain;
- the same effective diagnostic drives terminal rendering, lifecycle `err.*`, and serialized snapshots;
- initialize-time failures route through the source's owed `blocked`/`finalize` events, while terminal-time failures retain the triggering event and owed closure;
- no failed transition partially activates the target or emits source completion twice; and
- restored or erased diagnostics do not fall back to string parsing at the coordinator boundary.

The most sensitive files are `composition/error`, `composition/lifecycle/{context,executor}`, the CLI loop-control dispatcher/proxy modules, and the prompt/orchestration types. These conflicts must be resolved as state-machine behavior, not line-by-line text.

### 3. Sequence Plus inside the active-document coordinator

Sequence Plus substantially changes the same CLI sequence and harness modules that proxy-with changes to keep a proxy contained within a sequence step. The conflict covers JIT preparation, runtime state, preflight graphs, task/group execution, lifecycle setup/teardown, output attribution, interruption, and step advancement.

The merged contract must preserve all of the following:

- A proxy within a sequence step runs the target under the coordinator but does not restart, duplicate, or advance the containing step.
- JIT composition still sees prior output and invocation-local `set` mutations.
- Task/group preflight uses the same resolved bytes that execution uses.
- The adopted target owns its loop and lifecycle closure, while the sequence command retains step/task state and ordered output ownership.
- Parallel groups retain bounded concurrency, snapshot isolation, all-child settlement, declaration-order state merging, attributed terminal frames, and process-tree interruption.
- Proxy overlays survive refreshes of their immediate target but do not leak into a downstream proxy hop.

The direct conflicts in `wrap/sequence/{iterate,mod,phase1c}.rs` and adjacent harness orchestration are only the visible portion. The new Sequence Plus modules added on one branch and the new coordinator/launch modules added on the other must be reviewed together for duplicate responsibilities and stale adapters.

### 4. Launch-plan rebuild, retry/resume, and session compatibility

Proxy-with rebuilds provider, profile/binary, model, interactivity, permission mode, structured-output mode, MCP tags/injection, credentials, argv, environment, child CWD, system-prompt delivery, dispatch, closure, and loop decisions after a handoff. Error propagation and Sequence Plus also change harness attempt classification, process termination, and orchestration errors.

The proxy branch's complete launch rebuild should remain the architectural owner of target-dependent launch state. It must, however, consume the merged prepared document and request-scoped resolution state. No resolution metadata such as `launch_area` may be allowed to become a second execution anchor. Retry and resume must refresh exactly the documented facets, maintain document-scoped versus invocation-scoped budgets, and reject incompatible live sessions with a typed diagnostic that names the changed facets.

Automatically merged environment, target, attempt, and wrapper files require explicit review. A clean textual merge can still leave the coordinator computing a compatibility key from one plan while spawning another.

### 5. `proxy.with`, whole-value typing, and deferred schema verdicts

The overlay is evaluated once in the source lifecycle context, but applied to the target before its canonical preparation. This overlaps with Darkmatter context changes from both branches and with Sequence Plus runtime state/coercion.

Required final ordering is:

1. target-authored frontmatter;
2. shallow `with:` overlay, with null removing a top-level target key; and
3. caller `key=value`/`--set` overrides.

Whole-value expressions must retain booleans, numbers, nulls, arrays, and objects. Mixed strings remain strings. Overlay evaluation must use one captured fallback context, fail atomically, remain transient and hash/file neutral, redact values from diagnostics/status/tracing, and remain subject to target-side schema, effect, permission, shell, filesystem, network, messaging, and provider policies.

The deferred schema-verdict support in Darkmatter must be retained: target bootstrap may coerce values before `initialize`, but the authoritative verdict occurs after the stabilized reread. The merge must prove this ordering for direct, initialize-proxy, recovery-proxy, and looping targets.

### 6. Typed rendering and concurrent terminal output

Error propagation establishes exactly-once `TerminalRenderable` diagnostics with plain/no-color parity. Proxy-with adds typed lifecycle/handoff renderers and redacted transition status. Sequence Plus adds synchronized task-stream frames across stdout/stderr.

The merge must not reintroduce ad hoc `println!`/`eprintln!` paths, duplicate an error at both a lower harness and command boundary, or allow task framing to reorder diagnostics and lifecycle status. The effective diagnostic selector must remain shared by terminal, lifecycle, and serialization paths. OSC 8, Unicode fallback, `NO_COLOR`, non-TTY output, concurrent task attribution, and flush/close ownership need real-terminal verification after integration.

### 7. Generated inventories, drift guards, skills, and documentation

Both branches update generated dispatch inventory data, `claudine-gen` drift fixtures, the Claudine skill, architecture docs, composition/lifecycle topics, `CLAUDE.md`, and memory files. These should not be resolved by mechanically concatenating both sides.

- Resolve source and architecture first, then regenerate or intentionally refresh derived inventories and hashes with their owning tools.
- Rebuild test/drift baselines from the merged tree; do not copy a branch's line-number inventory unchanged.
- Reconcile documentation against the final grammar and stage matrix, especially file-reference precedence, proxy handoff ownership, `proxy.with`, Sequence Plus grammar, and the Linux L2 workflow description called stale by proxy review 18.
- Preserve the trunk's lifecycle schema scaffolding and local-runner research changes.
- Preserve the trunk's transient system-prompt `.gitignore` correction while adapting it to the merged pipeline.

### 8. Textual conflict inventory

The feature-branch preview reports 36 content conflicts. They group as follows.

#### Architecture and repository records

- `.claude/skills/claudine/SKILL.md`
- `.claude/skills/claudine/architecture.md`
- `.claudine/memory/commits.md`
- `CLAUDE.md`

#### CLI preparation, orchestration, and sequence

- `claudine/cli/src/commands/compose/prep.rs`
- `claudine/cli/src/commands/wrap/composition/mod.rs`
- `claudine/cli/src/commands/wrap/composition/pipeline.rs`
- `claudine/cli/src/commands/wrap/composition/runner.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/proxy.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/mod.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs`
- `claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/requeue.rs`
- `claudine/cli/src/commands/wrap/harness_orch/prompt.rs`
- `claudine/cli/src/commands/wrap/harness_orch/types.rs`
- `claudine/cli/src/commands/wrap/overlay.rs`
- `claudine/cli/src/commands/wrap/sequence/iterate.rs`
- `claudine/cli/src/commands/wrap/sequence/mod.rs`
- `claudine/cli/src/commands/wrap/sequence/phase1c.rs`
- `claudine/cli/src/commands/wrap/wrapper_stages.rs`

#### Claudine library composition and lifecycle

- `claudine/lib/src/composition/error/mod.rs`
- `claudine/lib/src/composition/error/render/mod.rs`
- `claudine/lib/src/composition/error/tests.rs`
- `claudine/lib/src/composition/lifecycle/context.rs`
- `claudine/lib/src/composition/lifecycle/executor.rs`
- `claudine/lib/src/composition/lifecycle/executor/tests/mod.rs`
- `claudine/lib/src/composition/looping/engine.rs`
- `claudine/lib/src/composition/mod.rs`
- `claudine/lib/src/composition/preflight.rs`
- `claudine/lib/src/composition/prepare.rs`
- `claudine/lib/src/composition/types.rs`

#### Generated data, documentation, and Darkmatter

- `claudine/docs/providers/dispatch-inventory.json`
- `claudine/docs/topics/composition.md`
- `claudine/gen/tests/drift.rs`
- `darkmatter/lib/src/markdown/compose/context/options.rs`

This list is a minimum review set. Files changed by both branches that Git auto-merges, and new modules added by only one branch, can still be semantically incompatible.

## Functionality outside the Claudine package area

“Outside Claudine” here means paths outside `claudine/`, including supporting packages and repository infrastructure.

### Biscuit File

The file-resolution branch makes substantial public-library changes rather than adding a Claudine-only adapter:

- host-independent `FileReferenceKind` and effective-kind classification;
- an explicit, derivable resolution context capturing CWD, repository, package area, HOME, environment, and configured roots;
- deterministic candidate planning and typed probe/provenance results;
- repository-first implicit-relative precedence while explicit `./` and `../` stay source-relative;
- cross-platform HOME and Windows/Unix absolute-path handling;
- shared completion behavior and execution/completion round-trip tests;
- rejection of rooted payloads after a magic `@` prefix; and
- `ListFormat`/`classify_list` for Sequence Plus data sources.

These are workspace-level behavior changes. Every affected caller must be audited; passing Claudine tests alone is insufficient.

### Darkmatter

Darkmatter adopts the shared request-scoped file-resolution model throughout Markdown composition, expressions, links, transclusion, reference graphs, and schema detect/format/resolve/rewrite/validate paths. It preserves child source provenance and routes missing-target/transclusion resolution through parsed `FileReference` candidate plans.

Sequence Plus also adds Darkmatter state/effect support, including `set(key, value)`, `last()` coverage, and opt-in name coercion. Proxy-with adds deferred schema-verdict support to `ComposeOptions` and schema validation so target initialization can precede the final verdict without losing early coercion.

The direct Darkmatter conflict is in compose context options, but schema validation and many auto-merged resolution surfaces need joint semantic review.

### Biscuit Test Harness

The branch adds or changes real-terminal support used by the acceptance suites:

- macOS modified-key injection and focus handling;
- Linux X11 `xdotool` injection;
- Windows keyboard injection;
- WezTerm GUI reachability detection; and
- shared helpers/documentation for L2/L3 terminal assertions, including OSC 8 behavior.

These changes are functional test infrastructure, not incidental fixtures. If they are lost, platform-dependent tests can skip, fail before assertions, or provide false confidence.

### Rendezvous daemon

Sequence Plus cross-platform work includes a narrow Windows named-pipe `tonic::transport::server::Connected` adapter and dependency gating in the rendezvous daemon. It exists to keep Windows compilation/test targets valid. It should be retained and verified through the rendezvous area gates and native Windows CI where available.

### Repository infrastructure and shared assets

The branches also change:

- `.config/nextest.toml` timeout/test profiles;
- `.github/workflows/claudine-tests.yml`, adding a dedicated Linux tmux-backed L2 job;
- shared implementation/review prompts;
- repository error-transport and lifecycle-doc guard scripts;
- shared `just` support;
- root and local agent guidance/memory; and
- package dependency documentation.

These changes affect whether regressions are detected. Guard scripts, test filters, skip/fail-closed policy, and CI wiring must be reviewed as part of the product merge.

## Recommended merge procedure

### Phase 0: freeze and protect the inputs

1. Record the three exact tip SHAs above and stop feature work on the fork branches during integration.
2. Preserve every current dirty/untracked worktree change before merging. Confirm `git status --short --branch` is clean in the actual integration worktree.
3. Create a recoverable integration branch from `claudine`; keep the original three branches unchanged.
4. Capture baseline gate results independently at both feature tips. A red or backend-blocked baseline is recorded as debt, not silently waived.
5. Build one acceptance ledger from the four specs, Sequence Plus validation matrix, and proxy-with acceptance map. Each criterion needs merged-code evidence, a responsible test, platform, and final status.
6. Confirm the GitNexus index is current. Before editing conflicted symbols, run upstream impact analysis and record callers/processes/risk. Warn before proceeding with any HIGH or CRITICAL result. Run change detection against `main` before each merge commit.

### Phase 1: merge the foundation branch

Merge `error-prop-and-file-resolution` into the integration branch with a non-fast-forward, no-commit merge so conflicts and the staged result can be reviewed before committing.

Resolve the three predicted trunk conflicts in `.claudine/memory/commits.md`, `CLAUDE.md`, and `prompts/_implement/implement-suggestions.md`. Review the auto-merged composition pipeline, wrapper stages, composition/system-prompt topics, and trunk `.gitignore` fix manually.

Before moving on, run the Biscuit File, Darkmatter, Biscuit Test Harness as applicable, Rendezvous, and Claudine L1/lint gates. Run focused file-resolution, diagnostic, Sequence Plus, and completion suites. This produces a known-good foundation and keeps later failures attributable.

### Phase 2: merge proxy-with by architectural responsibility

Merge `proxy-with` with `--no-commit`. Resolve the 36 conflicts in dependency order:

1. shared data types and error/diagnostic shapes;
2. Darkmatter compose options and schema-stage contract;
3. library preparation service and request/file-resolution context;
4. coordinator, lifecycle executor, loop engine, and transition ownership;
5. CLI preparation, launch-plan rebuild, harness orchestration, and retry/resume;
6. Sequence containment and task/group integration;
7. terminal rendering and error emission;
8. tests, generated inventories, skill/docs, and repository guards.

Use the proxy branch's command-owned coordinator and complete launch-rebuild model as the routing backbone. Integrate the foundation branch's typed diagnostic, `FileReference`, request-context, Sequence Plus, and task-stream behavior into that backbone. Do not preserve two preparation services, two proxy-return paths, two path grammars, or two diagnostic selectors.

For every conflicted file, inspect base, both branch versions, callers, and acceptance tests. Avoid broad `--ours` or `--theirs` resolutions. After resolving all markers, scan both the working tree and staged blobs for conflict markers before committing.

### Phase 3: regenerate and reconcile derived material

Regenerate or refresh dispatch inventories and drift fixtures from the merged source. Reconcile docs and skills only after behavior is settled. Correct the stale proxy acceptance-map statement about the Linux L2 job. Verify documentation examples against the final Sequence Plus grammar, file-reference precedence, lifecycle routing, and `proxy.with` overlay semantics.

### Phase 4: layered verification

Run focused tests immediately after each subsystem is reconciled, then the full gates. Do not debug all failures from one giant final run if they can be localized earlier.

Recommended area gates are:

```text
cd biscuit-file && just test && just test-l2 && just lint
cd darkmatter && just test && just test-l2 && just lint
cd claudine/rendezvous && just check && just test && just lint
cd claudine && just test && just test-l2 --no-fail-fast && just lint
cd claudine && just check-windows
```

`just test` is the L1 gate and uses nextest. `cargo test` is not an acceptable substitute. `cargo fmt` must not be run; `just lint` may perform the repository's read-only formatting check.

Sequence Plus's focus-stealing L3 keyboard tests must be run only through their guarded, attended/native or designated CI workflows. Native Windows runtime evidence must not be replaced by `just check-windows`, which only type-checks the target. The dedicated Linux L2 CI job must execute and reach test assertions; a tmux setup failure is not a passing or skipped feature test.

### Phase 5: final audit and closeout

1. Review `git diff` and GitNexus change detection for unexpected symbols, callers, processes, deleted guards, or duplicate execution paths.
2. Confirm all generated files match their generators and all source-scan/drift guards pass.
3. Run the full L1, L2, and lint gates again from a clean merged checkout.
4. Run native macOS, Linux, and Windows evidence required by the acceptance ledger, distinguishing runtime, real-terminal, type-check, and source-audit evidence.
5. Update every feature's acceptance/validation record with the merged commit SHA and fresh results. Do not inherit a green checkmark solely from a pre-merge branch run.
6. Move feature/fix records to `_completed` only after every required criterion is either proven or explicitly accepted as unresolved debt by the owner.

## Acceptance assurance matrix

| Workstream | Contracts that must survive | Required merged evidence |
|---|---|---|
| Error propagation | No known typed in-process flattening; complete diagnostic registry; effective diagnostic parity across renderer, `err.*`, and snapshot; one registered cause; lossless resolution/frontmatter detail; exactly-once and no-color rendering; unchanged exit/lifecycle/recovery behavior | Registry, chain, facet, snapshot/restoration, source-scan, boxed-diagnostic and transport guards at L1; direct/proxy/terminal error matrices and OSC 8/plain/no-color captures at L2; full Claudine L1/L2/lint |
| File resolution | `FileReference` is the sole grammar; bare repository-first and explicit source-relative semantics; deterministic candidate/probe provenance; one request snapshot; nested source re-anchoring; completion/execution parity; direct/proxy/sequence/Darkmatter/schema/transclusion parity; native path behavior | Biscuit File grammar/context/precedence/detailed-resolution/completion suites; Darkmatter composition/reference/schema/transclusion suites; Claudine direct/proxy/sequence and L2 file-resolution captures; caller audit; Biscuit File, Darkmatter, and Claudine test/lint gates; Windows and Unix path coverage |
| Sequence Plus | Retained behavior; typed rejection of invalid constructs; complete source/list grammar; JIT state/output visibility; exact approved/executed shell bytes; serial/parallel ordering and deterministic merge; lifecycle teardown; attributed output; interruption/process-tree ownership; final docs | Sequence validation matrix re-anchored to merged SHA; Claudine/Biscuit File/Darkmatter L1; task-stream and terminal L2; guarded L3 interruption evidence where required; `just check-windows` plus native Windows runtime evidence; full lints |
| Proxy-with | One canonical preparation service; coordinator-owned atomic transitions; direct/proxy equivalence; target-owned context/schema/lifecycle/loop/launch/closure; retry/resume compatibility; initialize-before-verdict staging; typed/redacted failures; complete `with:` typing, precedence, null removal, refresh/hop, policy, and file/hash neutrality | All 30 criteria remapped to merged tests; coordinator/state/overlay/launch/session tests at L1; the full real-CLI proxy equivalence and diagnostic matrices at L2; Linux L2 CI reaching assertions; direct/retry/resume/loop/provider-switch cases; full Claudine test/lint |
| Cross-feature seams | Proxy targets resolve through shared request context; proxy failures retain snapshots; proxies inside sequence steps remain contained; overlays and runtime state do not leak; schema and preflight order agree; one launch plan is both keyed and spawned | New or retained seam tests that combine features, not four isolated suites: implicit bare proxy target with candidate detail, proxy-with overlay into a Sequence step/loop, direct-versus-proxy typed file/schema failure, retry/resume after target/provider change, preflight shell-byte equality, and concurrent output plus lifecycle failure ordering |

## Risk-reduction work before the first merge

- Clean and protect the current worktree, especially the untracked merge-report directories.
- Run and record branch-tip gates now. This separates pre-existing failures from merge regressions.
- Restore a usable tmux/WezTerm L2 backend or arrange the Linux CI run before integration closeout. Proxy review 18 explicitly leaves current L2 evidence incomplete.
- Arrange native Windows execution for the Sequence Plus process-tree/console-control paths. Cross-compilation catches API drift but not runtime behavior.
- Export the 36-path conflict list into the merge checklist and assign each cluster an acceptance owner.
- Precompute symbol impact for the preparation, lifecycle executor, loop engine, wrapper pipeline, sequence runner, and Darkmatter compose-options seams.
- Decide the final responsibility map before resolving code: Biscuit File owns reference grammar/planning, Darkmatter owns Markdown composition/schema mechanics, Claudine library owns provider-neutral preparation/lifecycle/coordinator/sequence semantics, and Claudine CLI owns command orchestration/launch/terminal execution.
- Enable `git rerere` for this integration branch if repeated merge attempts are likely, while still reviewing reused resolutions.
- Review and checkpoint conflict resolutions by subsystem with patches, `rerere`, or other recoverable snapshots, then create the merge commit only after the complete index is coherent. Keep unrelated cleanup and formatting out of the merge.
- Preserve full branch ancestry with merge commits. Squashing or cherry-picking the 369 unique commits represented by the two fork histories would obscure provenance and make acceptance archaeology harder.

## Completion definition

The mega-merge is complete only when the `claudine` branch contains both feature histories, no duplicate legacy execution path remains, all four specifications are mapped to fresh merged-code evidence, every required L1 and L2 assertion actually ran and passed, all lints and drift guards pass, and the cross-platform claims are supported at the level stated. Backend denial, skipped tests, compile-only checks, and pre-merge results must be labeled accurately rather than counted as merged acceptance.
