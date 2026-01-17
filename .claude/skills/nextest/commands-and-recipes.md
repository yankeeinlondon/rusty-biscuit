# Commands & recipes

## Install

Recommended: prebuilt binaries (fastest), otherwise cargo install.

* Prebuilt (Linux/macOS) per nextest docs:
  
  * Linux: `curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin`
  * macOS: `curl -LsSf https://get.nexte.st/latest/mac | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin`
* Cargo install (must use `--locked`):
  
  * `cargo install --locked cargo-nextest`

## Day-to-day

* Run all tests:
  * `cargo nextest run`
* List tests (debug filters, verify discovery):
  * `cargo nextest list`
* Use a config profile:
  * `cargo nextest run --profile ci`
* Reduce concurrency (resource-constrained / DB-bound tests):
  * `cargo nextest run --test-threads 4`
  * `cargo nextest run -2` (2 fewer than CPU count)

## Targeted runs (filtersets)

* Single package:
  * `cargo nextest run 'package(my-crate)'`
* Regex-ish name selection:
  * `cargo nextest run 'test(/^auth::/)'`
* Exclude slow tests:
  * `cargo nextest run 'not test(/^slow::/)'`
* Integration but not flaky:
  * `cargo nextest run 'package(integration) and not test(/^flaky::/)'`

## CI sharding / partitioning

Run on N workers with hash partitioning:

* Worker 1 of 4:
  * `cargo nextest run --partition count:4,hash:1`
* Worker 2 of 4:
  * `cargo nextest run --partition count:4,hash:2`

## Archive and reuse (build once, run many)

* Create archive after building tests:
  * `cargo nextest archive --archive-file test-archive.tar.zst`
* Run in CI from archive (saves compile time in split pipelines):
  * `cargo nextest run --archive-file test-archive.tar.zst`

---