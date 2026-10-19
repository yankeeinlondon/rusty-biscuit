# Test Design Contract

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

Before adding or changing a test, name the observable failure it must catch,
the cheapest boundary that proves it, and the state that boundary consumes. If
existing coverage already proves it, extend that instead; not every
implementation edit needs a new test.

- **Assertion quality.** A regression test keeps the original failing input,
  and its assertion must tell the broken behavior from the fix. Assert the
  dependent output, state, or error; an exit-success check or a value compared
  with itself proves nothing. Update test names and comments when behavior
  changes.
- **Boundary and cost.** Put exhaustive value and representation combinations
  at the cheapest boundary that proves them, and keep a few representative
  real-CLI, persistence, or protocol cases where the wiring matters. Stub
  external providers, never the behavior under test. Extend an existing
  shipped-artifact corpus test rather than rescanning the corpus in a new
  process per regression, but keep one end-to-end case through a real shipped
  artifact when changing its parser, schema, template, or configuration path.
- **Configuration matrices.** Test each decision once, where it is made. Field
  shapes (absent, null, wrong type, empty) belong to the field's validator:
  test them once per validator, whatever the source format. Document shapes
  (duplicate key, trailing content, format syntax) belong to the loader: test
  them once per distinct loader, and formats sharing a loader branch are one
  loader. Assert cells in-process through the crate's public result; spawn the
  binary only for a control and one or two errors per loader, to prove wiring
  and location rendering.
- **Explicit inputs.** Use the area's command builder
  ([spawning-binaries.md](spawning-binaries.md)) and fixture-owned directories,
  home/config/cache, environment, and tool lookup; disable unrelated service
  connections. A temporary input file does not isolate the launch directory or
  parent-repository discovery. Keep a real host detector only when that
  observation is the subject of the test.
- **Time.** Synchronize on readiness or on the asserted condition, with a
  deadline; a fixed sleep is not readiness proof. When elapsed time is the
  contract, keep its semantic floor and justify the budget and polling cadence
  under CI contention.
- **Terminal probes.** A bare PTY answers no terminal query, so a child that
  probes it (cursor position, OSC 10/11 colors) waits a full timeout per probe
  before printing anything. Answer each probe as it appears, matching the newly
  read chunk rather than the whole transcript so a late duplicate reply cannot
  land in a raw-mode prompt as a keystroke. See
  `claudine/cli/tests/common/pty.rs` and `biscuit-terminal`'s `ProbeAnswer`
  (`biscuit-terminal/lib/tests/common/pty.rs`).
- **Ownership.** Fixtures own and clean up children, threads, sockets, and
  directories, on failure too. Serialize only tests that share a real resource,
  with coordination the runner can see: nextest gives every test its own
  process, so `#[serial]` there enforces nothing and labeling a per-test
  resource as shared misleads.
- **Reachability.** Confirm that the test's name, `cfg`, required features,
  and recipe select it on the intended platforms
  ([tier-filters.md](tier-filters.md)). Missing, skipped, or resource-gated
  tests are not passing evidence.
- **Performance evidence.** Prove contracts such as "no repository walk" or
  "one request per operation" with stable work counters, not timings. Compare
  timings only between matched runs, and do not hide cost by changing tiers,
  dropping assertions, adding retries, or raising limits
  ([test-speed.md](test-speed.md)).

When a test exposes defective shared setup, check its sibling consumers too,
repair the shared fixture within scope, and record remaining consumers for
follow-up. Report what the targeted tests prove and which evidence is still
pending on CI; implementation complete and verified are separate claims.

A comprehensive audit or test-performance spec, and only that, follows
[test-suite-audits.md](test-suite-audits.md).
