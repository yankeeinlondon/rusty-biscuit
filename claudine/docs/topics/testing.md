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
- PTY coverage stays manual-only: `cargo test -p claudine-cli --test pty_tests -- --ignored`
- Snapshot updates should be reviewed with `cargo insta review` and accepted with `cargo insta accept`
- Benchmarks are opt-in and non-gating: `cargo bench -p claudine --bench runtime_hot_paths`
- CLI integration helpers live under `claudine/cli/tests/common/mod.rs`
- Inline unit tests are preferred for private library logic such as harness, sequence, dispatch, and TUI reducers
