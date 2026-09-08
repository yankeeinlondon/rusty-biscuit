# Steering Uncertainty Register

Updated: 2026-09-08

This register separates contract shape from provider evidence and implementation.
It is based on the revision-3 reports. Pi now has two passing, narrowly scoped
macOS fixture records from the initial pass, two abort/EOF regression records,
two controlled switch/provider-kill records, and one external-tool cleanup record;
the other providers have no live-test records. No production
mechanism is activated. “Representable” below means the research schema can state the fact;
it does not mean the fact has been established for a provider.

The required scope remains both provider-native sessions and Claudine-managed
sessions. A managed profile is not evidence that an already-running native
session can be attached, and lack of native attachment evidence is not evidence
that the provider cannot ever support it.

## Resolved contract gaps

The controlled Pi switch experiment resolves one concrete targeting question:
an old-session state query and successful steering acknowledgment can both occur
while a switch is pending, followed by message loss at switch completion. The
controller must coordinate sends with switches; external or extension-driven
switches still need an effective guard or a profile limitation. A provider kill
after acknowledgment also loses the queued text before history/model consumption.
These are measured failure cases, not evidence of safe targeting or crash recovery.
See [the scoped results](verification/pi-macos-0.84.4-switch-crash.json).

These earlier pilot gaps are expressible in revision 3 and should not drive
another schema revision unless a concrete provider fact cannot fit the existing
vocabulary:

- Launch-dependent reachability is represented by `launch_profiles`, profile IDs
  on cases, access findings, and compatibility records, `endpoint_scope`, lifetime, origins, launch modes,
  startup requirements, and `interface_inventory`.
- Acceptance, persistence, scheduling, conversation delivery, and receipt timing
  are separate in `receipt_guarantees`, `delivery_states`, and
  `receipt_observations`. An early receipt can therefore remain weaker than
  queued or delivered.
- Provider execution state is separate from delivery state. Discovery records
  carry observation source and time, while target preconditions and guards can
  express active-operation eligibility. Later tool or permission holds need not
  be mislabeled as held inbound delivery.
- Sender correlation and retry uncertainty are represented by
  `sender_message_id`, `correlation`, `retry_policy`, and `duplicate_handling`.
- Interrupt-then-submit is represented as a distinct operation intent with
  `interruption_phases` and `interruption_partial_failure`.
- Missing profile combinations are retained explicitly through the interface
  inventory and unknown cases. Schema coverage therefore no longer requires a
  broad verdict that conflates native and managed launches.

Sources: [revision-3 schema](../../docs/research/steering/_schema.yaml),
[fleet run, Revision 3 Follow-Up](fleet-run.md#revision-3-follow-up), and
[specification, Schema Refinements](spec.md#schema-refinements-required-by-the-second-pilot).
Some `gaps` prose in the Codex and OpenCode reports still describes revision-1
expressibility problems. Treat those entries as historical rationale where the
fields above now exist; retain their provider-specific unknown facts.

## Provider evidence unknowns

The [abort/EOF follow-up](verification/README.md#abort-and-stdin-eof-follow-up)
establishes cooperative cancellation, queue clearing before abort, preserved
steering history affecting an explicit later turn, independent rejection of a
malformed replacement, and loss of acknowledged steering on stdin EOF. The
switch/provider-kill follow-up adds controlled queue loss during a pending switch
and after abrupt termination. A separate built-in bash experiment confirms abort
removes one marked external process, while killing Pi leaves it running until
fixture cleanup. Remaining questions include additional subprocess descendants
and detached work, failures after replacement acceptance, full-duplex loss, controller/host
failure, restart recovery, and broader concurrent scheduling.

### Priority 1: Pi

The [first verification pass](verification/README.md) passed on Pi 0.84.4 with
a deterministic local model in a real RPC child. It establishes early admission,
full tool-batch delivery, same-session continuity, duplicate-ID behavior, fixture
resource loading, extension handling without model invocation, session identity
replacement, and an unanswered extension confirmation. The checks below remain
open for broader profiles and failure/race conditions; successful fixture loading
does not establish compatibility with every user extension or cloud backend.

Pi is the first disposable-harness target because retained RPC exposes explicit
steer, idle prompt, abort, queue, activity, and settlement signals. The evidence
now proves same-conversation continuity in the fixture, but does not establish
safe operation across the production profiles Claudine would enable.

- **Native ordinary sessions:** no peer endpoint or attachment contract connects
  an ordinary CLI PID or persisted session file to a writable live conversation.
  Next check: passively confirm this for the tested release, then keep native
  ordinary sessions discoverable but unavailable unless a provider attachment
  interface appears.
- **Managed target identity:** RPC reports mutable current-session state and has
  no expected session or operation guard. Next check: in a retained disposable
  RPC child, race `switch_session`, `new_session`, and `fork` against steer and
  require Claudine to reject any changed target.
- **Receipt boundary:** steer and prompt return early responses; queue events,
  model-visible incorporation, `agent_end`, and `agent_settled` are later facts.
  Next check: correlate a unique nonce across the response, queue update,
  transcript, assistant response, and settlement. Test disconnect and ambiguous
  response without retry.
- **Delivery boundary and limitation:** source says steering drains after the
  assistant response and the complete tool batch. This is a known boundary for
  the examined source and the live two-tool parallel fixture. Behavior under
  prolonged generation and sequential batches remains unverified. Next check: bounded
  generation, long-tool, and sequential-batch fixtures; do not classify Pi steering as
  loop rescue when the boundary never arrives.
- **Interruption:** abort and replacement are separate, abort retains queues, and
  replacement can fail after cancellation. Next check: test queue cleanup,
  completed side effects, process cleanup, idle races, and deliberate replacement
  failure. This path remains manual-consent-only.
- **Input and feature parity:** skills/templates and extensions can transform or
  reject input, and unattended extension UI handling is unresolved. Next check:
  compare native and retained-RPC inventories and behavior for literal text,
  slash-prefixed text, skills, templates, extensions, context, MCP, model,
  sandbox, and permissions.
- **Platform/version scope:** installed-source and live fixture evidence is macOS Pi 0.84.4 only.
  Next check: repeat compatibility and disposable tests on native Linux and
  native Windows; record exact version and launch profile per result.

Exact evidence: [Pi report](../../docs/research/steering/pi.md) evidence IDs
`official-rpc`, `source-rpc`, `source-session`, `source-loop`, `source-settled`,
`local-pi`, and `wrapper`; [official RPC documentation](https://pi.dev/docs/latest/rpc);
[pinned RPC source](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/modes/rpc/rpc-mode.ts);
[pinned session source](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session.ts);
and [pinned loop source](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/agent/src/agent-loop.ts).

### Priority 2: Codex and OpenCode

For Codex, managed app-server is a candidate while ordinary CLI attachment is
unknown. Next checks are: retain and register Claudine-owned app-server transport;
verify `threadId` plus `expectedTurnId` immediately before `turn/steer`; test stale
turn, idle, review/compaction, and subagent rejection; correlate
`clientUserMessageId` with item and turn notifications; and test stdio plus each
supported local transport on every native OS. Evaluate the provider-native
next-turn queue as a separate mechanism rather than treating it as active steer.
Interrupt and replacement must be tested and reported as independent phases.

Exact evidence: [Codex report](../../docs/research/steering/codex.md) evidence IDs
`official-app-server`, `local-help-0-153-4`, `local-schema-0-153-4`,
`wrapper-inspection`, and `final-race-inference`; [versioned app-server protocol](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/app-server/README.md).

For OpenCode, exposed HTTP profiles are candidates while ordinary TUI and
ordinary `run` have no established external attachment. Next checks are: use a
Claudine-owned authenticated server registration; bind endpoint ownership to the
same OS user and selected conversation; probe compatibility through documented
health/schema surfaces; correlate a chosen message ID through history, SSE, and
model-visible output; and test busy generation, long tools, multi-tool turns,
idle races, disconnect ambiguity, and abort-then-prompt partial failure. An async
204 establishes request handling only. Synchronous prompt completion is a
terminal response and does not satisfy the requested prompt return-after-acceptance
behavior without a separate early acknowledgment.

Exact evidence: [OpenCode report](../../docs/research/steering/opencode.md)
evidence IDs `official-server`, `official-cli`, `source-launch-1-18-29`,
`source-api-1-18-29`, `source-runner-1-18-29`, `source-auth-1-18-29`, and
`local-sniff-1-18-29`; [official server documentation](https://opencode.ai/docs/server/);
[pinned session handler](https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts);
and [pinned prompt runner](https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/effect/runner.ts).

### Native Claude Code track

Claude Code is distinct because the provider documents native peer messaging for
ordinary interactive and retained non-interactive sessions. This should be
tested as provider-native discovery and delivery, not converted into a
Claudine-managed-server design.

- On macOS/Linux, test registry-to-socket identity, process-start freshness,
  working/idle state transitions, raw frame negotiation, correlated status, held
  and refused policy outcomes, next-tool-boundary delivery, and idle new-turn
  delivery. A socket write alone proves nothing beyond the write.
- On native Windows, the target-generated token has no supported external
  credential path in current evidence. Keep delivery unavailable unless Anthropic
  supplies a controller path or a provider-owned relay can be tested without
  scraping process environments.
- Ordinary one-shot sessions remain distinct from retained `-p` sessions. A
  completed one-shot is history, not an active idle target.

Exact evidence: [Claude Code report](../../docs/research/steering/claude.md)
evidence IDs `official-cross-session`, `local-registry-2-1-263`,
`local-binary-2-1-263`, and `official-cli`; [cross-session messaging documentation](https://code.claude.com/docs/en/cross-session-messaging).

### Remaining providers

- **Goose:** managed ACP steering has correlated queued/pickup signals, but only
  drains between model/tool turns; ordinary attachment, long-generation rescue,
  transport behavior, and guarded cancellation remain unknown. Next check:
  retained-stdio versus exposed-server ownership plus exact v1.49.0 disposable
  tests. Source: [Goose report](../../docs/research/steering/goose.md).
- **Gemini:** managed ACP supports idle prompts and interrupt-then-prompt, with no
  non-interrupting active steer found. Next check: new versus loaded conversation
  identity, terminal-only prompt receipt, cancellation cleanup, and parity with
  ordinary extensions, skills, templates, context, hooks, and tools. Source:
  [Gemini report](../../docs/research/steering/gemini.md).
- **Kimi:** managed web and ACP interfaces have different semantics; ordinary
  attachment is unknown. Next check: compare web next-step steering with ACP
  cancellation/replacement under generation, tools, compaction, transformations,
  ambiguous receipts, and all native OSes. Source: [Kimi report](../../docs/research/steering/kimi.md).
- **Qwen:** managed daemon follow-ups are admitted for a later turn and cannot
  rescue a current turn that never ends. Next check: queue order/retention,
  multi-tool and cancel races, SSE gaps, duplicate ambiguity, profile parity, and
  ordinary-session discovery. Source: [Qwen report](../../docs/research/steering/qwen.md).
- **Kilo:** managed HTTP async input reaches processing only after current stream
  and inline tools drain; ordinary client/server/session attribution is unknown.
  Next check: correlate 204, SSE, history, and model output; test active-operation
  races, prompt transformations, abort partial failure, daemon ownership, and the
  examined release separately from the future V2 inbox. Source:
  [Kilo report](../../docs/research/steering/kilo.md).
- **Antigravity:** retained stream-JSON is currently an idle-only candidate with
  unknown framing and receipt timing; active delivery, Remote Control/internal
  RPC access, native attachment, feature parity, and other OSes are unresolved.
  Next check: obtain a versioned structured-input contract before testing
  correlated input, then separately investigate documented Remote Control access.
  Source: [Antigravity report](../../docs/research/steering/antigravity.md).

## Implementation work after evidence passes

1. Add generated, profile-specific steering metadata and keep unsupported or
   unverified cases visible without making them selectable.
2. Reuse rendezvous local identity and transport facilities for Claudine-managed
   registration, leases, liveness, process-start identity, and same-user checks.
   Provider-native registries remain provider adapters rather than being replaced.
3. Implement provider adapters with compatibility probes, target guards,
   acknowledgment-strength reporting, correlation, and fail-closed ambiguous
   outcomes. Do not retry unless idempotency is proven.
4. Add `claudine steer`, active-session listing, disabled-row rendering, explicit
   targeting, and the interactive interruption-consent path. Keep setup separate
   from sending.
5. Add reusable secret redaction and steering logs that preserve full redacted
   text, target identity, launch profile, receipt strength, delivery observations,
   and partial interruption outcomes.
6. Add early repetition episodes, threshold rounding, recovery-window fixtures,
   the execution-local warning cap, configuration precedence, and non-resetting
   guard/timeout behavior. Automatic delivery must use only verified
   non-interrupting mechanisms.
7. Build non-focusing disposable tests keyed by provider, exact version, native
   OS, profile, origin, launch mode, state, and mechanism. A passing record supports
   only the behavior it actually tested; expected-loss assertions do not enable
   delivery, and adapter review remains a separate gate.

Sources: [feature specification](spec.md), [fleet implementation gates](fleet-run.md#outstanding-implementation-gates), and [Rendezvous local IPC contract](../../docs/rendezvous/local-ipc.md).

## Known limitations that tests should preserve

- Pi steering waits for the full tool batch; Goose drains at a model/tool
  boundary; Qwen follow-ups start at the next turn; Kilo async input waits for the
  current stream and inline tools. These mechanisms cannot be advertised as
  recovery from a boundary that never arrives.
- Gemini active replacement and the researched ACP cancellation paths interrupt
  before submitting replacement work. Automatic loop intervention cannot use
  them, and manual use requires explicit consent.
- Codex `turn/steer` applies only to the exact eligible active regular turn;
  starting an idle turn is a different operation.
- OpenCode and Kilo early HTTP receipts do not establish model delivery. Claude
  Code raw socket writes and Antigravity stdin writes likewise establish no
  provider acceptance without a correlated provider signal.
- Native discovery and managed discovery remain separate on every provider.
  Session history never proves liveness, ownership, state, or reachability.
- Every current mechanism remains disabled until a matching disposable-test
  record passes. This activation gate is a product requirement, not a schema
  limitation.
