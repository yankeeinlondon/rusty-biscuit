# CI contract schemas

Three documents carry every decision this repository's CI makes. All are
defined once in [`scripts/ci/schema.py`](../../../scripts/ci/schema.py), which
is also their validator.

| Document | Version | Written by | Read by |
|---|---|---|---|
| Resolved plan | 4 | `scripts/ci/affected_scope.py --resolved-plan` | `ci.yml`, `ci-rollup`, `just ci-local --plan`, the pre-push hook |
| Validation receipt | 2 | the pre-push hook, `scripts/cross-check.sh` | `scripts/ci/local_evidence.py`, the planner, `ci-rollup` |
| Scope receipt | 1 | the pre-push hook (`local_evidence.py scope-record`) | `ci.yml` through `local_evidence.py scope-verify` |

`contract.json` in this directory is the field contract dumped from that module
so Rust tooling can assert against it without running Python. Regenerate it with
`python3 scripts/ci/schema.py`; `scripts/ci/test_schema.py` fails when it drifts.

> **Status.** `affected_scope.py` emits the resolved plan with
> `--resolved-plan`, or with `--plan-out` alongside the legacy scope document.
> Both receipt producers — the pre-push hook and `scripts/cross-check.sh` —
> write version 2; version 1 is read-only and never upgraded in place.
> `ci-rollup rollup --plan` derives its expected cells from `cells[]`, and
> still reads the legacy *policy* document for the governance of a
> `gates = false` package, which owns no cells at all.
>
> **The legacy `scope.json` projection is still live.** `ci.yml` and
> `just/ci-local.just` read it, and `affected_scope.py::legacy_scope_document`
> projects it from the same cells — a projection, not a second calculation, so
> the two documents cannot disagree about what CI will run. It carries the
> area fan-out (`scheduled_areas`, `area_matrix`, `area_slugs`) and the build
> owner fan-out (`build_owners`). `build_slices`, `build_artifacts`, and
> `build_runners` retain the package-keyed artifact inventory for diagnostics;
> they do not determine the job matrix.
> Deleting it means moving those consumers onto `cells[]` first; the governance
> metadata of a non-gating package has to move into the plan before the policy
> half can go.
>
> A third document, the rollup's own `ci-results.json`, is **not** defined here:
> it is Rust-owned by `scripts/ci-rollup.rs` and is at `schema_version: 4`,
> versioned independently of the plan's 4, the receipt's 2, and the baseline's
> 3. The plan fields that tool reads are asserted against `contract.json` by
> `plan_fields_match_the_frozen_contract`, so renaming one breaks a test rather
> than silently dropping a field serde never recognized.
>
> The version-1 field list was amended twice, both times additively and
> both times without a version bump because no consumer had read the document
> yet: `source_packages` and `reverse_dependencies` first (AC1 is stated over
> the difference between "changed" and "selected", and the OQ1 seam report needs
> the dependent names), then the optional `input_paths` (a package's build
> closure, which the gate-input identity is defined over) and
> `evidence_rejections`.
>
> Version 2 makes a carried plan self-sufficient: CI applies verified evidence
> to a matching scope receipt (`affected_scope.py --apply-to`) and re-projects
> `scope.json` from it without re-selecting, so the plan carries the
> `environments` table it was resolved against, each package's
> `l1_include_slow` and (for `gates = false`) its `exclusion`, and each cell's
> `reusable`. They are required, so a version-1 scope receipt misses as
> `scope-schema` and CI calculates scope itself — the miss the receipt
> contract permits — rather than hitting and then having to re-run selection.
>
> Version 3 adds **build records**
> (`fixes/2026-09-12-single-os-compile/spec.md`): the plan-level `builds[]`
> list and the per-cell `build` reference that names one of them. A version-2
> receipt carries no build records at all and misses as `scope-schema` for the
> same reason — the alternative would be inventing ownership for a selection
> this tool did not make.
>
> Version 3 also added the required `change_inventory`: the changed paths,
> normalized and bucketed once by the calculator so the plan renderer, the
> pre-push report, and `ci-reporting` all state the same thing about what
> changed.
>
> Version 4 is those two together. Build records and the change inventory each
> called themselves version 3 on separate branches, so "3" named two
> incompatible shapes: a document from either branch passed the version check
> and then failed on a field it never carried, which reads as corruption rather
> than as a version skew. 4 names the union, and the validator checks the
> version before the field set so the report says so. A version-2 or version-3
> scope receipt misses once as `scope-schema` for the same reason a version-1
> one did; validation receipts are untouched, so their version stays at 2 and
> nothing already-recorded is invalidated.

## Resolved plan

One document describing everything a run will do, consumed identically by local
validation and hosted CI. It replaces the unversioned scope document, whose
`excluded_environment` field could suppress a *matrix* entry while leaving the
*policy* expecting a CI result for it — the PR #76 seven-cell regression.

Identity and grouping are deliberately separate. **Package is the stored
identity** of every cell, artifact, baseline entry, and receipt. **Area is a
derived grouping field** carried alongside it for presentation and outcome
ownership. A cell whose `area` disagrees with its package record is invalid.

- `change_inventory` — the changed paths, normalized to one repository-relative
  POSIX spelling, de-duplicated, and sorted into exactly one of
  `configuration`, `documentation`, `source`, `other`, with per-bucket and
  total counts. A manual full-scope run consulted no diff and records
  `diff_available: false` with a reason instead of empty buckets, which would
  read as "nothing changed". It is computed from the paths alone and is a
  *sibling* of `change_class`, not a summary of it: `change_class` is derived
  from the gating packages a change selects, so a change to a `gates = false`
  package's Rust source correctly reports `change_class: documentation` beside
  a `source` bucket.
- `areas[]` — one entry per selected area, with the reason it was selected and
  the packages contributing to it. Nested areas such as `claudine/rendezvous`
  are their own entries, never folded into a parent.
- `packages[]` — the package records: gates, explicit Cargo `targets`, tiers,
  features, backends, native dependencies, and an optional `dependent_seam`
  describing downstream packages compiled inside this package's own check.
  A `gates = false` package holds a record with an empty gate list and no
  cells: a governed absence stays visible without demanding a result.
- `source_packages[]` and `reverse_dependencies[]` — which packages *changed*,
  and which direct reverse dependents were reported but selected nowhere. An
  unchanged dependent receives no area, package record, or cell (AC1); its
  seam is compiled inside the changed package's own `ubuntu-latest` check
  cell, whose `dependents` names it (Open Question 1, Option B).
- `environments[]` — the `.github/ci/environments.json` records the cells were
  resolved against, verbatim: the runner labels, native keys, and capabilities
  that evidence application and the legacy projection read.
- `l1_include_slow` on a package record, and `exclusion` on exactly the
  records whose `gates` list is empty — the policy facts the legacy matrix and
  rollup policy documents are projected from.
- `input_paths[]` on a package record — the manifest directories of its build
  closure, dev-dependencies included. The gate-input identity of a cell is
  computed over these plus that gate's global inputs; a plan that omits them
  makes its cells exact-tree-only for reuse. Optional, because only a verifier
  needs them.
- `evidence_rejections[]` — one coded reason per receipt cell that was refused,
  so a reviewer can tell a missing receipt from a rejected one. Optional.
- `cells[]` — one record per `{package, environment, gate}`, carrying
  `execution` (what will happen), `origin` (where the result comes from),
  `state` (what is true now), `reusable` (whether any local receipt may ever
  satisfy it — false for `check` and for the L1 host a companion suite needs),
  the target kinds covered, which gate supplied the compile coverage, and a
  selection reason.
- `builds[]` — the run-scoped build records the executing test cells consume
  (see below). Empty when nothing executes.
- `accepted_evidence[]`, `policy_gaps[]`, `prohibited_cells[]` — the three
  reasons a cell is not executed, kept as separate lists so none can be
  silently read as another.

`execution`, `origin`, and `state` are independent axes. The cross-field rules
that make a combination meaningful — a reused cell must name its evidence, an
accepted gap must name its governing policy entry, a prohibited cell can never
be scheduled — are enforced by `validate_resolved_plan`.

### Build records

A build record is **plumbing, not a result cell**: it describes one immutable
compile configuration, is keyed `{package, producer environment, build}`, and
is never baseline-eligible. Result identity stays `{package, environment,
gate}` and is untouched by any of this.

- `key` — the **planned build key**: sixteen hex digits of xxHash over the
  canonical form of `identity` below, computed only through `ci-build key`
  (`scripts/ci/build_key.py`). There is deliberately no second implementation
  and no fallback: an unavailable helper fails the plan.
- `identity` — every plan-known compile-affecting input, stored unhashed so a
  reader can see *why* two cells share or split a key: source commit, lockfile
  digest, pinned Rust and Nextest, compiler host, target triple, Cargo profile,
  encoded flags and config, linker, archive format, package, target kinds,
  isolated feature arguments, native inputs, archive includes, and sidecars.
- `producer` — the one native environment that compiles it. Two owners for one
  key is invalid.
- `artifact` — `build-<package>-<producer>-<key>`. Package-keyed like every
  other store; `build` is an internal artifact tier, never a result gate.
- `compatible_environments` and `compatibility_reason` — where the archive may
  execute, and why. Declared by the producer's contract in
  `environments.json`, never inferred from an OS name.
- `consumers[]` — the sorted `{environment, gate}` cells that execute it.

Derivation happens **after** evidence is applied to the cells, so a cell
satisfied by verified evidence or standing as a governed gap creates no
consumer demand and an all-reused plan schedules no owner at all. The
`--apply-to` overlay only *removes* demand — it computes no key, which is what
keeps the valid-receipt path free of a Rust toolchain. `lint` and `check` cells
never reference a build: Clippy is another compiler driver and check-only
target kinds may emit no executable (spec section 6).

`validate_resolved_plan` refuses a dangling reference, an unconsumed build,
two owners for one key, an artifact-name collision, a consumer outside its
producer's declared compatibility, unsorted or phantom consumers, a test
execution with no build, and any build attached to lint or check.

A validation receipt's cell may carry an optional
`build = {key, digest}`: the planned key and the producer's realized digest of
the archive whose binaries that cell actually executed, read from the manifest
the consumer verified rather than from any plan's expectation of it. It is
optional because a cell that compiled in place has no archive to name, and
because every receipt written before archives existed must stay readable; both
fields are present or neither is.

### The producer's manifest

`ci-build produce` writes one `<artifact>.manifest.json` per planned key, at
`MANIFEST_SCHEMA_VERSION` 2. It is defined in `scripts/ci-build-archive.rs`
rather than here, because it is Rust-owned on both sides — the producer writes
it and `ci-build verify` reads it — and is versioned independently of the plan.

It carries the plan's `key` and `identity` verbatim, the producer's *discovered*
inputs (rustc, Cargo and Nextest versions, compiler host, target, linker, and
the environment's arch/ABI/libc predicates), the archive's and each sidecar's
size and BLAKE3 digest, the expected test-binary inventory, the runtime assets
the archive carries, optional compiler-work counts, and per-stage timings.

`digest` is the **realized build digest**: xxHash over everything above except
`digest`, `timings`, and `compiler_work`. Those three are excluded because two
runs that produced byte-identical artifacts have to agree on the digest —
which also means that editing any *claim* in the manifest breaks it, so tamper
detection covers the description as well as the bytes.

### The consumer's verdict

`ci-build verify --verdict-out <file>` writes the same document it renders, at
`VERDICT_SCHEMA_VERSION`: `accepted`, the environment, the key, digest,
package, and producer it judged, whether the archive's own contents were
listed, every rejection, and `timings`.

`timings` is `{identity_ms, extract_ms, total_ms}` — identity and checksum
checks, then the archive's extraction and listing. It is written for a refusal
too: a cell that never started is exactly the one whose transfer and
verification cost a reader wants. `extract_ms` is how the consumer's extraction
stage is reported apart from its test time, because
`cargo nextest run --archive-file` extracts inside the run. See
[What each stage cost](../README.md#what-each-stage-cost).

## Validation receipt

Version 2 keys evidence per `{package, environment, gate}` instead of per
environment, so results from several hosts combine without overwriting one
another. Each cell carries its outcome, exit code, completion, counts, duration,
bounded failure detail, backend proof, report path, and its **gate-input
identity** — the hash of the build closure's tree entries plus the global inputs
that gate depends on. Two heads with equal gate-input identity for a cell may
share that cell's result; nothing else makes an older result reusable.

Verification *recomputes* that identity over both trees rather than trusting the
one the receipt stored: a stored identity is an unverified claim by the host
that wrote it, and both trees are present locally, so the comparison can be made
instead of believed. The stored value remains as provenance.

A gate that produced no report is recorded `partial`. It has an exit code and
nothing to attribute it to, so it is published (the run happened) and refused
for reuse (`incomplete-run`), rather than being made unpublishable or silently
credited as a tested cell.

A `complete` failing run is diagnostic evidence and is rejected for reuse. An
`interrupted` run contributes nothing; an interrupted *cell* inside a complete
run is rejected alone and leaves its completed passing siblings reusable.

### Version-1 receipts

Version 1 has no per-cell outcome: presence in a package list *is* the pass
claim. It is accepted only on exact tree identity, only as pass-only
whole-environment evidence, never through gate-input equivalence, and is never
upgraded in place. Its measurements render as the exact string
`not recorded (v1 receipt)`.

## Scope receipt

What the planner selected for one committed `base..head`, published on
`refs/notes/ci-local/scope` — one ref for every host, since scope is
OS-independent — and bound to the exact `{base, head, tree}`. It carries two
documents: `plan`, the canonical resolved plan exactly as `--plan-out` writes
it, and `scope`, the legacy `scope.json` projection `ci.yml` still fans out
from; `plan_schema_version` names the plan's version so a plan migration is
refused as `scope-schema` rather than parsed. `validate_scope_receipt` checks
structure only — that the carried plan validates and names the receipt's base
and head, and that the projection carries every key the workflow reads
(`SCOPE_PROJECTION_FIELDS`) and the plan's packages. The identity comparison
is the verifier's, in R3's order: schema, head, tree, base, then structure.

CI reads it from the event head only and, on a hit, writes both documents out
in place of running the planner; verified validation evidence is then applied
to the carried plan (`affected_scope.py --apply-to`) and the projection is
re-derived from the result, still without selection. Its miss codes are
`SCOPE_REJECTIONS`
(`scope-missing`, `scope-schema`, `scope-head-mismatch`, `scope-tree-mismatch`,
`scope-base-mismatch`, `scope-malformed`), kept apart from the cell rejections
below because they refuse a whole document rather than one outcome.

## Rejection vocabulary

Every refusal to accept evidence names one code from `schema.REJECTIONS`, so the
planner, the verifier, and `just ci-local --plan` describe the same refusal the
same way. Adding a code is a contract change.

### Build rejections

`schema.BUILD_REJECTIONS` is a third, separate vocabulary. `REJECTIONS` refuses
a cell's *outcome* and `SCOPE_REJECTIONS` refuses a whole scope document; these
refuse the **inputs** to a cell that has not run yet, and the three are never
reported in one list.

`ci-build verify` answers them in a versioned verdict document and exits `3` —
distinct from `2`, which still means the tool itself failed. Every one is an
infrastructure verdict a workflow reports, never a test result, and **never
something a consumer may repair by compiling a replacement**. `ci-build`'s own
`REJECTIONS` constant is asserted against `contract.json`'s
`vocabulary.build_rejections`, so a code cannot exist on only one side.

| Code | Refuses |
|---|---|
| `build-manifest-missing` | no manifest where the consumer was told to look |
| `build-manifest-malformed` | not JSON, or missing a required field |
| `build-manifest-schema` | written by another manifest generation |
| `build-digest-mismatch` | the manifest's own fields no longer hash to its `digest` |
| `build-key-mismatch` | the plan names no such key, or a different package/producer/artifact/identity for it |
| `build-source-mismatch` | built at a revision the plan does not resolve |
| `build-environment-incompatible` | this environment is not in `compatible_environments` |
| `build-runtime-incompatible` | the host's or the plan environment's arch/ABI/libc is not the one the archive was built for |
| `build-archive-missing` / `build-archive-corrupt` | the archive is absent, or its size or BLAKE3 digest disagrees |
| `build-sidecar-missing` / `build-sidecar-corrupt` | the same, for a declared sidecar |
| `build-asset-missing` | a declared `archive-includes` entry never reached the manifest |
| `build-inventory-incomplete` / `build-inventory-unexpected` | the archive contains fewer, or more, test binaries than the manifest declares |
