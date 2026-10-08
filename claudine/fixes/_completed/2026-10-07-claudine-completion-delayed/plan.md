---
kind: plan
created: 2026-10-07
total_phases: 5
phase: 5
agent: codex/gpt-6.1-sol
yolo: true
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/fixes/2026-10-07-claudine-completion-delayed/plan.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/spec.md
docs_created_during_phase_1:
  - claudine/fixes/2026-10-07-claudine-completion-delayed/implementation-log.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - Cargo.lock
  - claudine/cli/Cargo.toml
  - claudine/cli/src/commands/wrap/exec/completion_fixture.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/reader_join.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/provider_streams.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/tests.rs
  - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
  - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
  - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/src/commands/wrap/exec/spawn/tests/captured.rs
  - claudine/cli/src/commands/wrap/exec/spawn/tests/inherited.rs
  - claudine/cli/src/commands/wrap/exec/stream_capture.rs
  - claudine/cli/src/commands/wrap/exec/wiring/session.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
  - claudine/cli/src/commands/wrap/output_worker.rs
  - claudine/cli/src/commands/wrap/output_worker/tests.rs
  - claudine/cli/src/commands/wrap/run_scope.rs
  - claudine/cli/src/commands/wrap/run_scope/observation.rs
  - claudine/cli/src/commands/wrap/run_scope/retention.rs
  - claudine/cli/src/commands/wrap/run_scope/tests.rs
  - claudine/cli/src/commands/wrap/stream_io.rs
  - claudine/cli/src/main.rs
  - claudine/cli/tests/bin/fake_completion/main.rs
  - claudine/cli/tests/l1/completion_delayed.rs
  - claudine/cli/tests/l1/context_construction_guard.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/claude/tests.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/codex/tests.rs
  - claudine/lib/src/stream/semantic.rs
  - claudine/lib/src/stream/semantic/tests.rs
docs_updated_during_phase_2:
  - claudine/docs/dependencies.md
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/topics/timeouts.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/implementation-log.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/plan.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/spec.md
  - docs/dependencies.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_3:
  - claudine/cli/src/commands/wrap/exec/reader_join.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/matrix.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/provider_streams.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/tests.rs
docs_updated_during_phase_3:
  - claudine/docs/topics/timeouts.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/implementation-log.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/plan.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/spec.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
  - .claude/skills/claudine/SKILL.md
packages:
  - claudine
  - claudine-cli
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/tests/l1/completion_delayed.rs
docs_updated_during_phase_4:
  - claudine/README.md
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/timeouts.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/implementation-log.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/plan.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/spec.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/flow-control/lifecycle.md
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/timeouts.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/implementation-log.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/plan.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/spec.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/SKILL.md
source_code:
  - Cargo.lock
  - claudine/cli/Cargo.toml
  - claudine/cli/src/commands/wrap/exec/completion_fixture.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/reader_join.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/matrix.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/provider_streams.rs
  - claudine/cli/src/commands/wrap/exec/reader_join/tests.rs
  - claudine/cli/src/commands/wrap/exec/spawn/captured.rs
  - claudine/cli/src/commands/wrap/exec/spawn/inherited.rs
  - claudine/cli/src/commands/wrap/exec/spawn/retained.rs
  - claudine/cli/src/commands/wrap/exec/spawn/semantic.rs
  - claudine/cli/src/commands/wrap/exec/spawn/tests/captured.rs
  - claudine/cli/src/commands/wrap/exec/spawn/tests/inherited.rs
  - claudine/cli/src/commands/wrap/exec/stream_capture.rs
  - claudine/cli/src/commands/wrap/exec/wiring/session.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/output_worker.rs
  - claudine/cli/src/commands/wrap/output_worker/tests.rs
  - claudine/cli/src/commands/wrap/run_scope.rs
  - claudine/cli/src/commands/wrap/run_scope/observation.rs
  - claudine/cli/src/commands/wrap/run_scope/retention.rs
  - claudine/cli/src/commands/wrap/run_scope/tests.rs
  - claudine/cli/src/commands/wrap/stream_io.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/src/main.rs
  - claudine/cli/tests/bin/fake_completion/main.rs
  - claudine/cli/tests/l1/completion_delayed.rs
  - claudine/cli/tests/l1/context_construction_guard.rs
  - claudine/cli/tests/l1/main.rs
  - claudine/lib/src/stream/providers/claude.rs
  - claudine/lib/src/stream/providers/claude/tests.rs
  - claudine/lib/src/stream/providers/codex.rs
  - claudine/lib/src/stream/providers/codex/tests.rs
  - claudine/lib/src/stream/semantic.rs
  - claudine/lib/src/stream/semantic/tests.rs
documentation:
  - claudine/README.md
  - claudine/docs/dependencies.md
  - claudine/docs/providers/dispatch-inventory.json
  - claudine/docs/topics/composition.md
  - claudine/docs/topics/flow-control/lifecycle.md
  - claudine/docs/topics/non-interactive-sessions.md
  - claudine/docs/topics/timeouts.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/implementation-log.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/plan.md
  - claudine/fixes/2026-10-07-claudine-completion-delayed/spec.md
  - docs/dependencies.md
implemented: false
completed_phase: 2
---

# Claudine completion delay implementation plan

## Phase 1 — Scope, success, and rulings

Implement the follow-up to `2026-10-06-stream-reader-join-timeout` by extending
the existing reader settlement and run observation seams. Preserve confirmed
provider outcomes, identify unconfirmed completion with a stable diagnostic,
retain available response data before blocking callbacks, and carry a frozen,
populated diagnostic through session storage and lifecycle handling. Add
shipped-CLI caller regressions and update the current behavior documentation.

Success means Claude and Codex success chains execute their next command even
when Claudine cleanup stalls after the verdict. Genuine failures remain
failures; native exit 0 without a verdict returns caller exit 1 and
`timeout.claudine_completion_delayed`. A handler can read retained data with
honest completeness flags, including during a result-line callback stall.
Blocked terminal delivery cannot prevent storage of the diagnostic or cause
duplicate session/lifecycle completion. Existing deadlines, output ordering,
process teardown, and recovery semantics remain intact. Representative real
Claude and Codex runs provide separate observations after installation.

### Necessary Rules

1. **CPU decision remains with the author.** Recommend specification Open
   Question 1 option 1: omit `cpu_observation` entirely. Static CPU facts cannot
   answer the question, and an always-null registry field is a lasting API
   commitment. This plan does not claim the recommendation is ratified. Record
   the author's ruling before locking the new registry fields; independent
   observation and fixture work can proceed. Do not add a sampler or expand
   `sniff` under this fix. If the author selects option 2, settle the exact
   unavailable-value shape before implementing it. Option 3 requires a scope
   revision rather than an implicit expansion of this plan.
2. **Inline retention decision:** cap retained answer text at 256 KiB and
   retained raw stdout at 256 KiB per run, separately, measured in source
   bytes. Keep a prefix rather than a rolling tail; clip text at a UTF-8
   boundary. Preallocate bounded observation buffers, avoid repeated full
   transcript copies, and account for frozen snapshot copies and JSON escaping
   when validating the persisted-size bound. These are diagnostic limits,
   not new limits on normal parser output or successful answers.
3. **Overflow is visible.** An inline-clipped answer has
   `response_complete: false`, even when the source answer was complete.
   `raw_output_truncated` records omission of observed bytes. Without a complete
   artifact, `raw_output_complete` is false on overflow. With an existing,
   verified complete capture, expose its path and document that
   `raw_output_complete` describes the combined inline/artifact representation;
   `raw_output_truncated` still describes inline retention. A full raw protocol
   artifact does not make a clipped extracted answer complete. Unknown
   completeness is null; an empty identified answer is distinct from no answer.
4. **Raw bytes stay raw.** Retain stdout before decoding/provider parsing,
   including record delimiters. Use a string for valid UTF-8; for invalid UTF-8,
   use a tagged lossless representation in `raw_output` with `encoding` and
   `data` (base64), rather than lossy replacement or fabricated answer text.
   Freeze and document this field shape in Wave 1 before consumers implement
   it. The 256 KiB limit applies before encoding; persisted sizing includes its
   expansion. Malformed protocol content can be retained without being an
   identified answer.
5. **Capture lifetime:** reuse only the opt-in `CLAUDINE_RAW_STREAM_DIR`
   capture. Its file persists until the directory owner deletes it; Claudine
   adds no automatic retention promise or spool. A handler may read it during
   `failure`/`finalize` if it still exists. Publish its path only after creation
   is known; a buffered or failed flush cannot establish complete coverage.
   Do not synchronously flush or take the blocked reader's writer lock at
   cutoff. The diagnostic is immutable; an incomplete referenced artifact may
   receive late writes and must never retroactively change its frozen flags.
6. **Outcome precedence:** actual panic remains `parse_failure`; established
   provider failure, interruption, and provider timeout keep their identities.
   Native nonzero exit without a result uses `exit_failure`; interruption uses
   `interrupted`. Keep `exit_failure`'s existing diagnostic mapping behavior;
   adding a mapping is unnecessary for this fix. Only native exit 0 with no
   verdict and no stronger failure selects `claudine_completion_delayed`.
   Presentation loss after confirmed success remains subordinate. Stderr join
   timeout remains warning-only.
7. **Budget invariants:** retain one shared monotonic cleanup origin after
   teardown, the 120-second drain limit, the five-second settled-pipe cap, and
   the continuous 250 ms settle requirement. Tests inject shorter budgets;
   production receives no new public setting. Neither observations nor output
   drain reset the clock. Lifecycle deadlines remain independent.
8. **No performance spike:** the spec's six-answer and large-answer figures
   already answer the computation-cost question. Atomic operation/time stores
   with no allocation or formatting per record are a design constraint, not
   another benchmarking task. Do not repeat those measurements, saturate hosts,
   or widen the work into terminal recovery. A future wider measurement requires
   an explicit author scope decision.
9. **Input robustness scope:** this plan adds observations and diagnostic
   output, not configuration fields or provider-format acceptance rules.
   Keep populated detail in the existing runtime outcome/diagnostic transport
   rather than introducing a new configuration reader. The input matrix is
   therefore not applicable to that output. Wave 1 must inspect any proposed
   persisted-summary deserializer change: if new fields affect decisions when
   read back, add a per-format matrix for every such field covering control,
   absent, null, wrong whole/element types, empty, duplicate keys, and trailing
   content, with public-result assertions, before that change proceeds. Do not
   substitute serialization round trips for this reader coverage.

### Wave 1 — Contract and dependency review

- [x] **Trace boundaries**
  - Map `run_scope.rs::feed_line`, `DeferredSink`, `ReaderProgress`, semantic
    spawn wiring, output-worker state, summary settlement, session publication,
    attempt results, and `LifecycleErrorInfo::from_action_failure`.
  - Enumerate every `stream_reader_timeout` consumer with a repository text
    search, including `reader_join.rs`, its matrix and `provider_streams`,
    `policy/session_end_tests.rs`, and `harness_orch/attempt/tests.rs`.
  - Identify where an earlier answer callback can prevent a later verdict
    from being read and where result-line callbacks currently lose answer data.
    Inspect storage publication ordering relative to summary terminal output.
- [ ] **Freeze interfaces**
  - Phase 1 checkpoint: the independent interfaces and test owners are recorded
    in the implementation log; only the author's CPU ruling remains pending.
  - Record the CPU ruling and retention decisions, the immutable detail type,
    explicit unknowns, operation vocabulary, counters, and publication sequence.
    Keep provider/run/attempt identity sourced from actual launch context.
  - Choose the smallest existing typed diagnostic transport that reaches
    enclosing composition/sequence errors; retain the label-only fallback for
    unrelated kinds. Resolve any deserializer impact under rule 9.
  - Define bounded atomic operation/time observations for reader and output
    worker separately; a queued frame must not overwrite an in-progress
    terminal delivery observation. Specify coherent reads of tag/time pairs.
  - Define test-fixture-only budget/stall controls reachable by shipped-CLI
    tests, without ambient production environment knobs or startup sleeps.

The two tasks may be researched concurrently, but interface freezing depends
on the boundary inventory. Checkpoint: record decisions and exact source/test
owners in the implementation log. No spike is scheduled. Wave 2 begins after
the transport and observation interfaces are fixed; CPU-dependent registry
work waits for the author's ruling.

## Phase 2 — Bounded observation and retained data

### Wave 2 — Independent implementation tasks

- [x] **Observe operations**
  - Extend the existing run/progress seams with bounded atomic operation tags,
    monotonic offsets, and counters. Cover pipe wait/arrival, record processing,
    JSON decoding/signal observation, provider parsing, verdict publication,
    summary construction, render computation, output submission, semantic
    logging, hooks/lifecycle callbacks, join start/end, cutoff, and settlement.
  - Record queued and in-progress terminal delivery separately in the existing
    worker. Report `PipeOpen` as waiting for pipe data; remove unsupported cause
    inference from processing-timeout warning wording and associated comments.
  - Never hold observation/run-state locks across callbacks, terminal I/O, or
    storage publication. Cutoff must obtain observations without the parser.
- [x] **Retain payloads**
  - Capture bounded raw stdout before decoding and parsing. Publish identified
    answer data before deferred callbacks, including before a completion-line
    callback. Preserve provider verdict/task-ledger semantics independently.
  - Make Codex last-message fallback available as answer data only, with bounded
    reading and honest completeness; it cannot publish a verdict. Do not copy
    the full growing summary solely to retain each text delta.
  - Share capture existence/coverage facts without joining the reader or
    accessing its blocked buffered writer. Preserve the existing capture
    envelope format and conservative completeness guarantees.
  - Freeze retained data at settlement with a race-safe close/publication
    boundary. Release late readers in tests and prove they cannot alter detail,
    emit into the next run, or repeat lifecycle/session completion.
- [x] **Build fixtures**
  - Prepare portable fake Claude/Codex scripts or compiled stubs using
    `CliProcessFixture` and test-toolkit. Add handshakes for a verdict callback
    stall, an earlier answer callback stall, and unfinished output delivery.
  - Add shell-chain helpers for POSIX shells and native Windows `cmd.exe` that
    capture Claudine's exit separately from the marker command's exit.
  - Use fixture-owned homes/CWD/PATH, child-local silent audio defaults, and
    isolated output files. Load the `os` skill before Windows or path changes.

Parallel ownership: operation/progress/output-worker code, retention/snapshot
code, and CLI fixture files are separate assignments. Agree shared
`run_scope.rs` edits in Wave 1; one owner integrates that file. Coordinate
builds rather than launching overlapping broad Cargo runs.

### Wave 3 — Observation validation

- [x] **Verify snapshots**
  - Add focused L1 tests for every observed boundary, queued versus active
    delivery, pipe-held-open state, shared elapsed clock, and frozen snapshots.
  - Test complete, partial, unavailable, empty, overflowed, multibyte-boundary,
    malformed/raw-byte, capture-disabled, unflushed, failed-flush, and verified
    complete-artifact cases. Assert public detail and persisted size bounds.
  - Assert answer retention during the completion record's blocked callback
    and during an earlier answer callback before any verdict is available.
  - Reuse existing blocked-sink/output-worker tests for output ordering and
    bounded delivery; add assertions only for newly observable behavior.

Checkpoint: controlled stalls return a coherent bounded snapshot before the
reader is released. No verdict is inferred from text, a file, or operation
state. Wave 4 depends on this checkpoint.

## Phase 3 — Outcome selection and diagnostic propagation

### Wave 4 — Settlement and diagnostic tasks

- [x] **Select outcomes**
  - Update stdout reader settlement to the rule 6 precedence and preserve
    confirmed summaries/native exit. Attach reader/output observations to
    stronger outcomes without replacing their primary identity.
  - Keep `stream_reader_timeout` only as warning/subordinate observation on
    stdout; retain stderr warning behavior. Update all consumers inventoried
    in Wave 1 and audit relevant behavioral comments.
  - Preserve early termination overrides, structured-provider verdict checks,
    incomplete subagents, and bounded process-tree teardown.
- [ ] **Carry diagnostics**
  - Implement the typed `ClaudineCompletionDelayed` diagnostic through the
    existing `Diagnostic`/`BlockError` architecture. Register
    `timeout.claudine_completion_delayed` as timeout/transient/internal with
    default warning severity and add the error-kind mapping.
  - Include `provider`, `exit_code`, `elapsed_ms`, `limit_ms`,
    `verdict_received`, `response_text`, `response_complete`, `raw_output`,
    `raw_output_complete`, `raw_output_truncated`, `raw_output_path`, and
    `observed_operation`; apply the recorded CPU ruling. Unknowns are null.
  - Carry populated frozen detail from settlement through the runtime outcome,
    session record, attempt status, enclosing errors, and lifecycle context.
    Prefer that snapshot over catalog fields synthesized from a label.
  - Publish the diagnostic using existing run/session storage before any
    potentially blocked summary presentation and before lifecycle completion.
    Keep response text out of concise `err.msg` and notification messages.
  - Update registry/catalog introspection and `claudine errors` coverage.

These tasks can proceed concurrently against the frozen interfaces, with one
owner integrating shared settlement/outcome types. Preserve the label-only
path for unrelated diagnostics; do not backfill other timeout details.

### Wave 5 — Integrated outcome validation

- [ ] **Verify propagation**
  - Update existing reader matrix, provider-stream, session-end, and attempt
    helpers to assert the new primary identities and subordinate observations.
  - Assert native exit 0/no verdict gives caller failure; native nonzero/no
    result gives `exit_failure`; successful verdict/native exit 0 stays success.
    Cover real panic payloads, interruption, provider timeout, genuine provider
    errors, and incomplete-subagent semantic failure with native exit 0.
  - Assert a lifecycle handler receives non-null known detail values, matching
    machine/session diagnostics, through direct composition and sequence
    wrapping. Verify diagnostic facets and error-code catalog output.
  - Block terminal delivery and prove storage publication still occurs exactly
    once; distinguish native exit from attempt/caller outcome in assertions.

Checkpoint: one selected primary condition survives every transport boundary;
success with presentation loss never fires `failure`. Wave 6 depends on this
checkpoint and the fixtures from Wave 2.

## Phase 4 — Caller and author behavior

### Wave 6 — Parallel regression suites

- [ ] **Prove caller exits**
  - [x] For Claude and Codex, execute the shipped CLI in a real shell chain with
    native exit 0, nonempty answer, and a parsed successful verdict. Stall a
    completion callback and, separately, terminal delivery under short budgets.
    Assert Claudine exit 0 and the subsequent marker exists.
  - Use the same fixture to prove missing verdict, provider error, native
    nonzero exit, real reader panic, interruption, and provider timeout produce
    their expected caller outcomes and prevent the marker where appropriate.
  - Assert text-only and Codex last-message-only controls fail honestly.
    Exercise native Windows as well as POSIX shell forms; these are L1 tests,
    not emulator tests or duplicates of the helper matrix.
- [ ] **Prove author policies**
  - Create a complete sequence fixture with `fail_fast: false`; the delayed
    step's `failure` handler reads retained response detail, the next step runs,
    and the original step remains failed in the summary. Printing/storing data
    must not repair it.
  - Verify default `fail_fast: true` halts later work, explicit successful
    recovery follows existing semantics, and `finalize` runs exactly once.
    Cover failure/finalize access to the same frozen snapshot.
  - Reuse existing retry/resume/proxy and loop-policy tests; add coverage at the
    new diagnostic seam where needed. Do not add directives or change budgets.

**Partial caller checkpoint:** confirmed-success callback/delivery stalls and
native-exit-0/no-verdict callback stalls are verified for Claude and Codex.
The remaining caller controls and author-policy suite are still open; Phase 3's
populated diagnostic transport awaits the author's CPU-field ruling.

Checkpoint: both portable suites pass using synchronization, with no
120-second waits, startup sleeps, focused windows, or real-provider CI
dependencies. Wave 7 starts after these assertions pass.

## Phase 5 — Documentation, verification, and real observations

### Wave 7 — Parallel review and documentation

- [ ] **Document behavior**
  - [x] Update `docs/topics/timeouts.md`, including nonzero-exit outcome tables,
    operation observations, retention limits/flags, artifact lifetime, and the
    unresolved cause and terminal-visibility limitations.
  - Update lifecycle, composition, and non-interactive-session topic pages with
    a runnable `fail_fast: false` sequence example that reads response detail
    before choosing recovery. Explain that transient retry may repeat provider
    side effects already completed. Include a compact flow diagram if useful.
  - Remove implemented planned markers, update affected README descriptions,
    diagnostic catalog guidance, and the claudine skill where architecture or
    workflow changed. Current topic docs must not name/link this fix directory.
- [x] **Review invariants**
  - [x] Audit the combined change for coherent atomic observations, bounded memory
    and persisted data, freeze races, UTF-8/raw-byte handling, callback ordering,
    stronger-outcome precedence, provider identity, and exactly-once storage.
  - [x] Confirm no new settings, samplers, spool services, terminal writers,
    deadline extensions, protocol-acceptance changes, or new CI gates.
    Check relevant comments for drift and rule 9's matrix applicability.

**Phase 5 review checkpoint:** the implemented retention, settlement, and
caller paths have been audited and pass local gates plus focused Linux checks.
The audit confirms that populated diagnostic publication/transport is missing;
it does not establish exactly-once storage for that unfinished behavior.
Documentation retains planned markers for it. Native Windows/WSL evidence,
the runnable author example, installation, and live-provider observation remain
open behind the missing earlier checkpoints and author ruling.

### Wave 8 — Final validation and evidence

- [ ] **Run validation**
  - [x] From `claudine/`, run focused nextest-backed `just test-cli` and
    `just test-library` filters during development, then the relevant area
    `just test` and `just lint` after integration. Record commands and results;
    reuse passing evidence if subsequent edits do not affect it.
  - Read the `os` skill and use the established cross-check/evidence routes
    for macOS, Linux, native Windows, and WSL2. Prove the synthetic observations,
    retained data, diagnostic propagation, and portable caller suite on each;
    record executed versus skipped tests and any missing evidence explicitly.
  - Run `just test-l2` only if implementation introduces a question requiring
    a real terminal. Keep terminal/browser windows unfocused. Do not add CI
    environments, run `cargo fmt`, commit, or move the fix to `_completed`.
- [ ] **Observe real runs**
  - After passing local validation, install with the normal `just install`
    recipe and observe representative non-interactive Claude and Codex runs
    from disposable workspaces without changing window focus.
  - Record tested revision plus uncommitted build identity, provider versions,
    native/caller exits, completion observations, and session identity. Keep
    this evidence separate from hermetic tests; recurrence is not required.
  - If a delay recurs, preserve the frozen diagnostic and retained artifact
    references, distinguish the outstanding operation from inferred cause,
    and record remaining uncertainty before proposing broader changes.
  - Record implementation departures and validation limitations in the
    implementation log. Terminal state: implementation complete, ready for
    review; lifecycle completion/movement belongs to the author.

Run validation before installation and real observation; these tasks are
sequential. Final acceptance requires all earlier checkpoints and an evidence
record that distinguishes passing checks, skips, unavailable environments,
and observed real-run behavior without claiming a future failure probability.
