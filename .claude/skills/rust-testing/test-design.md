# Test Design Contract

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Before adding or changing a test, identify the observable failure it must
catch, the boundary needed to prove it, and the state that boundary consumes.
Reuse or extend existing coverage when it already provides that proof; a new
test is not required for every implementation edit.

- **Assertion quality.** For a regression, retain the original failing input
  and show that the relevant assertion distinguishes the broken behavior from
  the fix when feasible. Assert dependent output/state and meaningful error
  behavior. An exit-success check or a value compared with itself is not proof
  of a promised result. Review names and comments when behavior changes.
- **Boundary and cost.** Put exhaustive value/representation combinations at
  the cheapest boundary that proves them. Keep representative real-CLI,
  persistence, or protocol coverage where wiring matters. Stub external
  providers, not the application behavior under test. Extend a shared passive
  shipped-artifact corpus test rather than rescan the whole corpus in a new
  process for every regression; keep an end-to-end case through a real shipped
  artifact when changing its parser, schema, template, or configuration path.
- **Explicit inputs.** Use the area's command builder and fixture-owned
  directories, home/config/cache state, environment, and tool lookup. Disable
  unrelated service connections. A temporary input file alone does not isolate
  launch CWD or parent repository discovery. Host discovery tests retain the
  real detector only when that observation is the subject of the test.
- **Time and ownership.** Synchronize on readiness or the final condition being
  asserted, with a deadline; do not use a fixed sleep as readiness proof. When
  elapsed time is the contract, preserve its semantic floor and justify the
  budget, polling cadence, and shutdown margin under CI contention. Also check
  whether the *child* is waiting on the harness: a bare PTY answers no terminal
  query, so a child that probes it — DSR cursor position (`ESC[6n`), OSC 10/11
  foreground and background colour — pays a full timeout per unanswered probe
  before it emits anything. Answer each on observation, as
  `claudine/cli/tests/common/pty.rs` and `biscuit-terminal`'s `ProbeAnswer` do;
  match the newly-read chunk, not the cumulative transcript, so a late
  duplicate reply cannot land in a raw-mode prompt as an `ESC` keystroke.
  Fixtures own and clean up children, threads, sockets, and directories on
  failure too. Serialize only tests sharing an actual resource, using
  runner-visible coordination when tests execute in separate processes — and
  do not label a per-test resource shared: nextest gives every test its own
  process, so a `#[serial]` group there enforces nothing and misleads.
- **Reachability.** Confirm that the test's name, `cfg`, required features, and
  recipe select it on the intended platforms. L1 includes hermetic subprocess
  and filesystem tests. OS-specific behavior alone does not require L2/L3.
  Missing tests, skipped tests, and unavailable resources are not passing
  evidence; terminal/browser tests must preserve focus.
- **Performance evidence.** Use stable work counters for contracts such as
  “no repository walk” or “one request per operation”; use timings to measure
  actual latency. Compare matched tests with the same features, profile,
  concurrency, platform, and cache conditions. Separate build time from test
  execution. Do not hide cost by changing tiers, dropping assertions, adding
  retries, or raising runner limits without diagnosing the cause.

When a test exposes defective shared setup, inspect its siblings even if they
have never crossed a slow threshold. Repair the shared fixture within the
authorized scope and record remaining consumers for follow-up. Report which
behavior the targeted tests prove, relevant broader gates, and evidence still
pending on CI; distinguish implementation completion from verification.

For a requested comprehensive test audit or test-performance specification,
read [test-suite-audits.md](test-suite-audits.md). Ordinary feature work does
not require a suite-wide audit. The tooling those audits run on (listing
captures, CI JUnit gates, inventory reconciliation, cost attribution,
alternating-run measurement, and work-count comparison) is the shared
`tools/test-audit` package, driven by a per-area `audit.config.json`; see
[test-audit-tooling.md](test-audit-tooling.md) for install, commands,
configuration, and how to read its numbers.
