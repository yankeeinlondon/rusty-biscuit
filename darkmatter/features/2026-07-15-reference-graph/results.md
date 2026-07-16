# Reference-Graph Performance Evidence

Records the Phase 4 performance check for the opaque reference-graph work. The
bench target is `darkmatter/lib/benches/reference_graph.rs`, registered in
`darkmatter/lib/Cargo.toml`.

## Goal

Confirm that validating a **prebuilt** graph (`validate_references_with_graph`)
stays materially faster than rebuilding (`validate_references`), and that the
provenance the opacity cutover adds does not introduce a material graph
**construction** regression or superlinear work.

## Method

Three fixture shapes, each measured three ways:

- `build_and_validate` — `validate_references` (build **and** validate).
- `validate_prebuilt` — `validate_references_with_graph` with the graph built
  **outside** the timed loop; only provenance checking, descendant
  re-verification, and flattening are timed.
- `construct` — `reference_graph` alone; isolates provenance-construction cost.

Context (`ComposeContext`) is captured **once per fixture**, outside every timed
loop, for two reasons: a fresh `ComposeOptions::default()` runs sniff-driven
discovery that would otherwise dominate the measurement, and the prebuilt graph
and the validation request must share one options identity — otherwise the
opaque-graph guard (correctly) rejects the mismatched pairing with an
`Options` mismatch. This was observed directly: an early bench draft that
re-captured context per call produced `ReferenceGraphMismatch(Options)` on
`validate_prebuilt`, which is the guard doing its job.

## Fixtures

Deterministic, defined in-source (no external corpus):

| Fixture | Shape |
|---|---|
| `small` | one document, 4 remote links, no transclusion, no disk reads |
| `large` | one document, `LARGE_LINK_COUNT = 200` remote links |
| `multi_transclusion` | root + `TRANSCLUSION_CHILD_COUNT = 12` on-disk `::file` children |

Reproducibility fingerprint — `bench-source-sha256:
7fb12746003f5e38fbffa986eb27cda87c88a0ce40209c422f96c433371ba8bf`
(`shasum -a 256 darkmatter/lib/benches/reference_graph.rs`). The fixture bodies
are pure constants / index-seeded strings in that file, so this hash pins the
exact inputs.

## Environment

- Host: Apple M4 Max, macOS (Darwin 25.5.0, arm64)
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `--release`
- Branch / commit: `darkmatter` @ `bc1c148f2`
- Criterion config for the recorded run: `--warm-up-time 1 --measurement-time 4`,
  `sample_size(30)` (30 samples per function; `multi_transclusion/construct`
  reports the standard Criterion "unable to complete 30 samples" note because a
  single iteration exceeds the target time — the 30 samples were still
  collected over the estimated window).

## Raw Criterion output (median in the middle of each `[low median high]` triple)

```text
reference_graph/small/build_and_validate
                        time:   [239.19 µs 241.21 µs 243.58 µs]
reference_graph/small/validate_prebuilt
                        time:   [34.501 µs 34.685 µs 34.900 µs]
reference_graph/small/construct
                        time:   [202.91 µs 204.78 µs 206.68 µs]
reference_graph/large/build_and_validate
                        time:   [6.3252 ms 6.3581 ms 6.3931 ms]
reference_graph/large/validate_prebuilt
                        time:   [104.64 µs 105.17 µs 105.67 µs]
reference_graph/large/construct
                        time:   [6.1953 ms 6.2283 ms 6.2657 ms]
reference_graph/multi_transclusion/build_and_validate
                        time:   [10.288 ms 10.455 ms 10.723 ms]
reference_graph/multi_transclusion/validate_prebuilt
                        time:   [4.1206 ms 4.1522 ms 4.1835 ms]
reference_graph/multi_transclusion/construct
                        time:   [6.0306 ms 6.0571 ms 6.0896 ms]
```

## Reuse win (median)

| Fixture | build+validate | validate prebuilt | speedup |
|---|---:|---:|---:|
| `small` | 241.21 µs | 34.685 µs | ~7.0× |
| `large` | 6.3581 ms | 105.17 µs | ~60× |
| `multi_transclusion` | 10.455 ms | 4.1522 ms | ~2.5× |

Prebuilt validation is materially faster on every fixture. For
`multi_transclusion` the prebuilt path still re-reads and re-hashes all 12
children from disk (descendant verification), so its floor is higher than the
single-document fixtures — but it is still ~2.5× faster than rebuilding,
confirming the Finding-18 reuse win survives the opacity cutover.

Dispersion is tight (each `[low, high]` spread is within a few percent of the
median), so the ordering is stable, not noise.

## Construction / provenance cost

Provenance construction is the only new work the opacity cutover adds to graph
construction:

- `ReferenceDocumentIdentity::capture` — three xxHash passes over the root
  (frontmatter map, body, whole-represented-state), plus one per unique visited
  local child recorded in the dependency manifest.
- `ReferenceGraphOptionsIdentity::capture` — one xxHash over the canonical
  options encoding, plus three `Weak` handle clones.

None of these retain large objects (only `u64` fingerprints and `Weak`
handles), and the per-child work is O(unique visited children) — the manifest
dedupes by resolved source, so each child is hashed at most once. The measured
`construct` times track `build_and_validate` minus validation and are dominated
by InlinePre compose + reference extraction, not the handful of extra hashes:
`small` construct 204.78 µs, `large` 6.2283 ms, `multi_transclusion` 6.0571 ms.

## Regression budget — measured cross-commit `construct` comparison

The acceptance rule (spec `spec.md:612-624`, AC13 `spec.md:678-679`) is: a
construction regression is unacceptable only when it exceeds **both** 5% **and**
100 µs at the median on a stable fixture. This is the primary evidence; the
analytical bound below is retained only as a consistency check.

### How the baseline was produced

The bench target is new to this branch, so the pre-opacity commit had no
`reference_graph` bench. To obtain a true baseline the candidate bench file was
copied **byte-for-byte** into a detached worktree at the pre-opacity commit and
registered with a matching `[[bench]]` entry. The pre-opacity public API
(`Markdown::reference_graph`, `ReferenceGraphOptions::with_compose`,
`ReferenceValidationOptions::with_graph`,
`validate_references{,_with_graph}`) is signature-identical to the candidate, so
the bench compiled and ran **unmodified — no trimming was required**; all three
functions (`build_and_validate`, `validate_prebuilt`, `construct`) ported
cleanly, and only the `construct` medians are used for this comparison.

Byte-identity of the workload across both commits was verified: the candidate
and the pre-opacity copy are `diff`-clean and share
`sha256 db628b1593fe4ffca8a35e7b946c167dd15cebe05bef4a69d15bf0ce3e110a39`.
(This supersedes the stale `bench-source-sha256` recorded above, which predated
a later edit to the bench file.)

### Run parameters (identical on both commits)

- Commits: baseline `db7e46792` (pre-opacity parent of `a8e5e98d9`;
  `provenance.rs` absent, `ReferenceGraph` still has public `root`/`nodes`
  fields) vs candidate `b425fb466` (main worktree `HEAD`).
- Host: Apple M4 Max, macOS (Darwin 25.5.0, arm64,
  `Darwin Kernel Version 25.5.0 … RELEASE_ARM64_T6041`).
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `--release`.
- Criterion: `sample_size(30)`, `--warm-up-time 1 --measurement-time 3`
  (same flags both sides; each worktree has its own `target/`, both measured in
  the same session on the same idle host).
- Filter: `-- construct` (isolates the three `construct` benches).

### Measured `construct` medians (Criterion `[low median high]`)

| Fixture | Baseline `db7e46792` | Candidate `b425fb466` | Δ median (abs) | Δ median (%) |
|---|---|---|---:|---:|
| `small` | 167.18 µs `[166.01, 168.65]` | 211.83 µs `[210.20, 213.69]` | +44.65 µs | +26.7 % |
| `large` | 6.1351 ms `[6.1019, 6.1710]` | 6.2115 ms `[6.1796, 6.2405]` | +76.4 µs | +1.25 % |
| `multi_transclusion` | 5.8717 ms `[5.8387, 5.9078]` | 6.0398 ms `[6.0088, 6.0713]` | +168.1 µs | +2.86 % |

Dispersion is tight on both sides — every `[low, high]` confidence interval is
within ~±1 % of its median — so the deltas are signal, not sampling noise.
Criterion flagged 1–4 mild/severe high outliers per bench (≤13 % of 30
samples), the routine long-tail of a live machine; medians are robust to them.
The `change:` line Criterion prints compares against each worktree's own stale
saved baseline, **not** across commits, so it is not the comparison here — the
cross-commit deltas above are computed from the two medians directly.

### Verdict against the 5 % / 100 µs budget

A regression fails **only** when it exceeds **both** thresholds at the median.

| Fixture | >+5 %? | >+100 µs? | Both? | Result |
|---|:--:|:--:|:--:|---|
| `small` | yes (+26.7 %) | no (+44.65 µs) | no | **PASS** — under the µs floor |
| `large` | no (+1.25 %) | no (+76.4 µs) | no | **PASS** |
| `multi_transclusion` | no (+2.86 %) | yes (+168.1 µs) | no | **PASS** — under the % floor |

**Overall: PASS.** No fixture trips both gates. The small fixture regresses
notably in *percentage* terms (+26.7 %) because its absolute cost is tiny, so
the fixed-cost provenance hashing is proportionally visible — but the +44.65 µs
absolute delta is well under the 100 µs floor, which is exactly the case the
two-threshold rule is designed to absorb. The two larger fixtures each trip at
most one gate. The measurement is consistent with the analytical bound above
(bounded constant-factor hashing, no large-object retention, O(unique children)
manifest work): the provenance addition is a small fixed cost, dominant only
where the total construction cost is smallest.

## Re-measurement against the FINAL implementation (review-2 Finding 2)

The cross-commit table above measured candidate `b425fb466`, which **predates**
the identity-encoder fixes (exact length-delimited `GraphIdentityEncoder::path`
replacing lossy `Path::display()`), the complete-context fingerprint, and the
volatile-context / remote-fetch graph tests that landed afterward. A
byte-identical bench source does not make those two implementations identical
(review-2, High — "The construction-regression report does not measure the
reviewed implementation"). This section re-runs all three `construct` fixtures
against the same pre-opacity baseline for the FINAL worktree.

### Final revision under test

- Base commit: `e58ceff35` (`git rev-parse HEAD`).
- Working tree: **uncommitted** — `git status --short` reports modified
  `darkmatter/lib/src/markdown/compose/context/options.rs`,
  `darkmatter/lib/src/markdown/compose/remote_fetch.rs`,
  `darkmatter/lib/src/markdown/reference/graph.rs` (plus `CLAUDE.md`). These are
  the identity-encoder, remote-fetch, and graph changes made after
  `b425fb466`, so this run measures the reviewed final implementation.
- Bench source unchanged: `shasum -a 256` of `reference_graph.rs` is still
  `db628b1593fe4ffca8a35e7b946c167dd15cebe05bef4a69d15bf0ce3e110a39` — the
  same workload as the cross-commit table, so only the library changed.

### Environment and host-state caveat

- Host: Apple M4 Max, macOS (Darwin 25.5.0, arm64).
- Toolchain: `rustc 1.96.0 (ac68faa20 2026-05-25)`, `--release`.
- Criterion: `sample_size(30)` (in-source), `--warm-up-time 1
  --measurement-time 3`. **No sample-size reduction was applied**; every fixture
  completed its 30 samples (the "unable to complete 30 samples in 3.0s" notes
  are Criterion's standard message for benches whose single iteration exceeds
  the target window — the 30 samples were still collected, as with the original
  run).
- **The host was NOT idle during this run.** Load averages ranged from
  ~6.6/21.8/27.8 at the start to ~10–17 (1-min) during the timed loops, with a
  background Virtualization VM, MacWhisper, and a busy WindowServer competing.
  The recorded pre-opacity baseline (167.18 µs small `construct`) and candidate
  `b425fb466` were both taken "in the same session on the same idle host." A
  loaded-host absolute cannot be compared apples-to-apples against that idle
  median.

Because of the load confound, the primary evidence below is a **same-session
paired run**: the pre-opacity baseline commit `db7e46792` was checked out into a
**non-destructive `git worktree`** (the same method the cross-commit table used),
the byte-identical bench was registered there, and baseline + final candidate
were run **back-to-back in this session under the same load**, so the host-state
contamination cancels in the delta.

### Primary: same-session paired `construct` (load-neutral)

| Fixture | Baseline `db7e46792` (same-session) | Candidate `e58ceff35`+worktree (same-session) | Δ median (abs) | Δ median (%) |
|---|---|---|---:|---:|
| `small` | 254.23 µs `[252.13, 256.43]` | 332.28 µs `[329.67, 334.67]` | +78.05 µs | +30.7 % |
| `large` | 6.2241 ms `[6.1932, 6.2538]` | 6.3633 ms `[6.3394, 6.3863]` | +139.2 µs | +2.24 % |
| `multi_transclusion` | 6.9256 ms `[6.8608, 6.9948]` | 7.0875 ms `[7.0268, 7.1423]` | +161.9 µs | +2.34 % |

All `construct` intervals are tight (within ~±1 % of the median) on both sides —
the gate metric is reliable even under load; only the `build_and_validate` /
`validate_prebuilt` multi-transclusion samples caught a transient load spike
(recorded below). 30 samples per function per side.

Verdict against the two-part budget (fails only when a fixture exceeds **both**
5 % **and** 100 µs at the median):

| Fixture | >+5 %? | >+100 µs? | Both? | Result |
|---|:--:|:--:|:--:|---|
| `small` | yes (+30.7 %) | no (+78.05 µs) | no | **PASS** — under the µs floor |
| `large` | no (+2.24 %) | yes (+139.2 µs) | no | **PASS** — under the % floor |
| `multi_transclusion` | no (+2.34 %) | yes (+161.9 µs) | no | **PASS** — under the % floor |

**Overall: PASS for the final implementation.** The delta structure matches the
`b425fb466` run (small trips the % gate only; large/multi trip the µs gate only).
The post-`b425fb466` identity-encoder work raised the small-fixture absolute
delta from +44.65 µs to +78.05 µs — closer to, but still comfortably under, the
100 µs floor — consistent with the exact path encoder doing slightly more
fixed-cost work than `Path::display()`. AC13's no-material-construction-regression
requirement holds for the reviewed final implementation.

### Why the naive comparison against the recorded idle baseline is invalid

Comparing the loaded-host candidate directly against the recorded **idle**
medians produces spurious failures that are host state, not code:

| Fixture | Recorded idle baseline | Candidate (loaded) | Δ abs | Δ % | Naive result |
|---|---|---:|---:|---:|---|
| `small` | 167.18 µs | 332.28 µs | +165.1 µs | +98.8 % | trips both — spurious FAIL |
| `large` | 6.1351 ms | 6.3633 ms | +228.2 µs | +3.72 % | PASS |
| `multi_transclusion` | 5.8717 ms | 7.0875 ms | +1215.8 µs | +20.7 % | trips both — spurious FAIL |

The same-session baseline is the control: **identical pre-opacity code** measured
254.23 µs (`small`) here versus 167.18 µs idle — a +52 % host-load inflation —
and 6.9256 ms (`multi`) versus 5.8717 ms idle (+18 %). That inflation, not the
opacity cutover, is what pushes the naive `small`/`multi` deltas over the gate.
Once host state is controlled by same-session pairing, all three fixtures pass.
This is precisely the confound review-2 Finding 2 flags: AC13 must be measured on
matched host state, which the same-session paired run provides and the
loaded-vs-idle comparison does not.

### Reuse win — final implementation (same-session candidate)

| Fixture | build+validate | validate prebuilt | speedup |
|---|---:|---:|---:|
| `small` | 496.77 µs `[490.11, 504.77]` | 88.815 µs `[87.319, 89.934]` | ~5.6× |
| `large` | 6.5064 ms `[6.4844, 6.5372]` | 143.98 µs `[143.00, 145.02]` | ~45× |
| `multi_transclusion` | 16.245 ms `[12.322, 20.912]` † | 7.4628 ms `[6.0404, 9.6215]` † | ~2.2× |

Prebuilt validation stays materially faster than rebuilding on every fixture, so
the Finding-18 reuse win survives the final identity implementation. The
final-impl `validate_prebuilt` is slower than the `b425fb466` numbers (e.g.
`small` 88.8 µs vs 34.7 µs) because it now performs the complete-context
provenance check and descendant re-verification on the timed path — the intended
safety cost — but the ratio remains a clear win.

† The `multi_transclusion` `build_and_validate` / `validate_prebuilt` intervals
are wide because a host-load spike hit during that fixture's candidate run
(load 1-min ~17). A separate, less-loaded candidate run earlier in this session
measured `multi_transclusion` cleanly at build+validate 11.690 ms
`[11.559, 11.804]` and validate prebuilt 4.0327 ms `[4.0026, 4.0641]` → ~2.9×,
corroborating that the reuse win is real and the ~2.2× above is a load-floor
artifact, not a regression. The `construct` medians (the AC13 gate metric) were
tight in both runs.

### Summary

- AC13 is **established for the final implementation `e58ceff35` + uncommitted
  worktree**: same-session paired `construct` deltas are +78.05 µs/+30.7 %
  (`small`), +139.2 µs/+2.24 % (`large`), +161.9 µs/+2.34 %
  (`multi_transclusion`) — **no fixture trips both gates → PASS**.
- Reuse win preserved: ~5.6× / ~45× / ~2.2–2.9× (prebuilt faster than rebuild on
  every fixture).
- The measurement was taken on a **loaded** host; the same-session paired method
  neutralizes that, but the absolute medians here are not comparable to the
  earlier idle-host runs and should not be read as such.
