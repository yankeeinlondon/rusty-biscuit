---
area: sniff
status: draft-spec
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
reviewed: true
reviewed_by: codex/gpt-6-sol
reviewed_on: 2026-09-26
review_iterations: 3
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
provenance does not pay to compute it, and a caller that does read it avoids
retaining unrelated lockfile fields. Parsing still takes time proportional to
the lockfile's size. No change to what `RepoInfo` reports for a caller that requests
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
smaller than the lockfile. Unlike the pnpm and uv checks, Cargo does not compare
exact member sets: dependencies add many lockfile packages, and the current
check also accepts stale extra workspace member entries.

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
measurement after `2026-09-20-repo-perf` lands. The request change intentionally
changes structure-tier provenance; full and explicit provenance requests retain
the existing result.

### 1. Make lockfile provenance something a request asks for

Lockfile corroboration becomes demand-driven: a request that does not ask for
it leaves `MonorepoLayer::provenance` at its manifest-derived value and
`lockfile_match` at `None`. A structure-only request then reads no lockfile;
requests for dependency versions can still read one for that separate purpose.

### Open question: should structure detection retain the old default?

This changes a public result, so the author should choose the compatibility
policy before implementation.

- **Option A — opt-in.** `RepoRequest::structure()` and the public
  `detect_repo_structure` convenience function stop computing it; an explicit
  request turns it on; `RepoRequest::full()` and `detect_repo` keep it. This
  saves work for existing structure callers, including Darkmatter, but changes
  their serialized `RepoInfo`: a confirmed layer retains its membership-derived
  provenance and omits `lockfile_match`. Audit consumers first, including the
  Sniff CLI, Claudine, Darkmatter, and any CI planner inputs.
- **Option B — opt-out.** Behavior is unchanged by default; Darkmatter's capture
  passes `detect_repo_with_request` with a request that declines it. Existing
  output stays stable, but the default continues to charge other structure
  callers; direct convenience calls need a separate way to decline.

Recommendation: **A, gated on the consumer audit.** "Structure" is the tier a
caller picks when it wants topology cheaply, and corroborating a lockfile is
not topology. If the audit finds a structure-tier consumer that depends on
`Lockfile` provenance, move that caller to an explicit request where possible;
if a stable public output cannot be preserved that way, choose B and record why.

**Decision (2026-09-26, review cycle 2): A.** The consumer audit in the
implementation log found no structure-tier caller that reads or serializes
`provenance` or `lockfile_match`. Every public output that serializes a
complete `RepoInfo` runs on `full()` and is unchanged. Bare `sniff repo --json`
serializes both fields from a `focused(...)` request, so it opts in explicitly
and its output stays byte-stable. The default is confined to the `RepoRequest`
constructors, so reversing it to B changes only `structure()` and `focused(...)`.

**Request and compatibility contract if A is chosen:** add an optional
lockfile-corroboration setting to the Sniff library's
[`RepoRequest`](../../lib/src/request.rs). `structure()` sets it to false,
`full()` sets it to true, and `focused(...)` defaults to false unless its caller
explicitly asks. The public `detect_repo_structure` convenience function uses
`structure()`; callers needing corroboration use `detect_repo_with_request`.
On deserialization, an absent setting means the old behavior (corroboration
enabled), so persisted request plans do not silently change. A newly
constructed request sets the value explicitly. Update struct literals,
request round-trip tests, and API documentation with this distinction. The
setting controls only corroboration: a request for dependency versions may
still read `Cargo.lock` when corroboration is declined.

Whichever is chosen, Sniff's existing request-tier vocabulary and work counters
apply. A structure-only request that declines corroboration must perform zero
lockfile read attempts and zero parses. The current
`filesystem.repo.lockfile_parses` counter measures parse attempts, while the
general file-open counter cannot isolate lockfiles. Add one stable lockfile
read-attempt counter at the shared lockfile read site and at
`CargoLockVersions::parse`; do not infer reads by subtracting unrelated file
opens.

`None` for `lockfile_match` already means "no lockfile, or unparseable"; its
field documentation also says no lockfile was parsed. Under this change it
additionally means "not requested". Preserve the existing wire shape and
document the ambiguity in the field and request API. Consumers that must
distinguish these cases should request corroboration and inspect the result;
adding a serialized state solely for a skipped check would expand the public
model without supplying lockfile evidence.

### 2. Make the Cargo lockfile check proportional to the question

Independently of who asks, `CargoLockVersions::parse` should not build a generic
value tree of 1,497 packages to answer name membership. A typed parser must
still scan the whole input and retain every package name and version because
other callers resolve versions; the expected saving is allocation and generic
value construction, not sublinear parsing.

Direction, not a mandated implementation: deserialize into a typed struct that
keeps only `package[].name` and `package[].version`, letting `serde` skip
`source`, `checksum`, and `dependencies` instead of allocating them. This keeps
a real TOML parser — **do not** hand-roll a line scanner over `Cargo.lock`; its
format is stable in practice but not a contract, and a scanner trades a
measurable cost for a silent-wrongness risk.

`CargoLockVersions::resolve` has other callers (version resolution for
workspace-inherited versions). Preserve its actual contract: it returns the
first version recorded for a name. The parsed index must therefore retain
every name and its versions in lockfile order. Preserve malformed-input
behavior too: entries with absent or non-string names or versions were skipped
by the generic parser, not grounds for rejecting the whole lockfile.

The `pnpm-lock.yaml` path has the same shape (whole-document generic YAML parse
to read `importers:` keys) and the same remedy. Include it only if measurement
on a pnpm-authoritative fixture shows it matters; this checkout's 186 samples
suggest it does, but this is a Cargo-authoritative repo with an incidental pnpm
layer and is not representative.

**Owner decision, 2026-09-26: the pnpm parser moves out of this fix.** The
measurement met the plan's threshold (`results.md` § 5), and review 3 found the
parser missing. The owner then ruled that lockfile corroboration as a whole
covers an arbitrary set of ecosystems and must be redesigned in
`2026-09-26-lockfile-corroboration`. The typed pnpm parser and its parity tests
were written and then moved to that feature, which owns every lockfile parser
from here on. This fix does not change pnpm parsing.

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

1. A consumer audit is recorded in the implementation log: readers of
   `MonorepoLayer::provenance`, `PackageSeed`/`Package` provenance, and
   `lockfile_match`, plus callers that serialize complete `RepoInfo` values
   across the workspace. Record each caller's request tier and whether it
   depends on the old value. The chosen default cites this audit. Check both
   explicit request plans and direct `detect_repo_structure` calls.
2. Under a fresh work collector, a structure detection that declines lockfile
   corroboration reports zero lockfile read attempts and zero lockfile parses;
   one that requests it performs the same lockfile checks as today. A focused
   request for dependency versions may still read a lockfile, so it must not
   claim zero reads solely because corroboration is off. An absent counter
   means zero.
3. For a request that asks for provenance, complete `RepoInfo` output is
   unchanged on a controlled fixture covering: Cargo, pnpm, and uv authorities;
   a matching lockfile; extra and missing members; an absent lockfile; an
   unparseable lockfile. Assert independently expected values, not only
   before/after equality. Cargo's existing name-presence check accepts extra
   lockfile entries; pnpm and uv require exact member sets. Confirm that
   difference explicitly instead of treating all extra entries as stale.
4. `CargoLockVersions` parity: for this checkout's `Cargo.lock` and for a fixture
   with duplicate package names at multiple versions, the typed parse resolves
   the same names to the same ordered versions as the current implementation.
   Keep the current implementation as a test-only reference for this comparison.
5. With option A, Darkmatter's existing `detect_repo_structure` capture
   automatically gets the cheaper result; with option B, migrate that call to
   an explicit request. `just test` in `darkmatter/` passes, including the
   three observation-boundary tests named
   in `2026-09-20-repo-perf`. The `REPOSITORY_DISCOVERY_COUNT` expectations are
   unchanged: this spec changes the cost of a discovery, never the count.
6. Request serialization tests prove that legacy plans without the new setting
   retain lockfile corroboration, while newly constructed structure and focused
   requests use their documented defaults. Public function and field docs,
   Sniff's request-cost skill text, and README descriptions of structure output
   are updated where behavior changes. The CLI's JSON output remains valid and
   accurately reflects the chosen request tier.
7. `just test` and `just lint` pass in `sniff/` with the recipe's `remote`
   feature coverage. Run affected Darkmatter and Claudine tests. No new CI
   matrix cell or timing gate is added; exercise the fixtures through existing
   test workflows on macOS, Linux, native Windows, and WSL2, reusing qualifying
   evidence for each environment.

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
