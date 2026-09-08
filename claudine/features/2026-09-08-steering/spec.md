# Steering Running Agent Sessions

Status: Draft — interactive requirements discovery; not ready for implementation.
Created: 2026-09-08
Updated: 2026-09-08

## Purpose

Let a user course-correct a running agent through `claudine steer "message"`.
Use the same delivery capability to warn an agent when Claudine detects early
signs of repetitive output, giving it an opportunity to recover before the
existing runaway guard terminates it.

Provider support must come from evidence-backed fleet research and generated
metadata. A provider's ability to resume a completed conversation does not prove
that it can receive a message during an active run.

## Confirmed Requirements

- Add `claudine steer {msg}` with an interactive list for choosing the intended
  active session. One invocation sends to the selected session.
- Support an explicit session ID to bypass the picker, plus a way to list session
  IDs for scripts. Never infer an automation target. If delivery requires
  interruption, explicit targeting does not grant permission to interrupt:
  require the interactive choice, or fail with an explanation when interaction
  is unavailable. Exact command flags and listing format remain to be specified.
- Return from `claudine steer` once the provider confirms acceptance. Report
  “queued” when that is all it confirms; claim delivery only with evidence of
  delivery. Do not wait for a later tool boundary, agent response, or completed
  turn. A successful socket write alone is not confirmed provider acceptance.
- Discover sessions on the same host under the same OS user.
- Include both sessions launched through Claudine and sessions launched directly
  through a provider. Discover and steer each wherever technically possible;
  explain provider or launch-mode limitations for discovered unavailable sessions.
- Include both actively working sessions and open sessions waiting for input.
  Mark waiting sessions as idle and explain when sending a message starts their
  next turn. Ended conversations retained only in history are not active sessions.
- Keep provider setup separate from `claudine steer`. When steering requires
  configuration, an extension, or special startup options, explain the missing
  prerequisite and offer a separate setup path. Sending a message must not
  install extensions or mutate provider configuration. Research determines the
  concrete setup guidance; this decision does not require a new setup command.
- List discovered active sessions even when they cannot be steered. Their rows
  must be de-emphasized and rendered with a single strikethrough, and selection
  must be disabled. Also explain why they are unavailable in text.
- Investigate every eligible entry in [the provider roster](../../docs/providers.yaml).
  Respect `skip_research`; do not maintain a separate research enumeration.
- Add an early automatic helper message for suspicious output. Existing stuck-agent
  termination remains required if the session does not recover.
- Warn before the existing stop limit without extending it. Sending, accepting,
  or delivering a warning must not reset guard evidence or timeout clocks.
- Manual steering prefers non-interrupting delivery. When a provider requires
  interruption to inject the message, explain that limitation and interactively
  offer the user the choice to interrupt and deliver, or cancel. Do not interrupt
  without that explicit choice.
- Automatic loop intervention never interrupts to deliver steering. If the
  provider cannot support non-interrupting delivery, warn on STDERR that steering
  cannot be sent and continue enforcing the existing error limit.
- Undocumented provider messaging mechanisms are permitted with research-backed
  compatibility checks. Require evidence for supported versions and verify the
  actual session's interface before use. Failed or inconclusive checks leave that
  mechanism unavailable; do not optimistically send through an unknown protocol.
- Every steering mechanism, documented or undocumented, must pass a live test
  against a deliberately created disposable agent session before being enabled.
  Verify delivery to the intended conversation and the claimed interruption
  behavior. Documentation, source inspection, and compatibility checks alone
  cannot satisfy this activation gate. Untested mechanisms remain visibly
  unverified and unavailable for manual or automatic steering.
  Test evidence applies only to the OS, version, launch conditions, and behavior
  it establishes; do not imply untested cases passed.
- Automatic loop warnings are enabled by default, with an explicit opt-out.
  Opting out disables automatic steering attempts and their unavailable-delivery
  warnings, but does not disable loop detection, existing stop limits, or manual
  `claudine steer`. Configure this in user or repo configuration, with repo values
  overriding user values. An environment variable overrides both at runtime.
  Precedence is environment > repo config > user config > enabled built-in default.
  Exact key and environment-variable names remain to be specified. CLI flags and
  task-document frontmatter overrides are not part of this agreed configuration
  surface.
- Trigger an early repetition warning halfway to the configured repetition stop
  limit: 15 full repetitions for the default limit of 30. Integer rounding for
  small or odd limits remains to be specified; a warning
  must never delay or supersede a hard stop.
- Allow one automatic warning per separate repetition episode, subject to a total
  default cap of three warnings per agent execution. Ordinary conversation turns
  do not reset the allowance. Separate agent executions, including separate tasks
  in a sequence, have independent allowances; the cap is not shared across the
  outer Claudine command or persisted across executions of a resumed conversation.
  A new episode requires a sustained break in repetition; a single different
  line or brief wording change does not qualify. The measurable recovery window
  will be defined using representative detector fixtures. This warning-eligibility
  rule does not change the existing detector's hard-stop behavior. Reaching the
  cap does not change detection or termination limits.
- Log full steering message text after heuristically replacing potential secrets
  with `*` characters, together with delivery details. Implement secret detection
  as shared, reusable functionality; reuse or consolidate existing heuristics
  rather than creating a steering-specific duplicate.
- Follow [the typed knowledge pipeline](../../docs/topics/agentic-research-as-a-typed-knowledge-pipeline.md):
  per-provider prose and schema-validated frontmatter, deterministic consumption,
  generated provider metadata, and a cross-provider summary.
- Refine this specification interactively before implementation. Execute research
  after its schema and prompts are settled sufficiently to produce useful evidence.
  Research tasks must use `gpt-5.6-sol` with low thinking.
- Claudine must compile and work on macOS, Linux, and Windows. Provider-specific
  limitations must be represented per OS rather than inferred from this Mac.

## Existing Behavior and Integration Grounding

Verified from repository source and documentation on 2026-09-08:

- [`ContentDetector`](../../lib/src/runaway/detector.rs) is a pure, stateful
  detector. It assembles lines across stream chunks and recognizes repeated
  groups of lines. Its configurable default is 30 full repetitions, with groups
  up to 16 lines. Separate volume limits default to 50,000 lines or 32 MiB.
  `reset_turn` resets volume counters but preserves repetition history and cycle
  state, so a turn boundary must not be interpreted as repetition recovery.
- Semantic content guards observe assistant output and reasoning, excluding tool
  arguments and results. The capture path has a volume cap, without live
  repetition detection. See [timeouts](../../docs/topics/timeouts.md).
- Explicit exit expressions, repetition, and volume guards produce terminal
  outcomes. A warning must be a separate nonterminal observation, not a terminal
  error disguised as recovery.
- Silence and wall-clock timeouts are separate controls. Sending a steering
  message must not count as agent progress or refresh their clocks.
- Provider metadata in `lib/src/provider/<slug>/data.rs` is generated through
  `claudine-gen`; provider behavior is hand-written separately. Extend that
  pipeline rather than adding a competing capability table or scattered provider
  switches. See [provider metadata](../_completed/2026-07-02-provider-metadata/spec.md).
- Rendezvous already provides a per-user local endpoint abstraction: Unix-domain
  sockets on macOS/Linux and named pipes on Windows. Its existing
  [local IPC contract](../../docs/rendezvous/local-ipc.md) is the reference for
  ownership, identity, and connection handling. Reuse suitable existing discovery
  and transport facilities after tracing their actual implementation; do not
  assume session history is proof of a live, reachable session.

The user's Claude Code report describes a PID registry under `~/.claude/sessions/`,
authenticated sockets under `/tmp/cc-socks/`, and delivery at the next tool round.
These are **unverified research leads**, including the claims about non-interactive
availability, authentication, idle notices, and the meaning of registry fields.
No provider has been classified as steering-capable by this specification yet.

### Claude Code Pilot Findings

The [completed passive pilot](../../docs/research/steering/claude.md) corroborates
the session registry and peer sockets on macOS with installed Claude Code 2.1.263.
It records all 24 baseline cases and passes schema and relational review, with
no live verification records and therefore no activated mechanism.

[Official cross-session documentation](https://code.claude.com/docs/en/cross-session-messaging)
establishes non-interrupting delivery between tool calls and new-turn delivery
for idle sessions. Long-running non-interactive sessions have inboxes unless
launched in bare mode. Contrary to the original sibling-key account, token
authentication is optional on macOS/Linux and mandatory on native Windows.
The earlier interim concern about absent `.key` files therefore does not block
Unix delivery; a supported external credential path remains unresolved on Windows.

Inbound policy may hold or refuse messages. Held messages require approval or a
policy change and are not merely queued for the next tool boundary. Raw response
framing and acknowledgment correlation still need implementation-level evidence.
Claude Code non-interactive sessions cannot display the approval dialog. A held
message therefore has no human approval path in that session; default-held
messages expire unless a policy/mode change permits delivery, while explicitly
held messages can remain until session end. Do not offer “approve in the receiving
session” as guidance for non-interactive targets. A known hold/refuse policy makes
unattended steering unavailable; explain the separate setup needed to accept
messages. If a send unexpectedly reports held, report it as undelivered and
requiring a policy change, never as queued for automatic delivery. Do not interrupt
or change provider policy to bypass its inbound controls.
The pilot identified schema gaps around access prerequisites and acknowledgment
lifecycle; refine those before full-roster research. No credentials were recorded
and no active delivery probe was made.

### OpenCode Second Pilot Findings

The [OpenCode pilot](../../docs/research/steering/opencode.md) examined installed
v1.18.29 and pinned source at `16747470f976aca3d362ad730bcd3fe82ecc2c9a`.
It produced all 24 baseline cases and identified further schema refinements
needed before full-roster research. No live delivery tests were performed.

**Launch configuration changes reachability.** The pinned implementation uses
internal transport for an ordinary TUI launch unless network options are supplied;
ordinary `run` also uses an in-process server. Explicitly exposed TUI servers,
`serve`, and `run --attach` have different reachability and lifetimes. Claudine's
current [prompt delivery](../../cli/src/commands/wrap/profile/opencode.rs) passes
non-interactive tasks positionally to `run`; it must not assume an external
endpoint exists. The report records the difference between broad public server
documentation and the pinned implementation instead of treating documentation
wording as proof of current endpoint availability.

**Acceptance is weaker than scheduling.** The pinned
[`promptAsync` handler](https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts)
forks prompt handling without awaiting message persistence before returning 204.
The [runner](https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/effect/runner.ts)
waits for an existing run when busy rather than scheduling a separate run.
The research identifies possible later tool-loop consumption and possible lack
of consumption when no further iteration occurs. These are source-derived
possibilities requiring disposable tests, not observed successful steering.
HTTP acceptance therefore cannot be labeled queued or delivered.

**Interruption is a multistep operation.** Abort and replacement-message submission
are separate requests. If the latter fails, the original work may already be
stopped. Manual reporting must preserve that partial outcome, and automatic
warnings must never use the fallback.

**A server is not a session or an OS user.** One server can host multiple
conversations; a stored conversation may outlive its client. Loopback HTTP alone
does not establish the required user boundary. Research and runtime checks must
identify the owning process/user, routing context, conversation, and authentication
conditions independently.

The current broad cases remain unknown where they combine incompatible launch
configurations. This is not evidence that OpenCode lacks all steering capability.
Missing live verification separately blocks activation even when documentation
or source establishes a candidate capability.

### Schema Refinements Required by the Second Pilot

These pilot findings informed schema revision 2, now authored in the sidecar.
The full-fleet refresh migrates the pilot reports alongside the remaining roster.
Do not generate runtime eligibility from the former broad case summaries.

1. **Represent launch profiles explicitly.** Introduce provider-defined, stable
   profile IDs describing how the process starts, whether its endpoint is internal
   or externally reachable, and its lifetime. Capability cases must refer to a
   specific profile. Preserve OS/origin/mode/state coverage without assigning one
   verdict to both ordinary and server-attached launches. Generator eligibility
   consumes specific cases; any broad matrix is a derived summary, not competing
   source data.
2. **Record what each acknowledgment proves.** Separate request acceptance,
   message persistence, scheduled delivery, and confirmed conversation delivery.
   Record provider signals and evidence for each fact, with explicit unknowns.
   Preserve the user's return-after-acceptance decision: return promptly, but say
   only accepted when the protocol confirms no more. Avoid inventing a queued
   state or equating a stored message with model consumption.
3. **Separate delivery state from execution state.** A message can reach the
   agent before its requested tool is blocked by permissions. Incoming-message
   approval holds and later tool/question holds are different observations.
   Runtime session observations need a source, observation time, and explicit
   unknown state; idle cannot be inferred from mere absence in a busy-session map
   or presence in stored history.
4. **Make correlation and retry semantics machine-readable.** Capture whether the
   sender may choose a message ID, how acceptance/delivery events correlate, and
   whether replaying that ID suppresses duplicates. Unknown duplication behavior
   must not become a presumed safe retry policy.
5. **Model partial interruption outcomes.** Describe whether interruption and
   submission are one operation or separate phases, evidence of stopping, and
   what the caller can observe if delivery fails after interruption. Preserve
   those outcomes in diagnostics and steering logs.

Prefer a small shared vocabulary plus provider-specific evidence records over
an ever-growing enum mirroring every provider's internal state. Research should
surface unsupported vocabulary for review rather than filling a superficially
valid record that loses the behavior needed by the generator.

### Codex Third Pilot Findings

The [Codex pilot](../../docs/research/steering/codex.md) examined installed
`codex-cli 0.153.4`, local help/schema, and official versioned documentation.
The initial pilot used revision 1 and identified further distinctions for the
revised contract; the linked report has since been refreshed to revision 2.
No live delivery tests were performed.

The [versioned app-server protocol](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/app-server/README.md)
defines `turn/steer` for an exact active regular turn, requiring `threadId` and
`expectedTurnId`. A mismatch, idle thread, or ineligible turn rejects steering.
`turn/start` starts work; `thread/resume` loads a conversation; neither is an
automatic substitute for rejected steering. Interruption acknowledgment is
separate from the later completion event. The protocol also distinguishes stdio
JSONL from WebSocket framing over Unix sockets and marks TCP WebSocket transport
experimental/unsupported. These transport facts do not establish external access
to arbitrary ordinary CLI sessions.

The current [Claudine wrapper](../../cli/src/commands/wrap/profile/codex.rs)
launches `exec` with an initial stdin prompt and uses `exec resume` for resumption.
Research did not establish an external steering channel into those ordinary
executions. A managed app-server profile is a distinct integration, and retained
stdio requires its owning process to expose an authorized local control path.

The broad cases remain unknown where revision 1 combines unmanaged and managed
profiles. That ambiguity is separate from the live-verification activation gate.
Accepting input for an active turn does not prove that a model generating tokens
without a further boundary will consume it in time to escape a repetition loop.

### Additional Schema Refinements from Codex

Codex reinforces OpenCode's launch-profile, acknowledgment, message-correlation,
and partial-interruption findings. It adds these requirements:

1. **Target preconditions and concurrent changes.** Represent conversation identity
   separately from the current turn/operation identity. Capture how to obtain
   that identity, the required expected-ID check, and stale-target outcomes.
   Rejection must not silently retarget later work or become a new-turn request.
2. **Operation intent and eligibility.** Distinguish steering current work,
   starting an idle turn, queuing a follow-up, and interruption followed by input.
   A target's role and current activity can prohibit direct input despite being
   active. Record those restrictions and their observable checks.
3. **Transport, framing, and maturity.** Store carrier transport, wire framing,
   initialization handshake, authentication, and provider-declared experimental
   or unsupported status separately. Documented does not imply stable or suitable
   for production, and a Unix socket does not imply raw newline-delimited JSON.
4. **Read-only discovery versus loading.** List and inspect already-loaded
   conversations using passive APIs. Discovery and compatibility checks must not
   resume, load, start, or fork work merely to make a target appear reachable.
   Stored history alone is not a live session. Capture control-channel ownership
   and whether an external client can reach that existing owner.

These changes are now represented in schema revision 2. Validate the revised
contract against all four pilot reports before accepting full-roster results.
Live tests must additionally cover stale-turn races, ineligible turns,
long-running tools and generation, and interruption-completion acknowledgment.

## Decisions in Progress

The table records accepted product decisions. Remaining implementation choices
are called out explicitly and must not override those decisions.

| ID | Question | Recommendation | Status |
| --- | --- | --- | --- |
| D1 | Include sessions launched directly through the provider? | Include both native and Claudine-launched sessions wherever reliable discovery and delivery exist; explain launch requirements for other sessions. | Accepted by user: both |
| D2 | Does an automatic warning extend the existing stop threshold? | Warn early and preserve all existing stop thresholds and clocks. Warning frequency is a separate decision. | Accepted by user: warn earlier, keep existing limit |
| D3 | Which delivery mechanisms may count as steering? | Manual steering offers an explained, interactive interruption fallback; automatic warnings require non-interrupting delivery, otherwise warn on STDERR and retain the existing error limit. | Accepted by user |
| D4 | May production use an undocumented provider protocol? | Allow research-verified mechanisms with version-specific evidence and runtime compatibility checks; incompatible or unverified mechanisms remain unavailable. | Accepted by user: yes, with compatibility checks |
| D5 | When and how often should automatic warnings fire? | Warn halfway to the configured repetition stop limit (15 of the default 30), once per separate episode, with a default cap of three warnings per agent execution. Require sustained non-repeating output before a new episode. Conversation turns do not reset the cap; separate executions have independent allowances. Recovery-window sizing and integer edge cases remain implementation details to specify and test. | Threshold, per-episode policy, three-warning default, execution boundary, and sustained-recovery rule accepted by user |
| D6 | What happens without an interactive terminal? | Support explicit session IDs and a listing usable by scripts; never guess a target. Interruption still requires an interactive choice. Exact flags remain open. | Accepted by user: explicit session ID |
| D7 | Should automatic help be enabled by default and configurable? | Enable by default; environment override > repo config > user config > built-in default. Opt-out is independent of existing guards and manual steering. Exact names remain to be specified. | Default, opt-out, and configuration precedence accepted by user |
| D8 | What message content should be retained in logs? | Retain full message text with potential secrets heuristically masked using `*`, plus delivery details. Share reusable detection logic rather than duplicate it for steering. | Accepted by user |
| D9 | Is a live disposable-session test required before enabling a mechanism? | Require it for every mechanism, documented or undocumented; verify same-conversation delivery and interruption behavior. Untested cases remain unverified and unavailable. | Accepted by user |
| D10 | Where should prerequisite setup happen? | Explain missing prerequisites in `steer` and offer setup separately; do not turn message delivery into a configuration workflow. | Accepted by user |

### Pi Final Pilot Findings

The user selected Pi as the fourth and final pilot before schema revision.
The current [Pi wrapper](../../cli/src/commands/wrap/profile/pi.rs) supplies an
initial stdin prompt, while generated non-interactive flags select print mode
and JSON event output. This is distinct from Pi's separately documented RPC
mode; JSON output alone does not establish bidirectional message delivery.
The [completed passive pilot](../../docs/research/steering/pi.md) examines official
v0.84.4. Steering waits for the full current assistant tool batch, including
remaining sequential or parallel calls. Acceptance does not prove incorporation.
Abort retains pending queues; clearing them is a separate operation. The RPC
connection targets a mutable current session without an expected-session guard,
and skill/template interpretation can change message text. These require explicit
fields and tests in the revised research contract. No live delivery was tested.

In response to the user's question about RPC versus JSON output, the recommended
managed-launch design is Pi RPC mode for executions Claudine needs to steer.
RPC retains structured event output while adding inbound commands; JSON output
mode alone does not supply an ongoing command channel. This is a proposed launch
change subject to pilot review and mandatory live verification, not implemented
behavior or proof that independently launched sessions become attachable.

Migration requires retained stdin ownership, command/event demultiplexing,
response correlation, existing semantic-stream compatibility, unattended handling
of extension UI requests, and explicit completion/EOF/cleanup behavior. Route
external `steer` requests to that owner over the established private local control
boundary; do not have competing readers or writers attach to the child's pipes.

The full-fleet review corrected an earlier pilot assumption: official v0.84.4
`AgentSessionEvent` includes `agent_settled`, and the RPC subscription forwards it.
Do not close the process at the earlier `agent_end` event while retries,
compaction, queued continuation, or extension settlement remain. The refreshed
[Pi execution research](../../docs/research/non-interactive-sessions/pi.md)
records the source-backed completion distinction and shutdown requirements.

**User correction:** Pi must retain extensions, skills, prompt templates, and
context files. Claudine must not automatically inject `--no-extensions`,
`--no-skills`, `--no-prompt-templates`, or `--no-context-files` as a consequence
of selecting structured output or RPC mode. Respect explicitly authored provider
settings rather than disabling these features for unattended execution.
The earlier suggestion to preserve the current disabling flags is superseded.
Permission handling, including the existing `--no-approve` behavior, is a separate
policy concern and is not implicitly authorized to change by this correction.

Research and disposable tests must exercise enabled extensions and resources.
Determine how extension UI requests behave without a human response path, and
how skill/template expansion or extension-command interpretation affects a
steering message. Do not solve those questions by silently disabling the features.

### Follow-on Research: Prefer Bidirectional Control

The user requested revisiting the fleet research that selected JSON instead of
RPC. The proposed policy is to prefer a usable RPC/control interface for managed
execution, including one-shot work. RPC and JSON are not competing encodings:
a control protocol can carry both commands and streaming JSON events.
Availability must be established for the actual OS, version, and launch profile,
with provider features preserved. Lack of an existing Claudine adapter is an
implementation gap, not evidence that a provider interface is unsuitable.

The causal trail is concrete:

- The [non-interactive fleet prompt](../../docs/research/non-interactive-sessions/_fleet.md)
  asks researchers to choose an output format first and prioritizes streaming
  over request/reply, without requiring comparison of two-way control plus events.
- Its [Pi report](../../docs/research/non-interactive-sessions/pi.md) recognizes RPC
  but recommends JSON because it is simpler for the existing one-shot wrapper.
  It also recommends disabling extensions and resources, contrary to the user's
  requirement for this feature.
- [Pi facts](../../docs/providers/facts/pi.yaml) explicitly omit RPC because the
  output-format model cannot represent it and copy the disabling companion flags.
  The generator registry consumes output formats from facts; revising research
  prose alone will not change generated behavior.

The current research run updates Pi's execution report and this existing fleet
prompt rather than creating a competing source of execution recommendations.
Typed execution-interface descriptors and full-roster execution-topic refresh
remain follow-on work. Requirements for that work:

1. Enumerate execution interfaces before selecting one: output-only CLI,
   bidirectional stdio, local server, and relevant SDK interfaces. Keep transport,
   wire encoding, event stream, and control operations as separate facts.
2. Record each interface's launch and connection ownership, session targeting,
   prompt/steer/cancel/state operations, acceptance versus completion, queue and
   process lifetime, unattended requests, and feature parity. Reuse steering
   mechanism identifiers and evidence instead of duplicating delivery semantics.
3. Compare candidates for each supported execution profile. Prefer usable RPC;
   require an evidenced reason for choosing a one-way alternative. Research must
   distinguish provider limitations, missing Claudine implementation, and missing
   verification. Do not select an interface solely because today's wrapper handles it.
4. Separate provider facts from Claudine selection policy. Generate typed execution
   interface descriptors and selection records through the existing catalog
   pipeline; keep output formats as an independent property. Do not disguise an
   RPC adapter as an output-format flag or execute research snippets at runtime.
5. Validate selection references, compatibility coverage, feature preservation,
   and fallback reasons. Exercise lifecycle and unattended requests in disposable
   tests with extensions and skills enabled before activation. Unknown behavior
   remains unknown rather than being inferred from a protocol's name.
6. Reconcile research, facts, overrides, generated metadata, parsers, and wrapper
   lifecycle behavior before declaring a migration complete. Refresh the full
   eligible roster with `gpt-5.6-sol`, low thinking, after schema/prompt review.

**Accepted fallback policy:** If the preferred RPC interface cannot be used for
a launch, Claudine may automatically use a verified alternative. Emit a warning
on STDERR explaining why RPC is unavailable, which interface was selected, and
which capabilities are unavailable, including steering when applicable. Preserve
the user's execution settings and enabled provider features; if no verified
alternative satisfies those requirements, fail with an explanation.
A missing Claudine implementation remains visible as migration work rather than
being recorded as a provider limitation. This launch fallback does not authorize
interrupting an active session or replaying work after an ambiguous submission.
Manual steering retains its explicit interactive interruption choice; automatic
loop warnings retain their non-interrupting-only requirement and existing limit.

## Proposed Session Selection and Delivery Contract

Proposed command forms for implementation review:

```sh
claudine steer "Recheck the failing test before changing the implementation."
claudine steer --list
claudine steer --list --json
claudine steer --session <session-id> "Recheck the failing test."
```

`--list` sends nothing and includes unavailable sessions and reasons. Its JSON
output must include the exact unambiguous ID accepted by `--session`, provider,
working directory, state, steering availability, and setup requirements. A
non-interactive invocation without an explicit target fails with guidance to
list and choose an ID. Listing does not require a message. Use the normal
Claudine separation between machine-readable stdout and status/warnings on STDERR.
These flag names are proposed engineering defaults, not an implemented CLI.

1. Accept a nonempty message and discover current-user sessions locally.
2. Correlate provider records with live process/session identity. Deduplicate
   native and Claudine observations of the same session. A PID alone is not a
   sufficient identity because the OS can reuse it.
3. Display provider, session name or short identity, working directory, running
   or idle state when known, and enough distinguishing information for identical
   projects or parallel agents. Exact column layout remains open.
4. Determine availability for the actual session: provider version, OS, launch
   mode, required startup options, reachable channel, and implemented adapter.
   A provider-wide support boolean is insufficient.
   Distinguish non-interrupting steering, interruption-required steering, and
   unavailable steering. A session with a supported interruption fallback remains
   selectable, labeled as requiring interruption; it is not an unavailable row.
5. Disable unavailable rows with dim styling and single strikethrough. Provide a
   plain-text reason so color or strikethrough support is not required to understand
   the list. Use `TerminalRenderable` components for terminal output.
   Where setup could enable steering, include an actionable separate setup path.
   Distinguish changes that help an existing session from those requiring a future
   launch; never imply that configuring the provider retrofits an open session.
6. Unless an explicit session ID was supplied, let the user select one eligible
   session, including when only one is eligible. An explicit ID bypasses only the
   picker, not eligibility checks or required interruption consent.
   Cancellation sends nothing. No sessions and no selectable sessions have clear
   empty states; neither falls back to broadcasting.
7. If the selected session requires interruption, explain what would be stopped
   and any known effect on running tools. Interactively offer interruption and
   message delivery or cancellation. Without explicit acceptance, send nothing
   and interrupt nothing. An unavailable interactive prompt must not imply consent.
8. Revalidate target identity and availability immediately before delivery. If it
   ended or changed, report that outcome without redirecting the message elsewhere.
   If non-interrupting delivery becomes unavailable, never silently switch to
   interruption; the same explanation and interactive choice are required.
9. Report only what the protocol establishes: accepted/queued is different from
   injected into the conversation, and neither proves that the model acted on it.
   Return after confirmed acceptance without waiting for later delivery or a reply.
   Report uncertain delivery honestly; do not blindly retry an ambiguous send and
   risk duplicate messages.

Discovery is best effort. An undiscoverable native session cannot be listed;
document coverage gaps rather than promising exhaustive discovery without evidence.
Active sessions include both working and idle-but-open sessions. Research must
establish how each provider distinguishes those states from ended conversations;
when state cannot be established reliably, display it as unknown rather than
guessing that the session is idle or working.

## Proposed Automatic Warning Contract

Automatic intervention applies where Claudine already observes a live semantic
stream for an agent execution. Discovering a native session for manual steering
does not itself attach an output monitor or enable automatic intervention for
that session. The existing capture-only path cannot promise early repetition
warnings without new live observations.

Proposed configuration names are `steering.automatic.enabled` in user/repo
configuration and `CLAUDINE_AUTO_STEER` for the runtime override. Apply the agreed
precedence to explicitly supplied values; an absent repo value must not erase a
user opt-out. Names and malformed-value diagnostics must be reconciled with
existing configuration conventions before implementation. This setting controls
automatic help only, including whether an unavailable-delivery warning is emitted.

- Keep content detection pure. Emit a nonterminal warning observation that a
  separate runtime delivery component handles using the same service as `steer`.
- Route directly to the session producing the suspicious stream; automatic help
  never opens a picker and never guesses among other sessions.
- Do not block stream consumption, timeout checks, cancellation, or termination
  while contacting a provider. Delivery attempts and queues must be bounded.
- A warning describes observed repetition as a suspicion, asks the agent to check
  whether it is making progress, and suggests changing approach or explaining why
  it cannot proceed. It must not instruct the agent to claim success or bypass
  its instructions.
- Prefer counts and a concise description over echoing arbitrary output into the
  message. Any excerpts must be bounded and clearly identified as observed data.
- Proposed message: “Claudine has detected repeated output that may indicate a
  loop. Please check whether you are making progress toward the user's task.
  If you are repeating the same approach, change your approach or stop and explain
  what is preventing progress. Claudine's existing runaway limits still apply.”
- Unsupported delivery, connection failure, or ambiguous acknowledgment does not
  disable any existing guard. A hard-stop observation takes priority over pending
  warnings. Do not enqueue a warning after deciding to terminate the session.
- If non-interrupting delivery is unsupported for the actual session, emit a
  warning on STDERR explaining that Claudine cannot send automatic steering and
  will continue enforcing the existing error limit. Do not offer an interactive
  prompt, interrupt a turn, or cancel a tool to deliver an automatic warning.
  Deduplicate this warning so repetitive output does not flood STDERR.
- A single output chunk can cross both warning and stop thresholds. Processing
  must preserve the existing hard stop even if no useful delivery window exists.
- Sending, acknowledging, or replying to a warning is not proof of recovery.
  Existing detector behavior remains the basis for recognizing changed output;
  steering itself must not clear accumulated guard evidence.
- Providers that inject only at the next tool call may be unable to rescue a
  token-generation loop. Capability metadata and diagnostics must expose that
  limitation. Recovery is best effort, not a guarantee.
- Derive automatic rescue suitability from the selected operation's conversation
  effect and delivery boundary, in addition to access and live-verification gates.
  A next-turn-only follow-up cannot rescue an endless current turn; report that
  limitation on STDERR and preserve the existing error limit. It can still be a
  manual messaging candidate. An idle-start or interruption operation is never
  an automatic rescue operation for a working session.
- A next-tool or end-of-batch boundary is a conditional rescue opportunity for
  repeated tool rounds, not a promise of delivery during uninterrupted generation
  or a blocked tool. Unknown boundaries remain unknown. Do not create a second
  independently authored rescue-support flag that can drift from these facts.
- Early volume warnings, silence warnings, retry-churn warnings, and changes to
  explicit exit-expression behavior are not yet agreed scope. Do not silently
  turn all termination causes into delayed termination.

## Proposed Fleet Research Contract

Research artifacts now exist under `claudine/docs/research/steering/`:
[`_fleet.md`](../../docs/research/steering/_fleet.md) and
[`_schema.yaml`](../../docs/research/steering/_schema.yaml). Produce one `<slug>.md`
per eligible provider after pilot review. Use a Darkmatter
SimplifiedSchema sidecar referenced by each document's `$schema` frontmatter.
The revision-3 sidecar extends the completed revision-2 fleet with receipt timing,
interface inventory, and case-specific discovery gaps. This research contract
is not a generated Rust API.

| Record | Required information | Consumer decision |
| --- | --- | --- |
| Document identity | Schema revision, roster slug, created/updated dates, research agent and model, examined provider versions | Coverage and freshness |
| Launch profile | Stable profile ID, applicable OS/mode/origin combinations, endpoint ownership, lifetime, startup requirements, feature preservation, baseline marker | Which execution environment a capability actually describes |
| Evidence | Stable evidence ID, official URL/source permalink or sanitized local artifact, version/commit, date, method, exact claim and limitations | Whether a claim justifies enabling behavior |
| Discovery method | Method ID, OS, launch origin, registry/API/process mechanism, identity and liveness checks, exposed labels, evidence references | How to find and identify sessions |
| Delivery mechanism | Mechanism ID, documented/undocumented status, protocol family, startup requirements, destination/authentication description, evidence references | Which hand-written adapter could implement it |
| Capability case | Profile ID, OS, interactive/non-interactive mode, native/Claudine launch origin, running/idle state, supported/unsupported/unknown verdict, mechanism and discovery references, reason | Whether this particular session can be selected |
| Receipt guarantees | Independent acceptance, persistence, scheduling, and conversation-delivery guarantees; provider signals and correlation; separate later delivery states | What the sender can honestly report and when |
| Receipt observations | Early, terminal, multi-phase, absent, or unknown acknowledgment; exact confirming signal and evidence | Whether acceptance can be reported before execution finishes |
| Interface inventory | Considered interfaces, included profile references, excluded or unknown alternatives, reasons and evidence | Whether selection considered usable control interfaces and explicit coverage gaps |
| Discovery gaps | Exact profile/OS/origin/mode/state case, missing discovery evidence, next check | Prevent unrelated gaps from justifying selectable sessions |
| Delivery semantics | While-running injection/next-tool-boundary/next-turn/interruption/resume-only/unknown, long-tool behavior, accepted versus delivered acknowledgments, ordering, duplication and cancellation behavior, limits | Honest user feedback and suitability for loop rescue |
| Compatibility | Tested version versus documented version bounds, feature probes, required flags/configuration, known incompatible variants | Runtime eligibility without broad version assumptions |
| Live verification | Mechanism, OS, exact version, launch conditions, session state, test date/outcome, sanitized fixture, assertions, limitations, evidence references | Mandatory activation gate distinct from researched support |
| Implementation gaps | `requires_claudine_update`, reason, unresolved questions, changes on refresh | Follow-up work and review |

Use explicit `unknown` where evidence is missing. `unsupported` requires evidence
of a limitation; failure to find documentation is not proof of absence. Separate
native Windows from Linux/WSL observations. Enumerate OS and launch-mode coverage
so omissions cannot look like support.

Multiple mechanisms may exist for one provider. Keep independently addressable
mechanism and evidence records rather than collapsing them into one provider-wide
verdict. Research describes facts; generated code selects only implemented,
reviewed mechanisms that satisfy the eventual product policy.

### Research Prompt Requirements

For each roster provider, the prompt must:

1. Read the topic schema and use the Claudine skill. Research this provider's
   current CLI, not a similarly named IDE or hosted product.
2. Start with official documentation and versioned source. Inspect local help or
   installed artifacts when useful, distinguishing observations from inference.
   Use Sniff for host/process discovery and `FileReference` for file-reference
   resolution where code is needed.
3. Investigate native peer messaging, SDK/server protocols, structured input,
   hooks/extensions, and required launch modes. Do not assume stdin is usable or
   that a documented SDK feature is enabled in ordinary CLI sessions.
4. Establish whether a new message reaches the same running conversation, at what
   boundary, and whether any in-flight tool or turn is canceled. Treat resume-only
   behavior and terminal keystroke injection as distinct findings, not proof of
   live steering.
5. Investigate discovery independently from delivery, including native sessions,
   process liveness, authentication, version differences, and per-OS gaps.
6. For Claude Code, independently check the user's socket/registry claims. For
   every provider, require evidence for non-interactive support and long-tool
   behavior rather than extrapolating from an interactive demonstration.
7. Produce implementation-level prose: discovery, protocol framing, request/response
   examples without secrets, timing, failure modes, compatibility, sources, and gaps.
8. Populate metadata with claim-linked evidence. Preserve creation dates and record
   changes on refresh; old research is context, not current proof.
9. Limit writes to assigned research artifacts. Do not send test messages into
   the user's existing sessions. Any active delivery experiment uses a deliberately
   created disposable session; provider/model costs and launch details must be
   defined before that experiment. Do not focus terminal or browser windows.
10. Validate the artifact with `md schema validate`; report unresolved findings
    honestly rather than inventing support to satisfy the schema.

### Execution and Quality Gates

1. Review the schema and prompt with the user, especially the distinction between
   live delivery and interruption/resumption alternatives.
2. Pilot Claude Code with `gpt-5.6-sol`, low thinking, to challenge the socket lead
   and whether the schema can express undocumented, tool-boundary delivery.
3. Revise the schema and prompt from pilot findings, then research the full roster
   using Claudine sequence orchestration and the same requested model/thinking.
   Verify model selection in run metadata before accepting results.
4. Gate artifacts on current schema validity, correct provider identity, complete
   capability cases, resolvable evidence references, and substantive prose.
   Timestamp and successful process exit alone are insufficient.
5. Use bounded lifecycle recovery. Surface contradictory evidence or missing
   mechanisms as review items; do not retry indefinitely.
6. Review findings with the user and update this spec's decisions and a per-provider
   capability matrix. Claude Code, OpenCode, Codex, and Pi passive pilots are complete;
   the full revision-2 roster refresh is now complete; see the run report below.
7. Plan and run disposable-session tests for candidate mechanisms. Preserve
   sanitized evidence of target identity, actual conversation delivery, delivery
   timing, and effects on the running turn or tool. Acceptance alone does not
   prove delivery. Failed or missing tests block activation of the corresponding
   capability cases, without preventing publication of research findings.
8. Add a topic summary under `docs/research/summary/` and publish it through the
   established Claudine skill publication workflow once findings are accepted.

## Generated Metadata and Runtime Boundary

### Steering Logs and Shared Secret Redaction

Persist manual and automatic steering messages with potential secrets replaced by
asterisks, together with target identity, origin, timestamps, delivery mechanism,
interruption choice where applicable, and acknowledgment or failure outcome.
Redact before persistence, including diagnostic errors or provider responses that
echo message content. The message delivered to the intended agent retains the
user's original text; masking applies to the logging copy.

Repository inspection found reusable starting points:

- [`protect::scrub`](../../lib/src/protect/scrub.rs) has a compiled pattern catalog
  for common API keys, GitHub/Slack tokens, bearer tokens, and JWTs, plus structured
  sensitive-key handling. Its current policy also masks email addresses, rewrites
  home paths, and uses `<redacted>`; it is not a drop-in implementation of the
  requested secret-only asterisk masking.
- [Wrapper argument sanitization](../../cli/src/commands/wrap/env/sanitize.rs)
  recognizes sensitive flags, environment key names, and token prefixes; argument
  masking already uses `****` but is not a free-text redactor.
- [`messaging::send`](../../lib/src/messaging/send.rs) includes webhook URL
  redaction, another relevant secret surface.

Implementation must assess these consumers and factor shared secret recognition
into an appropriate reusable library boundary. Keep consumer-specific formatting
and privacy policies separate; do not copy pattern catalogs or silently change
existing consumers' behavior. Exact API, location, and mask length remain design
details to settle during implementation impact analysis.

Test realistic prose containing tokens, credential assignments, authentication
headers, and credential-bearing URLs, as well as ordinary text that must remain
readable. Include overlapping matches, Unicode, multiline content, repeated
redaction, and provider errors echoing secrets. Heuristic redaction is best
effort and must not be described as a guarantee that every secret is recognized.

### Metadata Consumption

Extend the existing catalog types, research loading, generator, and drift checks.
The concrete integration points are shared vocabulary in
[`catalog-types`](../../catalog-types/src/lib.rs), schema-validated research in
[`ProviderInputs`](../../gen/src/inputs.rs), declared research ownership in the
[`mapping registry`](../../gen/src/registry.rs), and existing catalog coercion
and [Rust emission](../../gen/src/emit/mod.rs). Steering needs a deterministic
relational/evidence gate in addition to shape validation, including case coverage,
reference integrity, and live-verification applicability. The revision-3 fleet
invokes `claudine providers steering check` through the generator boundary for
deterministic checks. Source review and matching live tests remain separate gates.
Generated records should express discovery and delivery capabilities, compatibility,
and known limitations. Protocol implementation stays in reviewed provider behavior
code behind a common steering interface.

Do not execute research-provided shell snippets as runtime discovery or delivery
commands. Map validated mechanism identifiers to implemented behavior. Unknown
mechanisms, missing evidence required by policy, and unsupported runtime versions
must not make a session selectable. Compatibility probes must be read-only.
Successful disposable-session test evidence is required in addition to these
checks. Keep researched capability distinct from activation eligibility so a
source-documented mechanism can be represented while its implementation or live
verification is still pending.

Use the same OS user boundary for discovery and send. Provider credentials or peer
keys are authentication material, never list columns or diagnostic payloads. Keep
steering local even if Rendezvous knows about paired remote hosts. Revalidate
ownership and destination identity before using provider-advertised endpoints.

Whether managed-session delivery requires a new Rendezvous operation or an existing
wrapper-owned channel remains an implementation design question. Resolve it after
research establishes connection ownership and launch requirements.

## Acceptance Criteria to Refine After Research

- A manual send reaches only the selected, revalidated session; canceled selection
  and unavailable rows send nothing.
- Interruption-required sessions are selectable with their limitation explained.
  Manual interruption occurs only after the user explicitly accepts the fallback;
  cancellation or inability to prompt leaves the running session untouched.
- For automatic loop intervention, a session lacking non-interrupting delivery
  produces a bounded STDERR warning, receives no steering-related interruption,
  and still terminates if it reaches the existing error limit.
- Unsupported sessions remain visible with the required styling and a textual
  reason. Plain-output users can understand their availability.
- Tests cover duplicate observations, reused PIDs, ended sessions, unavailable
  endpoints, ambiguous acknowledgments, and concurrent senders.
- An early warning can be observed before a repetition hard stop. A simulated
  agent that changes its output avoids a repetition trip under existing detector
  rules; continued repetition still terminates at the configured threshold.
- Failed/slow steering cannot stall stream processing or termination. Warning
  traffic does not refresh watchdog clocks or reset detector counters.
- Tests cover split chunks, warning and termination in one chunk, unavailable
  next-tool delivery, bounded warning frequency, and stopping during delivery.
- Existing exit-expression, volume, timeout, and cancellation behavior remains
  covered. Research-driven exceptions require explicit changes to this spec.
- Generated metadata has roster coverage and deterministic drift checks; evidence
  and schema validation failures are surfaced before generation enables a mechanism.
- Run appropriate `just test`, `just test-l2`, and `just lint` checks for the eventual
  implementation. Native Windows, macOS, and Linux transport behavior needs runtime
  evidence; compilation alone does not prove provider steering support. L2/L3 tests
  must not take window focus.
- Update public CLI docs, relevant topic docs, and the Claudine skill with the final
  behavior. Update dependency docs only if dependencies change.

## Full-Fleet Research Outcome

The authorized full steering fleet completed with `gpt-5.6-sol` and low reasoning.
All ten provider reports completed schema revision 2 and passed shape, identity, coverage,
and relationship checks: **24 launch profiles, 360 cases, 31 mechanisms, and
91 evidence records**. All 240 ordinary baseline combinations are represented;
special profiles contribute 120 additional cases. The research sessions' own
execution metadata confirmed model and effort; mismatched initial launches were
stopped and excluded. See the [run report](fleet-run.md) for provenance, review
corrections, per-provider findings, and remaining gates.

The findings reinforce profile-specific selection. Managed control interfaces
are often available where ordinary CLI sessions expose no external delivery
channel. Their behavior differs: current-turn steering, tool-boundary pickup,
next-turn follow-up, idle turn start, and cancellation/replacement must remain
separate. In particular, Qwen's queued follow-up cannot rescue a current turn
that never ends. Missing implementation, missing access, and missing live
verification are distinct from a provider lacking a capability.

Pi's execution report and the existing non-interactive fleet prompt were updated
in that run to prefer usable RPC/control interfaces while preserving provider
features. The subsequent contract backfill extends typed interface coverage
across the execution topic. Delivery adapters and wrapper interface selection
remain implementation targets.

## Post-Fleet Review: Recommended Research Refinements

Status rechecked on 2026-09-08: all five accepted batches report two successful
steps and no failures. All ten steering reports pass the schema and relationship
checks again; Pi's execution report passes its topic schema. All eleven recorded
research execution contexts confirm `gpt-5.6-sol` with low reasoning. These checks
establish artifact completeness and execution provenance, not live delivery.

The full run supports targeted contract improvements before metadata consumption.
The user authorized these next steps. Revision 3 now backfills the completed
reports from existing evidence; the original revision-2 fleet remains recorded
in the run history. Another full steering fleet run is not needed for that
backfill. The refinements below guide the contract and validation work.

1. **Make usefulness for loop rescue explicit.** Qwen can accept a follow-up
   without interrupting, but waits until the current turn ends. That capability
   can serve manual messaging while being ineffective against a turn that never
   ends. Derive rescue eligibility from operation intent and delivery boundary,
   distinguishing generation loops, repeated tool rounds, and blocked tools.
   Require evidence for the boundary and retain unknown outcomes. Do not treat
   `non_interrupting` alone as proof that automatic steering can help.
2. **Record when acknowledgment becomes available.** Goose's idle prompt returns
   at turn completion; OpenCode's asynchronous response can precede persistence
   and scheduling. Add receipt timing and the exact confirming signal alongside
   the existing independent guarantees. Explicitly represent no separate early
   acknowledgment. A late successful response must not imply that Claudine can
   return immediately after submission, and a timeout must not imply rejection.
3. **Represent execution interfaces independently of output formats.** The
   non-interactive topic still primarily types invocation and output, despite
   its improved prompt. Add interface candidates, launch conditions, feature
   preservation, unattended request handling, and selection/fallback evidence
   to that topic's contract. Reference steering mechanisms instead of duplicating
   their protocol facts. Prefer usable bidirectional control; compare actual
   capabilities when several candidates exist. Kimi's web steering and ACP
   interruption demonstrate why the protocol name alone cannot select a winner.
4. **Separate input handling and execution settlement.** Pi extensions can handle
   accepted input without starting a model turn, and `agent_end` differs from
   full `agent_settled` completion. Require an operation-specific account of input
   transformation/handling, turn scheduling, and settlement. Verify lifecycle
   claims against both the event definition and its forwarding to the selected
   interface at the examined version. Keep incoming approval holds separate from
   tool permission requests after delivery; neither permits fabricated approval
   in an unattended session.
5. **Make semantic checks durable.** The original run used a temporary coordinator
   script. The revision-3 fleet calls the maintained generator validation command.
   Validation includes case-state versus
   operation compatibility and discovery gaps tied to specific cases. Require an
   evidenced interface inventory and explicit exclusions: complete coverage of
   self-declared profiles cannot detect an omitted useful launch mode. Keep source
   review and live tests separate from deterministic validation.

This follow-up updates the contracts and checks, then backfills affected claims
from existing evidence. Use focused provider research
only where evidence is missing or contradictory, with `gpt-5.6-sol` and low
reasoning. The execution-topic backfill must retain unknowns where interface
selection or feature parity lacks evidence. Preserve the verified-model
launch procedure and prohibition on recursive fleet launches in subsequent runs.

### Completed Follow-Up

All ten steering reports now use revision 3, and all ten execution reports have
typed interface/selection/request/settlement/reference backfills. The permanent
`claudine providers steering check [slug] [--json]` command validates both
steering relationships and populated execution-topic references. Generation runs
this gate before applying generated catalog/data artifacts when the topic exists;
legacy areas without the topic and the existing hand-owned scaffolding workflow
remain compatible.

The regenerated catalog contains the new execution-research metadata, with no
generated provider behavior changes from this follow-up. The catalog byte baseline
was refreshed with Biscuit-hash. Generator tests pass (161 passed, one skipped),
generator lint passes, and the generator/provider CLI compile. Source claims and
live delivery remain separate review gates; see [the run report](fleet-run.md).

## Current Work Boundary

This feature remains in specification and implementation planning. The four
pilots informed revision 2, and the full passive roster refresh is complete.
The authorized follow-up adds revision-3 metadata and permanent research-validation
tooling. The first disposable Pi 0.84.4 macOS RPC experiment now passes with a
deterministic local model and isolated resource fixtures. Its two scoped records
cover active steering and idle prompting; the other providers remain untested.
No production steering capability is activated by these research results.
Unresolved protocol, compatibility, and feature-parity details
remain explicit in each report.

See the [uncertainty register](uncertainties.md) for resolved contract gaps versus
remaining provider questions, and [verification notes](verification/README.md)
for the reusable opt-in harness, measured outcomes, and next experiments.

The abort/EOF follow-up adds two scoped regression records. In Pi 0.84.4, abort
can move queued steering into saved history without model consumption; an explicit
next prompt then consumes that history. Clearing before abort prevents this in
the fixture, but clearing must remain an explicit queue policy, not an automatic
way to discard other pending work. Report cancellation, queue/history disposition,
and replacement acceptance separately.

Closing owned RPC stdin while tools are held can lose acknowledged steering with
exit code zero. Keep the input channel open through settlement, and do not infer
delivery or durable storage from acknowledgment or successful process exit.
Disconnect outcomes must remain undelivered or unknown according to available
evidence; ambiguous submissions must not be replayed automatically. These findings
cover input EOF and cooperative tools only, not full transport loss, cloud errors,
or OS subprocess cleanup.

The controlled switch/crash follow-up establishes two additional Pi 0.84.4/macOS
failure boundaries. During a switch paused by an extension, `get_state` still
reports the old session and `steer` can succeed, yet completing the switch loses
that queued text. Steering submitted after the switch reaches the new session.
Killing the provider after acknowledgment while tools are held also loses queued
text before either transcript persistence or model consumption.

The managed controller must serialize session-changing operations with target
validation and submission, then invalidate cached identity after a switch.
A pre-send state query alone is insufficient. Where extensions or another actor
can change the session outside that coordination, require a verified provider
guard or report that safe targeting is unavailable for that profile. Detecting
a changed identity after submission does not permit automatic replay into either
session. Maintain separate queue, conversation-history, and delivery outcomes
after process failure; an acknowledgment is not a crash-persistence guarantee.

Managed execution must also retain a way to terminate its owned tool processes
when the provider fails. A provider's abort acknowledgment, idle state, or exited
main process does not establish that external tools have stopped. Integrate this
with Claudine's existing termination ownership and platform-specific cleanup;
verify macOS, Linux, and Windows independently. Report cancellation and remaining
process cleanup separately, and never signal unrelated sessions. Do not enable
an interrupting adapter based solely on cooperative in-process tool tests.

Next work is to settle the remaining engineering choices, implement the typed
metadata consumer and reviewed adapters, and perform the required disposable
session tests before activation. Before implementation edits, run the
repository-required GitNexus impact analysis on affected symbols and report the
blast radius. Schema and relationship validation establish a usable research
artifact, not factual certainty or successful runtime delivery.
