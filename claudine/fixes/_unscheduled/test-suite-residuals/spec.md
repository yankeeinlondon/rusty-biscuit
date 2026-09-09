---
area: claudine
status: unscheduled
created: 2026-09-08
origin: claudine/fixes/2026-09-07-faster-claudine-tests/results.md
packages:
    - claudine
    - claudine-cli
    - claudine-contract
---

# Test-suite residuals deferred by the 2026-09-07 faster-tests fix

Seven findings the
[faster-claudine-tests fix](../../2026-09-07-faster-claudine-tests/results.md)
surfaced and could not close inside its scope. Each carries the evidence it
was found with, the reason it was deferred, and what closing it looks like.
None is a fixture migration: the L1 spawn allow-list is empty and that clause
of the fix's acceptance criteria is not what any of these defers.

## 1. Four zero-byte bench entry points

- **Evidence:** `claudine/lib/benches/{claude_parse,opencode_parse,pre_flight_checks,prompt_preparation}.rs`
  are 0 bytes, tracked since `0110a7a76`. Only `runtime_hot_paths.rs` is a
  declared `[[bench]]`; the four are auto-discovered, compile as empty libtest
  bench targets on every `just bench`, and contribute nothing
  (`inventory.md` § Benches).
- **Why deferred:** a hygiene change with no test-performance content;
  deleting them is behaviour-free and belongs with whoever owns the benchmark
  surface, in its own commit.
- **Closes when:** the four files are deleted or given real benchmarks, and
  `just bench` still exits 0.

## 2. `context` reports have no in-process render seam

- **Evidence:** `render_default_report` and its siblings write to `log::data`
  rather than returning text, so the two `context_command` width sweeps
  (12 and 7 launches) can only be proved at the CLI boundary. Phase 6 hoisted
  the fixture out of the loops (2.18 s → 0.90 s for the widest sweep) but
  every launch remains.
- **Why deferred:** adding a capture seam is a production change, which the
  fix's scope excludes.
- **Closes when:** a renderer returns (or accepts a sink for) the report text,
  the width × mode matrix moves to a library test, and one representative
  real-binary case per report mode stays in `context_command.rs`.

## 3. `a_failed_ownership_setup_kills_the_spawned_command` holds vacuously

- **Evidence:** in `composition::sequence::task::tests`, the injected failure
  fires on the statement after `spawn`, so the kill reaches the shell before
  its first command runs and nothing is ever backgrounded; the
  `!marker.exists()` assertion has always been true. Converting it to
  `BackgroundedDescendant` fails on exactly that (Phase 7, `plan.md`).
- **Why deferred:** strengthening it needs the task runner to expose the
  direct child's pid, a production change. The limit is now stated at the
  test rather than implied by its name.
- **Closes when:** the runner exposes the child pid (or an equivalent
  observation), and the test asserts that a spawned child is killed on
  ownership-setup failure with a non-vacuous witness.

## 4. `real_provider` fails `Unauthorized` where its contract says it skips

- **Evidence:** `just test-real` in `claudine/`: four
  `claudine-contract::real_provider` identities fail with `Unauthorized` on a
  host whose provider CLI is not authenticated, identically under the retired
  `cargo test` route (Phase 6). The file's module contract says each test
  "skips cleanly when its provider/model is unavailable".
- **Why deferred:** deciding whether an expired credential is "unavailable"
  or a real failure is a contract question for the adapter's owner; the fix
  recorded the tier as pending rather than changing what the tests assert.
- **Closes when:** the contract names the credential case explicitly and the
  tests either skip with a reason or fail with a diagnostic that says
  "authenticate", and `just test-real` on an authenticated host is 5 of 5.

## 5. `completion_perf::perf_enter_compose_partial_meets_target` fails on this host

- **Evidence:** the `#[ignore]`d harness reports
  `autocomplete requires an interactive terminal` from its PTY chooser, before
  and after the fixture migration (Phase 5, verified by restoring the
  pre-migration file).
- **Why deferred:** pre-existing, reachable only by `--ignored`, and a
  wall-clock budget assertion the fix's own measurement rules keep out of
  always-on gates.
- **Closes when:** the harness either drives the chooser through a PTY that
  satisfies the interactivity check or documents the host requirement and
  skips with that reason.

## 6. `claudine-cli-ci-l1` runs at `max-threads = 1`

- **Evidence:** the CI-profile test group serialises `claudine-cli`'s ~2,500
  L1 identities, so runner elapsed equals summed duration on every leg
  (`baseline/34173378609/`: 324.9 vs 324.7 s on Ubuntu). It is the single
  largest CI cost lever in the area (`inventory.md` § Runner override census,
  entry 11).
- **Why deferred:** relaxing it needs the candidate CI runs the fix could not
  produce (no push); a cap change without that evidence is the runner-limit
  change AC5/AC7 forbid.
- **Closes when:** three green candidate runs per leg exist, the group is
  raised one step at a time, and each step's four legs stay green with no
  new `LEAK`, timeout, or flake.

## 7. Budget derivation needs a JUnit → family aggregator

- **Evidence:** `attribution.ts deriveBudgets` refuses (`missing-leg`) while
  `perLegFamilySummed` is empty; `attribution.ts` reads nextest logs and the
  CI evidence is JUnit XML. `inventory-reconciler.ts`'s matcher already maps
  an identity to a family, so the missing piece is a join, not a classifier.
- **Why deferred:** the fix's Phase 9 ended with one baseline run per leg, so
  the aggregator would have had nothing to aggregate; writing it against a
  single run risks fitting the tool to the data it will later gate.
- **Closes when:** a script reads the stored `baseline/<run>/<env>/` trees,
  emits `perLegFamilySummed` per leg, `budgets-pending.json` reaches
  `runsPerLeg` 3, and `deriveBudgets` produces the table that lands beside the
  baseline in `inventory.md` § Budgets.
