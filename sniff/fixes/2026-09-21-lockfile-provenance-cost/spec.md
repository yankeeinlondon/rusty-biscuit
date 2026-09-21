---
area: sniff
status: draft
created: 2026-09-21
owner: Ken Snyder <ken@ken.net>
origin: darkmatter slow-test investigation on feat/dark-fixes, 2026-09-21
related:
    - 2026-09-20-repo-perf
packages:
    - sniff
---

# Stop paying for lockfile provenance on every structure detection

## Outcome

A caller that asks Sniff for repository **structure** and never reads lockfile
provenance does not pay to compute it, and a caller that does read it pays a
cost proportional to the question asked rather than to the size of the
lockfile. No change to what `RepoInfo` reports for a caller that requests
provenance, on macOS, Linux, native Windows, and WSL2.

This spec is the complement of `2026-09-20-repo-perf`, not a replacement. That
fix parallelizes the nested-marker walk and explicitly leaves the rest of
`detect_repo_structure` unattributed: "The draft attributed about 60 ms of a
232 ms debug `detect_repo_structure` call to other work; remeasure that
attribution." This spec is that remeasurement and what it found.

## Problem and evidence

All figures below come from one host and one checkout: `aarch64-apple-darwin`,
16 logical cores, this monorepo at `feat/dark-fixes` on 2026-09-21, debug
build, warmed filesystem cache, host under load from unrelated processes
(1-minute load average 8–12 during sampling). They are observations to
reproduce, not fixture expectations.

### Where a detection's time goes

`sample` at 1 ms resolution over a Darkmatter unit test that performs six
ambient root composes, each of which runs one `detect_repo_structure`. Of 1,651
samples inside `detect_repo_inner_with_shared_request_and_ownership`:

| Callee | Samples | Share |
|---|---|---|
| `discover_nested_workspace_outcomes` (the nested-marker walk) | 1,003 | 61% |
| `upgrade_provenance_with_lockfile` | 426 | 26% |
| everything else (manifest reads, seeds, `create_package_from_seed`, …) | 222 | 13% |

The 61% is `2026-09-20-repo-perf`'s subject. The 26% is this spec's. Its share
is consistent with the earlier investigation's "about 60 ms of 232 ms", which is
independent corroboration, not proof: neither measurement controlled cache
state or sampled more than one run.

Inside `upgrade_provenance_with_lockfile`'s 426 samples:

| Callee | Samples | What it does |
|---|---|---|
| `cargo_lockfile_matches` → `ManifestStore::cargo_lock` → `toml::from_str::<Value>` | 150 | Parses the whole `Cargo.lock` into a generic TOML value tree |
| `cargo_lockfile_matches` → `ManifestStore::cargo` | 80 | Parses member `Cargo.toml` files to read `package.name` |
| `pnpm_lockfile_matches` | 186 | Parses the whole `pnpm-lock.yaml` with `serde_yaml_ng` into a generic value |

This checkout's `Cargo.lock` is 398,485 bytes, 17,250 lines, and 1,497
`[[package]]` entries. `CargoLockVersions::parse` deserializes all of it into
`toml::Value`, then builds a `HashMap<String, Vec<String>>` of every package's
versions. `cargo_lockfile_matches` then asks one question of it — does each
workspace member's name resolve — for a member count two orders of magnitude
smaller than the lockfile.

### Who pays, and for what

`upgrade_provenance_with_lockfile` runs unconditionally inside detection. The
cheapest request tier, `RepoRequest::structure()`, still computes it.

It is the outlier. The other two sites that load `Cargo.lock` —
`synthesize_root_package_repo_with_store` and the per-seed enrichment that calls
`lock_versions_for_seed` — are already gated on `request.wants_dependencies()`,
which is false for a structure-only request. So in the structure tier the
provenance check is the **only** reason the lockfile is read and parsed:
declining it removes the parse rather than moving it to the next caller of the
`ManifestStore` cache. Re-verify this before implementation; a new ungated load
site would silently undo the saving, which is what acceptance criterion 2's
zero-read assertion exists to catch.

Darkmatter is the heaviest known caller. Every `DarkmatterOwned` compose request
fixes one ambient repository observation (`CurrentAuthority::
establish_ambient_repository` → `AmbientRepository::discover` →
`sniff_repo::detect_repo_structure`), by design and pinned by tests (see
`2026-09-20-repo-perf`, "Why this matters beyond Sniff"). Darkmatter reads the
observation's root, its scope catalog, and the projected `Repo`-group `ctx`
keys. A search of `darkmatter/lib`, `darkmatter/cli`, and `darkmatter/dmls` for
`lockfile_match`, `PackageProvenance`, and `.provenance` on a Sniff type finds
no reader. Darkmatter pays for lockfile corroboration on every compose and
never looks at the answer.

Measured consequences on this checkout, debug build:

- One ambient compose costs roughly 0.25–0.36 s of CPU, almost all of it
  detection. A Darkmatter test that composes N times costs about N × that.
- Four Darkmatter tests and one Claudine test crossed the 5 s slow-test budget
  for this reason and were restructured on `feat/dark-fixes` (matrices split
  per case; a full `ComposeContext::capture()` hoisted out of a loop, 11.0 s →
  1.9 s). Those were symptomatic fixes. The per-detection cost is unchanged.
- A release `sniff repo packages` on the same checkout takes about 0.14 s wall,
  so the same work is a visible share of every `md compose` a user runs.

> **Not established:** that removing this work yields a 26% detection speedup.
> Sampling shares under host load are not additive wall-time guarantees, and
> once `2026-09-20-repo-perf` lands the walk shrinks, which makes this share of
> the remainder *larger*, not smaller. Measure after that fix, not before.

## Scope and design decisions

Two independent changes. Either is worth shipping alone; decide their order by
measurement after `2026-09-20-repo-perf` lands.

### 1. Make lockfile provenance something a request asks for

Lockfile corroboration becomes demand-driven: a request that does not ask for
it leaves `MonorepoLayer::provenance` at its manifest-derived value and
`lockfile_match` at `None`, and reads no lockfile.

Open design question, to be ruled before implementation — **what is the
default?**

- **Option A — opt-in.** `RepoRequest::structure()` stops computing it; a new
  request knob turns it on; `RepoRequest::full()` keeps it. Cheapest for every
  existing structure caller, but it silently changes what `structure()`
  returns: `provenance` would read `Manifest` where it reads `Lockfile` today.
  Every consumer of `provenance`/`lockfile_match` must be audited first
  (`sniff repo` CLI output, serialized `RepoInfo`, Claudine, the CI planner's
  inputs if any).
- **Option B — opt-out.** Behavior is unchanged by default; Darkmatter's capture
  passes a request that declines it. Zero risk to other callers, but every
  future structure caller keeps paying until it learns to decline, which is how
  Darkmatter ended up here.

Recommendation: **A, gated on the consumer audit.** "Structure" is the tier a
caller picks when it wants topology cheaply, and corroborating a lockfile is
not topology. If the audit finds a structure-tier consumer that depends on
`Lockfile` provenance, fall back to B for that release and record why.

Whichever is chosen, Sniff's existing request-tier vocabulary and work counters
apply (load the `sniff` skill before designing the knob): a declined lockfile
read must show up as **zero** lockfile reads under a fresh collector, not as a
cheaper read.

`None` for `lockfile_match` already means "no lockfile, or unparseable". Under
this change it would also mean "not asked". If any consumer distinguishes
those, that is a second open question; do not overload `None` silently — decide
it in review.

### 2. Make the Cargo lockfile check proportional to the question

Independently of who asks, `CargoLockVersions::parse` should not build a generic
value tree of 1,497 packages to answer name membership.

Direction, not a mandated implementation: deserialize into a typed struct that
keeps only `package[].name` and `package[].version`, letting `serde` skip
`source`, `checksum`, and `dependencies` instead of allocating them. This keeps
a real TOML parser — **do not** hand-roll a line scanner over `Cargo.lock`; its
format is stable in practice but not a contract, and a scanner trades a
measurable cost for a silent-wrongness risk.

`CargoLockVersions::resolve` has other callers (version resolution for
workspace-inherited versions). Preserve its full contract: every package name,
every version, same ordering of versions for a name.

The `pnpm-lock.yaml` path has the same shape (whole-document generic YAML parse
to read `importers:` keys) and the same remedy. Include it only if measurement
on a pnpm-authoritative fixture shows it matters; this checkout's 186 samples
suggest it does, but this is a Cargo-authoritative repo with an incidental pnpm
layer and is not representative.

### Out of scope

- The nested-marker walk — `2026-09-20-repo-perf`.
- Darkmatter's one-observation-per-request contract. It is deliberate, pinned
  by tests, and not a Sniff concern. This spec makes the observation cheaper; it
  does not make it lazier.
- Any cross-request or on-disk cache of detection results. Sniff's observation
  reuse is request-scoped by design.
- The `ManifestStore::cargo` member-manifest parses (80 samples). They are
  cached per store and shared with package identity; they are listed above for
  completeness, not as a target.

## Acceptance criteria

1. A consumer audit is recorded in the implementation log: every reader of
   `MonorepoLayer::provenance`, `PackageSeed`/`Package` provenance, and
   `lockfile_match` across the workspace, with the request tier each uses. The
   Option A/B ruling cites it.
2. Under a fresh work collector, a structure detection that declines lockfile
   provenance reports zero lockfile reads and zero lockfile parses; one that
   requests it reports exactly the reads it does today. An absent counter means
   zero.
3. For a request that asks for provenance, complete `RepoInfo` output is
   unchanged on a controlled fixture covering: Cargo, pnpm, and uv authorities;
   a matching lockfile; a stale lockfile (extra and missing members); an absent
   lockfile; an unparseable lockfile. Assert independently expected values, not
   only before/after equality.
4. `CargoLockVersions` parity: for this checkout's `Cargo.lock` and for a fixture
   with duplicate package names at multiple versions, the typed parse resolves
   the same names to the same ordered versions as the current implementation.
   Keep the current implementation as a test-only reference for this comparison.
5. Darkmatter's capture uses the cheaper request, and `just test` in
   `darkmatter/` passes, including the three observation-boundary tests named
   in `2026-09-20-repo-perf`. The `REPOSITORY_DISCOVERY_COUNT` expectations are
   unchanged: this spec changes the cost of a discovery, never the count.
6. `just test` and `just lint` pass in `sniff/` with the recipe's `remote`
   feature coverage. No new CI matrix cell or timing gate is added; exercise the
   fixtures through existing test workflows on all four environments and reuse
   qualifying evidence.

## Performance verification

Follow `2026-09-20-repo-perf`'s protocol so the two fixes' numbers are
comparable: same checkout, root spelling, lockfile, feature set, profile, host,
and harness; at least three warmups and 20 measured samples per case; median
and range; baseline and changed runs alternated; warmed-cache results labeled as
such. Record 1-minute load average with every sample set — this investigation's
host swung between load 8 and load 62 within an hour, and a number taken without
it is not interpretable.

Measure, separately, in debug and release:

- `upgrade_provenance_with_lockfile` in isolation, before and after change 2.
- Public `detect_repo_structure(<repo root>)`, with provenance requested and
  declined.
- One ambient Darkmatter compose of a trivial document (the unit the test suite
  actually pays), before and after.

Report the detection-level and compose-level effect as measured. Do not report
the lockfile parse's speedup as detection's, or detection's as the suite's.
