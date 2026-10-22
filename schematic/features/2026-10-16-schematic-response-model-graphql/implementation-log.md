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
    **The feature this one builds on has not been built yet. Should this run wait for it?**

    This feature (a new way for API definitions to describe their replies:
    streams, several reply shapes, error tables, and GraphQL) is written on top
    of an earlier feature, `2026-10-15-schematic-definition-vocabulary`. That
    feature adds the self-contained "definition bundle" file format and new
    endpoint metadata (rate limits, deprecation, per-endpoint auth). Its
    specification says it must be **finished and reviewed first**. Today only
    the first of its six phases is done. No code for it exists yet.

    Phase 2 of this plan starts changing the same files that phases 2–5 of the
    earlier feature will change (`schematic/define/src/builders.rs`, `types.rs`,
    the OpenAPI extension code, and the four provider definitions). This
    feature's own plan says that if a prerequisite is missing, it records the
    block and stops. Its specification is also still marked
    `refresh_required: true` and `draft-spec`. It has to be updated with what
    the earlier features actually built before anyone implements it.

    Options:

    - **A. Pause this feature; finish and review the earlier feature first, then refresh this spec and rerun.**
      - Pros: this is the order the specification requires. No two automated
        runs edit the same files. The refreshed spec uses the real names and
        file formats instead of guesses.
      - Cons: this feature starts later.
    - **B. Run this feature now, with the earlier feature on hold.**
      - Pros: work starts right away.
      - Cons: requirement R44 (the definition-bundle round trip), half of the
        input-testing tables, and requirement R26 (the OpenAPI extension
        carries rate-limit, deprecation, and auth fields) all depend on code
        that does not exist yet. They would have to be skipped, faked, or
        built here, which the plan forbids. When the earlier feature lands,
        large parts would have to be merged again or redone.
    - **C. Run this feature now, alongside the earlier feature.**
      - Pros: both features move forward together.
      - Cons: two runs would change the same files at the same time, which
        means certain merge conflicts. This feature would also be built from
        an outdated spec.

    **Recommendation: A.** The specification and plan both require it. The
    cost of waiting is only time; the other options create rework in shared
    files. When this run is restarted, refresh `spec.md` first, as its
    "Refresh notes" section describes, and then rerun Phase 1 from the start.
message_to_agent: |-
  Phase 1 stopped at the prerequisite gate (plan Rule 1): spec 4b
  (2026-10-15-schematic-definition-vocabulary) has only completed its Phase 1
  of 6. Its bundle loader, endpoint metadata vocabulary, and auth override do
  not exist in code. This spec is still draft-spec with refresh_required: true.
  Do not start Phase 2 until 4b has landed and been reviewed, and this spec has
  been refreshed. Then redo Phase 1 from the start: the consumer and provider
  inventories, baseline, and timeout spike were deliberately not performed
  because 4b changes the files they would capture. The prerequisite inventory
  below is a snapshot of the tree on 2026-10-21 and must be refreshed.
  Phase 1 was rerun once on 2026-10-21 with nothing changed and stopped at
  the same gate (see "Rerun on 2026-10-21"); do not rerun it again until 4b
  is landed and reviewed and this spec has refresh_required: false.
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
