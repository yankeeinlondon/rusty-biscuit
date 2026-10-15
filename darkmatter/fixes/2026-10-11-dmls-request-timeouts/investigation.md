---
kind: research
area: darkmatter
created: 2026-10-11
packages:
    - dmls
---

# Current-build request responsiveness investigation

The current release build did not reproduce the historical 120-second request
timeouts in the measured scenarios. It does exhibit the same class of queue
blockage: after the repository index was populated, an unrelated request waited
790–1,254 milliseconds behind one edit and 1.768–11.884 seconds behind a burst
of 20 edits. An isolated confirmation measured 5.299 seconds from sending the
first burst edit to receiving the unrelated response, including pipe backpressure.
The document request handlers themselves subsequently completed in milliseconds.
This supports fixing responsiveness without claiming to have identified the
historical incident's exact cause.

## Authority and conditions

- Source revision: `2965a7c078707547492414f2dadba46e1aaf0ffb`.
- Build: `cargo build --locked -p dmls --release`, successful in 2m09s.
  No production source modifications were present. The specification and an
  unrelated review prompt had existing uncommitted edits; the specification's
  measured text was therefore newer than that revision.
- Host observations used the installed Sniff CLI's focused `os`, `cpu`, and
  `memory` JSON commands. Sanitized characteristics: macOS 27.2, arm64,
  Apple M4 Max, 16 physical/logical cores, 128 GiB total memory, approximately
  89.8 GiB reported available, approximately 24.6 GiB used swap. The host was
  not dedicated to benchmarking; CPU load and thermal state were not recorded.
- Every server scenario had a 150-second process watchdog. No server scenario
  exceeded it. The investigation stayed well below the two-hour authorization.
- The current binary ran from repository build output, using standard-input
  and standard-output protocol framing. Nothing installed or replaced the
  user's binary; no editor, browser, or terminal window was launched or controlled.
- Server stderr was drained and discarded after extracting startup completion
  time and document count. No document bodies, full logs, environment values,
  credentials, hostnames, or personal absolute paths are retained in evidence.

## Protocol method

Each process initialized with reliable client file watching, opened a tiny
unrelated untitled buffer, and opened the selected existing document. It then
ran open, single-edit, 20-edit burst, and request-then-cancel phases. Edits were
full-text replacements differing by trailing newlines, with monotonically
increasing versions. The burst sent edits as quickly as the pipe permitted;
it was not a measured sustained typing frequency.

Each phase first queried folding ranges on the unrelated buffer, then queried
highlights, code actions, full semantic tokens, folding ranges, document links,
hover, and completion on the selected document. Positions were at the start
of the document; these are basic protocol probes, not exhaustive feature or
schema-completion workloads. Ordinary phases were repeated three times per
method, except populated-workspace scenarios, which used one sample per
method. Requests were serial, not an arbitrary concurrent client workload.

Response latency uses Python's monotonic `perf_counter`, starting immediately
before framing/writing a client request and ending after the reader thread
decodes its matching response. It includes client pipe, server queuing, handler,
and decode costs; it does not distinguish server receipt time. The latest
probe also records time spent sending edits and elapsed time from the first
edit to the unrelated response, to expose pipe backpressure. Earlier runs
did not record that additional clock origin.

All measured requests returned without protocol error, including requests
immediately followed by cancellation. That outcome is legal for requests
already finishing and does not measure cancellation receipt-to-stop latency.
The protocol has no acknowledgement exposing cancellation receipt or work
stopping. Instrumentation is required to measure those intervals reliably.

## Observations

| Scenario | Selected document bytes | Indexed documents | Unrelated delay after one edit | Unrelated delay after 20-edit burst |
| --- | ---: | ---: | ---: | ---: |
| Existing mapping-only corpus, `feature.md` | 1,852 | 23 | 11.929 ms | 360.142 ms |
| Existing sequence-descent corpus, `implement-plan.md` | 12,202 | 1 | 28.895 ms | 204.060 ms |
| Repository root, specification, startup overlapping requests | 12,242 | 5,308 after startup | 60.612 ms | 1,140.585 ms |
| Repository root, specification, requests after startup adoption | 12,242 | 5,308 | 790.167 ms | 11,884.443 ms |
| Repository root, specification, isolated confirmation after startup adoption | 12,242 | 5,308 | 1,254.421 ms | 1,767.516 ms |

In the populated-workspace run, the seven selected-document handlers took
2.659–9.402 milliseconds after the edit burst drained. Peak child-process RSS
was 800,014,336 bytes (approximately 763 MiB). This is a process-lifetime high
water mark, not a measurement of queue allocation or steady-state memory.
The isolated confirmation reached 903,020,544 bytes (approximately 861 MiB).
It spent 3.532 seconds sending the 20 edits, and the response arrived 5.299
seconds after the first edit was sent. Counting only the 1.768-second request
latency would hide work already happening while the client writes were blocked.
Isolated startup adoption took 39.309 seconds. These two post-startup runs have
only one sample per method per phase and show substantial variability; they
establish queue blockage rather than a precise performance distribution.

The first three scenarios used three samples per method per phase. Their
phase-wide medians mix different methods and are available in the raw metrics,
but are not a useful percentile guarantee. The populated-workspace row uses
one sample per method per phase; no population percentile can be inferred.

The first populated-workspace startup overlapped a separate index benchmark.
Its 41.436-second startup time is therefore not an isolated startup result.
The benchmark lasted 35.173 seconds, discovered 5,308 Markdown documents,
produced 136,314 graph nodes and 156,825 edges, and reported approximately
353 MiB peak RSS. Discovery took 234 ms and graph assembly 2.084 seconds.
Markdown parsing accumulated 505.258 seconds across parallel workers; these
overlapping worker durations must not be added to wall-clock stages. The
benchmark does not execute diagnostics and reports zero for that stage.

## Source-established risks

These facts explain plausible present delays; they do not establish the old
installed binary's behavior or causation:

- In `dmls/src/router.rs`, the protocol router performs request and
  notification dispatch synchronously. Opening or changing one buffer
  synchronously indexes it and refreshes diagnostics for every open buffer.
  An unrelated request queued after those notifications must wait.
- `dmls/src/graph/invalidate.rs` reparses changed text and rebuilds the entire
  workspace graph for every changed document. Startup adoption also builds
  the graph while holding the shared index lock.
- `dmls/src/diagnostics/scheduler.rs` carries the configured debounce interval
  but currently publishes immediately; the setting does not coalesce analysis.
- In `dmls/src/router.rs`, cancellation is handled in the same synchronous
  loop and checked only before request dispatch. Work already executing has
  no cooperative cancellation checkpoint through this router.
- `dmls/src/workspace/startup.rs` limits worker count to available CPU
  parallelism and file count, but queues every discovered file into an
  unbounded channel. Result channels are also unbounded. Discovery has no
  explicit file-count, byte-count, or elapsed-time cap.
- `dmls/src/workspace/discover.rs` honors ignore rules and does not follow
  symlinks. Its configured include/exclude matches filter files rather than
  introducing a general traversal budget; a huge non-ignored tree can still
  be walked. Local reads and filesystem resolution can be uninterruptible.
- `dmls/src/overlay/mod.rs` scans schema triggers when constructing document
  analysis. No timing breakdown in this investigation isolates that cost.

Source inspection found passive parser/validator calls rather than composition
or shell execution in the investigated paths. Existing
`dmls/tests/l1/no_side_effects.rs` provides stronger sentinel-based coverage;
that suite was not run here. This investigation did not monitor network
traffic, filesystem writes, credential reads, or subprocess creation, and
cannot claim independent proof that every passive-analysis boundary holds.

## Proposals requiring human approval

The measured handlers are cheap enough to support proposing a maximum
250-millisecond response target for highlights, code actions, hover, and
completion, and a maximum one-second target for full semantic tokens, folding,
and links, including when unrelated analysis is paused. These are proposed
future guarantees under an approved reference workload, not current guarantees.

The directly observed workload is two open buffers, a selected document up to
12,242 bytes, up to 5,308 indexed documents, and up to 20 successive full-text
edits. A larger supported document/open-buffer/workspace envelope needs explicit
human selection and targeted validation; these measurements alone cannot
justify a large-file or sustained-edit guarantee. The exact current corpus
should be recorded with its build revision when used as a reference workload.

For work bounds, latest-only pending document analysis and a bounded shared
workspace rebuild are more directly connected to the observed failure than an
arbitrary memory ceiling. No queue-count, worker-count, retained-byte, or
receipt-to-stop number is established by this probe. Such numbers need a
human ruling and validation using internal work counters/checkpoint events.

A small implementation-time test seam to pause one analysis operation and
observe queued/running work would prove isolation, obsolete-result suppression,
and stopping behavior deterministically. The current source has no such seam;
the baseline did not introduce artificial pauses or change production code.
No separate architectural prototype is necessary to establish the measured
queue problem. A prototype may still be useful if the planned concurrency
design cannot explain bounded ownership and safe stopping from code review.

## Reproduction and retained evidence

Run from the repository root with Python 3 and a freshly built local binary:

```sh
cargo build --locked -p dmls --release
python3 darkmatter/fixes/2026-10-11-dmls-request-timeouts/probe.py \
  target/release/dmls darkmatter/dmls/tests/fixtures/mapping_only_corpus \
  darkmatter/dmls/tests/fixtures/mapping_only_corpus/feature.md
python3 darkmatter/fixes/2026-10-11-dmls-request-timeouts/probe.py \
  target/release/dmls . \
  darkmatter/fixes/2026-10-11-dmls-request-timeouts/spec.md \
  --rounds 1 --wait-startup
```

On Windows use the corresponding `.exe` binary path. No non-macOS runtime
evidence was gathered. The probe is an investigation artifact, not a new
automated test or a CI scheduling request. The script accepts paths only as
command-line inputs; its output does not retain them.

`measurements.json` preserves sanitized per-request metrics and the index
benchmark. `probe.py` contains the bounded headless reproduction client.
Neither file contains document text. The temporary raw numeric outputs under
the ignored build-output directory are not needed to reproduce the findings.

Unmeasured behavior includes save/rescan clients, watched-file notifications,
configuration reload, multiple workspace roots, cross-file refresh/recovery in
real editors, large or malformed document families, shared-work cancellation,
sustained overload and memory growth, exact queue counts, and Windows/Linux/WSL2
runtime performance. Historical editor logs and hang traces were not read.
