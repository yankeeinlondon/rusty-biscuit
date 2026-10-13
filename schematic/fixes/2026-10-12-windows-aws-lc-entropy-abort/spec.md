---
status: draft-spec
created: 2026-10-12
reviewed: false
review_iterations: 0
clarified: false
implemented: false
human_review: true
area: schematic
packages:
    - schematic-runtime
related:
    - 2026-10-12-schematic-runtime-envelope
human_review_items:
    - |-
        **Choose the repair.** The abort is in aws-lc's CPU-jitter entropy
        setup, not in reqwest or rustls, and every crate in the workspace that
        uses reqwest with rustls links the same aws-lc. Each repair below
        changes something wider than `schematic-runtime`; see "Repair options".
---

# Native Windows: aws-lc's entropy setup aborts the process under parallel load

## Summary

On native Windows, a process that makes its first TLS handshake while the
machine is under heavy parallel load is sometimes killed with `0xc0000409`
(`STATUS_STACK_BUFFER_OVERRUN`, subcode 7 `FAST_FAIL_FATAL_APP_EXIT`, which is
how the Microsoft C runtime reports `abort()`). Stdout and stderr show nothing.

The four loopback TLS tests in `schematic/runtime/tests/l1/builder.rs` hit it
on most parallel runs of the runtime suite, so they are ignored on native
Windows with a reason naming this fix:

- `an_added_pem_root_lets_the_client_reach_a_server_it_signed`
- `an_added_der_root_lets_the_client_reach_a_server_it_signed`
- `without_the_added_root_the_handshake_fails_as_a_tls_error`
- `accepting_invalid_certificates_reaches_a_self_signed_server_that_the_default_rejects`

macOS, Linux, and WSL2 run them.

## Cause

aws-lc (through `aws-lc-sys` 0.40.0, the provider behind reqwest 0.13's
`rustls` feature and the tests' `tokio-rustls` server) seeds its random number
generator from CPU timing jitter. The first random bytes a process asks for
initialize a process-global seed generator once
(`tree_jitter_initialize` → `CRYPTO_once` → `tree_jitter_initialize_once`).
That step calls `abort()` when the jitter collector cannot be created or when
three attempts to read entropy all fail its health tests
(`crypto/fipsmodule/rand/entropy/tree_drbg_jitter_entropy.c`). Under heavy
parallel CPU load the jitter measurements fail, and the process dies.

In a TLS client the first random bytes are the ClientHello's session ID, so
the abort surfaces inside a send.

Faulting stack, captured with `cdbX64.exe` on `build-win-native`
(debug build, frames from the bottom of the stack trimmed):

```text
(1f0c.29bc): Security check failure or stack buffer overrun - code c0000409 (!!! second chance !!!)
Subcode: 0x7 FAST_FAIL_FATAL_APP_EXIT
ucrtbase!abort
l1!aws_lc_0_40_0_set_thread_and_global_tree_drbg_reseed_counter_FOR_TESTING   <- static fn, nearest export
l1!aws_lc_0_40_0_CRYPTO_set_thread_local                                      <- static fn, nearest export
ntdll!RtlRunOnceExecuteOnce
KERNELBASE!InitOnceExecuteOnce
l1!aws_lc_0_40_0_CRYPTO_once
l1!aws_lc_0_40_0_tree_jitter_initialize
l1!aws_lc_0_40_0_get_entropy_source
l1!aws_lc_0_40_0_set_thread_and_global_tree_drbg_reseed_counter_FOR_TESTING   <- static fn, nearest export
l1!aws_lc_0_40_0_set_thread_and_global_tree_drbg_reseed_counter_FOR_TESTING   <- static fn, nearest export
l1!aws_lc_0_40_0_RAND_bytes
l1!aws_lc_rs::rand::fill
l1!rustls::crypto::aws_lc_rs::AwsLcRs::fill
l1!rustls::msgs::handshake::SessionId::random
l1!rustls::client::hs::ClientHelloInput::new
l1!rustls::client::client_conn::connection::ClientConnection::new_with_alpn
l1!tokio_rustls::client::TlsConnector::connect
l1!hyper_rustls::connector::HttpsConnector::call
l1!reqwest::connect::ConnectorService::connect_with_maybe_proxy
...
l1!schematic_runtime::client::dispatch
l1!l1::builder::send_item
l1!l1::builder::an_added_der_root_lets_the_client_reach_a_server_it_signed
```

A second capture (`accepting_invalid_certificates_…`) has the same frames from
`abort` to `RAND_bytes`.

## Evidence

All runs on `build-win-native` (12 logical processors), from PowerShell.
"Suite" is the runtime's Level 1 suite except the `compile_fail` test, run by
nextest; every test is its own process, so every TLS test process initializes
aws-lc.

| Run | TLS configuration | Runs of the four TLS tests | Aborts |
| --- | --- | --- | --- |
| suite, `just cross-check schematic-runtime --os windows` | reqwest built-in rustls path | 4 | 1 |
| suite, `--stress-count 25` | reqwest built-in rustls path | 100 | 25 (19 of 25 iterations failed) |
| suite, `--stress-count 2` | runtime-built rustls `ClientConfig` (`tls_backend_preconfigured`) | 8 | 3 |
| 12 parallel `cdbX64.exe` loops, one TLS test per process | runtime-built rustls `ClientConfig` | 49 | 2 (both stacks above) |
| suite, `just cross-check schematic-runtime --os windows` | runtime-built rustls `ClientConfig` | 4 | 0 |
| suite, `--stress-count 250`, aws-lc built with `AWS_LC_SYS_NO_JITTER_ENTROPY=1` | reqwest built-in rustls path | 1,000 | 0 |

Every abort was in one of the four TLS tests; the other 163 tests in each
iteration never aborted. Non-TLS tests do not ask aws-lc for random bytes.

The last row is the evidence for repair option 1. Its 250 iterations also had
4 ordinary failures among the 40,750 non-TLS test runs (a loopback mock
refusing a connection, and one deadline assertion), none an abort; those are
separate load flakes, not this defect.

### Earlier conclusions this corrects

The first investigation attributed the abort to reqwest's built-in rustls
client path. That was wrong:

- Handing reqwest a finished rustls configuration does not help (rows 3–4).
  Its earlier 0 aborts in 160 runs were a small sample.
- "Not the platform verifier, since clients that accept invalid certificates
  also aborted" was right for the wrong reason: the test that accepts invalid
  certificates first sends with a default client. Neither the verifier nor the
  handshake outcome matters; the first random bytes do.
- "It also aborts with a `ring` server" holds because the client still uses
  aws-lc.

## Exposure

Any native-Windows process in this workspace that uses reqwest with rustls
links the same aws-lc and seeds it the same way, so a CLI started on a
saturated machine can abort on its first HTTPS request. It has been observed
only under test-suite load; no production report exists. Upgrading
`aws-lc-sys` does not help: 0.45.0 has the same seeding code.

Possibly the same defect, not verified: `sniff`'s library tests aborted with
`0xc0000409` under load on the same host in September 2026, in a varying test
each run (recorded in `2026-09-07-faster-sniff-tests`). `sniff` links aws-lc
through reqwest. A captured stack there would confirm it.

## Repair options

1. **Build aws-lc without CPU jitter** (`AWS_LC_SYS_NO_JITTER_ENTROPY=1`, set
   for every build: the repository's `.cargo/config.toml` `[env]` and CI).
   aws-lc then seeds from the operating system's generator (and the CPU's
   `RDRAND` for extra input). Repo-wide: it changes the entropy source of
   every binary that links aws-lc, on every OS, and forces one aws-lc
   rebuild. The run in the last evidence row tests this option.
2. **Use the `ring` provider in `schematic-runtime`** (and in its tests' TLS
   server). Removes aws-lc from the runtime's handshakes only; every other
   reqwest user in the workspace keeps the exposure, and the runtime's crypto
   provider changes.
3. **Report upstream and wait.** Ask aws-lc not to `abort()` when jitter
   health tests fail under load (fall back to the OS generator, or retry with
   a delay). The tests stay ignored on native Windows until a fixed
   `aws-lc-sys` is locked.

## Acceptance

- The four tests above run on native Windows with their `ignore` attributes
  removed.
- A parallel run of the runtime suite on native Windows with on the order of
  1,000 executions of those four tests has no abort, recorded with the exact
  command, run count, and abort count.
- The `os` skill's `windows.md` entry is updated with the outcome.
