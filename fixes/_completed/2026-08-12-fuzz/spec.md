# Opt-In Fuzz Testing per Package

Status: draft — questions collected for a brainstorming session

Companion to `features/2026-08-12-perf-opt-in/spec.md` (its question 19 asks
whether fuzzing shares the same opt-in mechanism; this spec is the fuzz side
of that answer). Builds on `fixes/2026-08-06-cicd/spec.md`: the package is
the unit of CI selection, execution, and result identity.

## Where fuzzing stands today — measured 2026-08-12

`fuzz-nightly.yml` (02:00 UTC, ~15 min, green daily) runs **five hardcoded
targets across two packages** — biscuit-file's `pdf_extract`,
`toml_roundtrip`, `yaml_roundtrip`, `json5_roundtrip` and darkmatter's
`markdown_parser`. Per target it does two distinct things:

1. **Regression replay** — every committed crash fixture under
   `fuzz/crashes/<target>/` re-runs with `-runs=0`. This half works: real
   past finds are committed for all four biscuit-file targets, and they can
   never silently regress.
2. **Discovery** — `-runs=10000 -max_total_time=300`. This half is an
   illusion: libFuzzer executes thousands of inputs per second, so the runs
   cap ends discovery in **seconds** (~14 of the 15 job minutes are
   compilation), and **no corpus persists between nights** (only
   `pdf_extract` has a committed seed corpus), so every night re-explores the
   same shallow frontier from scratch.

New crashes upload as artifacts with commit-the-minimized-input instructions.
The target list lives in the workflow — the `areas.json` anti-pattern in
miniature: adding a fuzz target means editing CI, and the workflow, not the
package, owns the fact.

## The target design

A package **opts into fuzzing** and owns everything about it: which targets,
what time budget, what seed data. CI discovers opted-in packages the same way
it discovers everything else (package policy + workspace structure), runs
each package's fuzz job on a schedule (and possibly on impact — question 8),
persists the corpus so coverage-guided exploration **compounds across runs**,
gates on crash-replay regressions, and surfaces new finds loudly. A package
with no fuzz opt-in pays nothing.

Strawman manifest shape, deliberately underspecified:

```toml
[package.metadata.ci.fuzz]
targets = ["pdf_extract", "toml_roundtrip"]   # or derived from fuzz/fuzz_targets/?
budget-minutes = 20                            # discovery time per night
sanitizer = "address"
```

## What already exists to build on

- The canonical `fuzz` recipe in every package area (no-op without targets)
  — local parity is already free: a package opts in by having a `fuzz/`
  crate, the same way it owns `level2_*` tests by writing them.
- The committed-crash-fixture convention (`fuzz/crashes/<target>/` + README)
  and the replay-then-discover job shape — both worth keeping.
- The scope calculator's dependency-closure resolution, if fuzz ever runs
  on-impact rather than only on schedule.
- The result grid, baseline governance, and verdict — if fuzz results join
  the contract (question 12).
- `nextest` explicitly excludes fuzz from sanity/test/PR gates today; that
  boundary should survive.

## Design questions for the brainstorm

### Opt-in mechanism

1. **Derive or declare?** A `fuzz/fuzz_targets/*.rs` directory is
   file-visible — this is the cheap-derivation case the parent spec's R7
   permits (unlike name-encoded tiers). But budgets, sanitizers, and seed
   policy are not derivable. Split: presence derived, configuration
   declared? And must the undeclared-but-present case fail a contract test
   (a package with a `fuzz/` dir that CI doesn't know about), mirroring the
   AC12 tier guards?
2. **Granularity**: is the opt-in per package or per target? Can one target
   get a bigger budget than its siblings (pdf parsing wants more time than
   TOML round-tripping)?
3. **Same metadata namespace as perf?** `[package.metadata.ci.fuzz]` beside
   `[package.metadata.ci.perf]`, one shared "scheduled deep tier" table, or
   keep them fully separate? (Perf spec question 19 — answer once, here.)

### Corpus strategy — the effectiveness lever

4. **Where does the corpus live?** Options: `actions/cache` per
   `{package, target}` (fast, but the repo quota measurably holds ~13 entries
   total — see `project_ci_cache_quota_saturated`; corpora are small but the
   eviction pool is shared with rust-caches); committed to the repo
   (auditable, replayable locally, but binary churn in git); artifact
   upload/download chains (survive quota, awkward plumbing); external bucket
   (the kache/S3 lever, shared with the cache strategy decision). What is
   the loss mode when the corpus vanishes — silent shallow restart (today's
   behavior) or a visible warning?
5. **Corpus hygiene**: periodic `cargo fuzz cmin` (minimization) — on what
   cadence, and does the minimized corpus get re-persisted? Is there a size
   ceiling per target?
6. **Seed policy**: should every target be required to ship a seed corpus
   (pdf_extract has one; the others start from nothing)? Do fixtures from
   the package's test suite auto-seed?
7. **Dictionaries**: TOML/YAML/JSON5/Markdown are grammar-heavy — libFuzzer
   dictionaries (`-dict=`) multiply effectiveness on exactly this class. Per
   target in the fuzz crate, or not worth the maintenance?

### Scheduling and budget

8. **Schedule-only, or also on-impact?** Nightly deep runs are the classic
   shape. Is there additionally a cheap PR-time slice for impacted opted-in
   packages — replay-only (fast, gating), or replay + a 60-second discovery
   taste? Full discovery on PRs is surely too slow.
9. **Total nightly ceiling**: budgets are per package, but the sum grows
   with adoption. Serial on one runner (bounded wall-clock, ordering
   questions) or matrix fan-out (bounded per-job, unbounded runner count)?
   What happens when the sum of declared budgets exceeds the ceiling —
   round-robin nights, proportional scaling, or loud rejection?
10. **Toolchain pinning**: cargo-fuzz needs nightly Rust. Pin a known-good
    nightly (reproducible, ages) or track latest (fresh, breaks
    occasionally)? Who owns bumping it?

### Findings, gating, and visibility

11. **What is gating?** Proposed: crash-fixture replay failures are hard red
    (a regression on a known bug); harness breakage (fuzz target fails to
    build) is red; *new* discoveries are loud-but-advisory until triaged. Is
    that the right split, and does a new find open an issue automatically,
    or only upload an artifact + summary?
12. **Do fuzz results join the result contract?** A `{package, environment,
    tier="fuzz"}` cell in the schema-v2 grid (replay = pass/fail evidence,
    discovery = stats), or does fuzz stay a standalone workflow with its own
    summary? If standalone: what is the minimum failure surface so a fuzz
    pipeline that stops producing data cannot look healthy (the bench-nightly
    lesson — its upload failed silently for weeks)?
13. **Crash-fixture lifecycle**: today's convention is minimize + commit
    under `fuzz/crashes/<target>/`. Formalize: who minimizes (CI
    auto-minimizes and opens a PR? the finder?), when does a fixed crash's
    fixture migrate from "crash" to "regression corpus", and do fixtures
    expire?
14. **Triage ownership**: a new find in `biscuit-file` lands on whom? Does
    the fuzz opt-in carry an `owner` field like exclusions do?

### Execution environment

15. **Sanitizers**: ASan is cargo-fuzz's default; do we also want UBSan
    passes, and is Miri (different tool, overlapping value on UB) in or out
    of scope?
16. **Platform**: libFuzzer discovery on Linux only (cheapest, standard), or
    is there value in macOS/Windows fuzz legs for platform-conditional
    parsing code? (Current nightly is Linux-only; nothing platform-specific
    is fuzzed today.)
17. **Structure-aware fuzzing**: `arbitrary`-based targets (typed inputs
    rather than raw bytes) for the format round-trippers — adopt as the
    recommended pattern, or leave per-package?

### Growth and guards

18. **Which packages should opt in first?** Natural candidates by input
    surface: biscuit-file (already in), darkmatter (already in — expression
    parser and frontmatter beyond `markdown_parser`?), sniff's parsers,
    biscuit-hash, tree-hugger (tree-sitter inputs), schematic (OpenAPI
    ingestion). Is there a push (recommend in review for parser-shaped code)
    or purely pull?
19. **Non-vacuity guards**: contract tests that (a) every declared target
    exists and builds, (b) every `fuzz/` dir is either opted in or
    explicitly excluded with governance, (c) the replay step actually
    replays a non-empty fixture set for packages that have one — proven by
    neutering, per house rules.
20. **Migration**: the two current packages move onto the mechanism with no
    coverage gap, and `fuzz-nightly.yml`'s hardcoded list dies — the same
    "no silent drops" bar as the CI cutover (name the replacement for each
    of the five targets in the retirement PR).

## Out of scope (proposed — confirm in brainstorm)

- Fixing any bug fuzzing finds (normal burn-down).
- The perf opt-in itself (`features/2026-08-12-perf-opt-in/spec.md`) — only
  the shared-mechanism question (3) is joint.
- OSS-Fuzz / ClusterFuzz-style continuous external fuzzing — worth knowing
  it exists as the scale-up path; not this spec.

## Sequencing

Draft → brainstorm (jointly with or adjacent to the perf brainstorm, since
question 3 is shared) → ratified spec → plan. Until then `fuzz-nightly.yml`
keeps running unchanged — its replay half is real protection and its cost is
small; the corpus-persistence and budget fixes land with the redesign rather
than as patches to a workflow scheduled for retirement.
