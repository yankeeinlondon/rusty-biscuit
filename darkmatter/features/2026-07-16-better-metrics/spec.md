---
status: draft
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-07-16
created: 2026-07-16
inputs:
  - ../_completed/2026-07-15-reference-graph/results.md
  - ../_completed/2026-07-15-reference-graph/review-2.md
  - ../2026-07-15-performance-followup/results.md
  - ../2026-07-15-performance-followup/benchmarks/README.md
  - ../2026-07-15-performance-followup/benchmarks/manifest.yaml
  - ../../lib/benches/
  - ../../lib/tests/benchmark_fixtures.rs
  - ../../cli/src/commands/compose.rs
  - ../../justfile
related:
  - ../_completed/2026-07-15-reference-graph
  - ../2026-07-15-performance-followup
---

# Better Metrics

## Status

Draft. This specification proposes a durable performance measurement platform
for Darkmatter. It is a foundation feature: it changes how we measure, not what
we optimize. No optimization may land under this feature's charter.

## Summary

Darkmatter is not short of benchmarks. It has 15 Criterion bench targets, seven
`just bench*` recipes, a deterministic fixture generator, an immutable fixture
manifest, a manifest-drift guard test, and a dated run-record contract. That
machinery was built by the 2026-07-15 performance follow-up and it works.

The problem is that **a performance gate passed while users regressed**, and no
part of that machinery was capable of noticing. This feature exists because of
that specific failure, and its success is measured against it: the same
regression, replayed, must fail the gate.

Two structural defects allowed it:

1. **Benchmarks measure a workload nobody runs.** Bench fixtures pin *document*
   bytes but leave `ComposeOptions` near-default — no baseline schema and an
   empty context. A real CLI invocation applies the Darkmatter baseline schema
   and captures the always-on datetime context plus any `ctx.*` groups the
   document requests. The benchmark therefore exercised a materially different
   options identity from the command it was used to represent.
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
(`../_completed/2026-07-15-reference-graph/results.md`):

| Fixture | Baseline | Candidate | Δ median | Verdict |
|---|---|---|---:|---|
| `small` | 167.18 µs | 211.83 µs | +44.65 µs (+26.7 %) | PASS — under the 100 µs floor |
| `large` | 6.1351 ms | 6.2115 ms | +76.4 µs (+1.25 %) | PASS — under both |
| `multi_transclusion` | 5.8717 ms | 6.0398 ms | +168.1 µs (+2.86 %) | PASS — under the 5 % floor |

The rule — *fail only when a regression exceeds **both** 5 % **and** 100 µs* —
was applied correctly. No fixture tripped both gates.

The performance follow-up then measured the code-equivalent pre-opacity audit
revision `51c1f16e1` against `b425fb466` at the CLI level and found four compose
cases regressed **+14 % to +35 %**, with `compose_trivial` moving
10.49 → 13.67 ms (`../2026-07-15-performance-followup/results.md`). The commits
between `51c1f16e1` and `db7e46792` are documentation/planning-only, so this is
the same production baseline as the construction comparison. Same host.
Opposite verdict.

Neither measurement is wrong. They disagree because the Criterion fixture builds
near-default `ComposeOptions`, while the CLI applies the Darkmatter baseline
schema by default (`cli/src/commands/compose.rs`) and uses a captured,
demand-driven context. The regression was localized to reference-graph/options
identity construction in the command setup envelope; the microbenchmark neither
constructed the same identity nor measured that envelope. The bench could not
see the cost it was used to gate.

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
4. Make every performance-sensitive change runnable against a declared
   known-good revision in the same measurement session, rather than requiring a
   later feature to reconstruct the comparison.
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

- **15 Criterion bench targets** in `lib/benches/` registered in `lib/Cargo.toml`.
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
- **A retained in-crate harness** (`lib/src/perf_harness.rs`) for crate-private
  targets. It writes Criterion-compatible sample vectors without widening the
  production API and is part of the platform that must be promoted.

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
cheap, then appears as +3.3 ms in the measured `md compose` command workload.

**P3 — No comparison anchor outlives a feature.** Criterion's
`--save-baseline` writes to a local `target/`, which is not committed, is wiped
by `cargo-sweep`, and is per-worktree. No durable record names the known-good
revision and workload contract to rebuild in a later same-session comparison,
so regressions are found by archaeology (as here: two features and a bisect)
rather than by a gate.

**P4 — Bench targets are named after project archaeology.** `phase6_interpolation`,
`phase8_render`, `phase9_remote`, `phase10_residuals` name the phases of a
completed plan. A newcomer cannot tell what they cover, whether they overlap, or
which is authoritative for a given operation.

**P5 — Host fitness is assumed, never checked.** Every result depends on a
stable-enough measurement window, and nothing verifies it. Measurements taken
under noisy or drifting load are silently recorded as facts. (This is live: the
current worktree cannot be trusted to measure while other work runs.)

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

### AD-1 — A workload identifies the full execution recipe

A benchmark case is a **workload**: a manifest entry that references a document
fixture, an execution profile, and a measurement boundary. Fixture identity
alone is insufficient, and an arbitrary Cartesian product of every fixture and
profile is not implied.

Execution profiles are committed JSON documents. The initial set is:

- `micro-minimal` — `ComposeOptions::new()` with an empty context. This retains
  today's isolation workload for mechanism analysis.
- `micro-cli-frozen` — the Darkmatter baseline schema, a committed trigger
  registry, and a frozen representative context. This lets Criterion attribute
  costs under a CLI-shaped options value without pretending to measure process
  startup or live discovery.
- `command-cli-default` — the release `md compose` binary with the ordinary
  baseline schema, trigger discovery, and normal demand-driven context capture.
  No benchmark-only context injection is allowed on this path. The runner
  materializes the declared workspace fixture into one temporary root used by
  both binaries, so discovery does not depend on either source worktree.
- `command-cli-stateful` — `command-cli-default` plus representative public
  `--state` and `--set` inputs. It does not claim to model Claudine-only library
  configuration such as `magic_paths`, which `md compose` cannot express.

This distinction is intentional. Freezing the command context would make the
run reproducible by changing the user-visible operation being measured. Command
profiles therefore freeze the **recipe** and record the resolved context group
names and redacted shape in each run; micro profiles may freeze concrete
`ComposeOptions` values for attribution.

Every profile declares:

- runner and boundary;
- argument vector, working-directory fixture, stdin/TTY mode, and output mode;
- an environment allowlist plus explicit overrides (`NO_COLOR`, locale, and
  timezone included);
- context policy (`frozen` or `live-demand-driven`), trigger-root policy, and
  cache state (`cold`, `warm`, or `not-applicable`);
- network, shell, and prompt policy — default deny for all three;
- expected exit status and output-identity policy.

Paths in a profile are logical, repository-relative references. The runner
resolves them through `biscuit_file::FileReference`; absolute checkout paths are
run facts and never profile identity.

Profile identity is independent of production cache/graph identity. The
manifest records the profile file's byte count and a `biscuit-hash` xxHash64 of
its committed bytes. Reusing `compose_cache_fingerprint` or
`ReferenceGraphOptionsIdentity` would couple the measuring instrument to the
implementation under measurement and would invalidate profiles whenever an
internal identity domain changes.

One non-production Rust support module under `benchmarks/support/` owns
construction of the two micro profiles and is included by the Criterion targets
and manifest guard. Runner adapters consume the same manifest/profile schema.
An authoritative benchmark or command run missing a workload/profile
declaration is a manifest validation error. Rust cannot make arbitrary
hand-written bench code a compile error, so the earlier draft's stronger claim
is not enforceable and is removed.

### AD-2 — Three boundaries, matched to the claim

Every workload declares one of three boundaries:

- **Component** — Criterion or the retained in-crate harness around a focused
  mechanism. It explains *why* a result moved and is never sufficient for a
  broader operation claim.
- **Library operation** — an end-to-end public Darkmatter library operation,
  normally measured in-process with Criterion. It may gate a claim specifically
  about that library API.
- **Command** — a release CLI process measured with `hyperfine`. It is the
  authority for `md` latency and other command-level user claims.

The rule is not "Criterion can never gate." The rule is that the authoritative
boundary must contain the operation named by the claim. The reference-graph
failure used a component result to pass an `md compose` claim; the command
boundary would have prevented that category error.

Explicitly retained from follow-up AD-A: CLI and PTY evidence do not run through
`just bench`. Each runner writes the common run-record shape, and interactive
and piped workloads remain distinct.

Before timing two implementations, the runner executes each once outside the
timed samples and requires equal exit status and output identity. Markdown
outputs use Darkmatter's frontmatter/body hash; other outputs use
`biscuit-hash`. A faster command that changed output is a correctness failure,
not a performance result.

### AD-3 — Workload budgets plus a measured noise bound

There is no universal 2 %/1 ms threshold. Different operations have different
user budgets and noise floors. Each authoritative workload records, before
capture:

- a relative regression budget;
- an absolute regression budget;
- maximum admissible sample dispersion and bracket drift;
- minimum warm-up and sample counts; and
- the required confidence rule.

Command comparisons use a same-session `candidate A → baseline → candidate B`
drift bracket. Let `B` be the baseline median and let `C` be the lower (more
conservative) of the two candidate medians. `measured_drift` is the absolute
difference between the two candidate medians. A regression fails only when:

1. `C - B` exceeds `max(relative_budget × B, absolute_budget, measured_drift)`;
2. the bootstrap 95 % interval for the baseline does not overlap either
   candidate interval; and
3. the baseline and both candidate arms produced equivalent output.

The initial `compose_trivial × command-cli-default` workload declares 5 % and
1 ms. The known +3.3 ms / approximately +31 % regression clears both product
budgets, the observed drift bracket, and the confidence rule. The exact budget
is now attached to the user-visible command workload rather than borrowed from
a microbenchmark with a different scale.

Raw observation vectors remain mandatory. Thresholds are declared before the
baseline is captured, and a no-win disposition remains a legitimate close.

### AD-4 — One durable, area-level evidence home

Promote the follow-up's layout to `darkmatter/benchmarks/`, owned by the area:

```text
darkmatter/benchmarks/
├── README.md              # schemas, boundary rules, and run-record contract
├── generate.sh            # deterministic document-fixture generator
├── manifest.yaml          # fixture, profile, and workload identity authority
├── fixtures/              # committed, byte-identical documents and roots
├── profiles/              # committed execution-profile JSON
├── baselines/             # known-good refs and advisory historical observations
├── run-command.ts         # portable hyperfine orchestration; no shell pipelines
├── recompute.ts           # statistics from retained observation vectors
└── raw/<checkpoint>/<run-id>/
```

The follow-up's feature-local directory remains its historical record. Promotion
copies fixture bytes once, verifies byte and Darkmatter-hash equality, and makes
the area manifest authoritative for future work; historical paths and results
are not rewritten.

`lib/tests/benchmark_fixtures.rs` becomes the area-manifest guard and validates
fixtures, profiles, workload references, budgets, and generator version. It also
rejects duplicate ids and a command workload whose profile permits network,
shell execution, or prompts without an explicit workload-level exception.

### AD-5 — Fitness is a same-session quality check

The strict gate is based on measurements every supported OS can provide:
warm-up stabilization, sample dispersion, and pre/post drift calibration. A run
with excessive dispersion, failed calibration, or a drift bracket larger than
the workload's admissible noise aborts and writes an **invalid** run record. An
invalid record retains its observations for diagnosis but is inadmissible as a
pass or fail.

Use `sniff` for the static host fingerprint: OS, architecture, CPU identity,
logical core count, and memory. `sniff` does not currently expose portable load
average, active-core, or thermal-throttling state, so this feature must not
silently introduce hand-rolled platform probes. If those signals become
available through `sniff`, record them as supplementary context; absence is not
an error and they are not a hard gate in v1.

No committed absolute calibration range is used. Such a range would be an
Apple-M4-Max-specific policy masquerading as a cross-platform invariant. The
fitness decision comes from within-run dispersion and drift, while the host
fingerprint explains and groups historical observations.

Fitness logic accepts an injected probe result so fit/unfit/unsupported cases
are deterministic unit tests. Real macOS, Linux, and Windows smoke runs prove
the platform adapters and record shape; a synthetic-load test is supporting
evidence, not a reliable automated assertion on every scheduler.

### AD-6 — Committed references, same-session hard baselines

`benchmarks/baselines/` commits the known-good revision and workload/profile
revision for each gate, plus historical observations for trend analysis. The
committed absolute median is **advisory**: even identical code has drifted by
approximately 50 % across sessions on the current host, so an old number cannot
be a hard gate.

For a hard comparison, the runner builds the declared known-good revision and
the candidate into isolated target directories, then measures both in one
session under AD-5. Command workloads place both absolute binaries against the
same fixture root, working directory, environment, and cache policy. The run
record captures the commits, dirty candidate diff identity, Cargo lockfiles,
toolchain, build commands, and binary hashes.

The candidate is bracketed around the baseline within one `hyperfine`
invocation. Component comparisons should prefer the retained harness's
sample-by-sample interleaving; where that is impossible they use the same
bracket rule. A baseline revision that no longer builds is a blocked gate, not
permission to advance the reference automatically.

Baseline refs are refreshed only by an explicit owner decision backed by a
passing comparison, equivalent output, and a recorded reason. This prevents
slow regressions from ratcheting into the platform while still allowing an
intentional product-budget change.

### AD-7 — `--perf` separates structural time from breakdowns

`--perf` remains diagnostic attribution; `hyperfine` wall time is the command
gate. Its command envelope becomes a non-overlapping structural tree:

```text
compose command (run_compose envelope)
├── resolve input
├── load input
├── capture context
├── prepare options
├── validate references
├── compose pipeline
├── emit output and diagnostics
└── unattributed
```

`prepare options` is self time and explicitly excludes `validate references`.
This intentionally corrects the old inclusive `build options` meaning; keeping
that meaning would preserve the double count that caused P6. Historical reports
retain their original interpretation and are not rewritten.

Context-group timings and compose-stage timings remain **breakdowns** beneath
their structural parent. They may overlap or run concurrently and therefore do
not participate in reconciliation. `unattributed` is
`max(0, envelope total - sum(structural children))`, making missing coverage
visible without manufacturing negative time.

The public `ComposePerfReport` / `ComposePerfMetric` flat library contract stays
source-compatible. Claudine already consumes those types and builds a richer
tree; changing the public struct has a HIGH downstream blast radius and is not
required to fix the Darkmatter CLI envelope. The CLI-private projection renders
with `biscuit_terminal::MetricsTree` (a `TerminalRenderable` component) rather
than hand-built spacing or ANSI strings.

### AD-8 — Catalog now; archaeology renames later

Audit all 15 Criterion targets and the retained in-crate harnesses into the area
manifest. For each, record boundary, workloads, owner, and whether it is
authoritative, diagnostic, redundant, or historical.

Do **not** rename `phase6_*`, `phase8_*`, `phase9_*`, or `phase10_*` in this
feature. Renaming has no bearing on the known failure, touches historical
commands and scripts, and is the first scope item that should be deferred. A
later cleanup may rename active targets from the catalog with coordinated link
updates.

## Compatibility and Correctness Invariants

1. No optimization and no behavior change to compose, render, hash, or
   validation lands under this feature. Production-path edits are limited to
   additive measurement/profile support and the `--perf` envelope correction.
2. Fixture bytes already committed by the follow-up do not change. If a promoted
   fixture must change, it gets a new id and a generator version bump.
3. The follow-up's and the 2026-07-12 review's recorded evidence is not
   rewritten. Cross-links only.
4. The public `ComposePerfReport`, `ComposePerfMetric`, and `ComposeStage`
   contracts remain source-compatible. The CLI-private inclusive `build options`
   measurement is deliberately replaced by non-overlapping `prepare options`
   self time.
5. The platform runs on macOS, Linux, and Windows. Host-fitness probing is
   target-gated where necessary, but unsupported load/thermal signals never
   prevent a platform from running the portable calibration and drift checks.
6. Command profiles are non-interactive and side-effect-denying by default. A
   workload that intentionally exercises a side effect must use an isolated
   local fake and declare the exception in the manifest.
7. The command runner passes argument arrays, uses `hyperfine --shell=none`, and
   does not depend on Unix redirection or `/dev/null`.
8. No write-mode formatter is authorized.

## Verification

### Platform behavior

- The manifest guard fails when a fixture byte changes, when a profile
  definition changes without a manifest update, when a workload references a
  missing id, or when a profile is added without identity.
- The registry validator rejects an authoritative workload without a declared
  profile, boundary, budgets, output policy, or effects policy.
- Injected fit, unfit, excessive-drift, and unsupported-signal probe cases have
  deterministic tests.
- Run records without a fitness verdict or raw observations are rejected; an
  invalid run is retained but cannot produce a pass/fail verdict.
- Frozen micro-profile construction is byte-identical across processes and
  clones. Command-profile **recipe** identity is byte-identical; live resolved
  context is recorded as a per-run fact and is not required to be identical.
- Baseline and candidate output mismatch aborts before timed sampling.
- Structural `--perf` children never exceed the command-envelope total;
  concurrent/overlapping details are marked as breakdowns and excluded from
  reconciliation.

### The acceptance test — replay the failure

This is the test that decides whether this feature was worth building.

Reconstruct the reference-graph regression: measure `db7e46792` (pre-opacity)
against `b425fb466` (post-opacity) on `compose_trivial` at the **command tier**
with the **`command-cli-default` profile**, under a passing fitness gate and the
same-session drift bracket.

**The new gate must fail.** If it passes, the platform has reproduced the
original defect and the design is wrong.

Additionally, the `micro-minimal` component measurement must reproduce the much
smaller delta that originally passed. The result demonstrates that both
measurements are valid for their declared boundaries and that only the command
measurement can decide the `md compose` claim.

### Required gates

- Use `sniff repo packages` and GitNexus impact analysis to record the affected
  packages, package areas, and downstream consumers before selecting gates.
- Darkmatter `just test`, `just lint`, and `just build` for the affected area;
  use Nextest through the `just` recipes, never `cargo test`.
- `git diff --check`.
- GitNexus `detect_changes({ scope: "compare", base_ref: "main" })` before
  commit; scope must show no compose/render behavior change beyond invariant 1.
- Real command-runner/fitness smoke records on macOS, Linux, and Windows.

## Acceptance Criteria

1. Every authoritative workload references a fixture, execution profile, and
   measurement boundary with recorded manifest identity and a passing guard.
2. `command-cli-default` invokes ordinary release `md compose` with live
   demand-driven context capture and is authoritative for default CLI compose
   claims; `micro-cli-frozen` is explicitly diagnostic.
3. A performance claim cites a measurement whose boundary contains the named
   operation. A component result alone cannot pass a command claim.
4. Each workload declares relative and absolute budgets before capture, and a
   hard verdict also clears its same-session drift and confidence rules.
5. `darkmatter/benchmarks/` is the area-level evidence home; the follow-up's
   conventions are promoted intact and its historical evidence is untouched.
6. Fitness uses portable dispersion/drift checks, records a `sniff` host
   fingerprint, and marks inadmissible runs invalid rather than warning past
   them.
7. A hard gate rebuilds and measures its declared known-good revision in the
   same session. Committed absolute medians are advisory trend data only.
8. `--perf` structural timings reconcile without double counting;
   `prepare options` excludes `validate references`, while detailed concurrent
   timings are labeled breakdowns.
9. The public compose-performance report remains compatible, and terminal
   presentation uses `MetricsTree` through `TerminalRenderable`.
10. All 15 Criterion targets and retained in-crate harnesses are cataloged;
    phase-name renames are deferred.
11. **The replayed reference-graph regression fails the new gate.**
12. No optimization landed under this feature.

## Review Decisions

> **Reader's note:** review changed four draft proposals that looked attractive
> in isolation but conflicted with existing evidence or repository contracts.

- **Independent profile hashes, not production fingerprints.** Production
  identity reuse would be self-checking, but it would make the instrument change
  identity whenever the measured implementation changes. Raw committed profile
  bytes plus `biscuit-hash` give stable ownership and a simpler guard.
- **Live command context, frozen micro context.** Freezing both is maximally
  reproducible but stops measuring the ordinary CLI. Making both live is
  representative but makes mechanism attribution unstable. Splitting the
  profiles preserves both purposes without calling them equivalent.
- **Same-session hard baseline, committed trend reference.** A single-host hard
  median is simple but produced up to approximately 50 % drift on identical
  code; per-host-class medians multiply maintenance without removing session
  drift. Rebuilding the declared baseline in the candidate session costs more
  time but is the only option supported by the existing evidence.
- **CLI-private tree, public flat report.** Replacing `ComposePerfReport` with a
  tree would centralize structure, but GitNexus reports HIGH impact (17 direct
  dependents and 23 affected symbols), including Claudine's existing tree.
  Correcting Darkmatter's non-overlapping command envelope solves P6 without a
  cross-area migration.
- **Catalog before rename.** Immediate phase-target renames improve discovery
  but churn historical commands and do not affect correctness. A catalog closes
  the ownership gap now and makes any later rename evidence-driven.

No blocking design questions remain after review. Workload-specific budgets may
be tuned only through the explicit baseline-refresh decision in AD-6; that is a
recorded product decision, not an implementation-time open question.
