---
total_phases: 5
created: 2026-10-12
phase: 1
agent: codex/gpt-6.1-sol
yolo: "true"
source_files_during_phase_1:
    - darkmatter/lib/tests/l1/request_context_builder.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
packages:
    - darkmatter
---

# Implementation Plan: The `fm` Frontmatter Namespace

## Scope and Successful Completion

Implement `2026-10-18-fm-namespace` across Darkmatter's library, `md`, DMLS,
Claudine's composition and hook adapters, and current repository artifacts.
The work combines a hard namespace rename with a document-lifecycle environment
contract: document expressions use `fm`, hook payload expressions use `hook`,
and all environment consumers of one document share its immutable captured map.

Completion means all eight specification acceptance criteria have observable
evidence: the exact fixed-root catalog is `fm`, `ctx`, `env`, `current`;
released names behave as ordinary properties; interpolation dependencies,
shell tokens, effective overrides, and lifecycle mutations remain correct;
hook dispatch preserves its payload shape; each new document captures its own
environment; passive tooling performs no observations; current prompts, schemas,
docs, and skills are migrated; and the required package and prompt gates pass.
Historical records remain intact. Implementation ends ready for review; the
author closes the feature after review.

Waves are globally numbered execution steps. Tasks within a wave marked
**parallel** may be assigned to separate subagents with the file ownership
described below. Every wave waits for the preceding wave's checkpoint unless
explicitly stated otherwise. Each task includes its tests and relevant comment
updates; final phases integrate and validate those changes. Implementation
agents must inherit the session's non-interactive constraints.

## Phase 1: Resolve Contracts and Reduce Risk

### Necessary Rules

1. **Namespace boundaries are settled.** Use `fm` only for document frontmatter
   and `hook` only for Claudine hook payloads. Neither old name has an alias or
   a special removal diagnostic. Keep `hook` out of Darkmatter's fixed catalog.
   Declared Claudine globals `err`, `timing`, and `group` retain their existing
   scope mechanism. Do not add `seq` or `loop`, retire sequence/loop properties,
   or reconcile the authored catalog's `tracking` entry here.
2. **Approval ordering needs an explicit ruling.** This feature is pipeline
   step 8; `2026-10-23-shell-template-approval` is step 13. The environment
   contract assumes later sequence commands receive template approval, which
   may not exist when this feature lands. Before Phase 3, record whether that
   prerequisite is already implemented, must land first, or the author approves
   an interim contract retaining exact-byte approvals and refusing changed
   commands before execution. Recommend the existing fail-closed audit as the
   interim behavior, if the author accepts it. Never treat stale approvals as
   permission for new bytes, silently introduce template approval here, or
   claim the specified template behavior is implemented without evidence.
   An unresolved ruling blocks completion of the affected environment work.
3. **Environment capture has a narrow boundary.** Capture at document entry,
   before environment-dependent bootstrap and `initialize`, and reuse the map
   through re-preparation, iterations, retries/resumes, handlers, and finalize.
   A sequence task, group leg, and proxy target each start a new document;
   transclusion does not. Preserve request-owned home, identity, CWD, repository
   observations, source anchors, launch magic scope, and policy/cache authority.
   Libraries accept supplied inputs; only the CLI boundary captures process
   environment. Do not call `RequestSnapshot::from_process()` repeatedly to
   refresh unrelated request state. If resolving the source reference precedes
   lifecycle entry, record which map selects that source and ensure all reads
   belonging to the entered document use its captured map.
4. **No performance spike.** The spec settles lookup cost: dotted lookup
   traverses the effective map and copies only its result; bare `fm` snapshots
   the object. Preserve that implementation property by review. No benchmark
   matrix or wider environment-capture measurement is scheduled.
5. **Input robustness applicability.** The planned changes alter expression
   lookup, in-memory request inputs, and authored schema descriptions/keys, not
   file-format or configuration deserialization. Markdown and expression
   grammars are expressly excluded from the requested matrix, so no matrix is
   required for this scope. If the Phase 1 inventory discovers a necessary
   manifest/config/schema-loader input-contract change, enumerate every
   load-bearing field for every supported format before implementing it. Add
   the complete absent/null/wrong-field-type/wrong-one-element/wrong-all-elements/
   empty/duplicate/trailing-content outcome table and positive control, then
   one real-tool-fixture matrix test per format asserting public outcomes.
   Audit the specified serde/default/filtering smells. A new reader must not
   enter the implementation without this extension to the plan.
6. **Migration is semantic.** Edit authored expression spans and prose about
   the retired roots. Preserve ordinary keys, paths, provider payload fields,
   inserted data, escaped examples, hash-kind `fm`, and historical records.
   Search matches require classification, not blanket replacement.

### Wave 1 — Contract Discovery (sequential)

- [x] **Surface Inventory**
  - Inspect Darkmatter's root catalog, `expression/doc_namespace.rs`,
    conditions, shell expansion, effective-state lookup, interpolation,
    error blocks, lazy-root capture, and preflight; inspect DMLS classification,
    completion, hover, and diagnostics consumers.
  - Trace Claudine's `dispatch/expression.rs` into templates, `when`, event
    matchers, and harness validation. Trace lifecycle bindings and the
    preparation, sequence/group, and proxy paths through their environment
    and approval inputs. Include CLI composition/wrapper entry points.
  - Record production sites, tests, passive expression extractors, authored
    artifacts, and excluded historical records in `implementation-log.md`
    beside this plan. Map each acceptance criterion to its owning task.
  - Resolve or explicitly record the approval ruling above and confirm matrix
    applicability. Read relevant request, expression, testing, and OS skills
    before designing API changes or platform-specific evidence.

- [x] **Environment Spike**
  - Prerequisite: inventory. Run once, before Phase 3; use one host and a tiny
    supplied-map fixture, without mutating process environment or launching a
    real provider. Exercise the existing context derivation API to determine
    whether an environment-only replacement preserves source/tree anchors,
    home, launch scope, and request authority.
  - Demonstrate map A in a document and its included child, then map B in a
    separately entered document; show expression and file-reference agreement.
    Trace approval re-audit with changed bytes to establish the current refusal
    behavior. Do not implement the future template-approval feature as a spike.
  - Record the minimum API seam, actual capture sites, approval evidence, and
    any needed API change. Retain useful fixture assertions as regression
    tests; do not schedule the spike again in later phases.

**Checkpoint:** An inventoried migration surface, an environment ownership
design, and an explicit approval disposition exist. Any unresolved author
ruling is recorded as a dependency; independent namespace work may proceed.

## Phase 2: Implement Namespace and Passive Tooling Contracts

### Wave 2 — Runtime Roots (parallel)

- [ ] **Document Roots**
  - Own Darkmatter runtime expression, interpolation, shell-token, lazy-root,
    preflight, and error code plus their tests. Rename internal namespace
    helpers/module to `fm`; update all special-case consumers and fixed catalog
    to exactly `["fm", "ctx", "env", "current"]`.
  - Preserve whole-object snapshot semantics, effective caller overrides,
    qualified missing-value behavior, inserted-text safety, and cheap dotted
    traversal. Add literal-index dependency collection: `fm['a']` waits for
    `a`, and `fm['release.channel']` waits for that exact key. Bare `fm` adds
    no dependency; computed indexes keep existing behavior.
  - Remove `current_env` refresh machinery, environment-refresh capability
    reads, preflight dynamic-shape handling, and nesting errors. Keep `current`
    lazy observation, per-evaluation memoization, and existing restrictions.
  - Test rendered values and public warnings/errors for dotted and bracket
    reads, missing qualified fields without fallback, dependency ordering,
    dotted/bracket cycles, bare snapshots before/after writes, shell token
    executable precedence, `fm.fm`, caller overrides, and inert inserted values.
  - Test ordinary present/absent `doc` and `current_env`, schema declarations,
    unchanged advisory suppression/identity, fixed-root shadowing, and
    qualified access to colliding document keys.

- [ ] **Hook Adapter**
  - Own Claudine dispatch/harness adapter code and its tests, disjoint from
    Darkmatter files. Rename payload lookup and internal object helpers to
    `hook`; remove special treatment of `doc`.
  - Preserve normalized fields, `extra`, `os`, `hardware`, `git`, `project`,
    omission/null behavior, unprefixed event paths, and existing hook `env.*`.
    Bare `hook` excludes process `env`; missing `hook.*` never falls back.
  - Assert actual template output, `when` decisions, matcher decisions, and
    harness validation messages for bare/qualified `hook`, obsolete `doc`,
    missing qualified fields, nested `hook.extra.doc`, and unprefixed paths.

**Checkpoint:** Runtime tests prove both hard renames and the retained `current`
behavior. No production Rust lookup special-cases literal `doc`.

### Wave 3 — Catalog Consumers (parallel)

- [ ] **Passive Tooling**
  - Prerequisite: Wave 2 catalog. Own DMLS and passive validator/advisory
    consumers and tests; use shared catalog classification rather than another
    root list. Update passive shell-read classification as needed.
  - Prove built-in completion and hover offer `fm` and its frontmatter members,
    drop `doc`/`current_env` built-ins, and preserve schema-based completion,
    hover, and diagnostics for ordinary `doc`, `current_env`, and `hook` keys.
  - Use existing effects instrumentation to prove completion, hover,
    expression validation, shell validation, and advisory classification do
    not evaluate expressions, observe `current`, run shells, or capture process
    environment. Preserve warning identities and source spans.

- [ ] **Authored Catalogs**
  - Own `darkmatter/schemas/` and Claudine schema-description changes. Use
    `git mv` for `partials/doc.yaml` to `partials/fm.yaml`; update references,
    the type key, and descriptions in `darkmatter.yaml` and the partial.
  - Change Claudine `mutations.yaml` and `side-effect.yaml` namespace wording;
    leave unrelated schema semantics and `tracking` alone.
  - Run existing passive schema/artifact checks and a normal-invocation fixture
    consuming the current catalogs to detect broken partial references.

**Checkpoint:** Catalog runtime and authored projections agree, editor behavior
is passive, and released names remain usable document properties.

## Phase 3: Establish One Environment per Document

### Wave 4 — Environment Authority (sequential)

- [ ] **Document Snapshot**
  - Prerequisites: Phase 1 design and resolved approval ruling, Phase 2 runtime.
    Own shared request/context APIs and the minimal Claudine input plumbing.
    Add the smallest explicit document-entry environment seam needed by the
    spike; retain a single request authority for unrelated captured state.
  - Make the document map available before bootstrap/`initialize`; thread it
    through `ComposeRequest`, effective context environment values,
    `FileResolutionContext` derivation, schema roots including `SCHEMAS_DIR`,
    shell preparation/approval, and provider preparation. Use `FileReference`
    for existing file-reference resolution; do not invent another resolver.
  - Freeze this map for the lifecycle and inherit it into transclusions.
    Keep direct `md compose` as one document and keep library APIs free of
    ambient process reads. Update relevant request docs/comments and context
    construction guards only with reasoned allowances.
  - Assert same-map agreement for `env.*`, context environment values,
    environment-expanded file references, schema selection, and approval
    bytes, with supplied maps differing from ambient values.

**Checkpoint:** A reviewed document-entry API exists, consumer agreement tests
pass, and source anchors/request-owned state remain unchanged by map replacement.

### Wave 5 — Lifecycle Routing (parallel)

- [ ] **Entry Routing**
  - Own Claudine preparation, sequence task/group execution, proxy control,
    and CLI boundary plumbing. Route each direct/inline/wrapper document,
    sequence task, serial or parallel group leg, and proxy target through the
    document-entry seam. Avoid shared mutable environment state between legs.
  - Re-preparation after initialize, loop iterations, retry/resume, failure
    handling, and finalize reuse the entered document's map. Update sequence
    and proxy tests that currently require request-wide environment reuse.
  - Test supplied maps A/B for successive tasks, distinct group legs, and
    proxy source/target. Within one document exercise all lifecycle events,
    loop iterations, retry/resume, provider preparation, and included files.
    Assert outputs/resolved files and observable capture counts, including
    capture before initialize and no extra capture on re-entry.
  - Integrate the ruled approval behavior. Prove a changed later-task
    environment value never executes under stale authorization; if templates
    are available, test runtime value checks, otherwise test the approved
    interim refusal and record its departure from the spec.

- [ ] **Lifecycle Bindings**
  - Own `lifecycle/bindings.rs`, related binding/error tests, and approval
    refusal/error definitions; coordinate shared fixtures with Entry Routing.
    Set `LATE_NAMESPACES` to `["current"]`, remove retired namespace errors,
    and retain the refusal for `current` in shell approval scopes.
  - Prove lifecycle and approval scope declarations still shadow frontmatter
    `err`, `timing`, and `group`; `fm.err`, `fm.timing`, and `fm.group` read
    the document. Test fixed-root collisions and lifecycle `set` followed by
    bare/qualified reads and an earlier object's unchanged snapshot.
  - Cover sequence-owned frontmatter properties without migrating them to
    future roots. Test ordinary `current_env` in scopes formerly refusing it.

**Checkpoint:** Every document boundary has changed-map evidence; every reuse
boundary has stable-map evidence; approval remains fail-closed; lifecycle
globals retain their availability behavior. No test mutates real process env.

## Phase 4: Migrate Current Repository Artifacts

### Wave 6 — Repository Migration (parallel)

- [ ] **Prompt Migration**
  - Own current shipped prompts in `prompts/`, `darkmatter/prompts/`, and
    `claudine/prompts/`, frozen prompt fixtures in library/CLI suites, and
    remaining executable examples/test documents outside the runtime owners'
    files. Inspect root meaning before changing each expression.
  - Migrate `doc.doc` in clarification prompts to `doc` or `fm.doc`; migrate
    old frontmatter roots to `fm` and event-payload roots to `hook`. Remove
    retired live-environment behavior without deleting ordinary properties.
  - Exercise frozen fixtures through normal composition/wrapper invocation
    and maintain silent, hermetic child-process policy. Run changed shipped
    prompts through the existing opt-in passive corpus checks locally.

- [ ] **Documentation Migration**
  - Own current READMEs, Darkmatter and Claudine topic/inline docs, DMLS docs,
    and skills. Cover every page named in specification Decided §5 plus
    additional current pages found by the inventory.
  - Explain `fm` lookup/snapshots, root precedence and collisions, hook-only
    `hook`, ordinary released names, and per-document environment capture with
    compact examples. Update compose-request diagrams and the expression
    classification Mermaid diagram to `fm ctx env current`.
  - Add compact before/after migration tables for document `doc` → `fm`, hook
    `doc` → `hook`, and removal of live environment reads. Do not imply child
    shell exports can change the parent's or next document's environment.
  - Update Darkmatter request/compose/schema/library/testing guidance and
    Claudine CLI guidance; append a dated Claudine timeline entry using the
    implementation date. Preserve older entries. Current docs must state
    behavior directly and never name or link this feature/spec.

**Checkpoint:** Current artifacts describe and exercise the implemented
contract, historical records are unchanged, and frozen fixture coverage remains
in ordinary L1 while shipped-prompt corpus checks remain local opt-in.

## Phase 5: Verify Coverage and Prepare Review

### Wave 7 — Integration Evidence (sequential)

- [ ] **Migration Audit**
  - Run the specification's discovery searches and a repository-wide
    `current_env` search, excluding the stated historical records. Review
    production special cases and use existing passive extractors for bracket,
    multiline, shell-token, and expression-valued fields the regexes miss.
  - Classify each remaining authored-expression hit as ordinary property,
    explicit migration example, unrelated literal/path, or defect. Record
    evidence in the implementation log, including dispatch and harness
    coverage. Historical exclusions are scoped by path/date, not by declaring
    every difficult hit historical.
  - Reconcile the eight acceptance criteria with named tests, artifact checks,
    and evidence. Review changed symbols' comments for drift, cheap dotted
    lookup, passive observation boundaries, and environment ownership.

- [ ] **Required Gates**
  - Run `just test` and `just lint` in `darkmatter/` and `claudine/`; Darkmatter's
    area recipe includes DMLS. Use the repository Test Toolkit/nextest and the
    area's `CliProcessFixture`; do not run `cargo test` or `cargo fmt`.
  - Update `claudine/cli/tests/level2/level2_ac28_lifecycle_lazy_roots.rs` to
    cover `current` only and run it under Claudine `just test-l2`. Require
    actual execution evidence rather than reporting a missing harness skip
    as a pass. Terminal/browser windows must never gain focus.
  - Run root `just test-prompts` for the opt-in shipped-prompt checks. Ensure
    frozen prompts and authored schemas also have passive and normal-path
    coverage. Add no CI cells and no internal-prompt CI coverage.
  - Record host/environment, commands, outcomes, and unavailable evidence.
    Tests must be portable across macOS, Linux, native Windows, and WSL2;
    use the OS skill and existing evidence channels before claiming a platform
    cannot be checked. Reuse qualifying existing evidence and retain the
    event-driven CI policy; do not infer portability from one host's pass.
  - Resolve failures within feature scope and rerun affected checks. Serialize
    build-heavy gates when they share the Cargo target lock.

- [ ] **Review Handoff**
  - Finish the implementation log with capture-boundary evidence, approval
    ruling, any authorized spec departures, migration-hit classification, and
    gate results. Keep the finalized spec unchanged.
  - Confirm no unresolved necessary ruling is hidden by passing tests; any
    remaining blocker means implementation is not complete. Mark completed
    plan tasks only when their checkpoints have evidence.
  - Leave the feature in place as implementation complete, ready for review.
    Do not commit, move it to `_completed`, or run `just complete` without the
    separately required author action.

**Checkpoint:** All eight acceptance criteria are accounted for, required gates
have passing evidence, and the final change is ready for the author's review.
