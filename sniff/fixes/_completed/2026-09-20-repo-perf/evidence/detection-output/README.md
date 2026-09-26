# Phase 6: complete detection output, before and after (AC5, checkout)

AC5 asks for the complete detection/topology output to be compared before and
after on this checkout. Phases 1–4 recorded candidate-level parity and
unchanged counters, but no full-output comparison, so Phase 6 added one.

## Method

- A temporary detached worktree (`/Volumes/coding/wt/rusty-biscuit/repo-perf-ac5`,
  removed afterwards) built the disposable probe
  [`ac5_dump.rs.txt`](ac5_dump.rs.txt) as an untracked release example at:
  - baseline: harness commit `43a08f94e` (serial walk);
  - after: `9d2d39c6d` (the parallel walk; its non-test lib code equals
    `f9af74815`).
- The probe runs public `detect_repo_structure(<root>)` under a fresh
  `PerformanceCollector` and prints the resulting `RepoInfo` as pretty JSON.
- Both binaries read **this** checkout, `/Volumes/coding/wt/rusty-biscuit/fix-sniff`,
  with the same root spelling as Phase 4. The runs alternated
  (after, baseline) three times, after one initial baseline run.

```sh
cargo build -q --release -p sniff --example ac5_dump   # in the temp worktree, per side
/tmp/ac5-{baseline,after} /Volumes/coding/wt/rusty-biscuit/fix-sniff > ac5-<side>-<n>.json
```

## Result: byte-identical

- Every run of both sides produced the same bytes; SHA-256 values are in
  [`sha256.txt`](sha256.txt). The output itself is
  [`detect_repo_structure-corpus.json`](detect_repo_structure-corpus.json):
  77 packages, 2 monorepo standards, 2 monorepo layers.
- Every run printed `nested_marker_walks=Some(1)` on stderr, so each request
  took the fallback walk rather than supplied evidence.

The controlled-fixture half of AC5 is covered by tests, not by this file:
`filesystem::repo::nested::tests::supplied_evidence_starts_no_fallback_walk`
(fallback and supplied evidence reach the same detector outcomes and seeds),
the parity suite in the same module, and the unchanged `detection.rs`
request-cost and reuse tests.
