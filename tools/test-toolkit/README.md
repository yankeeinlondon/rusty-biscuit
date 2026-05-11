# test-toolkit

Shared test lifecycle helpers for the Rusty Biscuit workspace.

## Running tests

Use `cargo test` for standard test execution, or `cargo nextest run` for
parallel, profile-aware test execution:

```bash
# Standard test run
cargo test -p test-toolkit

# Nextest run (picks up .config/nextest.toml)
cargo nextest run -p test-toolkit
```

## Nextest configuration

The workspace `.config/nextest.toml` defines slow-test thresholds:

- **default profile**: Tests slower than `5s` are flagged as slow after 3 periods.
- **ci profile**: Tests slower than `10s` are flagged as slow after 2 periods,
  and JUnit XML is written to `test-results.xml`.

## Verification

To verify that nextest picks up the slow-timeout configuration, run the
`nextest_config_verification` integration test and check that the 6-second test
is flagged as slow:

```bash
cargo nextest run --profile default -p test-toolkit --test nextest_config_verification 2>&1 | grep -i slow
```

Or use the justfile recipe:

```bash
just verify-nextest-config
```
