---
spec: "/Volumes/coding/wt/rusty-biscuit/feat-better-schematics/schematic/features/2026-10-16-schematic-response-model-graphql/spec.md"
plan: "schematic/features/2026-10-16-schematic-response-model-graphql/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - schematic/features/2026-10-16-schematic-response-model-graphql/plan.md
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
docs_created_during_phase_1:
  - schematic/features/2026-10-16-schematic-response-model-graphql/implementation-log.md
skills_files_updated_during_phase_1: []
packages: []
human_review: true
human_review_items:
  - |-
    **The earlier feature is now built, but its review is waiting on your decisions. This feature should wait for them.**

    This feature lets API definitions describe more kinds of reply: streams,
    several possible reply shapes, error tables, and GraphQL. It builds on an
    earlier feature, `2026-10-15-schematic-definition-vocabulary`. That earlier
    feature is now fully built and committed. Its first review, however,
    stopped with six questions for you, and none has been answered yet
    (`review-1.md` in that feature's folder).

    Three of those answers set things this feature's first phase must lock
    down for every later phase:

    - the layout of the portable "definition bundle" file, which sets this
      feature's bundle round-trip requirement and its file-format tests;
    - the order in which an endpoint's replacement login credentials are
      chosen, which sets what the exported OpenAPI extension describes;
    - whether to keep the "retry guidance" fields, which changes that same
      extension.

    This feature's specification also says it must be refreshed from those
    review outcomes before its plan runs. It is still marked
    `refresh_required: true`.

    Options:

    - **A. Answer the six questions in the earlier feature's review, refresh this spec, then rerun this phase.**
      - Pros: this phase records final facts, so later phases do not have to
        be redone.
      - Cons: about one review session of your time before this feature
        starts.
    - **B. Run now against the code as built, assuming you accept every recommendation in that review.**
      - Pros: work starts immediately. The review recommends "keep as built"
        for most items.
      - Cons: if you choose differently on the bundle layout or the
        credentials order, the locked contracts and parts of Phases 2–8 must
        be redone. The spec would also still be unrefreshed.

    **Recommendation: A.** Answering the review takes far less work than
    unwinding a phase built on a guess. After the rulings (and any code change
    they need) land, set this spec's `refresh_required` to `false` and rerun
    Phase 1 from the start.
message_to_agent: |-
  Phase 1 is still blocked (fourth run, 2026-10-22, HEAD 9a50aad7b). Nothing
  changed since the third run. 4b's review-1.md
  (2026-10-15-schematic-definition-vocabulary) still has human_resolved: false,
  its "Human rulings" section still says "Pending", and its six decisions are
  open (bundle v1 layout, auth-override precedence, retry vocabulary, OpenAI
  query omission, explode default, OAuth client auth). This spec is still
  refresh_required: true. Rerun Phase 1 from the start only after those
  rulings are recorded and this spec is refreshed (refresh_required: false).
  The consumer/provider inventories, baseline, timeout spike, and contract
  freeze have not been done. The 2026-10-22 table in the log is the current
  prerequisite snapshot.
---

# Implementation Log for 2026-10-16-schematic-response-model-graphql (8 phases)

## Phase 1

### Outcome: blocked at the prerequisite gate

Plan Rule 1 requires the run to stop with a recorded block if a required
prerequisite surface is absent. The spec's "Dependencies and ordering" section
says `2026-10-15-schematic-definition-vocabulary` (spec 4b) must be **landed
first**. On 2026-10-21 it is not, so this run stops after Wave 1's
prerequisite inventory. No source, test, generated artifact, doc page, or skill
changed.

Only the **Prerequisite inventory** task is checked off in the plan. Its
deliverable is this section. The following tasks are deliberately left
unchecked:

- **Consumer inventory** and **Provider inventory**: 4b Phases 2–5 rewrite the
  builders, auth, OAuth2, query fidelity, and dependent call sites these
  inventories would count. Counts taken now would be stale before Phase 2
  could use them.
- **Baseline capture**, **Timeout spike**, and **Contract freeze**: the plan
  says Wave 2 runs only "after Wave 1 clears the gates". The bundle-format
  matrix columns cannot be frozen because no bundle loader exists.

### Prerequisite inventory (tree state 2026-10-21, branch `feat/better-schematics`)

| Prerequisite | Spec state | Required surface | Present in code? |
| --- | --- | --- | --- |
| `2026-10-11-schematic-generation-integrity` (spec 1) | `implemented: true`, closed | validation structure, R42 diagnostics format | yes (not re-verified in detail; refresh with the spec) |
| `2026-10-12-schematic-runtime-envelope` (spec 2) | `implemented: true`, closed | envelope, typed send, `EndpointSpec`, codecs, hook marker, `send_raw` | **yes**: `schematic/runtime/` (`schematic-runtime`). `EndpointSpec` at `runtime/src/endpoint.rs:29`, `ErrorBody` at `endpoint.rs:65`, `Decode` at `codec.rs:33`, hook marker `SupportsHooks` at `hooks.rs:41`, `ErrorKind<E>` at `response.rs:140` (exists), `NoDeclaredError` at `response.rs:251`, `send_raw` at `client.rs:660`, transport builder `SharedTransportBuilder` at `transport.rs:34` |
| `2026-10-13-schematic-pagination` (spec 3) | `implemented: true`, closed | pagination model, Relay preset | yes: `runtime/src/pager/` (`Paginated`, `CursorKind`, `ResumeAfter`/`ResumeBefore`), `define/src/pagination/selector.rs`. Whether a named Relay preset exists, or only a Relay-capable selector, was not checked; check during refresh (D-12) |
| `2026-10-14-schematic-definition-api` (spec 4a) | `implemented: true`, `status: human-in-the-loop`, 3 review iterations, still in `features/` (not closed) | builders, `DefinitionContext`, typed registration, `.returns`/`.errors` placeholders | **yes, partly placeholder**: `DefinitionContext` at `define/src/registration.rs:121` (`register`, `register_for`, `register_as`). `Success(ApiResponse)` newtype at `define/src/builders.rs:48`. `ErrorSet { _private: () }` at `builders.rs:61`. `EndpointBuilder::errors(self, _: ErrorSet)` at `builders.rs:123` discards its argument (the placeholder this spec fills). `Endpoint::response: ApiResponse` still at `define/src/types.rs:395` |
| `2026-10-15-schematic-definition-vocabulary` (spec 4b) | `implemented: false`, `status: draft-spec`; **Phase 1 of 6 complete**, plan `phase: 2` | self-contained definition bundle (4b Phase 4); rate-limit/deprecated/auth-override vocabulary (4b Phases 2–3) that R26's `x-schematic` must carry; builder integration (4b Phase 2) | **no**: no bundle reader or writer in `schematic-define` (no `bundle` symbol in `define/src`). No rate-limit or deprecation metadata in `define/src`. `cross_origin` and `request_id_header` exist already (from spec 2/4a) |

Other facts gathered:

- 62 Rust files outside `target/` still name `ApiResponse` (a raw `grep -l`
  count, not the plan's consumer inventory).
- The uncommitted change to
  `features/2026-10-14-schematic-definition-api/implementation-log.md` already
  in the working tree is someone else's work. It was not touched.

### Why this is blocking rather than a defaulted choice

Under the rulings section of the run prompt, a block needs all three of these:

1. The specification offers no way to proceed without 4b. It says "Requires,
   landed first", and plan Rule 1 says to stop.
2. No option is consistent with the spec: R44 and the definition-bundle input
   matrices need 4b's bundle, and R26 needs 4b's metadata vocabulary. The plan
   forbids building those here ("this plan does not silently build those
   features"; "Unsupported bundle formats are not added to satisfy this plan").
3. A wrong guess is costly. Phase 2 edits `schematic-define` builders, stored
   types, and the serialized shape that 4b Phases 2–4 are about to change. Two
   features rewriting the same public API and serialized format would cost far
   more to reconcile than a review diff.

This spec is also still `refresh_required: true`. Its "Refresh notes" list
outcomes from 4a and 4b review 1 as "the largest source of change". Those
outcomes do not exist yet.

### Authorizations and defaults

- Pre-authorizations: confirmed by the run (plan Rule 2). **None were used**:
  nothing was regenerated, deleted, or migrated, and no dependency changed.
- Defaults D-4–D-31: **not adopted yet**. Rule 3 makes their adoption subject
  to the prerequisite review outcomes, which do not exist. Record them in the
  rerun's Phase 1.

### Gates

There are no source changes, only this log and the plan's checkbox and
frontmatter. The scope calculator was run after every edit was saved:

```sh
git status --porcelain --untracked-files=all | awk '{print $NF}' \
  | xargs python3 scripts/ci/affected_scope.py
```

Result: `packages: []`, `change_class: documentation`. As the run prompt
allows, `just test` and `just lint` were **skipped**. No dependent-package
tests or cross-OS checks apply, because no code changed. The `schematic-define`
skill needed no update.

### Rerun on 2026-10-21: still blocked

Phase 1 was invoked again without its prerequisite having moved. Before doing
any work, the gate was re-checked against the tree (HEAD `c135102a4`, the
previous run's Phase 1 close; clean working tree):

| Check | Result |
| --- | --- |
| `2026-10-15-schematic-definition-vocabulary` spec | `implemented: false`, `status: draft-spec`, `refresh_required: true` (unchanged) |
| its plan | `phase: 2` (Phase 1 of 6 complete, unchanged) |
| bundle reader/writer in `schematic/define/src` | none (no `bundle` symbol) |
| endpoint rate-limit / deprecation vocabulary | none (the only `deprecated` hits are OpenAPI parameter/operation fields in `openapi/export` and `openapi/import`) |
| this spec | `status: draft-spec`, `refresh_required: true` (unchanged) |
| 4a placeholders | unchanged: `Success(ApiResponse)` at `define/src/builders.rs:48`, `ErrorSet { _private: () }` at `builders.rs:61`, `errors(self, _: ErrorSet)` at `builders.rs:123`, `Endpoint::response: ApiResponse` at `define/src/types.rs:395` |

The three blocking conditions recorded above still hold, so this run stops at
the same point. No source, test, generated artifact, doc page, or skill
changed; only this log's rerun section and its `message_to_agent` were
updated. The prerequisite inventory table above is still accurate as of this
rerun. The remaining Wave 1 and Wave 2 tasks stay unchecked for the reasons
already given.

**Process note for the author:** rerunning Phase 1 cannot clear this gate.
The pipeline should not reinvoke this plan until 4b has landed and been
reviewed and this spec has been refreshed (`refresh_required: false`).

**Rerun gates.** The changed files are this log and the plan's frontmatter.
The scope calculator, run after every edit was saved
(`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`),
returned `packages: []` and `change_class: documentation`, so `just test` and
`just lint` were skipped as the run prompt allows. No dependent tests, cross-OS
checks, or skill updates apply.

### Rerun on 2026-10-22: prerequisite landed, still blocked on its review and this spec's refresh

The prerequisite has moved since the 2026-10-21 runs, so the gate was
re-evaluated rather than assumed closed. Tree: HEAD `ea86eb8ab`, branch
`feat/better-schematics`. The working tree already had uncommitted changes to
`features/2026-10-15-schematic-definition-vocabulary/spec.md` and an untracked
`review-1.md` there. Both belong to someone else and were not touched.

| Check | Result on 2026-10-22 |
| --- | --- |
| 4b code | **landed**: Phases 2–6 committed (`13c168975`, `69e819c97`, `c00790c57`, `cf1a27e80`, `ea86eb8ab`). `DefinitionBundle` with `to_json`/`from_json` and `format_version` at `schematic/define/src/bundle.rs:48`; `RateLimit` at `define/src/metadata.rs:99`, `RetryGuidance` at `metadata.rs:286` |
| 4b spec | `implemented: true`, `review_iterations: 1`, still `status: draft-spec` and `refresh_required: true` |
| 4b review 1 | a triage of decisions the author must make. No implementation review was done: `human_review: true`, **`human_resolved: false`**, and six open decisions (listed below) |
| 4a | `status: human-in-the-loop` (unchanged) |
| specs 2 and 3 | closed in `features/_completed/` (unchanged) |
| 4a placeholders this spec fills | unchanged: `Success(ApiResponse)` at `define/src/builders.rs:48`, `ErrorSet { _private: () }` at `builders.rs:61`, `errors(self, _: ErrorSet)` at `builders.rs:125`, `Endpoint::response: ApiResponse` at `define/src/types.rs:466` |
| this spec | `status: draft-spec`, **`refresh_required: true`** (unchanged) |
| Rust files outside `target/` naming `ApiResponse` | 64 (raw `grep -l`, up from 62) |

**Why this is still blocking.** Plan Rule 1's narrow "surface absent" condition
no longer holds: the runtime, builders, and bundle all exist. Two other
conditions still stop the run:

1. **This spec has not been refreshed.** Its Refresh notes say "Before its
   plan runs, refresh it", and they name the outcomes of 4a and 4b review 1
   as "the largest source of change". Those review outcomes do not exist yet.
   Refreshing the spec is an authoring step that comes before this plan, not a
   Phase 1 task. The plan says it is "a plan for the tree after the
   prerequisite reviews".
2. **4b review 1 is waiting on the author, and three of its six open decisions
   set inputs that this phase's Contract freeze must lock down:**
    - the version-1 bundle layout (schemas and bindings kept together in
      `api.schemas`, or separate maps). It decides R44 and the
      definition-bundle input-robustness columns.
    - the precedence for endpoint auth replacement, and whether anonymous calls
      may bypass it. It decides what R26's `x-schematic` extension carries.
    - whether to keep the retry-guidance vocabulary. It changes R26's field
      list.

    The other three decisions (omitting OpenAI query parameters, the importer's
    explode default, and OAuth client authentication) affect the baseline the
    Baseline capture task would compare against.

The rule "conform to what landed" in the spec's Dependencies section was
considered. It does not cover this case, because what landed is still waiting
on author decisions that may change it. Locking down contracts that the author
may reject in 4b is the costly wrong guess the blocking test describes. Phase 1
takes inventories and freezes contracts for every later phase, so redoing it
after the 4b rulings costs less than redoing Phases 2–8.

**What did not change.** No source, test, generated artifact, doc page, or
skill changed. No pre-authorization was used. Defaults D-4–D-31 are still
not adopted. The Consumer inventory, Provider inventory, Baseline capture,
Timeout spike, and Contract freeze tasks remain unchecked. The prerequisite
inventory table above is superseded by this section's table for 4b.

**What unblocks it:** (1) the author records rulings on the six 4b review 1
decisions (`human_resolved: true`); (2) any code change those rulings require
lands; (3) this spec is refreshed from the 4a/4b outcomes and set to
`refresh_required: false`. Then rerun Phase 1 from the start.

**Gates.** See the scope-calculator result recorded below.

The scope calculator was run after all edits were saved
(`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`).
It returned `packages: []` and `change_class: documentation`. The working tree
it saw included the two pre-existing 4b feature files. `just test` and
`just lint` were therefore skipped. No dependent tests, cross-OS checks, or
skill updates apply. Only this log changed in this run; the plan and spec were
not edited.

### Rerun on 2026-10-22 (fourth run): unchanged, still blocked

The gate was checked again at HEAD `9a50aad7b`. Nothing that unblocks it has
changed since the previous run:

- 4b review 1 still has `human_resolved: false`. Its `## Human rulings`
  section still says "Pending", and its six decisions are still open.
- The uncommitted 4b log entry in the working tree records the 4b
  review-to-implement pass. It says that pass had no findings to implement and
  that it is still waiting on the author. It contains no rulings, and this run
  did not touch it.
- This spec is still `refresh_required: true` and `status: draft-spec`.

The reasons in the previous section still apply, and so does the unblock path:
rulings recorded, any code they require landed, this spec refreshed and set to
`refresh_required: false`, then Phase 1 rerun from the start. No source, test,
doc, plan, spec, or skill file changed, and no plan task was checked off. Only
this log changed.

**Gates.** The scope calculator was run after the edit
(`git status --porcelain --untracked-files=all | awk '{print $NF}' | xargs python3 scripts/ci/affected_scope.py`).
The result is recorded below.
It returned `packages: []` and `change_class: documentation`, so `just test`
and `just lint` were skipped.
