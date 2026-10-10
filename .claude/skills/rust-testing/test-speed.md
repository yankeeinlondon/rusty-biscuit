# Test Speed — the 5 s Budget

An L1 test that takes more than 5 s is a failing test, even when its
assertions pass. Hosted CI runners have 2–4 cores and run a package's whole
suite in parallel. A test that takes 5 s alone on a 12-core development host
reaches the termination ceiling there, and every slow test lengthens the run
that gates a merge. Aim for about 2.5 s or less when the test runs alone on a
development host.

These do not count as fixes, because the test is still slow wherever it runs:

- a `slow_` prefix or another tier marker that only moves the test out of L1;
- a `slow-timeout` override in `.config/nextest.toml`;
- `#[ignore]`, or `#[cfg]` that skips the test on the OS where it is slow;
- removing cases or assertions.

## Measuring

Nextest prints each test's duration in its `PASS [ 1.234s]` line. Measure the
test alone and inside its full package run. A large gap between the two means
the test competes for a shared resource, usually CPU or process spawns.

```sh
cargo nextest run -p claudine-cli -F claudine-cli/test-fixtures -E 'test(/^compose_caller_file_provenance::/)'
```

## Causes Found in This Repository

Each pattern below has made a test in this repository at least 5 s slower.
Check them in this order.

### 1. Repository state captured from the process directory

Nextest runs each test with its package directory as the current directory.
That directory is inside this monorepo. Any call that captures runtime context
or discovers Git state from the current directory walks the whole repository.
On this repository that costs 0.4–6 s per call, more on Windows. The usual
sources are:

- claudine: `ComposeContext::capture_for_dir(&std::env::current_dir()?)`;
- darkmatter: the `test_request(options)` unit-test helper, whose request
  directory falls back to the process directory.

If the test is not about the current directory, anchor the capture in the
test's own temporary directory:

```rust
let dir = tempfile::tempdir().unwrap();
let request = ComposeRequest::prepare(options, &RequestSnapshot::new(dir.path())).unwrap();
let context = ComposeContext::capture_for_dir(dir.path());
```

Measured results: 15.5 s → 0.75 s (darkmatter compose cleanup) and
6.8 s → 0.6 s (claudine sequence preflight).

### 2. A matrix of process spawns in one test

Spawning `claudine` or `md` costs about 50–300 ms, and more on Windows. A test
that loops over every option, spelling, and preceding word spawns hundreds of
processes one after another. Generate one `#[test]` per row with a small
`macro_rules!`, so nextest can spread the rows over the runner's cores:

```rust
macro_rules! listed_id_tests {
    ($($test:ident => $id:literal),* $(,)?) => {
        $(#[test] fn $test() { assert_listed_id_hides_only_its_warning($id); })*
    };
}
```

Splitting a loop into named tests means a row added to the source table later
gets no test. Add a guard test that compares the names listed in the macro
with the table:

```rust
#[test]
fn every_warning_id_has_a_listed_test() {
    let ids: Vec<&str> = WARNINGS.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["F1", "F2", /* … */ "D2"]);
}
```

Do not spread the rows over threads inside one test (`std::thread::scope`
fan-out). Nextest already schedules tests by the runner's core count. Threads
inside tests multiply against it and oversubscribe a 2-core runner, which
turns a slow test into a timeout.

When you split a test, keep each repository-file literal (`include_str!` and
the like) inside every `#[test]` function. Moved into a helper, the literal
makes CI run the whole test binary whenever that file changes (rule 7 in
[SKILL.md](SKILL.md)).

### 3. Waiting out a production deadline

A test of "after the deadline, the outcome is X" that uses the production
deadline waits the full time; one claudine test spent 10 s this way. Pass the
code a short deadline through the internal entry point the production
constructor calls. Then assert the evidence that the deadline branch fired,
typically its error text:

```rust
let cancellation = Instant::now() + Duration::from_secs(1); // ~500× the fake's 2 ms poll
let report = executor::deliver(&session, request, "fixture", DeliveryDeadlines { cancellation: Some(cancellation), acceptance: cancellation + Duration::from_secs(1) }).await;
assert!(report.error.as_deref().is_some_and(|e| e.contains("cancellation was not confirmed before the deadline")));
```

Without that assertion, a runner too slow to meet the short deadline can pass
through a different branch that produces the same outcome. Keep a separate
test that checks the production constructor wires in the real deadline.

### 4. The same expensive work repeated in every row

If every row of a matrix rebuilds an identical expensive input (a parsed
catalog or a validated document set), compute it once per test process with
`std::sync::LazyLock` and share it across rows. Share only data the rows do
not modify, and key any cache on the input's content, so a changed file is
always recomputed.
