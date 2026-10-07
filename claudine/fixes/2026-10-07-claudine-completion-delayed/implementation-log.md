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
packages: []
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
