---
spec: "/Volumes/coding/wt/rusty-biscuit/fix-wt-message/claudine/fixes/2026-10-07-claudine-completion-delayed/spec.md"
plan: "claudine/fixes/2026-10-07-claudine-completion-delayed/plan.md"
implemented_by: codex/gpt-6.1-sol
started_phase: 1
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

# Implementation Log for 2026-10-07-claudine-completion-delayed (5 phases)

## Phase 1

### Scope and starting state

Phase 1 is Wave 1's boundary inventory and interface review, not the runtime
implementation assigned to Phases 2–4. No source behavior, configuration,
provider acceptance rules, dependencies, or shipped artifacts change here.
Consequently there are no new regression tests to turn red in this phase;
the requirement-to-test map below is the implementation prerequisite for the
subsequent phases. Existing tests are baseline evidence, not proof that the
planned behavior already works. No package manifests are touched (`packages: []`).

Read the Claudine and rust-testing skills, including test-design guidance.
The requested `@claudine/claudine/...` plan reference has an extra directory;
the supplied spec and actual plan both live under `claudine/fixes/`.

At entry the plan and empty implementation log were untracked; the spec,
Claudine skill, and OS skill's `macos.md` already had local changes. Preserve
those changes. This phase only edits the plan, spec metadata, and this log.
No formatting, staging, commits, installation, lifecycle-directory movement,
benchmarking, or real-provider runs are part of this phase.

### Boundary inventory (Trace boundaries complete)

Paths below are repository-relative. Owners are files, not additional agents;
this headless run performs the review directly.

| Boundary and owner | Current behavior and required seam |
| --- | --- |
| `claudine/cli/src/commands/wrap/run_scope.rs` — `feed_line`, `DeferredSink`, `ResultSnapshot` | A thread-local queue defers semantic sink events until `parser.feed_line` returns. TurnComplete/terminal Error marks `reported`; parser snapshot then publishes before deferred callbacks. Scope close is an atomic flag, but `update` does not check it under the snapshot mutex. Replace the separate close/read with one freeze boundary for new retained detail. |
| `claudine/cli/src/commands/wrap/exec/reader_join.rs` — `ReaderProgress`, `join_reader`, `settle_parser` | Packed atomic waiting-time state distinguishes pipe wait, processing, and EOF. Joining shares the post-teardown origin and preserves a handed-back parser or published summary. Empty slot/no summary still synthesizes `stream_reader_timeout`, including native nonzero exits. Panic payload remains distinct. |
| `claudine/cli/src/commands/wrap/exec/spawn/semantic.rs` | Creates scope/progress/parser slot; stdout uses `BufRead::lines`, then tail ring, heartbeat, buffered capture, JSON signal observation, and `feed_line`. Raw bytes/delimiters are already lost before this loop. Child wait and process-tree teardown precede the shared cleanup origin; stdout settlement, scope close, stderr join, output drain, `parser.finish`, early-termination override, and stderr finalizer follow. Keep the launch provider and native exit separate from caller failure. |
| `claudine/cli/src/commands/wrap/policy.rs` — `build_structured_plumbing` | Installs DeferredSink around LiveSemanticSink; direct text/reasoning callbacks also defer. Instrument rendering, semantic logging, and hooks at their actual call boundaries, not just the enclosing parser call. |
| `claudine/cli/src/commands/wrap/live_semantic_sink/{event_sink,mod}.rs` | Semantic callback work dispatches rendering, event recording, and hooks. It must not own the observation mutex while dispatching. |
| `claudine/cli/src/commands/wrap/output_worker.rs`, `stream_io.rs`, `claudine/cli/src/terminal_gate.rs` | One lazy bounded worker owns terminal writes. Queue.writing records an active write separately from queued frames. Drain disables delivery at the shared deadline; terminal gate prevents another writer. Add separate atomic active-delivery observations; submit cannot overwrite them. Preserve existing loss counters. |
| `claudine/cli/src/commands/wrap/exec/stream_capture.rs` | Opt-in NDJSON envelope `{ts_ms, raw}` and sidecar; drops delimiters and skips whitespace-only lines; reader owns BufWriter and only Drop flushes. Path existence and complete coverage are different facts. Existing capture cannot automatically prove byte-complete raw output, even after flush. |
| `claudine/cli/src/commands/wrap/policy.rs` — `emit_stream_summary_inner`; `claudine/lib/src/stream/reporting.rs` | Summary rendering/submission and final drain currently precede synthetic SessionEnd JSONL publication. Move publication before trailer presentation without duplicating SessionEnd. Compute final delivery-loss facts before publication using the bounded cleanup drain. |
| `claudine/cli/src/commands/wrap/harness_orch/attempt.rs`; `claudine/lib/src/harness/{model,runtime}.rs` | AttemptOutcome carries label/message and OutputStatus, not populated diagnostic detail. CLI assembles it manually; build_attempt_outcome is a second construction seam. Both need the same optional owned snapshot. |
| `claudine/cli/src/commands/wrap/harness_orch/loop_control.rs` | Failure classification preserves semantic errors at native exit 0, then lifecycle err is reconstructed through from_action_failure. Prefer the attempt's populated snapshot; retain this fallback for unrelated kinds. Interruption has a separate early branch. |
| `claudine/lib/src/composition/lifecycle/context.rs` | LifecycleErrorInfo already embeds DiagnosticSnapshot. from_action_failure uses from_code, which fills catalog fields with null. Add a snapshot-taking constructor rather than reconstructing retained detail from a label. |
| `claudine/lib/src/diagnostics/{snapshot,error_kind,registry}.rs` and `composition/error/mod.rs` | DiagnosticSnapshot is the existing owned serialization boundary; detail is opaque JSON and facets are strings. Reuse it. LoopIterationFailed and SequenceTaskPromptLaunch already have boxed snapshot fields; preserve selected identity through them. |
| `claudine/cli/src/commands/compose/loop_run.rs`; `commands/wrap/sequence/{iterate,task_run}.rs` | Provider-exited loop errors currently use snapshot: None; sequence has snapshot-aware result fields but nonzero result routes can omit it. Carry the same runtime snapshot rather than infer it from exit_reason. |

**Answer callback ordering finding:** the predecessor's deferred result-line
answer gap described in the spec is already repaired in the working code:
`feed_line` publishes the full parser snapshot before completion-line sink
work. `provider_streams::a_reader_stalled_in_a_completion_callback_keeps_every_providers_answer_and_verdict`
asserts answer/session/usage/verdict against an unblocked production parser.
Do not regress or reimplement this protection. An earlier answer callback can
still block before the next result record is read; `reported` is false then,
so no summary is published. Bounded answer retention before that callback is
still necessary. Record this difference here; leave the spec snapshot intact.

**Timeout-label inventory:** repository-wide `rg -n stream_reader_timeout`
found executable label selection in `exec/reader_join.rs`, assertions in
`exec/reader_join/{tests,provider_streams,matrix}.rs`, and current-behavior
rows in `claudine/docs/topics/timeouts.md`. There is no diagnostic mapping.
`policy/session_end_tests.rs::reader_timeout_warning` and
`harness_orch/attempt/tests.rs` consume timeout warning text indirectly and
must be updated when wording changes. Other literal matches are this plan,
this spec, and predecessor fix plans/logs/specs; historical snapshots must
not be rewritten. Outcome and warning identity are separate responsibilities.

### Interface decisions (CPU ruling still pending)

**Author ruling:** none is recorded. Phase 1 cannot ratify omission of
`cpu_observation` on the author's behalf: Necessary Rule 1 explicitly reserves
that decision. Recommend option 1 (omit it); do not add a sampler or register
a permanently-null field. Spec metadata records a human review item before
Phase 2 and a handoff message. Independent observation/retention/fixture work
is designed below; final registry field locking remains blocked on this ruling.
The Freeze interfaces checkbox stays open solely for that unresolved decision.

**Detail owner and transport:** add a library-owned typed
`CompletionDelayedDetail` and `ClaudineCompletionDelayed` in
`claudine/lib/src/diagnostics/completion_delayed.rs` (future source).
Implement Diagnostic/BlockError using the existing discovery/selection seam;
project once to DiagnosticSnapshot at settlement. StreamExecutionSummary and
AttemptOutcome gain optional populated diagnostic snapshots; OutputStatus
continues to hold presentation warnings, never the primary failure. Use the
existing snapshot fields in enclosing errors and sequence results. Prefer the
snapshot in LifecycleErrorInfo over from_action_failure; no backfill of other
timeout codes. Retained fields are public output, never executable instructions.

| Detail field | Frozen representation |
| --- | --- |
| provider | Actual launch Provider slug; no default Claude identity |
| exit_code | Observed native child i32 exit status; caller exit remains separate |
| elapsed_ms, limit_ms | Integer milliseconds from shared cleanup origin and its unchanged budget |
| verdict_received | Boolean; false when this diagnostic is primary |
| response_text | Optional original UTF-8 answer prefix; null if unidentified; empty string if an identified empty answer |
| response_complete | Optional bool; null if unknown, false if partial or clipped, true only for a known complete inline answer |
| raw_output | Null if unavailable; UTF-8 string if valid; otherwise `{ "encoding": "base64", "data": "..." }`, RFC 4648 standard padded base64 of retained bytes |
| raw_output_complete | Optional bool; null if unknown; true only for complete inline or verified complete combined inline/artifact coverage |
| raw_output_truncated | Boolean for omission of already observed bytes from inline retention |
| raw_output_path | Optional path to an actually created opt-in capture; null otherwise |
| observed_operation | Object with reader, output_worker, and settlement lanes; each contains operation and monotonic at_ms, plus bounded counters; unavailable lane is null |

Each registered detail key is present, including explicit nulls. CPU is not
part of this provisional table until the author rules. Snapshot message is
concise cleanup-expired/unconfirmed-verdict wording and contains no response.

**Retention and freeze:** preallocate separate 256 KiB source-byte prefixes
for answer and raw stdout per run. Append deltas/replace a final authoritative
answer in-place; never repeatedly clone the growing full summary for retention.
UTF-8 answer clips at a character boundary. Raw bytes retain exact bytes,
including delimiters, before decoding; use a bounded read adapter at the
underlying Read boundary so malformed UTF-8 remains observable without
changing parser acceptance. Cover managed-stdio sources as well as ordinary
child stdout; no additional protocol reader thread. Normal parser answers
are not capped by these diagnostic limits. A complete Codex last-message file
is bounded answer-only fallback, never a verdict; read limit + 1 byte to detect
overflow and retain the prefix with honest completeness.

Keep a closed flag inside the retained-state mutex. Publication checks it
under the same lock; freeze closes, moves the bounded buffers and capture
facts out once, and releases the lock before JSON projection or storage.
Late writes are discarded. Existing run-scope closed gating still rejects
late output/events; test admission races explicitly. No locks span parser,
callback, terminal, capture writer, or storage operations. Atomic observation
lanes freeze into the owned detail only once; late activity cannot mutate it.

Bound per-instance detail serialization by 6 * (256 KiB + 256 KiB) + 64 KiB
= 3,211,264 bytes (worst-case JSON escaping plus bounded observation/path
metadata); base64 expansion is smaller than the 6x raw-byte bound. Tests must
also account for enclosing record fields separately rather than call this a
bound on the uncapped normal successful response. Share the frozen payload
where possible, avoid retaining mutable and frozen buffer duplicates, and
include each necessary snapshot copy in memory-bound assertions. Paths are
existing filesystem paths; cap optional metadata serialized into detail so
it cannot consume an unbounded remainder.

Capture files persist until their directory owner deletes them; handlers may
read existing files during failure/finalize. Share creation, observed coverage,
omissions, read EOF, write error, and successful flush facts independently of
the blocked writer. Never flush or lock it at cutoff. Existing whitespace
omissions, unrepresented delimiter variants, invalid UTF-8, missing EOF, or
failed/unconfirmed flush prevent byte-complete artifact claims. Preserve the
capture envelope; its existence alone does not upgrade completeness. An
incomplete file may receive late writes, but its frozen flags never change.

**Atomic observations:** use single AtomicU64 packed operation/time per lane,
with 8 tag bits and 56 monotonic microsecond-offset bits from the run origin;
saturate offsets instead of wrapping. Release store/acquire load yields one
coherent pair without a retry loop, lock, allocation, or formatting per record.
No claim of one globally simultaneous reading across independently sampled
lanes. Stdout, stderr, and settlement have separate single-writer lanes; the worker's
active delivery is written only by its worker. Queue counts/bytes and submitted,
delivered, failed, dropped, rejected counters are separate bounded atomic
values (saturating where necessary), not active-operation tags.

Reader vocabulary: pipe_wait, bytes_arrived, record_processing, json_decode,
signal_observation, provider_parse, verdict_publication, answer_publication,
summary_construction, render_computation, output_submission, semantic_logging,
hook_callback, lifecycle_callback, eof. Settlement vocabulary: join_start,
join_end, cutoff, settlement, storage_publication. Worker vocabulary: idle,
terminal_delivery, delivery_complete, delivery_failed, disabled; abandonment
belongs to settlement and must not erase an outstanding terminal_delivery.
Offsets are run-relative; cleanup elapsed is separately measured from the one
post-teardown origin. PipeOpen means waiting for data, not proof of which
process owns the pipe. Processing means the observed operation, not an
inferred terminal or CPU cause. Existing 120 s / 5 s / 250 ms bounds remain.

**Publication sequence:** child exit → bounded teardown → establish one
cleanup origin → join readers against it → freeze retained observations →
finalize parser and apply stronger-outcome overrides → select one primary
condition and project snapshot → drain already queued output within remaining
shared budget → freeze final loss counters → publish exactly one SessionEnd
through existing JSONL storage → submit trailer/status → finalize attempt and
run failure/success/finalize lifecycle. Trailer-only presentation loss remains
subordinate: expose through existing attempt OutputStatus without appending a
second SessionEnd or mutating the frozen diagnostic. Pre-publication drain
handles an existing blockage; publication never writes the terminal. Final
trailer drain consumes only remaining time. Preserve early termination,
interruption, task-ledger errors, panic payloads, and stderr warning-only policy.

**Persisted-reader impact:** no new decision-bearing deserializer is needed.
AttemptOutcome is Clone/Debug, not Deserialize. Reuse DiagnosticSnapshot's
opaque detail and existing EventMeta extra JSON for storage and display.
Runtime outcome selects failure before serialization; loops/sequences should
receive the snapshot from the runtime result, not parse JSONL into a new
configuration decision. Existing optional error snapshots already model this
boundary. If implementation instead adds a read-back field used for outcome
selection, stop that design and supply the full rule 9 per-format matrix.
Read/write/read twice tests opaque snapshot preservation; they do not claim
to satisfy a configuration-input matrix. No parser/schema/template changes
or shipped-artifact corpus additions are required in Phase 1.

**Fixture-only controls:** reuse `claudine-cli/test-fixtures`, already enabled
by local recipes and CI metadata. Add `exec/completion_fixture.rs` behind that
feature to consume a child-only `CLAUDINE_TEST_COMPLETION_FIXTURE` control-file
path. Production default/install builds have no reader for it. Configure
ReaderBudget and stall point once at startup; invalid test controls fail
explicitly rather than defaulting silently. This is test control input: its
parser must receive invalid/missing/null/type/duplicate/trailing tests if a
structured format is used. Prefer a fixed enum scenario plus fixture directory
with fixed handshake filenames to avoid a new configuration grammar.

A compiled Claude/Codex provider stub (extend fake_codex or add one gated
fixture binary) emits literal real-protocol records, exits with the selected
native status, and records launch/prompt. Hooks pause only after a readiness
handshake at completion callback, earlier answer callback, or FrameSink write;
parent releases with a fixture-owned file/pipe and bounded condition polling.
No startup sleeps, ambient production knobs, second terminal writer, or
120-second test waits. Always release/terminate children and blocked workers
on failure. Shell-chain tests use CliProcessFixture.apply_policy_to on the
shell process, quoting literal paths with platform helpers. Capture Claudine's
exit to its own file before conditionally executing the marker command, so a
marker exit cannot conceal the wrapper status. POSIX sh and native cmd.exe
are separate platform forms of the same L1 promise; load the OS skill before
implementing Windows branches.

### Requirement-to-test map for subsequent implementation

No tests are added or renamed in Phase 1. The following are concrete test
owners and assertions to write or extend before their implementation changes.
All synthetic tests are L1; no emulator, focused window, or real provider is
needed. The historical live stall's exact bytes are unavailable; do not invent
them. Preserve the original report text “Stream parser thread panicked” and
use the existing literal Claude/Codex protocol fixtures for reproducible stalls.

| Requirement | Concrete tests and observable assertions |
| --- | --- |
| Preserve confirmed outcomes and precedence | Extend `exec/reader_join/matrix.rs::a_reader_stalled_before_handing_back_its_parser_never_turns_a_run_into_success` and `tests.rs::a_published_successful_summary_does_not_turn_a_nonzero_exit_into_success`: no verdict/exit 0 selects delayed completion; exit 2 selects exit_failure; interrupted 130 remains interrupted; real panic keeps payload; timeout and provider/task-ledger failures outrank delay. Assert native exit, is_error, selected kind, warning, diagnostic. |
| Completion-line answer and earlier answer stall | Keep `provider_streams.rs::a_reader_stalled_in_a_completion_callback_keeps_every_providers_answer_and_verdict`; add `an_answer_callback_before_the_verdict_retains_partial_text_without_success`. Block on a handshake before later verdict; assert retained text/raw, completeness, session record and caller failure before releasing. |
| Coherent observations and clock | Add run_scope tests `operation_and_time_are_one_coherent_observation` and `freezing_retention_rejects_late_publication`; extend reader join shared-clock/held-pipe tests; output_worker `queued_frames_do_not_replace_active_delivery`. Assert public frozen detail names the exact blocked lane and counters; transitions do not reset elapsed time. |
| Bounded honest data | Add `completion_delayed` library tests `retention_shapes_survive_two_round_trips` and `retained_detail_has_a_bounded_serialized_size`: table of absent/empty, complete/partial, overflow, multibyte edge, invalid UTF-8, malformed protocol, no capture, unflushed/failed flush, verified complete artifact. Assert detail after two JSON write/read cycles, downstream completeness and source-byte caps. No answer inferred from raw text. |
| Catalog and transport | Extend diagnostics registry/error-kind tests, lifecycle/context tests and CLI `errors` tests: timeout/transient/internal/warning facets and non-null known fields survive direct composition and sequence wrappers unchanged; unrelated label-only fallback remains unchanged. |
| Exactly-once publication before presentation | Extend `policy/session_end_tests.rs::a_stall_found_by_the_final_drain_is_on_the_session_end_record` and attempt output-loss tests. Add `diagnostic_storage_precedes_blocked_summary_presentation`: blocked sink returns boundedly; one SessionEnd exists with frozen detail before release; late readers cannot add a record or lifecycle completion. |
| Caller shell boundary | New `cli/tests/l1/completion_delayed.rs::successful_verdict_runs_the_next_shell_command` and `unconfirmed_or_failed_completion_stops_the_shell_chain`, gated test-fixtures: table over Claude/Codex, completion callback/output stall, missing verdict, genuine error, native nonzero, panic, interrupt, timeout, text-only and last-message-only. Assert separately saved wrapper/native exits, exact response/session observations, marker existence/absence. |
| Existing author policy | Same L1 module: `retained_response_does_not_repair_a_failed_sequence_step`, `default_failure_halts_the_sequence`, `explicit_recovery_finalizes_once`. Full normal compose/sequence path reads err.detail in failure/finalize; fail_fast false reaches step two while step one remains failed; default stops; explicit recovery follows existing semantics; finalize counter exactly one. |

Unit tests above compile through existing source `mod tests` and normal binary
or lib targets. New CLI L1 file must be declared in `tests/l1/main.rs` because
CLI has `autotests = false`; gate only on test-fixtures, which is present in
`[package.metadata.ci.tests] features` and local-features. Any new stub binary
needs explicit Cargo target and required-features. Test path segments must not
start with reserved tier markers accidentally. Use literal include_str or
manifest_dir/repo_root joins for repository fixtures; run
`just check-tier-coverage claudine`. Passive corpus and shipped-artifact E2E
coverage become necessary only if later implementation changes such inputs.

### Verification

Validation commands/results are recorded below as each finishes. The full
area baseline is required by the user even though this phase changes only
planning metadata. No cross-platform source or paths change; remote runs and
real-provider observations remain Phase 5 work, not claimed evidence here.

- **PASS — `cd claudine && just test`:** exit 0; Nextest run
  `2a023f82-a254-4201-b240-b53639dbef1e`; 8,482 tests passed across 12
  binaries in 98.448 s, 36 marked slow by elapsed runtime, 9 ignored tests
  skipped. No failures. Local recipes enable test-fixtures and leave daemon,
  terminal, and real-provider feature targets disabled as designed.
- **PASS — `cd claudine && just lint`:** exit 0. Includes error transport
  guards, lifecycle documentation facet guard, and Clippy with warnings denied
  for catalog-types, library, contract, CLI, and generator. Inspected `_lint`:
  this checkout's recipe does not invoke cargo fmt; no formatter ran.
- **PASS — `just check-tier-coverage claudine`:** exit 0; one area listed,
  zero stranded tests. The recipe also lists nested rendezvous packages when
  auditing the area's stubbed browser tier; it executes no additional tests.
- **PASS — matching `cargo nextest list` with the same five package selections,
  test-fixtures feature, and L1 filter:** verified the nine skipped identities
  are ignored performance tests, not missing completion regressions:
  `completion_perf::{perf_compose_empty_partial_meets_target,
  perf_compose_long_prefix_meets_target, perf_enter_compose_partial_meets_target,
  perf_inline_compose_empty_partial_meets_target}`,
  `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`, and
  `system_prompt_perf_bench::{bench_raw_darkmatter_compose_passes,
  bench_request_topology_probe_and_reuse, bench_resolve_and_prepare_step_by_step,
  bench_system_prompt_resolution_cold_and_warm}`.
- **Existing build warning:** macOS linker reports an oversized `__eh_frame`
  section for lib/CLI test binaries. Tests and lint pass; no source changes
  caused it and no workaround was introduced.
- **PASS — `git diff --check`:** no whitespace errors. Pre-existing spec/skill
  edits are preserved. No source files were created, changed, or renamed.
- **Not run:** L2/L3/browser/real tiers, installation, remote OS tests, or
  monorepo-wide tests. Phase 1 is planning only; it introduces no OS behavior,
  shipped artifacts, parser/schema inputs, or new tests requiring those gates.
  macOS baseline evidence does not stand for other operating systems.

Validation logs are local scratch artifacts:
`/tmp/claudine-completion-phase1-test.log`,
`/tmp/claudine-completion-phase1-lint.log`, and
`/tmp/claudine-completion-phase1-list.json`.

### Closing state and handoff

Boundary inventory, independent interface design, requirement-to-test mapping,
metadata, and baseline validation are complete. The sole unfinished Phase 1
task is recording the author's CPU ruling and thereby completing Freeze
interfaces. The headless session cannot obtain that ruling; it records
`human_review: true` and readable options in the spec rather than guessing.
This is an explicit plan requirement, not a tool denial or a skill-imposed
approval flow. Provide the CPU ruling before the next phase to avoid repeating
this blocker; independent work may proceed, but registry locking must wait.

No new targeted tests were added because no implementation behavior changes in
this phase. The existing result-line answer-preservation regression was
identified and passed in the baseline; the map names the new assertions and
public boundaries needed before each subsequent behavior change. Current
behavior topic pages and the Claudine skill need no Phase 1 changes because
architecture and runtime behavior are unchanged. Phases 2–5 checklist tasks
remain untouched. Phase 1 is ready for review of its design record, with the
CPU decision still required; do not report all Phase 1 tasks complete.

Final reachability check confirmed both cited completion-callback and
session-publication baseline tests are compiled in the Claudine binary test
target and selected by the L1 filter. Closing with an incomplete-phase status
because the author-reserved ruling is unavailable, rather than reporting success.

## Phase 2

### Behavior-to-test map before implementation

- Atomic operation/tag offsets and every named boundary: `run_scope::tests::operation_and_time_are_one_coherent_observation`; reader join held-pipe/shared-clock tests; operation snapshots serialize stable names.
- Earlier answer callback and result callback stalls: run-scope handshake tests and existing `provider_streams` completion callback regression; frozen response completeness and verdict remain independent.
- Raw stdout before UTF-8 decoding: retained-reader tests with exact `b"\\xff\\r\\n"`, valid/malformed JSON, delimiters, EOF, read errors, prefix overflow, UTF-8 boundaries, and bounded serialized output after two round trips.
- Freeze races: concurrent publishers released after close cannot change the snapshot or deliver another sink event.
- Capture facts: existing envelope tests plus unflushed, failed flush, omission, and completed capture observations; cutoff reads facts without the writer lock.
- Worker queue versus active write: `output_worker::tests::queued_frames_do_not_replace_active_delivery`, extending the existing gated sink test.
- Fixture invocation: compiled fake provider with portable protocol output, test-only handshake controls, and POSIX/native Windows chain helpers; tests compiled in declared L1 targets with `test-fixtures`.

These are hermetic L1 boundaries, including filesystem/subprocess checks. No parser acceptance, configuration, or shipped prompt changes are intended. Test controls use fixed scenario names rather than a structured input format. CPU contract remains pending; no registry field is locked in this phase.

### Implementation checkpoints

- Build fixtures complete: compiled `claudine-fake-completion` emits the literal Claude/Codex regression records; the declared L1 module uses CliProcessFixture and normal wrapper invocation. Fixed scenario directory names configure feature-only short budgets and ready/release handshakes. Production default/install builds cannot read the fixture control. POSIX and native cmd shell helpers save wrapper exit independently of marker execution. Three fixture tests passed (both providers and all three stall points). No terminal or browser is opened.
- Regression-first evidence: the reader warning assertion failed before its implementation change, reporting unsupported pipe-owner inference. The newly added Codex earlier-answer regression then failed because Codex deliberately emits Reasoning rather than OutputText. The parser now publishes original text through a nonblocking sink observation callback before Reasoning, with forwarding in existing sink wrappers. No provider protocol acceptance changed.
- The early-answer fixture exposed existing caller behavior: reader settlement marks the unconfirmed stream failed, but the direct wrapper currently returns its native exit 0. Phase 2 records the separate exits and checks retained data; enforcing caller exit 1 is Phase 3/4 outcome work, not silently folded into this phase. The next phase must fix that caller projection.
- Frozen data is carried in `ProcessResult.completion_observation`; no persisted-summary deserializer or registry field was added. Phase 3 must project this data into the typed library diagnostic and carry it to session/lifecycle outputs. CPU ruling remains pending.
- Capture envelopes cannot prove byte-complete coverage because delimiters are omitted. Tests exercise the generic verified-coverage conjunction, but shipped captures deliberately mark omission and cannot upgrade overflowed inline data to complete. No capture format or retention service changed.
- Raw and answer prefixes are separately capped at 256 KiB; metadata path is capped at 8 KiB. Buffers move out under the close/publication lock, then JSON/base64 projection happens after releasing mutable-state locks. The frozen scope copy and returned result copy are each bounded; normal parser summaries retain their existing uncapped successful-answer contract.

- Additional regression-first evidence: `clipped_multibyte_answer_stays_a_prefix_after_later_deltas` failed with a later ASCII byte filling a gap left by an omitted multibyte character. Clipping is now sticky until an authoritative replacement, preserving an exact prefix. Both buffers are preallocated at run construction; replacements clear/reuse the answer allocation. The targeted test then passed.
- Initial broad test runs found test-authoring issues (assuming Codex's existing text-only parser finish already fails; using a nonexistent progress Default constructor). Assertions now check absence of a verdict record and use the normal bridge-created progress cell. These were corrected rather than altering Phase 3 outcomes or broadening a constructor API.

- The full suite's dispatch inventory scan rejected a new provider tuple-array in a test. The earlier-answer regression now calls its shared assertion directly for the two known fixtures, preserving the centralized dispatch rule. Regenerated the existing dispatch inventory through `CLAUDINE_UPDATE_INVENTORY=1 just test-cli dispatch_inventory::`; all 12 inventory tests passed.
- Original Claude answer observation now precedes display-newline normalization as well as callbacks. Explicit parser text observations prevent duplicate retention of the corresponding OutputText event; event-only providers retain the existing fallback. Separate stdout/stderr thread-local lanes preserve single-writer operation observations, including deferred callbacks.
- OS review: the `os` skill and build-host instructions were read before adding native Windows cmd handling. Only BUILD_LINUX is declared here; BUILD_WIN/BUILD_WSL/BUILD_MACOS are absent. Remote cross-check was not invoked because its implementation stages a temporary index and creates a transient commit (`scripts/cross-check.sh`), while this session explicitly forbids staging and commits. The portable fixtures were executed on macOS; other OS behavior remains unclaimed and will need CI or an authorized cross-check run.

- Broad validation also caught captured-stdout/stderr tests asserting the removed speculative pipe-owner wording. Their assertions now use the observed pipe-wait wording. This completes the warning-consumer audit across semantic, inherited, and captured readers; actual panic payload assertions remain unchanged. The seam scanner reads source files independently, so the fixture initialization function now has a local test-fixtures gate in addition to its module gate; all 10 seam-gate tests passed.

- Cutoff-race regression: `released_completion_logger_cannot_dispatch_into_a_settled_run` failed with one late TurnComplete hook after the held logger resumed. The live sink now rechecks run closure before logging and again before downstream hook dispatch, without holding observation locks across either callback.
- Unavailable-answer regression: the provider-error completion callback test failed with Some("") versus null when no answer existed. Failed summaries with no text no longer fabricate an identified empty answer; successful empty results and explicitly observed empty text remain distinguishable.
- Metadata-bound regression: the path boundary test failed because 16 KiB accepted an 8 KiB-plus-one path. The metadata cap is now 8 KiB; worst-case JSON escaping leaves room inside the 64 KiB metadata allowance. The test includes maximum control-character escaping and the exact oversized boundary.

### Final requirement-to-test mapping and reachability

| Requirement | Concrete coverage |
| --- | --- |
| Coherent operations, monotonic offsets, separate reader lanes | `run_scope::tests::{operation_and_time_are_one_coherent_observation, every_operation_has_a_stable_serialized_snapshot, stderr_callbacks_cannot_replace_the_stdout_operation}`; existing held-pipe and shared-clock reader tests |
| Exact original answer before callbacks, with independent verdict | Claude/Codex `original_answer_*` parser tests, sink wrapper forwarding test, reader `an_answer_callback_before_the_verdict_retains_partial_text_without_success`, and extended all-provider completion-callback test |
| Bounded raw/answer prefixes, unavailable versus empty, clipping, UTF-8, malformed/read-error bytes, fallback without verdict | `run_scope::tests::{retained_data_shapes_and_size_survive_two_round_trips, raw_read_precedes_decoding_and_preserves_delimiters, a_failed_raw_read_leaves_partial_bytes_and_no_answer, last_message_is_bounded_answer_only_with_honest_completeness, clipped_multibyte_answer_stays_a_prefix_after_later_deltas, a_raw_prefix_split_inside_utf8_remains_lossless, capture_path_limit_keeps_serialized_detail_within_its_bound}`; provider-error test asserts null for an unidentified answer |
| Capture disabled/unflushed/failed flush/verified coverage | New stream-capture tests plus `capture_completeness_requires_eof_flush_and_no_omissions_or_errors`; existing capture-envelope tests remain unchanged |
| Freeze and late activity cannot contaminate another run or dispatch completion again | `freezing_retention_rejects_late_publication`, `freezing_discards_deferred_callbacks_from_the_abandoned_run`, `released_completion_logger_cannot_dispatch_into_a_settled_run`, and the completion-callback helper's post-release equality assertion |
| Active delivery remains separate from queue submission and abandonment | `queued_frames_do_not_replace_active_delivery`; existing output-worker ordering/limits and session-end output-loss tests |
| Shipped CLI, portable fake providers, shell chain, isolated homes/CWD/PATH/audio, fixed fixture controls | Three `completion_delayed` L1 tests exercise both providers and all stall points, native/wrapper status separation, marker commands, invalid/missing controls, and no audio spool |

Exact added test identities (24), confirmed compiled and matched by the normal
L1 filter, with none ignored:

- `stream::providers::claude::tests::original_answer_observation_precedes_display_newline_normalization`
- `stream::providers::codex::tests::original_answer_is_observed_before_its_reasoning_callback`
- `stream::semantic::tests::sink_wrappers_forward_answer_observation_without_publishing_an_event`
- `completion_delayed::fixture_handshakes_retain_answers_before_both_callback_stalls`
- `completion_delayed::fixture_separates_queued_output_from_unfinished_delivery`
- `completion_delayed::invalid_or_missing_fixture_scenarios_fail_explicitly`
- `commands::wrap::exec::reader_join::provider_streams::an_answer_callback_before_the_verdict_retains_partial_text_without_success`
- `commands::wrap::exec::reader_join::provider_streams::released_completion_logger_cannot_dispatch_into_a_settled_run`
- `commands::wrap::exec::stream_capture::tests::existing_capture_is_visible_without_flushing_the_reader`
- `commands::wrap::exec::stream_capture::tests::failed_capture_flush_never_upgrades_raw_completeness`
- `commands::wrap::output_worker::tests::queued_frames_do_not_replace_active_delivery`
- `commands::wrap::run_scope::tests::a_failed_raw_read_leaves_partial_bytes_and_no_answer`
- `commands::wrap::run_scope::tests::a_raw_prefix_split_inside_utf8_remains_lossless`
- `commands::wrap::run_scope::tests::capture_completeness_requires_eof_flush_and_no_omissions_or_errors`
- `commands::wrap::run_scope::tests::capture_path_limit_keeps_serialized_detail_within_its_bound`
- `commands::wrap::run_scope::tests::clipped_multibyte_answer_stays_a_prefix_after_later_deltas`
- `commands::wrap::run_scope::tests::every_operation_has_a_stable_serialized_snapshot`
- `commands::wrap::run_scope::tests::freezing_discards_deferred_callbacks_from_the_abandoned_run`
- `commands::wrap::run_scope::tests::freezing_retention_rejects_late_publication`
- `commands::wrap::run_scope::tests::last_message_is_bounded_answer_only_with_honest_completeness`
- `commands::wrap::run_scope::tests::operation_and_time_are_one_coherent_observation`
- `commands::wrap::run_scope::tests::raw_read_precedes_decoding_and_preserves_delimiters`
- `commands::wrap::run_scope::tests::retained_data_shapes_and_size_survive_two_round_trips`
- `commands::wrap::run_scope::tests::stderr_callbacks_cannot_replace_the_stdout_operation`

The CLI L1 module is declared in `tests/l1/main.rs`; the compiled provider has
an explicit Cargo bin target requiring `test-fixtures`, enabled in local
recipes and CI metadata. In-source tests belong to the existing lib or binary
test targets. No reserved tier marker was added. `cargo nextest list` matched
every new identity; `just check-tier-coverage claudine` reported zero stranded
tests. No new parser/schema/prompt/template/configuration input format was
introduced, so a shipped-artifact corpus addition and the configuration input
matrix are not applicable. Existing inventory/corpus guards ran in L1. The
opaque observation serializer is tested through two JSON round trips; runtime
failure decisions do not read it back.

### Final validation

- **PASS — `cd claudine && just test`:** 8,506 tests passed across the area's
  selected targets in 69.007 s, 12 runtime-slow tests, 9 existing ignored tests
  skipped. Nextest ID `040883b6-180e-4fa4-adc4-94ef316e71b6`. No test failure
  remains. The initial failed runs described above were corrected.
- **PASS — `cd claudine && just lint`:** all five packages, transport guards,
  and lifecycle documentation guard; no formatter is invoked by this recipe.
  The last lint requested collapsing a nested condition; that equivalent guard
  expression was changed manually, without formatting adjacent code.
- **PASS — `cargo clippy -p claudine-cli --all-targets --features test-fixtures
  -- -D warnings`:** also checks the feature-only fixture code and binaries.
- **PASS — final `just test-cli reader_join::`:** all 31 reader tests passed
  after the lint-only guard collapse, including original/null response handling
  and late logger dispatch. Nextest ID
  `765af059-d025-4022-ae30-51458ba8bc4d`. The broad passing run is reused because
  the only subsequent code edit is this behavior-equivalent conditional fold.
- **PASS — `just check-tier-coverage claudine`:** zero stranded tests.
- **PASS — final nextest listing:** exact declared-target/L1 reachability for
  all 24 added tests, including feature-only process tests. The nine ignored
  tests are the four `completion_perf::perf_*`, one
  `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`, and four
  `system_prompt_perf_bench::bench_*` performance tests identified in Phase 1.
- **PASS — `git diff --check`:** no whitespace errors. A read-only comparison
  against HEAD confirms the entire prior Phase 1 log body is preserved.
- **Existing build warning:** macOS's oversized `__eh_frame` linker warning
  remains; no change or workaround was introduced for it.
- **Not run:** remote OS, L2/L3/browser/real-provider tiers, installation, or
  monorepo-wide testing. These controlled stalls and filesystem/process checks
  are L1. Remote evidence is unclaimed for the reasons recorded above; the
  real-provider/install observations belong to Phase 5.

Local scratch validation artifacts are `/tmp/claudine-phase2-test.log`,
`/tmp/claudine-phase2-lint.log`, `/tmp/claudine-phase2-fixture-lint.log`,
`/tmp/claudine-phase2-readers-final.log`, `/tmp/claudine-phase2-tiers.log`, and
`/tmp/claudine-phase2-list.json`. Red regression logs separately record warning,
multibyte-prefix, path-bound, unavailable-answer, and late-dispatch failures.

### Closing state and handoff

Phase 2's four tasks are implemented, verified, and checked off. The source,
documentation, skill, and package frontmatter records are complete; the source
file list includes every changed Rust file and its build manifests. Current
behavior docs and the Claudine skill describe the observation seam and its
limits. No file was staged, no commit was created, and cargo fmt was never run.
The fix remains active, with Phase 2 implementation complete and ready for
review; no lifecycle directory was moved.

No new author decision arose, but the existing CPU-field ruling is still
required before Phase 3 registers the diagnostic. `human_review: true` remains
set, with its existing options updated to identify that next-phase boundary.
Phase 1's Freeze interfaces checkbox remains open for this ruling; later phase
checkboxes are untouched. The spec handoff explains the populated observation
transport, native/wrapper exit gap, original-text callback, conservative
capture coverage, fixture names, and remaining diagnostic/session/lifecycle
work. No Phase 3 outcome or registry change is claimed here.

## Phase 3

### Scope and prerequisite

The CPU ruling is still explicitly pending in the spec and prior handoff.
Necessary Rule 1 reserves it for the author; this headless run cannot settle
it or treat the omission recommendation as approval. Diagnostic registration
and populated-detail propagation will remain unfinished rather than publish
an irreversible field contract without that ruling. Independent settlement
selection can proceed. No formatting, staging, commits, or lifecycle moves.

### Requirement-to-test mapping before implementation

- Native exit 0 without a published verdict selects
  `claudine_completion_delayed`: update the existing reader outcome matrix,
  `a_stalled_reader_with_no_published_result_is_an_incomplete_stream`,
  held-pipe regression, and provider-stream missing-result test. Assert failed
  summary, unchanged native exit, and subordinate reader warning. The original
  Claude stream with its result record excluded remains the protocol input.
- Native nonzero without a verdict selects `exit_failure`; interruption
  selects `interrupted`: update matrix rows and successful-snapshot/nonzero
  regression; add boundary cases including exit 1, 2, 130, and 137. Assert
  answer/session data are retained when a published success meets native
  failure, and actual provider errors outrank cleanup.
- Confirmed success, provider errors, incomplete subagents, early timeout,
  real panic payloads, and stderr warning-only behavior retain their outcomes:
  reuse the live-provider-parser callback tests, matrix, panic diagnostics,
  and session-end/attempt tests. Extend early-answer callback assertions to
  the new primary identity and its partial-data observation.
- Caller projection, typed diagnostic facets/catalog, lifecycle/session/
  sequence populated-detail equality, and blocked-terminal diagnostic storage
  need integrated tests when the CPU prerequisite is resolved. Existing
  observation-only CLI fixtures remain coverage of the independent Phase 2
  seams, not proof of diagnostic propagation.

All settlement tests are ordinary L1 tests in existing declared CLI binary
targets; no new target or tier marker is needed. No parser/configuration
acceptance change is planned in this independent work, so the configuration
robustness matrix and shipped-artifact corpus requirements do not apply.

### Settlement implementation and targeted validation

- Updated stdout fallback selection to delayed completion for exit 0 without
  a verdict, native `exit_failure` otherwise, and `interrupted` for exit 130.
  The native exit stays separate from the semantic failure flag. Reader
  warnings still name observed operations without claiming a cause.
- Kept published successful summaries' answer/session fields when a later
  native failure changes their verdict. Published provider failures, terminal
  bridge errors, genuine panic payloads, early-termination overrides, and
  stderr warning-only behavior keep the existing precedence. Budgets and
  teardown were untouched; ProcessResult still carries the frozen observation
  alongside stronger outcomes.
- Audited all Rust `stream_reader_timeout` consumers. Only the subordinate
  `ReaderFailure` identity/assertion remains; stdout summary assertions use
  the selected primary kind. Session-end/attempt helpers use warning text
  only and need no behavior change for this independent selection work.
- Added `native_failure_after_a_published_success_keeps_the_answer_and_session`
  and strengthened the existing matrix, held-pipe, missing-result, native
  failure, and early-answer callback tests. Original Claude/Codex protocol
  fixtures remain unchanged.
- RED: `just test-cli reader_join::` failed on the matrix missing-verdict
  row and both provider-stream missing-verdict checks against the old code
  (14 passed, 3 failed; 15 filtered tests not executed after fail-fast).
  `/tmp/claudine-phase3-red.log` preserves that proof.
- GREEN: `just test-cli reader_join::` ran all 32 tests successfully after
  implementation (`/tmp/claudine-phase3-readers.log`). The newly added test
  is compiled by the declared bin/claudine target and selected as ordinary L1.
- Checked Select outcomes immediately after that passing checkpoint. Carry
  diagnostics and Verify propagation remain open; no populated diagnostic
  registration/storage/lifecycle or caller-exit completion is claimed.
- Updated timeout outcome tables and the current skill guidance to distinguish
  implemented summary selection from the remaining transport/caller gap.

Two command setup mistakes were corrected: the first focused run addressed
`test-cli` from the repo root instead of the area, and the first edit script
used repo-relative paths while launched from the area. Neither changed source
or provides test evidence. Canonical runs above use the claudine area.

### Broader validation in progress

`just check-tier-coverage claudine` passed with zero stranded tests. Running
it concurrently with `just test` was a coordination mistake: its Cargo
listing without fixture features overwrote the shared CLI executable while
the broad feature-enabled process tests were running. The first broad run
failed in all three existing `completion_delayed` tests because fixture
controls were absent (4,946 passed, three failed; fail-fast left 3,558 unrun).
No source workaround or skipped assertion was introduced. Repeat the gates
sequentially; the initial log is `/tmp/claudine-phase3-test.log`, and the
sequential rerun is `/tmp/claudine-phase3-test-final.log`. This setup failure
is distinct from settlement regressions and from the CPU decision blocker.

### Final gates and closing state

- PASS: sequential `just test` in the claudine area — 8,507 passed, nine
  existing ignored performance tests, no failures. This is one more passing
  test than Phase 2 because of the new retained-answer/native-failure
  regression. The fixture tests that failed during concurrent Cargo runs
  passed in this sequential run. Log: `/tmp/claudine-phase3-test-final.log`.
- PASS: `just lint` in the claudine area — transport guards, lifecycle-doc
  checks, and Clippy for catalog types, library, contract, CLI, and generator.
  Log: `/tmp/claudine-phase3-lint.log`. The recipe invokes no formatter.
- PASS: `just check-tier-coverage claudine` — zero stranded tests. Log:
  `/tmp/claudine-phase3-tiers.log`. The 32 focused reader tests were actually
  executed by the declared bin/claudine target in L1, not just listed.
- PASS: `git diff --check`. No subsequent source edit affected passing evidence.
- Ignored tests: `completion_perf::perf_compose_empty_partial_meets_target`,
  `completion_perf::perf_compose_long_prefix_meets_target`,
  `completion_perf::perf_inline_compose_empty_partial_meets_target`,
  `completion_perf::perf_enter_compose_partial_meets_target` (the host's one
  platform variant),
  `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`,
  `system_prompt_perf_bench::bench_system_prompt_resolution_cold_and_warm`,
  `system_prompt_perf_bench::bench_resolve_and_prepare_step_by_step`,
  `system_prompt_perf_bench::bench_request_topology_probe_and_reuse`, and
  `system_prompt_perf_bench::bench_raw_darkmatter_compose_passes`. These were
  already ignored; no new skip, retry, or tier change was added.
- Existing macOS linker warning: oversized `__eh_frame` remains, with no
  workaround in this change. No pre-existing source test or lint failure
  remains in the final local gates.
- Not run: remote OS rigs, L2/L3/browser/live-provider tiers, installation,
  or monorepo-wide gates. This phase's partial changes are portable summary
  selection and ordinary L1 tests, with no OS branches or path comparisons
  changed. No remote OS passing evidence is claimed; caller projection and
  native Windows shell verification remain work for the unfinished phase.

The four source files changed in this phase are `reader_join.rs`,
`reader_join/matrix.rs`, `reader_join/provider_streams.rs`, and
`reader_join/tests.rs` under `claudine/cli/src/commands/wrap/exec/`.
Documentation/skill/frontmatter file lists are recorded on both the plan and
this log; the existing package union is preserved. Prior phase log content is
retained. The spec still sets `human_review: true` and describes the CPU
choices; its updated handoff explicitly prevents advancing to Phase 4 before
Phase 3 is finished.

**Phase 3 is not complete.** Select outcomes is complete, tested, and checked;
Carry diagnostics and Verify propagation remain open. In particular there is
no registered typed diagnostic, populated snapshot in lifecycle/session/
sequence output, exactly-once blocked-terminal diagnostic publication proof,
or fixed direct-wrapper semantic-failure caller projection. Resolve the
author's reserved CPU-field decision before publishing that field contract,
then finish those tasks and their integrated regressions. This run closes
with a nonzero status to report the blocked phase honestly. No file was
staged or committed, no `cargo fmt` ran, and the fix was not moved.

## Phase 4

### Prerequisite audit and requirement-to-test mapping

Phase 3 remains incomplete: the author-reserved CPU contract decision is
unresolved, and Carry diagnostics / Verify propagation are unchecked. Do not
freeze that contract or claim author-policy coverage without those prerequisites.
This run can independently prepare the caller regression and repair its known
exit projection; the phase remains partial.

Before implementation changes, the test mapping is:

- Confirmed successful verdict under a blocked completion callback: existing
  `completion_delayed::fixture_handshakes_retain_answers_before_both_callback_stalls`
  runs the compiled CLI in a shell for both Claude and Codex and checks native
  exit, wrapper exit, marker, retained answer, and observation.
- Confirmed successful verdict under blocked terminal delivery: existing
  `completion_delayed::fixture_separates_queued_output_from_unfinished_delivery`
  checks both providers and the shell marker alongside active delivery state.
- Native exit 0 with text but no published verdict: strengthen the original
  answer-callback case to require wrapper exit 1 and no subsequent marker,
  retaining native exit 0 and partial response observations. This assertion
  must fail before the caller projection repair.
- Populated failure/finalize detail, continuation, default halting, recovery,
  and exactly-once lifecycle execution: blocked by missing Phase 3 diagnostic
  transport. Existing tests cannot establish this new seam.
- Remaining caller error controls (provider error, nonzero exit, panic,
  interruption, timeout, and last-message-only): still required by Wave 6;
  not substituted with the helper matrix or claimed complete.

The changed behavior is caller output, not a configuration reader, parser,
schema, or shipped template. No input robustness matrix or artifact corpus
change is applicable. The test file is already declared by `tests/l1/main.rs`,
and its `test-fixtures` feature is enabled by both `just test-cli` and CI.
Use existing portable shell forms; no terminal windows are opened.

### Independent caller checkpoint

- RED: strengthened the original Claude/Codex answer-callback shell regression
  to require caller exit 1 with native exit 0. The old code returned 0 for
  Claude and allowed its subsequent marker. Two other focused tests passed.
  Log: `/tmp/claudine-phase4-red.log`.
- Changed the direct structured execution return to carry native exit evidence
  and caller exit separately through the execution stage and wrapper outcome.
  A summary with `is_error` and native exit 0 projects caller exit 1. Native
  nonzero exits remain unchanged; successful summaries remain caller exit 0.
  Existing summary/session publication still receives the original native exit.
- GREEN: `just test-cli completion_delayed::` passed all three tests, exercising
  both providers' original literal protocol records, both callback positions,
  and terminal-delivery stalls. The partial-response case now prevents the
  next shell marker, while successful-verdict cases continue.
  Log: `/tmp/claudine-phase4-callers.log`.
- No new test file, target, tier, feature, configuration reader, or deadline
  was introduced. Updated the behavior docs, README, skill, and stale execution
  module comments alongside the caller behavior change.
- One focused command was mistakenly invoked from the repository root, where
  `test-cli` does not exist; the canonical area invocation above is the evidence.

Neither Wave 6 todo is checked: Prove caller exits still needs the remaining
error controls, and Prove author policies depends on the unfinished Phase 3
transport. Checking either would falsely claim its entire task passed.

### Final gates and closing state

- PASS: focused `just test-cli completion_delayed::` — three tests passed.
  Exact targeted identities: `fixture_handshakes_retain_answers_before_both_callback_stalls`,
  `fixture_separates_queued_output_from_unfinished_delivery`, and
  `invalid_or_missing_fixture_scenarios_fail_explicitly`, all in
  `completion_delayed` under the declared `claudine-cli::l1` target.
  The first test's strengthened exit assertion is the targeted regression;
  no new or renamed test was added.
- PASS: `just check-tier-coverage claudine` — zero stranded tests.
  Log: `/tmp/claudine-phase4-tiers.log`. Ran before broader Cargo gates.
- PASS: `just test` in the claudine area — 8,507 passed, nine existing ignored
  performance tests, no failures. Log: `/tmp/claudine-phase4-test.log`.
  Ignored identities remain the same nine listed in Phase 3's closing state;
  no skip, retry, or tier change was added.
- PASS: `just lint` in the claudine area — transport guards, lifecycle-doc
  facets, and Clippy for all five area packages. No formatter was invoked.
  Log: `/tmp/claudine-phase4-lint.log`.
- PASS: `git diff --check`. The pre-existing macOS oversized `__eh_frame`
  linker warning remains; no test or lint failure remains after the repair.
- Not run: cross-OS rigs, L2/L3/browser/live-provider tiers, installation,
  or monorepo-wide tests. No OS branch or shell fixture form changed; the
  existing portable shell helper still selects POSIX `sh` or Windows `cmd`.
  Only macOS behavioral evidence is claimed. `BUILD_LINUX` was declared;
  Windows/WSL/macOS remote variables were unset on this host.

Checked the completed successful-verdict shell subtask in the plan after its
passing checkpoint. The parent caller todo remains open for the remaining
error controls. Author-policy tests remain blocked by Phase 3's missing
populated diagnostic transport and reserved CPU decision. The spec retains
`human_review: true` and its decision options; its handoff now describes this
partial caller repair and the remaining prerequisites. No new decision beyond
that existing blocker requires human involvement.

All four source files changed are recorded on both frontmatter blocks:
`cli/src/commands/wrap/mod.rs`, `wrapper_exec.rs`, `wrapper_stages.rs`, and
`cli/tests/l1/completion_delayed.rs` within claudine. Updated README, timeout
and non-interactive-session docs, the claudine skill, and plan/log/spec metadata;
created no documentation files. Preserved prior log content and package union.

**Phase 4 is not complete.** To unblock it, record the author's CPU-field
ruling, complete Phase 3 Carry diagnostics and Verify propagation, then finish
the remaining caller controls and author-policy suite. This session reports
a nonzero closing status rather than claiming the requested phase passed.
No file was staged or committed, no `cargo fmt` ran, and the fix was not moved.

## Phase 5

### Entry checkpoint and requirement-to-test mapping

Read the Claudine, rust-testing, and OS skills. Phase 5 starts with incomplete
Phase 1 Freeze interfaces, Phase 3 Carry diagnostics / Verify propagation,
and Phase 4 caller controls / author policies. The spec still explicitly
reserves the CPU-field ruling for the author. This non-interactive run cannot
supply that ruling, and the recommendation is not an approval. Wave 7 requires
Wave 6 to pass. Preserve planned markers for unavailable behavior rather than
publishing a runnable example that cannot receive populated detail.

No implementation behavior, parser, configuration, template, or persisted
reader is changed in this phase; no new regression, corpus, or input robustness
matrix is applicable. Independent review and verification use these existing
tests; they do not establish the missing diagnostic transport:

- Retention, unknown/empty/partial/overflow variants, UTF-8, repeated JSON
  round trips, and freeze isolation: `commands::wrap::run_scope::tests`,
  particularly `retained_data_shapes_and_size_survive_two_round_trips`,
  `freezing_retention_rejects_late_publication`,
  `clipped_multibyte_answer_stays_a_prefix_after_later_deltas`, and
  `a_raw_prefix_split_inside_utf8_remains_lossless`.
- Callback ordering and stronger-outcome precedence: reader-join matrix and
  `provider_streams`, including
  `an_answer_callback_before_the_verdict_retains_partial_text_without_success`
  and `released_completion_logger_cannot_dispatch_into_a_settled_run`.
- Active delivery versus queue state: `commands::wrap::output_worker::tests`
  and `completion_delayed::fixture_separates_queued_output_from_unfinished_delivery`.
- Portable Claude/Codex shell exits and retained answers:
  `completion_delayed::fixture_handshakes_retain_answers_before_both_callback_stalls`;
  invalid fixture controls: `invalid_or_missing_fixture_scenarios_fail_explicitly`.
- Populated lifecycle detail, registered diagnostic/catalog, exactly-once
  diagnostic storage, continuation/default halt/recovery: no completed tests
  at the new seam. These remain prerequisites, not covered by the above tests.

The CLI process tests are declared in `cli/tests/l1/main.rs` and use the
`test-fixtures` feature enabled by the area recipe and CI. No test is added
or renamed. Validation and metadata updates are independent of the author
ruling. No source files will change without a separate regression mapping.

### Independent documentation and invariant review checkpoint

- Updated timeout documentation with a field-state table distinguishing
  unavailable, empty, partial, overflowed, and complete data. Preserved the
  opt-in artifact's owner-deleted lifetime and its delimiter/flush limitations.
  Corrected drift where already implemented settlement behavior was described
  in the future tense and where undecided CPU support sounded guaranteed.
- Lifecycle, composition, non-interactive-session, and skill guidance now state
  the unavailable populated transport explicitly. Documented `fail_fast: false`
  continuation versus default halting and the risk of retrying completed side
  effects. Kept planned markers and withheld a purportedly runnable diagnostic
  example: the real path cannot yet supply the fields it would read.
- Reviewed packed atomic operation/time pairs, independent reader/settlement
  lanes, separate active output delivery, saturating counters, publication-lock
  ordering, frozen snapshot reuse, 256 KiB per-prefix bounds, UTF-8 clipping and
  lossless raw-byte base64, bounded capture-path metadata, answer-before-callback
  publication, last-message non-verdict fallback, primary outcome precedence,
  and launch-sourced provider/native exit in the controlled fixture. No new
  settings, CPU samplers, spool services, terminal writers, deadline extensions,
  protocol acceptance changes, or CI gates were introduced. No configuration
  reader or persisted-summary deserializer changed, so plan rule 9's input
  robustness matrix does not apply.
- Found the remaining production publication gap: `completion_observation` is
  populated in semantic spawn and otherwise carried in `ProcessResult`; only
  the test-fixtures publisher serializes that snapshot. `StoragePublication`
  exists in the operation vocabulary but has no production recording site.
  The new kind has no diagnostic registry or lifecycle-context mapping.
  Exactly-once populated diagnostic storage/propagation is therefore unproved,
  not an invariant established by the existing session-end tests.
- PASS: `just check-tier-coverage claudine` (zero stranded tests);
  `just test-cli completion_delayed:: run_scope:: reader_join:: output_worker::`
  (66 passed, 3,426 filtered); `just test-library stream::`
  (886 passed, 3,850 filtered). Logs: `/tmp/claudine-phase5-tiers.log`,
  `/tmp/claudine-phase5-focused-cli.log`,
  `/tmp/claudine-phase5-focused-library.log`. No new targeted tests were added;
  exact regression identities and the missing requirements are mapped above.

Checked the independent timeout-documentation and no-new-subsystems subtasks
as they completed. Parent tasks remain open where required diagnostic and
author-policy behavior is unavailable. An initial metadata/log edit used the
root-relative path from the area directory and failed before writing; reran
it from the repository root. This did not affect the test invocation.

### Local gates and cross-OS scope

- PASS: `just test` from the Claudine area — 8,507 passed, 15 runtime-slow,
  nine existing ignored performance tests; no failures. The ignored tests are
  the four `completion_perf::perf_*`, one
  `compose_ttff_perf::compose_emits_first_stderr_byte_within_budget`, and four
  `system_prompt_perf_bench::bench_*` identities enumerated in Phase 1.
  Log: `/tmp/claudine-phase5-test.log`.
- PASS: `just lint` from the Claudine area — transport guards, lifecycle doc
  facets, and Clippy across all five area packages. The recipe does not call
  cargo fmt. Log: `/tmp/claudine-phase5-lint.log`. No new pre-existing test or
  lint failure was encountered; macOS's previously recorded oversized
  `__eh_frame` linker warning remains nonfatal.
- Only `BUILD_LINUX=build-linux` is declared; `BUILD_WIN`, `BUILD_WSL`, and
  `BUILD_MACOS` are unset. Native Windows and WSL2 were not run; no behavioral
  evidence for those systems is claimed. L2/L3/browser tiers were not run:
  documentation and existing hermetic caller/retention checks introduce no
  new question requiring a terminal, browser, or focus change.
- Started `GIT_TERMINAL_PROMPT=0 just cross-check claudine-cli --os linux
  --features test-fixtures completion_delayed:: run_scope:: reader_join::
  output_worker::`. The supported feature override chooses native nextest,
  avoids unrelated daemon features, and produces behavioral evidence without
  an archive-mode receipt. The recipe's disposable snapshot is `f72d9bf7d`
  over `origin/main` at `d3fcc03ce`, with 224 changed files relative to that
  remote base, including prior-phase work. It does not modify the local branch
  or stage local files. Result will be recorded below before this run closes.

Checked the local validation subtask after its passing gates. The parent
validation task remains open for unavailable diagnostic/author-policy tests
and missing OS evidence. Installation and live Claude/Codex runs remain
unattempted because Wave 7 requires the unfinished Wave 6 checkpoint;
observing an incomplete build cannot satisfy final repair acceptance.

### Final evidence and closing state

- PASS: focused Linux cross-check — all 66 selected tests passed, 3,426 tests
  filtered out. The three `completion_delayed` process identities each ran;
  the shell/handshake test checks Claude and Codex inside the same identity.
  Log: `/tmp/claudine-phase5-linux.log`. This is native Linux behavioral
  evidence at the disposable snapshot recorded above, not a CI receipt and
  not evidence for missing diagnostic propagation or author policies.
- PASS: `git diff --check` after final documentation/metadata edits.
- Completed Review invariants as an audit task and checked it off; its finding
  is missing populated production storage/transport, not a claim that the
  unimplemented invariant passed. Other parent todos retain their honest
  incomplete status. The documentation's planned recovery example is not
  promoted to runnable, and no unavailable diagnostic code is advertised as
  registered. No new or renamed tests, source code, dependency, or config
  changes occurred during Phase 5.
- Updated both plan and log with Phase 5 source/docs/skill inventories and
  all-phase `source_code` / `documentation` unions, preserving the package
  union (`claudine`, `claudine-cli`) and prior log content. Phase 5 changed
  four topic documents, the Claudine skill, and plan/log/spec metadata.
  Created no documents. Source files changed during this phase: **none**.
- Departure from the requested completion metadata: `implemented` stays false
  in plan/log/spec, and `completed_phase` is 2 in plan/log, the last wholly
  implemented phase. Setting them to true/5 would falsely attest that the
  unregistered diagnostic, populated transport, author examples, and required
  evidence exist. The spec preserves the existing `human_review: true` and
  author decision options; no additional author decision was introduced.
  Its handoff now identifies the exact remaining seams and evidence.

**Phase 5 is partial, not implementation complete.** Finish the reserved CPU
contract decision, Phase 3 diagnostic registration/transport and propagation
checks, and Phase 4 error controls/author-policy suite before completing the
runnable docs, final cross-OS proof, installation, and real observations.
Those earlier tasks were not silently treated as complete or replaced by a
broad green test suite. No live provider was invoked, no terminal/browser
window was focused, no cargo fmt ran, no local branch commit or staging was
performed, and the fix directory was not moved. The cross-check recipe's
disposable snapshot left local branch/index untouched. This session reports
a nonzero closing status to make the unfinished prerequisite work visible.

Final frontmatter validation parsed all three YAML blocks, checked file
inventories and preserved package unions, and confirmed partial completion
flags. The final whitespace check passed.
