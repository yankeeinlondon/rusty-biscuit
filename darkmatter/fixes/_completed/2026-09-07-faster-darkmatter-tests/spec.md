---
area: darkmatter
status: draft
created: 2026-09-07
reviewed: true
reviewed_by: opencode/zai-coding-plan/glm-5.3
reviewed_on: 2026-09-07
implemented: true
review_iterations: 2
packages:
    - darkmatter
    - darkmatter-cli
    - dmls
    - zed-dmls-cli
---

# Faster Darkmatter tests through isolated composition and complete evaluation

## Outcome

Evaluate all Darkmatter test families and reduce incidental discovery, repeated
composition, unnecessary process launches, and waiting while preserving parser,
schema, rendering, persistence, and language-server coverage. Make deterministic
CLI tests isolated by default.

This document is a draft, not evidence that every suspected cost is present.
Use the [rust-testing contract](../../../.claude/skills/rust-testing/SKILL.md)
and [audit guidance](../../../.claude/skills/rust-testing/test-suite-audits.md).
Ratify numeric targets after collecting compatible baselines.

## Evidence and scope

`cli/tests/common/mod.rs::md_cmd` currently returns a raw
`assert_cmd::Command::cargo_bin("md")` without fixture CWD/home/cache
defaults. Seven integration binaries carry private copies of that helper
(`schema_detect.rs`, `schema_validate.rs`, `compose_schema.rs`,
`compose_schema_file_rewrite.rs`, `schema_about.rs`, `code_block.rs`,
`schema_triggers.rs`), putting roughly 509 `md_cmd()` call sites on the
runner's ambient CWD, HOME, and PATH. `dmls/zed-dmls-cli/tests/cli.rs`
spawns `Command::cargo_bin("zed-dmls")` the same raw way. A temporary
Markdown input therefore does not by itself isolate launch discovery,
configuration, or installed tools.

The same shared module's HTTP fixture spawns an accept loop without retaining
a join handle or explicit shutdown control, and
`dmls/tests/level2_editor_neovim.rs` polls its capture deadline on fixed
200 ms sleeps. Audit early-exit/error paths and synchronization points and
measure any effect; do not assume either is a dominant suite cost.

Inventory all embedded and external tests, shared helpers, doctests, and
benchmark/fuzz entry points in `darkmatter`, `darkmatter-cli`, `dmls`, and
`zed-dmls-cli` — the four packages the area's canonical `sanity` and `test`
recipes actually select. The `dmls/zed-dmls` WASM extension builds outside
that selection (workspace-excluded, `wasm32-wasip2` target, verified via
`just check-zed` / `just zed-verify`); give its checks inventory rows carrying
that execution route rather than presuming recipe selection. `dmls/vscode-dmls`
ships no in-repo automated tests; record that absence as a documented row
noting the manual packaging check (`install-vscode-package`), not as coverage.

Include L2/L3/browser tests and cfg/feature/slow exclusions. Local L1 and CI
have different slow-test and feature selections; preserve and report both.
Existing fixture helpers and good tests should be reused.

Production composition changes, changes to expression semantics or cache
freshness, rendering redesign, editor feature development, and global CI
infrastructure changes are outside scope. Track production findings separately.

## Required behavior

### 1. Inventory the entire test population

Produce `inventory.md` in this fix's directory
(`darkmatter/fixes/2026-09-07-faster-darkmatter-tests/`) mapping each test
identity to a reviewed row or explicitly enumerated family. Record behavioral
proof, assertions, boundary, shared setup, filesystem/environment/cache
inputs, effect execution, waiting, cleanup, tier/features/platforms, canonical
recipe, measured cost, and disposition. Separate static presence from runtime
execution.

Reconcile source definitions, Cargo targets, nextest selection, and non-nextest
entry points. Include ignored and unavailable tests and explain their execution
route. A fast test is still reviewed for correctness and accidental effects.

### 2. Establish a shared deterministic CLI fixture

Replace the raw shared command helper with a fixture-owned command path and
migrate deterministic `md` tests, including the seven independent `md_cmd()`
definitions, plus the raw `zed-dmls` spawns in `zed-dmls-cli` — or record a
specifically justified disposition for those spawns. Keep fixture directories
alive through process completion and cleanup. Each test constructs its own
fixture and lends it to the builder, so parallel tests never share a root.

Decision — fixture placement: the fixture lives beside the helper it replaces
in `darkmatter/cli/tests/common/`, mirroring claudine's `CliProcessFixture`
reference implementation rather than promoting a shared crate first. See
Open Questions for the alternatives considered and the tracked follow-up.

Default CWD, home/config/cache directories, rendering inputs, application
environment, Git plumbing, and PATH must be explicit. Use a minimal native
tool set plus stubs, with documented escapes for genuine tool tests. Preserve
Windows executable resolution and platform home variables. Reject temporary
roots inside the checkout, including symlink paths.

Tests about relative references, repository discovery, schema lookup, and
source-context behavior build the required topology inside their fixture.
Shipped content is copied without changing bytes or relative-reference
relationships where isolation requires relocation. Do not hard-code all
references as absolute and thereby erase relative-resolution coverage.

Add structural protection against new bypasses using the smallest maintainable
mechanism. Test both a raw-spawn violation and an attempt to undo defaults.
Temporary migration exemptions must have reasons and stale-entry checks; no
generic migration exemption remains at completion.

### 3. Use the right composition boundary

Audit library tests for ambient `ComposeContext` capture and repeated
repository/host discovery. Supply explicit test context through existing
request APIs when discovery is incidental. Preserve representative end-to-end
tests for context capture, lazy demand-driven capture, source provenance, and
CLI-to-library wiring.

Retain real composition when shell expansion, transclusion, interpolation,
hashing, or persisted state is the behavior under test. Test passive schema
validation, trigger matching, completion, and hover without executing effects.
Use sentinel effects/work counters where available to prove absence of I/O
rather than relying only on a fast result.

Extend shared passive shipped-artifact corpus coverage instead of adding
duplicate full-corpus scans. Retain representative normal-invocation tests
through real shipped artifacts and repeated read/write/read coverage for
persisted values. Exhaustive representation matrices may use cheaper APIs
when they prove the same contract.

### 4. Own network, process, and rendering resources

HTTP fixtures use local ephemeral endpoints, bounded request handling, explicit
shutdown, and joined workers. Failure before the expected request count must
not strand the server. Verify request count/content and preserve remote
consent, cache freshness, refresh, and error behavior without public-network
access in deterministic tests.

Shell/provider fixtures, DMLS sessions, and temporary files have bounded cleanup
on success, failure, and cancellation. DMLS protocol tests should synchronize
on protocol responses rather than arbitrary sleeps and isolate workspace/cache
state.

Rendering policy tests use explicit terminal capabilities or document models
where sufficient. Keep real-terminal tests for terminal behavior and headless
browser tests for computed layout/style. Poll the final asserted condition,
retain real-boundary coverage, and never raise terminal/browser focus.

### 5. Measure before assigning budgets

Record baseline revision, dirty state, toolchain, features, profile, concurrency,
cache state, environment, test identities, and failures/skips. Separate build
time, runner duration, per-test serial sum, and intentional timing floors.
Retain local-default and CI-selected populations as distinct cohorts.

Collect five alternating warm local runs per revision for affected cohorts and
the full relevant L1 suite. Use compatible work counters for discovery,
composition, effects, and HTTP requests. Establish justified family budgets
before judging remediation; do not extrapolate a speedup from an unprofiled
substage. The prior
[redundant-walk results](../2026-07-16-redundant-walk/results.md) illustrate why
attribution must precede a percentage target.

Collect one candidate run — same workflow definition and runner image as the
baseline — for configured CI package/environment legs, preserving any failed
attempt and compatible baseline artifacts.
No new runner matrix is required by this draft. Compare matched identities
within each environment and show added/removed/gated tests separately.
Measurements absent on a platform remain pending.

## Verification and acceptance

- Every test/family, including extension and higher-tier surfaces, has an
  explicit disposition and execution route. No timing-based exclusions.
- All deterministic CLI spawns use the fixture contract or a specifically
  justified equivalent; no generic migration exemptions remain.
- Representative hostile inherited CWD/home/cache/Git/rendering/application
  inputs do not affect unrelated tests. Relative-reference and source-context
  behavior remain covered using disposable repositories.
- HTTP/process/protocol fixtures terminate within documented bounds when the
  expected interaction never occurs; no detached worker or child is accepted
  as normal cleanup.
- Passive paths remain effect-free, shipped-artifact coverage remains present,
  and all assertion/population changes have replacement-proof explanations.
- Area `just test` and `just lint` pass for affected packages; changed terminal
  and browser helpers are verified through `just test-l2` and
  `just test-browser`. L3 remains opt-in and unavailable evidence is recorded.
  CI slow-test and feature coverage cannot be reduced to improve timing.
- `results.md` (in this fix's directory) reports ratified budgets, comparable
  timing/work-count evidence, failures/skips, and separate
  implementation/local/CI status. No retry or timeout-limit increase is used
  as a performance fix.
- Relevant area skills/READMEs describe any changed fixture or test workflow.
  Changes to shared production APIs require downstream verification by impact,
  not a workspace-wide default test run.

## Open Questions

### Fixture ownership: area-local copy or promoted shared crate?

The deterministic CLI fixture contract this spec requires already exists in
claudine (`claudine/cli/tests/common/mod.rs` — `CliProcessFixture` plus the
spawn-site guard). Building Darkmatter's version means duplicating several
hundred lines of containment, PATH, and environment-scrubbing mechanics.
Three candidate designs:

- **Area-local fixture (duplicate the mechanics).**
  - Pros: smallest blast radius — this fix touches only darkmatter packages;
    escapes and guard vocabulary can be tailored to `md` (rendering inputs,
    cache roots) without negotiating claudine's needs; matches the
    rust-testing contract's "one shared command builder per package area".
  - Cons: real duplication of core mechanics; the two fixtures can drift —
    a containment fix landed in one silently misses the other.
- **Promote the core into `test_toolkit` now; consume from both areas.**
  - Pros: one implementation of containment/PATH/env scrubbing; a third CLI
    area onboards cheaply; drift is eliminated at the root.
  - Cons: expands this performance fix into claudine's test suite with
    migration risk and no timing payoff there; abstracts before Darkmatter's
    rendering-specific needs are fully discovered — the assume-then-falsify
    pattern the redundant-walk results documented.
- **Area-local now plus a scheduled promotion follow-up.**
  - Pros: keeps this fix surgical while making the duplication explicit and
    tracked; promotion happens with two concrete consumers in hand, so the
    shared core is extracted from evidence rather than conjecture.
  - Cons: temporary duplication; the follow-up needs an owner or it decays
    into the first option permanently.

Recommendation: the third design. Implement the area-local fixture in this
fix, and record a `features/_unscheduled/` spec to promote the stabilized
core into `test_toolkit`, migrating claudine as the second consumer.
Duplication across exactly two areas for one cycle is cheaper than a
premature abstraction whose seam is later proven wrong, and the tracked
follow-up prevents the duplication from becoming silently permanent.

### Baseline-review decisions

- How much cost belongs to composition, discovery, rendering, and test setup?
- Which command paths genuinely require host tools beyond the minimal set?
- Which DMLS/extension checks are reachable from existing recipes?
- Which repeated corpus/setup operations can be consolidated without reducing
  diagnostic quality or representative integration coverage?

## Rulings

- **2026-09-09 — one candidate CI run per leg, not three.** The text above
  originally required three consecutive candidate CI runs per configured leg
  before budgets could be derived, and the plan, the test-audit aggregator, and
  the results all inherited that number. Ken ruled it out: a full-scope run of
  this branch takes more than a day, so the PR gets exactly one candidate run
  per leg and is never re-run for sampling. Budgets that need more samples stay
  open as a residual rather than gating this fix.
