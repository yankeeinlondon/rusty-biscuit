# Worker diagnostics (after side, `HEAD` `2f4eb5264`)

Both diagnostics were temporary `#[ignore]`d tests added to the measurement
worktree's copy of `nested.rs` (`/Volumes/coding/wt/rusty-biscuit/repo-perf-measure`,
detached at `2f4eb5264`), run once, and reverted with `git checkout`. They use
the private R1 seam `walk_for_nested_markers_with_threads` and the Phase 3 R2
test counters. None of this code exists in the fix branch.

## Observed worker count on the corpus

Corpus: `/Volumes/coding/wt/rusty-biscuit/fix-sniff`, 12,908 admitted entries.
Default worker policy (`threads = None`, `ignore` 0.4.25
`available_parallelism().min(12)` = 12 on this 16-CPU host).

```sh
PHASE4_CORPUS=/Volumes/coding/wt/rusty-biscuit/fix-sniff \
  cargo test -p sniff [--release] --lib phase4_scratch -- --ignored --nocapture
```

| profile | runs | admitted entries (R2 counter) | serial reference entries | distinct worker threads |
|---|---:|---:|---:|---:|
| debug (test profile) | 5 | 12,908 every run | 12,908 | 12 every run |
| release | 5 | 12,908 every run | 12,908 | 12 every run |

Every one of the 12 default workers visited entries on every run, and no
visitor's work went missing from the collector.

## Walk latency by worker count (the tiny-tree floor)

Release test profile, `walk_for_nested_markers_with_threads(tree, threads)`
called directly (the fallback walk alone, no detection). Tiny tree: the same
8-file layout the probe uses. 20 samples per row; a tiny sample is 200
iterations, a corpus sample 10; iterations/4 warm-up. Host load (1-minute)
was 24.8 at the start and 26.1 at the end.

| tree | threads | median µs | min µs | max µs |
|---|---|---:|---:|---:|
| tiny | `Some(1)` | 265.1 | 261.6 | 273.3 |
| tiny | `Some(2)` | 1,582.0 | 1,553.9 | 1,609.3 |
| tiny | `Some(4)` | 1,695.6 | 1,645.5 | 1,780.9 |
| tiny | `Some(8)` | 2,179.2 | 2,049.9 | 2,420.6 |
| tiny | `None` (12) | 3,014.4 | 2,831.1 | 3,314.1 |
| corpus | `Some(1)` | 64,247.8 | 62,482.4 | 65,024.0 |
| corpus | `Some(2)` | 38,197.5 | 36,761.2 | 40,798.6 |
| corpus | `Some(4)` | 23,083.3 | 21,831.8 | 25,039.6 |
| corpus | `Some(8)` | 20,466.6 | 19,649.9 | 21,165.6 |
| corpus | `None` (12) | 19,282.2 | 18,512.3 | 19,777.0 |

## Mechanism

`ignore` 0.4.25 `Worker::get_work` (`src/walk.rs:1807-1852`): a worker that
finds no work deactivates and then polls its queue with
`std::thread::sleep(Duration::from_millis(1))` until work or a quit message
arrives. The walk ends only after every worker has gone idle, and each
`run` spawns its workers fresh in a `std::thread::scope` (`walk.rs:1410`). So
any walk with two or more workers pays thread spawn/join plus at least one
1 ms sleep cycle, whatever the tree's size. With one worker, the lone worker
deactivates to zero at once and quits without sleeping, so `Some(1)` costs
what the serial walk costs.
