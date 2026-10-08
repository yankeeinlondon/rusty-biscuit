---
kind: fix
name: planner-metadata-no-deps
date: 2026-10-06
status: draft-spec
related:
  - 2026-10-06-archive-key-helper
  - 2026-10-06-archive-guard-full-tree
reviewed: true
reviewed_by: codex/gpt-6.1-sol
reviewed_on: 2026-10-06
review_iterations: 0
clarified: false
implemented: true
human_review: false
implemented_by: claude/sonnet
message_to_agent: |-
  Phase 5 (last) landed docs only; there is no next phase. Remaining for the author: criterion 5 (Windows timing) is pending hosted evidence, and the docs state the Cargo-faithful weak-feature rule rather than the spec's rule 4.
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

The CI planner in the `repo-deps` package,
[`affected_scope.py`](../../scripts/ci/affected_scope.py), loads the workspace with
`cargo metadata --format-version 1`. That is a full dependency resolve. On a
host with no Cargo registry cache, Cargo first downloads the crates.io index
entries and resolved dependencies' sources. The planner needs none of that: it
ultimately consumes only **edges between workspace members**.

Here, a member is a package listed in the root Cargo workspace, and an edge
means one member depends directly on another. The planner uses those edges
to report direct dependents and to collect the member packages, system
libraries, and input directories needed for a package's tests and builds.

This fix switches the planner to `cargo metadata --no-deps --offline`. It
derives the member-to-member graph from the members' own manifests, including
which optional member dependencies are active under the workspace's default
features. The draft reports that its prototype matches Cargo's graph on the
real workspace: 205 member edges, with the same dependency kinds. The plan
must not change. The metadata read and graph derivation need no network or
registry cache. A complete planner invocation still needs the pinned Cargo
toolchain and the prebuilt build-key helper supplied by
`2026-10-06-archive-key-helper`; this fix does not remove those prerequisites.

> **Reviewer's note:** the reported 205-edge match is evidence for the current
> workspace, not proof of every Cargo manifest shape. This review corrects an
> additional feature-activation rule and separates registry-free planning from
> tests that compare against a full Cargo resolve. The placement of the real
> workspace comparison and support for feature requests through packages
> outside the workspace remain design decisions in **Open Questions**.

## Problem

### What the CI runs show

A planner-spawning test in an archive consumer can pay for dependency
downloads while the registry cache is empty. Tests on the same runner can
share the resulting cache; this is not a claim that each call downloads every
dependency independently. An archive consumer installs a toolchain for a
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

In `repo-deps`, [`load_metadata`](../../scripts/ci/affected_scope.py) loads a
full dependency resolve each time a planner caller asks for workspace
metadata. The other inspected CI metadata loaders already use `--no-deps`.
The planner reads the result through these `repo-deps` helpers:

- [`workspace_packages`, `package_directories`, `validate_no_shadow_workspaces`,
  and `package_ci_policy`](../../scripts/ci/affected_scope.py), plus
  [`targets_from_metadata`](../../scripts/ci/test_inputs.py). These read
  only `workspace_members` and the members' `packages` records, which
  `--no-deps` returns unchanged.
- [`reverse_dependency_map` and `build_closure`](../../scripts/ci/affected_scope.py)
  read `resolve.nodes`, but only
  edges whose endpoints are both members. Both filter on `packages`, the
  member set. These two functions are the only reason the planner resolves.

Lockfile-only changes retain the repository's input-based selection policy:
they do not select packages just because a dependency version changed. The
retained `repo-deps` helper
[`lockfile_impacted_names`](../../scripts/ci/affected_scope.py) parses lockfile
text independently, but the current scope calculation does not call it. This
fix must not restore lockfile-driven package selection.

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
**is** active, and `repo-deps`'s
[`RealWorkspaceNativeGuardTests`](../../scripts/ci/test_affected_scope.py)
pins it, because dropping
it would lose `playa`'s ALSA/PulseAudio requirements. So optional edges can
be neither all kept nor all dropped. The planner has to decide which are
active.

## Design

### 1. Load metadata without a resolve

In `repo-deps`, [`load_metadata`](../../scripts/ci/affected_scope.py) runs:

```text
cargo metadata --no-deps --offline --format-version 1
```

- `--offline` turns any accidental registry or network access into an
  immediate error instead of a silent download.
- With an empty `CARGO_HOME`, this command succeeds, while the full resolve
  fails with `no matching package named 'clap' found` (checked 2026-10-06).
- On a warm developer host the call takes 0.04 s instead of 0.48 s. That
  ratio is from one host and implies nothing about CI durations.

Cargo returns `"resolve": null`, not an absent key. The planner must work
with either a null or absent `resolve` and must never inspect it. This matches
the [Cargo metadata format](https://doc.rust-lang.org/cargo/commands/cargo-metadata.html#output-format)
and was confirmed during review with an empty `CARGO_HOME` on 2026-10-06.

Preserve the existing UTF-8 subprocess decoding and propagation of Cargo
errors. Do not retry with a full resolve, run `cargo fetch`, or silently fall
back to all declared edges if metadata or graph construction fails. The
metadata call must not create or update `Cargo.lock` or compile a helper.
`--offline` constrains Cargo's dependency access; the pinned toolchain must
already be installed so that rustup does not need to download it.

Removing full resolution also removes its incidental validation of registry
versions and external feature names from planning. Actual Cargo build/check
cells retain that validation. This fix adds no replacement resolve or CI cell
to a metadata-only planning path.

### 2. Derive the active member graph

Add `repo-deps`'s proposed
[`member_dependency_graph(metadata)`](../../scripts/ci/affected_scope.py).
For each member it returns the active edges to other members, each with its
set of dependency kinds (`None` for normal, `"dev"`, `"build"`). These are
dependencies of the shipped package, its tests/examples, or its build script,
respectively. Use Cargo's opaque package IDs as graph keys, with an empty
dependency map for a member with no active member dependencies. A concrete
return shape is
`dict[package_id, dict[target_package_id, set[dependency_kind]]]`.

The compatibility target is the full metadata graph produced from this
repository's virtual workspace root, with no feature flags and no platform
filter, using the toolchain pinned in `rust-toolchain.toml`. Every member
starts with its default features, and feature requests are combined across
the workspace. This is the existing planner's workspace-wide dependency
view; it does not describe which features a particular package build enables
under Cargo's version-2 resolver. Preserve that distinction: a package's CI
feature flags do not become inputs to this graph in this fix.

Evaluate the following rules repeatedly until no enabled feature or
dependency changes:

1. **Starting point.** Every member requests its own `default` feature, if it
   declares one.
2. **Matching an edge to a member.** A dependency entry points at a member
   when its `path` identifies that member's manifest directory. Compare both
   sides using the same host-native path normalization, once per distinct
   path. Account for symlinks such as macOS's `/var` spelling, and native
   Windows separators and case semantics; never lowercase Unix paths or
   infer a path from the package ID. A path dependency
   inherited with `workspace = true` reports its `path` the same way. Any
   other entry contributes no direct member edge, as do path dependencies
   outside the member set (for example `schematic/schema`). Dependencies must
   never be matched to a member by name alone: a registry package with the
   same name is a different package. The effect of outside packages on
   member feature activation is addressed in **Open Questions**.
3. **When an entry is active.** A non-optional entry is always active. An
   optional entry is active once its dependency key (the `rename`, else the
   `name`) is activated.
4. **Expanding an enabled feature `f` of member `P`.** Each item listed in
   `P.features[f]`:
   - `dep:k` activates `P`'s dependency `k`, without enabling a same-named
     feature of `P`;
   - `k/g` activates `k` if it is optional, enables `P`'s feature `k` if that
     feature exists for the optional dependency, and requests feature `g` on
     every active member entry with key `k`;
   - `k?/g` requests `g` on `k` only once `k` is active (a weak feature); it
     never activates `k`;
   - a plain name requests that feature of `P` from its metadata feature map.
     Cargo already adds implicit optional-dependency features to that map.
     Do not manufacture a missing feature from a dependency name: `dep:k`
     can intentionally suppress the implicit feature.

   Dependency activation is separate from named-feature activation. Preserve
   that distinction even when `k` is also a non-optional dev dependency; that
   declaration alone must not enable an optional normal dependency. Retain
   weak requests until dependency activation changes, so `k?/g` still takes
   effect when `k` becomes active in a later pass.
5. **Following an active member edge.** It requests the entry's `features` on
   the target, plus the target's `default` if that feature exists and
   `uses_default_features` is true. A member without a default feature is
   valid; do not report an unknown-feature error for that case. This applies
   to normal, build, and dev edges alike for this metadata
   compatibility target. Since every member is also a root with its own
   default enabled, one dependency's `default-features = false` does not
   disable the target member's root defaults.
6. **Kinds.** An edge's kinds are the union of `kind` over all of `P`'s
   *active* entries for that target. An inactive optional entry contributes no
   kind. The entry's `target` (`cfg(…)`) is ignored, as `build_closure`
   already ignores it.

A prototype of these rules reproduced the real resolve exactly on
2026-10-06: 205 member edges, none missing, none extra, and identical kinds.
That is the draft's reported baseline and must be reproduced during
implementation. Do not hard-code the count; workspace membership can change.

> **Reviewer's note:** a path-only Cargo fixture checked during review gives
> package `a` the features `default = ["b/x"]` and
> `b = ["dep:b", "dep:c"]`. Cargo enables both
> `b` and `c` as dependencies of `a`. Merely activating dependency `b` for
> `b/x` misses `c`; enabling the same-named feature of `a` is necessary.
> `dep:b` alone does not enable that feature. The
> [Cargo features reference](https://doc.rust-lang.org/cargo/reference/features.html)
> also documents implicit-feature suppression and weak dependency requests.

Use only Cargo-normalized metadata for member manifests: do not parse them
again or reconstruct workspace inheritance. Unknown feature requests on a
member, ambiguous member path matches, or unsupported dependency kinds must
produce an error naming the package and offending entry, rather than silently
dropping a dependency. The ordinary metadata path cannot validate external
feature names without downloading those dependencies; that remains Cargo's
build-time responsibility.

The algorithm must terminate for permitted dev-dependency cycles and feature
cycles. Features and activated dependency keys only accumulate; process each
new fact or repeat passes over these finite sets until they stop changing.
Do not add per-edge Cargo subprocesses, persistent graph caches, or a new
performance spike. The existing measurements already establish the benefit
of removing resolution; functional comparisons must establish correctness.

### 3. Point the two graph readers at it

In `repo-deps`, [`reverse_dependency_map` and
`build_closure`](../../scripts/ci/affected_scope.py) take the member graph
instead of reading `metadata["resolve"]["nodes"]`. Compute it once in each
[`calculate_scope`](../../scripts/ci/affected_scope.py) invocation and pass it
through all callers, including the direct-dependent calculation, native
requirements, build-input directories, unchanged dependents compiled in the
changed package's check cell, and packages selected only because their tests
read a changed file. The already-planned `--apply-to` path must continue to
read neither metadata nor a graph. Their documented semantics do not change:
direct dependents only, dev-dependencies of dependencies not propagated, and
target restrictions not evaluated.

Update the relevant `repo-deps` docstrings and test failure messages to
describe the member graph and the same workspace-wide optional dependency
coupling. Keep the `biscuit-speaks` → `playa` explanation, since it still
holds. Deduplicate direct edges, preserve the existing sorted plan outputs,
and do not change dependency-kind traversal or schedule transitive dependents.

Plan fields, build identities, event scheduling, evidence eligibility, and
the merge gate remain unchanged. This is an internal graph representation
change, so no plan-schema version or new CI job is required. Qualifying
evidence is still evaluated by the existing planner input checks; never
force old evidence to remain reusable by bypassing those checks.

### 4. Tests

In the `repo-deps` Python suite
([`test_affected_scope.py`](../../scripts/ci/test_affected_scope.py)):

- **Comparison against Cargo.** Compare the derived graph, edge for edge and
  kind for kind, with member edges of a full resolve. Small temporary
  workspaces whose dependencies are all local paths can exercise Cargo itself
  with `--offline`, an empty temporary Cargo home, and no compilation. Keep
  these in the existing suite. A real-workspace comparison is also required
  before implementation is accepted, but its ongoing placement is unresolved
  in **Open Questions**: an unconditional full resolve in this suite would
  restore registry downloads in the Ubuntu archive consumer. It is not a
  registry-free check merely because the suite resolves dependencies today.
  Full comparison commands must not rewrite the shipped workspace's lockfile;
  use `--locked` for that workspace and diagnose an inconsistent lockfile
  separately. Compare named edges and kind sets, not platform-specific opaque
  ID strings from separate hosts.
- **Rule fixtures.** Use small hand-written `--no-deps`-shaped documents, one
  case per feature and matching rule above. Retain independent expected
  dependency sets, and use the path-only Cargo comparisons for the subtle
  activation cases so the expected answer is not copied from the algorithm:
  - default features;
  - `dep:` syntax, `k/g`, and the weak `k?/g`, both with the dependency
    active and inactive;
  - an implicit optional-dependency feature;
  - a renamed dependency;
  - a member requesting features on another member;
  - a member's default remaining enabled despite an incoming
    `default-features = false` declaration;
  - `k/g` enabling a same-named feature that activates another dependency,
    contrasted with `dep:k`, which does not enable that feature;
  - suppression of an implicit feature by `dep:k`, and rejection of an
    unknown member feature request;
  - a weak request whose dependency becomes active in a later pass;
  - a dependency declared both as dev and as inactive-optional normal (the
    `biscuit-terminal` → `test-toolkit` shape);
  - two declarations or aliases reaching the same member with different
    kinds or requested features, and target-specific declarations without
    platform filtering;
  - dependency and feature cycles, including the rule that a dependency's
    dev-only edges do not extend a package's build requirements;
  - an inherited workspace dependency, a non-member package record, and a
    registry dependency with the same name as a member;
  - a path dependency outside the member set, with feature feedback handled
    according to the author's decision in **Open Questions**;
  - normalized host-native path identity using fixture-owned directories;
    exercise alternate Windows spelling in the existing Windows suite, without
    adding a CI environment or treating a Windows path as a Unix path.
- **Fixture migration.** Convert the existing fixtures that express edges
  through a synthetic `resolve` (about 30 sites) to declare `dependencies` on
  the package records, as real `--no-deps` input does. Empty dependency lists
  and feature maps must be explicit in shared package fixtures. Search all
  `scripts/ci` suites, including fixture mutations, rather than treating the
  approximate site count as complete. A shared fixture-shape assertion rejects
  a populated synthetic resolve but accepts null or absence. Separately,
  pass representative planning inputs through a mapping that rejects reads
  of `resolve`, including `.get` and indexing, to prove the planner ignores
  it. Each migrated dependency fixture retains its expected dependents,
  native requirements, or input directories; deleting the old edges and
  keeping only a successful-plan assertion is insufficient.
- **Keep the native guard.** The `repo-deps`
  [`RealWorkspaceNativeGuardTests`](../../scripts/ci/test_affected_scope.py)
  still pins `biscuit-speaks` → `playa`, now through the member graph. Extend
  that guard to assert the expected system-library requirements as well.
- **No-network guard.** Capture the `repo-deps`
  [`load_metadata`](../../scripts/ci/affected_scope.py) subprocess call and
  assert both flags, UTF-8 decoding, and the absence of a fallback when Cargo
  fails. Also run the real metadata command once against the shipped
  workspace with a temporary empty Cargo home. Assert that it returns member
  packages with a null resolve, leaves registry/git caches empty, and leaves
  `Cargo.lock` unchanged. Provision the pinned toolchain first.
- **Tool availability.** New Cargo-dependent tests use
  [`tool_guard.require_tools`](../../scripts/ci/tool_guard.py) in `repo-deps`,
  matching the suite's existing convention: a developer host may skip when
  Cargo is absent; the scheduled companion suite fails when its declared
  `BISCUIT_REQUIRE_CARGO=1` tool is missing. Correct the touched guard's
  description to name `_package-ci.yml`'s `Companion suites` step, rather
  than the removed `ci-tooling` job. A skipped comparison never counts as
  passing acceptance evidence.

### 5. Documentation

- `.github/ci/README.md`: the scope job still materializes the toolchain on a
  miss. State that the planner reads `cargo metadata --no-deps --offline`
  and derives member edges itself, so its metadata and graph calculation
  never need the registry.
- `docs/topics/ci-cd.md`: the same statement, wherever it describes how scope
  reads `cargo metadata`.
- `.claude/skills/rust-devops/ci-cd.md`: one line saying the planner's
  metadata read is registry-free. A new planner input that requires a resolve
  needs its own decision.
- Explain in those pages that the graph retains the current default-feature,
  all-platform view and that complete planner tests still require a pinned
  toolchain and prebuilt helper. Document the chosen external-package feature
  boundary and real-workspace comparison workflow when the open questions are
  resolved. These current-behavior pages must not link to this fix's spec.

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

1. On the pinned toolchain, the derived member graph equals the real
   workspace's full resolve, edge for edge and kind for kind. The comparison
   completes without a skip, according to the author's chosen placement in
   **Open Questions**. The small, entirely local Cargo comparison fixtures
   pass in the existing `ubuntu-latest` companion suite; member graph and
   path-matching behavior work on macOS, Linux, native Windows, and WSL2
   through the existing package tests. No additional CI environment is added.
2. The plan is unchanged. Compare old and new graph calculations against the
   **same input tree**, not two worktrees whose metadata paths or policy
   files differ. Hold root, metadata member records, environment table,
   policy, event, date, base/head IDs, supplied evidence, constraints, and
   prebuilt helper constant. The old graph can be obtained from one saved
   full metadata result on that tree; the new graph must use its dependency
   declarations without reading the resolve. Compare canonical serialized
   resolved plans and their legacy projections for `--all`, `-- README.md`,
   `-- claudine/lib/src/lib.rs`, `-- Cargo.lock`, and
   `-- biscuit-speaks/lib/src/lib.rs`. The lockfile case checks current
   input-based selection; a differing base lockfile must not select packages
   through the retained, unused lockfile-impact helper. Also retain coverage
   for dependency kinds, packages selected only by test inputs, and native
   requirements of unchanged dependents compiled in a changed package's
   check cell. Report any mismatch by field; do not remove fields to make
   the comparison pass. This is a local correctness check, not a CI cell.
3. `repo-deps`'s [`load_metadata`](../../scripts/ci/affected_scope.py)
   succeeds with an empty `CARGO_HOME`, and no planning path reads `resolve`.
   Show this once with both the metadata call and a complete local planner
   run. Set `CARGO_HOME` only in the child environment and remove its temporary
   directory afterward. Keep the installed pinned toolchain available through
   `RUSTUP_HOME` and supply the prebuilt helper through the existing helper
   override, so the check does not trigger a separate Cargo compile. Registry
   and git dependency caches remain empty, and `Cargo.lock` is unchanged.
4. All existing `scripts/ci` unittest suites pass, with the fixtures migrated
   as described in §4.
5. On `windows-latest`, the `test-toolkit` planner tests
   (`the_shipped_planner_*`, if they still exist) and `repo-deps`'
   `every_build_record_the_shipped_planner_…` and
   `the_real_planners_plan_rolls_up` finish below the 30 s slow mark. Judge
   them against the `windows-latest` runs in the first table above, never
   against build-host or `ubuntu-latest` figures. If a test is still slow,
   find its cause before claiming this fix resolved it. Keep the existing
   test identities and time limits; do not add timing assertions, retries,
   workloads, or a multi-host performance spike. Functional implementation
   completion and this pending hosted timing evidence must be reported
   separately if the next scheduled Windows run has not happened yet.

## Verification plan

- Local: `python3 -m unittest discover` in `scripts/ci`; `just test repo-deps`
  and `just test test-toolkit` from the root; the byte-identical plan diff
  (criterion 2); and the empty-`CARGO_HOME` planner run (criterion 3).
- Compare against the real Cargo workspace once with already-cached
  dependencies and the pinned toolchain; this proves graph behavior, not a
  latency estimate. Record the complete command, graph differences (if any),
  toolchain, and tree used. Adopt the author's chosen ongoing comparison
  workflow before claiming recurring coverage. No new performance spike is
  needed: the draft already identifies the resolution cost.
- `windows-latest` evidence comes from PR #117's next run if this lands on
  that branch, which already carries `ci:all-os`. Otherwise it comes from the
  next push to `main`. No extra full-scope run.
- Review `just ci-local --plan` before pushing.

## Risks

- **Cargo semantics drift.** A future Cargo could change how
  default-feature unification or weak features activate optional
  dependencies. The small Cargo fixtures compare against the pinned Cargo
  version in the existing suite. Continuous comparison of the real workspace
  depends on the unresolved test-placement decision; do not claim it runs
  every time until that decision is made and wired into an existing owner.
- **New manifest shapes.** A member that uses a feature form these rules
  do not handle must fail clearly or fail a Cargo comparison, never silently
  lose an edge. A manifest-only edit does not automatically schedule every
  planner suite under current CI policy. The chosen comparison workflow must
  say how manifest changes and toolchain upgrades are checked; this fix must
  not add a blanket CI trigger to compensate. Local packages outside the
  workspace and registry replacement by a workspace member need an explicit
  supported boundary, as described below.
- **Fixture churn.** About 30 test fixtures change shape. The change is
  mechanical, and separate fixture-shape and "no `resolve` read" assertions
  catch stale metadata while retained expected-output assertions catch lost
  edges.

## Open Questions

### Where should the real-workspace Cargo comparison run after implementation?

The existing Python suite is owned by `repo-deps` and runs as a companion of
its Ubuntu test cell. The
[`_package-ci.yml` workflow](../../.github/workflows/_package-ci.yml) restores
no Cargo cache in that archive consumer. Keeping a full-workspace resolve in
the ordinary suite would preserve its current download cost even after all
planner callers become registry-free. Removing that recurring comparison,
however, reduces automatic protection against manifest shapes absent from the
small fixtures. This is a correctness-versus-test-cost decision for the author.

| Suggested solution | Pros | Cons |
|---|---|---|
| Keep the real-workspace comparison in the existing Ubuntu companion suite | Automatically checks the current workspace whenever the suite runs; no new job or store | Retains registry downloads in that consumer; does not run on every manifest-only change; using `--offline` would fail on its empty cache |
| **Recommended: compare small local workspaces in the scheduled suite, and make the real-workspace comparison an explicit local implementation and manifest/toolchain-change check** | Exercises Cargo's subtle feature rules automatically without downloads; compares the shipped workspace before accepting the fix; adds no CI cell or committed graph snapshot | Relies on authors running the documented real-workspace check for later manifest or toolchain changes; does not promise continuous full-workspace coverage |
| Run the real-workspace comparison on an existing producer after Cargo has prepared its dependencies | Uses dependencies already present from building; could provide automatic real-workspace coverage | Changes suite placement and producer responsibility; must preserve `repo-deps` ownership and planned execution instead of creating a second scheduler; unavailable when no relevant build runs |

Recommend the second solution because it removes the known consumer download
without expanding CI scheduling or storing a second copy of the dependency
graph. Record its reduced recurring coverage honestly in the docs. If the
author requires continuous full-workspace comparison, decide its owner and
execution point before implementing the third solution. Do not silently skip
a scheduled full comparison when a registry cache is absent.

### How should feature requests through local packages outside the workspace be handled?

`--no-deps` includes no package records for dependencies outside the member
set. Yet those packages can request features on members. For example, member
`a` can depend on a local non-member `outside`, which depends on member `b`
with feature `extra`; `b.extra` can enable an optional dependency on member
`c`. A derivation that reads only member declarations can miss the direct
`b` → `c` edge. It must not invent a direct `a` → `b` edge: the old planner
does not bridge through non-members.

An entirely local Cargo fixture confirmed during review that the example
above produces the member edge `b` → `c` without a direct `a` → `b` edge.

The current excluded [`schematic-schema`](../../schematic/schema/Cargo.toml)
package already depends on workspace members `schematic-define` and
`schematic-definitions`, so feature feedback is possible in this repository.
The draft's reported parity only shows that no additional edge was lost in
the measured tree. Registry or git dependencies replaced by a member through
Cargo's `[patch]`, `[replace]`, or configuration overrides can also defeat
path-only matching. No such manifest replacement was found during this
review; it is not implicitly supported by the proposed rules.

| Suggested solution | Pros | Cons |
|---|---|---|
| Support only member-to-member feature propagation, with an explicit restriction and a guard against unsupported feature feedback or member replacements | Keeps the proposed implementation small and registry-free | A safe guard must inspect outside manifests or conservatively reject them; the existing excluded package prevents assuming there are no outside-to-member dependencies; a documentation-only warning leaves a silent under-selection risk |
| **Recommended: propagate features through reachable local path packages, but return only direct member-to-member edges** | Handles the repository's existing local-package pattern and future feature requests while preserving graph consumers and avoiding registry downloads | Requires an explicit local-manifest loader that honors inheritance, ignores non-member dev dependencies, and handles cycles; adds filesystem work; registry/git replacement remains unsupported unless designed separately |
| Resolve arbitrary dependency sources with Cargo | Uses Cargo's complete source and feature semantics, including replacements | Restores the registry and network dependency this fix removes; unsuitable as a silent fallback or as the implementation of registry-free planning |

Recommend the second solution because excluded local packages are already
present and may legitimately enable features on members. Parse each reachable
local manifest once and use member metadata whenever available; do not add
one Cargo subprocess per dependency. Keep non-member packages out of direct
dependency results, system-library unions, and build-input directories, which
preserves the old planner's traversal boundary. Any unsupported replacement
that could map to a member must cause a clear error rather than a missing
edge. The author must settle this support boundary and its guard before
implementation; the member-only rules above are not a complete general Cargo
resolver.
