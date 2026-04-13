# Testing

Claudine's default verification path is package-scoped `cargo nextest run`:

```bash
cargo --version
rustc --version
cargo nextest run -p claudine
cargo nextest run -p claudine-cli
```

Additional expectations for the Claudine test suite:

- PTY coverage stays manual-only: `cargo test -p claudine-cli --test pty_tests -- --ignored`
- Snapshot updates should be reviewed with `cargo insta review` and accepted with `cargo insta accept`
- Benchmarks are opt-in and non-gating: `cargo bench -p claudine --bench runtime_hot_paths`
- CLI integration helpers live under `claudine/cli/tests/common/mod.rs`
- Inline unit tests are preferred for private library logic such as harness, sequence, dispatch, and TUI reducers
