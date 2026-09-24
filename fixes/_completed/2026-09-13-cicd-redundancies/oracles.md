---
title: Phase 2 pending-contract oracles
created: 2026-09-14
phase: 2
spec: fixes/_complete/2026-09-13-cicd-redundancies/spec.md
plan: fixes/_complete/2026-09-13-cicd-redundancies/plan.md
rulings: fixes/_complete/2026-09-13-cicd-redundancies/rulings.md
tree: aad933bdb (branch `fix/cicd-improvements`)
host: macOS 27.0.0, Apple Silicon
---

# Phase 2 — Pending-Contract Oracles

Twenty-six pending fixtures were added across four Python suites and two Rust
suites. Every one was run **unwrapped** and observed failing for its recorded
reason; every one was then run **wrapped** and observed passing, so the three
suites are green and a real regression in a later phase is still visible.

A fixture that failed for a *setup* reason — an import error, a missing
fixture file, a panic from a helper rather than from an assertion — is not an
oracle. Two were caught and repaired during this phase; both are recorded in
[Fixtures repaired](#fixtures-repaired).

## How to run them

| what | command |
|---|---|
| held (the normal suite; pending fixtures invert and pass) | `python3 scripts/ci/test_affected_scope.py` |
| unwrapped (see which contracts now hold) | `BISCUIT_PROMOTE_PENDING=1 python3 scripts/ci/test_affected_scope.py` |
| Rust, held | `cargo nextest run -p test-toolkit --test ci_workflow_contracts` |
| Rust, unwrapped | `BISCUIT_PROMOTE_PENDING=1 cargo nextest run -p test-toolkit --test ci_workflow_contracts` |

`BISCUIT_PROMOTE_PENDING=1` was already the Python decorator's behavior. It was
added to both Rust `pending_contract` wrappers in this phase so all three
languages share one switch.

**Promotion is deleting the decorator or the wrapper**, never setting the
variable. A pending fixture whose body passes fails its suite (`ContractLanded`
in Python, an explicit panic in Rust), so a contract cannot land and leave a
fixture behind claiming it has not.

---

## Work-group 2.A — Python plan/scope fixtures

### Suite-ownership registry (`test_affected_scope.py`)

Seven fixtures, one oracle. The accessors `suite_registry()` and
`validate_suite_registry()` raise it, so the string is one this suite owns
rather than an `AttributeError`'s wording.

> `affected_scope declares no suite-ownership registry: SUITE_REGISTRY is not
> defined, so no suite has a declared owner, recipe, or environment`

| fixture | asserts |
|---|---|
| `test_every_registered_suite_has_exactly_one_owner` | the companion half of the registry is the specification's ownership table exactly; every entry has a non-blank recipe and declares `ubuntu-latest` (R7, S3) |
| `test_an_unknown_suite_name_fails_validation` | a declaration naming an unregistered suite is rejected **by name** |
| `test_a_doubly_owned_suite_fails_validation` | a suite two packages claim is rejected by name |
| `test_a_recipe_less_suite_fails_validation` | a blank `recipe` is rejected by name |
| `test_a_registered_suite_nobody_declares_fails_validation` | a registered suite no package declares is rejected by name |
| `test_every_registered_python_suite_names_a_shipped_file` | passive corpus check: every `*.py` entry names a file under `scripts/ci/` |
| `test_the_shipped_registry_validates_against_its_own_owners` | the shipped table validates cleanly |

**Target contract the fixtures pin** (Phase 4 implements it):

```python
SUITE_REGISTRY: dict[str, dict[str, str]]
    # name -> {"owner", "recipe", "environment", "kind"}   kind: cargo | companion

def validate_suite_registry(registry, declarations) -> list[str]
    # declarations: package -> the suite names its
    # [package.metadata.ci.tests].companion-suites claims.
    # One problem per defect, each naming the offending suite.
```

The expected companion table is `homelab-frontend` → `homelab-server`, the ten
`scripts/ci/test_*.py` suites → `repo-deps`, and `test-audit-typecheck` /
`test-audit-vitest` → `test-toolkit`.

### Path-to-owner selection (`test_affected_scope.py`, `ToolingPathOwnershipTests`)

Four pending fixtures, one oracle:

> `<path> must select exactly <expected>, got []: the path-to-owner table does
> not select its owner`

| fixture | path(s) | today | target |
|---|---|---|---|
| `test_a_policy_store_change_selects_repo_deps_alone` | `.github/ci/ci-baseline.toml`, `.github/ci/environments.json` | `[]` | `['repo-deps']` |
| `test_a_workflow_change_selects_test_toolkit_alone` | `.github/workflows/{_area-ci,ci,_package-ci}.yml` | `[]` | `['test-toolkit']` |
| `test_a_test_audit_change_selects_test_toolkit_alone` | `tools/test-audit/{package.json,src/cli.ts}`, `pnpm-lock.yaml`, `pnpm-workspace.yaml` | `[]` | `['test-toolkit']` |
| `test_the_owners_manifest_is_an_explicit_trigger` | `scripts/Cargo.toml` | `[]` | `['repo-deps']` |

Plus one on the flag itself:

| fixture | oracle |
|---|---|
| `test_the_ci_tooling_flag_is_gone_from_the_plan` | `the plan still carries flags.ci_tooling; ownership replaces the boolean, it does not sit beside it` |

**Three are deliberately NOT pending**, and they are what stops Phase 4 from
satisfying the table by selecting an owner for everything:

- `test_a_scope_calculator_change_selects_repo_deps_alone` — R13.4: `scripts/**`
  is `repo-deps`'s own package directory, so source ownership already selects
  it. Phase 4 must **not** add a `scripts/**` trigger that double-selects.
- `test_test_toolkit_source_still_selects_itself_alone` — same, for
  `tools/test-toolkit/**`. Pinned so deleting `CI_TOOLING_PATHS` cannot narrow
  it and a `.github/workflows/**` trigger cannot widen it.
- `test_documentation_selects_neither_tooling_owner` — `docs/topics/ci-cd.md`,
  `<area>/docs/...`, `<package>/README.md` select nothing, and must keep
  selecting nothing.

> **RESOLVED in Phase 4** — see `rulings.md` R13.6. The trigger is implemented
> for both tooling owners' manifests, the fixture stands as written, and the
> exception's scope is recorded. The question as posed follows.
>
> **Open question for Phase 4 — the owner's manifest.**
> `changed_package_ids` states, repository-wide, that *"`Cargo.toml` and
> lockfile edits select no package by themselves; the next source edit
> exercises the resulting package graph."* Spec §2 names *"its manifest"* as an
> explicit trigger for the owner. The two disagree.
> `test_the_owners_manifest_is_an_explicit_trigger` pins the specification's
> side and is the one fixture whose promotion requires a ruling rather than
> only an implementation. Phase 4 must either amend the ruling record with a
> `scripts/Cargo.toml` exception and say why it is not general, or amend the
> fixture. It must not do neither.

### Change inventory (`test_affected_scope.py`, `ChangeInventoryTests`)

Five fixtures, one oracle:

> `the resolved plan carries no change inventory: change_inventory is absent
> from the resolved plan, whose fields are [accepted_evidence, areas, base,
> cells, change_class, environments, evidence_rejections, flags, full_scope,
> full_scope_gates, head, job_estimate, packages, policy_gaps, preflight_os,
> preflight_reason, prohibited_cells, reverse_dependencies, schema_version,
> source_packages]`

| fixture | asserts |
|---|---|
| `test_the_inventory_buckets_every_changed_path_exactly_once` | at least `configuration`/`documentation`/`source`/`other`; exhaustive; no path in two buckets |
| `test_each_bucket_is_sorted_normalized_and_counted` | sorted, `/`-spelled, no `./` prefix, per-bucket counts match, `total` is the sum |
| `test_a_rename_contributes_one_logical_path` | three spellings of one destination collapse to one entry |
| `test_a_full_scope_run_records_no_diff_inventory` | `--all` sets `diff_available: false` with a reason, never an empty list |
| `test_change_class_is_retained_beside_the_inventory` | R8: `change_class` stays and is allowed to disagree with the buckets |

**Target shape:**

```python
{"diff_available": True,
 "paths":  {"configuration": [...], "documentation": [...], "source": [...], "other": [...]},
 "counts": {"configuration": 0, "documentation": 0, "source": 0, "other": 0, "total": 0}}
```

### Schema bump (`test_schema.py`)

| fixture | oracle |
|---|---|
| `test_the_plan_schema_carries_the_change_inventory_at_version_3` | `the resolved plan schema must be version 3 once it carries the change inventory, got 2` |
| `test_a_plan_without_the_inventory_is_rejected` | `a plan without a change inventory must be rejected; validate_resolved_plan reported nothing` |

**NOT pending:** `test_the_other_schema_counters_do_not_move` pins R9's other
half — `RECEIPT_SCHEMA_VERSION` 2, `LEGACY_RECEIPT_SCHEMA_VERSION` 1,
`SCOPE_RECEIPT_SCHEMA_VERSION` 1 — so the inventory bump cannot drag a receipt
version with it and invalidate evidence this fix never touched.

**B5 reminder for Phase 6:** `scripts/ci-rollup.rs:70`'s
`const PLAN_SCHEMA_VERSION: u32 = 2` must bump in the same commit, or every
`ci-rollup rollup --plan` invocation fails. No Phase 2 fixture covers it because
`ci-rollup` reads the constant rather than the schema module; Phase 6's own
change is where it lands.

### Scope-receipt migration (`test_resolved_plan.py`)

| fixture | oracle |
|---|---|
| `test_a_version_2_scope_receipt_misses_once_with_scope_schema` | `a version-2 scope receipt must miss with scope-schema once the plan carries the change inventory; it validated cleanly` |

**NOT pending:** `test_a_current_generation_scope_receipt_still_validates` is
the non-vacuity guard — it proves the rejection will come from the version
comparison rather than from a receipt the fixture builds wrongly.

### Preflight (`test_affected_scope.py`, `test_ci_local.py`)

| fixture | oracle |
|---|---|
| `PreflightSkipTests.test_a_documentation_only_change_expands_no_preflight_job` | `documentation-class preflight must expand no job, got ['ubuntu-latest'] for docs/architecture.md` |
| `PreflightPrerequisiteTests.test_preflight_runs_no_test_suite` | `preflight must run no test suite; it still runs 9: [...]` |

The nine named in that message are exactly the nine the rulings record's
baseline capture lists (`ci.yml:418-437`), eight of which `ci-tooling` runs
again on the same commit.

**NOT pending:**

- `test_a_package_change_keeps_its_preflight_breadth` — R10 narrows the
  documentation class alone; a planner that emptied preflight for real package
  work would pass the pending fixture and delete the bootstrap gate.
- `test_preflight_retains_its_prerequisite_steps` — the suites must be
  *removed*, not the job emptied. Slimming preflight by deleting its toolchain,
  metadata, and canonical-recipe checks would satisfy AC1 while deleting what
  the fan-out depends on.

### Empty matrix (`test_ci_local.py`, `EmptyMatrixGuardTests`)

| fixture | oracle |
|---|---|
| `test_both_matrix_jobs_carry_a_scalar_guard` | ``preflight must be guarded by a scalar plan output before matrix expansion; its `if:` is None`` |
| `test_a_documentation_only_plan_skips_both_matrix_jobs` | `a documentation-only plan must publish an empty preflight matrix, got ['ubuntu-latest']` |

**NOT pending:**

- `test_neither_matrix_job_is_labelled_with_a_matrix_expression` — both jobs
  already omit `name:` (R12); pinned so adding the scalar guard cannot arrive
  with a display name that reaches the Checks tab as raw expression text.
- `test_a_plan_with_no_executing_test_cell_still_fans_out_its_area` — AC16.
- `test_a_scheduled_plan_publishes_both_matrices` — non-vacuity for the class.

> **Correction to the plan's fixture list.** Plan 2.A asks for a
> *zero-executing-cell* plan. There is no such plan: `lint` and `check` cells
> carry `reusable: false` by design, because a local receipt holds no JUnit
> evidence for them, so they always execute. The reachable shape is
> *every **test** cell reused*, and that is what the fixture builds. It is also
> why the all-reused shape asserts the matrices are **not** empty — AC16
> requires that area to still fan out far enough to publish its result slice.

---

## Work-group 2.B — Rust fixtures

### Workflow contracts (`tools/test-toolkit/tests/ci_workflow_contracts.rs`)

| fixture | oracle (observed, unwrapped) |
|---|---|
| `ci_defines_exactly_the_six_surviving_top_level_jobs` | `ci.yml must define exactly ["area-ci", "ci-gate", "ci-reporting", "preflight", "scope", "validation"], got ["area-ci", "biscuit-tui-captured-stdout", "ci-gate", "ci-tooling", "preflight", "scope", "summary", "validation"]` |
| `ci_gate_needs_exactly_the_four_surviving_blocking_jobs` | `ci-gate must fold exactly ["validation", "scope", "preflight", "area-ci"], got ["validation", "scope", "preflight", "area-ci", "biscuit-tui-captured-stdout", "ci-tooling"]` |
| `the_windows_captured_stdout_workflow_is_absent` | `biscuit-tui-windows-captured-stdout.yml must be absent once the test is ordinary L1 evidence` |
| `ci_reporting_is_advisory_and_no_blocking_job_is` | ``ci-reporting must exist; `summary` has not been replaced`` |

`ci_gate_needs_exactly_...` also asserts, inside the same body, the two things
spec D6 forbids changing: the `success\|skipped` accept clause and the literal
`name: ci-gate` the `protect-your-bacon` ruleset requires. Those halves pass
today; the `needs` list is what fails, which is the intended oracle.

`the_windows_captured_stdout_workflow_is_absent` additionally covers **B4** —
the `biscuit_tui` scope output (`ci.yml:75`, `:264`) and the `"biscuit-tui"`
area-flag entry (`affected_scope.py:1958-1961`) that the retiring job is the
sole consumer of. The plan's Phase 8 does not name either.

### Companion completeness (`scripts/ci-rollup-tests.rs`)

> **PROMOTED in Phase 5 (2026-09-14).** All three `pending_contract` wrappers
> are gone and the fixtures pass against the per-suite `companions` record.
> `pending_contract` itself was deleted from this file with its last caller;
> `ci_workflow_contracts.rs` keeps its own copy for the Phase 8 fixtures.

| fixture | oracle (observed, unwrapped) |
|---|---|
| `one_declared_suites_success_cannot_cover_anothers_skip` | `a successful companion must not cover a second declared suite that was skipped` |
| `a_status_naming_an_unregistered_suite_fails_validation` | `an unregistered companion suite in a producer status is a mis-wired producer, not evidence` |
| `a_companion_with_no_counts_renders_not_recorded_never_zero` | ``an unmeasured companion must reach the report as `not recorded`, got: {...}`` |

**These three are written against the producer status's JSON, not against a
`ProducerStatus` literal.** That is deliberate: the JSON is the real artifact
`_package-ci.yml` uploads, and it is the boundary that survives Phase 5's
replacement of `companion: Option<String>`. The target `companions` object is
inert today — serde ignores unknown fields — so each fixture fails on the
legacy `companion` string alone, and becomes load-bearing the moment Phase 5
reads the object.

**Target producer-status shape the fixtures pin:**

```json
{"package": "...", "job": "L1", "result": "success", "environment": "ubuntu-latest",
 "companions": {"<suite>": {"outcome": "success|failure|skipped",
                            "tests": 218, "duration_s": 1,
                            "reason": "typecheck gate reports no test counts"}}}
```

`reason` is what AC13 renders in place of counts. S3 measured the concrete
case: `pnpm --dir tools/test-audit typecheck` is `tsc --noEmit`, a pass/fail
gate with no test cardinality, and no flag can invent one.

**NOT pending:** `the_legacy_single_companion_outcome_still_downgrades` pins
today's behavior, so Phase 5 cannot satisfy the three contracts by changing
what the fixture builds rather than what the rollup reads.

---

## Work-group 2.C — Windows discovery

Asserted from `ci_workflow_contracts.rs` rather than from a Biscuit TUI test,
because the subject is *discoverability*: a test that no recipe selects cannot
assert its own reachability. That is precisely the failure mode
`features/2026-07-24-devops/ci-failure-inventory.md` records.

| fixture | oracle (observed, unwrapped) |
|---|---|
| `the_windows_captured_stdout_test_is_discoverable_as_ordinary_l1` | ``the test must be reachable by the canonical L1 recipe, but it is still `#[ignore]`d`` |

The body asserts all four conditions at once: no `#[ignore]`, no
`sleep(Duration::from_millis(`, the `#![cfg(windows)]` inner attribute
**retained** (spec D4 — the `cfg` is how the tier contract expresses
"Windows-only"), and no `test-windows-captured-stdout` recipe in
`biscuit-tui/justfile`. All four are present today; the `#[ignore]` assertion
fires first.

**NOT pending:**
`the_biscuit_tui_cli_l1_cell_already_compiles_the_terminal_test_target` pins
the starting advantage Phase 7 depends on —
`biscuit-tui/cli/Cargo.toml`'s `[package.metadata.ci.tests] features =
["terminal-tests"]`. Removing it would leave the test green-by-absence, which
is worse than the status quo.

> **B3 stands, and Phase 7 still owns it.** S2 established that
> `F2 precondition HELD` never reaches captured output, because
> `establish_console()` redirects `STD_OUTPUT_HANDLE` to `CONOUT$` before
> `assert_console_precondition` prints. No Phase 2 fixture can assert a log
> line the test does not emit, so the recommended repair — deleting the
> unnecessary `STD_OUTPUT_HANDLE` redirect — is verified by Phase 7's native
> Windows checkpoint, not here.

---

## Fixtures repaired

Two fixtures failed for a setup reason and were repaired before this record
was written. Both are mechanism-level traps worth recording.

### `unittest.subTest` cannot be used inside `@pending`

A `subTest` block records its failure on the result object instead of raising,
so the decorator saw the body return cleanly and raised `ContractLanded` —
reporting seventeen unbuilt contracts as landed while the sub-test failures
were also printed. Every pending body now loops without `subTest` and raises on
the first offender; a comment in `test_every_registered_suite_has_exactly_one_owner`
records why.

### A helper's panic is not an oracle

`ci_reporting_is_advisory_and_no_blocking_job_is` called `job_block`, which
panics with *"ci.yml must define the `ci-reporting:` job"* for a job that does
not exist. The wrapper correctly refused it:

```text
pending contract AC12 failed, but not for the recorded reason.
Expected the failure to mention "ci-reporting must"; the fixture itself is
probably broken.
```

`optional_job_block` was added so the fixture owns its own wording. This is the
`pending_contract` mechanism working exactly as designed, and it is why the
wrapper checks the message rather than merely observing a failure.

---

## Checkpoint

| suite | tests | held (normal run) | unwrapped |
|---|---:|---|---:|
| `scripts/ci/test_affected_scope.py` | 152 | OK | 18 fail |
| `scripts/ci/test_schema.py` | 48 | OK | 2 fail |
| `scripts/ci/test_resolved_plan.py` | 45 | OK | 1 fail |
| `scripts/ci/test_ci_local.py` | 68 | OK | 3 fail |
| `test-toolkit::ci_workflow_contracts` | 99 | 99 passed | 5 fail |
| `repo-deps::bin/ci-rollup` | 182 | 182 passed | 3 fail |

Every unwrapped failure above is an `AssertionError` / assertion panic raised
by the fixture itself. None is an import error, a missing fixture file, or a
helper's own wording.

The remaining seven Python CI suites were run unchanged and stay green:
`test_constraints.py`, `test_evidence_reuse.py`, `test_local_evidence.py`,
`test_publish_gaps.py`, `test_reuse_validation.py`, `test_runner_loss.py`.
