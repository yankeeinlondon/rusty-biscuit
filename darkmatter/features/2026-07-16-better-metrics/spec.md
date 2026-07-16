---
status: draft
reviewed: false
created: 2026-07-16
inputs:
  - ../2026-07-15-reference-graph/results.md
  - ../2026-07-15-reference-graph/review-2.md
  - ../2026-07-15-performance-followup/results.md
  - ../2026-07-15-performance-followup/benchmarks/README.md
  - ../2026-07-15-performance-followup/benchmarks/manifest.yaml
  - ../../lib/benches/
  - ../../lib/tests/benchmark_fixtures.rs
  - ../../cli/src/commands/compose.rs
  - ../../justfile
related:
  - ../2026-07-15-reference-graph
  - ../2026-07-15-performance-followup
---

# Better Metrics

## Status

Draft. This specification proposes a durable performance measurement platform
for Darkmatter. It is a foundation feature: it changes how we measure, not what
we optimize. No optimization may land under this feature's charter.

## Summary

Darkmatter is not short of benchmarks. It has 14 Criterion bench targets, seven
`just bench*` recipes, a deterministic fixture generator, an immutable fixture
manifest, a manifest-drift guard test, and a dated run-record contract. That
machinery was built by the 2026-07-15 performance follow-up and it works.

The problem is that **a performance gate passed while users regressed**, and no
part of that machinery was capable of noticing. This feature exists because of
that specific failure, and its success is measured against it: the same
regression, replayed, must fail the gate.

Two structural defects produced it:

1. **Benchmarks measure a workload nobody runs.** Bench fixtures pin *document*
   bytes but leave `ComposeOptions` near-default — no baseline schema, an empty
   `ctx.*` context. Real CLI invocations populate both. A cost proportional to
   options/context size is therefore invisible to the bench and fully visible to
   the user.
2. **Evidence is feature-local and dies with the feature.** Fixtures, manifest,
   thresholds, and baselines live under
   `features/2026-07-15-performance-followup/benchmarks/`. Nothing carries
   forward, so the next feature either reinvents the harness or measures against
   nothing. There is no persistent baseline that a regression can be detected
   *against* outside of a single feature's lifetime.

This feature promotes the follow-up's proven conventions out of a feature
directory into an area-level platform, and closes the representativeness gap
that let the regression through.

## Motivating Evidence

This is not hypothetical. It is the recorded outcome of the two features that
just shipped, and it is the acceptance test for this one.

The Opaque Reference Graph feature measured its own construction cost
cross-commit on an idle host and **passed** its declared gate
(`2026-07-15-reference-graph/results.md`):

| Fixture | Baseline | Candidate | Δ median | Verdict |
|---|---|---|---:|---|
| `small` | 167.18 µs | 211.83 µs | +44.65 µs (+26.7 %) | PASS — under the 100 µs floor |
| `large` | 6.1351 ms | 6.2115 ms | +76.4 µs (+1.25 %) | PASS — under both |
| `multi_transclusion` | 5.8717 ms | 6.0398 ms | +168.1 µs (+2.86 %) | PASS — under the 5 % floor |

The rule — *fail only when a regression exceeds **both** 5 % **and** 100 µs* —
was applied correctly. No fixture tripped both gates.

The performance follow-up then measured the **same two commits** at the CLI
level and found four compose cases regressed **+14 % to +35 %**, with
`compose_trivial` moving 10.49 → 13.67 ms
(`2026-07-15-performance-followup/results.md`). Same code. Same host. Opposite
verdict.

Neither measurement is wrong. They disagree because the Criterion fixture builds
near-default `ComposeOptions`, while the CLI applies the Darkmatter baseline
schema by default (`cli/src/commands/compose.rs:657`) and carries a fully
populated `ctx.*` context. The new per-capture work scales with exactly those
two inputs. The bench could not see the cost it was gating.

Three properties of the platform failed at once, and each maps to a problem
below:

- the fixture was not representative (**P1**);
- the threshold was expressed against the fixture's own tiny absolute cost
  rather than against a user-visible operation, so +26.7 % was absorbed as
  "under the µs floor" (**P2**);
- nothing compared the library gate against the command-level reality until a
  *different feature* happened to measure it (**P3**).

## Goals

1. Make the default measured workload resemble a real invocation, so a cost that
   scales with options or context size cannot hide.
2. Give Darkmatter one durable, area-level evidence home that outlives any
   feature.
3. Express regression thresholds against user-visible operations, not against a
   microbenchmark's incidental scale.
4. Detect regressions continuously against a committed baseline, rather than
   only when a feature goes looking.
5. Make measurement refuse to run — loudly — on a host too noisy to produce a
   trustworthy number.
6. Make per-phase attribution (`--perf`) structurally unambiguous.
7. Preserve every convention the follow-up got right: immutable fixture
   identity, deterministic generation, dated run records, predeclared
   thresholds, retained raw samples.

## Non-Goals

- Fixing any regression, including the reference-graph setup cost. This feature
  builds the instrument; the Opaque Reference Graph feature owns its
  disposition. **No optimization lands here.**
- Building CI infrastructure or a hosted results dashboard.
- Cross-host comparison. Measurements remain same-host, same-session.
- Replacing Criterion or `hyperfine`.
- A universal runner. Architecture Decision A of the follow-up explicitly
  rejected forcing CLI and PTY evidence through `just bench`; that ruling stands.
- Rewriting the 2026-07-12 or 2026-07-15 historical evidence.

## Current State

### What exists and is worth keeping

- **14 Criterion bench targets** in `lib/benches/` registered in `lib/Cargo.toml`.
- **Seven `just bench*` recipes**: `bench`, `bench-schema`, `bench-compose`,
  `bench-render`, `bench-baseline`, `bench-compare`, `bench-dmls`.
- **A deterministic fixture generator** (`generate.sh`, versioned) plus 13
  committed fixtures.
- **An immutable fixture manifest** (`manifest.yaml`) recording per fixture:
  bytes, lines, headings, Darkmatter frontmatter/body hashes, and an xxHash64
  whole-file identity.
- **A manifest-drift guard** — `lib/tests/benchmark_fixtures.rs` recomputes every
  manifest field from committed bytes and fails on drift.
- **A dated run-record contract** — `raw/<checkpoint>/<run-id>/` owning commits,
  commands, host facts, environment, TTY mode, warm-up, sample count,
  dispersion, predeclared thresholds, and raw samples.

The fixture-identity and run-record discipline is genuinely good work. This
feature's job is to promote it, not to redesign it.

### Problems

**P1 — Fixtures pin the document, not the workload.** `manifest.yaml` records
document bytes and hashes with real rigor, and says nothing about
`ComposeOptions` or `ComposeContext`. Bench code constructs those ad hoc and
near-default. A "fixture" is therefore only half of a workload, and the recorded
half is the half that didn't vary.

**P2 — Thresholds are anchored to the fixture, not the user.** The 5 %/100 µs
rule is reasonable for a microbenchmark and produces a wrong answer at `small`'s
scale: a fixed per-invocation cost is absorbed as noise because the fixture is
cheap, then reappears as +3.3 ms on every real compose.

**P3 — No baseline outlives a feature.** Criterion's `--save-baseline` writes to
a local `target/`, which is not committed, is wiped by `cargo-sweep`, and is
per-worktree. There is no persistent number that a change can be measured
against, so regressions are found by archaeology (as here: two features and a
bisect) rather than by a gate.

**P4 — Bench targets are named after project archaeology.** `phase6_interpolation`,
`phase8_render`, `phase9_remote`, `phase10_residuals` name the phases of a
completed plan. A newcomer cannot tell what they cover, whether they overlap, or
which is authoritative for a given operation.

**P5 — Host fitness is assumed, never checked.** Every result depends on an idle
machine, and nothing verifies it. Measurements taken under load are silently
recorded as facts. (This is live: the current worktree cannot be trusted to
measure while other work runs.)

**P6 — `--perf` spans are nested but read as flat.** `build options`
(`compose.rs:230`–`386`) fully encloses `validate references`
(`compose.rs:277`–`321`). The follow-up's report reads their deltas as two
separate costs — "`validate references` 3.6 → 6.9 ms **and** `build options`
4.0 → 7.4 ms" — when it is one +3.3 ms cost counted twice. The instrument
invited the misreading.

**P7 — Evidence home is feature-local.** Per Architecture Decision A, the
follow-up put its evidence beside its spec. Correct for that feature; wrong as a
permanent home.

## Proposed Design

### AD-1 — Workload profiles are first-class fixture identity

A benchmark case is a **workload**: `(document fixture × options profile)`. Both
halves get recorded identity in the manifest; neither may be constructed ad hoc
in bench code.

Define a small, named, committed set of options profiles. At minimum:

- `minimal` — `ComposeOptions::new()`, empty context. What today's benches
  measure. Retained as the isolation profile for mechanism work.
- `cli-default` — exactly what `md compose` builds for a plain invocation:
  Darkmatter baseline schema applied, trigger schemas on, real `ctx.*` context
  shape. **This is the default profile for any user-facing claim.**
- `cli-rich` — `cli-default` plus `--state`/`--set` overrides and a populated
  magic-path set, representing a Claudine-style invocation.

Requirements:

1. Profiles are constructed by one committed, crate-visible builder shared by
   benches and run records. No bench hand-rolls options.
2. The context in a profile is a **frozen snapshot**, not a live capture. A
   profile that captures a live `ctx.*` is not reproducible across hosts or
   across seconds, and would make the fingerprint below meaningless.
3. The manifest records each profile's identity alongside fixture identity, and
   the guard test verifies it the same way it verifies fixture bytes.
4. A workload case declares its profile explicitly. A bench with no declared
   profile is a build error, not a silent `minimal`.

> **Open question O1** — the natural identity for a profile is the very
> `compose_cache_fingerprint` / options-identity machinery whose cost this whole
> investigation is about. Reusing it is elegant and self-checking; it also
> couples the measurement platform to code under active change, and a
> fingerprint change would invalidate every recorded profile identity. The
> alternative is an independent, simpler encoding owned by the manifest. **This
> needs a ruling before implementation.**

### AD-2 — Two tiers, explicitly declared

Every measurement declares its tier, and the tiers answer different questions:

- **Micro (Criterion, `lib/benches/`)** — establishes *mechanism*. May use
  `minimal`. Never authoritative for a user-facing claim.
- **Command (release CLI, `hyperfine`)** — establishes *user impact*. Uses
  `cli-default` or `cli-rich`.

A finding may only claim a win or pass a regression gate on the strength of a
**command-tier** result. A micro-tier result is supporting evidence for *why*.
The reference-graph case is exactly the failure this rule prevents: a micro-tier
PASS was treated as the gate.

Explicitly retained from follow-up AD-A: this does **not** funnel CLI or PTY
evidence through `just bench`. Each tier keeps its own runner and writes its own
run record.

### AD-3 — Thresholds anchor to a user-visible operation

Replace the per-fixture 5 %/100 µs rule with a threshold expressed against the
end-to-end operation the fixture stands in for.

Proposed default: **a change is a regression when it costs more than 2 % of the
command-tier median for that workload, or more than 1 ms absolute, whichever is
larger — measured at the command tier.**

Under this rule the reference-graph cost (+3.3 ms on a 10.49 ms
`compose_trivial`, ≈+31 %) fails clearly, which is the required outcome. The
`minimal`-profile micro number that passed is no longer eligible to be the gate.

Retained unchanged from the follow-up's evidence contract: thresholds are
declared **before** the baseline is captured; raw samples are retained; a no-win
disposition is a legitimate close.

> **Open question O2** — 2 %/1 ms is a starting proposal, chosen to fail the
> known case with margin. It should be calibrated against the measured run-to-run
> dispersion of `cli-default` on an idle host before it is ratified; a threshold
> tighter than the noise floor produces false failures and trains people to
> ignore the gate. **Needs a ruling, informed by AD-5's measured noise floor.**

### AD-4 — One durable, area-level evidence home

Promote the follow-up's `benchmarks/` layout to `darkmatter/benchmarks/`, owned
by the area rather than by a feature:

```
darkmatter/benchmarks/
├── README.md            # manifest schema, profiles, runners, run-record contract
├── generate.sh          # deterministic fixture generator (versioned)
├── manifest.yaml        # fixtures AND options profiles — one identity authority
├── fixtures/            # committed, byte-identical documents
├── profiles/            # committed options-profile definitions (AD-1)
├── baselines/           # committed rolling baselines (AD-6)
└── raw/<checkpoint>/<run-id>/
```

The follow-up's feature-local evidence stays where it is, as its historical
record. Migration copies conventions forward; it does not move or rewrite that
feature's recorded results.

`lib/tests/benchmark_fixtures.rs` moves with the manifest and extends to verify
profile identity.

### AD-5 — Host fitness gate

Measurement must refuse to produce a number on a host it cannot trust.

Before any run, the harness records and checks: load average, active core count,
thermal/throttling state where the OS exposes it, and the dispersion of a short
calibration workload against a committed expected range.

On failure the run **aborts with a diagnostic**. It does not warn-and-continue —
a warned-past result becomes a cited fact three documents later.

The fitness reading is recorded in the run record. A run record without one is
not admissible evidence.

Use `sniff` for host facts rather than hand-rolled probes.

> **Open question O3** — the calibration workload's expected range is itself
> host-specific, and Ken's host (Apple M4 Max) is the only one this has ever run
> on. Committing a range calibrated on one machine may make the gate useless
> elsewhere. Options: commit per-host-class ranges, derive the range from
> dispersion alone (host-independent), or gate on dispersion only and record the
> rest as context. **Needs a ruling.**

### AD-6 — Rolling committed baselines

Commit a baseline per `(workload, tier)` to `benchmarks/baselines/`, recorded on
a known-good host under a passing fitness gate, with its commit, host facts, and
dispersion.

This is the missing piece behind P3. A change compares against a committed
number rather than against whatever happens to be in a local `target/`. It makes
"did this regress?" answerable at the time of the change instead of by a later
bisect.

Baselines are refreshed deliberately, never automatically: an automatic refresh
would ratchet in exactly the kind of slow regression this exists to catch.

> **Open question O4** — baselines are host-specific, and this repo has one
> reachable host. A committed baseline is therefore honest only for that host,
> and a second developer would see false failures. Scope options: (a) accept
> single-host baselines and gate them on matching host facts, skipping elsewhere;
> (b) store per-host-class baselines; (c) treat baselines as advisory trend data
> and keep the gate same-session. **Needs a ruling — this decides whether AD-6
> is a gate or a trend line.**

### AD-7 — `--perf` spans form an explicit tree

Fix P6 at the instrument. Spans must be either non-overlapping, or explicitly
parent/child with nesting visible in the output — a child indented under its
parent, and a parent's self-time distinguished from its inclusive time.

The current output invites summing `build options` and `validate references`,
which double-counts a single cost. Any consumer reading two numbers as two costs
is reading the instrument as designed; the design is wrong.

This is a change to `--perf` output shape. It is user-visible and needs a
compatibility note, but `--perf` is a diagnostic surface rather than a data
contract.

### AD-8 — Benches named for what they measure

Retire archaeology names (`phase6_*`, `phase8_*`, `phase9_*`, `phase10_*`) in
favor of operation names. Audit the 14 targets for overlap and redundant
coverage, and record one authoritative target per operation.

This is the lowest-value item here and the easiest to defer. It is listed last
deliberately — if scope must be cut, cut this first.

## Compatibility and Correctness Invariants

1. No optimization and no behavior change to compose, render, hash, or
   validation lands under this feature. A `git diff` that touches those paths
   for reasons other than `--perf` span structure is out of scope.
2. Fixture bytes already committed by the follow-up do not change. If a promoted
   fixture must change, it gets a new id and a generator version bump.
3. The follow-up's and the 2026-07-12 review's recorded evidence is not
   rewritten. Cross-links only.
4. `--perf` span *content* is preserved; only its structure and presentation
   change.
5. The platform runs on macOS, Linux, and Windows. Host-fitness probing is
   OS-divergent by nature and any platform split must be target-gated. Windows
   compile evidence plus a real Linux behavioral run is the minimum bar for the
   fitness gate.
6. No write-mode formatter is authorized.

## Verification

### Platform behavior

- The manifest guard fails when a fixture byte changes, when a profile
  definition changes, and when a profile is added without identity.
- A bench declaring no profile fails to compile.
- The fitness gate aborts under synthetic load and passes on an idle host.
- Run records without a fitness reading are rejected.
- Profile construction is byte-identical across processes and across clones
  (a profile whose identity is unstable is not a fixture).

### The acceptance test — replay the failure

This is the test that decides whether this feature was worth building.

Reconstruct the reference-graph regression: measure `db7e46792` (pre-opacity)
against `b425fb466` (post-opacity) on `compose_trivial` at the **command tier**
with the **`cli-default` profile**, under a passing fitness gate.

**The new gate must fail.** If it passes, the platform has reproduced the
original defect and the design is wrong.

Additionally, the `minimal`-profile micro measurement must still show the small
delta it originally showed — demonstrating that the two tiers disagree *by
design*, and that the platform's contribution is knowing which one is the gate.

### Required gates

- Darkmatter `just test`, `just lint`, `just build`.
- `git diff --check`.
- GitNexus `detect_changes()` before commit; scope must show no compose/render
  behavior change (invariant 1).
- Linux behavioral run for the host-fitness probe; Windows compile evidence.

## Acceptance Criteria

1. A workload is `(fixture × options profile)`, both with recorded manifest
   identity, both verified by the guard test.
2. `cli-default` exists, matches what `md compose` actually builds, and is the
   default profile for user-facing claims.
3. A user-facing performance claim or regression gate cites a **command-tier**
   result. A micro-tier result alone cannot pass a gate.
4. Thresholds anchor to a user-visible operation and are declared before the
   baseline is captured.
5. `darkmatter/benchmarks/` is the area-level evidence home; the follow-up's
   conventions are promoted intact and its historical evidence is untouched.
6. Measurement aborts on an unfit host and records its fitness reading.
7. `--perf` output makes nesting unambiguous; `build options` and
   `validate references` can no longer be read as two independent costs.
8. **The replayed reference-graph regression fails the new gate.**
9. No optimization landed under this feature.

## Open Questions for the Owner

Consolidated from the design above. These are blocking for implementation, not
for this draft:

- **O1** — should profile identity reuse the existing options-identity
  machinery, or get an independent encoding owned by the manifest? (AD-1)
- **O2** — is 2 %/1 ms the right threshold, and what is the measured `cli-default`
  noise floor on an idle host? (AD-3)
- **O3** — how does host fitness calibrate on a repo with one reachable host?
  (AD-5)
- **O4** — are committed baselines a **gate** or a **trend line**? This is the
  most consequential question here: it decides whether AD-6 catches regressions
  automatically or merely records them. (AD-6)
- **O5** — scope. AD-1/AD-3/AD-4 are the core and directly address the known
  failure. AD-5 through AD-8 are valuable but severable. Should this land as one
  feature or as a core feature plus follow-ups?
