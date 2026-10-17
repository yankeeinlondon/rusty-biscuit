# Test Speed — the 5 s Budget

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

An L1 test over 5 s is a failing test even when its assertions pass: hosted
CI runners have 2–4 cores and run the whole package suite in parallel, so a
test near the limit on a development host times out there. Aim for about
2.5 s when run alone on a development host.

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

### 4. The same expensive input rebuilt for every row

When every row rebuilds an identical expensive input (a parsed catalog, a
validated document set), build it once per process with
`std::sync::LazyLock`. Share only data the rows do not modify, and key any
cache on the input's content so a changed file is recomputed.
