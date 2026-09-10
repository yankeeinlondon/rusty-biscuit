# Steering implementation plan

Status: Ready for non-interactive execution; implementation phases not started.
Created: 2026-09-08
Specification: [spec.md](spec.md)
Evidence: [fleet run](fleet-run.md), [uncertainty register](uncertainties.md),
[disposable verification](verification/README.md)

## Execution contract

Execute Phase 1 through Phase 8 in order. This plan contains no deferred user
choices. The specification's **Resolved Engineering Contract** supplies concrete
CLI, configuration, control-channel, timeout, recovery, logging, and failure
policies. Earlier pilot recommendations are historical context where superseded.
Do not reopen accepted decisions during execution.

Each phase produces a reviewable change and records its changed files, checks,
results, and remaining external blockers in a progress section appended here.
Do not mark a phase complete while its required implementation or regression
checks are unfinished. A missing provider binary, credential, service, or native
OS runner blocks that capability's activation; it does not justify fabricated
evidence or prevent independent phases from proceeding. Record a phase with such
outstanding required evidence as **implemented; verification blocked**, not complete.

Before editing existing symbols, run GitNexus upstream impact analysis and report
callers, affected processes, and risk. Warn on high/critical risk. If a symbol is
unindexed, refresh or inspect its source/callers and record the coverage limitation;
never describe an unknown result as zero impact. Keep changes within the reviewed
scope, preserve unrelated work, and update behavioral comments with code changes.
Do not commit or run `cargo fmt` without a separate explicit instruction.

Use the Claudine skill throughout, plus the relevant package skills for touched
areas. Use Sniff for host/process discovery and stable user identity, Biscuit-file
for file-reference resolution and data-format conversion, Darkmatter for Markdown
hashes where present, and Biscuit-hash for generated non-Markdown hash baselines.
Use existing library/CLI boundaries and generated provider dispatch; do not add
runtime shell commands copied from research or a parallel capability table.

Runtime human consent for `claudine steer` interruption remains a product feature.
Test it with controlled terminal/input fixtures; the implementation agent must not
stop for a human to operate a test. Research workers use `gpt-5.6-sol` with low
reasoning. Real-provider tests use deliberately created disposable sessions,
prefer deterministic local models, preserve enabled resources, and never focus a
terminal/browser or contact an existing user session.

## Starting baseline

The following work already exists and must be extended rather than repeated:

- All ten roster providers have revision-3 steering research and typed execution
  interface backfills. The full passive fleet and four pilots are complete.
- `claudine providers steering check [slug] [--json]` and the generator equivalent
  check schema relationships, coverage, references, and execution-topic links.
  Generation already gates application of catalog/provider data on these checks.
- Execution-research metadata is already present in the regenerated catalog.
  Runtime steering selection, reviewed production adapters, and the public
  `steer` command are not implemented by that metadata work.
- Pi 0.84.4/macOS has seven scoped verification records. The real-RPC fixtures
  establish useful delivery behavior and important expected failures. They are
  native disposable fixtures, not production-wrapper verification or broad
  extension/profile/OS certification.
- Pi accepts queued text before delivery; duplicate IDs do not suppress duplicate
  messages; pending switches and provider termination can lose accepted text.
  Abort can move queued text into conversation history. Normal abort cleans up
  the tested external process, while killing Pi leaves it alive until fixture
  cleanup. Preserve these regression cases.

No phase may enable a capability merely because a report contains `outcome: passed`.
Required successful-delivery assertions, exact applicability, compatible runtime
interface, and reviewed adapter implementation are separate gates.

## Phase 1 — Typed contracts and generated eligibility

**Dependencies:** Existing research and the resolved specification.

**Work**

1. Add shared steering and execution-interface vocabulary to
   [catalog-types](../../catalog-types/src/lib.rs), keeping transport, encoding,
   operation intent, receipt strength, execution state, and delivery state distinct.
   Define typed runtime request/result and identity contracts in the Claudine
   library. Preserve explicit unknown and partial-interruption outcomes.
2. Extend [research loading](../../gen/src/inputs.rs),
   [field ownership](../../gen/src/registry.rs), catalog coercion, and
   [Rust emission](../../gen/src/emit/mod.rs). Consume the existing steering
   records and execution selections rather than reauthoring provider facts.
3. Extend the maintained checker with deterministic activation applicability:
   exact provider/version/OS/profile/origin/state/operation, adapter revision,
   and required assertion coverage. Represent reviewed adapter bindings by
   implemented identifiers, separate from the generated factual catalog.
   Add typed assertion/adapter references if the current prose-only assertion
   lists cannot support deterministic decisions; preserve historical records as
   evidence without automatically making them activation grants.
4. Derive manual and automatic eligibility from operation effects, delivery
   boundaries, access, compatibility, and applicable verification. A next-turn
   follow-up is not active-loop rescue. A terminal-only response cannot satisfy
   prompt return-after-acceptance without separately established early acceptance.
5. Regenerate affected catalog/provider artifacts and update their existing drift
   and hash baselines. Keep all unimplemented or insufficiently verified bindings
   unavailable, with an actionable reason.

**Validation:** Generator/catalog unit tests; maintained steering check for all
roster providers; deterministic regeneration; negative fixtures for missing IDs,
wrong OS/profile/origin/version, unreviewed adapter revision, expected-loss-only
records, unsupported mechanisms, and malformed execution references.

**Exit:** Generated facts and deterministic activation policy agree; adding a
source claim or an unrelated passing record cannot enable delivery. Existing
research remains intact and the checker reports specific blocking reasons.

**Failure behavior:** Repair schema/relationship regressions before later phases.
Unresolved provider facts remain unknown. Do not rerun a full fleet to repair a
local contract mapping; use bounded targeted research only where evidence is missing.

## Phase 2 — Shared secret recognition and steering audit

**Dependencies:** Phase 1 request/result contracts.

**Work**

1. Introduce `claudine::secrets` for shared text-span and sensitive-key recognition.
   Extract common recognition from [protect scrubbing](../../lib/src/protect/scrub.rs),
   [argument sanitization](../../cli/src/commands/wrap/env/sanitize.rs), and
   [webhook redaction](../../lib/src/messaging/send.rs) where their rules overlap.
   Keep each existing consumer's replacement and privacy policies unchanged.
2. Implement steering's `****` replacement over merged UTF-8-safe secret spans.
   Preserve ordinary prose, email addresses, and paths unless a recognized secret
   is present. The delivered message retains its original bytes.
3. Add typed steering events to the existing local JSONL logging path: full
   redacted text, opportunity/request/target identities, mechanism/profile,
   timestamps, consent, receipt strength, and separate cancellation/replacement
   results. Redact errors and provider echoes before tracing or rendering.
4. Correlate append-only late-result updates without treating them as new sends.
   Audit failures produce content-free diagnostics and never cause replay or
   terminate an otherwise valid agent execution. Keep existing retention policy.

**Validation:** Meaningful fixtures for token prose, assignments, auth headers,
credential-bearing URLs, overlap, Unicode, multiline text, repeated masking,
provider echoes, original-message preservation, and existing-consumer behavior.
Inject log-write failure and confirm the send result and execution remain correct.

**Exit:** Shared recognition has one authoritative implementation; steering logs
contain full masked text and delivery details, while original text reaches only
the intended in-memory delivery path.

**Failure behavior:** Fix redaction leakage before integrating live sends. Do not
solve regressions by deleting required audit text or changing unrelated privacy policy.

## Phase 3 — Session ownership, discovery, and local routing

**Dependencies:** Phases 1–2.

**Work**

1. Extend Rendezvous core protobuf, client, and daemon with a local-only managed
   control stream, target listing, and send routing. Reuse
   [local IPC protections](../../docs/rendezvous/local-ipc.md). Do not add the
   daemon/database dependency to ordinary wrapper execution or duplicate its
   Unix/Windows transport implementation.
2. Connect an execution-owned controller to the wrapper lifecycle near
   [session reporting](../../cli/src/commands/wrap/session_report.rs). Keep control
   registration distinct from replicated presence and its reporting opt-out.
   The controller exclusively owns provider I/O, correlates requests, and
   serializes mutations while continuously draining output/events.
3. Implement per-execution UUIDs, wrapper/process-start identity, provider
   conversation generation, disconnect cleanup, and fresh target checks. Invalidate
   targets on conversation replacement; reject stale IDs rather than retargeting.
4. Implement bounded routing: 16 pending requests per execution, one automatic
   request pending/in flight, 10-second manual acceptance and 2-second automatic
   deadlines. Discard expired unsent requests; report unknown after ambiguous
   submission. Never replay on reconnection or duplicate correlation IDs.
5. Build a discovery aggregator over managed control registrations and generated
   native-discovery bindings. Use the five-second/four-concurrent-provider bounds,
   exact-identity deduplication, unknown states, partial discovery errors, and
   unavailable reasons specified in the spec. History and mesh presence are hints,
   not proof of local ownership, liveness, or a writable channel.
6. Keep direct automatic delivery available inside an owner when Rendezvous is
   absent. Expose missing external routing as unavailable; daemon failure must
   not fail the wrapped task. Native provider discovery remains independent.

**Validation:** Fake-controller integration tests for concurrent senders, stale
IDs, reused PIDs, duplicate observations, disconnect/reconnect, missing daemon,
queue saturation, deadlines, late replies, wrong users, and remote mesh exclusion.
Test Unix sockets and native Windows named pipes with their real ownership rules.
No original message text may enter replicated registers or durable retry storage.

**Exit:** A separate local client can route a request to exactly one live owner
and receive only the receipt strength the owner establishes. Target changes and
transport failures cannot redirect or replay work.

**Failure behavior:** Product routing regressions block live integration. Missing
native-OS runners remain explicit verification blockers; do not substitute WSL
for native Windows evidence.

## Phase 4 — Managed Pi RPC execution and adapter

**Dependencies:** Phases 1–3; existing Pi fixtures.

**Work**

1. Implement Pi's retained RPC profile in the wrapper and hand-written provider
   behavior, using the generated execution-interface selection. Reconcile
   [Pi facts](../../docs/providers/facts/pi.yaml), overrides, generated data,
   [launch profile](../../cli/src/commands/wrap/profile/pi.rs), and stream parsing.
   RPC is a control interface with structured events, not merely an output flag.
2. Preserve extensions, skills, templates, context, MCP, model, sandbox, and
   permission settings. Remove automatically injected resource-disabling flags
   from the selected managed profile. Do not conflate Pi resource trust with tool
   or extension approval and do not broaden existing permission policy.
3. Implement initial prompt, active steering, idle prompt, state/identity checks,
   acceptance correlation, and manual abort-then-submit. Preserve pending queues;
   clearing is not part of the default interruption workflow. Verify cancellation
   before revalidating and sending replacement; report partial outcomes honestly.
4. Handle unattended requests as specified: existing policy, informational
   responses, documented deny/cancel, or `input_required` failure. Never fabricate
   approval or silently disable the requesting extension.
5. Preserve existing semantic output, usage/error reporting, cancellation,
   watchdogs, and execution results. Wait for `agent_settled` before one-shot
   shutdown. Keep stdin open through settlement and retain ownership sufficient
   to clean up tools after provider failure.
6. Integrate effective session-change guards. The existing gated-switch test is
   a demonstrated counterexample to snapshot-only safety. If configured extensions
   can bypass ownership coordination, keep that profile unavailable for steering;
   preserve its resources and record the specific blocker.
7. Apply pre-submission fallback policy to a verified feature-preserving alternate
   interface. No fallback or initial-task replay after ambiguous submission.

**Validation:** Extend [real Pi tests](../../cli/tests/real_pi_steering.rs) through
Claudine's actual wrapper, not only direct Pi children. Cover all existing fixture
assertions, working/idle delivery, settlement, resources, unattended requests,
queue retention, switch races, EOF, crash, process cleanup, and ordinary wrapper
output/error parity. Keep the existing expected-failure evidence as regressions.

**Exit:** The managed RPC implementation and wrapper-level deterministic tests
pass. Activate only exact profiles with effective targeting and matching delivery
verification. A Pi profile blocked by independent extension mutation is explicitly
blocked, not advertised as safe because its basic RPC fixture passed.

**Failure behavior:** Fix wrapper regressions before enabling RPC by default.
Missing credentials or unproven guards block the affected profile; continue later
provider-independent work using fake adapters. Do not disable extensions to pass.

## Phase 5 — Manual steering CLI and selection

**Dependencies:** Phases 1–3; Phase 4 supplies the first candidate real adapter.

**Work**

1. Add the specified command forms, validation, exit codes, list JSON schema, and
   explicit-ID send JSON. Update CLI dispatch, help, argument normalization, and
   completions using existing conventions. Do not introduce send/list subcommands
   or an interruption-consent bypass flag.
2. Render Provider, Session, Directory, State, and Steering with
   `TerminalRenderable` components. Show full IDs and wrapped reasons/details;
   unavailable rows are dim, single-struck, and unselectable. Plain output retains
   clear availability labels. Sort and deduplicate according to the spec.
3. Require a selection even with one eligible session; support working and idle
   targets and explain when input starts a turn. Handle no sessions, none eligible,
   unknown state, stale selection, and user cancellation without broadcast.
4. Implement interactive interruption consent bound to the selected identity and
   action. Explain running-tool and pending-message effects, revalidate before
   cancellation and replacement, and preserve their separate outcomes. Non-TTY
   and JSON execution never infer consent. If non-interrupting delivery becomes
   unavailable, do not switch to interruption silently.
5. Return after provider acceptance and report accepted/queued/delivered only as
   established. Show held as undelivered, explain separate setup, and keep sending
   free of configuration changes or extension installation.

**Validation:** Parser and JSON-contract tests; simulated picker/confirmation
input; no-focus terminal tests for disabled styling, narrow layouts, and plain
output; integration tests for exact targeting, stale targets, late replies,
partial interruption, cancellation, and unavailable unattended consent.

**Exit:** Manual selection, explicit targeting, listing, receipts, and failures
work with shared service contracts and verified adapters. Unavailable providers
remain visible with accurate reasons. No interactive test needs a real human.

**Failure behavior:** Block command release on misdelivery, consent bypass,
incorrect receipt upgrades, or leaked text. Provider absence does not make fake
CLI tests inconclusive or justify hiding unsupported rows.

## Phase 6 — Automatic repetition warnings

**Dependencies:** Phases 1–3 and shared delivery exercised in Phase 5.

**Work**

1. Extend [ContentDetector](../../lib/src/runaway/detector.rs) with separate pure
   warning/recovery observations while preserving its terminal trip behavior.
   Warn at ceiling-half the stop limit only when repetition is established and
   before termination. A limit of one has no warning opportunity.
2. Implement recovery as `max(8, 2 * previous_cycle_length)` nonblank normalized
   semantic-output lines without detected repetition. Freeze that cycle length
   until recovery; blank lines do not advance, detected repetition resets, and
   turn boundaries/acknowledgments/silence do not reset or advance recovery.
3. Track three opportunities per agent execution, including unavailable and failed
   attempts. No same-episode retry/refund. After recovery a new episode must reach
   its own warning threshold. Retry/resume executions have distinct budgets even
   when conversation history is reused; ordinary turns retain their allowance.
4. Resolve `steering.automatic.enabled` and `CLAUDINE_AUTO_STEER` with explicit-value
   inheritance and the specified parsing/errors. Opt-out removes automatic help
   and unavailable notices, preserving guards and manual steering.
5. Wire observations through the existing live semantic sink to the producing
   execution's controller. Deliver the spec's helper text using only eligible
   non-interrupting operations. Do not attach monitoring to discovered native
   sessions or invent early warnings for capture-only paths.
6. Enforce hard-stop priority within a chunk and during delivery, bounded work,
   unavailable-notice deduplication, and unchanged timeout clocks. Do not count
   steering control traffic as semantic progress or clear repetition evidence.

**Validation:** Deterministic detector/runtime fixtures for odd/small limits,
split chunks, same-chunk warning/stop, single/multiline cycles, new repeated
blocks, brief wording changes, blank lines, recovery, turns, execution restarts,
three-opportunity exhaustion, opt-out/inheritance, unsupported boundaries,
slow/failed sends, and termination during delivery. Preserve existing expression,
volume, silence, wall-clock, and cancellation regressions. Simulate recovery
and continued repetition, proving unchanged hard-stop thresholds.

**Exit:** Early helper delivery is observable before the existing limit when a
verified boundary permits it; continued repetition still stops on the existing
schedule. All automatic outcomes are bounded and never interrupt to deliver.

**Failure behavior:** Any changed hard-stop timing or evidence reset blocks this
phase. Unavailable provider delivery emits the bounded notice and retains the
original failure path; it is not grounds to weaken detection.

## Phase 7 — Remaining provider adapters and native coverage

**Dependencies:** Phases 1–6 shared infrastructure; no dependency on Pi activation.

**Work**

1. Visit every eligible entry in [providers.yaml](../../docs/providers.yaml),
   respecting `skip_research`. Use this implementation order for the researched
   candidates: Codex, OpenCode, Claude Code, then remaining roster entries in
   roster order. Pi's managed candidate was addressed in Phase 4; include its
   native discovery/access disposition here. The roster remains authoritative.
2. For each candidate, implement researched discovery and usable delivery/control
   interfaces through shared protocol families and generated bindings. Keep
   ordinary native launches distinct from deliberately managed/server launches.
   Preserve provider features and exact compatibility probes.
3. Codex: test exact thread/expected-turn guards, idle-start separation, retained
   app-server ownership, receipt timing, and cancellation completion.
   OpenCode: test exposed-server versus internal launch access, same-user/session
   routing, early HTTP acceptance versus delivery, and partial interruption.
   Claude Code: test exact registry/socket framing, authentication per OS, inbound
   policy, correlation, and non-interactive held-message behavior.
4. Apply equivalent case-specific work to Goose, Kimi, Qwen, Kilo, Antigravity,
   and any subsequent eligible roster additions. Reuse ACP/HTTP/stdio behavior
   where protocols actually match; do not force every operation into active-turn
   steering or implement an unsupported terminal-keystroke substitute.
5. Close concrete gaps with versioned-source inspection and bounded disposable
   tests. At most two corrective research passes per gap, with the requested
   model/effort. Preserve existing evidence and activation distinctions.
6. Record each case as verified-enabled, implemented but externally blocked,
   evidenced unsupported, or unknown with a concrete next check. Missing access
   or implementation is not proof of provider incapability. Do not leave known
   implementable work unfinished under a generic unknown label.

**Validation:** Fake-protocol failure tests plus disposable exact-profile delivery
and interruption tests for every enabled binding. Include long tools/batches,
stale identity, duplicate IDs, queue/receipt ambiguity, policy holds, process loss,
resource parity, and native/managed distinctions appropriate to each mechanism.

**Exit:** Every eligible provider has an explicit implementation/verification
outcome and discovery coverage. Available mechanisms are implemented and tested;
external prerequisites alone can leave otherwise implemented cases blocked.
Unsupported and unknown cases remain visible and cannot be selected accidentally.

**Failure behavior:** Isolate unavailable credentials/binaries/hosts and continue
other providers. Do not silently change research model, configure real user
providers, or send into existing sessions. Reproducible implementation defects
remain required work, not external blockers.

## Phase 8 — Platform verification, documentation, and release gates

**Dependencies:** Implementation outputs from Phases 1–7.

**Work**

1. Run required unit/integration/lint checks for affected packages using `just test`,
   `just test-l2`, and `just lint`; use nextest for targeted runs, never `cargo test`.
   Include catalog types, generator, library, CLI, changed Rendezvous crates, and
   affected redaction consumers. Keep ordinary L1 provider spawning hermetic;
   real-provider targets remain explicit opt-in tests with `real-tests`.
2. Verify build and local IPC behavior on macOS, native Linux, and native Windows.
   Run provider delivery tests on each OS/version/profile before enabling that
   combination. Linux/WSL results never stand in for native Windows; compilation
   alone never satisfies a live-delivery assertion.
3. Exercise a production-wrapper manual send, an automatic warning followed by
   recovery, and continued repetition reaching its original stop limit. Exercise
   native discovery/delivery independently wherever enabled. Test daemon absence,
   slow routing, controller/provider failure, cleanup, and no accidental replay.
4. Run the maintained all-provider steering checker, schema validation, activation
   checks, and deterministic generation/drift checks. Verify that expected-loss
   artifacts and stale adapter revisions cannot activate delivery.
5. Update public README/help, CLI and timeout/signal topics, local IPC docs,
   provider setup/compatibility guidance, and the Claudine skill. Update dependency
   docs only for actual dependency changes. Produce the cross-provider research
   summary through the existing summary/publication workflow.
6. Update this plan's progress, the spec status, uncertainty register, and fleet
   report with final implemented/enabled/blocked coverage. Preserve evidence paths,
   exact versions, OS and launch profiles, and remaining external requirements.

**Validation:** Record the package, platform, end-to-end delivery, activation,
generation, and documentation checks above with actual results. Check public
examples against the implemented parser and JSON schema. Missing external tests
remain explicitly unrun; previous research results do not replace wrapper tests.

**Exit:** Required code checks pass; user-visible behavior and generated metadata
agree; every enabled capability has matching delivery and ownership evidence.
At least one real production-wrapper path demonstrates manual delivery and
non-interrupting automatic help before claiming the feature operational. Do not
claim universal support, complete platform activation, or successful delivery for
cases still externally blocked.

**Failure behavior:** Fix product regressions before declaring implementation
complete. If external resources prevent required verification, finish independent
checks and documentation, retain unavailable defaults, and report the exact blocked
cases. The plan finishes with an honest readiness report, not an interactive
permission question or a fabricated all-green result.

## Progress

| Phase | Status | Evidence |
| --- | --- | --- |
| Phase 1 | Not started | Existing research/checker are the baseline |
| Phase 2 | Not started | Shared recognition sources identified |
| Phase 3 | Not started | Existing local IPC contract is the baseline |
| Phase 4 | Not started | Seven scoped Pi research records; wrapper verification required |
| Phase 5 | Not started | CLI contract resolved in spec |
| Phase 6 | Not started | Warning, recovery, and cap decisions resolved in spec |
| Phase 7 | Not started | Full-roster passive research available |
| Phase 8 | Not started | No new implementation validation claimed by this plan |
