---
reviewed: true
reviewed_by: codex/default
reviewed_on: 2026-08-31
implemented: true
review_iterations: 4
---

# Wrapped runs can stall indefinitely at startup or report success with incomplete work

## Outcome

Structured, non-interactive wrapped runs must satisfy both of these contracts:

1. The existing `step_timeout` rule bounds silence from successful child spawn,
   including the period before the first activity signal. It remains the same
   stream-silence rule, not a third startup timeout.
2. A provider process exiting with code 0 is not sufficient proof of success.
   Provider-semantic failures represented by `StreamExecutionSummary.is_error`
   must route through `AgentFailure`; in particular, a Claude session with an
   unresolved stopped or unfinished background task must not fire the `success`
   lifecycle stack.

Capture and interactive/passthrough modes remain outside `step_timeout`, as they
are today, because they do not have the structured-stream heartbeat required by
the silence rule.

> **Reader's note:** The draft proposed consulting the watchdog's
> `recent_subagents` ring after process exit. That ring contains only the five
> most recent terminal observations and is intentionally diagnostic-only. It
> also never receives Claude `task_notification` events today: the Claude parser
> maps those events to `Info` and drops their task status. This specification
> instead requires terminal notification normalization plus a complete,
> session-scoped task ledger. It also preserves the provider's real exit code and fixes
> the general semantic-error classification boundary rather than manufacturing a
> nonzero provider exit code for this one failure shape.

## Problem

Two failure modes surfaced together during the 2026-08-30/31 `just commit`
incident (five consecutive failed attempts across OpenCode and Claude). Both
are wrapper-layer outcome/timeout defects, not composition or provider-catalog
defects: the same document, model, and binaries pass under healthy conditions.

1. **Startup stall is invisible to `step_timeout`.** A wrapped provider that
   hangs before its first activity signal is never caught by the silence rule.
   Only the wall-clock `timeout` can end it, but `timeout` is opt-in and has no
   built-in default.
2. **A run whose background tasks were force-stopped can report success.**
   Claude Code can emit `task_notification` with `status: "stopped"`, finish the
   parent turn, and exit 0. Claudine currently loses that status during stream
   normalization and later classifies the attempt from process termination and
   exit code alone, allowing the `success` lifecycle stack to fire with no work
   completed.

## Observed incidents

- **OpenCode, 3 runs (2026-08-30 21:10, 22:37, 23:07 UTC):** OpenCode
  bootstrapped, logged its configuration-file loads, and then produced no LSP
  initialization, session creation, or stream events for 30 minutes. The
  document's `timeout: 30m` ended each run (`session_end` reported
  `error_kind: "timeout"` and `log_records_parsed: 10`); its
  `step_timeout: 12m` never fired. The silent phase is where OpenCode resolves
  plugins and fetches the models.dev catalog. The user configuration pinned
  `opencode-gemini-auth@latest`, forcing an npm registry lookup during startup.
- **Claude, 1 run (2026-08-31 06:37 UTC):** The orchestrator ran for 90
  seconds, dispatched two commit subagents, and then the parent and both
  subagents became stream-silent. The subagents emitted `before_tool` for their
  first `git log` calls but no results. After 600 seconds Claude Code reported
  both tasks as `stopped`, completed the parent turn, and exited 0. Claudine
  fired `commit.md`'s `success` stack (message plus `shell: just gitnexus`) even
  though no commit existed.

The diagnostic recipe and evidence pointers are recorded in session memory
`project_just_commit_hang_2026_08_31.md` (JSONL filters, the OpenCode log
divergence point, and the subagent transcripts).

## Root cause

### 1. Unbounded startup grace in the silence rule

`evaluate_timeout_tick` in
`claudine-cli/src/commands/wrap/exec/watchdog/evaluate.rs` evaluates
`step_timeout` only when `LiveMetrics::last_activity_at()` returns a value. A
child that emits no qualifying event or non-whitespace byte therefore has no
silence-clock origin. The older `detect_step_timeout` helper in
`exec/timeouts.rs` documents the same first-event grace. The wall-clock rule is
described as the backstop, but `TimeoutConfig::timeout` defaults to `None`.

OpenCode adds a second suppression: while no `step_finish` has populated
`provider_status`, the provider-specific cold-start guard returns `Ok`
unconditionally. Even a spawn-based fallback clock would remain ineffective if
this guard were not bounded by the same `step_timeout` budget.

### 2. Claude terminal task status is discarded

`ClaudeTaskEvent` in `claudine/lib/src/stream/protocol/claude.rs` parses task
identity and `status`, but `ClaudeStreamParser` currently routes both
`task_progress` and `task_notification` through `handle_task_progress`.
`task_notification` therefore becomes a generic `SemanticEvent::Info`; its
status and identity are not represented as a terminal subagent observation.
Consequently, neither the live task state nor the final stream summary can know
that the tasks stopped.

The CLI watchdog's `WatchdogState::recent_subagents` is not a suitable repair
point. It is a five-entry, newest-first ring used to enrich timeout diagnostics,
not an authoritative session outcome ledger. Using it would silently miss the
sixth and earlier task and would couple provider outcome semantics to CLI-only
watchdog state.

### 3. Semantic failure is dropped before lifecycle classification

`StreamExecutionSummary` already distinguishes the native `exit_code` from the
provider-semantic `is_error` flag. `AttemptOutcome` does not carry `is_error`,
and `classify_failure` treats `ProcessTermination::Completed` plus exit code 0
as success. This is broader than the stopped-task incident: any parser that
truthfully reports `is_error: true` while its provider exits 0 can be silently
misclassified.

## Required behavior

The requirements below adopt the recommended fail-closed ruling in the open
question. If implementation review chooses another option, revise the outcome,
ledger finalization, and acceptance criteria together before coding.

### 1. Bound startup with the existing `step_timeout` rule

- Use the existing monotonic child `started_at` instant as the silence-clock
  fallback until activity exists. On each tick, the silence reference is the
  most recent of `started_at`, `last_event_at`, and `last_byte_at`. No separate
  startup duration, configuration key, or termination reason is introduced.
- Non-whitespace child bytes continue to refresh `last_byte_at`. Startup is
  therefore bounded from the most recent real output, not blindly from process
  spawn when a provider is visibly making progress.
- The in-flight tool/subagent gate remains unchanged after activity begins.
  Before the first task/tool start there is nothing in flight, so it cannot
  suppress the spawn-based budget.
- The OpenCode cold-start guard may suppress evaluation only while the same
  spawn/activity silence age remains below `step_timeout`. It must not create an
  unbounded exception. The existing mid-step rule remains: a live step is
  suppressed while at least one activity clock is fresh, but not when both are
  stale for the full budget.
- Wall-clock `timeout` keeps precedence when both rules breach on the same
  watchdog tick.
- The `step_timeout` plus one watchdog interval bound governs **detection and
  the termination request** — the tick on which the wrapper decides the rule
  breached and signals the child. `kill_grace` is outside that bound: the
  platform termination ladder is a separately configured budget that
  necessarily follows, so the child's final reap is bounded by
  `step_timeout` + one watchdog interval + `kill_grace`.
- A startup breach remains `ProcessTermination::TimedOut` with
  `error_kind: "step_timeout"` and follows the existing platform-specific
  termination ladder (SIGTERM to SIGKILL on Unix and the established Windows
  equivalent). The diagnostic must distinguish:
  - no activity since launch; and
  - OpenCode activity followed by a stall before its first completed step.
- `step_timeout_warn` uses the same spawn/activity fallback clock. Its warning
  must identify startup silence when no activity has yet occurred, fire once
  for that stall episode, and reset under the existing warning-reset rules when
  activity resumes.

### 2. Normalize terminal Claude task notifications

- `task_progress` remains `SemanticEvent::Info`.
- A `task_notification` with a terminal status must become a terminal subagent
  observation carrying provider task ID, name, raw status, and relevant
  metadata. At minimum, `completed`, `success`, and `succeeded` are successful;
  `stopped` is unsuccessful. `task_completed` remains a terminal observation
  and follows the same normalization path.
- Preserve unknown terminal status strings in machine data. Do not infer
  success from an unknown value; surface it as unresolved until the provider
  vocabulary is explicitly extended and tested.
- Because `task_notification` carries no inherent terminality, its status is
  routed by three explicit vocabularies — successful, unsuccessful, and
  nonterminal/progress — under this ratified rule:
  1. an **absent or whitespace-only** status is progress (`Info`); the provider
     said nothing, so nothing is lost;
  2. a status in the successful or unsuccessful vocabulary is a terminal
     observation preserving ID, name, and raw status;
  3. a status in the **progress** vocabulary (`thinking`, `running`,
     `in_progress`, `progress`, `started`, `starting`, `pending`, `queued`,
     `working`, `active`, `resumed`) is progress (`Info`);
  4. a status that is **present but in none of the three** is a terminal
     observation with an unresolved outcome, its raw string preserved verbatim.
  Branch 4 is the fail-closed residue: an unrecognized status must never be
  read as still-in-flight, because that would silently accept an unknown future
  terminal status as non-failing. Extending the progress vocabulary requires
  fixtures, exactly as extending the successful one does. Matching is
  case-insensitive and trim-normalized throughout.
- Missing task IDs must not be collapsed into the empty-string ID. Preserve
  each anonymous terminal observation as a distinct fact so one event cannot
  accidentally clear another. Names are display metadata, not identity.

### 3. Keep an authoritative session task ledger

- The library stream layer owns a session-complete ledger; the CLI watchdog's
  bounded `recent_subagents` ring remains unchanged and diagnostic-only.
- Reconcile observations by provider task ID. A later successful terminal
  observation for the **same ID** clears an earlier `stopped` state. A newly
  started/resumed task with that ID becomes in-flight again until another
  terminal observation arrives.
- A different task with a similar name, description, or apparent purpose does
  not clear the stopped task. Claudine has no sound way to prove semantic
  equivalence from prose.
- At session end, both of these states are incomplete and poison success:
  - terminal status is `stopped` or an unknown non-success status; or
  - a task was started but has no terminal observation.
- The ledger must not cap or evict entries. It lives only for one provider
  attempt and is released with the stream parser/summary.

### 4. Project semantic failure without falsifying the native exit

- Add a provider-agnostic, serializable task-outcome fact type to
  `StreamExecutionSummary`, and serialize the nonempty list into the synthetic
  `session_end` row as `extra["subagent_outcomes"]`. Each item carries the task
  ID when known, name when known, normalized outcome, and raw provider status.
  This top-level extra field is the stable machine contract; do not bury it in
  Claude's `raw_summary`.
- When incomplete tasks remain, finalization sets `summary.is_error = true`,
  `summary.error_kind = "incomplete_subagents"`, and an operator-facing
  `summary.error_message`. Preserve `summary.exit_code` as the provider's real
  exit code (0 in the incident) and preserve
  `ProcessTermination::Completed`, because Claudine did not kill the child.
  The `error_kind` assignment is **unconditional**: it is the stable
  machine-facing exit reason for this shape, so an `error_kind` the parser
  recorded earlier in the attempt may not hold the slot. That earlier failure
  is still operationally valuable and `error_message` is the only place its
  text still has, so the composed message names both the incomplete tasks and
  the displaced provider failure, within the same 240-character concise-message
  budget. The displaced clause is budget-reserved, so the truncatable tail is
  the task list — which stays complete in `subagent_outcomes` regardless.
- Carry `StreamExecutionSummary.is_error` into `AttemptOutcome`.
  `classify_failure` must classify a completed attempt as `AgentFailure` when
  either `exit_code != 0` **or** `is_error` is true. This is the general
  semantic-error contract; it intentionally applies to all providers, not only
  Claude.
- The failure lifecycle stack fires; the success stack does not. This failure
  is fail-fast unless the document's ordinary `failure` stack explicitly
  chooses recovery. It is not `Timeout`, `Aborted`, or an automatically
  retryable transport error.
- The failure carries two identities, and they are deliberately different
  spellings of the same condition:
  - the stable **machine** identity is `summary.error_kind ==
    "incomplete_subagents"`, carried into the synthetic `session_end` row
    alongside `extra.subagent_outcomes`;
  - the **lifecycle-stack** identity is the locked catalog code `err.code ==
    "provider.incomplete_subagents"` under `err.category == "provider"`, with
    `disposition: unrecoverable` and `origin: provider` encoding the fail-fast
    rule above. `err.kind` and `err.variant` are the deprecated spellings of
    `err.category` and `err.code`, so they read `"provider"` and
    `"provider.incomplete_subagents"`. A `when:` clause should pin `err.code`.

  The draft asked for `err.kind = "incomplete_subagents"`. That is not
  implementable: `err.kind` is the deprecated alias for the diagnostic
  `Category`, a closed 12-value taxonomy, and `incomplete_subagents` is not a
  category — making it one would corrupt the taxonomy for every other error.
  Adding the catalog row is the additive, non-breaking way to give the failure
  the same machine-matchable identity the draft was reaching for.
- The concise lifecycle/error headline follows the existing 240-character
  hygiene contract and includes the incomplete count and as many names as fit.
  The full terminal diagnostic enumerates every incomplete task using
  `TerminalRenderable` components (`Prose` plus `UnorderedList`), and the
  machine-readable list remains complete regardless of display truncation.
- Existing reporting ingestion must retain the new `session_end` extra data so
  `claudine logs` and the dashboard can expose it without re-parsing prose. If
  either consumer projects a fixed schema that drops arbitrary extras, update
  that projection in the same change.

> **Reader's note:** Treating `is_error` as authoritative can reveal existing
> parser-semantic failures that previously exited 0 and appeared successful.
> That is an intended correction, not a compatibility regression. Tests must
> cover at least one non-task semantic error with native exit 0 so the broader
> effect is explicit.

### 5. Documentation and operational guidance

- Update the authoritative timeout and signal/outcome topic docs and their
  materialized skill copies together:
  - `claudine/docs/topics/timeouts.md`
  - `.claude/skills/claudine/timeouts.md`
  - `claudine/docs/topics/signal-handling.md`
  - `.claude/skills/claudine/signal-handling.md`
  - `claudine/docs/topics/non-interactive-sessions.md`
  - `.claude/skills/claudine/summaries/non-interactive-sessions.md`
    (generated — never hand-edit this file. `just publish-summary-research`
    composes `claudine/docs/research/summary/*.md` through `md compose` into
    this directory, so a direct edit is overwritten on the next publish. The
    durable edit site is the source,
    `claudine/docs/research/summary/non-interactive-sessions.md`; correct it
    there and republish. That source's *body* is itself produced by the
    document's own `sequence`, so a correction that must also survive a
    research regeneration belongs in the source's frontmatter `prompt` as a
    standing rule — the precedent is the prompt-iteration technique recorded in
    `claudine/features/_completed/2026-07-02-provider-metadata/model-config-refresh.md`.
    This fix did both. Scope note: the summary is a cross-provider research
    comparison, not a Claudine behavior contract, so only its Claudine-facing
    recommendations are in scope for a behavior fix — here, the claim that
    Claude's `result` event is by itself semantic completion)
  - `.claude/skills/claudine/opencode-event-sources.md` (skill-only by
    design — the `2026-05-12-opencode-stderr-returns` fix that created it
    asked for a file under `.claude/skills/claudine/`, so there is no
    authoritative topic counterpart to pair it with)
- Update the two-rule comments in `watchdog/evaluate.rs`, `timeouts.rs`, and
  `HarnessPlan::step_timeout`; remove all claims that startup has unbounded
  first-event grace or that completed exit code 0 alone means success.
- Add a startup-stall section to the timeout topic explaining that OpenCode
  plugin specifications using `@latest` make startup depend on npm
  reachability. Recommend exact plugin-version pins. Claudine must not mutate a
  user's OpenCode configuration.
- Fixing this host's `~/.config/opencode/config.json` is a separate manual
  operator action, not an implementation step or acceptance criterion for this
  repository change.

## Open question for implementation review

### Can an unresolved `stopped` task be compatible with success?

Claude may eventually document a deliberate abandon-and-succeed workflow. The
stream does not currently carry an explicit "intentionally abandoned and no
longer required" outcome, so Claudine cannot distinguish that workflow from the
observed gave-up-and-exit-0 failure.

1. **Fail every unresolved stopped task (recommended).**
   - Pros: deterministic, provider-version-independent, fail-closed, and fixes
     the incident without a timing heuristic.
   - Cons: a provider that deliberately abandons optional speculative work must
     complete or explicitly replace that task before the run can succeed.
2. **Warn but allow success.**
   - Pros: never rejects a deliberate abandonment.
   - Cons: preserves the false-success defect and allows success side effects
     to run after known incomplete work.
3. **Fail only when stops correlate with the parent's terminal flush.**
   - Pros: narrows the failure rule to the observed gave-up signature.
   - Cons: depends on buffering and event timing, is fragile across provider
     versions, and misses the same semantic failure when unrelated events occur
     between stop and result.

**Recommendation:** Choose option 1 for structured non-interactive runs. A
known unsuccessful terminal state is stronger evidence than native exit code 0,
and there is no reliable positive signal for intentional abandonment. If Claude
later adds such a signal, add it to the success allowlist with fixtures rather
than weakening the default.

## Out of scope

- Repairing the environmental network/npm/API stall. This specification bounds
  and reports it.
- Answering Claude Code `control_request` events or changing tool-permission
  plumbing; the incident does not implicate either.
- Changing OpenCode `stall_timeout` semantics. That live-but-dead detector is
  unchanged.
- Adding retry-on-startup-stall policy. The run ends `TimedOut`, as it does for
  other `step_timeout` breaches.
- Inferring task equivalence from names, descriptions, prompts, or repository
  effects.

## Acceptance criteria

1. **Generic startup stall:** a re-exec test fixture that emits no output is
   terminated without a wall-clock `timeout`; the summary reports `TimedOut`
   and `error_kind: "step_timeout"` with a startup-specific message. The
   wrapper must decide the breach and signal the child within `step_timeout`
   plus one watchdog interval, and the child must be reaped within that plus
   `kill_grace`. The reap is the quantity integration coverage asserts,
   because it is the last event the wrapper observes; a generous outer process
   timeout remains only as a hang backstop.
2. **Startup heartbeat:** non-whitespace bytes before the first semantic event
   refresh the startup silence clock; whitespace-only bytes do not. A fixture
   proves both sides.
3. **Startup warning:** `step_timeout_warn` fires once from the spawn fallback,
   uses startup wording, and resets after real activity under the existing
   warning contract.
4. **OpenCode guard:** the no-output OpenCode fixture is no longer exempted by
   missing `provider_status`. Companion tests preserve wall-clock precedence
   and mid-step suppression while either activity clock is fresh, and prove a
   breach when both clocks are stale.
5. **Terminal normalization:** Claude fixtures prove that `task_progress`
   remains `Info`, terminal `task_notification` preserves ID/name/status, and
   `status: "stopped"` is not discarded. Fixtures also prove all four
   notification-routing branches: absent status and an explicit progress word
   stay `Info`, while a present-but-unrecognized status becomes a terminal
   unresolved observation whose raw string survives into
   `subagent_outcomes[].raw_status`.
6. **Silent-success poisoning:** replay the incident shape (`task_started` x2,
   terminal `task_notification status:"stopped"` x2, result, native exit 0).
   The native exit remains 0 and termination remains `Completed`, but
   `is_error` is true, `error_kind` is `incomplete_subagents`, the failure stack
   fires, the success stack does not, every stopped task appears in the full
   diagnostic, and all facts appear in `session_end.extra.subagent_outcomes`.
7. **Ledger completeness:** more than five stopped tasks are all retained and
   reported. Anonymous terminal observations remain distinct. Duplicate events
   for one ID do not create duplicate facts.
8. **Identity reconciliation:** stopped then completed for the same task ID
   succeeds; stopped task A followed by completed task B with the same name does
   not clear A; started-without-terminal also fails.
9. **General semantic failure:** a non-task summary with `is_error: true`, exit
   0, and `ProcessTermination::Completed` classifies as `AgentFailure`. A clean
   summary with exit 0 remains successful.
10. **Machine compatibility:** old summary JSON without `subagent_outcomes`
    deserializes to an empty list, empty lists are omitted on serialization, and
    reporting/dashboard projections retain a populated list.
11. **Cross-platform termination:** startup-timeout integration coverage uses
    the existing platform-neutral test harness and does not assume Unix signal
    numbers in shared assertions. No terminal or browser window gains focus.
12. **Non-vacuity:** each new guard is proven non-vacuous by temporarily
    neutralizing the condition, observing the focused test fail, and restoring
    it before the final test run.
13. **Verification:** run the package area's canonical `just test`, relevant
    `just test-l2`, and `just lint` recipes through nextest; do not use
    `cargo test`.
14. The documentation, skill snapshots, code comments, and operational note in
    Required behavior #5 land in the same change.
