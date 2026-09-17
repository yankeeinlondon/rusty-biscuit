---
created: 2026-09-15
total_phases: 8
phase: 3
agent: opencode/zai-coding-plan/glm-5.3
yolo: true
area: claudine
spec: ./spec.md
packages:
    - claudine-cli
source_files_during_phase_1:
    - claudine/cli/tests/level2_initialize_generated_transclusion.rs
    - claudine/cli/Cargo.toml
docs_updated_during_phase_1:
    - claudine/fixes/2026-09-15-initialize-after-proxy/spec.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_1:
    - claudine/fixes/2026-09-15-initialize-after-proxy/design.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
    - darkmatter/lib/src/markdown/compose/context/options.rs
    - darkmatter/lib/src/markdown/compose/preflight/collect.rs
    - darkmatter/lib/src/markdown/compose/preflight/mod.rs
    - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
    - darkmatter/lib/src/markdown/compose/mod.rs
    - darkmatter/lib/tests/frontmatter_surface_projection.rs
docs_updated_during_phase_2:
    - darkmatter/docs/inline/preflight-checks.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
    - .claude/skills/darkmatter/compose.md
source_files_during_phase_3:
    - claudine/lib/src/composition/prepare.rs
    - claudine/lib/src/composition/prepare/bootstrap.rs
    - claudine/lib/src/composition/prepare/bootstrap/tests.rs
    - claudine/lib/src/composition/prepare/service.rs
    - claudine/lib/src/composition/preflight.rs
    - claudine/lib/src/composition/mod.rs
    - claudine/cli/tests/composition_seams.rs
docs_updated_during_phase_3:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
    - .claude/skills/claudine/architecture.md
source_files_during_phase_4:
    - claudine/cli/src/commands/compose/prep.rs
    - claudine/cli/src/commands/wrap/composition/staged_boot.rs
    - claudine/cli/src/commands/wrap/composition/mod.rs
    - claudine/cli/src/commands/wrap/composition/pipeline.rs
    - claudine/cli/src/commands/wrap/composition/runner.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt.rs
    - claudine/cli/src/commands/wrap/harness_orch/prompt/tests.rs
    - claudine/cli/src/commands/wrap/harness_orch/mod.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/coordinator.rs
    - claudine/cli/src/commands/wrap/harness_orch/loop_control/tests/coordinator_adoption.rs
    - claudine/cli/src/commands/wrap/mod.rs
    - claudine/cli/tests/compose_initialize_staged_boot.rs
    - claudine/cli/tests/composition_seams.rs
    - claudine/lib/src/composition/looping/seed.rs
    - claudine/lib/src/composition/mod.rs
docs_updated_during_phase_4:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
    - .claude/skills/claudine/architecture.md
source_files_during_phase_5:
    - claudine/cli/src/commands/wrap/sequence/mod.rs
    - claudine/cli/src/commands/wrap/sequence/tests.rs
    - claudine/cli/tests/sequence_initialize_include_preflight.rs
    - claudine/cli/tests/level2_initialize_generated_transclusion.rs
docs_updated_during_phase_5:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
    - .claude/skills/claudine/architecture.md
source_files_during_phase_6:
    - prompts/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md
    - claudine/cli/tests/fixtures/shipped_implement_route/shipped-hashes.json
    - claudine/cli/tests/shipped_prompt_contract.rs
    - claudine/cli/tests/shipped_prompt_route_drift.rs
docs_updated_during_phase_6:
    - claudine/fixes/2026-09-15-initialize-after-proxy/plan.md
    - claudine/fixes/2026-09-15-initialize-after-proxy/implementation-log.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
human_review: false
message_to_agent: |-
    Phase 6+ notes from Phase 5 (sequence static-preflight boundary; see implementation-log.md ## Phase 5).
    (1) OQ1 Option A is implemented as guidance only: `approve_preflight_graph` (cli/src/commands/wrap/sequence/mod.rs) emits a
    stderr Prose `note:` ahead of the UNCHANGED typed error when static preflight fails with
    `PreFlightDiscoveryFailed(Transclusion(Io(NotFound)))` (`is_missing_include`). The code stays `composition.failed`; nothing
    about staging, approval, or dry run changed.
    (2) Plan deviation: the note does NOT suggest "add a prior task that creates it". A probe showed static preflight composes
    every prompt document before step 1, so an earlier `shell:` step cannot satisfy the include. Phase 8 docs must say "create
    the file before starting the sequence", not "add a prior task".
    (3) Pre-existing behaviors observed, not changed, worth a reviewer's eye (and accurate wording in Phase 8 docs):
    (a) a sequence step's prompt is composed BEFORE that step's `initialize` runs, so content `initialize` appends to an
    existing include is absent from that step's prompt (pinned in
    `sequence_initialize_include_preflight::existing_include::a_sequence_whose_include_exists_runs_unchanged`);
    (b) `sequence --dry-run --yolo` executed a `shell:` step (created its marker file); composition.md's Dry Run section only
    carves out `::shell` spans. Not pinned by any test.
    (4) Phase 7's AC coverage can cite `cli/tests/sequence_initialize_include_preflight.rs` (L1) and
    `level2_initialize_generated_transclusion::level2_sequence_preflight_note_renders_for_an_include_initialize_would_create` (L2)
    for the sequence boundary. That L2 file's header no longer says the tests are expected to fail (drift fixed).
    (5) Cross-OS: see the Phase 5 log (build-win-native still out of disk).
    Phase 6 notes (shipped prompt repair; see implementation-log.md ## Phase 6).
    (6) The planned `parent_dir(spec)` -> `dirname(spec)` log repair was ALREADY on the branch (commit 47e9259ab, as
    `dirname(spec || plan)`); Phase 6 made no expression change there.
    (7) The real Phase 6 defect was the logging blocks in `prompts/_implement/implement-plan.md`: `initialize` runs
    `ensure_file: log` before the body composes, so `!file_exists(log)` could never be true and a fresh run was told
    "the log file already exists". The blocks now key on an UNSTARTED log:
    `!file_exists(log) || (markdown_body_empty(log) && is_empty(frontmatter(log)))` and its complement. Phase 8 docs describing
    the prompt should use that wording.
    (8) For AC10 (Phase 7) the L1 e2e `shipped_prompt_contract::shipped_implement_plan_logging_instructions_follow_log_content`
    drives the side-effect-free fixture through `claudine compose --goose` and asserts `ensure_file` creates an empty log
    on first run and preserves existing bytes afterwards (useful AC4 evidence). New guard
    `shipped_prompt_route_drift::fixture_body_matches_the_shipped_body` makes the fixture a faithful stand-in for the shipped body.
    It does NOT use the router or preserve an original spec spelling; Phase 7 still owns the router e2e.
    (9) Unrelated flake seen once under load (concurrent `just lint`): LEAK-FAIL in
    `claudine composition::sequence::task::tests::shell_tasks::an_early_wait_error_still_reaps_the_whole_tree` (32s); passed
    3/3 in isolation and in a clean `just test` rerun.
---

# Execution Plan — Run Initialization Before Body Discovery

Converts `spec.md` into an ordered, observable execution plan.

## Work Summary and Success Criteria

### The problem

A live document that declares `initialize` cannot create the files its own
body will later include. Two code sites read the document body before
`initialize` runs:

1. The command-level template audit in
   `claudine/cli/src/commands/compose/prep.rs` (`prepare_and_run_active_document`,
   the `resolve_shell_approvals(Some(&source.markdown), ...)` call near line 662)
   discovers body `::shell` commands through Darkmatter's condition-blind
   `compose_preflight`, which dereferences every `::file` transclusion. A missing
   transclusion target is fatal here — before `initialize` ever fires. This is
   the reported failure.
2. Canonical preparation (`claudine/lib/src/composition/prepare/service.rs`
   `prepare_document` → `prepare.rs` `prepare_direct_with_prompt` /
   `prepare_inline`) calls `Markdown::compose_with` over the entire document to
   build a `PreparedComposition`. Moving only the earlier shell-discovery pass
   would reveal this second pre-initialization read of the same missing
   transclusion rather than fix the ordering.

The harness already contains the correct staged boot for newly adopted proxy
targets (`claudine/cli/src/commands/wrap/harness_orch/loop_control.rs`,
`run_initialize_stages` and `bootstrap_adopted_document_phase`: narrow
initialize-shell gate → `initialize` → stabilized reread → full audit). The
defect is that the command-level scan and canonical preparation happen before
that sequence is reached, for direct documents and adopted targets alike.

### The fix

- Add a **frontmatter/lifecycle-only projection** at the Darkmatter shared
  composition boundary so a caller can compose the effective frontmatter and
  lifecycle surface without dereferencing body transclusions.
- Add an explicit **initialize-bootstrap phase** to Claudine's canonical
  preparation service producing a distinct `BootstrapPreparation` (identity +
  retained inputs/provenance + effective lifecycle surface — not a
  `PreparedComposition`, no composed prompt).
- Move the premature command-level template audit so the **coordinator owns the
  ordering**: for live documents declaring `initialize` (direct or newly
  adopted proxy target), run bootstrap → narrow gate → `initialize` →
  stabilized reread → **then** full body discovery, approval, canonical
  preparation, and schema verdict. Documents without `initialize` keep the
  existing eager path unchanged. One lifecycle engine, one composer, no
  prompt-specific exceptions (R4).
- Sequences keep their static side-effect-free preflight (OQ1 Option A ruling
  in Phase 1).
- Repair the shipped prompt's `log` expression
  (`prompts/_implement/implement-plan.md`: `parent_dir(spec)` → `dirname(spec)`)
  as a separate, identified change; do not redefine `parent_dir`.

### Naming decisions fixed up front

| Concept | Name | Home |
|---|---|---|
| Darkmatter frontmatter/lifecycle-only projection | `ComposeOptions::only_frontmatter_surface()` (final name may adjust; must live in Darkmatter) | `darkmatter/lib/src/markdown/compose/` |
| Bootstrap output type | `BootstrapPreparation` | `claudine/lib/src/composition/prepare/` |
| Canonical bootstrap entry | `prepare_bootstrap` | `claudine/lib/src/composition/prepare/service.rs` |
| Staged-boot eligibility predicate | reuse `defers_schema_verdict_to_initialize` semantics (authored `initialize` frontmatter key) | `claudine/cli/src/commands/compose/prep.rs:67` |
| Post-initialize fresh read | "stabilized reread" (existing term) | harness orchestration docs |

### Reading this plan

- Phases run in order. A task marked **‖ wg-<id>** belongs to a *work-group*:
  tasks sharing a work-group label inside one phase touch disjoint files and
  may run concurrently. Cross-phase concurrency is called out explicitly.
- Each phase ends with a **Checkpoint** — a concrete, runnable verification.
  Do not start the next phase until its checkpoint passes.
- `just` recipes run from the relevant package area (`claudine/`,
  `darkmatter/`) unless stated otherwise. Tests use nextest via `just test`
  (L1) and `just test-l2` (L2).
- Tests must use `CliProcessFixture`, isolate filesystem and provider state,
  suppress lifecycle audio, never launch real providers, and never focus
  terminal/browser windows.

### What successful completion looks like

- The reported invocation shape — a router proxying to a target whose
  `initialize` creates an absent file the target's body then includes —
  succeeds through the normal CLI path with a fake provider receiving the
  completed prompt, and `initialize` ran exactly once with creation preceding
  discovery/composition.
- All twelve acceptance criteria in `spec.md` have passing, hermetic
  regression coverage; approval and lifecycle semantics (denials, `skip`,
  `error`, proxy chains, retry/resume, loop continuation, dry run) remain
  intact; documents without `initialize` behave exactly as before.
- The shipped `implement-plan.md` prompt uses `dirname(spec)` for `log`, its
  fixture/hash manifests are updated, and its logging instructions read
  correctly when initialization has already created an empty file.
- Composition/lifecycle docs and the Claudine skill describe the new ordering;
  comments on changed symbols have been reviewed for drift.
- Validation evidence (`just test`, `just test-l2`, `just lint` for claudine
  and darkmatter areas) is recorded in the fix directory; remaining OS gaps,
  if any, are reported. The agent's terminal state is "implementation
  complete, ready for review" — moving the fix to `_completed` is the
  author's job, never the agent's.

---

## Phase 1 — Rulings, Reproduction, and Spikes

Produces the decisions and evidence every later phase depends on. **No
production code changes in this phase** beyond a committed, currently-failing
reproduction test and a design note.

### Necessary Rules

These rulings resolve the specification's open points. Record each decision
(and its spec citation) in `claudine/fixes/2026-09-15-initialize-after-proxy/design.md`
as it is confirmed.

1. **OQ1 — sequences (binding).** Adopt Option A: sequence execution keeps its
   recursive, side-effect-free static preflight; an absent transclusion that
   `initialize` would create remains a static-preflight failure for a sequence
   task. Add a characterization test and an actionable diagnostic directing
   authors to create the artifact (or a prior task that creates it) before
   starting the sequence. Do not adopt Option B, and do not build any part of
   Option C in this fix. If sequence parity becomes a priority later, it is a
   separate feature with an explicit kinded artifact contract.
2. **Staged-boot eligibility (binding).** One predicate selects the staged
   boot for live entry: the *authored* frontmatter contains an `initialize`
   key (exactly the existing `defers_schema_verdict_to_initialize` semantics —
   malformed or empty `initialize:` still stages). A newly adopted proxy
   target always stages. Retry, resume, and later loop iterations never
   re-stage. No second detection mechanism may be introduced.
3. **Projection placement (binding).** The frontmatter/lifecycle-only
   projection is added inside Darkmatter at the shared composition boundary
   and is consumed by both the bootstrap and full preparation paths through
   the same option assembly. Claudine must not reimplement interpolation.
4. **Parsing is not discovery (binding).** Loading/parsing the root document,
   assembling caller inputs and proxy overlays, resolving target identity,
   capturing the document epoch, and lexically inspecting for context
   requirements (`ContextRequirements::for_document`) remain permitted before
   `initialize`. Any dereference of a body dependency is not.
5. **Dry-run contract (binding).** `--dry-run` performs no `initialize` side
   effects, does not traverse a dynamic proxy, and may still report a missing
   transclusion that only `initialize` would create. This fix changes live
   ordering only.
6. **`parent_dir` semantics (binding).** The shipped-prompt repair changes the
   `log` expression to `dirname(spec)`; `parent_dir` keeps its meaning and its
   other uses (display strings) are untouched.
7. **Header placement (advisory).** Status/header rendering based only on the
   root document and resolved target (including the proxy-handoff
   announcement) is not body discovery and stays where it is (R3).

- [x] **Reproduction test (red)**
      - Add a hermetic L2 reproduction in `claudine/cli/tests/` (new file, e.g.
        `initialize_generated_transclusion.rs`) using `CliProcessFixture`: a
        router prompt whose `initialize` proxies to a target whose
        `initialize` stack runs `ensure_file` on an absent `implementation-log.md`,
        and whose body includes that file through nested
        `::block when="file_exists(log)"` / `::block when="phase > 1"` guards,
        with fixture-owned spec/plan data and a fake provider.
      - Use an **unambiguous valid log path** (correct `dirname`-style
        expression) so the failure cannot be attributed to the shipped
        prompt's `parent_dir` defect.
      - Assert the current typed failure (`TransclusionError: I/O failure` /
        `PreFlightDiscoveryFailed` family) before initialize; mark the test
        `#[ignore]`-free and let it fail red now — it is the Phase 7 AC2/AC10
        regression once green.
      - Record the exact observed failure text in `design.md`.
- [x] **‖ wg-a** **Darkmatter projection spike**
      - Enumerate `ComposeOperation` variants and the existing `only(&ops)`
        prior art in `darkmatter/lib/src/markdown/compose/preflight/collect.rs`
        to determine the exact projection that composes effective frontmatter
        (including the deferred lifecycle subtree, with `{{ }}` spans retained
        for event-time surfaces) while performing **no** transclusion
        dereference, no body directive parsing beyond what frontmatter
        requires, and no remote fetch.
      - Confirm the projection composes with `deferred_schema_verdict(true)`
        and with lifecycle keys excluded from body compose, matching the
        option assembly in `prep.rs` and `prepare.rs`.
      - Write findings (chosen API shape, operation set, invariants) into
        `design.md` as the Phase 2 contract.
- [x] **‖ wg-a** **Loop-path ordering trace**
      - Trace the loop route end to end: `build_and_run_loop`
        (`prep.rs:834`) → `build_loop_seed_with_lifecycle` → the engine's
        `initialize` transition → iteration-1 `kind.prepare_staged` closure
        (`prep.rs:966` area) → `execute_composition_attempt`. Identify
        precisely which of these reads the body pre-`initialize` for a
        looping, initialize-declaring document, and where the stabilized
        reread must land so iteration 1 composes post-`initialize` disk
        state. Record the decision in `design.md` (this drives Phase 4's
        loop task).
- [x] **‖ wg-a** **Regression archaeology (timeboxed, optional)**
      - Timebox: one hour of `git log`/`git bisect` on
        `claudine/cli/src/commands/compose/prep.rs` and
        `claudine/lib/src/composition/preflight.rs` to identify the first
        commit that placed template discovery before initialization. The spec
        notes this is not yet established; a known commit informs the review
        narrative but does not block any phase.

**Checkpoint 1.** `design.md` exists with all seven rulings recorded (or
explicitly reaffirmed), the projection contract written, and the loop-path
decision written. The reproduction test fails with the documented typed error
against the current build (`just test-l2` filtered to the new test). No other
behavior changed: `just test` still passes in `claudine/`.

---

## Phase 2 — Darkmatter Frontmatter-Surface Projection

Adds the shared-boundary projection the bootstrap composes through. Touches
only `darkmatter/`.

- [x] **Projection API**
      - Implement the frontmatter/lifecycle-only projection decided in
        Phase 1 (e.g. `ComposeOptions::only_frontmatter_surface()` or an
        equivalent method on the compose pipeline) in
        `darkmatter/lib/src/markdown/compose/`. It must reuse the same
        composition primitives and option assembly as a full compose —
        frontmatter interpolation, text replacement, and the retained
        lifecycle subtree — differing only in that no body operation runs.
      - Guarantee, structurally where possible: no `::file` transclusion
        resolution, no `::shell`/`::shell-block` body discovery, no page-block
        evaluation, no remote fetch, no body-dependent I/O or effects.
- [x] **Projection unit tests**
      - A document whose body includes a missing file composes successfully
        through the projection (the include is not dereferenced).
      - Frontmatter interpolation (including `dirname`, `file_exists`, and
        caller-set values) resolves; lifecycle subtree values retain their
        `{{ }}` spans verbatim; `deferred_schema_verdict` is honored.
      - The same document fully composed still fails on the missing
        transclusion (the projection did not silently weaken `compose_with`).
      - Existing `compose_preflight` behavior is untouched: condition-blind
        discovery still resolves every transclusion and still fails on a
        missing target (regression guard for R2's "do not fix by ignoring
        missing-file errors").
- [x] **API documentation**
      - Document the projection on the compose module (`//!` level): what it
        projects, what it never does, and that it exists for staged
        initialization ordering (link the concept, not this fix's path).

**Checkpoint 2.** From `darkmatter/`: `just test` and `just lint` pass,
including the new projection tests and the untouched-preflight regression
guards.

---

## Phase 3 — Canonical Initialize-Bootstrap Phase

Extends Claudine's canonical preparation service. Touches only `claudine/lib/`.

- [x] **BootstrapPreparation type**
      - Add `BootstrapPreparation` under `claudine/lib/src/composition/prepare/`
        containing exactly: resolved root identity (`resolved_path`,
        `source_repo_root`), retained input/provenance state (the
        `CallerInputLayers`/`PrepareOptions` inputs it was assembled from),
        effective selection hints parseable from the projected frontmatter,
        and the effective lifecycle surface (parsed `LifecycleConfig` with
        C3-resolved shell commands — reuse
        `preflight.rs::resolve_lifecycle_shell_commands` so approved bytes
        equal executed bytes).
      - It must **not** hold a composed prompt, a `CompositionClosurePlan`,
        or any body-derived state; it is not a `PreparedComposition` variant.
- [x] **prepare_bootstrap service entry**
      - Add `prepare_bootstrap` to
        `claudine/lib/src/composition/prepare/service.rs` alongside
        `prepare_document`: same source resolution, same option assembly and
        document-epoch observation points, but composing through the Phase 2
        projection. Record the `PreparedContextConsumer` observation for the
        bootstrap read so epoch semantics match the full read.
      - Both `CompositionMode::ChainedDocument` and
        `CompositionMode::InlineFrontmatterPrompt` must be supported (R4:
        parity across `compose` and `inline-compose`).
- [x] **Narrow-gate integration**
      - Expose from the bootstrap result everything
        `resolve_lifecycle_shell_approvals(&bootstrap.lifecycle, path,
        &[LifecycleSignal::Initialize], …)` needs
        (`preflight.rs:180` — already exists); approvals continue to land in
        the shared invocation cache so the post-stabilization full audit
        reuses them without a second prompt (R2).
- [x] **Lib unit tests**
      - `prepare_bootstrap` on a document whose body includes a missing file
        succeeds and returns a lifecycle surface; the same document through
        `prepare_document` still fails (ordering, not error suppression).
      - A malformed lifecycle surfaces the same typed error as full
        preparation; a late-binding root in an initialize shell command is
        rejected exactly as today.
      - Bootstrap and full preparation agree on effective frontmatter for a
        body-independent document (same values, same C3-stamped commands).

**Checkpoint 3.** From `claudine/`: `just test` passes including new
`composition::prepare` tests; `just lint` clean. No CLI behavior has changed
yet (the bootstrap is not yet called from the coordinator).

---

## Phase 4 — Coordinator-Owned Staged Ordering

The core fix. Moves the premature template audit and pre-initialize full
preparation behind the staged boot for live initialize-declaring documents,
across direct entry, proxy adoption, and the loop route.

- [x] **Stage the command-level audit**
      - In `claudine/cli/src/commands/compose/prep.rs`
        (`prepare_and_run_active_document`): when the staged boot applies
        (ruling 2 — `first && defers_schema_verdict_to_initialize(&source)`,
        or a non-first/proxy-target document), **skip** the pre-initialize
        `resolve_shell_approvals(Some(&source.markdown), …)` body discovery
        and the pre-initialize full `kind.prepare_staged` composition.
      - Run instead: `prepare_bootstrap` → narrow initialize gate →
        `initialize` emission through the **existing** machinery → on
        `skip`/`error`/`proxy`, terminate/propagate/surface with today's
        control-flow semantics and **no body discovery** for the abandoned
        document; on proceed, stabilized reread and only then the full
        discovery/approval/canonical preparation with
        `SchemaStage::Validate`.
      - Documents without `initialize` keep the current eager path byte for
        byte (R6/AC11).
- [x] **Restructure the pipeline boundary**
      - Adjust `claudine/cli/src/commands/wrap/composition/pipeline.rs`
        (`route_initialize`, `execute_initialize_catch`) and the request
        construction in `execute_loop_or_single` so initialize routing is
        driven from the bootstrap lifecycle surface for staged documents, and
        the full `PreparedComposition` handed to the executor is always the
        stabilized post-`initialize` read. Do **not** add a second lifecycle
        engine, a second composer, or a placeholder
        `PreparedComposition` (R4).
- [x] **Adopted-target unification**
      - Align `bootstrap_adopted_document_phase` /
        `run_initialize_stages` (`loop_control.rs:941`/`:827`) so a newly
        adopted proxy target's bootstrap read is the Phase 3 bootstrap
        (lifecycle surface only) rather than a full composition — the same
        ordering at every hop of a proxy chain (R3/AC7). Direct documents
        still enter at stage 4 without re-emitting `initialize`.
- [x] **Loop-route ordering**
      - Apply the Phase 1 loop-path decision: for a looping,
        initialize-declaring document, the seed's lifecycle parse and the
        engine's `initialize` transition use the bootstrap surface;
        iteration 1's body compose runs against the stabilized reread
        (post-`initialize` disk state with caller inputs, overlay, epoch, and
        file-resolution context reapplied), not the pre-`initialize`
        in-memory snapshot. Later iterations and retry/resume keep the
        existing stage matrix — no duplicate `initialize` (R6/AC7).
- [x] **Stabilized-reread state discipline**
      - The reread re-reads from disk and reapplies the same caller inputs,
        origins, proxy overlay, `FileResolutionContext`, and document epoch;
        it extends the retained context snapshot for newly demanded groups
        but never recaptures launch state or reinterprets caller-authored
        file values. Parse authored references with
        `biscuit_file::FileReference` and resolve through the existing
        `FileResolutionContext` — no prefix checks, no ambient resolution
        (R4). `ensure_file` and a later include of that file must agree on
        one resolved identity across macOS/Linux/Windows/WSL2.
- [x] **Failure fidelity**
      - A file still missing after `initialize` fails through the existing
        typed diagnostic exactly once via the active target's ordinary
        `blocked`/`finalize` routing, and no provider launches (R5/AC8). A
        bootstrap-gate failure (denied initialize shell command) keeps its
        pre-ownership routing and produces no command effects (R2/AC5).
- [x] **Entry-path smoke parity**
      - Extend existing L1/L2 seams (`wrap_compose_preflight.rs`,
        `composition_seams.rs`, `level2_lifecycle_control.rs` families) to
        cover: direct `compose` staged boot, `inline-compose` staged boot,
        proxy adoption staged boot, and a loop-owning target's first
        iteration — each asserting initialize-once and
        creation-before-discovery ordering.

**Checkpoint 4.** The Phase 1 reproduction test now passes end to end with a
fake provider. From `claudine/`: `just test` and `just test-l2` pass.
Specifically verify AC11 posture: a no-`initialize` document with a missing
transclusion still fails at its existing preparation boundary with the
identical error text recorded in Phase 1.

---

## Phase 5 — Sequence Static-Preflight Boundary

Implements OQ1 Option A exactly. Small, deliberately bounded.

- [x] **Characterization test**
      - In the sequence test surface (`claudine/cli/tests/sequence_cli.rs` or
        lib sequence preflight tests as appropriate): a sequence task whose
        prompt document declares an `initialize` that would create an
        included file fails static preflight with the typed missing-file
        error before any step starts. Assert no side effects and no step
        execution.
- [x] **Actionable diagnostic**
      - Where the sequence static preflight surfaces the missing-transclusion
        failure, add a bounded note directing authors to create the artifact
        (or add a prior task that creates it) before starting the sequence —
        generated transclusions in sequences are intentionally unsupported by
        this fix. No contract change, no relaxation, no new approval prompt
        mid-sequence.
- [x] **Negative coverage**
      - A sequence referencing a prompt whose transclusion *exists* still
        runs unchanged; sequence `--dry-run` behavior is untouched.

**Checkpoint 5.** From `claudine/`: `just test` and `just test-l2` pass with
the characterization green; the diagnostic renders in the captured output
(no window focus — use the capture harness).

---

## Phase 6 — Shipped Prompt Repair

Independent of Phases 2–5; **may run concurrently as work-group wg-prompt**
with any of them since it touches only prompt artifacts and their manifests.

- [x] **‖ wg-prompt** **Fix the log expression**
      - In `prompts/_implement/implement-plan.md` line 35: change
        `parent_dir(spec) + "/implementation-log.md"` to
        `dirname(spec) + "/implementation-log.md"`. Leave every other
        `parent_dir(...)` use (display strings in messages) untouched, and do
        not change `parent_dir`'s meaning anywhere.
      - *Phase 6 note:* already present before this phase — commit `47e9259ab`
        landed it as `dirname(spec || plan)` (covers plan-only invocations).
        Verified in both the shipped prompt and its fixture; no edit needed.
- [x] **‖ wg-prompt** **Review logging instructions**
      - Re-read the body's logging guidance (the "start by creating the log
        file" block around lines 126–132) so it stays accurate when
        `initialize`'s `ensure_file` has already created an empty file:
        the instruction must read as "ensure it exists, then append" rather
        than assuming creation is the agent's job.
- [x] **‖ wg-prompt** **Update fixture and hashes**
      - Mirror the repair into
        `claudine/cli/tests/fixtures/shipped_implement_route/_implement/implement-plan.md`
        if it carries the same expression, and refresh the shipped-prompt
        hash manifest (`shipped-hashes.json`) using `md hash` per repo
        convention. `shipped_prompt_contract.rs` /
        `shipped_prompt_route_drift.rs` must stay green.

**Checkpoint 6.** From `claudine/`: targeted `just test` for the shipped
prompt contract tests passes; `git diff` shows only the prompt, fixture, and
hash-manifest changes in this work-group.

---

## Phase 7 — Acceptance Regression Coverage

Turns the twelve acceptance criteria into hermetic tests. Test authoring for
disjoint AC clusters is parallelizable; the harness/fixtures are shared, so
coordinate fixture landings before forking.

- [ ] **AC cluster: generated-file flows (AC1–AC4)**
      - AC1 absent generated file via proxy router; AC2 nested
        `file_exists(log)` / `phase > 1` guards at phase 1 and a later phase;
        AC3 unconditional include of the generated file; AC4 `ensure_file`
        preserves existing contents, second invocation reads the persisted
        file and initializes once per invocation. Fake providers; assert
        creation preceded discovery/composition.
- [ ] **AC cluster: approval integrity (AC5, AC8)**
      - Deny an initialize shell command → no effects, no launch.
        Initialization creates/rewrites an included document containing a
        `::shell` directive → its command is audited before execution. A
        false-condition include still contributes commands to the approval
        set (condition-blind discovery preserved). Missing-after-initialize
        → typed error, exactly once, no launch.
- [ ] **AC cluster: mutation and control flow (AC6, AC7)**
      - Initialization mutates a body dependency or the target document →
        prompt and approvals use reread state, not cached bootstrap bytes.
        Proxy chain, clean exit, error, and `skip`: abandoned bodies never
        read; no second `initialize` during adoption, retry, resume, or
        ordinary loop continuation.
- [ ] **AC cluster: entry-path parity (AC9)**
      - Direct invocation and proxy adoption through both `compose` and
        `inline-compose` at their shared boundaries, including a loop-owning
        target's first iteration; sequence coverage per Phase 5.
- [ ] **AC cluster: shipped artifact e2e (AC10)**
      - Passive coverage of the relevant shipped prompts plus a hermetic
        end-to-end regression driving the implementation router
        (`prompts/implement.md` → `_implement/implement-plan.md`) through the
        normal CLI path with fixture-owned spec/plan data, the **original
        spec argument spelling preserved** in the fixture, and fake
        providers. Success must not be attributable to the prompt repair
        alone — the AC1–AC3 fixtures use unambiguous valid paths.
- [ ] **AC cluster: unchanged reasons and dry run (AC11)**
      - No-`initialize` document still fails a missing transclusion at its
        existing boundary with pre-fix error text; retry/resume audit a
        fresh read without another initialization; dry run performs no
        initialization side effects (including no `ensure_file` creation).
- [ ] **AC cluster: resolution parity (AC12)**
      - An explicit-relative reference and at least one repository-scoped or
        magic reference: the lifecycle effect (`ensure_file`) and the
        transclusion resolve through the same request-scoped resolution to
        one identity. Assert on identities/behaviors, never host-specific
        separator text.
- [ ] **‖ wg-evidence** **Evidence and gaps report**
      - Run the full local matrix — `just test`, `just test-l2`, `just lint`
        in `claudine/` and `darkmatter/` — and write
        `claudine/fixes/2026-09-15-initialize-after-proxy/evidence.md`
        listing exact commands, results, and any OS coverage gaps with the
        reuse/qualification plan per the repo's evidence rules.

**Checkpoint 7.** Every AC 1–12 maps to at least one named passing test;
`evidence.md` exists with the recorded runs. Full `just test` + `just test-l2`
green in both package areas.

---

## Phase 8 — Documentation, Skill, and Drift Pass

- [ ] **‖ wg-docs** **Repo docs**
      - Update claudine's composition/lifecycle documentation wherever it
        describes preparation ordering (`claudine/docs/pipeline.md` and the
        relevant topic docs): initialize now precedes body discovery and the
        schema verdict for live staged documents; dry-run contract unchanged;
        sequences unchanged per OQ1 Option A.
- [ ] **‖ wg-docs** **Claudine skill**
      - Update `.opencode/skill/claudine/composition.md` and
        `lifecycle.md` (and `SKILL.md` links if structure changes) where they
        describe the affected ordering.
- [ ] **‖ wg-docs** **Comment drift pass**
      - Review every changed symbol's `///`/`//!` and inline comments —
        `prepare_and_run_active_document`, `execute_loop_or_single`,
        `build_and_run_loop`, `resolve_shell_approvals`,
        `prepare_document`/`prepare_bootstrap`, `run_initialize_stages`,
        `bootstrap_adopted_document_phase`, `route_initialize`, and the
        Darkmatter projection — fixing or deleting drifted comments in the
        same change. Per repo rule: where drift is found, the code is
        correct and the comment is wrong.
- [ ] **Final validation**
      - Re-run `just lint`, `just test`, `just test-l2` for both package
        areas after all doc/comment edits; confirm the Phase 1 reproduction
        and Phase 7 suites remain green; append final results to
        `evidence.md`.
- [ ] **Closure readiness**
      - Confirm `spec.md`'s completion conditions read satisfied: generated
        file flow succeeds through the normal invocation path, acceptance
        cases pass, approval/lifecycle semantics intact, evidence recorded.
        Update `spec.md` frontmatter (`implemented: true`) only if the review
        cycle directs it; otherwise leave the flag to the author. Report
        "implementation complete, ready for review".

**Checkpoint 8.** Documentation and skill render correctly (no broken
references), `git diff` shows comment-only changes confined to changed
symbols, and the final validation results are appended to `evidence.md`.
