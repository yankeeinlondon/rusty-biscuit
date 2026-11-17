# Test Speed — the 5 s Budget

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

An L1 test over 5 s is a failing test even when its assertions pass: hosted
CI runners have 2–4 cores and run the whole package suite in parallel, so a
test near the limit on a development host times out there. Aim for about
2.5 s when run alone on a development host.

## Under load: the 10 s allowance

A development host is often busy with other work, and a full package run then
stretches every test. Nextest still marks anything over 5 s `SLOW` and the
slow-test log records it, so read a loaded run this way:

| Duration in a loaded run | Verdict |
| --- | --- |
| 5 s or less | passes |
| over 5 s, under 10 s | re-measure alone; it fails only if it is over 5 s there |
| 10 s or longer | fails, whatever it takes alone |

A host is under load when its load average is well above its core count
(`uptime`), or when other heavy jobs are visibly running. A test that reaches
10 s competes so heavily for a shared resource that it will time out on CI.

A test that asserts a wall-clock bound must survive the same load. Bound the
quantity the code under test controls tightly (for example, the wrapper's own
measurement of a child's lifetime), and give any whole-process wall-clock
backstop no less than 10 s, or it fails on a busy host.

These are not fixes, because the test stays slow wherever it runs: a `slow_`
or other tier prefix, a `slow-timeout` override in `.config/nextest.toml`,
`#[ignore]` or a `cfg` that skips the slow OS, or removing cases or
assertions.

## Measuring

Read the duration in nextest's `PASS [ 1.234s]` line. Measure the test alone
(`-E 'test(/^module::/)'`) and inside its full package run; a large gap means
it competes for a shared resource, usually CPU or process spawns.

## Causes, in the order to check them

### 1. Repository state captured from the process directory

Nextest runs a test with its package directory as the current directory,
which is inside this monorepo, so anything that captures runtime context or
discovers Git state from the current directory walks the whole repository.
Typical sources are `ComposeContext::capture_for_dir(&std::env::current_dir()?)`
and darkmatter's `RequestSnapshot::from_process()`. Unless the current
directory is the subject, anchor the capture in the test's temporary
directory:

```rust
let dir = tempfile::tempdir().unwrap();
let request = ComposeRequest::prepare(options, &RequestSnapshot::new(dir.path())).unwrap();
let context = ComposeContext::capture_for_dir(dir.path());
```

### 2. A loop that grows: spawns or rows in one test

A test that loops over a table runs every row serially in one process, and
gets slower each time the table grows. Spawning a binary per row makes this
worse. Give nextest the rows to schedule instead:

- Stamp one `#[test]` per row, or per bounded group of rows, with a
  `macro_rules!` macro. Size groups against the fixed per-process setup
  (grammar or query compilation) each one repeats.
- Add a guard test that every table row lands in exactly one test or group,
  and, for groups, that none exceeds a size bound; when the bound trips, add a
  group. Put "some row exercises X" assertions on one bounded group holding
  the witnesses.
- Keep each repository-file literal (`include_str!` and the like) inside
  every `#[test]`; moved into a helper it schedules the whole binary in CI
  ([ci-execution.md](ci-execution.md)).
- Do not fan rows out over threads inside one test (`std::thread::scope`):
  that multiplies against nextest's own parallelism and oversubscribes a
  2-core runner.

Examples: `listed_id_tests!` with its guard `every_warning_id_has_a_listed_test`
in `claudine/cli/tests/prompts/prompt_guide_defects.rs` (one test per row);
tree-hugger's `declaration_fact_group_tests!` and `shape_tests!` in
`tree-hugger/lib/tests/l1/` (row groups and one test per shape).

### 3. Waiting out a production deadline

A test of "after the deadline, the outcome is X" waits the full production
deadline. Pass a short deadline through the internal entry point the
production constructor calls, assert evidence that the deadline branch fired
(typically its error text) so a slow runner cannot reach the same outcome
through another branch, and keep one separate test that the production
constructor wires in the real deadline. Example: the `DeliveryDeadlines`
tests in `claudine/cli/src/commands/wrap/exec/codex_app_server/tests.rs`.

A size cap is the same case: a "past the cap, the result is truncated" test
that writes the production cap's worth of files pays for every one. Sniff's
10,000-file inventory cap made two tests take 6–10 s on native Windows;
injecting a cap of 48 through the walk's internal entry point made them under
0.1 s.

### 4. The same expensive input rebuilt for every row

When every row rebuilds an identical expensive input (a parsed catalog, a
validated document set), build it once per process with
`std::sync::LazyLock`. Share only data the rows do not modify, and key any
cache on the input's content so a changed file is recomputed.

Under nextest every test is its own process, so a `LazyLock` shares nothing
between tests. When many tests build the same starting fixture, build it once
on disk beside the test binary, keyed by the binary (a rebuild gets a new
key), rename it into place from a temporary directory so racing processes are
safe, and copy it per test. Example: `template()` in
`worktree/cli/tests/remove.rs` (fixtures about 0.65 s → 0.2 s each on Windows).

### 5. A process per question on Windows

A process spawn costs ~1–5 ms on macOS and Linux and ~25–47 ms on native
Windows, so a test that starts `git` (or anything) per fact it builds or reads
can take 5–19 s there and under 4 s elsewhere. Count the spawns first (the
`worktree` skill's `fixtures.md` has a shim that ranks a whole suite), then
cut them at the source: answer repeated reads in-process in the product, with
a parity test against the command each replaces, and build fixtures in bulk
(one `git fast-import`, one `update-ref --stdin`, batched self-checks).
Measured on `build-win-native`: `fork_origin_cutoff_matrix` 7.2 s → 1.0 s.

### 6. Fast alone, 2–3× slower in the full run

A test at 1.5 s alone and 3.5–5 s in the full run is usually not waiting: it
is one of many process-heavy tests in a binary that saturate the host
together. Measure CPU, not only wall time: a Job Object around one test (CPU
about equal to wall means CPU-bound) and a host CPU sampler during the full
run. On `build-win-native` Defender alone took a quarter of the CPU (the `os`
skill's `windows.md`). Capping product threads did not help there; less work
per test did (fewer processes per run, templated fixtures, a matrix row per
test): `worktree-cli`'s slowest test 3.97 s → 3.46 s, total 607 s → 548 s.

### 7. A system service that serves one process at a time

A test at about 1 s alone and 6–9 s in the full run, alongside others at the
same duration, may be queuing on a host daemon rather than on CPU. Run the
test binary 1, 3, and 6 times at once: wall time that grows with the count
while user CPU stays low is serialization. On macOS, CoreAudio setup behaves
this way (the `os` skill's `macos.md`). Narrow the request of every test that
does not assert what the service returns; a live network lookup the test
never reads is the same case. Sniff's slowest such test went 8.7 s → 2.8 s.

### 8. Shipped schemas prepared from text

Simplified Schema, Darkmatter, and Claudine load their compiled-in schemas
from checked-in artifacts instead of preparing them per process
(`simplified-schema/docs/topics/shipped-artifacts.md`). Measured debug
medians on an M4 Max under load (old and new binaries run side by side):
`md compose` of a three-line document went 290 → 65 ms, and
`claudine compose --goose --dry-run` of the frozen clarify prompt
1450 → 839 ms. Two consequences for tests:

- **Do not shadow a shipped schema to save time.** Copying a reduced schema
  over a shipped trigger (for example `inline-compose.yaml` as
  `schemas/claudine.yaml`) used to skip preparing Claudine's large bundle.
  That preparation is now precompiled. A shadow still avoids compiling the
  full schema's validator, but that saving has not been measured, and the
  shadow stops the test checking the schemas users get. Shadow only when
  shadowing is what the test checks.
- **A stale artifact is slow, not wrong.** A refused artifact or section
  falls back to preparing from text with the same verdicts. Claudine's
  bundle takes about 630 ms to prepare that way in debug, against about
  180 ms to decode its prepared section. If a schema-heavy suite slows
  after a shipped YAML edit, run `just shipped-schemas`; the
  `shipped_artifact_is_fresh` tests fail until you do. Under the
  `work-counters` feature, `ShippedRuntimePreparation` and the
  `ArtifactFallback*` counters show a fallback deterministically.

What remains per process is not precompiled: compiling each effective
schema into a validator (once per layered identity, and per document folder
when the schema checks files), resolving a document's own `$schema`, and
merging its layers. In a Time Profiler sample of that Claudine run (debug),
the validator compile was about 17% of the process (about 150 ms) and
decoding the prepared bundle section about 15%.

### 9. Every process-spawning test 3–10× slower on a loaded host

When tests spread across a whole family each take about 1 s alone and 6–15 s
in a full run on a host loaded far above its core count, no single test is
waiting: each process is CPU-bound in unoptimized code, and the host's load
divides the CPU among them. Splitting tests or cutting fixture spawns trims
the edges; what fixes the family is less CPU per process. Profile one process
without root (`xcrun xctrace record --template 'Time Profiler' --launch --
<binary> <args>`, then `xcrun xctrace export --xpath` the `time-profile`
table) and attribute samples to the nearest workspace crate:

- **Generic code compiled at opt-level 0.** `core`, `hashbrown`, and `alloc`
  frames monomorphized into a workspace crate run at that crate's
  opt-level. Simplified Schema's preparation was about 90% of a refused
  `claudine sequence` and of a schema-matrix cell. The workspace
  `Cargo.toml` sets `opt-level = 3` for it (and its YAML parser, and the SVG
  stack `GitGraph::plan` uses) under `[profile.test.package.*]`: 0.43 → 0.15 s
  of CPU per `claudine sequence`, 1.6 → 0.6 s per matrix cell, 1.7 → 0.7 s
  per graph-layout test. It costs a few seconds when that crate is rebuilt.
  `opt-level = 1` gained under a quarter of that; measure before settling.
- **Work repeated per item.** A walk that recomputes a pure per-schema-node
  answer for every instance value, or finds earlier entries by a linear
  scan, grows with the document: Darkmatter's operand walker took 3.2 s of
  CPU on a 200-item lifecycle document until it memoized each node's markers
  and indexed sites by path (1.0 s).

### 10. Every test of a binary slower by the same few seconds

When a binary's tests take about 0.4 s alone but share one floor in a run
(every one about 3.4 s), they are waiting on one shared first step, not
competing for CPU. Time the phases inside one test (`eprintln!` of
`Instant::elapsed`, run with `--success-output final`). In repo-deps' archive
tests the floor was the per-run `cargo build` of the shipped `ci-build`: a
no-op that waited about 3 s on `target/debug`'s artifact lock ("Blocking
waiting for file lock on artifact directory"; `lsof` on
`target/debug/.cargo-artifact-lock` found only the host's kache daemon with
it open; the wait was intermittent), with every other test queued behind it. The Nextest setup script already
built the binary, so it now exports the path through `$NEXTEST_ENV`
(`_ci_build_for_tests`) and no test calls Cargo. A test that only inspects a
produced artifact reads the run's shared one rather than compiling its own
(`shared_fixture` in `scripts/ci-build-archive-tests.rs`): the four that built
privately took 6-7 s together and 0.3 s each now.

### 11. Tests that each run a real compile, fast alone and slow together

A test that compiles a fixture workspace starts a Cargo that uses every core.
Eleven such repo-deps tests (0.5-4.2 s alone) started together at the end of
the package run and the last took 5.6-9.3 s on a quiet 16-core host. Remove
redundant builds first (feature attribution's `wrapper_in` now reads the
setup script's `ci-build` too). Then put only the compiling tests in a bounded
Nextest test group (`repo-deps-fixture-compiles`, `max-threads = 4`, in
`.config/nextest.toml`). A test waiting for a group slot has not started, so
its time is not counted, and the run's wall-clock stayed about 10.8 s. Keep
cheap inspections out of the group, and add a new fixture-compiling test to
the group's override filter.
