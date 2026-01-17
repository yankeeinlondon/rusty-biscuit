# Configuration & profiles (`.config/nextest.toml`)

## Minimal starter config (local + CI)

Create `.config/nextest.toml`:

````toml
nextest-version = "0.9.50"

[store]
dir = "target/nextest"

[profile.default]
default-filter = "all()"
test-threads = "num-cpus"
retries = 0
status-level = "pass"
failure-output = "immediate"
fail-fast = true
slow-timeout = { period = "60s", on-timeout = "fail" }

[profile.ci]
inherits = "default"
retries = 3
fail-fast = false
status-level = "all"
failure-output = "immediate-final"
global-timeout = "2m"

[profile.ci.junit]
path = "junit.xml"
````

Notes:

* `inherits = "default"` is important; profiles do **not** implicitly inherit.
* JUnit `path` is commonly **relative to the store directory** (see gotchas).

## Per-package overrides

Use overrides to tailor retries/timeouts/threads for specific packages or patterns:

````toml
[profile.default]

[[profile.default.overrides]]
filter = 'package(my-core)'
retries = 2
slow-timeout = "120s"

[[profile.default.overrides]]
filter = 'package(my-db)'
slow-timeout = { period = "300s", terminate-after = 1 }
threads-required = "num-cpus" # effectively serializes on a full-machine resource budget
````

## Tool-config-file vs repo config

If a user wants to avoid committing config, they can supply:

* `cargo nextest run --tool-config-file path/to/nextest.toml`

But remember config precedence can confuse outcomes—prefer a single committed `.config/nextest.toml` for important settings.

---