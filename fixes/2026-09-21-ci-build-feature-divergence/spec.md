---
area: repo
status: draft-spec
created: 2026-09-21
clarified: false
reviewed: true
reviewed_by: opencode/zai-coding-plan/glm-5.3
reviewed_on: 2026-09-21
review_iterations: 0
implemented: false
owner: Ken Snyder <ken@ken.net>
origin: review of pull request 92's `build (ubuntu-latest)` producer job, 2026-09-21
related:
    - 2026-09-12-single-os-compile
packages:
    - repo-deps
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

# Stop divergent base features from recompiling the workspace per archive

> **Reader's note (inline review, 2026-09-21).** The evidence below was
> independently re-derived during review: the thirteen-row divergence table
> reproduces exactly at this tree, and each base flag's provenance was traced
> through `Cargo.lock` (both recorded inline). The review also filled two
> design gaps — the isolation remedy's hidden coupling to the affected-scope
> planner, and the alignment crate's wiring and fan-out side effects —
> corrected one overclaim about what attribution can derive without building,
> and left one genuine design decision, how an isolated guard test stays
> scheduled, under Open Questions.

## Outcome

The environment owner's archive build compiles each workspace crate once per
configuration that some package's tests **need**, not once per configuration
that an unrelated third-party feature flag happened to produce. The producer
job's critical path drops accordingly, measured on the same selection.

`2026-09-12-single-os-compile` stays in force. That fix ruled that packages
remain separate Cargo invocations in one target tree so that Cargo "may reuse an
exact dependency configuration across packages while retaining separate
artifacts for incompatible feature or flag sets", and that doing so "avoids
silently enabling a union of features". This spec does not reopen that ruling.
It reports what the ruling's own gate — "the rollout measurements" — shows now
that the design is running, and proposes narrowing the divergence the design
faithfully preserves.

## Problem and evidence

All figures come from one completed job: `build (ubuntu-latest)`, run
`35657985254`, job `106528099555`, on pull request 92 at `d639c66fb`,
2026-09-21. Selection: ten owner packages (`biscuit-file`, `claudine`,
`claudine-cli`, `claudine-gen`, `darkmatter`, `darkmatter-cli`, `dmls`,
`repo-deps`, `sniff`, `sniff-cli`). One run, one selection, one runner class:
observations to reproduce, not a baseline. The same job on the previous head
took 51.8 minutes and was not analyzed.

### The dependency cache is working

| Measure | Value |
|---|---|
| Job wall time | 37.6 min |
| Sum of Cargo `Finished` times across the ten archives | 27.9 min |
| Third-party crates compiled, whole job | **0** |
| Workspace crate compiles, whole job | **89** |
| Distinct workspace crates compiled | 28 |

`Swatinem/rust-cache` restored every third-party artifact, including every
feature variant of them. It never stores workspace members, by design, so those
are rebuilt in every job. Both behaviors are correct. The cost is in how many
times each workspace crate is rebuilt *within* the job.

### One crate, many configurations

| Owner package | Cargo build time | Workspace crates compiled |
|---|---|---|
| `biscuit-file` | 15 s | 7 |
| `claudine` | 178 s | 17 |
| `claudine-cli` | 543 s | 23 |
| `claudine-gen` | 51 s | 3 |
| `darkmatter` | 381 s | 9 |
| `darkmatter-cli` | 150 s | 1 |
| `dmls` | 47 s | 2 |
| `repo-deps` | 80 s | 8 |
| `sniff` | 112 s | 8 |
| `sniff-cli` | 116 s | 11 |

Compiles per workspace crate across the job: `biscuit-file` 8, `biscuit-terminal`
7, `schematic-define` / `schematic-definitions` / `schematic-schema` / `sniff` /
`darkmatter` 6 each, `renderable` / `biscuit-test-harness` 5, `biscuit-hash` /
`test-toolkit` 4.

Cargo prints one `Compiling` line per package per invocation, so these count
invocations that rebuilt the crate, not rustc units. Order is irrelevant: within
one target tree Cargo retains every variant, so a crate is compiled exactly once
per distinct configuration. Eight compiles of `biscuit-file` means eight
distinct configurations of it were demanded.

### Where the configurations come from

Resolved features were compared locally (`cargo tree -e features,normal,build,dev`,
no compilation) for two adjacent owners, `claudine` and `claudine-cli`, at the
analyzed job's tree (`feat/dark-fixes`), not at `main`; re-derive the table at
the implementing branch's base before relying on any single row. Their
closures share 631 crates. **13 resolve to different feature sets**, and
`claudine-cli` only ever adds:

| Crate | Features `claudine-cli` adds |
|---|---|
| `libc` | `extra_traits` |
| `proc-macro2` | `span-locations` |
| `serde_core` | `default` |
| `mio` | `default`, `log` |
| `time` | `local-offset` |
| `clap`, `clap_builder` | `env`, `unstable-ext`, `wrap_help` |
| `crossterm` | `bracketed-paste`, `default`, `derive-more`, `events`, `windows` |
| `hyper` | `server` |
| `hyper-util` | `server`, `server-auto`, `service` |
| `tokio-stream` | `net` |
| `tower` | twelve, including `balance`, `buffer`, `limit`, `tracing` |
| `darkmatter` | `effects-instrumentation` |

Review reproduced the comparison on macOS at the same tree: all thirteen rows
confirmed and no others. Two methodology traps for the re-derivation: key rows
on `(crate, version)` — merging `float-cmp 0.9.0` (via `strict-num`) with
`float-cmp 0.10.0` (via `expectrl`) fabricates a fourteenth row — and ignore
the root package's own feature display, which fabricates a `claudine +default`
row on the `-p claudine` side.

Provenance of the three base flags, traced through `Cargo.lock` during review
(re-verify at the implementing branch's base):

- `proc-macro2/span-locations` — declared by `claudine/cli`'s
  `[dev-dependencies]` for the source-scan guard test, as the spec's part 2
  table records.
- `libc/extra_traits` — enabled by `nix 0.26.4`, which enters through
  `expectrl 0.8.0` → `ptyprocess 0.5.0`. `expectrl` is a dev-dependency of
  five workspace crates — `claudine/cli`, `sniff/cli`, `biscuit-terminal/lib`,
  `biscuit-terminal/cli`, and `biscuit-tui/cli` — so within this selection
  **two** owners (`claudine-cli` and `sniff-cli`) demand the variant and the
  other eight pay for it. The newest `expectrl` (0.9.0) still rides
  `ptyprocess 0.5.0`, so a version bump alone does not retire `nix 0.26`. The
  lock also carries `nix 0.29.0` (via `mac_address`) and `nix 0.31.3` (via
  `ctrlc`, reached by `repo-deps`'s optional `local-tools` feature); part 1
  must confirm which `nix` versions enable `extra_traits` before counting
  exactly which owners demand the variant.
- `serde_core/default` — enabled by `axum 0.8.9`, present in `claudine-cli`'s
  closure only.

Three of these — `libc`, `proc-macro2`, `serde_core` — sit under nearly
everything. A crate's artifact identity includes its dependencies' identities,
so one added flag on `libc` gives a new identity to every crate above it,
workspace crates included. The third-party variants come back from the cache.
The workspace variants do not exist anywhere and are compiled. That is the 543 s
`claudine-cli` archive rebuilding `biscuit-file`, `biscuit-hash`, `renderable`,
`sniff`, `darkmatter`, and `claudine`, all of which the `claudine` archive had
just built with **identical features of their own**.

A second, smaller cause is divergence in the workspace crates' own features,
which the ruling exists to preserve: `biscuit-file` gains `fetch` and
`biscuit-hash` goes from no features to `blake3`, `default`, `xx_hash` between
the `biscuit-file` owner and the `claudine` owner, and `darkmatter` gains
`effects-instrumentation` under `claudine-cli`.

> **Not established:** how the 89 compiles split between those two causes, or
> how much wall time each accounts for. Only one owner pair was compared, and
> compile counts are not seconds. A `libc` flag that forces a rebuild of
> `darkmatter` is expensive; one that forces a rebuild of `biscuit-hash` is not.
> Attribution is this spec's first task.

### Why the ruling did not anticipate this

The fixture that pins the ruling,
`one_owner_tree_shares_a_dependency_compile_without_unifying_features` in
`scripts/ci-build-archive-tests.rs`, models two packages and one shared
dependency with a deliberately different feature. It proves the mechanism. It
cannot show what happens when the divergent crate is `libc` and 28 workspace
crates stand on it, because the cost is a property of this workspace's graph,
not of the mechanism.

## Scope and design decisions

### 1. Attribute every workspace recompile to its cause

Before changing anything, produce, for one representative selection, a table of
every `(workspace crate, configuration)` the owner builds, and for each
configuration beyond the first, the reason it differs: the crate's own features,
a workspace dependency's features, or a third-party dependency's features (and
which crate and flag).

Use what Cargo already exposes and what runs locally: `cargo tree -e features`
per owner package, and Cargo's unit graph or fingerprint logging for the units
themselves. **Do not trigger CI runs to learn this.** The configuration
attribution — which `(workspace crate, configuration)` pairs exist and which
flag caused each — is derivable from `Cargo.lock`, the manifests, and `cargo
tree` alone. The seconds are not: completed CI logs carry no per-crate timings.
Take them from one local timed pass — build each owner package's archive once
locally (for example `cargo build --timings` for the same package and feature
graph, or timing the archive command per owner) and charge each configuration
the seconds of the crates it re-identifies. The ranked output — configurations
by the compile seconds they cause — decides how far part 2 needs to go.

Deliverable: the table, the script that produced it, and a one-page reading of
it in this fix's implementation log. If a reusable report is worth keeping, it
belongs beside `ci-build`, emitted from the plan the owner already reads; decide
that from how useful the first table was.

### 2. Remove base-of-graph divergence, by the mechanism each case allows

The workspace has no `[workspace.dependencies]` table, and most of the divergent
flags are not requested by any workspace manifest, so "declare it once and
inherit" is not available. Tracing the three base crates shows three different
shapes, each with its own remedy:

| Flag | Who enables it | Shape |
|---|---|---|
| `proc-macro2/span-locations` | `claudine/cli/Cargo.toml`, a **dev-dependency**, for one syntax-tree test guard (`tests/error_guards/source_scan.rs`, which needs a span's line number) | declared by a member |
| `libc/extra_traits` | `nix 0.26`, in the closures of the two owners holding `expectrl` as a dev-dependency (`claudine-cli`, `sniff-cli`), absent from the other eight | transitive, from a third-party crate only some owners have |
| `serde_core/default` | `axum`, present in `claudine-cli`'s closure only | transitive, same |

`proc-macro2` is under every derive macro, so that one dev-dependency line gives
a new identity to every crate that derives `serde`, `clap`, or `thiserror`
traits whenever `claudine-cli` is the owner. It is a legitimate need and the
likeliest single largest cause; part 1 confirms or refutes that.

Remedies, cheapest first. Choose per flag from part 1's ranking:

- **Isolate the requester.** Move the test that needs the flag into a small
  test-only crate, so one small archive pays for the divergent configuration
  instead of `claudine-cli`'s whole closure. Fits the member-declared shape, and
  unifies nothing. Two requirements the move must meet, neither obvious:
    - The new crate's closure stays third-party-only. The guard reads the
      area's production sources as files; it must not take a Cargo dependency
      on `claudine` or any workspace crate, because that edge would re-import
      the divergent configuration for the dependency's whole closure and erase
      most of the saving.
    - The affected-scope planner must still select the guard when `claudine`'s
      sources change. `scripts/ci/affected_scope.py` selects packages from
      Cargo's dependency graph, and a crate with no edge is invisible to it:
      moved naively, the guard silently stops running on exactly the changes
      it exists to check. Resolving that coupling is a planner-contract
      decision, carried as the open question below.
- **Remove the source.** Where a flag arrives through an old or incidental
  third-party crate, upgrading or dropping that crate removes the divergence.
  For `libc/extra_traits` the pull chain is now known (review, above):
  `expectrl 0.8` → `ptyprocess 0.5` → `nix 0.26.4`, held as a dev-dependency
  by five workspace crates, and the newest `expectrl` still pins the same
  `ptyprocess` — so this remedy is blocked upstream unless `expectrl` itself
  is replaced. Re-check for a newer release at implementation time before
  falling back to alignment.
- **Align the flag for every owner.** The only mechanism that reaches a
  transitive flag is a crate every member depends on that itself depends on the
  third-party crate with the flag enabled — a `workspace-hack` crate, hand-written
  and limited to reviewed entries. This is a deliberate, narrow exception to "do
  not unify", and the line it draws is the design decision to review:
    - **Eligible:** third-party crates near the base of the graph, features that
      add capability without changing behavior a test can observe
      (`libc/extra_traits` adds trait impls; `proc-macro2/span-locations` adds
      diagnostics data; `serde_core/default` is the crate's ordinary default),
      one reviewed entry at a time, each with a comment naming the recompiles it
      removes.
    - **Never eligible:** workspace crates' own features. `biscuit-file/fetch`,
      `biscuit-hash/blake3`, and `darkmatter/effects-instrumentation` are exactly
      what `2026-09-12-single-os-compile` protects: a package whose tests pass
      only because another package enabled a workspace feature has a missing
      declaration, and unifying would hide it.
    - **Ruled per crate, default no:** `time/local-offset`, `hyper/server`,
      `tower/*`, `crossterm/events`. These add real behavior, so aligning them
      lets a package's tests pass on a third-party feature its own manifest never
      declared. Leave them divergent unless part 1 shows one is expensive, and
      then rule on it here.

  If the crate is created, its wiring is fixed by what the owner actually
  builds — `cargo nextest archive` invocations, which always resolve
  dev-dependencies:

    - It is a root workspace member under `tools/` with no code of its own —
      no bins, no tests, only `[dependencies]` entries — beside the other
      workspace-wide crates.
    - Every member carries it as a **dev-dependency only**. That reaches every
      owner's archive invocation, which is the only place the divergence
      costs anything, while normal and release graphs — including any
      published manifest — stay untouched.
    - Adding or changing an entry updates `docs/dependencies.md` and the
      `rust-devops` skill in the same change (the per-entry comment naming
      the recompiles it removes is already required above).
    - Known side effect: because every member depends on it, a change to the
      crate is a changed direct dependency of every package. The established
      compile-coverage rules bound the fan-out — unchanged reverse dependents
      compile inside one Ubuntu `check` cell rather than re-running
      everywhere — and entries are add-only and rare. Record the first real
      fan-out in the implementation log so the cost stays visible.

Prefer the first two remedies wherever they apply: they shrink the divergence
without weakening the ruling at all. The hack crate is for what remains.

### Alternatives considered

- **`cargo-hakari` (a generated `workspace-hack` crate).** The standard tool for
  this problem, and the automated form of part 2's third remedy. Not recommended
  as the first step: it unifies *every* third-party feature, including the
  "ruled per crate, default no" set, which gives up more of the ruling's
  protection than the evidence yet justifies, and it needs a CI check that the
  generated crate is current. Revisit if the hand-written crate's list grows
  past what is pleasant to review.
- **Caching workspace crates across runs** (`rust-cache`'s
  `cache-workspace-crates`). Expected not to help: Cargo decides a path
  dependency's freshness from file modification times, and a fresh checkout
  resets them, so restored workspace artifacts would be judged stale. Verify
  that expectation cheaply before dismissing it for good; do not adopt it on
  hope.
- **Reordering or grouping owner packages.** No effect. Within one target tree
  the compile count is the number of distinct configurations, whatever the
  order.
- **The ruling's own fallbacks** — planner-declared cohorts, one owner per
  package, a remote compiler-artifact service. All three address critical path
  through parallelism or reuse, and none reduces the number of configurations.
  Cohorts would make this problem worse: a configuration shared across cohorts
  compiles once per cohort. They remain the fallback for a *different* finding.

### Out of scope

- `area-drift`'s separate `cargo build -p sniff-cli --release` (about four
  minutes, eleven workspace crates). It is a different profile on a different
  runner and can share nothing with the owner's `test`-profile build. Whether a
  debug `sniff` would serve that contract — it did, locally, on 2026-09-21 — is
  a small separate question: a cheaper build against roughly seventy slower
  `sniff` invocations. Measure before proposing it.
- Any new CI run, matrix cell, fixture, or gate.

## Acceptance criteria

1. The attribution table from part 1 exists for the ten-package selection above,
   with every configuration beyond a crate's first explained, and the
   implementation log states the split between third-party-inherited and
   workspace-own divergence in compile seconds, not only counts.
2. Each flag handled under part 2 is a separate, reviewable change that names
   the crate, the flag, the remedy chosen (isolate, remove the source, or align),
   and the recompiles it removes. Isolation and source removal are tried before
   alignment, and the log says why they did not apply where alignment was used.
   No workspace crate's own feature is aligned. No isolation change lands
   before the Open Questions section below is resolved and the ruling recorded
   in this spec.
3. For every aligned third-party feature, the reason it cannot change a test
   outcome is written down. A feature that cannot meet that bar is not aligned
   without an explicit ruling recorded in this spec. If the alignment crate is
   created, it is a `tools/` workspace member with no bins or tests, carried by
   every member as a dev-dependency only, and the change that introduces it
   updates `docs/dependencies.md` and the `rust-devops` skill.
4. `one_owner_tree_shares_a_dependency_compile_without_unifying_features` and the
   rest of `scripts/ci-build-archive-tests.rs` pass unchanged. The mechanism is
   not what changed.
5. Re-running the part 1 script after alignment shows fewer configurations for
   the affected workspace crates, and the next ordinary pull request that selects
   a comparable package set shows a lower sum of Cargo build time in its owner
   job. Record both. Do not gate on either, and do not trigger a run to obtain
   the second: take it from the next run that happens anyway.
6. Isolated-package fidelity still holds where it matters: for each package whose
   closure changed, `cargo test -p <package>` alone still builds, so no package
   came to depend on a feature only the workspace-level declaration provides.
   Reuse qualifying local evidence.
7. The `rust-devops` skill's CI notes record the finding — divergent base
   features multiply workspace compiles, and how to read the attribution table —
   in the same change. Any part 2 change that adds, moves, or removes a
   dependency edge updates `docs/dependencies.md` in that change too.

## Open Questions

### How does an isolated guard test stay scheduled?

The isolation remedy moves the source-scan guard out of `claudine-cli` into a
small crate with no Cargo edge to `claudine`, because any edge re-imports the
divergent configuration for `claudine`'s whole closure. But the planner
(`scripts/ci/affected_scope.py`) selects packages from Cargo's dependency
graph: with no edge, a change to `claudine`'s sources no longer selects the
guard, and the guard silently stops running on the changes it exists to check.
This touches the planner contract, so it needs a ruling before any isolation
work starts.

1. **Do not isolate; align `proc-macro2/span-locations` instead.** The flag is
   already on this spec's eligible list — it adds diagnostics data no test can
   observe — so if the alignment crate exists anyway (likely, since removing
   the `libc` source is blocked upstream), one more reviewed entry there
   removes this divergence with no new mechanism and the test stays in
   `claudine-cli`.
    - Pros: no planner change; no crate beyond the alignment crate; the guard
      keeps running exactly where it runs today.
    - Cons: spends an alignment entry on a flag one test needs, where
      isolation would remove the divergence outright; every owner's
      proc-macro graph gains the feature (third-party and cache-restored, so
      the cost is compile identity, not seconds).
2. **Declare the guard's ownership in the planner.** Teach `affected_scope.py`
   a narrow declaration — for example a `[package.metadata.ci]` watch of the
   source paths a package scans — so a source-only guard is selected when the
   paths it reads change.
    - Pros: keeps the strongest outcome — the divergent configuration touches
      only third-party crates the dependency cache already restores per
      variant — and the declaration is explicit and testable in planner
      fixtures.
    - Cons: extends the planner contract with a new selection input, a CI
      scope change that must load the `rust-devops` skill and add fixtures;
      a stale watch is a silent hole in coverage.
3. **Give the guard a real dependency edge on `claudine`.** No new mechanism;
   the planner sees the edge and scheduling is guaranteed.
    - Pros: simplest; nothing new to build or maintain.
    - Cons: re-imports the divergent configuration for `claudine`'s whole
      closure inside the guard's own archive, so most of the saving evaporates
      and the remedy degrades to moving the cost to a smaller owner.

**Recommendation: option 1.** It is the only option that removes the
divergence without either extending the planner contract or gutting the
remedy, and it is available whenever the alignment crate exists. Choose
option 2 only if part 1's ranking shows the alignment crate is not warranted
at all. Option 3 is a last resort. Record the ruling here before
implementing.
