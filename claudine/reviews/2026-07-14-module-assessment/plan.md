---
total_phases: 12
created: 2026-07-14
source_review: review.md
---

# Claudine Module-Assessment Implementation Plan

This plan closes every remaining recommendation in [`review.md`](review.md). It
is ordered to preserve behavior at the riskiest boundaries, establish
regression guards before broad mechanical movement, and keep provider-neutral
policy in the library while CLI code retains process and terminal I/O.

## Completion criteria

The work is complete when all of the following are true:

- `run_harness_loop_inner` and
  `execute_composition_request_inner_with_guard` are readable coordinators over
  explicit phase functions and transition results; neither immediately
  destructures a large context into loosely coupled locals.
- Provider-neutral lifecycle transition policy has one owner in the library.
  The CLI supplies process launch, filesystem, messaging, and rendering
  adapters without introducing CLI dependencies into `claudine`.
- A hard structural test enforces the documented inline-test thresholds across
  all Claudine package-area crates, excludes generated sources, and has only
  narrow exceptions with recorded reasons.
- Every current inline-test violation is either migrated or explicitly
  justified, and the six largest extracted test suites are divided by behavior
  rather than retained as monolithic `tests.rs` files.
- Rendezvous is documented as a first-class package-area family, and its
  session-log responsibilities and oversized sync/service tests have coherent
  module owners.
- Wrapper spawning and termination are split by stable responsibility without
  duplicating the Unix/Windows signal ladder or changing interruption,
  timeout, watchdog, and child-reaping semantics.
- `claudine-gen` retains its bootstrap-independent crate boundary, has
  domain-oriented emitter/coercion modules, and renders every human-facing
  output path through `TerminalRenderable`; machine-facing JSON remains raw.
- Composition error rendering delegates by error family while the public
  `CompositionError` vocabulary remains intact.
- Loop/lifecycle interpolation either shares the Darkmatter substrate or has a
  documented intentional divergence backed by common conformance cases.
- The cited provider-metadata and package-architecture documentation drift is
  removed, generated artifacts remain byte-clean, and macOS, Linux, and Windows
  verification gates pass.

## Risk and sequencing constraints

GitNexus reports HIGH upstream risk for `run_harness_loop_inner`,
`run_child_stream_semantic`, and
`wait_with_signal_early_termination_and_completion`. The first affects
composition, lifecycle, wrapper, and execution flows; the latter two affect
the harness and Ctrl+C/termination paths. Before changing any named symbol in
this plan, rerun upstream impact analysis and stop for review if the result is
HIGH or CRITICAL or its direct callers differ from this baseline.

The plan deliberately does not split generated provider data,
`signals/generated.rs`, the declarative `gen/src/registry.rs`,
`catalog-types/src/signal.rs`, the central `CompositionError` enum, or cohesive
render/stream modules solely to reduce line counts.

## Phase 1 — Lock current orchestration and platform behavior

Establish a behavior baseline before changing either state machine or the
cross-platform process layer.

- Add focused characterization cases for lifecycle ordering across
  `initialize`, `start`, `success`, `failure`, `finalize`, and `loop`, including
  evaluation-error precedence, action errors, terminal-slot redesignation,
  and finalize-once behavior.
- Pin recovery behavior for `retry`, `resume`, `proxy`, `stop`, `error`, and
  unsupported setup-phase recovery, including attempt budgets, proxy chains,
  session availability, and the `provider_launched` re-entry distinction.
- Pin composition setup ordering for target selection, launch workspace,
  environment/MCP construction, argv and system-prompt preparation, lifecycle
  setup, initialize routing, and handoff to `run_composition_body`.
- Pin process semantics for inherited, captured, and semantic-stream modes:
  normal completion, user interruption, timeout, watchdog termination,
  completion-triggered termination, exit-summary projection, and child reaping.
- Record byte baselines for every generated provider artifact and stable
  snapshots for human-facing generator reports and composition error blocks.
- Capture the current test-placement inventory and `hug god-files --json
  claudine` result as measurement inputs; generated and test-only files must be
  labeled separately from actionable production code.

Acceptance gates:

- All new cases pass against the unrefactored implementation.
- `just test-library`, `just test-cli`, `just test-gen`, and `just
  test-rendezvous` pass.
- `just test-l2` passes on an available headless terminal backend; Windows
  completion/Ctrl+C cases are confirmed runnable in the Windows CI job.

## Phase 2 — Define the provider-neutral lifecycle transition core

Finish the C4 layering work before decomposing the two CLI callers.

- Extend `lib/src/composition/lifecycle/runtime.rs` with the smallest pure
  transition vocabulary needed by both preflight and the harness loop. Model
  state inputs and decisions explicitly: lifecycle event/slot, launched state,
  prior/evaluation/action error, control action, available session, attempt and
  proxy budgets, and finalize state.
- Return typed transition decisions such as continue/re-enter, finalize,
  terminal success/failure, proxy handoff, or abort. Keep filesystem access,
  process spawning, terminal output, messaging, and provider-specific command
  construction out of the library.
- Replace CLI-side “mirror” helpers only where both callers genuinely share the
  transition contract. Retain CLI adapters for executing the pure decision.
- Add table-driven library tests covering every event/control/error
  combination used by composition preflight and the harness run loop.
- Review and update docs/comments for the changed transition contracts; remove
  prose that still describes one CLI path as mirroring another.

Acceptance gates:

- The library transition matrix is exhaustive and both CLI paths consume it.
- No dependency from `claudine` to `claudine-cli` or CLI-only crates is added.
- Existing Phase 1 ordering and recovery tests remain unchanged and green.
- `just test-library`, `just test-cli`, and `just lint` pass.

## Phase 3 — Decompose composition request setup and initialize routing

Turn `execute_composition_request_inner_with_guard` into a coordinator without
changing its public wrappers or `SingleCompositionOutcome` contract.

- Introduce a cohesive composition-attempt state/context that remains intact
  across the pipeline instead of becoming a large set of locals.
- Extract stable preparation phases with explicit inputs and outputs:
  selection/launch resolution, environment and MCP preparation, argv and
  system-prompt construction, lifecycle runtime construction, initialize
  execution/routing, and provider-run handoff.
- Use a typed phase result for proceed, completed, blocked, or failed outcomes;
  route lifecycle transitions through the Phase 2 library core.
- Keep target selection, command construction, filesystem access, perf
  collection, dry-run rendering, and provider launch in CLI modules.
- Preserve `execute_composition_request`, sequence callers, interactive
  selection, silent/dry-run behavior, and error enrichment at the render
  boundary.

Acceptance gates:

- The root function reads as ordered phase calls with no duplicated error or
  finalize routing and no immediate context destructuring.
- Phase functions have focused unit tests, while Phase 1 end-to-end behavior
  remains byte/sequence equivalent.
- `just test-cli`, `just test-library`, and `just lint` pass.

## Phase 4 — Decompose the harness attempt/recovery state machine

Refactor the HIGH-risk `run_harness_loop_inner` after the shared transition
contract and composition caller have proven the boundary.

- Replace the current loose mutable locals with a `HarnessLoopState` that owns
  attempt count, retry/resume budgets, prompt/session state, proxy tracking,
  cached shell options, lifecycle guard state, and the immutable run context.
- Extract phases for prompt materialization/preflight, attempt execution,
  result classification, lifecycle event execution, terminal recovery,
  requeue/proxy handling, and next-attempt preparation.
- Make each phase return an explicit loop transition such as retry, resume,
  proxy, complete, or abort. Apply the Phase 2 provider-neutral decision in a
  CLI adapter rather than re-encoding event-specific gates.
- Preserve `drive_terminal_recovery` where it remains the single terminal-tail
  executor; do not create a second recovery abstraction with overlapping
  ownership.
- Keep process launch and provider command details in the existing CLI
  attempt/profile modules.

Acceptance gates:

- The loop body exposes phase ordering and re-entry points without reading an
  800-line function, and state ownership is visible from the context type.
- Retry/resume/proxy/finalize tests cover every transition and budget edge.
- The Phase 1 harness characterization suite, `just test-cli`, `just
  test-library`, `just test-l2`, and `just lint` pass.

## Phase 5 — Build the test-placement analyzer

Create an accurate, reusable analyzer before mechanically moving the current
violations.

- Add a structural test under the Claudine CLI integration-test tooling,
  following the existing dispatch-inventory scanner pattern, and scan
  `lib`, `cli`, `contract`, `catalog-types`, `gen`, and
  `rendezvous/{core,client,daemon}`.
- Centralize the documented thresholds: approximately 800 production lines or
  300 lines in an inline `mod tests` body. Count production and test bodies
  separately and handle attributes, comments, strings, raw strings, and nested
  braces so diagnostics are stable.
- Ignore generated files by explicit path/header rules, not by broad directory
  exclusions. Report the file, production-line count, test-line count, and
  threshold exceeded.
- Support a narrow exception table whose entries contain a path and durable
  rationale; reject stale exceptions when a file no longer violates a rule.
- Unit-test the analyzer with portable fixtures. Keep the repository-wide
  assertion in report-only mode until Phase 6 eliminates the current debt, so
  this phase remains green without grandfathering roughly 90 violations.

Acceptance gates:

- Analyzer fixtures cover Unix and Windows newlines and the Rust constructs
  above.
- The report reproduces the review's classes of violation and excludes
  generated provider/signal artifacts.
- `just test-cli` and `just lint` pass.

## Phase 6 — Eliminate inline-test debt and activate the hard gate

Apply the analyzer to the entire package area as a mechanical, behavior-neutral
migration.

- Move every current threshold-violating inline test module to a sibling
  `tests.rs` or `tests/mod.rs`, including the cited rendezvous sync/service,
  wrapper spawn/termination, stream-provider, composition/dispatch, and
  generator files.
- Preserve test names, module visibility, `use super::*`, `cfg` gates, fixtures,
  serial annotations, and platform-specific imports. Do not combine these
  moves with production refactors or formatting churn.
- Review each proposed exception individually. Keep only cases where
  co-location materially clarifies private invariants; record why extraction
  would be worse and require the exception to remain below a separately stated
  ceiling.
- Switch the Phase 5 repository assertion from report-only to a normal Level 1
  test and remove any temporary inventory/baseline entries.
- Update the Claudine architecture Test Placement section to identify the
  enforcing test and exception policy.

Acceptance gates:

- The structural test reports zero unapproved violations across all Claudine
  package-area crates.
- Production diffs in this phase contain only test-module declarations and
  necessary visibility/import adjustments.
- `just test`, `just test-rendezvous`, `just test-gen`, and `just lint` pass.

## Phase 7 — Divide the largest sibling tests by behavior

Resolve the remaining test-navigation hotspots without claiming a Rust
compile-unit optimization.

- Divide `composition/lifecycle/tests.rs` into parse/config, validation,
  action-shape/control, audio/emission, guard/runtime, and diagnostics suites.
- Divide harness `loop_control/tests.rs` into lifecycle ordering, terminal
  routing, retry/resume, proxy, and requeue suites.
- Divide lifecycle executor tests into action dispatch, conditions/control,
  event-time interpolation, mutation visibility, and filesystem/lookup suites.
- Divide OpenCode bridge tests into ingest/classification, session lifecycle,
  usage/retry guards, stalled-generation progress, stdout/stderr coordination,
  and signal projection suites.
- Divide rendezvous session-log tests into append/rotation, durability,
  replay/rehydration, remote validation, and replace/update suites.
- Divide loop-engine tests into seed/state, iteration/actions, rate limits, and
  lifecycle/control suites.
- Put shared fixtures in a small parent `tests/mod.rs`; avoid a new catch-all
  helper module that simply relocates the original hotspot.

Acceptance gates:

- Each test file has one discoverable behavioral responsibility and no copied
  fixtures or assertions.
- Test names and coverage remain stable, and the Phase 6 placement guard stays
  green.
- `just test-library`, `just test-cli`, `just test-rendezvous`, and `just lint`
  pass.

## Phase 8 — Give rendezvous an explicit architecture and session-log boundary

Apply the package area's architectural conventions to
`rendezvous/{core,client,daemon}`.

- Update the Claudine skill overview and architecture document to enumerate
  all three rendezvous crates, their dependency direction, public roles, and
  test commands.
- Keep `SessionLogManager` as the public facade while extracting its existing
  responsibilities into modules for local append/rotation, export/import
  staging, startup replay/rehydration, and remote metadata/schema/append-only
  validation.
- Make shared session state and invariants explicit without exposing internal
  Loro/storage types unnecessarily. Preserve persistence ordering and recovery
  semantics pinned in Phase 1.
- Complete the sync/service test extraction started in Phase 6 and organize
  those suites by protocol framing/timeouts, service RPC behavior, projection,
  and error mapping.
- Review session-log, sync, and service docs/comments for stale responsibility
  descriptions after the moves.

Acceptance gates:

- The facade's public API is unchanged unless a separately reviewed API change
  is required, and startup replay, crash-window, signature, append-only, and
  sync tests remain green.
- `cd claudine/rendezvous && just check && just test && just lint` passes.
- Root `cargo check --workspace --all-targets` still includes all three crates.

## Phase 9 — Split wrapper spawn modes and platform termination

Refactor the HIGH-risk process layer by execution mode and platform boundary.

- Split spawn code into shared command/process setup plus inherited-output,
  captured-output, and semantic-stream execution modules. Share only stable
  setup and wait contracts; keep mode-specific pipe/thread/parser behavior
  local.
- Split termination into provider-neutral termination reasons, summary/guard
  projection, and human-facing rendering, plus `cfg(unix)` and `cfg(windows)`
  wait/escalation implementations behind one internal interface.
- Keep one semantic signal ladder and one early-termination projection. Unix
  process-group signaling and Windows Job Object/console-event behavior must
  remain platform implementations of the same contract, not copied policy.
- Render termination messages through existing terminal components and retain
  stdout/stderr separation.
- Move the extracted platform tests into matching module trees and retain the
  L2/L3 integration coverage.

Acceptance gates:

- Normal exit, capture caps, semantic streaming, interruption feedback,
  timeout, watchdog, completion termination, and reaping cases pass.
- `just test-cli`, `just test-l2`, and `just lint` pass on macOS/Linux-capable
  paths.
- Windows CI runs `cargo check --all-targets` for `claudine-cli` and
  `just test-windows-ctrl-c`; no Unix-only import or path assumption reaches a
  Windows build.

## Phase 10 — Bound generator growth and complete its rendering contract

Preserve the strong generator crate boundary while reducing procedural
concentration and raw terminal output.

- Split `gen/src/emit.rs` by stable catalog domains: identity/paths,
  execution/prompting, models/offerings, event/support policy, and linking
  resources. Keep shared literal/import helpers small and leave
  `emit_data_file` as a thin, visibly ordered assembler.
- Split the `coerce_to_catalog_shape` decision tree along the same domain
  vocabulary so registry entries, coercion, and emission have predictable
  owners. Do not split the declarative registry merely because it is long.
- Introduce typed generator report data and `TerminalRenderable` renderers
  using biscuit-terminal components (`Prose`, `UnorderedList`, `Table`,
  `CodeBlock`, or status components as appropriate). Replace human-facing
  `println!`/`eprintln!` paths for generate, check, provenance, diff, prompt,
  and agent-error reports.
- Preserve raw JSON exclusively for explicit machine-facing modes such as
  mapping/structured reports, with stdout for data and stderr for diagnostics.
  Keep inherited-stdio `claudine providers generate` working.
- Refresh `lib/src/provider/mod.rs` to describe the completed generated-data /
  handwritten-behavior architecture, and update
  `docs/topics/provider-metadata.md` from the stale 18-site statement to the
  authoritative current inventory (19 at assessment time, derived rather than
  hard-coded where practical).

Acceptance gates:

- Generated `data.rs`, catalog, signals, family, and vocabulary artifacts are
  byte-identical before/after the structural split.
- Generator output tests cover stdout/stderr, `NO_COLOR`, `FORCE_COLOR`, and
  plain/non-TTY degradation; machine JSON parses without ANSI or prose.
- `cd claudine/gen && just check && just test && just lint` and `just test-gen`
  pass.
- The dispatch inventory test and generated census agree.

## Phase 11 — Delegate composition error rendering by family

Complete C6 at the actionable rendering boundary while preserving the central
typed error vocabulary.

- Keep `CompositionError` and its public variants in `error/mod.rs`.
- Divide `BlockError::status_block` rendering into focused modules for
  lifecycle, schema/frontmatter, selection/target, sequence/loop, and
  provider/execution/file-reference errors.
- Make the trait method a thin exhaustive dispatcher. Each family renderer
  returns the same `StatusBlock` and reuses shared path/link/code helpers rather
  than copying styles or prose.
- Preserve diagnostic source chains, error codes, frontmatter appendices,
  TTY/color behavior, and the CLI error walker's deepest-typed-error rule.
- Update comments only where responsibility or behavior descriptions moved.

Acceptance gates:

- Exhaustiveness remains compiler-enforced and no public error variant or code
  changes.
- Phase 1 snapshots and family-focused tests prove byte-equivalent terminal and
  plain rendering.
- `just test-library`, `just test-cli`, and `just lint` pass.

## Phase 12 — Resolve interpolation convergence and run the area-wide gate

Make the loop/lifecycle rendering relationship explicit, then close all
documentation and verification work.

- Add one shared conformance matrix for syntax supported by both loop actions
  and lifecycle actions: literal/mixed strings, whole-value typed expansion,
  arrays/objects, namespaces and missing values, functions, escaping,
  malformed expressions, and strict/fail-closed behavior.
- Compare `render_action_value`/`render_string_with_lookup` with lifecycle DM2
  `SubtreeCompose`. If the same input/state can preserve every established loop
  result, migrate loop action rendering to the Darkmatter substrate and remove
  the duplicate renderer. If a required semantic difference remains, keep the
  smaller implementation, document the exact differences and rationale in the
  composition architecture, and require both engines to pass the shared
  overlap matrix. This is an explicit evidence gate, not an open-ended design
  choice.
- Update all READMEs, Claudine skill pages, topic docs, and symbol comments
  affected by the 12 phases. Treat code and generated inventories as authority
  where old prose drifted.
- Rerun `hug god-files --json claudine` and report actionable production
  changes separately from generated/test files; success is clearer ownership
  and state-machine reviewability, not an indiscriminate line-count target.
- Run GitNexus `detect_changes` against `main` and inspect every affected
  execution flow before any commit. Re-run impact analysis for each changed
  public/shared symbol whose callers changed during implementation.

Final acceptance gates:

- `just sanity`, `just lint`, `just doctest`, `just test`, `just
  test-rendezvous`, `just signals-check`, and `just test-l2` pass from the
  Claudine area; `cargo fmt --check` is diagnostic only and no write-mode
  formatting is run.
- `cargo check --workspace --all-targets` passes on macOS and Linux CI, and the
  Windows workspace/all-targets plus Ctrl+C gates pass on a Windows host.
- The test-placement guard has zero unapproved violations, generator and
  dispatch drift checks are clean, and no cited documentation drift remains.
- The final diff contains no generated-file hand edits, unrelated cleanup,
  accidental formatting churn, or changes to the areas explicitly excluded
  from line-count-driven splitting.
