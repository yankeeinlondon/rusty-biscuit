# Gotchas & fixes

## 1) Doctests are not supported

Symptom: doctests don’t run under nextest.
Fix:

* `cargo nextest run` (unit/integration)
* `cargo test --doc` (doctests)

## 2) Installing/building requires `--locked`

Fix:

* `cargo install --locked cargo-nextest`

## 3) Config precedence surprises

Config sources (highest priority first):

1. `.config/nextest.toml`
1. `--tool-config-file ...`
1. embedded defaults

Fix: keep critical settings in one place; when debugging, specify `--tool-config-file` explicitly.

## 4) Thread count semantics

* `cargo nextest run -2` means “CPU count minus 2”.
* `--test-threads` expects a number or `num-cpus`.

Fix: pick one style and standardize in docs/CI scripts.

## 5) Fail-fast still allows stragglers

Fix: configure termination mode:

````toml
[profile.ci]
fail-fast = { max-fail = 1, terminate = "immediate" }
````

## 6) Platform filtering: host vs target confusion

Fix: in overrides, be explicit:

````toml
platform = { host = "cfg(unix)", target = "aarch64-apple-darwin" }
````

## 7) Leaky tests (child processes not cleaned up)

Symptom: tests “finish” but runner hangs or warns about leaks.
Fix:

* enable leak timeout:
  * `leak-timeout = "200ms"`
* fix test cleanup (reap/kill child processes, drop handles).

## 8) Profiles don’t inherit unless specified

Fix:

````toml
[profile.ci]
inherits = "default"
retries = 3
````

## 9) JUnit path is relative to store dir

Symptom: JUnit file not where you expected.
Fix:

* Use absolute path, or remember it lands under store (often `target/nextest/...`).

## 10) MSRV mismatch: building vs running

* Building/installing nextest requires modern Rust (per docs, 1.89+).
* Running your project’s tests can still target a lower MSRV.

Fix: in CI, install nextest using a modern toolchain even if your project MSRV is lower.