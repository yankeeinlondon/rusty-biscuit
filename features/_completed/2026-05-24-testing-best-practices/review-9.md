---
ready: false
agent: codex
model: ""
---

# Review #9 — Testing Best Practices

## Findings

### High — The new broker/attach path has no behavioral test coverage

Files:

- `/Volumes/coding/personal/rusty-biscuit/just/devops.just:267`
- `/Volumes/coding/personal/rusty-biscuit/biscuit-test-harness/src/bin/broker.rs:127`
- `/Volumes/coding/personal/rusty-biscuit/biscuit-test-harness/src/shared.rs:17`
- `/Volumes/coding/personal/rusty-biscuit/biscuit-test-harness/tests/shared_atexit.rs:1`

Review #8's blocker was the mismatch between process-local `SharedHarness` and nextest's process-per-test model. The implementation now introduces `biscuit-harness-broker`, `BISCUIT_SHARED_*` env vars, backend `attach`/`shared_or_spawn` helpers, and a rewritten `_test_l2` recipe that pre-spawns shared terminal resources before nextest. That is the right shape, but the behavior is currently only documented and compile-checked.

The existing `biscuit-test-harness` integration test still covers only same-process atexit cleanup: it proves a static `SharedHarness<T>` drops its inner value at process exit. It does not cover the new production-critical flow:

- broker `spawn <backend>` prints a usable id and leaks the owned harness;
- `_test_l2` exports that id through the expected `BISCUIT_SHARED_*` variable;
- two separate child processes attach to the same resource instead of spawning two resources;
- attach-mode `Drop` is a no-op;
- broker `kill <backend> <id>` tears down the resource.

This matters because the feature's prior production blocker was specifically about the canonical `just test-l2` workflow. A `cargo check -p biscuit-test-harness --bins --tests` pass and zero broker unit tests do not verify that workflow. Under the review rubric, the strongest verification for "canonical `test-l2` amortizes real-terminal spawn cost via broker" is compile-only plus unrelated Level-1 tests, which is below the required level for this user-observable command behavior.

Suggested fix: add at least one L1 test around env-var attach semantics and broker CLI argument behavior, then add an env-gated Level-2 test using the most portable backend (`tmux`) that proves two isolated subprocesses attach to the same broker-spawned session and that `broker kill tmux <id>` removes it. That test should be named `level2_...`, use `require_level!(Level::L2, TmuxHarness::available(), "tmux")`, and hard-fail when `BISCUIT_TEST_LEVEL_REQUIRED=2` is set.

## Verification-Level Audit

| Requirement | Strongest verification observed | Verdict |
| --- | --- | --- |
| `biscuit-harness-broker` compiles as a bin target | Compile: `cargo check -p biscuit-test-harness --bins --tests` passed | Pass |
| Existing same-process `SharedHarness` atexit cleanup still works | Level 1: `cargo test -p biscuit-test-harness --tests` passed | Pass |
| New backend `shared_or_spawn` env-var attach path works | Not directly tested | Gap |
| Canonical `_test_l2` pre-spawns one shared backend resource and nextest child processes attach to it | Not tested at Level 2 | Gap |
| Broker teardown removes shared resources after the suite | Not tested | Gap |

## Notes

I did not find a compile regression in the harness crate. The remaining issue is test rigor for the new behavior that closes the previous review's blocker.

Verification run:

- `cargo check -p biscuit-test-harness --bins --tests --color=never`
- `cargo test -p biscuit-test-harness --tests --color=never`
- `just --list`
