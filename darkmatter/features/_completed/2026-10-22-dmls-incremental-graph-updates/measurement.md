# Live rebuild cost after the optimization

Measured 2026-10-24. Everything below is measured unless marked as an
estimate. The baseline and the step-1 prototype are the spec's
"Measurement and decision record" (2026-10-23); this record does not repeat
them.

## Setup

- **Host:** Apple M4 Max (16 cores, 128 GiB), macOS 27.2.
- **Revision:** `183a542da` (it contains the Phase 1 and Phase 2 changes). The
  working tree also held this change's uncommitted Phase 3 tests and other
  sessions' uncommitted `darkmatter/lib` edits. Neither is part of the `dmls`
  binary's graph code.
- **Build:** `cargo build --release -p dmls`.
- **Temporary seam, removed after the sample:**
  - `WorkspaceIndex::rebuild` took `#[track_caller]` to attribute each rebuild
    to the method that asked for it. A rebuild deferred inside a batch was
    attributed to the batch's one rebuild, keeping all its causes.
  - Each rebuild printed its wall time and its thread CPU time
    (`libc::clock_gettime(CLOCK_THREAD_CPUTIME_ID)`) to stderr, plus four
    stages: universe reuse or build, table reuse or build, content assembly
    with reference resolution, and reverse-index finalization.
  - A direct `libc` dependency was added for the seam only. It, the seam, and
    the lockfile line were removed before the final gates, and `dmls`
    sources match the revision again.
- **Driver:** `measure-live-rebuild.py` in this directory, run as:

  ```sh
  python3 -I darkmatter/features/2026-10-22-dmls-incremental-graph-updates/measure-live-rebuild.py \
    run --dmls target/release/dmls --root . --out /tmp/dmls-measure \
    --source darkmatter/dmls/src/graph/invalidate.rs
  ```

- **Session.** The driver is a framed stdio LSP client that acknowledges
  dynamic watched-file registration and progress creation. It waits for the
  startup listing to end and the server to go quiet for 3 s, then times a
  100.0 s sample:
  - it opens `AGENTS.md`, `darkmatter/README.md`, and
    `darkmatter/docs/lsp/indexing.md`;
  - it types 250 single characters across them at six per second, with
    pauses;
  - it sends one hover and one completion;
  - it sends two save notifications, each with a matching watched-file change
    for the real, unchanged disk file;
  - a driver-owned Markdown file joins the list and later leaves it;
  - 50 driver-owned files under `target/` arrive in one notification;
  - it closes one buffer.

  It logs only times, methods, versions, and statuses, never document text.
- **Comparability.** The spec's original driver was not kept, so this one is a
  reconstruction from its description, not a byte-identical replay.
  - The open documents, edit count, typing rate, and event mix match the
    description.
  - The edit positions, pause lengths, and request positions are this
    driver's own.
  - `AGENTS.md` is a symlink outside the file list, so opening it adds one
    open-unlisted candidate. This rebuild is the third universe built below.

## Counts

- **Documents:** 5,910 listed Markdown files; 5,911–5,912 graph documents
  (the open, unlisted `AGENTS.md` and the joined file).
- **Graph size:** 5,929–6,353 nodes and 36–664 edges.
- **Not recorded:** the seam did not print parsed or dependency counts. Two
  lazy target parses (`ensure_parsed`) happened during the sample.
- **Excluded:** startup published 2 snapshots, which are not counted below.
- **Requests:** none timed out.

## Results, 100.0 s sample

| Cause | Rebuilds | CPU median | CPU total | Wall median |
| --- | --- | --- | --- | --- |
| open-buffer edit (`set_document`) | 259 | 2.60 ms | 0.700 s | 2.61 ms |
| context relink (`relink`) | 5 | 4.76 ms | 0.022 s | 43.6 ms |
| re-resolution (`reresolve`) | 2 | 6.73 ms | 0.014 s | 7.03 ms |
| lazy target parse (`ensure_parsed`) | 2 | 2.53 ms | 0.005 s | 2.53 ms |
| listed join (batched `disk_changed`) | 1 | 18.9 ms | 0.019 s | 19.2 ms |
| listed leave (batched `disk_deleted`) | 1 | 18.6 ms | 0.019 s | 19.0 ms |
| buffer close (`close_buffer`) | 1 | 2.09 ms | 0.002 s | 2.09 ms |
| **all** | **271** | **2.61 ms** | **0.780 s** | **2.61 ms** |

- **CPU share:** rebuilding used 0.780 CPU-seconds in 100.0 s, **0.78% of one
  core**. The read of the thread CPU clock never failed.
- **Server CPU:** the server used 8.65 CPU-seconds over the sample, from
  `ps` CPU time.
- **Wall time:** median 2.61 ms, p90 2.96 ms, maximum 52.6 ms. One rebuild
  exceeded 50 ms, a `relink` whose CPU time was under 5 ms, so its thread was
  waiting, not working.
- **Build output:** the 50 creations under `target/` caused no rebuild.

### Where the time goes

| Stage | Count | CPU median | CPU total |
| --- | --- | --- | --- |
| universe reused (inputs compared) | 268 | 1.00 ms | 282 ms |
| universe built | 3 | 14.4 ms | 43 ms |
| table reused (`DocumentTable::matches`) | 268 | 0.26 ms | 71 ms |
| table built | 3 | 3.21 ms | 10 ms |
| content assembly and resolution | 271 | 1.31 ms | 366 ms |
| reverse-index finalization | 271 | 0.03 ms | 7 ms |

The universe and table were built three times each: the open, unlisted
`AGENTS.md` joining the candidates, the listed join, and the listed leave.
Every other rebuild reused both.

## Comparison

| | Baseline | Step-1 prototype | This change |
| --- | --- | --- | --- |
| Rebuild CPU share of one core | 13.6% | 4.4% | **0.78%** |
| Median rebuild CPU | 42 ms | 12.35 ms | **2.61 ms** |
| Median rebuild wall time | 52 ms | 14.3 ms | **2.61 ms** |
| Server CPU over the sample | 20.6 s | 12.6 s | **8.65 s** |

- **Bounds.** Both of the spec's aids are now met: rebuild CPU is under 1% of
  one core, and the median rebuild wall time is under 50 ms. They are aids,
  not gates, and the sample is one run on one macOS host.
- **Largest remaining cost.** Comparing the cached universe's inputs is now
  the largest single stage, about 36% of rebuild CPU. It clones and compares
  the roughly 5,900 candidate paths on every rebuild. The plan does not
  authorize further optimization, so this is recorded and not acted on.
