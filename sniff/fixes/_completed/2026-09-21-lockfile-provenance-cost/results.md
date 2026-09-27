# Results: lockfile provenance cost

This file holds the performance half of review finding 3 ("Requested behavior
and performance have no new verification"). The functional half is recorded in
[`implementation-log.md`](implementation-log.md). Sections 4 and 5 answer
review 2's finding "The required isolated corroboration measurement is
missing".

- Spec: [`spec.md`](spec.md), section "Performance verification". Plan:
  [`plan.md`](plan.md) (R8, "Baseline timings", "After timings", "Results
  write-up").
- Evidence: [`evidence/`](evidence/). The host, tree states, binary hashes,
  harness, and run history are in
  [`evidence/environment.md`](evidence/environment.md). Raw samples are in
  `evidence/baseline/*.json` and `evidence/after/*.json`, and the generated
  table is [`evidence/summary-table.md`](evidence/summary-table.md).

## How to read these numbers

- **Scope:** one macOS host (M4 Max, 16 logical CPUs) and one corpus, this
  checkout at `/Volumes/coding/wt/rusty-biscuit/fix-sniff`.
- **Baseline:** `HEAD` `ea73a87aa` plus the uncommitted `nested.rs` change.
- **Changed:** the same state plus this fix's Findings 1 and 2. So the two
  trees differ only by this fix.
- **Cache:** all results are **warmed-cache**.
- **Protocol:** each set had 3 discarded warmup rounds, then 20 measured
  samples per case. Baseline and changed runs alternated, with a rotating order
  each round.
- **Figures:** each cell gives the median, with the min–max range of the 20
  samples in brackets.
- **Load:** the 1-minute load average is shown before → after each set.
- **Deferred sets:** none. For §§ 1–3 the load stayed at or below 9.18,
  under the 16-core threshold. For §§ 4–5 it stayed between 4.23 and 5.27,
  under that campaign's lower threshold of 12.
- **Separate effects:** the parser, detection, and compose effects are
  reported separately and must not be substituted for one another.
- **Wall clock is directional:** these are wall-clock timings on a shared
  host. The acceptance evidence for work removed is the counter tests (zero
  lockfile reads and parses when provenance is declined), recorded in the
  implementation log.

## 1. Parser: `CargoLockVersions::parse` on this checkout's `Cargo.lock`

This times the generic `toml::Value` parser (baseline) against the typed
parser (changed). Each sample covers one read and parse of the
396,787-byte lockfile, timed in-process after 3 in-process warmups.
`CargoLockVersions` is `pub(crate)`, so each tree's `parse` was copied
verbatim into a scratch example; see `environment.md` § Harness.

| Profile | Baseline (generic) | Changed (typed) | Change | Load |
|---|---|---|---|---|
| release | 2.540 ms [2.484–2.739] | 1.890 ms [1.852–1.984] | −0.650 ms (−25.6%) | 7.49 → 7.49 |
| debug | 24.526 ms [24.232–27.148] | 22.393 ms [22.065–22.889] | −2.133 ms (−8.7%) | 7.38 → 6.95 |

The two release ranges do not overlap. This is the parse alone; it is not a
detection or compose effect.

## 2. Detection: public `detect_repo_structure(<repo root>)`

Each sample is one detection call on the checkout root, timed in-process after
3 in-process warmups.

- **Baseline:** always corroborates the Cargo and pnpm lockfiles.
- **Changed, declined:** plain `detect_repo_structure`, which reads no
  lockfile.
- **Changed, requested:** `detect_repo_with_request(root,
  &RepoRequest::structure().with_lockfile_provenance(true))`.

| Profile | Baseline (always corroborates) | Changed, declined | Changed, requested | Load |
|---|---|---|---|---|
| release | 39.591 ms [35.612–48.348] | 31.601 ms [27.251–38.269] | 39.162 ms [34.135–45.405] | 7.49 → 7.68 |
| debug | 130.875 ms [128.644–140.021] | 75.015 ms [69.709–78.172] | 128.707 ms [126.577–132.049] | 6.95 → 6.09 |

### Declining provenance

Declining (the new default for the structure tier) removes the corroboration
step:

- release: −7.99 ms (−20.2%);
- debug: −55.86 ms (−42.7%).

The debug ranges do not overlap. The release ranges overlap at their edges,
but the medians are 8 ms apart.

### Requesting provenance

With provenance requested, the changed tree is within noise of the baseline:

- release: −0.43 ms (−1.1%);
- debug: −2.17 ms (−1.7%).

That difference is about the size of the parser change in § 1. Only the
Cargo lockfile uses the typed parser, and the pnpm corroboration step did not
change.

### How the saving relates to the parse

The corroboration step removed by declining (8.0 ms release, 55.9 ms debug) is
far larger than the Cargo parse alone (2.5 ms release, 24.5 ms debug). The
remainder includes the pnpm corroboration (a 287,979-byte `pnpm-lock.yaml` on
this checkout) and the match work. § 4 measures the step in isolation. The
parser speedup is not the detection speedup.

## 3. Compose: one ambient `md compose` of a trivial document

Each sample is the wall-clock time of one whole
`md compose evidence/trivial.md --output markdown` process, run with cwd set
to the checkout root. The `md` binaries were built from each tree.

The CLI's launch-repository capture calls `detect_repo_structure`. So does
the ambient-repository `Repo` capture that `AmbientRepository::discover`
performs. See `environment.md` § Harness for the code path.

| Profile | Baseline `md` | Changed `md` | Change | Load |
|---|---|---|---|---|
| release | 110.276 ms [100.093–122.723] | 91.835 ms [83.745–113.092] | −18.44 ms (−16.7%) | 9.18 → 8.85 |
| debug | 342.856 ms [335.695–411.192] | 234.017 ms [220.857–491.956] | −108.84 ms (−31.7%) | 8.85 → 8.31 |

The ranges overlap in both profiles, but only through a few samples.

- **Release:** 16 of the 20 changed samples fall below the baseline minimum
  (100.1 ms).
- **Debug:** 18 of the 20 changed samples fall below the baseline minimum
  (335.7 ms). The other two are outliers at 372 and 492 ms.

Each compose saving is close to twice the per-call detection saving in § 2
(release 2 × 8.0 = 16.0 ms, debug 2 × 55.9 = 111.7 ms). That agrees with the
two `detect_repo_structure` calls on this path. The agreement comes from the
medians; a counter trace of a compose was not taken to confirm it.

## 4. Corroboration step in isolation: `upgrade_provenance_with_lockfile`

This section answers review 2's second finding. It times only the corroboration
loop, `for layer in &mut monorepo_layers { upgrade_provenance_with_lockfile(…) }`,
on this checkout's root. The before tree has the generic Cargo parser and the
after tree has the typed one. The trees and the `nested.rs` alignment are the
same as in §§ 1–3.

- **Harness:** a scratch `#[doc(hidden)]` hook, identical in both trees, was
  appended to `detection.rs` and deleted after the build. See
  `environment.md` § "Corroboration-step harness".
    - The process runs `detect_repo_structure(root)` once, untimed, and captures
      the layers, the seeds, and a copy of the `ManifestStore` just before the
      corroboration loop.
    - Each call then gets fresh clones of the layers and seeds and a fresh store
      copy whose three lockfile caches are empty. So every timed call reads and
      parses its lockfiles, as real detection does.
    - Each process makes 3 in-process warmup calls, then times one call. The
      set protocol matches §§ 1–3.
- **Cases:**
    - `all`: every layer, which is the real step.
    - `cargo`: the Cargo layer only.
    - `pnpm`: the pnpm layer only.
- **`manifests-cached` variants:** the member `Cargo.toml` files that the Cargo
  match reads are parsed into the store copy before the timer starts.
    - At the capture point, 74 of them are not yet parsed.
    - In real structure detection, corroboration parses them first, and package
      construction then reuses them. Counters show 83 manifest parses for both
      declined and requested detection; only 2 lockfile reads and 2 lockfile
      parses differ.
    - So the `manifests-cached` figure is the step's **marginal** cost to
      detection. The plain figure also includes about 1.9 ms (release) or
      11.4 ms (debug) of manifest parsing that detection pays either way.
- **Cargo result on this checkout:** in both trees the Cargo layer reports
  `lockfile_match = false`, and the pnpm layer reports `true`.
    - Member `darkmatter/dmls/zed-dmls` (index 73 of 75) is not in `Cargo.lock`.
      The root `Cargo.toml` lists it under `exclude`, yet it appears as a layer
      member. That is a pre-existing detection behavior that this fix does not
      touch, and it should become its own fix.
    - The Cargo match exits at that member, so it walks 73 of the 75 members.

| Profile | Case | Before (generic Cargo parser) | After (typed) | Change | Load |
|---|---|---|---|---|---|
| release | all | 9.408 ms [9.013–10.648] | 8.702 ms [8.460–10.542] | −0.706 ms (−7.5%) | 4.90 → 4.66 |
| release | all, manifests cached | 7.281 ms [7.086–7.646] | 6.801 ms [6.635–7.304] | −0.480 ms (−6.6%) | 4.66 → 4.61 |
| release | cargo | 5.040 ms [4.744–5.830] | 4.495 ms [4.326–5.462] | −0.545 ms (−10.8%) | 4.61 → 4.61 |
| release | cargo, manifests cached | 3.082 ms [2.977–3.802] | 2.537 ms [2.440–2.765] | −0.545 ms (−17.7%) | 4.61 → 4.72 |
| release | pnpm (control, unchanged code) | 4.159 ms [4.027–5.027] | 4.202 ms [4.042–5.923] | +0.043 ms (+1.0%) | 4.72 → 4.72 |
| debug | all | 65.461 ms [64.821–66.829] | 63.920 ms [62.785–65.781] | −1.541 ms (−2.4%) | 4.69 → 4.23 |
| debug | all, manifests cached | 54.668 ms [53.239–55.631] | 52.510 ms [51.723–57.533] | −2.158 ms (−3.9%) | 4.23 → 4.75 |
| debug | cargo | 36.380 ms [35.534–37.238] | 34.414 ms [33.435–38.477] | −1.966 ms (−5.4%) | 4.75 → 4.95 |
| debug | cargo, manifests cached | 25.423 ms [25.030–25.646] | 23.377 ms [22.626–26.014] | −2.046 ms (−8.0%) | 4.95 → 4.87 |
| debug | pnpm (control, unchanged code) | 29.510 ms [28.870–33.037] | 29.404 ms [28.813–32.149] | −0.106 ms (−0.4%) | 4.87 → 5.12 |

How to read it:

- **The typed parser's effect on the step equals its effect on the parse.**
    - Release: the Cargo step drops by 0.545 ms, against 0.650 ms for the parse
      in § 1. With manifests cached, all 20 after samples fall below the
      before minimum.
    - Debug: 2.05 ms against 2.13 ms.
    - The whole step drops by 0.48–0.71 ms in release (−6.6% to −7.5%) and by
      1.5–2.2 ms in debug (−2.4% to −3.9%). The ranges overlap, but 15–20 of
      the 20 after samples fall below the before minimum in every `all` and
      `cargo` set.
- **The pnpm control did not move.** Its code did not change, and its medians
  are within 1% of each other.
- **Half of the step is pnpm on this checkout, even though it is
  Cargo-authoritative.** In the after tree with manifests cached, the pnpm
  layer takes 4.20 ms of 6.80 ms in release and 29.4 ms of 52.5 ms in debug.
  The Cargo parse is now the smaller part.
- **The isolated step agrees with the § 2 proxy.**
    - Release: the step with manifests cached costs 6.80 ms, and requested
      minus declined detection is 7.56 ms.
    - Debug: 52.5 ms against 53.7 ms.
    - The proxy was the right order of magnitude, but it is not the isolated
      figure.
- **None of this is a detection or compose effect.** The detection effect of
  declining provenance is in § 2, and the compose effect is in § 3.

## 5. pnpm-authoritative cost check and decision

This check covers plan spike S3 and requirement R6. The pnpm code did not
change, so only the after tree was timed. The fixtures are generated by
`evidence/pnpm-fixture.py.txt` under `/tmp` and are not committed.

- **`checkout-shape`** copies this checkout's `pnpm-workspace.yaml`,
  `pnpm-lock.yaml` (287,979 bytes; 3 importers, 2 members), and the three
  `package.json` files. It has no Cargo files.
- **`large`** has 40 members under `packages/*` and a synthetic
  `pnpm-lock.yaml` of 3,055,582 bytes (41 importers).
    - Each importer carries the frontend's dependency block.
    - The checkout's `packages:` and `snapshots:` sections are repeated 10×,
      with the keys renamed so they stay unique.
    - This stands in for a large JavaScript monorepo.

Both fixtures corroborate: the pnpm layer reports `lockfile_match = true` and
`provenance = lockfile`.

Every case runs in the same alternating set as the others for its fixture:

- **step:** the isolated `upgrade_provenance_with_lockfile` with a cold
  lockfile cache (§ 4 harness).
- **lock parse:** `ManifestStore::pnpm_lock` alone, which reads the file and
  runs the generic `serde_yaml_ng::Value` parse.
- **value probe** and **typed probe:** a scratch example that reads the file and
  either parses a `Value` or deserializes `struct { importers: BTreeMap<String,
  IgnoredAny> }`. The typed probe is the "typed `importers:`-keys-only parse"
  that R6 describes, measured only to size the possible gain. It is not a
  proposed implementation.
- **detect declined** and **detect requested:** the public
  `detect_repo_structure` and
  `RepoRequest::structure().with_lockfile_provenance(true)`.

| Profile | Fixture | Step | Lock parse | Value probe | Typed probe | Detect declined | Detect requested | Load |
|---|---|---|---|---|---|---|---|---|
| release | checkout-shape | 4.259 [4.070–4.759] | 4.295 [4.154–5.192] | 4.395 [4.278–5.332] | 3.135 [3.018–3.300] | 1.673 [1.631–3.125] | 6.097 [5.795–7.527] | 4.72 → 4.42 |
| release | large | 44.246 [42.554–45.951] | 44.825 [43.875–48.177] | 45.868 [44.953–51.236] | 33.669 [32.567–36.190] | 4.805 [4.601–6.101] | 51.758 [49.440–56.947] | 4.42 → 4.69 |
| debug | checkout-shape | 29.558 [29.123–31.725] | 29.545 [28.786–29.971] | 29.717 [29.153–30.062] | 16.439 [16.145–16.923] | 2.028 [1.924–3.542] | 32.066 [31.521–34.099] | 5.12 → 5.27 |
| debug | large | 305.189 [300.210–310.433] | 305.413 [301.699–312.240] | 308.892 [304.501–313.756] | 171.826 [169.227–174.988] | 6.669 [6.278–8.321] | 318.437 [313.784–325.126] | 5.27 → 4.27 |

All figures are in milliseconds, given as the median with the range in
brackets.

Findings:

- **The lockfile parse is the whole pnpm step.**
    - The lock-parse and step medians agree within noise, so
      `pnpm_lockfile_matches`'s set comparison costs nothing measurable.
    - Measured against R6's threshold ("`pnpm_lockfile_matches` costs at least
      10% of `upgrade_provenance_with_lockfile` time on a pnpm-authoritative
      fixture in a release build"), the share is about 100%, so the threshold is
      met.
    - On a pnpm-authoritative fixture the pnpm match is the only work in the
      step, so that threshold cannot fail there. It does not by itself show that
      a typed parser pays.
- **The pnpm parse costs about three times as much per byte as the typed Cargo
  parse.**
    - Release: 14.7–14.9 ns/byte for pnpm, 4.8 ns/byte for the typed Cargo
      parse, and 6.4 ns/byte for the old generic Cargo parse.
    - Debug: about 100 ns/byte for pnpm and 56 ns/byte for typed Cargo.
- **Corroboration dominates requested detection on a pnpm repository.**
    - Release: requested detection costs 3.6× declined detection on
      `checkout-shape` (+4.4 ms) and 10.8× on `large` (+47.0 ms).
    - The step is 70% and 85% of requested detection, respectively.
- **A typed importer-keys parse recovers only part of it.**
    - Release: the typed probe is 27–29% faster than the value probe, saving
      1.26 ms on `checkout-shape` and 12.2 ms on `large`.
    - Debug: 44–45% faster.
    - The YAML scan of the whole document remains, because a serde-typed parse
      still scans the full `packages:` and `snapshots:` sections.

**Decision: leave pnpm parsing as it is in this fix.**

This departs from the literal wording of R6, and the author should confirm it.
The reasons:

1. **Only opt-in requests pay this cost now.** Structure detection declines
   provenance by default, so the ambient Darkmatter compose and every plain
   `detect_repo_structure` read no pnpm lockfile.
    - `RepoRequest::full()`, an explicit `with_lockfile_provenance(true)`, and
      the CLI commands that opt in (including bare `sniff repo --json`) still
      pay it.
    - This rests on the provisional default that review 2 leaves to the author.
      If the author restores the old default, every structure detection of a
      pnpm repository pays it again, and this decision should be revisited.
2. **The remedy R6 names would leave most of the cost in place.** The probe
   shows that a typed serde parse keeps 71–73% of the release parse time. On
   this checkout that saves about 1.3 ms of a 39 ms requested detection (§ 2).
3. **A large gain needs a different technique.** That would be a reader that
   stops after the top-level `importers:` block, which precedes `packages:` in
   lockfile v9. It is a hand-rolled partial parser with its own correctness
   risk; the plan rejects that approach for Cargo (R5), and it falls outside
   this fix's scope.

**Recommended follow-up:** file an `_unscheduled` fix for pnpm lockfile
corroboration cost. Give it a stop-early or streaming design, and state its
evidence as this table: 44 ms release on a 3 MB lockfile, paid by full-tier
requests.

## What was not measured

- **Suite-level effect.** No test-suite wall-clock was measured. The compose
  figure is the per-compose unit that the suite pays; how many composes run on
  this corpus, as opposed to fixture repositories without lockfiles, was not
  counted. Do not multiply it into a suite claim.
- **Other hosts.** Only macOS was timed. No cross-OS timing was taken, and
  none is required.
- **Why the Cargo match costs more than the parse.** In release, the Cargo step
  with cached manifests (2.54 ms) exceeds the typed parse alone (1.89 ms,
  § 1). The remaining ~0.6 ms, which covers the lookups over 73 members, `Rc`
  allocation, and running inside a process that has just run detection, was not
  broken down.
- **A streaming or stop-early pnpm reader.** § 5 probes only a serde-typed
  parse, which still scans the whole document. A reader that stops at the end
  of `importers:` was not built or timed.
