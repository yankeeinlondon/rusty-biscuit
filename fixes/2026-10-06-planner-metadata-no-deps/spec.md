---
kind: fix
name: planner-metadata-no-deps
date: 2026-10-06
status: draft-spec
related:
  - 2026-10-06-archive-key-helper
  - 2026-10-06-archive-guard-full-tree
reviewed: false
review_iterations: 0
clarified: false
implemented: false
human_review: true
$schema:
  status: |-
    enum(
        draft-spec,
        finalized-spec,
        planned,
        implemented,
        review-findings,
        human-in-the-loop,
        completed,
        on-hold,
        abandoned
    ) -> an indicator of progress for this specification
  reviewed: boolean -> indicates whether the specification file has been reviewed by another agent from the one which created the spec
  reviewed_by: string -> the agent and model used in the spec review
  reviewed_on: date -> the date the spec was reviewed
  review_iterations: number -> the number of implementation reviews have taken place in the review/fix cycle
  clarified: boolean -> indicates whether the specification was built -- _in part_ -- with the 'clarify.md' prompt
  implemented: boolean -> indicates whether this spec's plan has been implemented
  implemented_by: string -> the agent who implemented the plan
---

# The planner reads workspace metadata without resolving dependencies

## Summary

The CI planner (`scripts/ci/affected_scope.py`) loads the workspace with
`cargo metadata --format-version 1`. That is a full dependency resolve. On a
host with no Cargo registry cache, Cargo first downloads the crates.io index
entries and every dependency's sources. The planner needs none of that: it
reasons only about **edges between workspace members**.

This fix switches the planner to `cargo metadata --no-deps --offline`. It
derives the member-to-member graph from the members' own manifests, including
which optional member dependencies are active under the workspace's default
features. On the real workspace, the derived graph is identical to Cargo's
resolve: the same 205 member edges, with the same dependency kinds. The plan
does not change, and the planner never touches the network or the registry.

## Problem

### What the CI runs show

Every planner-spawning test in an archive consumer pays for the dependency
download. An archive consumer installs a toolchain for a
`requires-toolchain` suite but restores no Cargo cache. The three
`windows-latest` runs of `test-toolkit` L1 isolate this cost from the key-helper
compile, which `2026-10-06-archive-key-helper` removed. Each row is compared
only with the other rows of its own environment:

| `windows-latest` run | Registry pre-fetched by a workflow step | Key helper prebuilt | `the_shipped_planner_*` (2 tests) | `repo-deps` `every_build_record_the_shipped_planner_…` |
|---|---|---|---|---|
| 37529082394 | no | no | killed at 90 s | no result |
| 37537695506 | yes | no | killed at 90 s | 3.6 s |
| 37556914419 (PR #117) | no | yes | killed at 90 s | killed at 90 s |

What the table shows:
- A planner call on `windows-latest` is fast once the registry is present
  (run 37537695506, `repo-deps` column).
- Without the registry, a planner test hits the 90 s kill even with the helper
  compile removed (PR #117).
- The workflow fetch step in run 37537695506 took 1 min 44 s on its own
  (commit `e6d2fbbc6`, since reverted).

The same three runs on `ubuntu-latest`:

| `ubuntu-latest` run | `the_shipped_planner_emits_plans…` | `the_shipped_planner_omits_deletions…` |
|---|---:|---:|
| 37529082394 | 74.2 s | 73.3 s |
| 37537695506 | 64.8 s | 63.7 s |
| 37556914419 (PR #117) | 21.8 s | 16.0 s |

### Why a fetch step is the wrong fix

Pre-fetching (the reverted `e6d2fbbc6`) moves the download from the tests into
the job. Each Windows `requires-toolchain` cell would still pay for it, and the
job pays it whether or not a planner test runs. The download exists only
because the planner asks Cargo for a resolve it then filters down to member
edges.

### What the planner actually reads

`load_metadata` (`affected_scope.py:623`) runs the one full-resolve
`cargo metadata` on the test path. Every other `cargo metadata` call in the
repository's CI tooling already uses `--no-deps`. The planner reads the result
through:

- `workspace_packages`, `package_directories`, `validate_no_shadow_workspaces`,
  `package_ci_policy`, and `test_inputs.targets_from_metadata`. These read
  only `workspace_members` and the members' `packages` records, which
  `--no-deps` returns unchanged.
- `reverse_dependency_map` and `build_closure` read `resolve.nodes`, but only
  edges whose endpoints are both members. Both filter on `packages`, the
  member set. These two functions are the only reason the planner resolves.

Lockfile changes do not go through metadata. `lockfile_impacted_names` parses
`Cargo.lock` itself.

### Why plain declared edges are not enough

Reading member edges straight from each member's `dependencies` list does not
reproduce the resolve. Measured on the real workspace on 2026-10-06:

- **7 extra edges.** All are optional dependencies that no member's features
  activate. Examples: `test-toolkit` as an optional dependency of
  `biscuit-icon-cli`, `tree-hugger-cli`, and `worktree-cli`, and
  `biscuit-browser-harness` from `biscuit-terminal` and `darkmatter`.
- **11 edges with a different kind.** For example, `biscuit-terminal` →
  `test-toolkit` is `dev` in the resolve, but declared as both `dev` and an
  inactive optional normal dependency.
- **Wider plans.** Using those declared edges would change 4 members'
  direct-dependent sets and 27 of 74 build closures. That widens `check`
  scope, native-requirement unions, and `input_paths`.

The reverse case also matters. `biscuit-speaks` → `playa` is optional and
**is** active, and `RealWorkspaceNativeGuardTests` pins it, because dropping
it would lose `playa`'s ALSA/PulseAudio requirements. So optional edges can
be neither all kept nor all dropped. The planner has to decide which are
active.

## Design

### 1. Load metadata without a resolve

`load_metadata` runs:

```text
cargo metadata --no-deps --offline --format-version 1
```

- `--offline` turns any accidental registry or network access into an
  immediate error instead of a silent download.
- With an empty `CARGO_HOME`, this command succeeds, while the full resolve
  fails with `no matching package named 'clap' found` (checked 2026-10-06).
- On a warm developer host the call takes 0.04 s instead of 0.48 s. That
  ratio is from one host and implies nothing about CI durations.

The result carries no `resolve` key. Nothing in the planner may read one.

### 2. Derive the active member graph

Add one function, `member_dependency_graph(metadata)`. For each member it
returns the active edges to other members, each with its set of dependency
kinds (`None` for normal, `"dev"`, `"build"`). It mirrors what Cargo's
workspace resolve contains for member-to-member edges when no feature flags
are passed: every member is selected with its default features, and feature
requests are unified across the workspace.

The rules, evaluated to a fixpoint:

1. **Starting point.** Every member requests its own `default` feature, if it
   declares one.
2. **Matching an edge to a member.** A dependency entry points at a member
   when its `path` is that member's manifest directory. A path dependency
   inherited with `workspace = true` reports its `path` the same way. Any
   other entry is ignored, as are path dependencies outside the member set
   (for example `schematic/schema`).
3. **When an entry is active.** A non-optional entry is always active. An
   optional entry is active once its dependency key (the `rename`, else the
   `name`) is activated.
4. **Expanding an enabled feature `f` of member `P`.** Each item listed in
   `P.features[f]`:
   - `dep:k` activates `P`'s dependency `k`;
   - `k/g` activates `k` if it is optional, and requests feature `g` on every
     active member entry with key `k`;
   - `k?/g` requests `g` on `k` only once `k` is active (a weak feature); it
     never activates `k`;
   - a plain name requests that feature of `P` if `P` declares it. Otherwise
     it activates the optional dependency of that name (an implicit feature).

   Requesting a name that `P` declares no feature for, but which names an
   optional dependency, also activates that dependency (implicit feature).
5. **Following an active member edge.** It requests the entry's `features` on
   the target, plus the target's `default` unless `uses_default_features` is
   false. This applies to normal, build, and dev edges alike, as the
   workspace-unified resolve does.
6. **Kinds.** An edge's kinds are the union of `kind` over all of `P`'s
   *active* entries for that target. An inactive optional entry contributes no
   kind. The entry's `target` (`cfg(…)`) is ignored, as `build_closure`
   already ignores it.

A prototype of these rules reproduced the real resolve exactly on
2026-10-06: 205 member edges, none missing, none extra, and identical kinds.

### 3. Point the two graph readers at it

`reverse_dependency_map` and `build_closure` take the member graph instead of
reading `metadata["resolve"]["nodes"]`. Compute the graph once per planner run
and pass it to every call site (`calculate_scope` and the native-closure and
`input_paths` helpers). Their documented semantics do not change: direct
dependents only, dev-dependencies of dependencies not propagated, and target
restrictions not evaluated.

Rewrite `build_closure`'s docstring: the closure now comes from the planner's
member graph, which reproduces the workspace-unified resolve's optional
edges. Keep the `biscuit-speaks` → `playa` explanation, since it still holds.

### 4. Tests

In the `repo-deps` Python suite (`scripts/ci/test_affected_scope.py`):

- **Parity against Cargo, on the real workspace.** Compute
  `member_dependency_graph` from `--no-deps` metadata. Compare it, edge for
  edge and kind for kind, with the member edges of a full `cargo metadata`
  resolve. This test is the one place a full resolve remains. It answers
  "do the rules still match Cargo for this workspace", so it fails loudly
  rather than skipping when Cargo is unavailable. It belongs in the
  `scripts/ci` unittest suite, which already resolves real metadata at class
  setup today, so it adds no new kind of cost.
- **Rule fixtures.** Use small hand-written `--no-deps`-shaped documents, one
  case per rule in §2:
  - default features;
  - `dep:` syntax, `k/g`, and the weak `k?/g`, both with the dependency
    active and inactive;
  - an implicit optional-dependency feature;
  - a renamed dependency;
  - a member requesting features on another member;
  - `default-features = false`;
  - a dependency declared both as dev and as inactive-optional normal (the
    `biscuit-terminal` → `test-toolkit` shape);
  - a path dependency outside the member set.
- **Fixture migration.** Convert the existing fixtures that express edges
  through a synthetic `resolve` (about 30 sites) to declare `dependencies` on
  the package records, as real `--no-deps` input does. A fixture that still
  carries `resolve` must fail. Assert in a shared fixture helper that the
  planner never reads that key, so stale fixtures surface instead of passing
  vacuously.
- **Keep the native guard.** `RealWorkspaceNativeGuardTests` keeps pinning
  `biscuit-speaks` → `playa`, now through the member graph.
- **No-network guard.** Assert that `load_metadata`'s command carries
  `--no-deps` and `--offline`, through the existing command-capture pattern if
  the suite has one, or a direct check of the argument list otherwise.

### 5. Documentation

- `.github/ci/README.md`: the scope job still materializes the toolchain on a
  miss. State that the planner reads `cargo metadata --no-deps --offline`
  and derives member edges itself, so planning never needs the registry.
- `docs/topics/ci-cd.md`: the same statement, wherever it describes how scope
  reads `cargo metadata`.
- `.claude/skills/rust-devops/ci-cd.md`: one line saying the planner's
  metadata read is registry-free. A new planner input that requires a resolve
  needs its own decision.

## Out of scope

- **The fetch step.** Do not re-add `e6d2fbbc6`. This fix removes the
  download instead of moving it.
- **Other `cargo metadata` callers.** `consolidation.py`, the workflow
  contracts, and the `just` recipes already use `--no-deps`, or are not on a
  test path. `scripts/dependency-report/collect.py` deliberately compares
  resolves and is not CI planning.
- **Feature flags on the planner.** The planner never passes `--features` or
  `--all-features`, and this fix does not add them. If it ever does, §2 must
  be extended and the parity test rerun with the same flags.
- **The `repo-deps` Windows MAX_PATH failures** (`LNK1104` in nested fixture
  builds) and the slow `test-toolkit` `junit_staging_contracts` fixtures. Both
  are red on `windows-latest` independently of the planner.
- **`2026-10-06-archive-guard-full-tree`.** That spec stands on its own design
  arguments. It deletes two planner tests, but other planner callers remain.

## Acceptance criteria

1. The parity test passes on macOS and on `ubuntu-latest`: the derived member
   graph equals the full resolve's member edges and kinds.
2. The plan is unchanged. For `--all`, `-- README.md`, `-- claudine/lib/src/lib.rs`,
   `-- Cargo.lock` (against a base whose lockfile differs), and
   `-- biscuit-speaks/lib/src/lib.rs`, the resolved plan from the new planner
   is byte-identical to the old planner's on the same tree. Run both locally
   and diff the output. This is an implementation check, not a CI cell.
3. `load_metadata` succeeds with an empty `CARGO_HOME` and `--offline`, and
   no planner code path reads `resolve`. Show this once by running the
   planner locally with `CARGO_HOME` set to an empty directory, process-scoped
   and never persisted to the host.
4. All existing `scripts/ci` unittest suites pass, with the fixtures migrated
   as described in §4.
5. On `windows-latest`, the `test-toolkit` planner tests
   (`the_shipped_planner_*`, if they still exist) and `repo-deps`'
   `every_build_record_the_shipped_planner_…` and
   `the_real_planners_plan_rolls_up` finish below the 30 s slow mark. Judge
   them against the `windows-latest` runs in the first table above, never
   against build-host or `ubuntu-latest` figures. If a test is still slow,
   find its cause before claiming this fix resolved it.

## Verification plan

- Local: `python3 -m unittest discover` in `scripts/ci`; `just test repo-deps`
  and `just test test-toolkit` from the root; the byte-identical plan diff
  (criterion 2); and the empty-`CARGO_HOME` planner run (criterion 3).
- `windows-latest` evidence comes from PR #117's next run if this lands on
  that branch, which already carries `ci:all-os`. Otherwise it comes from the
  next push to `main`. No extra full-scope run.
- Review `just ci-local --plan` before pushing.

## Risks

- **Cargo semantics drift.** A future Cargo could change how
  default-feature unification or weak features activate optional
  dependencies. The parity test compares against the real resolve on every
  run of the `scripts/ci` suite, so drift fails there, not silently in a
  plan.
- **New manifest shapes.** A member that uses a feature form these rules
  don't handle would make the parity test fail on its first change. Extend
  §2 and add a fixture then; do not loosen the parity test.
- **Fixture churn.** About 30 test fixtures change shape. The change is
  mechanical, and the "no `resolve` read" assertion keeps a missed fixture
  from passing vacuously.
