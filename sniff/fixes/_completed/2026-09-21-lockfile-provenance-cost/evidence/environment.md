# Performance environment

Captured on 2026-09-26 for the baseline and after timings of this fix. The
protocol follows `2026-09-20-repo-perf`. The harness is different: a scratch
example with one timed call per process, not Criterion (see "Harness").

## Host and toolchain

| Field | Value |
|---|---|
| Host | Ken's development Mac, Apple M4 Max (`Mac16,5`) |
| OS | macOS 27.2 (build 26B5091g), `aarch64-apple-darwin` |
| Cores | 16 logical |
| Memory | 128 GiB |
| Toolchain | `rustc 1.98.1 (48a229cea 2026-09-01)`; `cargo 1.98.1 (797e8a9bc 2026-08-05)` |
| Build wrapper | `rustc-wrapper = "kache"` (`~/.cargo/config.toml`). It affects compile caching only, never a timed region |
| Profiles | debug = Cargo `dev`; release = Cargo `release` |
| Features | `sniff` default features for the scratch example; `darkmatter-cli` default features for `md` |
| Cache treatment | **warmed cache**. Every set has 3 discarded warmup rounds, each process makes 3 in-process warmup calls before its timed call, and the corpus had been walked many times before the sets began. No cold-cache claim is made |

## Measured tree states

Both trees were built with a separate `CARGO_TARGET_DIR`
(`/tmp/lpc-target-baseline`, `/tmp/lpc-target-changed`), so they share no
artifacts.

| Tree | State |
|---|---|
| Baseline | Detached worktree `/tmp/lpc-baseline` at `HEAD` `ea73a87aa86051f782c7610f9c0197e91bdb0e55`, plus the other stream's uncommitted `sniff/lib/src/filesystem/repo/nested.rs` worker-cap change, applied from `git diff HEAD -- …/nested.rs` (patch sha256 `8d41071d…ef1f9d`; the baseline tree's whole `git diff HEAD` has the same sha256, so nothing else differed). The worktree was removed after the builds (`git worktree remove --force`) |
| Changed | This worktree, `/Volumes/coding/wt/rusty-biscuit/fix-sniff`, at the same `HEAD`, with this fix's uncommitted Findings 1 and 2 (request gate, read counter, typed Cargo parser) and the same `nested.rs` change. `git diff HEAD -- sniff/lib darkmatter claudine` had sha256 `f49fa115…d775` both before and after the builds, so no concurrent edit reached the measured sources |

So the two trees differ only by this fix.

## Corpus

- Root spelling: `/Volumes/coding/wt/rusty-biscuit/fix-sniff`, a linked Git
  worktree (`.git` is a file). Every side of every set was run against this same
  root, so the workload is fixed and only the binary changes.
- `Cargo.lock`: 396,787 bytes, sha256
  `296ac0a008bd0c54409972580f569a8b13bda15927c39bd69a139fc268ec5b53` (unchanged
  from `2026-09-20-repo-perf`).
- `pnpm-lock.yaml` (287,979 bytes) and `pnpm-workspace.yaml` are also present at
  the root, so the baseline's structure detection corroborates a pnpm layer as
  well as the Cargo layer.
- The compose document is `evidence/trivial.md`: a heading and one sentence,
  with no frontmatter and no `ctx.*` reference.

## Binaries

sha256 of every measured binary (also in `binaries.sha256`):

| Binary | sha256 |
|---|---|
| `baseline-debug-lpc_bench` | `69db23340493c822ce7dcf30b3b4c4555befe4481c9f6c5fc6dd8fbe5e4c0433` |
| `baseline-release-lpc_bench` | `6ee48e9016d79dfb4b305f8e0da3e1da67601dc31fc8f4120be2a232241f9a91` |
| `changed-debug-lpc_bench` | `66f3b79f3305c56bad41fbcaec7c030d07335b7ab482a4181aa67448c1d523a2` |
| `changed-release-lpc_bench` | `6f9b43e93f0aa1ea008aab8f07225d817c4ff62f74f2f41307565d539294cf0c` |
| `baseline-debug-md` | `be41ecfe4e476cd4ef5f889d29e97aef29d9ab7f22e825ee11100b8ffdbfbb9d` |
| `baseline-release-md` | `a244463818cee7a210573c342fee8787e041bbe7fd28d786c2eb42b45b891b73` |
| `changed-debug-md` | `594083101bb6fb6618f3a9c0a737db71a0cc7b0818a019e13e260428a15db1bf` |
| `changed-release-md` | `613edf844c639166f29ec1192fd24be6276bb2d02434c9aa8d733d7e22b5f2f2` |

Every sample JSON also records its binary's sha256.

## Harness

- `lpc_bench` was a scratch Cargo example, `sniff/lib/examples/lpc_bench.rs`. It
  was assembled from `lpc_bench-common.rs.txt` plus that tree's
  `lpc_bench-parser.rs.txt`. It was copied into each tree only for the build and
  deleted immediately afterwards, so neither tree kept a new source file.
    - Usage: `lpc_bench <case> <root> <warmups>`. It makes `<warmups>` calls,
      then times one call and prints the elapsed nanoseconds.
    - `parse`: `CargoLockVersions` is `pub(crate)` and cannot be reached from an
      example, so each tree's `CargoLockVersions::parse` (file read, counter
      calls, and TOML parse) was **copied verbatim by `sed`** from that tree's
      `manifest_index.rs` into the example. The copies are the two
      `lpc_bench-parser.rs.txt` files. The copied lines are unedited. Only the
      hand-written `use` header differs from the source, importing
      `sniff::performance` in place of `crate::performance`. The counter calls are the public
      `sniff::performance::increment_counter`, and with no collector installed
      they are no-ops.
    - `detect-structure`: public `detect_repo_structure(root)`.
    - `detect-provenance` (changed tree only):
      `detect_repo_with_request(root, &RepoRequest::structure().with_lockfile_provenance(true))`.
      The baseline has no such option. Its `detect_repo_structure` always
      corroborates, so it is the baseline for both detection cases.
- `md compose`: `md compose sniff/fixes/2026-09-21-lockfile-provenance-cost/evidence/trivial.md --output markdown`,
  run with cwd set to the checkout root, stdin/stdout/stderr set to
  `/dev/null`, and wall-clock time taken around the whole process. The time
  covers the ambient-repository capture path.
    - `darkmatter/cli/src/commands/compose.rs` runs `detect_repo_structure` on
      the launch repository.
    - `AmbientRepository::discover` (`darkmatter/lib/.../context/current.rs`)
      captures the `Repo` group, and that capture calls `detect_repo_structure`
      in `capture/snapshot.rs`.
- `measure.py.txt` is the driver.
    - Each set runs 3 discarded warmup rounds, then 20 measured rounds.
    - Each round runs every side once. The order rotates by one each round, so
      baseline and changed runs alternate and neither always leads.
    - The 1-minute load (`sysctl -n vm.loadavg`) is recorded before and after
      each set.
    - A set whose start load is above 16 waits, polling every 60 s for up to
      15 minutes, and is marked deferred if the load stays above 16.
- `summarize.py.txt` produces `summary-table.md` from the JSON files in
  `baseline/` and `after/`.

## Run history

`measure.log` records three invocations in order:

1. **Discarded.** A driver bug gave the two compose sides the same label, so
   their samples merged into one 40-sample list. The parse and detection sets
   from this run agreed with run 2 to within a few percent (release parse
   2.57 → 1.92 ms; release detect 41.4 / 33.5 / 40.5 ms). Run 2 overwrote their
   files.
2. **Kept for parse and detection.** An unfiltered rerun of all six sets. Its
   parse and detection JSON is the evidence here. Its compose sets had the same
   label bug and were deleted.
3. **Kept for compose.** The compose sets alone, with distinct labels.

No set was deferred. The 1-minute load stayed between 6.09 and 9.18 at every
set boundary of the kept runs.

## Corroboration-step campaign (review 2, `results.md` §§ 4–5)

This campaign was captured on 2026-09-26 on the same host and toolchain as the
earlier sets. Its purpose was to time `upgrade_provenance_with_lockfile` in
isolation and to run the pnpm-authoritative cost check.

### Trees

- **Baseline:** a fresh detached worktree, `/tmp/lpc-baseline`, at `HEAD`
  `ea73a87aa`.
    - The `nested.rs` patch was applied again (patch sha256
      `8d41071d…ef1f9d`, the same as before). The tree's `git diff HEAD` had that
      same sha256 before the harness was added.
    - The worktree was removed with `git worktree remove --force` after the
      build.
- **Changed:** this worktree.
    - `git diff HEAD -- sniff/lib/src` had sha256
      `8d3dec4d73e51073913094c96504c2de7e593aa0fb0ea89ac3981dbc65fe9a7d` before
      the harness was added, after it was removed, and at the end of the work.
    - `detection.rs` was restored from a copy taken before the harness was
      added.
- **Target directories:** `/tmp/lpc-target-baseline` and
  `/tmp/lpc-target-changed`, rebuilt for this campaign.

### Corroboration-step harness

- **The hook.** It is recorded in `lpc_corrob-detection.patch.txt`, which is
  the baseline tree's diff and is byte-identical to the text added in the
  changed tree.
    - One line, `lpc_capture(&monorepo_layers, &seeds, &manifests);`, was
      inserted immediately before the corroboration loop. In the changed tree
      that is before the `wants_lockfile_provenance()` gate.
    - A thread-local capture and a `#[doc(hidden)] pub fn __lpc_bench` were
      appended to `detection.rs`.
    - The capture runs only while `__lpc_bench` arms it. It asserts that no
      lockfile is cached yet, then copies the layers, the seeds, and every
      non-lockfile map of the `ManifestStore`.
- **Each timed call.**
    - It gets fresh clones of the selected layers and the seeds, plus a fresh
      store copy with empty Cargo, pnpm, and uv lockfile caches.
    - Only the loop over `upgrade_provenance_with_lockfile`, or
      `ManifestStore::pnpm_lock` for `pnpm-parse`, is inside the timer.
    - `*-manifests-cached` cases parse the Cargo layer's member manifests into
      the copy before the timer starts.
- **`lpc_corrob.rs.txt`** is the scratch example that calls the hook.
- **`lpc_pnpm_probe.rs.txt`** is a scratch example that only reads the lockfile
  and runs one of two parses: a generic `serde_yaml_ng::Value` parse, or a typed
  parse of the importer keys alone. It does not use library code.
- **`lpc_bench`** was rebuilt in the changed tree from
  `lpc_bench-common.rs.txt` and `after/lpc_bench-parser.rs.txt`. It supplies
  the fixture detection cases.
- **Cleanup.** All scratch sources were deleted immediately after the builds.
- **Binary hashes** are in `binaries-corroboration.sha256` and in every sample
  JSON.
- **Counter check.** A throwaway collector example, also deleted, was run on
  this checkout's root in release.
    - Declined and requested structure detection both record 83 manifest
      parses. Requested detection adds 2 lockfile reads and 2 lockfile parses.
    - An isolated `corroborate` call with a cold member-manifest cache adds 74
      manifest parses. That is why `results.md` § 4 reports the
      `manifests-cached` variant as the step's marginal cost.

### Fixtures

`pnpm-fixture.py.txt` generated both fixtures under `/tmp/lpc/corrob/fixtures`.
They were not committed.

| Fixture | `pnpm-lock.yaml` bytes | sha256 |
|---|---|---|
| `checkout-shape` | 287,979 | `b66a4ca51a734d2f841cafc2e107bcc08d8a300db5fb2a3ffa9ebef0d0ca0ced` |
| `large` | 3,055,582 | `9e024514b38297b4372844fb7becca76d4d39611be33989ad033bef60cafb092` |

### Driver and runs

- **Driver:** `measure-corrob.py.txt`.
    - The protocol is the same as `measure.py`: 3 discarded warmup rounds and 20
      measured rounds per set, with the side order rotating each round and 3
      in-process warmups per process.
    - The 1-minute load is recorded before and after each set.
    - The load limit is 12 on this 16-core host. Above it, the driver waits for
      up to 15 minutes and then marks the set deferred.
    - Each JSON also records the process's stderr, which holds the layers'
      `lockfile_match` and `provenance` after the timed call.
- **Runs:** `measure-corrob.log` records one invocation, and all 14 of its sets
  are kept.
    - Before it, a single smoke run of `release-corroborate-pnpm` was discarded.
    - No set was deferred. The 1-minute load stayed between 4.23 and 5.27 at
      every set boundary.
