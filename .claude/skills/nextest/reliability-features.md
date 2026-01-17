# Reliability features: flakes, timeouts, leaks, fail-fast

## Retries for flaky tests

Simple retry count:

````toml
[profile.ci]
retries = 3
````

With backoff:

````toml
[profile.ci]
retries = { backoff = "exponential", count = 5, delay = "1s", max-delay = "10s", jitter = true }
````

Good practice:

* Keep retries mostly in **CI profile**, not default local runs.
* Use overrides for known flaky groups rather than blanket high retries.

Example: flaky integration subset:

````toml
[[profile.ci.overrides]]
filter = 'package(integration) and test(/^flaky::/)'
retries = 5
````

## Slow tests and termination

Mark/act on slow tests:

````toml
[profile.ci]
slow-timeout = { period = "300s", terminate-after = 2 }
````

Per-pattern slow allowance:

````toml
[[profile.ci.overrides]]
filter = 'test(/^slow::/)'
slow-timeout = { period = "300s", terminate-after = 2 }
````

## Global timeout

Hard cap for the entire run (useful in CI to prevent infinite hangs):

````toml
[profile.ci]
global-timeout = "2m"
````

## Leaky test detection (child processes not cleaned up)

If tests spawn children and don’t reap them, enable leak detection:

````toml
[profile.ci]
leak-timeout = "200ms"
# or stricter:
# leak-timeout = { period = "500ms", result = "fail" }
````

## Fail-fast that actually stops work

Default fail-fast may allow “straggler” tests to continue. Prefer terminate mode:

````toml
[profile.ci]
fail-fast = { max-fail = 1, terminate = "immediate" }
````

Use this when:

* slow integration tests make “fail-fast” otherwise slower than expected.

---