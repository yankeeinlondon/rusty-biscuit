# Phase 4: work-counter comparison

Work counters were compared separately from timing, using the `work_counts`
example. Each side (baseline `43a08f94e`, after `2f4eb5264`) was built in the
fixed-path measurement worktree with `--features bench-internals`, in release
and debug. Every case ran once under a fresh `PerformanceCollector`. Binary
hashes are in [`../after/binaries.sha256`](../after/binaries.sha256).

```sh
export SNIFF_WORK_COUNTS_CORPUS=/Volumes/coding/wt/rusty-biscuit/fix-sniff
<bin>/{baseline,after}-{release,debug}-work_counts > work_counts-<side>-<profile>.md
```

## Result: every counter is unchanged

- The four files differ only in their `elapsed:` lines, which are
  directional and uncompared. The check was
  `diff <(grep -v '^elapsed' a) <(grep -v '^elapsed' b)`, which is empty for
  baseline against after in each profile, and for release against debug.
- They are also identical, apart from `elapsed:`, to the Phase 1 snapshot
  `../baseline/work_counts-release.md`.

The corpus case (`repo_structure_corpus`, `detect_repo_structure(<repo root>)`)
records the same values on both sides:

| counter | baseline | after |
|---|---:|---:|
| `filesystem.io.bytes_read` | 828598 | 828598 |
| `filesystem.io.canonicalizations` | 393 | 393 |
| `filesystem.io.file_opens` | 85 | 85 |
| `filesystem.io.metadata_probes` | 369 | 369 |
| `filesystem.io.read_dirs` | 2 | 2 |
| `filesystem.repo.lockfile_parses` | 2 | 2 |
| `filesystem.repo.manifest_parses` | 83 | 83 |
| `filesystem.repo.nested_marker_walks` | 1 | 1 |

What the unchanged counters establish:

- **Same logical work.** Fallback walks, manifest reads and parses, and
  metadata probes are unchanged, as the spec expects. The gain comes from
  running the same traversal concurrently and retaining fewer paths, not
  from doing fewer logical walks.
- **No disappearing worker counts.** The walk's work is recorded at the
  caller chokepoint (`read_dirs`, `nested_marker_walks`), and those values
  still match.
- **Worker work is accounted.** Per-visitor propagation is verified
  separately: on the corpus, the R2 test counter reached exactly 12,908
  admitted entries from 12 workers on every run
  ([`../after/worker-diagnostics.md`](../after/worker-diagnostics.md)).
- **The timing probes measured the fallback.** The probe's `counters` mode
  showed `nested_marker_walks = 1` with identical counters on both sides,
  for the tiny tree and for the corpus. The timing probes therefore measured
  the fallback path.
