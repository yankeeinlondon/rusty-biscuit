# CI contract schemas

Five documents carry every decision this repository's CI makes. All are
defined once in [`scripts/ci/schema.py`](../../../scripts/ci/schema.py), which
is also their validator.

| Document | Version | Written by | Read by |
|---|---|---|---|
| Resolved plan | 7 | `scripts/ci/affected_scope.py --resolved-plan` | `ci.yml`, `ci-rollup`, `just ci-local --plan`, the pre-push hook |
| Validation receipt | 2 | the pre-push hook, `scripts/cross-check.sh` | `scripts/ci/local_evidence.py`, the planner, `ci-rollup` |
| Scope receipt | 1 | the pre-push hook (`local_evidence.py scope-record`) | `ci.yml` through `local_evidence.py scope-verify` |
| Expected manifest | 2 | `just _expected_manifest`, on the execution target | `scripts/ci/completion.py` |
| Completion record | 1 | `scripts/ci/completion.py`, on success only | the area's `ci-rollup verdict` |

The first three describe what CI will do; the last two are how a producer job
proves it did it.

`contract.json` in this directory is the field contract dumped from that module
so Rust tooling can assert against it without running Python. Regenerate it with
`python3 scripts/ci/schema.py`; `scripts/ci/test_schema.py` fails when it drifts.

`archive_guard_cases.json` is the second cross-language document here, and the
only hand-authored one: an accept/reject corpus for the plan's `archive_guard`
scope. Each case carries the scope fragment, whether the contract accepts it,
and the rule it exercises. `scripts/ci/test_schema.py` runs it through
`validate_resolved_plan` and
`tools/test-toolkit/tests/ci_workflow_contracts.rs` through
`GuardPlan::from_plan_json`, so a case added here fails both suites until both
readers agree on it. It states behavior the generated field table cannot, which
is why it is written rather than dumped; the verdict is all it fixes, and each
side keeps its own tests for the wording it produces.

> **Status.** `affected_scope.py` emits the resolved plan with
> `--resolved-plan`, or with `--plan-out` alongside the legacy scope document.
> Both receipt producers — the pre-push hook and `scripts/cross-check.sh` —
> write version 2; version 1 is read-only and never upgraded in place.
> `ci-rollup rollup --plan` derives its expected cells from `cells[]`, and
> still reads the legacy *policy* document for the governance of a
> `gates = false` package, which owns no cells at all.
>
> **The `scope.json` projection is still live.** `ci.yml` publishes its
> outputs, and `affected_scope.py::legacy_scope_document` projects it from the
> same plan — a projection, not a second calculation, so the two documents
> cannot disagree about what CI will run. It carries the area fan-out
> (`scheduled_areas`, `area_slugs`, `area_rows`) and the build owner fan-out
> (`build_owners`). `build_slices`, `build_artifacts`, and `build_runners`
> retain the package-keyed artifact inventory for diagnostics; they do not
> determine the job matrix. It carries no per-package environment list: the
> `matrix` and `area_matrix` projections were retired in
> `2026-09-19-direct-cell-execution`, and `just/ci-local.just` reads the
> plan's package records and cells instead. The governance metadata of a
> non-gating package has to move into the plan before the policy half can go.
>
> A third document, the rollup's own `ci-results.json`, is **not** defined here:
> it is Rust-owned by `scripts/ci-rollup.rs` and is at `schema_version: 5`,
> versioned independently of the plan's 7, the receipt's 2, and the baseline's
> 3. The plan fields that tool reads are asserted against `contract.json` by
> `plan_fields_match_the_frozen_contract`, so renaming one breaks a test rather
> than silently dropping a field serde never recognized.
>
> Why each document is at its version, and what each generation changed, is
> recorded in one place: [CI schema versions](../../../docs/cicd/schema-versions.md).
> Update it in the same change as any version bump.

## Resolved plan

One document describing everything a run will do, consumed identically by local
validation and hosted CI. It replaces the unversioned scope document, whose
`excluded_environment` field could suppress a *matrix* entry while leaving the
*policy* expecting a CI result for it — the PR #76 seven-cell regression.

Identity and grouping are deliberately separate. **Package is the stored
identity** of every cell, artifact, baseline entry, and receipt. **Area is a
derived grouping field** carried alongside it for presentation and outcome
ownership. A cell whose `area` disagrees with its package record is invalid.

- `event`, `deferred_environments`, `proven_environments` — optional, absent
  rather than empty, so a plan resolved without an event is byte-identical to
  one from before they existed and the version does not move for them (R9).
  `event` is
  the GitHub event the plan was resolved for; `deferred_environments` lists
  the table's environments that event does not schedule, each with the
  `events` that do; `proven_environments` lists the environments an earlier
  run of another event already validated for this tree (a push to `main`
  after a reused pull request validation plans only the rest). Neither list
  may name an environment the plan also carries in `environments`.
- `change_inventory` — the changed paths, normalized to one repository-relative
  POSIX spelling, de-duplicated, and sorted into exactly one of
  `configuration`, `documentation`, `source`, `other`, with per-bucket and
  total counts. A manual full-scope run consulted no diff and records
  `diff_available: false` with a reason instead of empty buckets, which would
  read as "nothing changed". It is computed from the paths alone and is a
  *sibling* of `change_class`, not a summary of it: `change_class` is derived
  from the gating packages a change selects, so a change to a `gates = false`
  package's Rust source correctly reports `change_class: documentation` beside
  a `source` bucket. `deleted` names the subset the diff reports as removed,
  gated exactly like `paths` and `counts` and declared rather than inferred:
  `git diff --name-only` cannot tell a deletion from a path that is simply
  absent.
- **Every path list in the plan is repository-relative and stays there.**
  One POSIX spelling, no leading `./`, no surrounding whitespace, and — because
  every consumer resolves a listed path against the checkout root — no absolute
  path, no Windows drive prefix, and no `..` component. A path that leaves the
  checkout is rejected by the validator rather than resolved: joining it against
  the root would discard the root, and a *missing* escaping path would otherwise
  be indistinguishable from an ordinary deletion. Applies to
  `change_inventory.paths.*`, `change_inventory.deleted`, and
  `archive_guard.paths` alike; `test_toolkit::archive_guard` enforces the same
  rules on the reading side.
- `archive_guard` — whether the archive-path guard was selected and, if so,
  whether it scans the full eligible corpus or an explicit path list. Exactly
  one of `{selected: false, reason}`, `{selected: true, mode: "full", reason}`,
  or `{selected: true, mode: "changed", paths, reason}`; `paths` is present
  exactly in `changed` mode, and an explicitly empty list there is the real
  "nothing eligible changed" state, never an implicit full scan. The guard is
  Linux-hosted, so an event that schedules no `ubuntu-latest` records
  `selected: false` rather than adding a runner to that event.
  `BISCUIT_ARCHIVE_GUARD_PLAN` hands a document straight to
  `test_toolkit::archive_guard::GuardPlan`, which the validator never sees, so
  that reader enforces this whole shape again — including the document's
  `schema_version` against `PLAN_SCHEMA_VERSION`, the closed field set, the
  non-blank `reason`, and the sorted, unique `paths`. A plan it refuses is an
  error, never an empty scan.
- `areas[]` — one entry per selected area, with the reason it was selected, the
  packages contributing to it, and the `execution_path` that area dispatches
  through — always `rows`, the only form a shipped workflow implements; the
  environment-list form is refused. Nested areas such as `claudine/rendezvous`
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
  selection reason. An optional `companions_only` on a `lint` cell says the
  cell's required work is its companions and not the package's own Clippy —
  the shape a guard-only selection takes, where nothing selected the package.
  An executing test cell also carries the `profile` its
  nextest selection runs under — the same answer its expected-test listing and
  its gate command both read — and `requires_node` where that cell provisions
  Node and pnpm. An executing L2 cell also carries `backends`: the
  sorted subset of the package's `l2_backends` its environment can host, which
  is what the producer sets `BISCUIT_TEST_REQUIRED_BACKENDS` to and what
  `completion.py` demands a `backend-proofs.json` entry for. A gap, reused, or
  prohibited cell carries none; the GUI backends stay in the package record
  only. An executing L1 cell a changed test input selected carries
  `test_filter`: the nextest filterset naming exactly the tests that read the
  file, which the producer intersects with the tier expression
  (`BISCUIT_TEST_NARROW`). Only an exact-tree receipt cell carrying the same
  filter, from any host, satisfies it
  ([`docs/cicd/test-inputs.md`](../../../docs/cicd/test-inputs.md)). Optional;
  absent means the whole tier.
- `skip_policy` — the snapshot of [`ci-baseline.toml`](../ci-baseline.toml)'s
  approved exact-skip budget: `source` and `content_hash` say which file was
  read and what it hashed to, and `entries[]` carries the approvals that apply
  to this plan's cells, each with its owner, reason, source run, optional
  `backend` and `expiry`, and the exact test identities it covers. Producers
  and the area audit read this and never the file, so a run cannot be judged
  against a policy different from the one it was planned with. An expired
  approval, or one naming a cell the plan does not carry, is refused here.
- `rows` — the dispatch rows derived from the executing cells: per area, the
  `test`, `check`, `lint`, and `wsl` sets plus a scalar `has_*_rows` guard for
  each. A row carries dispatch identity alone — `{package, gate, environment,
  runner}` — and the four sets partition that area's executing cells with no
  duplicate key. Optional and derived: `affected_scope.row_sets` computes it,
  and every consumer re-derives rather than trusting a carried copy.
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
re-derived from the result, still without selection. A plan schedules the
environments of the event it was planned for (its optional `event`), so
`scope-verify --event` refuses a receipt planned for another event, or for
none. Its miss codes are `SCOPE_REJECTIONS`
(`scope-missing`, `scope-schema`, `scope-head-mismatch`, `scope-tree-mismatch`,
`scope-base-mismatch`, `scope-event-mismatch`, `scope-malformed`), kept apart
from the cell rejections below because they refuse a whole document rather
than one outcome.

## Expected manifest and completion record

The two documents a producer job proves itself with, added by
`features/2026-09-19-direct-cell-execution`. They version independently of the
plan and the receipts: how a job proves its cell is not a reason to invalidate
a resolved plan, and the reverse.

**The expected manifest** (version 2) is what `cargo nextest list` found on the
execution target, written by `just _expected_manifest` from the same
`_tier_filter` expression the gate ran with, in the same job, with the same
nextest binary. Per package it carries three disjoint identity sets — `tests`
(must report a result), `ignored` (`#[ignore]` excused it), and `excluded` (the
tier expression did not select it) — plus the provenance without which a
listing proves nothing about a report: `environment`, `tier`, `target`,
`nextest_version`, `from_archive`, and the `selection` (`filter`, `profile`,
`test_args`) it applied. Optional `backends` and `companion_suites` record what
the job was *told* to require; they are cross-checked against the plan rather
than trusted.

A test compiled out by `cfg` on this target appears in none of the three sets:
it does not exist here. That is why a manifest listed on another target is
refused for the comparison outright rather than diffed, and in archive mode the
recorded target is the **producer's** — `cfg` was evaluated when the archive was
built.

Version 1 recorded only the selected identities, so an identity absent from the
report could be a `cfg`-absent test, an ignored one, another tier's, or a lost
one — four verdicts wearing the same silence.

**The completion record** (version 1) is `scripts/ci/completion.py`'s output,
keyed `{package, environment, gate}` like every other stored result. It binds
the tested revision, the run and attempt, the resolved nextest version, the
report inventory the comparison read, and — where the cell has them — the build
key, the gate inputs, the companion suites, and the backends. It is written
**only after validation succeeds** (ruling R10), so `complete: true` cannot
exist without the evidence behind it, and a refusal removes any record an
earlier validation left at the same path. It ships as its own
`completion-<package>-<gate>-<environment>` artifact, separate from the
`always()` status artifact that stays the failure-path diagnostic.

> **The tightened rule.** A missing result is a failure even when its identity
> appears in a skip approval. An *observed* `<skipped/>` may be excused by an
> unexpired entry in the plan's `skip_policy`; silence may not be excused by
> anything. An entry naming no `tests` approves any observed skip in its cell.

**How the audit reads it.** `ci-rollup rollup` attaches a `completion` to every
cell the plan executes — check and lint included — and to no other. It finds
the record by the artifact name `scripts/ci/cell_contract.py` derives, then
requires that the record name that cell, claim `complete`, name the plan's
`head` and the cell's planned `build`, name this run when `--run-id` is given,
and declare exactly the reports uploaded for the cell. It recomputes no expected
identity and no skip: the producer did both before writing the record.
`ci-rollup verdict` blocks an otherwise-green cell whose record fails any of
that (`completion-unproven`), and refuses a record for a cell the plan did not
execute (`completion-unplanned`). Any attempt's record is accepted, so a rerun
of other jobs keeps a passing cell's earlier proof.

A cell with no `completion` — reused, governed, prohibited, or rolled up
without a plan — is the **legacy path**: `--expected-manifest` (version 1 only)
and the exact skip budget apply to it as before, and `skip_evidence_degraded`
is set only there. A version-4 result document carries no `completion` at all,
so it is refused rather than read as a legacy slice.

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

### Completion rejections

`schema.COMPLETION_REJECTIONS` is a fourth vocabulary, and the only one decided
**on the producer**, before any artifact is published. Each code means "this job
did not prove what it was asked to prove" — never "a test failed" and never "an
input was unreadable by the audit". `scripts/ci/completion.py` exits `1` for a
cell that did not prove itself and `2` when it could not read its own inputs,
so an unreadable input is never reported as an invented test failure.

| Code | Refuses |
|---|---|
| `completion-plan-unreadable` | the plan is absent, malformed, or carries no record for this package |
| `completion-cell-unknown` | the plan carries no such cell, or carries it twice |
| `completion-cell-not-executing` | the cell is reused, governed, or prohibited; nothing ran here |
| `completion-manifest-missing` | a test gate with no expected-test listing to compare against |
| `completion-manifest-schema` | a listing from another manifest generation |
| `completion-manifest-provenance` | the listing cannot say which environment, tier, nextest, or package it describes |
| `completion-manifest-target` | listed on a target other than the one that executed |
| `completion-manifest-selection` | selected with anything but the cell's gate's own tier expression (`schema.CANONICAL_SELECTION`: another tier's marker, an inverted or narrowed expression, a scope naming another package, a predicate no tier uses), another profile, `--run-ignored`, or backends/suites the plan does not declare |
| `completion-report-missing` / `completion-report-malformed` | a staged report was never written, or does not parse |
| `completion-report-duplicate` | one result staged twice, or one identity reported by two selections |
| `completion-test-missing` | an expected identity that no report mentions |
| `completion-test-unexpected` | a reported identity the listing did not expect |
| `completion-test-failed` | the final attempt failed |
| `completion-skip-unapproved` | an observed skip with no unexpired approval |
| `completion-expected-empty` | nothing expected and no plan-recorded reason |
| `completion-companion-incomplete` / `completion-companion-failed` | a declared suite did not complete, or failed |
| `completion-backend-unproven` | a declared L2 backend that nothing proves drove a test |
