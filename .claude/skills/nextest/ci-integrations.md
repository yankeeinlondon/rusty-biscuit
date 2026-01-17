# CI & tooling integrations

## JUnit XML reporting

In config:

````toml
[profile.ci.junit]
path = "junit.xml"
report-name = "nextest-run"
store-success-output = false
store-failure-output = true
````

Run:

* `cargo nextest run --profile ci`

Gotcha: JUnit `path` is often relative to the store dir (see gotchas).

## Coverage: cargo-llvm-cov + nextest

Install:

* `cargo install cargo-nextest cargo-llvm-cov`

Run coverage with nextest as backend:

* `cargo llvm-cov nextest --html`

This handles nextest’s process-per-test model and aggregates coverage correctly.

## Mutation testing: cargo-mutants

Run with nextest:

* `cargo mutants --test-tool nextest --jobs 8`

Use when you need repeated full-suite runs; nextest speed helps a lot.

## Snapshot testing: insta

Nextest works well with insta output isolation:

* `cargo nextest run`
* Update snapshots:
  * `INSTA_UPDATE=all cargo nextest run`

## Archiving for CI pipelines

When builds and test execution happen in different jobs:

* Build+archive job:
  * `cargo nextest archive --archive-file test-archive.tar.zst`
* Test job:
  * `cargo nextest run --archive-file test-archive.tar.zst`

---