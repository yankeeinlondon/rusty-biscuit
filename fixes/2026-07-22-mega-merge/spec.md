# Mega Merge

Status: complete

## Objective

Integrate the completed Claudine and Darkmatter merge streams and the Sniff
performance stream into `main` without losing behavior, weakening the Sniff
cost model, or accepting a mechanically clean merge as evidence of semantic
correctness.

The integration candidate lives in the `mega-merge` worktree on
`feat/mega-merge`. Its frozen base is `main` at
`d30aedd36829256bc677e1d2e73f47a9a2e6005f`.

## Inputs

| Stream | Frozen tip | Role in the integration |
|---|---|---|
| `sniff` | `0b3286a193899f800a97a24ee3e35c8042602cf6` | Host, repository, Git, worktree, and remote observation |
| `darkmatter` | `7fb7136dca32a7b1f971b4c83bc1733bcdedebee` | Composition, expressions, schemas, references, and cache behavior |
| `claudine` | `8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3` | Orchestration, lifecycle, dispatch, retry, and handoff behavior |

The source branches are frozen inputs. Conflict resolution and stabilization
belong only on `feat/mega-merge`; the source branches must not receive merges
from `main` or from one another.

## Decision

Use one serial integration candidate and preserve ancestry with merge commits:

1. Merge `sniff` into `feat/mega-merge` and stabilize the Sniff package area.
2. Merge `darkmatter` into the candidate and reconcile the Sniff–Darkmatter
   observation boundary.
3. Merge `claudine` into the candidate and reconcile the
   Darkmatter–Claudine composition boundary.
4. Reconcile generated and operational artifacts after behavior is settled.
5. Run the complete staged verification matrix.
6. Merge the single verified candidate into `main`.

This is a macro-order decision, not permission to apply a branch-wide conflict
preference. The spikes showed that both `-X ours` and `-X theirs` produce
textually resolved but internally inconsistent Darkmatter/Claudine files. The
real merges must use ordinary conflict markers plus the ownership matrix below;
auto-merged files in the semantic seam are mandatory review targets.

### Why this path

The dependency direction is Sniff → Darkmatter → Claudine. A consumer should
be reconciled only after its dependencies are stable. This provides one
resolution for every boundary, gives each failure a useful first-bad stage,
keeps `main` stable, and leaves all source branches recoverable.

The evaluated paths have approximately the same number of textual conflict
paths. The rejected paths therefore offer no meaningful mechanical advantage:

| Path | Approximate conflict paths | Reason rejected |
|---|---:|---|
| Darkmatter → Claudine → Sniff | 47–48 | Foundational Sniff behavior lands last and can invalidate earlier stabilization. |
| Sniff into `main`, then `main` into both consumers | Similar or higher | Resolves the same Sniff seams independently and can produce inconsistent decisions. |
| Claudine → Darkmatter → Sniff | 46–47 | A marginal count difference reverses dependency order and leaves the foundation last. |
| **Sniff → Darkmatter → Claudine** | **47–48** | Selected: one serial candidate in dependency order. |

Counts are conflict paths per merge stage, not unique files. Later counts vary
with earlier resolutions, so a difference of one or two paths is not material.

### Semantic ownership matrix

| Seam | Starting authority | Required composition |
|---|---|---|
| Sniff aggregate Git/worktree projection | Sniff | Preserve zero linked-repository opens and request-scoped observations. |
| Sniff focused worktree inspection | Darkmatter | Validate registered targets, omit absent stale registrations, and report existing corrupt repositories. |
| Sniff remote providers | Additive | Keep Sniff provider URL/remote selection and add Darkmatter exact/list PR and CI/CD methods with compatible defaults. |
| `biscuit-file` facade | Additive | Export Claudine's file-reference/list APIs and Darkmatter's YAML span/analyzer APIs; neither side's module declarations may replace the other. |
| Darkmatter request/file-resolution context | Claudine | Keep one captured `FileResolutionContext`; add Darkmatter remote, cache, identity, and meta-schema state to it rather than adding ambient recapture paths. |
| Darkmatter schemas and references | Symbol-level composition | Preserve Claudine provenance/typed-error behavior and Darkmatter origin, dependency, cache, trigger, meta-schema, and freshness behavior. Whole-file or global hunk preference is forbidden. |
| Claudine lifecycle/orchestration | Claudine | Keep the split test layout and canonical handoff/retry/resume behavior; port surviving Darkmatter assertions into that layout. |

## Baseline state and inherited debt

The merge must distinguish inherited failures from integration regressions.

### Claudine

The early draft inherited a phase-six report that
`level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`
failed because a `set_frontmatter` retry changed the agent to Gemini without
replacing the Goose provider. A managed L2 spike against the frozen Claudine
tip could not reproduce that report: the isolated test passed four consecutive
runs, and the existing L1 coverage also proves that unavailable scalar and
list provider selections are refused identically to direct selection.

The recorded failure is therefore stale evidence, not a known defect in the
frozen input. No retry production change is justified unless the failure
recurs on the exact staged candidate. Native Linux L1/lint/L2, native Windows
runtime, and attended L3 evidence remain absent.

### Sniff

Sniff review 12 remains `ready: false` because matched native Linux and Windows
work-count evidence is absent. The review did not identify a new macOS
correctness failure. Cross-platform evidence is a final merge gate.

### Repository hygiene

- The source Claudine worktree has a locally modified generated `CLAUDE.md`.
- Local `.claude/settings.local.json` files are untracked and out of scope.
- Claudine contains a tracked empty path named
  `~/features/2026-07-20-router-fixture/log.md`. Its fixture purpose and final
  location must be explicitly confirmed before the Claudine merge is accepted.
- Existing diff-check warnings are baseline evidence. This merge must not turn
  into a broad formatting or whitespace cleanup.

### Agent Skills

The merged Claudine, Darkmatter, and Sniff behavior must be reflected in their
three package skills. This is both a documentation-drift gate and an Agent
Skills portability gate; a skill that accurately describes the code but cannot
be loaded efficiently or validated as a portable skill is not complete.

The frozen inputs expose three concrete issues:

- The incoming Darkmatter `SKILL.md` is 874 lines / 6,766 words. It exceeds the
  skill-authoring guidance to keep the entry point below 500 lines and roughly
  5,000 words. Its DMLS chronology, extracted-surface catalog, and rendering
  implementation notes are candidates for directly linked topic references.
- The incoming Sniff `SKILL.md` is 331 lines but 5,320 words. Its work-counter
  evidence and detailed CLI/catalog material should move behind concise topic
  routing while the request/cost-model invariants remain in the entry point.
- Claudine's incoming `SKILL.md` is within both limits at 247 lines / 3,255
  words, but it still requires a post-merge accuracy, link, and duplication
  audit because its architecture and reference set change substantially.

All three entry files currently fail the portable skill validator because
`hash` and `last_updated` are non-standard top-level frontmatter keys. Before
changing those keys, confirm that no repository consumer depends on their
location. Then normalize the files to the portable Agent Skills frontmatter
surface, using the standard `metadata` bag only for bookkeeping that has a
real consumer. Do not add provider-specific sidecars merely to satisfy one
agent implementation; these repository skills target the portable Agent
Skills core.

The current Darkmatter entry point also contains a broken relative link to
`darkmatter/lib/src/markdown/code_block.rs`. Final skill validation must cover
relative-link resolution, directly linked long-reference navigation, absence
of duplicated detail between the entry point and references, trigger-quality
descriptions, and the portable validator. Run the validator in an isolated
environment because its `PyYAML` dependency is not installed in the host
Python environment:

```sh
uv run --with pyyaml \
  /Users/ken/.claude/skills/.system/skill-creator/scripts/quick_validate.py \
  .claude/skills/<skill>
```

## Conflict map

### Stage 1: Sniff into the candidate

The frozen base has one direct conflict, `CLAUDE.md`. Because this file contains
generated GitNexus counts, defer its final reconciliation until the generated
artifact stage. Stabilize Sniff behavior before accepting the stage.

### Stage 2: Darkmatter into the Sniff candidate

The simulated merge has approximately 17 direct conflict paths. The behavioral
center is the worktree and remote-observation boundary:

- `sniff/cli/src/output/repo_json.rs`
- `sniff/lib/src/filesystem/git/remote_refresh.rs`
- `sniff/lib/src/filesystem/git/worktree.rs`
- `sniff/lib/src/filesystem/mod.rs`
- `sniff/lib/src/filesystem/repo/area.rs`
- `sniff/lib/src/remote/mod.rs`
- `sniff/lib/src/remote/provider.rs`
- `sniff/lib/Cargo.toml`

The composite must preserve Sniff's request-scoped “observe once, project
many” architecture while retaining Darkmatter's bare-repository handling,
stale-versus-corrupt worktree distinction, merge-conflict prediction,
provider URL resolution, and exact/list pull-request and CI/CD queries.

Git can auto-merge several adjacent Git, identity, network, status, and manifest
files. They still require a semantic audit because they participate in the same
observation flows.

### Stage 3: Claudine into the Darkmatter candidate

The simulated merge has approximately 29–30 direct conflict paths. The
behavioral center is file-reference and composition context authority:

- `biscuit-file/lib/src/lib.rs`
- Darkmatter `ComposeOptions` and effective context state
- `resolve_document_file_ref`
- schema formatting, resolution, and validation
- reference graph construction and validation
- Claudine preflight, lifecycle executor, harness loop, prompt, and retry logic

The composite must preserve one request-scoped `FileResolutionContext` and one
`FileReference` authority while adding Darkmatter's Git/provider context,
compose identity/cache, meta-schema support, source-aware validation, and
reference freshness guarantees.

Claudine intentionally removed and split older lifecycle test modules.
Darkmatter assertions must be ported into the current test layout rather than
resurrecting deleted test files.

`resolve_document_file_ref` has a CRITICAL GitNexus blast radius with 41 known
upstream dependents. `run_harness_loop_inner` and Sniff's `get_worktrees` are
HIGH risk. Any production edit to these symbols requires a fresh impact check
against the integration candidate before editing.

### Generated and operational artifacts

Resolve these only after production behavior and tests settle:

- `CLAUDE.md` GitNexus counts and skill hashes
- provider dispatch inventory
- snapshots
- prompt templates
- workflows and `.gitignore`
- `.claudine/memory/commits.md`

## Risk-reduction spikes

A spike succeeds by producing an explicit composite contract, a trial
resolution, and focused test evidence. Spike commits are disposable evidence;
the real merge must replay the learned resolution against the exact staged
candidate rather than merging a throwaway spike branch.

### Spike A: Sniff worktree and remote observation

Questions:

1. Can worktree projection remain metadata-only for the ordinary path while a
   targeted fallback distinguishes stale registrations from corrupt repositories?
2. Can one request-scoped remote snapshot serve aggregate output and still
   expose Darkmatter's exact/list pull-request and CI/CD operations?
3. Can conflict prediction be added without introducing an implicit fetch,
   repeated repository discovery, or per-worktree status walks?

Required evidence:

- A trial merge resolving the listed Sniff production conflicts.
- Focused unit tests for current, linked, stale, corrupt, and bare repositories.
- Tests showing aggregate projection reuses observations and makes no network request.
- Existing provider implementers compile with the extended remote trait.
- Sniff L1 tests and lint pass on the trial result.

### Spike B: Composition file-resolution and schema authority

Questions:

1. Can every source-derived file reference use the captured request context
   without recapturing CWD or repository state?
2. Can `ComposeOptions` carry both request context and Darkmatter's remote,
   cache, identity, and meta-schema controls without creating competing sources
   of truth?
3. Can schema and reference resolution retain source spans, cache identity, and
   freshness guarantees while preserving Claudine's typed error mapping?

Required evidence:

- A trial merge resolving the file-reference, compose-context, schema, and
  reference-graph conflicts.
- Tests with a process CWD different from the document/source directory.
- Tests for local, Git/provider, cached, meta-schema, and stale-reference paths.
- Darkmatter and Biscuit File L1 tests pass on the trial result.
- The relevant Claudine composition/preflight tests compile and pass.

The first two merge-strategy probes are intentionally narrower than this full
success condition. Their job is to reject unsafe mechanical strategies before
the real staged candidate accumulates opaque fixes. The remaining green
composite work is tracked in the outcomes below.

## Merge execution plan

Each stage follows the same control loop:

1. Record the exact two parents and save the pre-merge status.
2. Run `git merge --no-ff --no-commit <source>`.
3. Classify every conflict as behavioral, test, generated, or operational.
4. Resolve behavioral conflicts from documented ownership and spike evidence.
5. Review auto-merged files on the semantic audit list.
6. Run the stage gates; do not start the next merge while the stage is red.
7. Record conflict decisions and verification evidence, then create the merge
   commit preserving both parents.

Do not add `-X ours` or `-X theirs` to step 2. For the Claudine stage, compile
in this order after each seam closes: `biscuit-file`, Darkmatter library,
Darkmatter tests, Claudine library, then Claudine CLI/tests. This keeps API-shape
failures separate from orchestration failures.

### Stage gates

| Stage | Required local evidence |
|---|---|
| Sniff | `just test` and `just lint` in `sniff`; focused aggregate/worktree/remote tests |
| Darkmatter | `just test` and `just lint` in `sniff`, `biscuit-file`, and `darkmatter`; focused schema/reference tests |
| Claudine | `just test` and `just lint` in `biscuit-file`, `sniff`, `darkmatter`, and `claudine`; relevant `just test-l2` through the managed, non-focus-stealing harness; unavailable-provider retry remains deterministic |
| Final | Root dependency-aware checks for affected packages, generated-artifact drift checks, and durable CI definitions for post-merge Linux/Windows validation |

Do not run L2 tests directly through Cargo or nextest. Use the package
`just test-l2` recipe so its broker owns terminal creation and teardown without
stealing focus.

## Acceptance criteria

- The final branch contains merge ancestry from all three frozen source tips.
- Every conflict has a recorded resolution rationale.
- Auto-merged semantic seams have been reviewed deliberately.
- Sniff retains bounded, request-scoped observation with no accidental network work.
- Darkmatter and Claudine share one file-resolution/context authority.
- The managed unavailable-provider retry L2 remains deterministic on the exact
  staged candidate; a recurrence is investigated before any production edit.
- Generated artifacts are regenerated only after behavior settles.
- L1, lint, and relevant L2 gates pass on the integration host. Durable Linux
  and Windows CI coverage is present in the candidate; native results are
  post-merge hardening evidence on `main`, not a pre-merge blocker.
- The Claudine, Darkmatter, and Sniff skills match the merged behavior, pass
  portable Agent Skills validation, resolve all local links, and satisfy the
  documented progressive-disclosure limits or record a reviewed exception.
- GitNexus change detection reports only the expected symbols and flows before
  the final merge into `main`.

## Spike outcomes

The spike worktrees are detached, uncommitted diagnostic composites. They are
evidence only and must not be merged into `feat/mega-merge`.

### Spike A outcome: green composite contract

Trial:

- Worktree:
  `/Users/ken/.claudine/worktrees/rusty-biscuit/mega-merge-spike-sniff-darkmatter`
- Base: Sniff `0b3286a193899f800a97a24ee3e35c8042602cf6`
- Incoming: Darkmatter `7fb7136dca32a7b1f971b4c83bc1733bcdedebee`
- Probe: `git merge --no-commit --no-ff -X ours`

The global hunk preference was acceptable only as a disposable probe because
Sniff owns this boundary's cost model. It exposed four immediate integration
defects: an unlinked `provider_url` module, missing CI/CD trait methods, a stale
worktree-helper name, and an `Option`/collector mismatch. It also exposed the
deeper semantic conflict: one worktree helper cannot simultaneously guarantee
zero linked-repository opens and validate stale/corrupt targets.

Chosen contract:

1. Aggregate repository projection reads Git administration metadata and does
   not open linked repositories. It preserves prunable registrations and the
   zero-open performance contract.
2. Focused worktree inspection, including `full_worktree_details(true)`, opens
   registered targets. It omits an absent stale target and returns a typed error
   for an existing corrupt repository.
3. Remote selection is computed from the request's existing repository handle;
   it does not rediscover the repository.
4. Provider CI/CD methods have compatible trait defaults, so existing provider
   implementations continue to compile while capable providers override them.

Evidence:

- Sniff library: 1,788 passed, 19 skipped.
- Sniff CLI: 782 passed, 3 skipped.
- `just lint` in `sniff`: passed.
- The final diagnostic run was warning-free after the unused CLI binding was
  corrected.

Residual risk:

- The trial ran on macOS only. Linux and Windows must prove the same work-count
  and path behavior before the final merge.
- The real merge must replay the explicit aggregate/focused split without the
  probe's branch-wide `-X ours` shortcut.

### Spike B outcome: mechanical strategies rejected, seam isolated

Two detached trials used the same parents:

- Base: Darkmatter `7fb7136dca32a7b1f971b4c83bc1733bcdedebee`
- Incoming: Claudine `8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3`
- Darkmatter-authority trial:
  `/Users/ken/.claudine/worktrees/rusty-biscuit/mega-merge-spike-darkmatter-claudine`
- Claudine-authority trial:
  `/Users/ken/.claudine/worktrees/rusty-biscuit/mega-merge-spike-darkmatter-claudine-theirs`

Both variants left the same two modify/delete conflicts: Claudine had split and
deleted `composition/lifecycle/tests.rs` and
`composition/lifecycle/executor/tests.rs`, while Darkmatter had continued to
modify the monoliths. The deletions are correct. A name comparison initially
found 12 Darkmatter assertions that were not present in the split Claudine
suites:

- `ctx_scan_hint_descends_container_literals`
- `file_exists_resolves_against_launch_area_after_chdir`
- `frontmatter_reads_resolve_against_launch_area_fallback`
- `regression_conflicting_filename_prompt_dir_wins`
- `regression_path_only_under_launch_area_resolves`
- `doc_err_inside_container_literal_is_still_allowed`
- `err_inside_array_literal_when_clause_is_rejected`
- `err_inside_object_literal_value_when_clause_is_rejected`
- `err_span_inside_object_literal_key_is_rejected`
- `stack_container_literal_key_is_not_an_undefined_variable`
- `stack_undefined_variable_inside_container_literal_is_rejected`
- `top_level_undefined_variable_inside_container_literal_is_rejected`

Semantic comparison reduced that list to eight assertions that survive the
accepted Claudine context model. The two deleted monolithic files must not
return.

#### Darkmatter-authority probe

`-X ours` preserved Darkmatter's overlapping hunks. `biscuit-file` initially
failed because Claudine's `ListFormat` export survived while its `mod
list_format` declaration did not. Adding the missing additive declaration made
all 723 Biscuit File library tests and all 61 CLI tests pass.

Darkmatter then failed with 30 production compile errors. The failures were
concentrated in the predicted seam: missing request-context parameters and
helpers, incomplete `ComposeOptions` destructuring, schema resolver signature
drift, reference-graph freshness APIs, and file-reference fallback APIs. This
strategy is rejected.

#### Claudine-authority probe

`-X theirs` preserved Claudine's overlapping hunks. `biscuit-file` initially
lost Darkmatter's `span` module/export; restoring that additive facade entry
again produced 723 passing library tests and 61 passing CLI tests.

The first Darkmatter build exposed two parser-level merge scars: a missing
closure terminator in `run_with_registry` and a missing wrapper around retained
inline schema tests. After pinning those boundaries to the source versions, the
library still had 25 production compile errors and the test build had 83. The
production failures cluster into four explicit work packets:

1. **Compose context:** `provider_queries`, file-resolution fields,
   `name_coercion_keys`, and ownership of moved seed/context values.
2. **Expression/file projection:** `make_portable_relative` and the canonical
   document file-reference fallback API.
3. **Reference graph freshness:** `validate_fresh_graph`, graph accessors,
   extraction callbacks, and cached heading state.
4. **Schema assembly:** parser imports, origin/dependency attribution,
   namespace caches, example read caches, and request-scoped schema resolution.

Focused assignments for the four packets:

| Packet | Required focused evidence before the packet closes |
|---|---|
| Compose context | `ComposeOptions` construction/destructuring; provider-query state; deferred schema/name coercion; seed-state ownership; Claudine prepare/preflight context tests |
| Expression/file projection | `resolve_ctx` and path-projection tests; different-CWD and captured-environment cases; lifecycle filesystem lookup tests |
| Reference graph freshness | graph, validate, and file-tree unit tests; fresh/stale prepared heading snapshots; reference integration tests |
| Schema assembly | schema format/resolve/validate tests; origin and dependency attribution; trigger/meta-schema cases; example/import read-cache invalidation |

Lifecycle assertion destinations:

- Port `ctx_scan_hint_descends_container_literals` into
  `composition/lifecycle/executor/tests/filesystem_lookup.rs`.
- Port the seven `err`/container-literal undefined-variable assertions into
  `composition/lifecycle/tests/validation.rs`.
- Retire the four old launch-area fallback assertions. Their expected behavior
  conflicts with the accepted rule that document-authored references resolve
  source-locally. Claudine's split suite already contains stronger inverse
  assertions: a changed process CWD does not alter the captured base directory,
  a same-named launch-area file does not override a source-local candidate, and
  a path present only in the launch area does not resolve. The launch area
  remains valid for caller-supplied top-level input, not as a nested document
  fallback.

`run_with_registry` is HIGH risk in GitNexus: two direct callers, 21 affected
symbols, and compose/pipeline/transclusion execution flows. The broader
`resolve_document_file_ref` seam was previously measured CRITICAL with 41
upstream dependents. These results reject global Claudine hunk authority too.

#### Decision produced by Spike B

Keep the macro order Sniff → Darkmatter → Claudine, but resolve the final seam
symbol by symbol using the ownership matrix. Start from Claudine's request and
orchestration semantics, then deliberately add Darkmatter's cache,
origin/dependency, trigger/meta-schema, provider-query, and freshness state.
Compile after each of the four work packets above. Do not attempt a single
large conflict-resolution pass, and do not use a whole-file checkout for the
schema or reference modules.

#### Green composite replay

The Claudine-authority trial was continued symbol by symbol rather than with
another merge-strategy preference. It proved that the four work packets can be
closed with one coherent model:

| Seam | Replay decision |
|---|---|
| Request context | Keep Claudine's captured `FileResolutionContext`; expose read-only identity inputs needed by Darkmatter instead of recapturing ambient CWD, environment, or repository state. |
| Compose cache identity | Give each option one classification and one encoding. `options_hash` delegates to `ComposeOptions::compose_cache_fingerprint` so cache eligibility and cache keys cannot drift. |
| Frontmatter reads | Preserve the captured local context and attach Darkmatter's already-authorized shared remote runtime when remote reads are enabled. Frontmatter, body, and shell ternary reads therefore share authorization and denial behavior. |
| Expression projection | Preserve typed whole-value object results and the single-lookup scalar/array fast path; route string object projection through the lookup hook so explicitly configured `name_coercion_keys` may select `.name`. |
| Reference freshness | Normalize request options exactly as graph construction does before comparing identities. Resolve the target through Claudine's context, then use Darkmatter's prepared-heading cache for fragment validation. |
| Reference recursion | Retain Darkmatter's canonical-path open-frame stack and depth cap while threading Claudine's immutable request context through every recursive scalar and root-union edge. |
| Reference error transport | Preserve an existing typed `ReferenceError` when validation builds its graph; stringify only genuinely non-reference graph failures. |
| Schema assembly | Preserve Claudine's request-scoped resolution and typed errors while restoring Darkmatter's schema origin, dependency, namespace/example caches, trigger, meta-schema, and source-aware validation state. |
| Lifecycle expression traversal | Extend Claudine's preflight walker for Darkmatter's array/object AST variants by descending array elements and object values; object keys remain data. |
| Lifecycle tests | Port the eight surviving assertions into Claudine's split modules and retire the four contradictory launch-area fallback assertions. |

This replay exposed and fixed eight integration defects. The first seven were
semantic failures that a clean compile would not have found; the eighth was an
exhaustiveness failure at the Claudine boundary:

1. Cache eligibility and cache hashing used different option encodings.
2. Fresh reference graphs were rejected because validation compared an
   unnormalized request identity to a source-normalized graph identity.
3. Cross-document fragment validation bypassed the prepared-heading cache.
4. Frontmatter provider reads lost Darkmatter's authorized remote runtime.
5. Object-name coercion was bypassed by the generic evaluator's optimized
   variable path.
6. Claudine's context-aware schema recursion had displaced Darkmatter's
   canonical-path cycle/depth guard, causing cyclic `$schema` graphs to abort
   with stack overflow and misclassifying over-deep acyclic chains.
7. Reference validation stringified typed file-reference syntax and permission
   failures even though enumeration and graph construction preserved them.
8. Claudine sequence preflight did not traverse Darkmatter's new array and
   object expression literals when searching for unavailable roots.

Focused evidence on the composite:

- Cache identity migration and shared-classification tests: passed.
- Provider-network surfaces, including deny-before-network behavior: 29 passed.
- Name-coercion behavior: 4 passed.
- Prebuilt reference graph compatibility/freshness: 18 passed.
- Fresh-seam reference tests: 2 passed.
- Cached cross-document fragment resolution: 5 passed across the cache-root
  and fragment-validation groups.
- Schema reference recursion: 9 passed across direct/transitive cycles,
  scalar/root-union entry, exact depth, depth exhaustion, repeated non-cyclic
  references, and multi-hop dependency collection.
- Typed file-reference transport: 2 integration tests passed across
  enumeration, graph construction, and validation for both syntax and
  permission failures.
- Ported lifecycle/container contract: 8 passed in Claudine's split validation
  and filesystem-lookup suites.
- Biscuit File library: 723 passed, 4 skipped.
- Biscuit File CLI: 61 passed.

The Biscuit File run is the file-resolution differential oracle. It covers
captured environment/CWD behavior, repository and source precedence, explicit
trusted-external derivation, request-root containment, package/magic/vault
roots, and portable Windows path grammar on the macOS host. Platform-native
Linux and Windows gates are still required.

Full macOS composite evidence:

- Darkmatter: 6,187 of 6,188 passed under full non-fail-fast load. The sole
  failure was the pre-existing slow cleanup characterization timing out at 30
  seconds; it passed alone in 9.1 seconds. Darkmatter library, CLI, and DMLS
  lint passed.
- Claudine catalog types: 21 passed.
- Claudine library: 3,963 passed, 7 skipped.
- Claudine contract: 47 passed, 5 skipped.
- Claudine CLI: 2,315 of 2,316 passed under full non-fail-fast load. The sole
  failure was an unrelated context-report width test timing out at 30 seconds;
  it passed alone in 24.2 seconds. Claudine library, contract, CLI, generator,
  error-transport guards, and lifecycle-doc-facet lint passed.
- Biscuit File library/CLI lint passed.

The two full-load timeouts are performance flakes, not green full-suite runs.
They must remain visible in the replay ledger and should be rechecked on the
exact staged candidate. They do not weaken the focused semantic evidence.

GitNexus change detection over the entire diagnostic composite reported 1,058
changed files, 11,971 changed symbols, 70 affected processes, and CRITICAL
risk. That breadth is expected because the disposable worktree contains both
completed source streams, not just the hand-authored seam fixes. It confirms
that whole-composite change detection is too coarse to certify the merge. Run
change detection after each real stage and once more against `main` on the
final candidate, and review the resulting high-risk symbol list before each
merge commit.

The real merge must replay these decisions in the same packet order. The
diagnostic files are not a patch source, and a clean compile is not sufficient:
each of the eight regression groups above is a mandatory focused gate.

### Spike C outcome: unavailable-provider retry report disproved

Trial:

- Worktree:
  `/Users/ken/.claudine/worktrees/rusty-biscuit/mega-merge-spike-claudine-retry`
- Tip: Claudine `8c7a7a8a57d6eebba2e7007df2a6523d9679bbb3`
- Command: managed `just test-l2` filtered to
  `level2_lifecycle_retry_to_an_unavailable_provider_matches_direct_selection`

The isolated fixture passed four consecutive runs. It used the managed L2
broker and its fixture-scoped `PATH`; no second provider child was spawned.
Together with existing L1 scalar/list refusal tests, this disproves the draft's
claim of a repeatable frozen-tip defect. The merge gate is now recurrence
detection on the exact candidate, not a speculative retry rewrite.

### Additional spikes considered

The following additional work provides useful evidence without changing the
selected macro path:

1. **Conflict replay packet:** retain the table above as the symbol-level
   resolution ledger and require focused gates after each packet.
2. **Freshness mutation matrix:** after a graph is built, mutate a child,
   descendant, heading, schema dependency, and option identity independently;
   each must invalidate only through its documented channel.
3. **Authorization parity matrix:** exercise the same authorized and denied
   provider read from frontmatter, body interpolation, and shell ternary
   evaluation; every surface must share allow/deny and cache behavior.
4. **Cross-platform path fixture:** run the captured-context oracle on native
   Windows and Linux, with special attention to drive-relative paths, UNC
   paths, separator normalization, symlink/reparse containment, and worktree
   administration paths.
5. **Generated-artifact quarantine:** compare generated outputs before and
   after the behavioral replay, regenerate once, and verify that no generated
   file is being used to smuggle a semantic conflict resolution.

Items 2 and 3 are now substantially covered by the focused composite tests;
their remaining value is as explicit replay matrices on the exact staged
candidate. Item 4 cannot be closed on this macOS host and remains a CI gate.
