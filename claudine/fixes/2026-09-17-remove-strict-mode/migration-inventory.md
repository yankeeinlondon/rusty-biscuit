# Remove Strict Mode: Migration and Verification Inventory

Design completion work, 2026-09-19. This is an inventory for review, not an
implementation plan or passing implementation evidence. Read it with
[design.md](design.md), [design-contracts.md](design-contracts.md), and the
[spike results](spike-results.md).

## Evidence and risk

Source baseline: `dec4a6f1ed3164ba619982635f512272c2860725`. GitNexus was
explicitly bound to this worktree as `rusty`; its indexed commit matched HEAD.
Its `EvaluationLookup` upstream impact reported **HIGH risk**, 17 direct
relationships and 25 total affected nodes. Zero reported processes does not
establish zero process impact. Some lifecycle context results still pointed at
unrelated renderable symbols; the graph is not a complete impact inventory.
Before implementation, repeat impact on each edited seam and resolve missing
or malformed results through source verification.

A repository-wide Rust source scan finds **21 actual lookup implementations**,
plus **two rustdoc examples**. The specification's 17 is historical graph
evidence, not the implementation checklist. Nineteen implementations are under
`src`; two are library integration fixtures. Four fixture implementations are
absent from the graph's 17-result set. This inventory supersedes the counts in
the old plan and earlier investigation checkpoints.

## Lookup-by-lookup disposition

Paths in this table are relative to the repository root. DM means
`darkmatter/lib/src/markdown/compose`; CL means `claudine/lib/src`.
Every forwarding implementation must forward rich resolution, descriptor access,
resolved-value formatting, and both resolution-context accessors where present.

| # | Implementation and location | Migration or compatibility disposition |
| --- | --- | --- |
| 1 | `EffectiveState`, DM `context/effective_state.rs:352` | Shared namespace/document resolution; remove bare-context fallback from underlying lookup as well as evaluator entry. Preserve name coercion and existing captured context/environment. |
| 2 | `ResolvingLookup`, same file `:411` | Forward the richer contract without mapping failures to missing data. Keep attached file context and existing remote-read policy. |
| 3 | `FrontmatterSeedState`, DM `frontmatter_interpolation.rs:96` | Preserve staged seed visibility and pending-property dependencies. Resolve `doc`/`ctx`/`env` before document roots, including exact namespace roots; remove fallback. Preserve seed coercion and captured file origin. |
| 4 | `ShortcutLookup`, DM `conditions.rs:345` | Delete the explicit fallback to `ctx.{path}`. Keep demand-driven explicit context capture and source resolution; descriptors must not trigger that capture. |
| 5 | `CtxLookup`, DM `expression/ctx.rs:82` | Remain a context-only lookup. Bare non-context paths are missing; preserve explicit `ctx.*` and existing whole-`ctx` behavior. Do not add document or lifecycle globals. |
| 6 | `LayeredLookup`, DM `subtree.rs:213` | Implement checked declarations/runtime association and operation-local lazy cache. Reserved namespaces precede injected names; unavailable globals cannot fall through. All descendants of one subtree share its session. |
| 7 | `FsLookup`, DM `expression/catalog/mod.rs:554` | Keep lightweight default rich resolution for ordinary data; retain filesystem context fixture. Add no host observations to descriptors. |
| 8 | `FixtureLookup`, same file `:885` | Keep ordinary-data default; retain catalog example assertions and context behavior. |
| 9 | `MapLookup`, same file `:1078` | Keep ordinary-data default. Missing data must exercise null semantics, not closed-world checking. |
| 10 | `FixtureLookup`, DM `expression/semantics.rs:810` | Keep ordinary-data default and semantic catalog tests. |
| 11 | `TestLookup`, DM `expression/mod.rs:833` | Keep ordinary-data default. Add focused richer fixtures for unavailable/null distinctions rather than teaching this map lifecycle policy. |
| 12 | `SizedLookup`, CL `composition/looping/actions.rs:248` | Forward the complete contract from the borrowed trait object; forwarding only `get` and `get_string` is insufficient. |
| 13 | `LoopExpressionLookup`, CL `composition/looping/expression.rs:139` | Register the existing `_loop_*` ambient values explicitly as loop globals. Preserve their names and existing observation timing. Missing explicit namespace fields cannot fall through to frontmatter. Do not inject lifecycle policy on this surface. |
| 14 | `MapLookup`, CL `composition/looping/actions/tests.rs:12` | Ordinary-data default; keep loop/action fixtures. |
| 15 | `SourceExpressionLookup`, CL `composition/sequence/expr.rs:75` | Treat the per-item overlay as the document layer, beneath reserved namespaces. Explicit `doc` selects that effective document. Preserve source file context and current environment policy. Replace duplicate interpolation traversal with shared prepared inputs. |
| 16 | `EventMetaExpressionLookup`, CL `dispatch/expression.rs:85` | Preserve the event's existing document projection, including extras and tool payloads. Namespace dispatch comes first; do not reinterpret event metadata as lifecycle globals or introduce implicit context. |
| 17 | `EventMetaConditionLookup`, same file `:169` | Forward the event lookup and explicit context lookup through the shared contract; retain event working-directory context. |
| 18 | `MapLookup`, CL `composition/lifecycle/tests/action_shape_control.rs:1410` | Retain as an ordinary-data fixture; use an explicit binding fixture where the test means a lifecycle global. |
| 19 | `EmptyLookup`, same file `:1418` | Retain; missing bare data returns null. It must not stand in for an unavailable declared global. |
| 20 | `Lookup`, `darkmatter/lib/tests/more_is_more_literals_and_indexes.rs:11` | Retain explicit `ctx.limit` fixture and owned resolution context; default richer method remains sufficient. |
| 21 | `Lookup`, `darkmatter/lib/tests/predict_conflicts.rs:147` | Retain ordinary missing data and borrowed resolution context. Activation capability rejection is separate from this ordinary read-side function test. |

The two `SimpleLookup` rustdoc examples in DM `expression/mod.rs` remain valid
with the additive method default. Update strictness examples and comments when
the relevant behavior changes; do not create a second trait migration for them.

## Evaluation entry points and duplicate checks

| Surface | Current source | Required disposition |
| --- | --- | --- |
| Lifecycle event interpolation | `claudine/lib/src/composition/lifecycle/executor.rs`, `resolve_string_value` | Prepared subtree plus a fresh checked session; remove strict builder and surviving-span rejection. Preserve successful literal bytes. |
| Lifecycle shell approval | `claudine/lib/src/composition/preflight.rs` | Same prepared language, explicit approval scope, resolve once and retain terminal approved bytes. |
| Sequence graph shell approval | `claudine/lib/src/composition/sequence/preflight/mod.rs` | Same language and approval scope across primary, setup, teardown, and referenced prompt commands. No lexical group computation here. |
| Sequence task values | `claudine/lib/src/composition/sequence/task/mod.rs` | Same subtree API, actual task scope and preserved value metadata. |
| Lifecycle direct guards/control arguments | `executor.rs`, `lifecycle/action_shape.rs` | Shared expression preparation and result-schema validation; remove local root/`err`/context AST walkers and result re-evaluation. |
| Sequence source expressions | `sequence/expr.rs` | Replace `evaluate_whole`/`render_interpolated` parser traversal with shared authored-mode preparation. Keep Claudine's item iteration and error context. |
| Loop and hook conditions/actions | `looping/expression.rs`, `looping/actions.rs`, `dispatch/expression.rs` | Rich evaluator migration even though these are not subtree callers. Keep loop/hook orchestration and scope ownership. |
| Lifecycle preparation | `composition/prepare.rs`, `lifecycle` validation | Shared passive validation of all authored branches; no eager event provider invocation, custom root allowlist, or custom surviving-brace check. |

Inventory the removal by symbol and responsibility, not just `.strict()` search:
`SubtreeStrictness`, strictness builder/convenience arguments,
`is_known_variable_root`, strict-root validation, `ctx_scan_hint` and associated
variable-path traversal, lifecycle undefined-root and event-`err` walkers,
surviving-span guards, and typed-result interpolation retries. Shape parsing of
actions remains Claudine orchestration; expression parsing and argument/result
schema checks do not.

## Executable-value transfer inventory

Each row must carry the envelope defined in C4. A plain JSON clone cannot retain
deferred stages or inherited policy. Existing source-reference metadata is
retained alongside policy metadata, not replaced by it.

| Boundary | Required transfer and oracle |
| --- | --- |
| Authored frontmatter into prepared composition | Keep authored spans, excluded lifecycle subtrees, captured source context, and pending stages. Ordinary FM1 completion does not consume deferred DM2. |
| `CallerInputLayers` / `CallerInputRecords` | Preserve immutable raw overrides and launch-time file-resolution origin across canonical preparation. Apply new policy to selected values without reanchoring file references. |
| Effective document layering and `layered_set_overrides` | Merge values and sparse metadata in the same precedence order. Replacement discards replaced source metadata; destination restrictions remain. |
| `RuntimeState::set`, `set_batch`, prior-value results | Validate every candidate before publishing value and metadata inside the same lock. Failed batches leave both unchanged. Returned prior values retain their old metadata. |
| `RuntimeState` / `RuntimeSnapshot` / `from_snapshot` | Snapshot and restore values and metadata together, including outputs and pending stages. Rebuilding after failure does not consume the retained snapshot. |
| `StackExecutionContext::live_frontmatter` | Replace the raw live map with an envelope-aware view. Runtime state and live frontmatter must represent the same committed generation; do not maintain an independently mutable metadata sidecar. |
| `StackExecutionContext::with_private_cells` | Redirect both private runtime state and private live frontmatter, including metadata. Parallel tasks cannot write parent or sibling policy/state. |
| Stack-local working state, `set`, and `unset` | Later actions see the committed envelope. Selection materializes ancestor restrictions; removal deletes the matching subtree metadata without touching siblings. |
| `proxy.with` evaluation → `ProxyHandoff` → `DocumentOverlay` | Evaluate the complete candidate against one pre-write view, validate it, then publish. Overlay values retain origin/stages through handoff; diagnostics keep the original typed cause. |
| `PreparedDocument::refreshed` and retry/resume | Retain the overlay envelope, reapply destination schema restrictions, and create new runtime sessions. Do not serialize/reparse completed literals or reuse lazy caches. |
| Sequence step parameters, overrides, and just-in-time task preparation | Carry envelopes through each merge into child preparation; preserve caller file origins and existing override precedence. |
| Group variable resolution and child prompt overlay | Evaluate variables before entering the new group's scope; forward selected values with metadata. Known members receive the lexical binding only after resolution. |
| Parallel task outputs and declaration-order merge | Keep each private envelope until the existing merge order commits it. Rebase metadata when outputs/arrays are reconstructed. |
| Loop `next_frontmatter` and iteration refresh | Preserve terminal values and pending work separately; a new event captures its normal snapshots and creates new caches. |
| Primary/setup/teardown shell approval storage | Retain the resolved command artifact and use its exact bytes at execution. Task stack parsing must not turn it back into authored interpolation. |
| Error recovery / initialization catch | Preserve source and destination restrictions plus execution prohibitions. Error projection is terminal data; recovery actions remain subject to passive validation and the initialization shell backstop. |
| Provider output, event/error snapshots, messages, audio, terminal rendering | Construct or export terminal data. Text containing braces is not an implicit new program. Any later executable use must carry the original stage contract rather than infer one from string contents. |

No new persisted executable-value format is introduced. Current handoff,
snapshot, and overlay boundaries are in-process. Audit every serde conversion
of these carriers during implementation: a transport that must later resume
execution needs a versioned metadata representation and round-trip tests before
it can strip JSON. Ordinary external output remains plain data. Read-only JSON
views cannot be reimported through an executable internal API.

## Schema and trigger migration

| Artifact family | Disposition |
| --- | --- |
| `darkmatter/schemas/darkmatter.yaml` and `partials/` | Single global entry point; compile transitive imports and use only its resolved `doc` definition as the document baseline. Replace legacy baseline embedding and context extraction. Exclude the registered global source from document auto-application. |
| `darkmatter/docs/schemas/expression-functions.yaml`, generic schema-definition metadata | Classify catalog and authoring metadata explicitly; preserve documented paths until references and parser support migrate together. |
| `darkmatter/docs/schemas/claudine.yaml`, `claudine-types.yaml`, `env.yaml` | Move Claudine-owned definitions to `claudine/schemas`; add the single binding catalog and use-site policies. No Claudine definitions embedded in DMLS or generic Darkmatter. |
| Global runtime consumers | Replace `current.ctx.*` with `current.*`, remove `current.env.*`, use `env` for environment access, and wire `err`/`tracking` through shared global schemas. Preserve event availability and capture timing. Keep lifecycle execution in Claudine. |
| Existing `claudine/schemas/review.yaml` and root `schemas/feature-review.yaml` | Classify exports deliberately; preserve authored root alternatives. Do not rewrite feature/fix alternatives into independent optional properties. |
| Trigger parser, grammar, matcher, lint and assembly | Canonical kind and C6 grammar in one Darkmatter implementation. Reject old kind/legacy shape with migration diagnostics; do not infer intended logic. |
| Three `darkmatter/tests/fixtures/schema-triggers/schemas/*.trigger.yaml` files | Migrate the old `all`/`none` rules to equivalent C6 expressions. Preserve absence/type guard semantics; merely renaming `kind` is insufficient. |
| Inline trigger YAML in library/CLI/DMLS tests | Re-run the source inventory and migrate active cases; retain deliberately invalid legacy inputs only as rejection tests. |
| Root `schemas/memory.yaml` | Currently a canonical-kind stub with `match: null` and no payload. It must be completed with an author-selected purpose or removed from active discovery; do not silently treat it as a valid no-op. No memory activation policy is invented by this fix. |
| Existing whole-file references and type imports | Exported whole-file targets remain explicit; types-only references require named imports. Relative imports stay rooted at the declaring source after moves. |
| DMLS overlay, diagnostics, hover and completion | Replace separate trigger JSON validation and explicit-only simplified assistance with one effective semantic view and dependency-aware suppression. |
| Current schema docs, CLI help, READMEs and skills | Update paths, kinds, Boolean grammar, optional-value language and ownership together with implementation. Historical feature/fix documents stay historical. |

The current trigger matcher supports type guards, `$path`, and Boolean
combinators in addition to top-level OR. Migration must retain these capabilities
under the explicit `document` predicate in C6. For an old top-level list, wrap
the translated arms in a single explicit `group:`; for an old mapping, translate that
mapping as one document predicate. This preserves logic while making the new
outer AND explicit. The kind rename supplies the grammar boundary; no legacy
OR-list detection heuristic or anonymous nested-list group syntax is supported.

The root memory stub is a repository-content cleanup prerequisite for an
all-repository activation acceptance run, not a reason to weaken malformed-rule
handling. It does not block implementation of the generic parser or isolated
tests.

## DRY and ownership audit

| Concern | Single authority | Retained caller work and reason |
| --- | --- | --- |
| State lookup and namespace precedence | Darkmatter resolver | Claudine owns document/overlay/runtime precedence and mutation transactions. |
| Context requirements | Darkmatter prepared AST metadata | Claudine chooses capture epochs and supplies existing snapshots; Sniff observes requested activation facts. |
| Binding names/types/availability | Generic Darkmatter descriptors; one Claudine YAML catalog | Claudine constructs runtime values and selects opaque event/task/approval scopes. DMLS does not reconstruct the catalog. |
| Parsing and evaluation | Existing Darkmatter parser, scanner, evaluator and function registry | Claudine decides when a stack/action runs; loop and sequence scheduling are not language implementations. |
| Passive/schema/result validation | Darkmatter semantic model | Claudine supplies required runtime schemas and expected action-result types. |
| Restrictions and provenance operations | Darkmatter envelope/policy logic | Claudine keeps atomic storage and shell-action backstops. An action prohibition and an expansion restriction cover different effects. |
| Semantic schema assembly | Darkmatter definitions/order/dependency model | DMLS schedules refresh and translates spans to LSP positions. |
| Errors | Typed Darkmatter causes | Claudine adds lifecycle/action/document context; CLI renders using TerminalRenderable components. Runtime `err` JSON is a projection, not the owned error. |
| Generated and dynamic definitions | Same Darkmatter semantic bundle | Runtime uses generated constructors, editor loads YAML; different delivery paths, same semantics and generation check. |

## Acceptance coverage map

These are required implementation oracles, not claims of tests already passing.

| Spec AC | Required coverage |
| --- | --- |
| 1 | Public API compile/update checks and source scan remove strictness/root-membership hooks. |
| 2–4 | Every production lookup path: namespace collision, explicit `doc.*`, missing bare value, unavailable global, and no context fallback. |
| 5–7 | Shared prepared validator used by all entry points; no Claudine root/`err` AST walkers; descriptors invoke zero lazy providers. |
| 8 | All-branch passive violations versus runtime short-circuit errors; complete triple-brace/escape outputs never rescanned after transfer. |
| 9 | Atomic message/proxy/set batches; failed last member publishes nothing; approval/execution byte equality for primary/setup/teardown and child prompts. |
| 10 | Unknown-property advisory deduplicated by source span; declared absent type retained; unavailable versus execution-dependent bindings; optional schema absence. |
| 11 | Unmodified `prompts/implement.md` reaches authored route error; hermetic CLI fixture records no provider process or audio spool. |
| 12 | Active docs/skills/help inventory corrected with implementation; no fallback advice to legalize missing properties. |
| 13 | This 21-entry inventory, contracts and human rulings reviewed before replacement planning. Historical graph counts are not acceptance counts. |
| 14 | DRY table above checked against final implementation; all retained policy/orchestration boundaries remain justified. |
| 15 | Applicable package `just test`, `just lint`, and existing `just test-l2` gates via Nextest; reuse qualifying OS evidence under repository policy. |
| 16 | Library downcasts typed cause after direct, proxy, lifecycle, catch and CLI wrapping; `err` projection does not replace cause. |
| 17 | Same expression fixtures pass through ordinary frontmatter and caller-schema lifecycle preparation/evaluation; action result types checked centrally. |
| 18 | Array/key-value descendant bans, origin-to-unrestricted and unrestricted-to-restricted moves, replacement/retry, initialize expansion and action backstop. |
| 19 | Launch/Completion required-property policy preserved; missing expression lookup remains null; required active schema failure blocks execution at its boundary; recovery retains restrictions. |
| 20 | Deterministic generated Rust plus semantic equality for YAML types, unions, descriptions, bindings, scopes, restrictions and portable origins. |
| 21 | Scope/precedence/dedup matrix; all predicate families; source-relative and workspace paths; schema-directory creation and explicit external SCHEMA_DIR; qualitative responsiveness observations. |
| 22 | Dependent diagnostic/hover/completion suppression and repair; missing import versus optional removal; unknown activation scope; interleaved independent refreshes and stale success/failure rejection. |

Promote the seven spike tests onto production contracts and replace assertions
that characterize today's trigger defects. Add transfer tests at the real proxy,
parallel-task, retry and approval boundaries; the spike only proved a subset.
Protocol-injected watcher events do not prove native event delivery: verify new
directory registration, delete/recreate, case/path identity and external roots
on macOS, Linux, native Windows and WSL2 using the existing OS evidence routes.
No new CI matrix, latency threshold, live provider, or focused UI is required.

The separate draft
[Darkmatter lifecycle extraction](../../../darkmatter/features/2026-09-22-lifecycle-events/spec.md)
overlaps these interfaces. This fix retains its approved ownership boundary;
it does not introduce that draft's lifecycle engine or `composed` alias. The
independent reviewer should check compatibility and sequencing, not assume the
other draft has already replaced this specification.
