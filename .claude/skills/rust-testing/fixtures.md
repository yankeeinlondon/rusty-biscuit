# Fixtures, Env Guards, and Pending Contracts

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## Fixtures and Env Guards

Use `test_toolkit::EnvGuard` for process-env setup/teardown and
`#[serial_test::serial]` when mutating global state:

```rust
use test_toolkit::{trace_phase, EnvGuard};
use rstest::{fixture, rstest};

#[fixture]
fn dry_run() -> EnvGuard {
    EnvGuard::set_safe("PLAYA_DRY_RUN", "1")
}

#[rstest]
#[tokio::test]
#[serial_test::serial]
async fn dispatch_with_dry_run(#[from(dry_run)] _g: EnvGuard) {
    // ...
}
```

## Pending Contracts (fixtures for behavior not built yet)

When a phased plan freezes a contract before implementing it, the fixture must
fail *now* and keep failing for the right reason. A plain failing test cannot
do that: it leaves the suite red, and a red suite hides the next real
regression.

Wrap it instead. The body asserts the **target** behavior; the wrapper runs it
and demands a failure whose message contains a recorded oracle string:

| Body outcome | Verdict |
|---|---|
| fails, message contains the oracle | pass — contract still pending |
| fails, message does not | **fail** — the fixture's setup broke |
| passes | **fail** — promote it by deleting the wrapper |

The third row is what makes this safe: a contract cannot be implemented and
silently leave a pending fixture behind claiming it is not.

The CI-tooling suites carry three implementations, one per language:

- Python — `scripts/ci/pending_contracts.py`, the `@pending(criterion, reason,
  oracle)` decorator. `BISCUIT_PROMOTE_PENDING=1` runs every body unwrapped, so
  you can see which contracts now hold at the end of an implementation phase.
- Rust — `pending_contract(criterion, reason, oracle, body)` in
  `scripts/ci-rollup-tests.rs`. It swaps the panic hook so a deliberate
  failure does not spam the output. (`ci_workflow_contracts.rs` carried a twin
  until its last pending contract, the accepted-gap publisher, landed.)
- Shell — `pending_contract` in `.githooks/tests/test-pre-push.sh`.

Pair every pending fixture with a non-pending one that pins the *current*
defect. Otherwise a later change can satisfy the pending contract by altering
what the fixture builds rather than what the code does.

Choose the oracle from a message the fixture itself controls — a helper that
raises "the resolved plan has no 'areas' field" — not from a library's wording,
which drifts.
