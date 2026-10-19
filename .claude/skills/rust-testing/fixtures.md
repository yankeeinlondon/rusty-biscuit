# Fixtures, Env Guards, and Pending Contracts

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## Env guards

Set process environment through `test_toolkit::EnvGuard` (restored on drop),
typically as an `rstest` fixture, and mark any test that mutates global state
`#[serial_test::serial]`:

```rust
#[fixture]
fn dry_run() -> EnvGuard { EnvGuard::set_safe("PLAYA_DRY_RUN", "1") }

#[rstest] #[serial_test::serial]
fn dispatch_with_dry_run(#[from(dry_run)] _guard: EnvGuard) { … }
```

For a spawned binary, set the child's environment through the area's command
builder instead ([spawning-binaries.md](spawning-binaries.md)).

## Pending contracts (fixtures for behavior not built yet)

When a phased plan freezes a contract before implementing it, the fixture must
fail now and keep failing for the right reason. A plain failing test leaves the
suite red, and a red suite hides the next real regression.

Wrap it: the body asserts the **target** behavior, and the wrapper demands a
failure whose message contains a recorded oracle string.

| Body outcome | Verdict |
|---|---|
| fails, message contains the oracle | pass: contract still pending |
| fails, message does not | **fail**: the fixture's setup broke |
| passes | **fail**: promote it by deleting the wrapper |

The third row means a contract cannot be implemented while a pending fixture
still claims it is not.

The live implementation is `scripts/ci/pending_contracts.py`, the
`@pending(criterion, reason, oracle)` decorator; `BISCUIT_PROMOTE_PENDING=1`
runs every body unwrapped, showing which contracts now hold at the end of a
phase. A Rust or shell suite that needs one follows the same three-row
contract, and a Rust wrapper should swap the panic hook so a deliberate failure
does not spam the output.

- **Pair every pending fixture with one that pins the current defect.**
  Otherwise a later change can satisfy the contract by altering what the
  fixture builds rather than what the code does.
- **Choose the oracle from a message the fixture controls** (a helper that
  raises "the resolved plan has no 'areas' field"), not a library's wording,
  which drifts.
