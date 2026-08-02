# Propagated request-context implementation plan

Reference: [`spec.md`](spec.md)

## Review conclusion

The architect's revisions are accepted. Source inspection confirms the added
accounting and the newly named paths:

- direct-wrapper startup discovers Git before starting a Sniff plan that
  discovers it again;
- one same-repository composition can enumerate topology four times before
  Darkmatter runtime-context capture adds further Git, topology, and environment
  work;
- sequence task/JIT/error paths re-enter context-free resolution;
- explicit system-prompt resolution builds a `FileResolutionContext` and drops
  it before composition;
- enabled passthrough harnesses use `ComposeOptions::new()` and no
  `FileResolutionContext`; and
- the direct wrapper does not transfer its available substage timings into the
  performance collector.

Two scope clarifications were added to the spec during this review:

1. `darkmatter` and `sniff` are affected packages, because the required
   evidence-aware seams cannot be implemented wholly inside Claudine.
2. This fix owns propagation and request-scoped evidence reuse. The related
   faster-compose feature owns compose-session caching, collapsing repeated
   in-memory passes, and reducing the cost of an individual detector.

Every symbol and path in this plan was checked against source commit `21d17a2`
and the architect's current working-tree spec revision on 2026-08-01. Locate
implementation sites by symbol when lines move.

## Completion contract

Implementation is complete only when all of the following are true:

- one invocation owner captures launch CWD, HOME, environment, Git, topology,
  and the current strict/best-effort discovery result;
- repository topology is observed once per distinct worktree entered by an
  invocation, including parallel sequence execution;
- the first resolved composition source is not immediately re-derived;
- canonical composition, sequence, system-prompt, appendix, and harness paths
  use explicit source contexts and do not fall back to ambient CWD/HOME/Git;
- Darkmatter runtime capture can consume supplied evidence without its hidden
  second Git discovery or a fresh environment snapshot;
- a provider memory file without harness properties is never body-composed by
  Claudine, while an enabled harness remains behaviorally equivalent;
- direct-wrapper `--perf` separates discovery, system-prompt, environment, MCP,
  harness eligibility, and harness materialization work while preserving tree
  reconciliation;
- deterministic tests prove work bounds without process-global counters or
  elapsed-time assertions; and
- the three formerly timing-out real-CLI tests keep their original assertions
  and complete under concurrent nextest execution.

## Verified impact and risk

GitNexus is indexed at `21d17a2`. Its exact call-graph results classify these
central seams as high or critical risk:

| Seam | Risk and relevant reach |
|---|---|
| `capture_file_resolution_context` | Critical; feeds composition, schema, preflight, sequence, JIT, and tests |
| `derive_request_context_for_source` | High; reaches compose, sequence, and harness orchestration |
| `resolve_and_prepare_for_session` | Critical; direct wrappers and all composition launch paths |
| `compose_prompt_markdown` | Critical; primary prompts and appendices |
| `ComposeContext::capture_for_document` | Critical; Darkmatter CLI/library and Claudine callers |
| `detect_with_plan` | Critical; 33 direct callers across Sniff and Claudine |
| `build_harness_shell_options` | Critical; direct and composition harness paths |
| `detect_wrapper_harness` | High but narrow; the direct-wrapper harness decision |

Therefore the Sniff and Darkmatter changes must be additive. Existing ambient
entry points remain compatibility APIs; migration is limited to canonical
Claudine invocation paths.

## Locked design decisions

### 1. The owner lives in the `claudine` library

Add an invocation-scoped context owner in the library so both `claudine-cli`
and library-owned system-prompt preparation can consume the same evidence. Use
one cohesive type, provisionally `InvocationContext`, rather than threading a
growing list of unrelated `Option<&...>` arguments.

The owner holds immutable launch facts plus a request-local repository
observation cache. It projects existing types instead of replacing them:

- `FileResolutionContext` for launch and source file resolution;
- `LaunchContext` for system-prompt candidate discovery;
- `LaunchWorkspaceContext` for child-CWD/repository behavior;
- `EnvironmentContext` for lifecycle/event data; and
- Darkmatter capture evidence for requested `ctx.*` groups.

`CompositionPrepContext` may remain as a CLI selection/provider wrapper, but it
must receive and reuse the invocation owner. It must no longer own an
independent Sniff/topology scan.

### 2. Repository observations are keyed by worktree identity

Use a request-local key built from platform-aware paths returned by Sniff, not
rendered path strings. The key includes the worktree root and worktree-specific
Git directory; linked worktrees must not collapse merely because they share a
common Git directory. Bare repositories use their Git directory identity.

Each cache entry is single-flight and retains:

- repository presence or explicit absence;
- the discovered Git handle or an immutable projection sufficient for later
  requested Git/status work;
- Git identity/summary evidence;
- the optional `RepoInfo` topology result;
- package/package-area projections; and
- the original typed failure plus its cloneable diagnostic projection.

Known roots use bounded `Path` containment. Before reusing an enclosing root,
check ancestors between the source and that root for a nested `.git` file or
directory so a nested repository retains its identity. A first unknown source
is resolved through a serialized request-local root-identification gate, then
inserted under its definitive key. Topology is protected by a per-entry
single-flight cell so parallel sequence tasks cannot duplicate it. Negative
non-repository observations are cached for the exact source base and never
start repository-structure detection.

Do not use a process-global cache, string-prefix comparison, lowercased paths,
or a common-Git-directory-only key.

### 3. Source context is a bundle, not a path hint

Deriving a source returns one bundle containing at least the resolved source
base, repository observation, repository/package roots, and the derived
`FileResolutionContext`. The same bundle is passed into preparation,
preflight, lifecycle, sequence, system-prompt, and harness consumers.

The top-level flow remains two conceptual steps—resolve against launch context,
then derive against the resolved source—but both use the same owner. The
definitive first-source bundle is passed to
`prepare_and_run_active_document`; it is not re-derived there.

Document content stays fresh at retry/resume/JIT boundaries. Only immutable
launch and repository evidence is reused.

### 4. Sniff gains one additive pre-observed-filesystem seam

Refactor Sniff's existing filesystem implementation so a detection request can
consume an already discovered Git observation and retain it for the
request owner. The ordinary `detect_with_plan` API delegates to this internal
path with no seed and preserves its current behavior.

The seam must:

- perform no second `GitRepo::discover` when a seed is supplied;
- reuse the same Git handle for `GitInfo`, repository walk-root selection, and
  later request-scoped Git/status facts;
- preserve bare-repository and linked-worktree handling;
- participate in the current performance collector and worker propagation; and
- produce output equivalent to `detect_with_plan` for an equivalent request.

This is evidence propagation, not a new detector and not an optimization of
Sniff's workspace walk.

### 5. Darkmatter distinguishes ambient capture from supplied evidence

Expose the existing context-group scan as a small public requirements type and
add an evidence-aware `ComposeContext` capture entry point. Its supplied mode
receives:

- the invocation environment snapshot;
- source/repository identity and topology;
- requested Git/status or file-change facts;
- requested docs/language facts; and
- requested OS/hardware/GPU facts, when applicable.

The existing `capture_for_content`, `capture_for_document`, and
`ComposeOptions::new()` remain ambient compatibility paths and delegate to the
same population code. The new supplied-evidence path is fail-closed with
respect to ambient discovery: absent evidence becomes the existing typed
partial-capture diagnostic/empty projection, not a call to CWD, HOME, Git,
Sniff, or `std::env::vars()`.

The invocation owner uses the public requirements scan before capture and
populates only requested groups. Group evidence is cached request-locally. A
later document may request an additional group, but it must reuse the stored
Git handle/topology and environment snapshot. This fix does not add a
long-lived Darkmatter compose session or collapse multiple calls over the same
in-memory document.

### 6. Harness eligibility is frontmatter-only

Use Darkmatter's canonical fallible Markdown parser to read authored
frontmatter and call the existing `has_harness_properties` predicate. Do not
introduce a YAML shortcut or duplicate property list.

Only an enabled harness enters canonical full materialization. That path uses
the source bundle's `FileResolutionContext`, demand-driven runtime evidence,
and explicit source repository root for shell policy. Malformed frontmatter
continues to raise its current typed error before returning “no harness.”

### 7. Work accounting is request-local

Add counters to the request owner and bridge Sniff's existing scoped
performance collector where useful. Count at the chokepoint for:

- Git root discovery;
- topology probe versus topology reuse;
- runtime evidence capture by group;
- system-prompt lookup and each compose operation;
- harness eligibility parse and full materialization; and
- ambient-fallback use on canonical paths (expected zero).

No test-only mutable static is an acceptance authority. Darkmatter's existing
`GIT_DISCOVERY_COUNT` may remain for local unit tests, but the new regression
tests must use injected/request-owned accounting and must include file-change
capture so the currently missed second discovery is observable.

### 8. Error and performance contracts remain structural

Cache success, absence, and failure. Strict consumers such as `--repo` surface
the retained typed failure; best-effort projections receive absent repository
fields and the same deferred diagnostic they receive today. Do not retry a
failed observation through another projection.

All new `--perf` substages are structural children of their current parent.
Nested details are breakdown-only where their duration is already included in
the parent. Extend reconciliation tests before changing report output.

## Implementation order

```text
baseline and inventory
    -> Sniff reusable Git observation
    -> Claudine invocation owner
    -> Darkmatter evidence-aware runtime capture
    -> composition and sequence migration
    -> system-prompt and direct-wrapper migration
    -> harness eligibility/materialization split
    -> performance attribution and hermetic CLI regressions
    -> documentation and final gates
```

Sniff and Darkmatter foundation work can be developed independently after the
baseline, but both must land before canonical Claudine paths are switched.

## Phase 0 — Freeze behavior and work baselines

### Source inventory

- [ ] Refresh GitNexus impact summaries for
  `capture_file_resolution_context`, `derive_request_context_for_source`,
  `CompositionPrepContext::new`, `resolve_and_prepare_for_session`,
  `compose_prompt_markdown`, `ComposeContext::capture_for_document`,
  `detect_with_plan`, `detect_wrapper_harness`, and
  `build_harness_shell_options`.
- [ ] Inventory production `GitRepo::discover`, `detect_git`,
  `detect_repo_structure`, and hand-rolled `.git` walks under
  `claudine/lib/src` and `claudine/cli/src`.
- [ ] Classify each occurrence as canonical invocation capture, distinct-source
  capture, compatibility fallback, command outside this spec, or redundant.
- [ ] Inventory every Claudine-created `ComposeOptions` for a file-backed
  document and record whether it carries a source file,
  `FileResolutionContext`, shell CWD, and demand-driven runtime context.

Known migration targets include:

- `claudine/lib/src/composition/resolve.rs`;
- `claudine/lib/src/composition/prepare.rs`;
- `claudine/lib/src/composition/lifecycle/control.rs`;
- `claudine/lib/src/system_prompt/{resolve,prepare}.rs`;
- `claudine/cli/src/commands/wrap/env/mod.rs`;
- `claudine/cli/src/commands/wrap/composition/prep_context.rs`;
- `claudine/cli/src/commands/{compose,wrap/sequence}/`;
- `claudine/cli/src/commands/wrap/{wrapper_stages,overlay}.rs`; and
- `claudine/cli/src/commands/wrap/harness_orch/shell_options.rs`.

### Reproducible baseline

- [ ] Run the current ignored diagnostics with the same built binary, fake
  provider, isolated HOME, isolated/repository CWDs, and rendezvous disabled.
- [ ] Record cold and warm wall time only as supporting evidence.
- [ ] Add or enable request-local work reporting before behavior changes and
  record Git/topology/runtime-capture/materialization counts for:
  direct wrapper, same-repo compose, same-repo sequence, cross-repo source,
  system prompt plus appendix, and memory file without harness properties.
- [ ] Run the three named timeout regressions together under nextest and retain
  their assertion lists as the utility baseline:
  `compose_system_prompt_shell_failure_renders_rich_block`,
  `compose_preflight_discovers_shell_inside_false_block`, and
  `compose_preflight_error_includes_source_provenance`.

### Checkpoint 0

- The call-site inventory is attached to implementation notes or tests.
- Every expensive unit has a deterministic observation seam.
- No optimization has changed user-visible output.

## Phase 1 — Sniff: reuse a pre-discovered Git observation

### Production changes

- [ ] In `sniff/lib/src/filesystem/mod.rs`, split
  `detect_filesystem_with_request_inner` into observation acquisition and
  request execution so the latter can accept a pre-observed Git handle.
- [ ] Add the narrow public or crate-exported seed/result type needed by
  Claudine's library boundary; keep `GitRepo` implementation details opaque
  where possible.
- [ ] In `sniff/lib/src/lib.rs`, add an additive planned-detection entry point
  that accepts the filesystem observation while keeping `detect_with_plan`
  byte-for-byte compatible for existing callers.
- [ ] Preserve scoped-thread performance collector propagation and count Git
  discovery at its single chokepoint.
- [ ] Review and update the comments in `filesystem/mod.rs` that currently say
  discovery occurs once; after this phase they must describe both seeded and
  ambient entry paths accurately.

### L1 tests

- [ ] Seeded and ordinary detection return equivalent `GitInfo`, `RepoInfo`,
  launch-root, and absence results for the same request.
- [ ] A seeded request records zero additional Git discoveries.
- [ ] Normal worktree, linked worktree (`.git` file), bare repository, nested
  repository, and non-repository fixtures retain their current semantics.
- [ ] A discovery failure is returned once and is not retried by the request
  executor.
- [ ] Performance counters from the filesystem worker reach the parent
  collector.

### Checkpoint 1

Run from `sniff/`:

```sh
just test
just lint
```

No Claudine caller switches yet; the existing Sniff API remains compatible.

## Phase 2 — Claudine: establish the invocation owner

### Production changes

- [ ] Add the invocation owner and repository observation/cache modules under
  `claudine/lib/src/`, with a narrow re-export for `claudine-cli`.
- [ ] Capture launch CWD once and construct the launch
  `FileResolutionContext` once so HOME, environment, configured roots, and
  magic roots are immutable for the invocation.
- [ ] Capture launch Git identity through the Phase 1 Sniff seam and retain the
  observation for later Git/status requests.
- [ ] Implement repository identity, nested-boundary checking, negative
  observations, per-entry topology single-flight, and request-local work
  accounting.
- [ ] Implement projections to existing `LaunchContext`,
  `LaunchWorkspaceContext`, and `EnvironmentContext` without further Sniff
  calls.
- [ ] Implement source derivation returning the definitive source bundle and
  source-derived `FileResolutionContext`.
- [ ] Retain the typed launch failure for strict consumers and its diagnostic
  projection for cloneable/best-effort consumers.
- [ ] Change `capture_file_resolution_context` and
  `derive_request_context_for_source` into compatibility wrappers over the
  owner, or retain them only for external callers. Correct their drifted
  “exactly once/no later reads” documentation.

### L1 tests

- [ ] One launch observation produces all four existing context projections.
- [ ] Same-repository source derivation reuses launch topology.
- [ ] Two sources in one sibling repository add one observation/topology probe.
- [ ] Nested repositories and linked worktrees get distinct keys.
- [ ] A non-repository source returns explicit absence and performs no topology
  walk.
- [ ] Parallel requests for the same unseen repository perform one root
  identification and one topology probe.
- [ ] Mutating process CWD, HOME, or environment after capture cannot alter any
  projection.
- [ ] Strict and best-effort consumers project one retained failure without a
  retry.
- [ ] Windows drive/UNC-shaped path tests exercise keying without string-prefix
  assumptions; filesystem-dependent link tests remain platform-gated only when
  necessary.

### Checkpoint 2

Run focused `claudine` library tests. Canonical CLI migration has not started,
so compatibility wrappers must keep existing callers green.

## Phase 3 — Darkmatter: evidence-aware demand capture

### Production changes

- [ ] Promote the internal `ContextGroup`/`scan_needed_groups` concept to a
  minimal public requirements API without exposing population internals.
- [ ] Add evidence structs for the raw facts `ContextCapture` consumes. Reuse
  Sniff `GitInfo`/file-change, `RepoInfo`, docs, OS, and hardware types rather
  than defining parallel domain models.
- [ ] Add supplied-evidence constructors in
  `markdown/compose/context/capture/snapshot.rs` and route them through the
  existing `populate_*` modules.
- [ ] Add `ComposeContext::capture_for_content_with_evidence` and
  `capture_for_document_with_evidence` in `context/runtime.rs`.
- [ ] Make `ComposeContext::from_values` accept the captured environment rather
  than unconditionally calling `std::env::vars()`; ambient callers pass a fresh
  snapshot, supplied callers pass the invocation snapshot.
- [ ] Eliminate the file-change worker's second `GitRepo::discover` by consuming
  the provided Git/status evidence or the original ambient capture handle.
- [ ] Ensure supplied mode never fills missing facts by ambient discovery.
- [ ] Keep `ComposeOptions::new()` and existing ambient capture methods
  behaviorally compatible; changing their default breadth belongs to
  faster-compose.

### L1 tests

- [ ] Requirements scanning remains identical for frontmatter and body
  references, escaped literals, aliases, and all context groups.
- [ ] Ambient and supplied captures produce the same values for the same frozen
  facts.
- [ ] Supplied repo/area/file-change capture performs no Git or topology
  discovery, including the file-change branch.
- [ ] Supplied environment values survive a later process-environment mutation.
- [ ] Missing supplied evidence produces the existing partial-capture
  diagnostic and never invokes an ambient fallback.
- [ ] Documents without a requested host group do not capture OS, hardware, or
  GPU facts.

### Checkpoint 3

Run from `darkmatter/`:

```sh
just test
just lint
```

Also run the affected Darkmatter CLI tests because the compatibility capture
entry points remain shared.

## Phase 4 — Migrate composition, lifecycle, and sequence

### Top-level composition

- [ ] In `claudine/cli/src/commands/compose/prep.rs`, create one invocation
  owner before top-level reference resolution, resolve with its launch context,
  derive one definitive source bundle, and pass that bundle to
  `prepare_and_run_active_document`.
- [ ] Remove the immediate second `derive_request_context_for_source` for the
  first active document.
- [ ] Refactor `CompositionPrepContext::new` to consume the invocation/source
  bundle and retain only provider inventory, selection configuration, and other
  facts not owned by the shared context.
- [ ] Make canonical `PrepareOptions` and every file-backed `ComposeOptions`
  carry the source `FileResolutionContext` and evidence-aware runtime context.
- [ ] Replace `prepare.rs::find_git_root_from_path` and
  `effective_source_repo_root` fallback use on canonical paths with the source
  bundle; leave a clearly named compatibility fallback only when no context is
  supplied.
- [ ] Replace `composition/resolve.rs::with_prompt_magic_paths` ambient error
  enrichment with candidate/provenance data derived from the supplied context.
- [ ] Replace lifecycle `package_area_for_source` topology discovery with the
  prepared source bundle's package-area projection.

### Re-entry and sequence paths

- [ ] Thread the invocation owner through proxy, retry, resume, and loop
  orchestration. Fresh reads keep the current entry matrix; context derivation
  occurs only when source identity changes.
- [ ] Change `wrap/sequence/task_run.rs`, `phase1c.rs`, JIT reloads, and sequence
  failure rendering to use `resolve_composition_source_in_context` with the
  active source/launch context rather than `resolve_composition_source`.
- [ ] Share the repository cache safely across serial and parallel task groups.
  Do not share mutable document state or change declaration-ordered merge.
- [ ] Preserve direct/proxy equivalence, launch-workspace child CWD, approval
  caching, lifecycle error routing, and retry/resume freshness.

### L1 tests

- [ ] Compose and inline-compose in the launch repository record one topology
  probe and one environment snapshot.
- [ ] Serial and parallel same-repository sequences record one topology probe.
- [ ] Same-repository proxy/retry/resume/JIT paths add no probe while fresh
  document content is observed.
- [ ] A sibling-repository target adds exactly one probe and becomes the base
  for its authored references.
- [ ] Schema, expression, transclusion, local-link, lifecycle, and error
  enrichment paths all see the same source context.
- [ ] Existing direct/proxy launch-bundle and session-compatibility tests remain
  unchanged and green.

### Checkpoint 4

Run focused composition, sequence, lifecycle, file-resolution, and proxy tests
from the `claudine` package area. Do not proceed while any work counter exceeds
the spec table even if wall time improved.

## Phase 5 — Migrate system prompts and direct-wrapper startup

### System prompt

- [ ] Change `resolve_and_prepare_for_session` to receive the invocation owner
  (retain an ambient convenience wrapper only for compatibility).
- [ ] Change explicit file resolution to use the owner's launch
  `FileResolutionContext` and retain the selected source bundle instead of
  returning only `PathBuf` plus text.
- [ ] Keep automatic candidate selection launch-scoped. Derive a source bundle
  only after a primary or appendix file is selected.
- [ ] Build the union context requirements for the primary and appendix, ask
  the owner for evidence once, and use Darkmatter's Phase 3 supplied-evidence
  capture.
- [ ] Attach each file's source-derived `FileResolutionContext` to its
  `ComposeOptions` while sharing the immutable runtime evidence.
- [ ] Keep `::shell` pinned to the launch repository root, not the selected
  prompt's directory or a sibling source repository.
- [ ] Preserve empty-disable, appendix fallback, discovered `mode` validation,
  append/replace delivery, built-in appendix behavior, and provider artifacts.

### Direct wrapper

- [ ] In `wrap/env/mod.rs`, replace the promptless-at-root pre-probe plus
  independent plan with the Phase 1 pre-observed Sniff path owned by the
  invocation context.
- [ ] Build `WrapStartupDetection` projections from that owner and retain the
  owner through system prompt, child environment, MCP, and harness stages.
- [ ] Preserve promptless-at-root topology omission, while testing ordinary,
  linked-worktree, bare, nested, and non-repository roots.
- [ ] Pass `--perf` intent into `build_child_env_with_launch` and transfer
  `EnvPlan.perf_substages` to the command collector instead of dropping them.

### L1 tests

- [ ] Automatic and explicit system prompts, repository/user appendices, and
  multiple local links add no topology probes after their repository is known.
- [ ] An explicit prompt in a sibling repository adds one observation without
  moving automatic prompt discovery away from launch CWD.
- [ ] Every provider's effective prompt content, argv/env delivery, and temp
  artifact behavior remains equivalent.
- [ ] Promptless root wrappers perform one Git discovery and no topology probe;
  prompted wrappers request topology only when needed.
- [ ] Shadow-HOME/process-CWD changes after capture do not affect prompt
  resolution or runtime `env.*`/`ctx.*` values.

### Checkpoint 5

Run focused system-prompt, wrapper environment, provider-delivery, and
launch-plan/session-key tests. Re-run the system-prompt shell-failure real-CLI
regression before moving to harness work.

## Phase 6 — Split harness eligibility from materialization

### Production changes

- [ ] Add a fallible frontmatter-only eligibility helper beside
  `detect_wrapper_harness` that reads the selected memory file once with
  Darkmatter's canonical parser and evaluates `has_harness_properties`.
- [ ] Return `None` immediately for a valid non-harness file without calling
  `materialize_passthrough_harness_seed`.
- [ ] Allow enabled materialization to reuse the already loaded Markdown value
  where doing so is surgical; avoiding a second parse is optional and belongs
  to faster-compose if it requires a compose session.
- [ ] Replace `ComposeOptions::new()` in `wrap/overlay.rs` with the source
  bundle's demand-driven supplied `ComposeContext`, source file,
  `FileResolutionContext`, and existing shell CWD.
- [ ] Store the propagated `FileResolutionContext` on
  `MaterializedHarnessPrompt` instead of `None`.
- [ ] Change `build_harness_shell_options` to accept the explicit source
  repository/policy root and delete the canonical path's hand-rolled `.git`
  ancestor walk. Retain a compatibility helper only for callers that truly
  lack a source bundle.
- [ ] Preserve prompt text, lifecycle parsing, approvals, overlay precedence,
  MCP-tag behavior, runtime state, retry/resume/proxy handling, and all typed
  errors.

### L1 tests

- [ ] No prompt performs no memory-file lookup.
- [ ] No candidate performs no parse or materialization.
- [ ] A valid ordinary memory file performs one frontmatter parse and zero body
  compositions, including a body containing a shell directive that would fail
  if composed.
- [ ] Malformed frontmatter returns the existing typed error.
- [ ] An enabled harness's materialized prompt/frontmatter and all launch facets
  match the pre-change path.
- [ ] Enabled harnesses capture only referenced runtime groups and perform no
  extra Git, topology, OS, hardware, or GPU probes.
- [ ] Shell approval policy uses the source repository for same-, sibling-, and
  non-repository memory files without walking for `.git`.

### Checkpoint 6

Run focused passthrough harness, lifecycle, shell-approval, launch-plan, MCP,
and overlay tests. Compare captured provider argv, environment, CWD, prompt,
and lifecycle trace to the Phase 0 baseline.

## Phase 7 — Performance attribution and hermetic real-CLI coverage

### Performance tree

- [ ] Add explicit direct-wrapper checkpoints around launch discovery,
  system-prompt lookup/runtime capture/primary compose/appendix compose/delivery,
  child environment, MCP, harness eligibility, and harness materialization.
- [ ] Place harness work inside the environment/preparation structural interval
  rather than leaving it in top-level `unattributed`.
- [ ] Attach child-environment internal timings as breakdown children only;
  never count their duration a second time structurally.
- [ ] Record source-context derivation as probe versus reuse through work
  metadata or a bounded timing substage.
- [ ] Extend `perf/tree.rs`, `perf/report.rs`, and their tests so success,
  failure, dry-run, no-prompt, no-harness, and enabled-harness trees reconcile.

### Real CLI fixtures

- [ ] Make the shared process-test default create an isolated CWD and HOME,
  disable rendezvous, and build PATH with `std::env::join_paths`.
- [ ] Require explicit fixture opt-ins for repository topology, root/user
  system prompts, non-interactive appendices, provider memory files,
  configuration, and shadow HOME.
- [ ] Preserve every original assertion in:
  `compose_system_prompt_shell_failure_renders_rich_block`,
  `compose_preflight_discovers_shell_inside_false_block`, and
  `compose_preflight_error_includes_source_provenance`.
- [ ] Add dedicated real-CLI fixtures proving automatic root/user prompt,
  appendix, provider-memory, repository/package, and shadow-HOME behavior was
  not removed by the new default isolation.
- [ ] Run the three timeout regressions concurrently under nextest. Do not add
  serialization, retries, or wider timeouts to make them pass.

### Diagnostic comparisons

- [ ] Re-run the exact Phase 0 cold/warm direct-wrapper, compose, system-prompt,
  sequence, and harness scenarios.
- [ ] Record wall time and work counts together. Treat wall time as supporting
  evidence; fail correctness only on structural work bounds and behavior.
- [ ] Confirm residual `unattributed` time is small and no named stage is
  structurally double-counted.

### Checkpoint 7

From the repository root:

```sh
just _test claudine-cli --no-fail-fast
```

The run must report no timeout for the three named regressions and no loss of
their assertion coverage.

## Phase 8 — Documentation and final verification

### Documentation/comment pass

- [ ] Update `.claude/skills/claudine/composition.md` with invocation ownership,
  per-repository reuse, and source-context propagation.
- [ ] Update `.claude/skills/claudine/system-prompt.md` with shared runtime and
  file-resolution evidence and the launch-root shell rule.
- [ ] Update `.claude/skills/sniff/architecture.md` and `performance.md` for the
  seeded observation seam and its counters.
- [ ] Update `.claude/skills/darkmatter/compose.md` for supplied-evidence
  demand capture while preserving ambient compatibility APIs.
- [ ] Update corresponding package docs/READMEs where the same architecture is
  documented.
- [ ] Review every touched `///`, `//!`, and inline comment. Remove or correct
  claims that a local helper performs the invocation's only discovery when the
  full call path says otherwise.
- [ ] Update dependency docs only if manifests actually change; no new crate
  dependency is expected because the current dependency direction already
  supports these APIs.

### Package gates

Run in dependency order:

```sh
cd sniff
just test
just lint

cd ../darkmatter
just test
just lint

cd ../claudine
just test
just lint

cd ..
just _test claudine-cli --no-fail-fast
```

Use `just test-l2` only if implementation changes behavior that requires a real
terminal to verify. Context propagation and performance work itself is L1 and
must not open or focus terminal/browser windows.

### Final acceptance audit

- [ ] Map every spec acceptance criterion to a named test, counter assertion,
  diagnostic result, or documentation change.
- [ ] Run `git diff --check` and inspect the final diff for unrelated changes,
  accidental test weakening, and stale comments.
- [ ] Run GitNexus `detect-changes` and refresh impact analysis for any public
  seam whose reach differs from this plan.
- [ ] If changes reach packages outside `sniff`, `darkmatter`, and the Claudine
  area, expand verification according to actual impact before declaring the
  fix complete.

## Acceptance-to-phase map

| Spec area | Implementation phase | Primary proof |
|---|---:|---|
| One invocation owner and context projections | 2 | projection/failure/CWD-HOME mutation L1 tests |
| One topology probe per repository | 2, 4 | request-local counters, serial/parallel sequence tests |
| No first-source re-derivation | 4 | compose/inline work-count tests |
| System-prompt and appendix propagation | 5 | source-context and provider-delivery tests |
| No per-link or Darkmatter ambient probe | 3, 4, 5 | supplied-evidence capture and multi-link tests |
| Harness eligibility/materialization split | 6 | no-compose ordinary file and enabled equivalence tests |
| Retry/resume/proxy freshness | 4 | existing equivalence suite plus topology counters |
| Direct-wrapper Git reuse | 1, 5 | seeded Sniff and promptless-root tests |
| Reconciling performance attribution | 7 | deterministic perf-tree tests |
| Former timeout regressions retain utility | 7 | original assertions plus concurrent nextest run |
| macOS/Linux/Windows support | all | path-shape L1 tests and platform-aware fixtures |
| Documentation/comment accuracy | 8 | final docs and drift audit |

## Principal risks and controls

| Risk | Control |
|---|---|
| A nested repository is mistaken for the launch repository | Bounded nested `.git` boundary check before containment reuse; nested-repo fixture |
| Linked worktrees collapse into one cache entry | Key by worktree root plus worktree-specific Git directory, not common directory |
| Parallel sequence tasks duplicate first-entry discovery | Request-local root-resolution gate plus per-repository single-flight topology |
| Cached evidence makes document content stale | Cache immutable discovery facts only; retain fresh reads at retry/resume/JIT boundaries |
| Darkmatter silently falls back to ambient state | Explicit supplied mode where missing evidence becomes diagnostics; zero-fallback counter |
| File-change capture hides a second Git discovery | Reuse the original handle/evidence and assert the file-change scenario's total count |
| Strict `--repo` behavior degrades to best effort | Retain typed failure and test both projections from one failed observation |
| System-prompt discovery shifts to a source repository | Keep selection launch-scoped; derive source context only after candidate selection |
| System-prompt shell CWD shifts with source | Pass launch repository root separately from source file-resolution context |
| Harness optimization weakens lifecycle or prompt semantics | Frontmatter-only early exit; enabled path uses canonical materialization and equivalence fixtures |
| Performance report double-counts nested work | Structural/breakdown distinction and reconciliation tests for every wrapper shape |
| This fix absorbs faster-compose scope | Keep compatibility capture calls and compose passes; optimize only evidence propagation |
| Test isolation removes discovery coverage | Explicit discovery fixtures alongside isolated defaults; original assertions preserved |
