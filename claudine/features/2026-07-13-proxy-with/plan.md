---
created: 2026-07-16
phase: 1
total_phases: 14
agent: claude/default
yolo: "true"
spec: ./spec.md
packages:
    - claudine
    - claudine-cli
source_files_during_phase_1:
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/control_dispatch.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/proxy.rs
    - claudine/cli/tests/composition_seams.rs
    - claudine/cli/tests/level2_lifecycle_control.rs
    - claudine/lib/src/composition/lifecycle/actions.rs
docs_updated_during_phase_1:
    - claudine/docs/providers/dispatch-inventory.json
    - claudine/features/2026-07-13-proxy-with/plan.md
docs_created_during_phase_1:
    - claudine/features/2026-07-13-proxy-with/notes/baseline.md
skills_files_updated_during_phase_1: []
---

# Execution Plan — Canonical Document Handoffs and Transient Proxy Frontmatter (`with:`)

## How to read this plan

Each phase ends with a **validation checkpoint**. Do not advance past a
checkpoint that regresses the recorded baseline; the final checkpoint must be
fully green unless the owner explicitly waives a named external failure. `just`
recipes run from the `claudine/` package area unless stated otherwise
(`just test` = L1, `just test-l2` = L2, `just lint`).

Tasks marked **[∥]** may run concurrently with the sibling task or phase named
in the marker. Everything else is dependency-ordered.

File anchors below were captured from `HEAD` on branch `claudine` at plan time.
Treat them as starting points, not as guarantees — re-locate before editing.

Follow the Claudine test-placement rule while adding L1 coverage: keep small
tests inline, but move them to a sibling `tests.rs` (or an existing sibling test
module) once the production file is about 800 lines or the inline test module is
about 300 lines. `claudine-cli/tests/test_placement.rs` is the structural gate.

### Execution preconditions and completion signal

- The spec is reviewed but still has `status: draft`. Do not start Phase 1
  repository changes, including test scaffolding and drift guards, until the
  owner marks the spec ready for implementation or explicitly authorizes work
  against the reviewed draft. Read-only discovery may continue.
- The two draft dependency specs have explicit integration gates below. A seam
  is not a substitute for satisfying a declared dependency at sign-off.
- The feature is complete only when the routed and direct forms of the
  motivating multi-phase document execute the same deterministic fake-provider
  fixture, acceptance criteria 1–30 map to passing tests, both drift guards are
  at their final baselines, and all three package-area checks are green.

### Phase index

| Phase | Exit outcome |
|---:|---|
| 1 | Baseline, deterministic failing reproduction, resolver seam, drift guards |
| 2 | Typed transitions and explicit state ownership |
| 3 | Validated `proxy.with` authoring surface and diagnostics |
| 4 | Atomic source-time evaluation into a typed request |
| 5 | One canonical document-preparation service |
| 6 | One coordinator owns active-document transitions |
| 7 | Safe target `initialize`, stabilization, and reread |
| 8 | Overlay precedence, semantics, and lifetime |
| 9 | Target-specific launch rebuilding and shell approval |
| 10 | Loop ownership follows the stabilized active document |
| 11 | Canonical retry/resume and session compatibility |
| 12 | Typed, redacted transition failures and status |
| 13 | Cross-platform direct/proxy equivalence matrix |
| 14 | Documentation, skill drift, and feature completion |

---

## Grounding: what the code actually looks like today

These facts were verified against the tree and shape the phase ordering.

**There are two parallel initialize-proxy channels, not one.**

- Library loop engine: `lib/src/composition/looping/engine.rs:415-464` returns
  the target out-of-band via `.with_init_proxy_target(resolved)` into
  `LoopExecutionResult::init_proxy_target` (`lib/src/composition/looping/types.rs:189`).
- CLI single-document pipeline: `cli/src/commands/wrap/composition/pipeline.rs:1130`
  declares its own `let mut init_proxy_target: Option<PathBuf> = None`, sets it
  at `:1175`, returns it at `:1215`, and feeds it to the harness loop at `:1407`
  as `initial_proxy_target` (`loop_control.rs:79`).

This is exactly the "second optional proxy-target channel" the spec bans in R1.

**The banned second composer is real and half-migrated.**
`cli/src/commands/wrap/harness_orch/prompt.rs:190` (`materialize_harness_prompt`)
hand-rolls compose options and calls Darkmatter `compose_with` at `:218` and
`:260`, with its own `apply_rematerialize_inputs` (`:70`) and
`rematerialize_compose_options` (`:93`). `preflight_proxy_target` (`:124`)
duplicates the option-building a third time (`:144-159`). The `Inline` arm
(`:281`) already delegates to `prepare_inline`, so the file is partly migrated.
Sanctioned composer sites are `lib/src/composition/prepare.rs:198`
(`prepare_direct`) and `:373` (`prepare_inline`).

**Proxy identity is swapped inside the attempt harness.**
`cli/src/commands/wrap/harness_orch/control_dispatch.rs:209-213` mutates
`prompt_state.source_path` / `original_ref` and clears the flow-control fields,
then re-enters at `attempt: 1`.

**Three pre-existing defects the refactor should absorb (do not fix ad hoc):**

1. `control_dispatch.rs:181` calls `claudine::harness::resolve_harness_path`
   **directly**, bypassing `resolve_proxy_target` (`lib/src/composition/lifecycle/control.rs:238`)
   and therefore skipping its `is_file` existence check. A `failure: proxy(missing.md)`
   resolves to a nonexistent path and fails later and differently than the same
   proxy at `initialize`. Phase 1 fixes this as the seam; Phase 6 makes it
   structurally impossible.
2. `looping/engine.rs:439` calls `proxy_handoff_allowed(&[prompt_path], &resolved)`
   with a **single-element chain slice**, so the loop route cannot see multi-hop
   history. The invocation-wide chain in Phase 2 subsumes this.
3. `proxy.target` loses whole-value typing: `executor.rs:1148` routes it through
   `render_message` (`:887-899`), which collapses to `scalar_string`. `with:`
   values must **not** reuse that path — they must reuse the typed rule.

**The typed whole-value rule already exists and must be reused, not rebuilt.**
`action_value_to_expr` (`lib/src/composition/lifecycle/action_shape.rs:329-367`)
uses `darkmatter::markdown::compose::expression::ExpressionFinder::find_all_plain`,
keeps a typed `Expr` when exactly one span covers the whole trimmed string, and
maps YAML numbers/bools to literals. DM2 evaluation is
`darkmatter::markdown::compose::subtree::SubtreeCompose` (`executor.rs:544-555`)
followed by `reject_surviving_spans` (`executor.rs:1258-1275`).

**Terminology correction for the spec's readers.** Composition lifecycle has
**7 signals** (`LifecycleSignal::ALL`, `lib/src/composition/lifecycle/mod.rs:299`:
`Initialize, Start, Success, Blocked, Failure, Finalize, Loop`). The "16 events"
are provider hook events (`AgenticEvent::ALL`, `lib/src/events/agentic_event.rs:51`)
and are a different axis. Do not conflate them when writing the stage matrix.

**Loop vs single is decided at `cli/src/commands/compose/prep.rs:462`** — the
presence of a `loop:` block via `resolve_loop_config`
(`lib/src/composition/looping/config.rs:61`). Today that decision is made about
the *router* before its `initialize` proxy fires. That is the motivating bug.

### Dependency integration gates — read before Phase 1

The spec's `depends_on` names two specs that are **`status: draft` and
unimplemented**:

- `claudine/features/2026-07-13-file-resolution/spec.md` — owns the shared resolver
  that R2 requires the coordinator to call.
- `claudine/features/2026-07-13-error-propogation/spec.md` — owns the typed
  diagnostic transport that R10 requires.

The dependency order is linear, not cyclic: file resolution declares
`depends_on` error propagation, while error propagation lists file resolution
only as `related`. The expected external order is therefore typed error
propagation → unified file resolution → the coordinator's final resolver
integration. This feature may use the narrow seams below while either
dependency is pending, but it must not reverse that ownership or recreate
either dependency locally.

After this feature's readiness precondition is satisfied, independent parser,
evaluator, state-model, and test-fixture work may proceed while these dependency
specs remain drafts. Phase 1 establishes narrow seams so that work does not
duplicate either dependency:

- **File resolution**: consolidate on the single existing owner
  (`resolve_proxy_target`, `control.rs:238`) and let the coordinator be its only
  caller. When the file-resolution feature lands, it replaces that one function
  body — not every call site.
- **Error propagation**: keep concrete typed errors (`CompositionError` /
  `HarnessError`) flowing to the render boundary and add **no new `eyre!`
  stringification** on any transition path. When error-propagation lands, it
  changes rendering/registry, not the transport.

The gates are explicit:

- **Before Phase 6 commits coordinator-owned file resolution**, the
  file-resolution spec must either be implemented or the owner must approve the
  existing `resolve_proxy_target` adapter as a temporary bridge. The completed
  coordinator must delegate to that spec's `biscuit_file::FileReference`-based
  authority; acceptance criterion 4 cannot be signed off while two resolver
  semantics remain possible. If the dependency is still pending at Phase 13,
  either implement it or amend this spec's `depends_on` with owner approval.
- **Before Phase 12 closes typed error transport**, the error-propagation spec
  must either be implemented or the owner must approve a documented integration
  boundary that preserves concrete errors to the renderer. Acceptance criteria
  28–29 cannot be deferred past feature completion.

If either dependency lands mid-flight, its shared authority replaces the seam;
do not absorb or reimplement the dependency's broader scope in this feature.

---

## Phase 1 — Baseline, dependency seams, and drift guards

Goal: make current behavior measurable, make the drift observable, and pin the
two dependency seams before any refactor starts.

- [x] Record the exact starting commit and existing worktree changes, then run
      `just test`, `just test-l2`, and `just lint` from `claudine/` before any
      feature production/test edits. Write pass/fail counts and pre-existing
      failures to `notes/baseline.md`. If unrelated worktree changes make the
      result unattributable, use a clean disposable worktree at the recorded
      commit; never reset or overwrite the user's changes.
- [x] Add an ignored L2 reproduction of the motivating bug in
      `cli/tests/level2_lifecycle_control.rs`. Use a self-contained temporary
      router/loop-target fixture and the deterministic fake provider used by the
      lifecycle suite; do not invoke a live Claude/Codex/Gemini service. Compare
      the routed run with direct execution and assert the same iteration count,
      phase mutations, and target `initialize` count. Document why it fails on
      the baseline and re-enable it in Phase 10. Keep the shipped
      `prompts/implement.md` command as a manual smoke case, not a CI dependency.
- [x] Fix the resolver bypass at `control_dispatch.rs:181`: replace the direct
      `resolve_harness_path` call with `resolve_proxy_target` so the failure-stack
      proxy gets the same existence check as the initialize route. Add focused
      L1 coverage that both dispatch paths call the shared resolver and reject a
      nonexistent regular path before target adoption. Exact cross-route typed
      identity belongs to Phase 12, after the error-transport dependency gate.
- [x] **[∥]** Add the production `compose_with` allowlist drift guard as a test
      under `cli/tests/`, modeled on the existing `dispatch_inventory.rs`
      site-level guard. It must enumerate call sites by stable module + enclosing
      function identity, not line number or a broad substring ban, and ship with
      today's explicit baseline: `composition::prepare::{prepare_direct,
      prepare_inline}`, `system_prompt::prepare::compose_prompt_markdown`,
      `wrap::overlay::materialize_passthrough_harness_seed`, and the two arms of
      `harness_orch::prompt::materialize_harness_prompt` with an expected count
      of two (the `Passthrough` and `Compose` arms). The guard fails on a new
      semantic owner or an unexpected count.
- [x] **[∥]** Add a companion guard that fails on any **new** `Option<PathBuf>`
      proxy-target return channel. Use the same Rust-aware structural scan and
      seed it with today's two semantic owners:
      `LoopExecutionResult::init_proxy_target` (`looping/types.rs:189`) and
      `route_initialize::init_proxy_target` (`pipeline.rs:1130`). Phase 6
      shrinks this list to zero; Phase 10 verifies it stays there.
- [x] Correct the stale placement docs on `Proxy`, `Retry`, `Resume`, and
      `Defer` in `lib/src/composition/lifecycle/actions.rs`. `is_valid_for`
      (`actions.rs:195-200`) makes every control except `Skip` universally valid;
      the enum comments still claim event-specific placement. Preserve useful
      action semantics, but remove the incorrect validity claims. Code is
      authoritative; the comments drifted.

**Validation checkpoint 1**
- `just test` and `just lint` green in `claudine/`.
- Both drift guards pass against their seeded baselines.
- The focused resolver-path tests pass; the motivating-bug L2 test is present
  and ignored with a comment naming Phase 10.

---

## Phase 2 — Typed transitions and the four state layers

**[∥ Phase 3]** — Phase 3 owns parser/diagnostic work while this phase owns the
new state/transition types. Agree on shared module exports and error payloads
before working concurrently.

Goal: land the provider-neutral vocabulary in `claudine::composition` with no
behavior wired to it yet. Nothing in this phase may perform process, terminal,
filesystem I/O/resolution, or provider-adapter work (R1); carrying a resolved
`PathBuf` value in a handoff type is expected.

- [ ] Define `DocumentTransition` in `lib/src/composition/` with the required
      semantic surface: `Continue`, `Retry`, `Resume { session, message }`,
      `Proxy(EvaluatedProxyRequest)`, `Complete`, and a typed abort path. Rust
      names may differ from the spec; the surface may not. The abort path may be
      generic or use a source-preserving envelope/coordinator outcome, but it
      must not force CLI-only errors into `CompositionError`, erase them into a
      string/`eyre!`, or create a library-to-CLI dependency.
- [ ] Define the two-stage handoff types: `EvaluatedProxyRequest { target: String,
      overlay: IndexMap<String, serde_json::Value>, provenance: ProxyProvenance }`
      and `ProxyHandoff { authored_target, resolved_target: PathBuf, overlay,
      provenance }`. The type system must make it impossible to construct a
      `ProxyHandoff` without a resolution step.
- [ ] Define `ProxyProvenance` carrying source path, lifecycle signal,
      action/property location (reuse the dotted `"{event}.stack[{i}].action[{j}]"`
      form from `preflight.rs:280-286`), and the proxy chain.
- [ ] Define the four ownership layers as distinct types (R2): invocation state
      (immutable inputs plus a coordinator-owned run ledger), handoff state,
      prepared document, and active-document execution state. Populate them from
      the spec's field inventories. Cross-check against
      today's carriers so nothing is dropped: `CompositionPrepContext`
      (`cli/src/commands/wrap/composition/prep_context.rs:38`), `PreparedComposition`
      (`lib/src/composition/types.rs:521`), `CompositionExecutionRequest`
      (`types.rs:626`), `HarnessPromptState` (`harness_orch/types.rs:21`),
      `MaterializedHarnessPrompt` (`types.rs:41`), `HarnessLoopState`
      (`loop_control.rs:177`), `LifecycleRunGuard` (`lifecycle/mod.rs:475`).
      Write the mapping into `notes/state-migration.md` — this table is the
      checklist Phases 5–9 execute against.
- [ ] Split immutable invocation inputs from a coordinator-owned run ledger
      within the invocation layer. Put the invocation-wide proxy chain/hop
      accounting, exact-command approval cache, command/sequence timing
      anchors, command-wide performance accumulation, and transition
      provenance in that ledger. Proxy may never mutate the immutable inputs or
      reset the ledger, and only the coordinator (or an adapter with a narrow
      coordinator-supplied capability) may mutate it. Today proxy tracking is
      `ProxyTracking { chain, pending }`
      (`loop_control/proxy.rs:117`) with the "is this doc already in the chain"
      test open-coded in four places (`loop_control.rs:203-208`,
      `control_dispatch.rs:89`, `:189`, `:246`). Provide **one** method; leave the
      call sites for Phase 6.
- [ ] Model active-document execution state as a document-iteration slice that
      owns retry/resume budgets plus a replaceable provider-attempt slice. A
      retry or resume replaces only the provider-attempt slice and retains and
      decrements its budgets; proxy and the next document-loop iteration receive
      fresh budgets. Give retry/resume/proxy/loop counters distinct labeled homes
      (R8 forbids an unlabeled shared counter). Today:
      `ControlBudgets { retry, resume }`
      (`control_dispatch.rs:14`) + `control_budget_for` (`control.rs:176`).
- [ ] L1: model each layer separately and assert which fields survive initial,
      retry, resume, proxy, and loop transitions. Assert active-document
      execution state is discarded by proxy, retry/resume cannot reset their own
      budget by replacing a provider attempt, immutable invocation inputs expose
      no mutation, and transition-ledger mutation is coordinator-only.
- [ ] Enforce construction boundaries with private fields and narrow
      constructors: lifecycle evaluation can construct only an
      `EvaluatedProxyRequest`; constructing `ProxyHandoff` requires the opaque
      resolved-reference value produced by the shared file-resolution authority
      plus a successful coordinator hop/cycle decision. Downstream preparation
      accepts the handoff by value and has no string-target resolver entry point.
      Use compile-fail doctests only if these are public API boundaries;
      otherwise unit-test the constructors and rely on Rust visibility rather
      than an aspirational "type-level test."

**Validation checkpoint 2**
- `just test` green. New types compile with no production call sites yet.
- `notes/state-migration.md` accounts for every field of all seven legacy
  carriers — each is either mapped to a layer or explicitly marked as deleted
  with a reason.

---

## Phase 3 — `proxy.with` parsing and diagnostics

**[∥ Phase 2]**

Goal: accept the authoring surface. No evaluation, no runtime.

- [ ] Add the typed `proxy` descriptor exception so the parser recognizes `with:`
      **before** the generic sibling-key rule fires. The rule lives at
      `parse.rs:438-483` (allowed siblings today: `when`, `action`, `no_error`);
      the key/value proxy branch is `action_shape.rs:518-521`. The exception is
      exact — it must not make nested maps valid for another proxy field or any
      other action.
- [ ] Accept `with:` only on key/value form. `with: {}` parses and is equivalent
      to omission.
- [ ] Reject a non-mapping `with:` with a typed, source-aware error.
- [ ] Reject dynamic or non-string `with:` keys with a source-aware diagnostic
      rooted at `{event}.stack[i].action[j].with`; append `.key` only when the
      key has a safe string representation. Never invent a misleading dotted
      path for an unrepresentable YAML key.
- [ ] Reject `with: "{{ payload }}"` (whole-mapping interpolation) in v1 with an
      actionable message pointing at explicit-key authoring. This is a named
      out-of-scope follow-up, so the error must say so.
- [ ] Reject a proxy-only `with:` on any other action.
- [ ] Confirm — with a test, do not just assume — that positional `proxy:` plus a
      sibling `with:` still produces the existing `LifecycleStackAmbiguous`
      (`parse.rs:642-650`) with its actionable key/value rewrite. This is
      acceptance criterion 19 and the spec asserts the *existing* diagnostic is
      already correct.
- [ ] Add the new error variants to `lib/src/composition/error/mod.rs` and map
      each to `FrontmatterHighlight::Property` in `frontmatter_block_spec()`
      (`error/mod.rs:1685-1703`) so the `FrontmatterExcerpt` path highlights the
      most specific locatable line.
- [ ] Add styled renderers in `lib/src/composition/error/render/lifecycle.rs`
      alongside the existing ones (`:254`, `:276`, `:297`, `:318`, `:340`, `:358`).
      Use `StatusBlock`/`Prose` — no raw `println!`/`eprintln!`.
- [ ] L1 in `lib/src/composition/lifecycle/tests/action_shape_control.rs`: parse
      key/value proxy with omitted/empty mappings and mappings containing scalar
      and nested values;
      reject non-mapping, dynamic-key, whole-mapping, and wrong-action cases.

**Validation checkpoint 3**
- `just test` green; every new diagnostic has a test asserting its typed variant
  **and** its rendered hint.
- Existing proxy parser tests (`action_shape_control.rs:125`, `:441`, `:771`,
  `:986`) still green — the `with:` exception did not widen the sibling rule.

---

## Phase 4 — `with:` evaluation into an evaluated request

Depends on Phase 2 (types) and Phase 3 (parse).

Goal: at event time, produce a fully-evaluated `EvaluatedProxyRequest` or fail
atomically. No filesystem access in this phase (R2: evaluation is
provider-neutral and does not consult the filesystem).

- [ ] Represent `with:` values as typed `Expr` trees at parse time by **recursing**
      the existing `action_value_to_expr` rule (`action_shape.rs:329-367`) through
      nested arrays and objects. Do not add a second interpolation grammar and do
      not route values through `render_message` (`executor.rs:887-899`) — that is
      the path that collapses `proxy.target` to a string.
- [ ] Evaluate at event time through `SubtreeCompose` with the same state,
      globals, and resolution context as the rest of the lifecycle surface
      (`executor.rs:501-555`), `.strict()`, followed by `reject_surviving_spans`
      (`executor.rs:1258-1275`).
- [ ] Implement the value rules: mixed string → string; exactly-one-span →
      preserved type (`bool`, number, string, array, object, null); authored YAML
      scalars/arrays/objects/nulls keep their types; nested strings follow the
      same rule.
- [ ] Guarantee no raw span survives into the overlay — a `{{ ... }}` must never
      be deferred into target-time evaluation (acceptance criterion 20).
- [ ] Resolve against the source document's **live** frontmatter
      (`MaterializedHarnessPrompt::live_frontmatter`, `harness_orch/types.rs:41`)
      so a preceding `set_frontmatter` in the same stack is visible to `with:`.
- [ ] Scope globals per event: `err`, `timing`, `current` where applicable;
      out-of-scope late-binding roots fail closed (reuse `LATE_BINDING_ROOTS`,
      `lifecycle/mod.rs:131`).
- [ ] Enforce atomicity: evaluate target **and** the complete mapping before any
      state change. On failure, install no partial overlay, leave the source
      active for diagnostic attribution, and do not touch the target.
- [ ] Keep `no_error` accepted (it is a universal field, `actions.rs:58-70`) but
      non-suppressing for proxy: proxy has no side-effect dispatch phase, so
      overlay/target evaluation failures stay fatal. Today `no_error` only
      suppresses dispatch (`executor.rs:767-793`), so this should require no
      behavior change — add a test that locks it.
- [ ] Emit a typed interpolation-failure diagnostic naming the exact nested
      `with` path (including object keys and array indices where representable),
      the lifecycle event, the proxy action, and the target — **without** dumping
      unrelated overlay values.
- [ ] L1: literal, mixed, whole-value, and nested interpolation semantics;
      unknown root, malformed expression, unknown function, out-of-scope global;
      atomic-failure assertions; `no_error` does not suppress.

**Validation checkpoint 4**
- `just test` green.
- A narrow structural test (or the existing composition-call inventory,
  extended if suitable) proves proxy-overlay evaluation delegates to
  `SubtreeCompose`; behavioral tests alone cannot prove the absence of a second
  interpolator.
- A test proves `with: { x: "{{ true }}" }` yields JSON `true`, not `"true"`.

---

## Phase 5 — The canonical preparation service

Depends on Phase 2.

Goal: one service, explicit stages, explicit entry reason. This is the largest
phase; land it in the listed order.

- [ ] Define the entry-reason enum and encode the spec's stage matrix as data,
      not as scattered `if`s. Rows: Direct document, Proxy target, Retry, Resume,
      Next loop iteration. Columns: source/input basis, `initialize`, schema +
      full shell audit, loop ownership. No entry reason may fall through to a
      different policy.
- [ ] Build the service around today's sanctioned composer (`prepare_direct`
      `prepare.rs:198`, `prepare_inline` `:373`), giving it explicit stage
      boundaries rather than a new parallel implementation.
- [ ] Make the prepared document store the **exact** `ComposeContext` used to
      compose it (R5), plus the explicit environment override layer. Body
      interpolation, effective frontmatter, lifecycle DM2 lookup, schema/file
      evaluation, and shell preflight must all read that stored snapshot.
- [ ] Remove `ComposeContext::capture()` as a runtime fallback on any prepared
      path. Today the capture sites are `prepare.rs:148`, `prepare.rs:328`,
      `compose/prep.rs:260`, `compose/prep.rs:716`, `pipeline.rs:1001`,
      `harness_orch/prompt.rs:96`. The snapshot must derive from immutable launch
      inputs plus target-specific source/repo/workspace and resolved
      provider/model identity — never from `std::env::current_dir()` after the
      wrapper changes child CWD. (See the recorded hazard: the wrapper
      intentionally mutates parent CWD to repo root.)
- [ ] Keep the late-binding `current.ctx.*` surface live and explicitly forbid it
      as a fallback for a missing prepared `ctx.*`.
- [ ] Retire the second composer: delete the `compose_with` calls at
      `harness_orch/prompt.rs:218` and `:260` and route `materialize_harness_prompt`
      through the canonical service. Follow the `Inline` arm at `:281` as the
      precedent — it already delegates to `prepare_inline`.
- [ ] Retire the third option-builder: `preflight_proxy_target`
      (`harness_orch/prompt.rs:124-159`) becomes a canonical-service call.
- [ ] Resolve `RematerializeInputs` (`lib/src/composition/types.rs:496`): either
      demote it to an internal input of the canonical service **with an honest
      limited name**, or delete it in favor of the Phase 2 invocation/document
      layers. Adding fields to it is explicitly not the answer. Note its in-flight
      mutation at `prompt.rs:180-186` (`preflight_proxy_target` folding newly
      approved commands into `pre_approved_commands`) — that behavior must be
      preserved by whatever replaces it.
- [ ] L1: prove canonical preparation returns **semantically equivalent** prepared
      documents for direct and proxy entry given the same resolved source and
      assembled input layers. When proxy carries an overlay, compare against a
      direct preparation supplied equivalent effective inputs rather than
      treating the overlay as accidental route drift.
- [ ] L1: lock the stage matrix per entry reason — exactly one `initialize`
      emission, full retry/resume validation, loop structural-plan reuse.
- [ ] L1: prove context construction is independent of later process-CWD changes
      (construct, then change CWD, then assert `ctx.area` is unchanged). Because
      CWD is process-global, use the existing RAII restore pattern and
      `#[serial_test::serial(...)]`; the test must restore CWD even on panic.

**Validation checkpoint 5**
- `just test`, `just test-l2`, `just lint` green in `claudine/`.
- The Phase 1 `compose_with` allowlist guard baseline **shrinks by two**:
  `harness_orch/prompt.rs:218` and `:260` are gone. Update the guard's baseline
  in the same commit and note the reduction.
- `RematerializeInputs` is either renamed or deleted — grep proves no
  source-specific field was added to it.

---

## Phase 6 — The active-document coordinator

Depends on Phases 2, 4, 5 and the file-resolution integration gate. This phase
removes the split ownership that causes the motivating bug; Phase 10 completes
the observable loop-equivalence fix.

- [ ] Introduce the coordinator above both the document-loop engine and the
      provider-attempt harness. Pure state decisions and the transition types
      live in `claudine::composition`; the CLI driver owns process, terminal,
      filesystem, and provider adapters (R1).
- [ ] Make the coordinator the **only** thing that may commit a change to active
      document identity.
- [ ] Convert the harness to *request* `Proxy` instead of swapping its own source.
      Delete the mutation at `control_dispatch.rs:209-213`.
- [ ] Collapse the two initialize-proxy channels into the one transition:
      - remove `LoopExecutionResult::init_proxy_target` (`looping/types.rs:189`)
        and `with_init_proxy_target` (`:228`); change `engine.rs:415-464` to
        return the transition;
      - remove `pipeline.rs:1130` / `:1175` / `:1215` and the
        `initial_proxy_target` parameter (`loop_control.rs:79`, `:130`, `:167`).
      There must be no supported proxy path whose consumption is optional.
- [ ] Make the coordinator the sole caller of `resolve_proxy_target` and of
      hop/cycle validation, and have it atomically commit a `ProxyHandoff`. While
      the file-resolution dependency is pending, `resolve_proxy_target` is only
      a bridge; the final implementation delegates to the shared
      `FileReference`-based resolver and preserves its typed resolved-reference
      provenance. No downstream layer resolves the target again.
- [ ] Replace the four open-coded chain checks (`loop_control.rs:203-208`,
      `control_dispatch.rs:89`, `:189`, `:246`) with the Phase 2 method. Fix the
      loop engine's single-element chain slice (`engine.rs:439`) by passing the
      invocation-wide chain.
- [ ] Implement clean-handoff semantics: once `proxy` is selected the coordinator
      synthesizes **no** later source terminal/finalize/loop signal and applies no
      uncommitted source closure. A proxy from `success`/`failure` skips that
      attempt's ordinary `finalize`; a proxy from `finalize` does not re-enter it.
      The target becomes the closure/output owner. Audit `LifecycleRunGuard`'s
      Drop net (`lifecycle/mod.rs:846`) and `reset_for_proxy` (`:704`) against
      this — the Drop net is a likely source of a synthetic emission.
- [ ] Nest the coordinator inside command-level ownership: `inline-compose` stays
      inline mode but only the final target is eligible for the inline closure;
      a sequence proxy stays inside its current step (no advance, no restart) and
      retains the step's scoped inputs and timing identity; `compose` routes the
      final active document's output to stdout.
- [ ] Preserve dry-run: it fires no lifecycle events and therefore never
      traverses a dynamic proxy route. Add an explicit test, not an assumption.
- [ ] L1: assert the provider harness cannot mutate active document identity
      (type-level if achievable, test-level otherwise).
- [ ] L1: assert every proxy producer returns the shared typed transition and
      every coordinator outcome consumes or explicitly rejects it.
- [ ] L1: lock closure/output ownership for direct, inline, and sequence-step
      modes, including the absence of a synthetic source finalize after a clean
      proxy.

**Validation checkpoint 6**
- The Phase 1 optional-proxy-channel guard baseline is **zero**. Delete the
  seeded list.
- `just test`, `just test-l2`, `just lint` green.
- Existing proxy L2s still green: `level2_lifecycle_initialize_proxy_runs_target_initialize`
  (`cli/tests/level2_lifecycle_control.rs:1118`), `..._cycle_guarded` (`:1382`),
  `..._respects_target_skip` (`:1311`), `..._respects_target_error` (`:1345`),
  `level2_lifecycle_failure_proxy_runs_target_document_no_loop` (`:889`) — note
  this last one encodes the corrected route drift and may need to be rewritten
  rather than kept; if so, say so explicitly in the commit body.

---

## Phase 7 — Staged bootstrap initialize, safety gate, and target reread

Depends on Phases 5, 6.

This phase establishes the staged reads and a single input-layer assembly point
using authored frontmatter plus caller overrides. Phase 8 wires the evaluated
proxy overlay into that point for both reads. The `proxy.with` path is not
end-to-end complete, and R4 is not signed off, until Phase 8 passes.

- [ ] Implement the staged canonical boot in order: resolve/read candidate →
      apply input layers → derive target-specific repo, selection hints,
      provider/model env, and the early-binding context `initialize` needs →
      parse the bootstrap lifecycle surface → narrow safety gate → run
      `initialize` through the normal evaluator → consume `skip`/`error`/`Proxy`
      atomically → reread the stabilized target → full preparation.
- [ ] Build the narrow safety gate: parse and approve **every potentially
      selected** `initialize` shell command against the same early-binding
      snapshot, and route all other initialize actions through the existing
      effect/permission engine. It does **not** run target schema validation or
      audit later-event commands. "Initialize before full pre-flight" never means
      "execute unapproved shell."
- [ ] Make the full post-stabilization audit cover every remaining lifecycle and
      template shell surface and **reuse** exact-command approvals already granted
      by the narrow gate (the cache is keyed on the normalized command string —
      `harness/shell.rs:39`, `:213-240`).
- [ ] Reread the stabilized target after `initialize` so successful
      initialize-time file/frontmatter mutations are visible, reapply caller
      layers through the shared input-layer assembly point, and do **not** fire
      `initialize` twice. Phase 8 adds the immutable overlay to this same point;
      do not create a temporary overlay-specific read path here.
- [ ] Support an initialize proxy chaining another atomic proxy; stabilize the
      chain before committing to provider launch or loop execution.
- [ ] Error routing at the boundaries: malformed frontmatter or a failure too
      early to construct the target's lifecycle config cannot fire target catch
      events — return the normal typed parse/bootstrap diagnostic. After the
      target lifecycle exists, later bootstrap/preparation failures follow normal
      `failure`/`finalize` routing without emitting either signal more than once.
      Cross-check against `execute_initialize_catch` (`pipeline.rs:1027`) and
      `route_initialize` (`:1086-1140`).
- [ ] Assert this staging did not create a second composition implementation —
      shared work is a shared service with explicit stage boundaries. The Phase 1
      allowlist guard is the mechanical check.
- [ ] L1: one initialize emission per document; reread-after-mutation observed;
      chained initialize proxy stabilizes atomically.

**Validation checkpoint 7**
- `just test`, `just test-l2` green.
- Allowlist guard unchanged (no new composer appeared).
- A test proves an `initialize` shell action is approved by the narrow gate
  before dispatch, and that the later full audit does not re-prompt for it.

---

## Phase 8 — Overlay input layering and precedence

Depends on Phases 4, 5, 7.

- [ ] Merge the resolved overlay into **every** read of the target's authored
      frontmatter: the bootstrap read before target `initialize` and the fresh
      read after initialize-time mutations. Reapply caller overrides after.
- [ ] Close the deferred R4 sign-off from Phase 7: prove initialize conditions,
      selection hints, and initialize actions observe the overlay on the
      bootstrap read, while full preparation observes the same immutable overlay
      after the stabilized reread.
- [ ] Place this before full Darkmatter composition and schema validation so the
      overlay participates in target frontmatter interpolation and computed
      properties, selection hints and initialize conditions, `SimplifiedSchema`
      validation/coercion/defaults/eager-file handling, target lifecycle parsing
      and shell discovery, loop configuration, and the prompt body.
- [ ] Implement precedence low→high: target-authored frontmatter < `proxy.with` <
      caller `key=value` / `--set`. The caller stays authoritative at every
      document; a router can never silently replace an explicit caller value.
- [ ] Implement shallow top-level semantics: scalar or array replaces; object
      replaces (no deep merge); `null` removes the target-authored top-level
      property before composition. Caller overrides may restore or replace any key.
- [ ] Route file-valued properties through the target's canonical file-resolution
      context — `with:` adds no second path resolver.
- [ ] Store the overlay at document scope as the **immutable, evaluated,
      pre-schema** input. Schema defaults, coercion, and invalid-optional drops
      affect prepared effective frontmatter only, never the stored overlay; a
      later refresh reapplies the same overlay deterministically.
- [ ] Implement overlay lifetime: survives retry, resume, and loop refresh of the
      same target; available to every lifecycle signal and body composition for
      that target; never written to disk; discarded when that target proxies on.
      A downstream proxy gets only its own `with:` plus caller overrides —
      forwarding is explicit; omitting `with:` installs an empty overlay.
- [ ] Confirm cycle detection and `MAX_PROXY_HOPS` (`control.rs:204`) still key on
      resolved document paths — an overlay does not create a distinct identity.
- [ ] Handle control-plane overlays: `with:` may set any top-level key including
      selection, lifecycle, loop, schema, timeout, and MCP. The target reparses
      and validates all resulting structural configuration, and every shell,
      filesystem, network, messaging, and provider effect stays subject to normal
      target-side policy. A source-resolved string installed under a target
      lifecycle key is **literal target data** — raw `{{ ... }}` may not survive
      DM2 and become a second target-time evaluation.
- [ ] L1: shallow replacement, null removal, precedence, atomic failure, and
      immediate-target overlay replacement — each tested independently.
- [ ] L1: schema normalization never mutates the stored overlay; control-plane
      values are source-resolved once, reparsed by the target, and policy-bound.

**Validation checkpoint 8**
- `just test` green.
- A test proves a caller `--set` beats a conflicting `with:` key.
- A test proves a target schema `required` property can be satisfied by `with:`,
  and that an invalid overlay produces the normal typed target schema error
  **without invoking the provider**.
- A byte/hash test proves neither source nor target Markdown bytes nor Darkmatter
  hashes change solely because `with:` was used.

---

## Phase 9 — Target-dependent launch rebuild and per-target shell approval

Depends on Phases 6, 7, 8. The two task groups below are **[∥]** with each other.
Phase 8 is required because control-plane overlay values can change selection,
MCP, workspace, and shell surfaces.

**Launch rebuild (R6)**

- [ ] On every active-document change, recalculate from the target plus immutable
      invocation state: provider/model selection, interactivity, MCP tags and
      runtime injection, workspace/repository behavior, child CWD,
      profile/binary, structured mode, system prompt, argv, child environment,
      dispatch context, and the target-owned closure plan.
- [ ] Keep the enclosing command/sequence output policy as invocation state — it
      is not rebuilt.
- [ ] Preserve normal precedence: explicit CLI intent stays authoritative; target
      frontmatter affects only what was not explicitly fixed. Proxying to a
      prompt pinned to another provider must behave like invoking that prompt
      directly under the same CLI arguments.
- [ ] L1: prove target-dependent decisions refresh on proxy while immutable CLI
      inputs do not.

**Shell approval (R9)**

- [ ] Give every fresh proxy target the same discovery and approval opportunity as
      a direct invocation, covering body, frontmatter, and lifecycle shell
      surfaces from the one prepared document that will execute.
- [ ] Allow the invocation-wide approval cache to be shared, but only an **exact**
      already-approved command may bypass a new prompt. Freezing the cache for the
      source must not block a target from requesting approval for newly
      discovered commands. Audit `CompositionPrepContext.shared_approval_cache`
      (`lib/src/composition/types.rs:697`) for a freeze that violates this.
- [ ] Rerun discovery and approval on retry/resume fresh-read preparation:
      unchanged exact commands hit the cache; new or changed commands get normal
      review. Loop-iteration materialization reuses the stamped structural plan
      and cannot introduce new command bytes.
- [ ] Guarantee approved bytes equal executed bytes. A `with:` value that
      influences a command must be present both at approval and at execution.
- [ ] L1/L2: a target-only command prompts even when the source's commands were
      already approved; an identical command does not re-prompt.

**Validation checkpoint 9**
- `just test`, `just test-l2`, `just lint` green.
- A test proves a proxy to a provider-pinned prompt selects that provider, and
  that an explicit CLI `--codex` still wins over it.

---

## Phase 10 — Loop ownership follows document identity

Depends on Phases 6–9. **This closes the motivating bug.** Overlay-derived loop
configuration and target-specific launch state must already be canonical before
loop ownership moves.

- [ ] Move loop recognition to after initialize routing stabilizes and before the
      first provider attempt for the active target. Today the decision is made
      about the router at `cli/src/commands/compose/prep.rs:462` via
      `resolve_loop_config` (`looping/config.rs:61`) before its `initialize`
      proxy fires.
- [ ] Give a proxied target the same document-loop coordinator it would receive
      when invoked directly.
- [ ] Preserve the ratified loop contract for later iterations: `initialize` runs
      once for the active document; the loop gate/mutations decide continuation;
      per-iteration refresh uses canonical preparation inputs and the stored
      document context and never falls back to the (now-deleted) reduced composer.
- [ ] Make a proxy emitted by the loop lifecycle end the source document and
      return a handoff to the coordinator — it does not become an extra iteration
      of the source loop.
- [ ] Reconcile the two copies of the initialize-control switch
      (`looping/engine.rs:400-480` and `pipeline.rs:1130-1215`) — after Phase 6
      they should already be one; verify and delete the remnant.
- [ ] **Re-enable the Phase 1 motivating-bug L2 test.** The deterministic routed
      fixture must match its direct target for iteration count, phase mutations,
      and target `initialize` count. Separately run the shipped
      `prompts/implement.md` → `prompts/_implement/implement-plan.md` command as
      a manual smoke test when credentials/provider availability permit; do not
      make live provider access part of the automated gate.

**Validation checkpoint 10**
- The Phase 1 ignored L2 test is enabled and **green**. This is the headline
  acceptance signal for the whole feature.
- `just test`, `just test-l2` green.
- `loop_initialize_proxy_hands_off_without_iterating`
  (`lib/src/composition/looping/engine/tests/lifecycle_control.rs:232`) still
  encodes correct behavior, or is rewritten with the change documented.

---

## Phase 11 — Retry and resume re-entry

Depends on Phases 5–10. Retry/resume refresh must consume the completed overlay,
target launch, shell-approval, and loop-ownership contracts rather than build a
second partial re-entry path.

- [ ] `Retry`: retain active document identity, caller overrides, the immediate
      overlay, context-derivation inputs, and proxy provenance. Create a fresh
      provider attempt/session. Refresh mutable document material through the
      canonical service and derive one new coherent prepared-context snapshot;
      do not reuse the previous snapshot as mutable state. Keep deriving
      pre/post-stage re-entry from `provider_launched`
      (`loop_control.rs:944`, `control.rs:138` → `reenter_preflight:
      !provider_launched`), but source the refreshed body, lifecycle, and launch
      plan from **one** coherent prepared document.
- [ ] `Resume`: retain the active document and a compatible live session. Refresh
      mutable document/lifecycle material canonically, then deliberately
      substitute the evaluated follow-up as provider input. Do not rerun
      `initialize`, change active document, or silently change session contract.
- [ ] Implement the **session compatibility key** on the prepared document,
      containing every launch property the provider cannot renegotiate on resume:
      at minimum provider, model, profile/binary and resume protocol,
      workspace/child CWD, permission/tool mode, structured-output mode,
      system-prompt delivery **and** content, and effective MCP server set. Allow
      provider adapters to contribute provider-specific identity fields. Today's
      gate is only `check_resume_support` (`cli/src/commands/wrap/resume.rs:48`,
      called `control_dispatch.rs:145`) — that is narrower than required.
- [ ] On key change after a canonical refresh, fail resume with a typed
      diagnostic that **names the incompatible facets** and recommends retry.
      Never mix a live session with a newly prepared launch plan.
- [ ] Scope retry/resume budgets to the active document iteration; proxy or the
      next loop iteration resets them while invocation-wide hop/cycle accounting
      continues. Replacing the provider-attempt slice must retain and decrement
      the current budgets, so a retry/resume cannot reset its own limit. Verify
      `ControlBudgets` / `control_budget_for` (`control.rs:176`) reset at the
      documented boundary.
- [ ] L1: retry starts a fresh session; resume retains only an identical-key
      session; proxy clears session and active-document execution state.
- [ ] L1: budgets persist across retry/resume attempts, then reset at the proxy
      or next-loop-iteration boundary while hop/cycle state continues.

**Validation checkpoint 11**
- `just test` green.
- One L2 per compatibility-key facet asserts resume refuses and names that facet.

---

## Phase 12 — Typed errors and status across transitions

Depends on Phases 3, 4, and 6–11, plus the error-propagation integration gate.
This phase unifies errors only after every transition producer and target boot
stage exists.

- [ ] Audit every transition path for `eyre!` stringification of a typed error and
      remove it. Resolution, initialization, preparation, schema, shell,
      selection, retry, resume, and proxy failures keep their concrete error and
      source/provenance context to the render boundary.
- [ ] Guarantee the same target failure has the same typed identity and rendering
      whether the target was direct, proxied from `initialize`, or proxied from
      terminal recovery.
- [ ] Add the remaining typed diagnostics from the spec's Errors section not
      already covered by Phase 3/4: target bootstrap/preparation failure with
      source and proxy provenance; resume incompatibility after canonical refresh;
      and any supported transition returned without an owning coordinator able to
      consume it.
- [ ] Implement handoff-failure routing: event-aware, no duplicate lifecycle
      emission. Before `finalize` it follows the existing failure/finalize
      transition; after `finalize` has fired it surfaces directly; a failure
      inside `finalize` never re-enters `finalize`. A failed handoff never
      half-activates the target.
- [ ] Status/tracing redaction: status may report that a handoff **includes** an
      overlay and tracing may record property names and counts, but neither may
      print overlay values — they can contain secrets. Follow existing redaction
      policy.
- [ ] Render all new terminal status/diagnostics through `TerminalRenderable`
      components (`StatusBlock`, `Prose`, lists, tables). No ad hoc
      `println!`/`eprintln!` on any transition path — add a guard or a test.
- [ ] Note for the seam: this phase adds **no** new error registry or rendering
      mechanics. Those belong to `../2026-07-13-error-propogation/spec.md`. If
      that spec has landed by now, use it; if not, keep transport typed and leave
      a `//!` pointer.

**Validation checkpoint 12**
- `just test`, `just test-l2` green.
- A test asserts identical typed identity + rendered diagnostic for one failure
  across all three routes (direct / initialize-proxy / recovery-proxy).
- A test asserts no overlay value appears in status or trace output.

---

## Phase 13 — L2 equivalence matrix and regression guards

Depends on Phases 6–12. Fixture files and harness scaffolding may be drafted
earlier, but assertions for typed rendering and final transition behavior are
enabled only after Phase 12.

- [ ] Build the equivalence harness: for each fixture, invoke the target directly
      and through an initialize router, then compare prompt and effective
      non-lifecycle frontmatter; `ctx.area`/`ctx.agent`/`ctx.model` and the
      corresponding `env.AGENT`/`env.MODEL` in body, frontmatter, **and**
      lifecycle surfaces; provider/model/interactivity, MCP, workspace/CWD,
      system prompt, argv, child environment; lifecycle signal order and target
      initialize count; loop iteration count and mutations; closure target,
      sequence-step identity, and stdout/stderr routing; shell approval and
      execution bytes; typed failure identity and rendered diagnostic.
- [ ] Use a fake provider and platform-neutral temporary paths so the matrix runs
      on macOS, Windows, and Linux. Do not hardcode `/tmp` or POSIX separators.
      Note the recorded hazard: `tempfile` creates `0755` dirs and some endpoint
      fixtures require `0700` — build the dir explicitly if permissions matter.
- [ ] Run the matrix in the repository's existing macOS, Windows, and Linux CI
      coverage. If a platform cannot be exercised from the implementation host,
      record the CI job and result in `notes/acceptance-map.md`; local macOS-only
      success is not cross-platform sign-off.
- [ ] Author the remaining L2 cases: caller-override precedence over `with:`;
      target schema/computed-property/initialize/body observation of typed overlay
      values; failure/finalize proxy using `err.*` inside `with:`; retry, resume,
      and loop refresh retaining the overlay; resume incompatibility per facet;
      a three-document chain with explicit and omitted forwarding;
      cross-repository proxy context and file resolution; target-authored
      provider/model and target-specific MCP tags; cycle, hop-limit, missing
      target, invalid overlay, schema failure, shell denial; initialize proxy
      returned from the library loop route proving it cannot be dropped silently;
      initialize-time mutation followed by stabilized reread; control-plane
      overlays adding target lifecycle/shell config with target-side validation
      and approval still running; `inline-compose` proxy closure ownership and a
      proxy inside a sequence step; dry-run proving no lifecycle side effect and
      no dynamic proxy traversal.
- [ ] Add passive corpus tests proving every production proxy route carries the
      complete handoff and every canonical-preparation caller supplies explicit
      context.
- [ ] Triage the existing suites (proxy parser, lifecycle placement, cycle/hop,
      retry, resume, loop, caller override). Each must stay green **or** be
      rewritten with an explicit note that it encoded route drift intentionally
      corrected here. `level2_lifecycle_failure_proxy_runs_target_document_no_loop`
      (`cli/tests/level2_lifecycle_control.rs:889`) is the most likely candidate —
      its name asserts the exact behavior R7 changes.
- [ ] Follow the L2 conventions: use `just test-l2`, never raw nextest. The
      recipe supplies the `level2_` filter, required-level environment, and
      bounded self-spawn concurrency. Harness capture has no scrollback, so long
      output needs a tall pane.

**Validation checkpoint 13**
- `just test`, `just test-l2`, `just lint` all green in `claudine/`.
- Every acceptance criterion 1–30 in the spec maps to at least one named test.
  Write that mapping into `notes/acceptance-map.md` — it is the sign-off artifact.
- Both Phase 1 drift guards are green at their final baselines.

---

## Phase 14 — Documentation and drift

Depends on Phase 13.

- [ ] Update the lifecycle topic doc (`claudine/docs/topics/`, surfaced as the
      skill's `lifecycle.md`) with: key/value `proxy.with` syntax and typed
      interpolation; source-time evaluation; the advanced control-plane-overlay
      trust model; precedence and immediate-target overlay lifetime; transient
      `with:` versus persistent `set_frontmatter`/`merge_frontmatter`.
- [ ] Update the composition topic doc with: the direct-versus-proxy equivalence
      contract; active-document ownership; the retry/resume re-entry contract and
      the resume compatibility key; the per-entry stage matrix; target-specific
      context/provider/MCP/workspace/loop behavior; inline closure ownership,
      sequence-step containment, and dry-run behavior.
- [ ] Recommend schema-declared data properties for ordinary parameter passing and
      explicitly call out control-plane overlays as an advanced, trusted-prompt
      capability.
- [ ] Correct stale documentation that describes retry/resume/proxy in terms of a
      reduced harness path, or that implies recovery is limited to `failure` when
      the universal lifecycle contract supports other runtime signals.
- [ ] Ship the reader note verbatim in the compatibility section: a proxied target
      may now run additional loop iterations, select its authored provider/model,
      request approval for its own shell actions, or surface the typed error
      direct invocation already produced. These are compatibility **fixes**, not
      preserved quirks.
- [ ] Update `.claude/skills/claudine/SKILL.md` and `architecture.md` for the new
      module structure (coordinator, canonical preparation service, transition
      types) and the retired carriers.
- [ ] Update `claudine/docs/dependencies.md` if any crate changed (e.g. `indexmap`
      for the overlay).
- [ ] Regenerate the skill `hash:` frontmatter with `md hash <file>` for every
      edited skill document.
- [ ] Move this feature directory to `features/_completed/` once checkpoint 14
      passes. Before moving it, update the spec status to the repository's
      implemented/completed convention and verify both declared dependency gates
      are satisfied; a completed directory must not hide an unresolved required
      dependency.

**Validation checkpoint 14**
- `just test`, `just test-l2`, `just lint` green.
- The `md hash` value of each edited skill doc matches its `hash:` frontmatter.
- No documentation still describes a reduced harness composer.

---

## Dependency graph

External dependency order: error propagation → file resolution → Phase 6 final
resolver integration. The Phase 12 gate remains necessary when Phase 6 proceeds
under an approved temporary resolver bridge.

```
1 ──> (2 ∥ 3)
2 + 3 ──> 4
2 ──> 5
2 + 4 + 5 + file-resolution gate ──> 6
5 + 6 ──> 7
4 + 5 + 7 ──> 8
6 + 7 + 8 ──> 9
6 + 7 + 8 + 9 ──> 10
5 + 6 + 7 + 8 + 9 + 10 ──> 11
3 + 4 + 6 + 7 + 8 + 9 + 10 + 11 + error-propagation gate ──> 12
6 + 7 + 8 + 9 + 10 + 11 + 12 ──> 13 ──> 14
```

Parallelizable:

- **2 ∥ 3** — state types versus parser/diagnostics; coordinate shared module
  exports and error payload types before editing.
- **9's two task groups** are internally parallel after their shared prepared
  launch-state shape is agreed.
- **Phase 13 fixture data/scaffolding** may start after Phase 4, but final
  assertions wait for Phase 12. This is preparation overlap, not permission to
  declare Phase 13 complete early.

## Risk register

| Risk | Signal | Mitigation |
|---|---|---|
| Spec is reviewed but still a draft | `status: draft` on this feature spec | Read-only discovery may continue; require owner authorization before any Phase 1 repository change. |
| Both dependency specs are unimplemented drafts | `status: draft` on file-resolution and error-propogation | Use the Phase 6 and Phase 12 integration gates; seams prevent duplication but do not waive dependencies. |
| Dependency order is accidentally reversed | File resolution depends on error propagation; the reverse edge is only `related` | Land or bridge typed error transport first, then unified file resolution, then replace the coordinator seam. |
| Phase 5 is the biggest single blast radius | Deletes the second composer that three call sites depend on | The `Inline` arm at `prompt.rs:281` already delegates — follow it as precedent. Allowlist guard gives a mechanical done-check. |
| Phase 10 changes observable behavior on purpose | Existing L2s named `..._no_loop` assert the old drift | Triage in Phase 13 is a task, not a surprise. Rewrite with an explicit note. |
| `ComposeContext` recapture after CWD change | The wrapper mutates parent CWD to repo root by design | Phase 5 removes `capture()` as a runtime fallback; the regression test uses serialized RAII CWD restoration. |
| L2 matrix is large and tmux-driven | Slow, flaky under load | Use `just test-l2`; do not compare across runs without a drift bracket. |
| A live provider makes the headline regression nondeterministic | Credentials, quotas, model changes, and network availability | Use the fake provider in CI; retain the shipped Codex command only as a manual smoke test. |
| Scope creep into `defer` | `Defer` is a parsed-but-unimplemented verb | Explicit non-goal. Do not implement or serialize a handoff. |
