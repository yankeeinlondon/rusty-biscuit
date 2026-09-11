# Testing

Claudine's default verification path is the package-area `just` recipes, which run package-scoped nextest when `cargo-nextest` is installed and fall back to `cargo test` otherwise:

```bash
cargo --version
rustc --version
just test
just lint
```

The root nextest config marks tests as slow after 5 seconds in the default profile and after 10 seconds in CI. CI also writes JUnit output to `test-results.xml`.

Additional expectations for the Claudine test suite:

- New and modified tests should prefer `#[rstest]` when fixtures or parameterized cases make the setup clearer; do not bulk-migrate unrelated tests.
- Tests that mutate process-global state, especially environment variables, should stack `#[serial_test::serial]` directly on the test.
- Environment setup/teardown should use `test_toolkit::EnvGuard` instead of local guard types. `EnvGuard::set` and `EnvGuard::remove` are unsafe and require the test to serialize environment access.
- Use `test_toolkit::trace_phase!` only for meaningful setup, body, or teardown boundaries where a tracing span helps diagnose hangs or fixture failures.
- PTY coverage stays manual-only and lives under the L2 filter set: `cargo test -p claudine-cli --test level2_pty_tests -- --ignored` (or `just test-l2`)
- Snapshot updates should be reviewed with `cargo insta review` and accepted with `cargo insta accept`
- Benchmarks are opt-in and non-gating: `cargo bench -p claudine --bench runtime_hot_paths`
- CLI integration helpers live under `claudine/cli/tests/common/mod.rs`
- Inline unit tests are preferred for private library logic such as harness, sequence, dispatch, and TUI reducers

## Silent audio fixtures

Use clear speech such as “This is a test message.” and explicit zero volume for
real playback. Pin test providers and voices; these tests do not validate the
operator's personal Claudine speech configuration. A fake agent does not disable
lifecycle TTS or sound effects in a real prompt template.

For composition/provenance tests that do not test audio, set `PLAYA_DRY_RUN=1`
on the child command and assert its private `PLAYA_SPOOL_DIR` is never created.
This preserves normal lifecycle evaluation while suppressing audio publication.
Publication tests retain real handoff with the workspace-shared
`test_toolkit::LockedAudioSpool` fixture: it holds `worker.lock` for its whole
lifetime and takes `queue.lock` before scanning and removing pending jobs, so a
publisher or preparation helper cannot commit behind the scan. Destruction runs
the same cleanup while unwinding, and releases `worker.lock` while still holding
`queue.lock` — the order `run_scheduler_with` uses — so no publisher can commit
between the final scan and the release of worker ownership. A cleanup failure is
reported on stderr and retains worker ownership instead of exposing what it
could not remove; it also panics unless the thread is already panicking.
Tests that execute helpers must release blocked fixtures and observe completion
before removing their spool. Never use the operator's real queue for test cleanup.
