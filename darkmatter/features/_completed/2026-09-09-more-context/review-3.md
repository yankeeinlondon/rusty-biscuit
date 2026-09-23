---
$schema: feature-review.yaml
ready: false
findings:
  - title: Finish the request-owned ambient repository observation and restore the L1 gate
    priority: high
  - title: Exercise prompts/format.md at the Level 2 boundary required by AC28
    priority: high
  - title: Finish the explicit Level 1 acceptance guards
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: 2026-09-17T18:25:42-07:00
spec: 2026-09-09-more-context/spec.md
implemented: true
implemented_by: claude/fable
log: darkmatter/features/2026-09-09-more-context/implementation-log.md
description: A **fix** review of `2026-09-09-more-context/spec.md`
fix: 2026-09-09-more-context/review-3.md
previous: 2026-09-09-more-context/review-2.md
next: 2026-09-09-more-context/review-4.md
---

# Review 3: More Context

## Verdict

The feature is **not production ready**. The durable benchmark-fixture move
closes review 2's all-target build finding, and the existing AC28 lifecycle
Level 2 suite still passes. However, the request-owned repository-observation
fix fails its own new Level 1 tests and still permits a lazy-only request to
choose its supposedly fixed observation at first evaluation rather than at
request creation. The required `prompts/format.md` Level 2 case and the
explicit AC4, AC29, AC30, and AC36 Level 1 guards also remain absent.

This verdict does not depend on missing cross-OS execution evidence or human
review. Ken's prior design rulings are already recorded as R34-R37.

## Prior Review Closure

- **Restore Darkmatter's all-target build after moving the performance
  fixtures:** implemented. The fixtures now live under the durable
  `darkmatter/benchmarks/` path, all Rust and benchmark consumers use that
  location, `just lint` passes across the package area, and the L1 run compiles
  all 140 binaries before reaching the new repository-observation failures.
- **Keep ambient current repository facts on the request-owned observation:**
  attempted but not complete. `AnchoredRefresh` now memoizes repository values
  and the new tests exercise work counters, but two tests fail in the canonical
  L1 run and lazy-only requests still establish the observation at first read.
- **Exercise prompts/format.md at the Level 2 boundary required by AC28:** not
  implemented. The only shipped-prompt Level 2 case still loads `plan.md`.
- **Finish the explicit Level 1 acceptance guards:** not implemented. The
  entropy, migration-allowlist, URL request-count, and warm-cache
  identity/probe cases identified in review 2 are unchanged.

Review 2's `## Blocked Findings` section was `None`, so no blocked finding was
available to become unblocked before this implementation.

## Unblocked Findings

### High — Finish the request-owned ambient repository observation and restore the L1 gate

The attempted fix does not yet satisfy D3 or pass its own tests.
`AnchoredRefresh::for_request` seeds the memo only when eager `ctx.*` capture
already requested the Repo group. Otherwise `repository()` calls
`capture_runtime_context_for_groups` from inside the first `current.*`
evaluation (`context/current.rs:102-132`). Consequently, a request containing
only `current.repo`, `current.repo_root`, or `current.packages` has no
repository observation at request creation: repository metadata or package
topology can change before the first expression and become the value treated as
fixed for the remainder of the request. That contradicts D3's “captured once
per request in ComposeOptions” boundary and the required behavior that lazy
providers never rederive repository root or topology downstream.

The new work-counter coverage also demonstrates that the wiring is not sharing
one observation through the normal compose path. The focused run failed
`a_request_that_never_captured_repo_discovers_it_once` with two root discoveries
and two topology walks where the test permits one (`tests/lazy_roots.rs:470-487`).
The canonical `just test` run reproduced this failure. It also failed
`repository_facts_hold_while_mutable_facts_refresh_after_on_disk_changes`
because the test assumes `ctx.packages` has a stable `alpha`, `alpha-cli`
ordering (`tests/lazy_roots.rs:438-455`), while the detector returned the same
set in the opposite order. That assertion should compare the semantic set or
the product should define and enforce ordering; an incidental filesystem walk
order is not stable test evidence.

Capture the immutable repository observation when the request is created,
independently of eager `ctx.*` requirements, and carry that same observation
through reference validation, preflight, the terminal compose, and child
pipelines. Add a regression that creates the request, mutates repository
metadata/topology before the first lazy evaluation, and proves `current.*`
still returns the request-start observation with zero downstream discovery.
Make the package assertion order-independent unless ordering is intentionally
part of the public contract. The canonical L1 gate must then pass.

### High — Exercise prompts/format.md at the Level 2 boundary required by AC28

AC28 explicitly requires every migrated shipped prompt, including
`prompts/format.md`, to compose through a real Claudine run. The Level 2
shipped-prompt test still reads only `prompts/plan.md` and validates
`current.time` (`level2_ac28_lifecycle_lazy_roots.rs:512-566`). The format
prompt's lifecycle output uses `length(current.dirty_files)`
(`prompts/format.md:25`), so the existing test does not traverse the
FileChanges refresh capability.

The four present AC28 tmux tests pass at Level 2, which proves lifecycle-event
wiring, environment refresh, branch refresh, and the `plan.md` DateTime path.
It cannot detect a broken or missing FileChanges provider. Add a focus-safe
tmux case that invokes the real `format.md` prompt in a fixture repository with
a known dirty-file set and asserts output that distinguishes the expected
non-empty count from an unsupported or empty value. This is still a missing
test at the specification's required level and remains a high-severity
readiness gap.

### Medium — Finish the explicit Level 1 acceptance guards

The four review-2 gaps remain:

- AC4 requires an entropy failure to surface as the typed
  `ExecutionNonceUnavailable` error through composition. The only failure test
  still stops at `PartialRuntimeCapture` inside `capture/document.rs:281-297`;
  no test evaluates `ctx.id` or `ctx.sid` through a compose boundary after an
  injected entropy failure.
- AC29/R37 requires a checked-in allowlist of file path plus expected count,
  with rejection of both new and stale `current.ctx.*` / `current.env.*`
  occurrences. The repository still has only individual negative tests and no
  scoped migration guard.
- AC30 requires a URL root to compose without an extra fetch. Existing request
  counters cover remote children and cache behavior, but no URL-root identity
  test asserts the exact root request count.
- AC36 requires warm `--cache-root` executions to prove that neither execution
  identity nor probe output is replayed. The unchanged CLI regression still
  compares only `ctx.timestamp_ms` (`compose_remote_caching.rs:304-358`), and
  the library equivalent does the same. Neither names `ctx.id`/`ctx.sid` nor a
  probe whose result changes between executions.

Add focused L1 tests through the normal composition or CLI boundary. None of
these cases requires a terminal, browser, live network, or OS input injection.

## Blocked Findings

None. Every finding is implementable with deterministic fixtures and the
existing headless tmux harness. No additional human decision is required.

## Requirement Verification Levels

No requirement involves terminal glyph geometry, SGR styling, keyboard input,
paste/IME, mouse behavior, or scrolling. Level 3 is not applicable. Level 2 is
required only where AC2 and AC28 explicitly require real Claudine execution;
all other deterministic behavior belongs at Level 1, while AC14 is a real
network-resource test rather than a terminal tier.

| Requirements | Required level | Strongest evidence and assessment |
| --- | --- | --- |
| AC1, AC19, AC26 | Level 1 passive corpus, generator drift, and listing CLI | Present at the correct level; all targets now compile during lint and L1 startup. |
| AC2 | Level 2 real Claudine | The linked/main-worktree case passed in review 2; no related implementation changed in this cycle. Correct level. |
| AC3-AC5, identity parts of AC36 | Level 1 fixed vectors and normal compose/CLI paths | Hash parity, retained identity, transclusion, uniqueness, and vectors exist. Entropy-to-compose-error and warm-cache identity cases remain missing. |
| AC6-AC8, AC33 | Level 1 deterministic ICMP transport/policy fixtures | Present at the correct level. |
| AC9, AC13 | Level 1 interface/route fixtures and projection | Present at Level 1; cross-OS execution belongs to CI. |
| AC10-AC12, AC34 | Level 1 controlled shell/profile fixtures | Present at the correct level; no real profile or focused window is required. |
| AC14 | Real network resource on each required OS | Tests exist; cross-OS results do not affect this readiness verdict. |
| AC15-AC18, AC20, AC32, AC35 | Level 1 compose/function fixtures | Present at the correct level. |
| AC21-AC23 | Level 1 ambient and supplied-evidence fixtures | Present at the correct level. |
| AC24-AC25, AC37 | Level 1 fixed repositories and CLI-format parity | Present at the correct level. |
| AC27, AC31 | Level 1 scripted provider, pipeline, and work counters | The ambient work-counter tests are at the correct level but fail, and lazy-only requests still capture fixed repository state at first evaluation rather than request creation. **High implementation gap.** |
| AC28 | Level 2 real Claudine, supported by Level 1 executor/provider tests | The four focus-safe tmux cases passed and cover all lifecycle events, environment and branch freshness, and `plan.md`. `format.md` remains absent. **Missing required Level 2 proof; high gap.** |
| AC29 | Level 1 passive documentation/migration guard | Documentation is migrated, but the required allowlist and stale-entry guard do not exist. |
| AC30 | Level 1 source-kind and request-count fixtures | Source identity coverage exists; the URL-root exact-request-count assertion is missing. |
| Cache portion of AC36 | Level 1 cold/warm cache fixture | General no-local-artifact and timestamp freshness checks exist; named identity and probe-output replay cases are missing. |

## Verification

- `cd darkmatter && just lint` passed for `darkmatter`, `darkmatter-cli`,
  `dmls`, `zed-dmls-cli`, and the `wasm32-wasip2` extension check.
- `cd darkmatter && just test --color=never` compiled 140 binaries, then
  stopped after 2,896 passed and 2 failed; 5,189 tests were canceled. Both
  failures are new ambient repository-observation tests described above.
- A focused no-fail-fast run of the three ambient repository tests failed all
  three: the same two defects plus a work-counter failure in the eager-Repo
  case under that concurrent selection.
- The focus-safe tmux AC28 binary passed 4 of 4 Level 2 tests with
  `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`; no test skipped. It contains no
  `format.md` case.
- `git diff --check` passed.
- GitNexus was bound to `rusty-biscuit`; the index matched the current commit
  and all 7,376 covered files. Its concept query found `CurrentProvider` but
  no execution flow for the new ambient implementation, so direct source and
  call-site inspection supplied the unresolved details.
- GitNexus `detect-changes --scope all` reports critical aggregate risk across
  473 changed files and 41 affected flows in the shared in-progress worktree.
  That result is not used to attribute unrelated edits to this feature, but it
  confirms that a clean regression gate is required before release.
