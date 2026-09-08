---
"$schema": "./_schema.yaml"
schema_revision: 3
provider: kilo
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
- Kilo Code CLI 7.3.45 (installed binary and commit 67b815466c9ab3e022f16692988673712437b881)
- Kilo Code main documentation and V2 session specification observed 2026-09-08 (comparison
  only; not attributed to 7.3.45)
launch_profiles:
- id: ordinary-cli
  description: Ordinary Kilo TUI or one-shot run, whose runtime may opportunistically
    attach to a daemon or use a private embedded/worker fallback but exposes no stable
    registered endpoint contract to an independent sender.
  endpoint_scope: unknown
  lifetime: unknown
  applicable_os:
  - macos
  - linux
  - windows
  applicable_launch_modes:
  - interactive
  - non_interactive
  applicable_origins:
  - native
  - claudine
  baseline: true
  startup_requirements:
  - ordinary `kilo` or `kilo run` launch
  preserves_extensions: 'yes'
  preserves_skills: 'yes'
  preserves_templates: 'yes'
  preserves_context: 'yes'
  evidence_ids:
  - official-runtime
  - official-cli
  - local-cli-7-3-45
  - claudine-wrapper
- id: managed-http-server
  description: Future Claudine-managed, authenticated `kilo serve` process with endpoint,
    directory context, and provider session IDs retained in Claudine registration.
  endpoint_scope: externally_reachable
  lifetime: long_lived
  applicable_os:
  - macos
  - linux
  - windows
  applicable_launch_modes:
  - non_interactive
  applicable_origins:
  - claudine
  baseline: false
  startup_requirements:
  - launch and retain `kilo serve` on loopback with Basic authentication
  - register endpoint, credentials reference, directory context, process identity,
    and Kilo session ID
  - keep ordinary configuration and do not use `--pure`
  preserves_extensions: 'yes'
  preserves_skills: 'yes'
  preserves_templates: 'yes'
  preserves_context: 'yes'
  evidence_ids:
  - official-runtime
  - official-testing
  - source-http-7-3-45
  - source-queue-7-3-45
access_findings:
- mechanism_id: http-prompt-async-active
  profile_id: managed-http-server
  os: macos
  status: setup_required
  prerequisite: A deliberately retained authenticated server, exact directory, live
    session ID, and just-in-time status check; this setup only helps sessions hosted
    by that server.
  applies_to_existing_sessions: 'no'
  evidence_ids: &1
  - official-runtime
  - official-testing
  - source-http-7-3-45
- mechanism_id: http-prompt-async-active
  profile_id: managed-http-server
  os: linux
  status: setup_required
  prerequisite: A deliberately retained authenticated server, exact directory, live
    session ID, and just-in-time status check; this setup only helps sessions hosted
    by that server.
  applies_to_existing_sessions: 'no'
  evidence_ids: *1
- mechanism_id: http-prompt-async-active
  profile_id: managed-http-server
  os: windows
  status: unknown
  prerequisite: The same managed-server setup plus native Windows endpoint and process-lifecycle
    verification.
  applies_to_existing_sessions: 'no'
  evidence_ids: *1
- mechanism_id: http-prompt-async-idle
  profile_id: managed-http-server
  os: macos
  status: setup_required
  prerequisite: A deliberately retained authenticated server, exact directory, known
    idle session ID, and just-in-time status check.
  applies_to_existing_sessions: 'no'
  evidence_ids: &2
  - official-runtime
  - official-testing
  - source-http-7-3-45
- mechanism_id: http-prompt-async-idle
  profile_id: managed-http-server
  os: linux
  status: setup_required
  prerequisite: A deliberately retained authenticated server, exact directory, known
    idle session ID, and just-in-time status check.
  applies_to_existing_sessions: 'no'
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: managed-http-server
  os: windows
  status: unknown
  prerequisite: The same managed-server setup plus native Windows verification.
  applies_to_existing_sessions: 'no'
  evidence_ids: *2
- mechanism_id: http-abort-then-prompt
  profile_id: managed-http-server
  os: macos
  status: setup_required
  prerequisite: Managed server, exact directory/session, observed busy state, explicit
    interactive user choice, successful abort settlement, then separately accepted
    prompt_async.
  applies_to_existing_sessions: 'no'
  evidence_ids: &3
  - source-http-7-3-45
  - source-queue-7-3-45
- mechanism_id: http-abort-then-prompt
  profile_id: managed-http-server
  os: linux
  status: setup_required
  prerequisite: Managed server, exact directory/session, observed busy state, explicit
    interactive user choice, successful abort settlement, then separately accepted
    prompt_async.
  applies_to_existing_sessions: 'no'
  evidence_ids: *3
- mechanism_id: http-abort-then-prompt
  profile_id: managed-http-server
  os: windows
  status: unknown
  prerequisite: The same two-operation sequence plus native Windows verification.
  applies_to_existing_sessions: 'no'
  evidence_ids: *3
delivery_states:
- mechanism_id: http-prompt-async-active
  states:
  - accepted
  - queued
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: The 204 response has no message ID. SSE and message history carry session/message
    data, but 7.3.45 source does not establish a sender-supplied idempotency key or
    direct correlation from the 204 to later delivery.
  evidence_ids:
  - source-http-7-3-45
  - source-queue-7-3-45
  - official-testing
- mechanism_id: http-prompt-async-idle
  states:
  - accepted
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: The sender can correlate by target session and content/time only; the
    no-content acknowledgment supplies no message ID.
  evidence_ids:
  - source-http-7-3-45
  - official-testing
- mechanism_id: http-abort-then-prompt
  states:
  - accepted
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: Abort returns a Boolean and prompt_async later returns 204, but neither
    response contains a shared operation/message correlation ID.
  evidence_ids:
  - source-http-7-3-45
receipt_guarantees:
- mechanism_id: http-prompt-async-active
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - HTTP 204 after session lookup and background-fiber creation
  - session.status/session.idle events
  - session message history
  correlation: none
  evidence_ids:
  - source-http-7-3-45
  - official-testing
  limitations: The handler forks prompt processing and returns immediately; later
    failure is emitted as session.error. Therefore 204 does not prove durable persistence,
    queue placement, promotion into history, or model consumption.
- mechanism_id: http-prompt-async-idle
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - HTTP 204
  - session.status/session.idle events
  - session message history
  correlation: none
  evidence_ids:
  - source-http-7-3-45
  - official-testing
  limitations: Initial acceptance is uncorrelated and asynchronous; the caller must
    observe later session events/history, and model incorporation remains unproven.
- mechanism_id: http-abort-then-prompt
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - Boolean abort response
  - later HTTP 204 from prompt_async
  - session events/history
  correlation: none
  evidence_ids:
  - source-http-7-3-45
  limitations: The Boolean applies only to the abort call. It says nothing about subsequent
    prompt acceptance or delivery; the two phases can fail independently.
evidence:
- id: official-runtime
  method: official_docs
  location: https://kilo.ai/docs/contributing/architecture/cli-runtime
  version: current documentation; exact release applicability unknown
  observed_on: 2026-09-08
  claim: Documents TUI/run daemon preference and embedded fallback, explicit serve/attach
    modes, directory-keyed instances, optional Basic Auth, daemon state/health/version
    checks, HTTP/SSE clients, and configuration ownership.
  limitations: Current documentation may postdate 7.3.45 and explicitly is not a complete
    endpoint catalog.
- id: official-cli
  method: official_docs
  location: https://kilo.ai/docs/code-with-ai/platforms/cli-reference
  version: current documentation; exact release applicability unknown
  observed_on: 2026-09-08
  claim: Documents ordinary TUI, run, serve, attach, session, ACP, daemon, plugin,
    and related launch surfaces.
  limitations: Does not promise external steering of an arbitrary already-running
    ordinary CLI.
- id: official-testing
  method: official_docs
  location: https://github.com/Kilo-Org/kilocode/blob/main/TESTING.md
  version: main observed 2026-09-08
  observed_on: 2026-09-08
  claim: Documents authenticated HTTP health, OpenAPI discovery, session list/create,
    prompt_async, message history, abort, and global SSE recipes plus directory routing.
  limitations: Main-branch contributor documentation, not a live test and not a compatibility
    promise for 7.3.45.
- id: source-http-7-3-45
  method: source_code
  location: https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts
  version: 7.3.45 / 67b815466c9ab3e022f16692988673712437b881
  observed_on: 2026-09-08
  claim: The HTTP handlers list status, require the target session, return 204 after
    forking prompt work, publish later failures, and expose abort as a separate cancel
    operation.
  limitations: Passive source inspection only; runtime framing and all OS behavior
    remain untested.
- id: source-queue-7-3-45
  method: source_code
  location: https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/session/prompt.ts#L1710-L1758
  version: 7.3.45 / 67b815466c9ab3e022f16692988673712437b881
  observed_on: 2026-09-08
  claim: Kilo creates the user message, does not cancel the in-flight fiber, serializes
    the new prompt, and checks the follow-up queue only after the current token stream
    and inline tool calls have drained.
  limitations: Source-derived, not disposable-session evidence; external receipt correlation
    and crash persistence are unresolved.
- id: source-status-7-3-45
  method: source_code
  location: https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/session/status.ts
  version: 7.3.45 / 67b815466c9ab3e022f16692988673712437b881
  observed_on: 2026-09-08
  claim: Status is process-memory state; absent entries read idle, busy states are
    listed, and idle removes the map entry while publishing status and idle events.
  limitations: An absent status is not proof that a persisted session is open in a
    particular client.
- id: local-cli-7-3-45
  method: local_inspection
  location: installed `@kilocode/cli` package and non-interactive `kilo --version/--help`
    output on macOS
  version: 7.3.45
  observed_on: 2026-09-08
  claim: The installed binary exposes serve, attach, run --attach, session list, ACP,
    daemon, loopback hostname/port, Basic-auth options, and native package variants
    for macOS, Linux, and Windows.
  limitations: Help inspection opened the normal runtime bootstrap and existing database
    path; no session was launched, no endpoint was contacted, and other OS binaries
    were not run.
- id: main-v2-spec
  method: official_docs
  location: https://github.com/Kilo-Org/kilocode/blob/main/specs/v2/session.md
  version: main observed 2026-09-08; not 7.3.45
  observed_on: 2026-09-08
  claim: The future/current-main V2 design separates durable admission, promotion,
    active-session snapshots, and interruption with message-ID idempotency.
  limitations: Not used to claim 7.3.45 behavior or support; it is a future compatibility
    signal only.
- id: claudine-wrapper
  method: local_inspection
  location: claudine/cli/src/commands/wrap/profile/kilo.rs
  version: workspace state observed 2026-09-08
  observed_on: 2026-09-08
  claim: Current Claudine uses ordinary Kilo TUI or one-shot `kilo run --format json`;
    it does not deliberately create and register a retained HTTP steering profile.
  limitations: Claudine-side observation establishes reachability limitations, not
    provider incapability.
discovery:
- id: ordinary-macos-native
  profile_id: ordinary-cli
  os: macos
  origin: native
  method: process_inspection
  locator: Sniff process discovery can identify `kilo` processes; provider session
    IDs may be listed from storage/API but no ordinary-launch endpoint/session ownership
    registry is documented.
  identity_check: PID/executable/start time can deduplicate processes, but cannot
    prove which Kilo session is selected; provider session ID is distinct from PID.
  liveness_check: Process existence/start time only; it does not establish a responsive
    conversation or distinguish helper/worker/server ownership.
  state_detection: Unknown without a reachable matching server; persisted history
    is not liveness and absent in-memory status means idle only within a specific
    server process.
  observation_source: Sniff process inventory plus provider records/API if separately
    reachable
  observed_at: delivery time
  available_labels: &4
  - pid
  - process start time
  - executable
  - provider session ID when separately known
  prerequisites: &5
  - read-only process access under the same OS user
  evidence_ids: &6
  - official-runtime
  - source-status-7-3-45
- id: ordinary-macos-claudine
  profile_id: ordinary-cli
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Current wrapper can know child PID but does not register a Kilo server
    endpoint or selected provider session ID.
  identity_check: PID/executable/start time can deduplicate processes, but cannot
    prove which Kilo session is selected; provider session ID is distinct from PID.
  liveness_check: Process existence/start time only; it does not establish a responsive
    conversation or distinguish helper/worker/server ownership.
  state_detection: Unknown without a reachable matching server; persisted history
    is not liveness and absent in-memory status means idle only within a specific
    server process.
  observation_source: Sniff process inventory plus provider records/API if separately
    reachable
  observed_at: delivery time
  available_labels: *4
  prerequisites: *5
  evidence_ids:
  - claudine-wrapper
  - official-runtime
- id: ordinary-linux-native
  profile_id: ordinary-cli
  os: linux
  origin: native
  method: process_inspection
  locator: Sniff process discovery can identify `kilo` processes; provider session
    IDs may be listed from storage/API but no ordinary-launch endpoint/session ownership
    registry is documented.
  identity_check: PID/executable/start time can deduplicate processes, but cannot
    prove which Kilo session is selected; provider session ID is distinct from PID.
  liveness_check: Process existence/start time only; it does not establish a responsive
    conversation or distinguish helper/worker/server ownership.
  state_detection: Unknown without a reachable matching server; persisted history
    is not liveness and absent in-memory status means idle only within a specific
    server process.
  observation_source: Sniff process inventory plus provider records/API if separately
    reachable
  observed_at: delivery time
  available_labels: *4
  prerequisites: *5
  evidence_ids: *6
- id: ordinary-linux-claudine
  profile_id: ordinary-cli
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Current wrapper can know child PID but does not register a Kilo server
    endpoint or selected provider session ID.
  identity_check: PID/executable/start time can deduplicate processes, but cannot
    prove which Kilo session is selected; provider session ID is distinct from PID.
  liveness_check: Process existence/start time only; it does not establish a responsive
    conversation or distinguish helper/worker/server ownership.
  state_detection: Unknown without a reachable matching server; persisted history
    is not liveness and absent in-memory status means idle only within a specific
    server process.
  observation_source: Sniff process inventory plus provider records/API if separately
    reachable
  observed_at: delivery time
  available_labels: *4
  prerequisites: *5
  evidence_ids:
  - claudine-wrapper
  - official-runtime
- id: ordinary-windows-native
  profile_id: ordinary-cli
  os: windows
  origin: native
  method: process_inspection
  locator: Sniff process discovery can identify `kilo` processes; provider session
    IDs may be listed from storage/API but no ordinary-launch endpoint/session ownership
    registry is documented.
  identity_check: PID/executable/start time can deduplicate processes, but cannot
    prove which Kilo session is selected; provider session ID is distinct from PID.
  liveness_check: Process existence/start time only; it does not establish a responsive
    conversation or distinguish helper/worker/server ownership.
  state_detection: Unknown without a reachable matching server; persisted history
    is not liveness and absent in-memory status means idle only within a specific
    server process.
  observation_source: Sniff process inventory plus provider records/API if separately
    reachable
  observed_at: delivery time
  available_labels: *4
  prerequisites:
  - native Windows read-only process inspection; not WSL evidence
  evidence_ids: *6
- id: ordinary-windows-claudine
  profile_id: ordinary-cli
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Current wrapper can know child PID but does not register a Kilo server
    endpoint or selected provider session ID.
  identity_check: PID/executable/start time can deduplicate processes, but cannot
    prove which Kilo session is selected; provider session ID is distinct from PID.
  liveness_check: Process existence/start time only; it does not establish a responsive
    conversation or distinguish helper/worker/server ownership.
  state_detection: Unknown without a reachable matching server; persisted history
    is not liveness and absent in-memory status means idle only within a specific
    server process.
  observation_source: Sniff process inventory plus provider records/API if separately
    reachable
  observed_at: delivery time
  available_labels: *4
  prerequisites:
  - native Windows Claudine launch; not WSL evidence
  evidence_ids:
  - claudine-wrapper
  - official-runtime
- id: managed-macos-claudine
  profile_id: managed-http-server
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Registered loopback base URL + directory context, then GET /global/health,
    GET /session, and GET /session/status.
  identity_check: Match registered process start identity and exact Kilo session ID/directory;
    deduplicate by endpoint + normalized directory + provider session ID, never PID
    alone.
  liveness_check: Authenticated health response plus process start identity and successful
    session lookup immediately before delivery.
  state_detection: A session.status entry establishes active server-owned work; absence
    is treated only as server-side idle and must be paired with session lookup and
    registration ownership.
  observation_source: Claudine managed-launch registry and read-only Kilo HTTP API
  observed_at: immediately before delivery
  available_labels: &7
  - endpoint
  - directory
  - provider session ID
  - session title
  - status
  - process start identity
  prerequisites: &8
  - managed authenticated server
  - retained credential reference
  - registered exact session
  evidence_ids: &9
  - official-runtime
  - official-testing
  - source-http-7-3-45
  - source-status-7-3-45
- id: managed-linux-claudine
  profile_id: managed-http-server
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Registered loopback base URL + directory context, then GET /global/health,
    GET /session, and GET /session/status.
  identity_check: Match registered process start identity and exact Kilo session ID/directory;
    deduplicate by endpoint + normalized directory + provider session ID, never PID
    alone.
  liveness_check: Authenticated health response plus process start identity and successful
    session lookup immediately before delivery.
  state_detection: A session.status entry establishes active server-owned work; absence
    is treated only as server-side idle and must be paired with session lookup and
    registration ownership.
  observation_source: Claudine managed-launch registry and read-only Kilo HTTP API
  observed_at: immediately before delivery
  available_labels: *7
  prerequisites: *8
  evidence_ids: *9
- id: managed-windows-claudine
  profile_id: managed-http-server
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Registered loopback base URL + directory context, then GET /global/health,
    GET /session, and GET /session/status.
  identity_check: Match registered process start identity and exact Kilo session ID/directory;
    deduplicate by endpoint + normalized directory + provider session ID, never PID
    alone.
  liveness_check: Authenticated health response plus process start identity and successful
    session lookup immediately before delivery.
  state_detection: A session.status entry establishes active server-owned work; absence
    is treated only as server-side idle and must be paired with session lookup and
    registration ownership.
  observation_source: Claudine managed-launch registry and read-only Kilo HTTP API
  observed_at: immediately before delivery
  available_labels: *7
  prerequisites:
  - managed authenticated server
  - retained credential reference
  - registered exact session
  - native Windows verification; WSL is insufficient
  evidence_ids: *9
mechanisms:
- id: http-prompt-async-active
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Start authenticated `kilo serve`, retain endpoint/directory/session
    registration, and optionally retain global SSE.
  operation_intent: queue_follow_up
  conversation_effect: preserve_running_turn
  delivery_boundary: end_of_tool_batch
  destination: Exact Kilo session ID within the directory-routed server instance.
  authentication: HTTP Basic Auth when KILO_SERVER_PASSWORD is set; managed profile
    requires it even though provider permits no-auth loopback.
  startup_requirements:
  - managed-http-server profile
  - provider configuration loaded normally
  - registered exact directory/session
  target_preconditions:
  - session lookup succeeds
  - session status is busy in the same server instance
  target_guards:
  - no expected active-operation ID exists in 7.3.45
  - recheck endpoint process identity, directory, session ID, and busy status immediately
    before POST
  - refuse on absent/stale registration
  request_framing: POST /session/{sessionID}/prompt_async with x-kilo-directory (or
    SDK directory routing), Basic Authorization, application/json, and a parts array
    containing a text part.
  response_framing: HTTP 204 No Content after the work is forked; later failures are
    session.error events.
  request_format: 'JSON PromptPayload; plain manual text is one `{type: text, text:
    ...}` part.'
  response_format: No response body; later SSE events/history are separate and uncorrelated
    with the initial receipt.
  long_tool_behavior: The running fiber is not canceled. Source checks follow-up only
    after handle.process drains the current token stream and inline tool calls; a
    hung long tool prevents delivery.
  tool_batch_behavior: continue_all
  queue_behavior: Per-session serialization; newer prompt is hidden from the current
    model context, then takes over after the current LLM step and inline tools drain.
    Multiple arrival ordering is source-derived FIFO, but disconnect/crash persistence
    is unknown.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable; this mechanism does not interrupt.
  ordering: Source uses a per-session promise tail and monotonic arrival sequence;
    cross-client runtime ordering is not live-tested.
  sender_message_id: unsupported
  retry_policy: never_retry
  duplicate_handling: No sender ID or duplicate suppression is exposed by the 7.3.45
    PromptPayload path; ambiguous responses must not be retried.
  cancellation: No per-message queue cancellation API was established; session abort
    cancels session work and clears queued follow-up loops.
  limits: No text/message-size limit was established. Input can be transformed by
    normal Kilo prompt/config/plugin behavior; literal preservation is unverified.
  evidence_ids:
  - source-http-7-3-45
  - source-queue-7-3-45
  - official-testing
- id: http-prompt-async-idle
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Same managed authenticated server and registration as the active
    mechanism.
  operation_intent: start_idle_turn
  conversation_effect: resume_same_conversation
  delivery_boundary: idle_turn_start
  destination: Exact idle Kilo session ID within the directory-routed server instance.
  authentication: Managed Basic Auth and loopback endpoint.
  startup_requirements:
  - managed-http-server profile
  - registered exact directory/session
  target_preconditions:
  - session exists
  - status lookup has no busy entry immediately before POST
  target_guards:
  - no operation guard
  - refuse if just-in-time status becomes busy unless caller explicitly selected working-session
    steering
  request_framing: Same POST /session/{sessionID}/prompt_async JSON request.
  response_framing: HTTP 204 No Content; later events/history must be observed separately.
  request_format: JSON PromptPayload with a text part.
  response_format: No response body.
  long_tool_behavior: Not applicable after an idle precondition; a race to busy converts
    behavior to queued follow-up and must be treated as ambiguous.
  tool_batch_behavior: not_applicable
  queue_behavior: Starts session processing asynchronously if still idle.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable.
  ordering: One request starts a turn; concurrent request order is not verified.
  sender_message_id: unsupported
  retry_policy: never_retry
  duplicate_handling: No 7.3.45 idempotency key; do not retry an ambiguous 204/connection
    loss.
  cancellation: Abort is a separate session-scoped operation.
  limits: Unknown message-size and transformation limits.
  evidence_ids:
  - source-http-7-3-45
  - official-testing
- id: http-abort-then-prompt
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Same managed server; only offered after explaining effects and obtaining
    explicit interactive user choice.
  operation_intent: interrupt_then_submit
  conversation_effect: cancel_turn_same_conversation
  delivery_boundary: idle_turn_start
  destination: Exact session ID; abort and replacement POST are two separate requests.
  authentication: Managed Basic Auth and exact directory routing for both requests.
  startup_requirements:
  - managed-http-server profile
  - explicit interactive approval
  target_preconditions:
  - session exists
  - status is busy
  - non-interrupting queueing is unavailable or user explicitly chose interruption
  target_guards:
  - no expected operation ID
  - revalidate registration and busy state immediately before abort
  - observe settlement and same session before submission
  request_framing: POST /session/{sessionID}/abort, wait for settled/idle evidence,
    then POST prompt_async with text parts.
  response_framing: Boolean abort response, followed independently by 204 No Content
    for prompt_async.
  request_format: Bodyless abort followed by JSON PromptPayload.
  response_format: Two uncorrelated responses plus later SSE/history.
  long_tool_behavior: Abort cancels the active session fiber and propagates cancellation
    to in-flight work where implemented; side effects already performed cannot be
    undone.
  tool_batch_behavior: stop_remaining
  queue_behavior: 7.3.45 cancel clears queued follow-up loops before canceling running
    state. Replacement is not retained automatically if its submission fails.
  message_interpretation: provider_defined
  interruption_phases:
  - revalidate exact target and busy state
  - obtain explicit user choice
  - POST abort
  - observe canceled turn settle/idle
  - revalidate same conversation
  - POST replacement prompt
  - observe later delivery
  interruption_partial_failure: If abort succeeds and replacement submission fails,
    the original work remains canceled, prior queued follow-ups have been cleared,
    and no replacement is guaranteed. Report this terminal partial failure; never
    silently retry.
  ordering: Abort settlement must precede replacement submission.
  sender_message_id: unsupported
  retry_policy: never_retry
  duplicate_handling: Neither phase provides idempotent sender identity in 7.3.45.
  cancellation: Abort is session-scoped and clears queued follow-up loops; it is not
    an atomic abort-and-submit operation.
  limits: No payload-size bounds established; interruption cannot roll back completed
    tool side effects.
  evidence_ids:
  - source-http-7-3-45
  - source-queue-7-3-45
compatibility:
- mechanism_id: http-prompt-async-active
  profile_id: managed-http-server
  os: macos
  versions_verified: &10 []
  documented_version_bounds: Source-derived for exactly 7.3.45; no broader bound claimed.
  read_only_check: Verify exact binary version, managed registration/process identity,
    authenticated health, OpenAPI presence of prompt_async/status, exact directory/session
    lookup, and busy status immediately before send.
  success_criteria: Every endpoint/schema and exact target check matches the registered
    7.3.45 server.
  failure_behavior: Mark unavailable; do not probe by sending and do not fall back
    to stdin or keystroke injection.
  evidence_ids: &11
  - local-cli-7-3-45
  - source-http-7-3-45
  - official-testing
- mechanism_id: http-prompt-async-active
  profile_id: managed-http-server
  os: linux
  versions_verified: *10
  documented_version_bounds: Source-derived for exactly 7.3.45; no broader bound claimed.
  read_only_check: Run the same checks on a native Linux managed server; source/package
    availability alone is insufficient.
  success_criteria: Every endpoint/schema and exact target check matches the registered
    7.3.45 server.
  failure_behavior: Mark unavailable; do not probe by sending and do not fall back
    to stdin or keystroke injection.
  evidence_ids: *11
- mechanism_id: http-prompt-async-active
  profile_id: managed-http-server
  os: windows
  versions_verified: *10
  documented_version_bounds: Source-derived for exactly 7.3.45; no broader bound claimed.
  read_only_check: Run the same checks on native Windows; WSL counts only as Linux-side
    evidence.
  success_criteria: Every endpoint/schema and exact target check matches the registered
    7.3.45 server.
  failure_behavior: Mark unavailable; do not probe by sending and do not fall back
    to stdin or keystroke injection.
  evidence_ids: *11
- mechanism_id: http-prompt-async-idle
  profile_id: managed-http-server
  os: macos
  versions_verified: &12 []
  documented_version_bounds: Source-derived for exactly 7.3.45.
  read_only_check: Same version/health/OpenAPI/session checks, requiring no busy status
    entry immediately before send.
  success_criteria: Exact session exists and is idle in the registered server.
  failure_behavior: Do not send; a busy race requires a separately selected working-session
    operation.
  evidence_ids: &13
  - source-http-7-3-45
  - source-status-7-3-45
- mechanism_id: http-prompt-async-idle
  profile_id: managed-http-server
  os: linux
  versions_verified: *12
  documented_version_bounds: Source-derived for exactly 7.3.45.
  read_only_check: Same version/health/OpenAPI/session checks, requiring no busy status
    entry immediately before send.
  success_criteria: Exact session exists and is idle in the registered server.
  failure_behavior: Do not send; a busy race requires a separately selected working-session
    operation.
  evidence_ids: *13
- mechanism_id: http-prompt-async-idle
  profile_id: managed-http-server
  os: windows
  versions_verified: *12
  documented_version_bounds: Source-derived for exactly 7.3.45.
  read_only_check: Same checks on native Windows; WSL is not native Windows proof.
  success_criteria: Exact session exists and is idle in the registered server.
  failure_behavior: Do not send; a busy race requires a separately selected working-session
    operation.
  evidence_ids: *13
- mechanism_id: http-abort-then-prompt
  profile_id: managed-http-server
  os: macos
  versions_verified: &14 []
  documented_version_bounds: Source-derived for exactly 7.3.45.
  read_only_check: Same exact-target checks plus busy state and explicit approved
    operation; endpoint presence cannot prove runtime cancellation semantics.
  success_criteria: Exact busy session and both endpoint schemas match; user explicitly
    chose interruption.
  failure_behavior: Refuse interruption; automatic warnings never invoke this mechanism.
  evidence_ids: &15
  - source-http-7-3-45
  - source-queue-7-3-45
- mechanism_id: http-abort-then-prompt
  profile_id: managed-http-server
  os: linux
  versions_verified: *14
  documented_version_bounds: Source-derived for exactly 7.3.45.
  read_only_check: Same exact-target checks plus busy state and explicit approved
    operation; endpoint presence cannot prove runtime cancellation semantics.
  success_criteria: Exact busy session and both endpoint schemas match; user explicitly
    chose interruption.
  failure_behavior: Refuse interruption; automatic warnings never invoke this mechanism.
  evidence_ids: *15
- mechanism_id: http-abort-then-prompt
  profile_id: managed-http-server
  os: windows
  versions_verified: *14
  documented_version_bounds: Source-derived for exactly 7.3.45.
  read_only_check: Same checks on native Windows, with native cancellation/process
    verification still required.
  success_criteria: Exact busy session and both endpoint schemas match; user explicitly
    chose interruption.
  failure_behavior: Refuse interruption; automatic warnings never invoke this mechanism.
  evidence_ids: *15
verification: []
cases:
- profile_id: ordinary-cli
  os: macos
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids: &16
  - ordinary-macos-native
  mechanism_ids: &17 []
  prerequisites: &18 []
  evidence_ids: &19
  - official-runtime
  - local-cli-7-3-45
  reason: Ordinary TUI may use a daemon or private worker/embedded fallback, but no
    stable way to discover its chosen endpoint and exact loaded conversation was established.
- profile_id: ordinary-cli
  os: macos
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids: *16
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids: *19
  reason: An open idle TUI is visible as a process, but process identity does not
    identify its selected provider session or writable control endpoint.
- profile_id: ordinary-cli
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-macos-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - official-runtime
  - claudine-wrapper
  reason: Current Claudine knows its child process but does not register a server
    endpoint/session identity; working-session reachability is unresolved.
- profile_id: ordinary-cli
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-macos-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - official-runtime
  - claudine-wrapper
  reason: Current wrapper registration cannot distinguish an idle open conversation
    from an exited or privately embedded runtime.
- profile_id: ordinary-cli
  os: macos
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids: *16
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids: *19
  reason: Ordinary `kilo run` is generally one-shot; it may attach to a daemon, but
    the invocation exposes no stable session-control registration to an independent
    sender.
- profile_id: ordinary-cli
  os: macos
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids: *16
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids: *19
  reason: A completed one-shot process is not an idle open session; a reused daemon
    is a separate unregistered runtime whose conversation ownership is unresolved.
- profile_id: ordinary-cli
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-macos-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - claudine-wrapper
  - official-runtime
  reason: Claudine's one-shot JSON wrapper has no registered steering endpoint; opportunistic
    daemon attachment is not a safe target contract.
- profile_id: ordinary-cli
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-macos-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - claudine-wrapper
  - official-runtime
  reason: The wrapper process exits after the turn; no retained idle-session control
    registration exists.
- profile_id: ordinary-cli
  os: linux
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-linux-native
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids: *19
  reason: Provider architecture suggests the same conditional runtime paths, but native
    Linux ordinary-session discovery and steering were not verified.
- profile_id: ordinary-cli
  os: linux
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-linux-native
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids: *19
  reason: Process/history inspection cannot prove the selected idle conversation or
    endpoint on native Linux.
- profile_id: ordinary-cli
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-linux-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - official-runtime
  - claudine-wrapper
  reason: Current Claudine does not register Kilo endpoint/session identity; Linux
    was not run.
- profile_id: ordinary-cli
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-linux-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - official-runtime
  - claudine-wrapper
  reason: Idle conversation identity and reachability remain unknown on native Linux.
- profile_id: ordinary-cli
  os: linux
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-linux-native
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids: *19
  reason: One-shot versus daemon-backed behavior is conditional and unregistered on
    native Linux.
- profile_id: ordinary-cli
  os: linux
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-linux-native
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids: *19
  reason: A one-shot exits; an ambient daemon is not safely attributable to the invocation.
- profile_id: ordinary-cli
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-linux-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - claudine-wrapper
  - official-runtime
  reason: Current wrapper exposes no retained registered control channel.
- profile_id: ordinary-cli
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-linux-claudine
  mechanism_ids: *17
  prerequisites: *18
  evidence_ids:
  - claudine-wrapper
  - official-runtime
  reason: No retained idle wrapper session exists.
- profile_id: ordinary-cli
  os: windows
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-windows-native
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids: *19
  reason: Native Windows ordinary-session discovery and endpoint attribution are unverified.
- profile_id: ordinary-cli
  os: windows
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-windows-native
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids: *19
  reason: Native Windows process/history data cannot yet identify an idle selected
    conversation.
- profile_id: ordinary-cli
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-windows-claudine
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids:
  - official-runtime
  - claudine-wrapper
  reason: Current wrapper has no registered provider endpoint/session identity on
    native Windows.
- profile_id: ordinary-cli
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-windows-claudine
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids:
  - official-runtime
  - claudine-wrapper
  reason: Idle open-session reachability is unverified on native Windows.
- profile_id: ordinary-cli
  os: windows
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-windows-native
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids: *19
  reason: One-shot versus daemon-backed runtime attribution is unresolved on native
    Windows.
- profile_id: ordinary-cli
  os: windows
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-windows-native
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids: *19
  reason: A one-shot cannot remain idle; any daemon is a separate unregistered target.
- profile_id: ordinary-cli
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - ordinary-windows-claudine
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids:
  - claudine-wrapper
  - official-runtime
  reason: Current Claudine run profile is one-shot and has no registered control endpoint.
- profile_id: ordinary-cli
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - ordinary-windows-claudine
  mechanism_ids: *17
  prerequisites:
  - native Windows verification; WSL is insufficient
  evidence_ids:
  - claudine-wrapper
  - official-runtime
  reason: No retained idle wrapper process or registered daemon conversation exists.
- profile_id: managed-http-server
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: non_interrupting
  discovery_ids: &20
  - managed-macos-claudine
  mechanism_ids: &23
  - http-prompt-async-active
  prerequisites: &21
  - managed authenticated server
  - exact registered directory/session
  - compatible 7.3.45 API
  - mandatory disposable-session test before activation
  evidence_ids: &22
  - source-http-7-3-45
  - source-queue-7-3-45
  - official-testing
  reason: Pinned source establishes queued same-conversation processing without canceling
    the current stream/tool work, but live activation remains blocked.
- profile_id: managed-http-server
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids: *20
  mechanism_ids:
  - http-prompt-async-idle
  prerequisites: *21
  evidence_ids: *22
  reason: The documented async endpoint starts processing for an existing idle session;
    initial 204 is not confirmed conversation delivery and live activation remains
    blocked.
- profile_id: managed-http-server
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - managed-linux-claudine
  mechanism_ids: *23
  prerequisites:
  - managed authenticated server
  - native Linux compatibility and disposable delivery tests
  evidence_ids: *22
  reason: Source/package evidence is cross-platform but native Linux transport and
    delivery were not tested.
- profile_id: managed-http-server
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - managed-linux-claudine
  mechanism_ids:
  - http-prompt-async-idle
  prerequisites:
  - managed authenticated server
  - native Linux compatibility and disposable delivery tests
  evidence_ids: *22
  reason: Idle-start semantics are documented/source-derived but unverified on native
    Linux.
- profile_id: managed-http-server
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - managed-windows-claudine
  mechanism_ids: *23
  prerequisites:
  - managed authenticated server
  - native Windows compatibility and disposable delivery tests; WSL is insufficient
  evidence_ids: *22
  reason: Native Windows server lifecycle, HTTP access, and delivery are unverified.
- profile_id: managed-http-server
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - managed-windows-claudine
  mechanism_ids:
  - http-prompt-async-idle
  prerequisites:
  - managed authenticated server
  - native Windows compatibility and disposable delivery tests; WSL is insufficient
  evidence_ids: *22
  reason: Native Windows idle-turn start remains unverified.
gaps:
- area: Live activation
  detail: No disposable-session record exists; verification is intentionally empty,
    so no mechanism may be enabled.
  next_check: Run non-focusing disposable tests for delivery, same-conversation identity,
    long tools, queue order, and interruption on every claimed OS/version/profile.
- area: Ordinary-session discovery
  detail: TUI/run dynamically choose daemon or private fallback. PID, database history,
    and session list do not prove which conversation a client currently owns, and
    helpers/servers can outlive clients.
  next_check: Instrument disposable ordinary launches to correlate parent/worker/server,
    daemon state, endpoint, directory, selected session, process start identity, and
    client closure without injecting input.
- area: Acceptance and correlation
  detail: 7.3.45 prompt_async returns uncorrelated 204 before background work completes;
    persistence, queue admission, promotion, delivery, duplication, expiration, and
    size bounds are not independently acknowledged.
  next_check: Use unique harmless message content in disposable sessions and correlate
    204, SSE events, message history, model response, disconnects, retries, restart,
    and limits.
- area: Active-operation guard
  detail: The v1 HTTP endpoint targets session ID but exposes no expected turn/tool
    operation ID; status can race after the read-only check.
  next_check: Test stale-target races and evaluate a version-gated V2 interface only
    after it ships in an examined CLI.
- area: Long tools and token loops
  detail: Source establishes delivery only after the current stream and inline tools
    drain. It cannot help a token-generation loop or hung tool that never reaches
    that boundary.
  next_check: Run bounded disposable infinite-stream and long-tool fixtures; keep
    automatic loop warnings non-interrupting and retain a separate hard-stop policy.
- area: Interruption partial failure
  detail: Abort and prompt are separate, uncorrelated operations; cancel clears queued
    follow-ups, and completed tool side effects remain.
  next_check: Test abort during generation, sequential/parallel tools, permission/question
    holds, and deliberate post-abort submission failure.
- area: Prompt interpretation
  detail: Plain text passes through normal Kilo prompt/config/plugin processing. Literal
    preservation versus commands, templates, skills, and extension transformations
    was not established.
  next_check: Test slash-prefixed text, templates, skills, enabled plugins, permissions,
    and input transformations while preserving the user's normal profile.
- area: OS/version bounds
  detail: Only macOS 7.3.45 was inspected locally. Linux and native Windows were not
    run; WSL would be Linux-side evidence only. No release range beyond the pinned
    source is claimed.
  next_check: Repeat read-only compatibility and disposable delivery tests on native
    Linux and Windows and after every protocol-affecting upgrade.
- area: V2 migration
  detail: Main documents a stronger durable inbox and idempotent message IDs, but
    this was not attributed to installed 7.3.45.
  next_check: When a released examined binary exposes V2, inspect its OpenAPI/source
    and test migration, receipts, active snapshots, and compatibility separately.
changes:
- Initial Kilo steering report under schema revision 2.
- Separated the 24 ordinary baseline combinations from a future managed authenticated
  HTTP-server profile.
- Recorded pinned 7.3.45 queue, status, async acknowledgment, and abort semantics
  while leaving live verification empty.
requires_claudine_update: true
reason: Kilo 7.3.45 has a promising documented/source-derived HTTP prompt queue only
  when Claudine deliberately owns and registers an authenticated retained server.
  Current ordinary wrappers do not provide dependable session/endpoint attribution.
  Claudine needs a managed launch profile, strict read-only compatibility checks,
  receipts/event correlation, and mandatory disposable tests before activation; automatic
  warnings must never use interruption.
discovery_gaps: []
interface_inventory:
- disposition: included
  evidence_ids:
  - official-runtime
  - official-cli
  - local-cli-7-3-45
  - claudine-wrapper
  id: profile-ordinary-cli
  profile_ids:
  - ordinary-cli
  reason: Ordinary Kilo TUI or one-shot run, whose runtime may opportunistically attach to a daemon or use a private embedded/worker fallback but exposes no stable registered endpoint contract to an independent sender.
- disposition: included
  evidence_ids:
  - official-runtime
  - official-testing
  - source-http-7-3-45
  - source-queue-7-3-45
  id: profile-managed-http-server
  profile_ids:
  - managed-http-server
  reason: Future Claudine-managed, authenticated `kilo serve` process with endpoint, directory context, and provider session IDs retained in Claudine registration.
- disposition: unknown
  evidence_ids:
  - official-runtime
  - official-testing
  - source-http-7-3-45
  - source-queue-7-3-45
  id: coverage-review-managed-http-server
  profile_ids:
  - managed-http-server
  reason: The existing profile does not cover other client launch modes, other launch origins. This migration does not establish that these combinations are impossible. Review interface ownership, lifetime, and discovery before expanding coverage; do not infer exclusion from current wrapper behavior.
receipt_observations:
- evidence_ids:
  - source-http-7-3-45
  - official-testing
  mechanism_id: http-prompt-async-active
  signal: No response body; later SSE events/history are separate and uncorrelated with the initial receipt. Initial receipt only; later processing and settlement have separate signals.
  timing: early
- evidence_ids:
  - source-http-7-3-45
  - official-testing
  mechanism_id: http-prompt-async-idle
  signal: No response body. Initial receipt only; later processing and settlement have separate signals.
  timing: early
- evidence_ids:
  - source-http-7-3-45
  mechanism_id: http-abort-then-prompt
  signal: Cancellation and replacement have separate outcomes. Two uncorrelated responses plus later SSE/history. The Boolean applies only to the abort call. It says nothing about subsequent prompt acceptance or delivery; the two phases can fail independently.
  timing: multi_phase

---
# Kilo Code steering research

## Overview

Kilo 7.3.45 has a credible non-interrupting steering candidate, but only through a deliberately retained HTTP server. `prompt_async` records a new user message and serializes it behind the currently running session work without canceling the active fiber. The source checks for the queued follow-up after the current model stream and inline tool calls drain. This is a queue boundary, not immediate mid-generation or mid-tool delivery.

This passive investigation was performed by Codex with the requested `gpt-5.6-sol` and low effort recorded by the sequence launcher. The provider-facing execution context did not expose an independent resolved-model metadata field, so model and effort provenance is launcher-supplied rather than independently verified; no contrary resolved metadata was fabricated. No Kilo session was launched and no message or interruption was sent. `verification` is therefore empty and activation is blocked.

## Session discovery

Kilo session IDs are provider identities, not process IDs. An ordinary TUI can attach to a reusable daemon or fall back to a private worker/embedded server, while `kilo run` similarly prefers a daemon and otherwise embeds its server. One process can host multiple directory-keyed runtime instances and multiple sessions. Consequently a PID can identify a live executable but cannot identify the selected conversation, and database/history records establish persistence rather than liveness.

For a managed profile, Claudine should register endpoint, normalized directory, Kilo session ID, server process start identity, and a credential reference at launch. Before sending, it can read health, session list/get, and status. A status entry is process-memory evidence of active work; absence means idle only within that server and does not prove that an arbitrary UI is open on the session. Endpoint + directory + provider session ID is the deduplication key; PID is only a liveness guard.

## Non-interrupting delivery

Pinned 7.3.45 source says the prompt path deliberately does not cancel the in-flight fiber. It enqueues per session, hides later queued prompts from the current model context, and checks for a newer prompt only after the current `handle.process` has drained tokens and inline tool calls. The current step therefore completes and the queued prompt becomes the next same-session work item. A long-running or hung tool delays it, and a token-generation loop that never finishes cannot consume it.

The HTTP handler returns 204 after spawning background processing. Later failures are emitted on the session event stream. Thus 204 confirms only that the endpoint accepted the request far enough to fork work; it does not independently confirm queue persistence, scheduling, promotion into conversation history, or model consumption. With no sender message ID or idempotency contract, an ambiguous response must never be retried.

## Interruption fallback

Kilo exposes a separate session abort endpoint, but non-interrupting queueing is preferable. Manual interruption may be offered only after explaining that the current model/tool work is canceled, already completed side effects remain, and queued follow-ups are cleared. The user must explicitly choose it interactively. Automatic loop warnings must never call abort.

Abort-then-submit is a six-phase sequence, not one atomic operation: revalidate, obtain consent, abort, observe settlement, revalidate the same session, then submit. If abort succeeds but submission fails, the prior work stays canceled and no replacement is guaranteed. Claudine must report that partial outcome and must not retry an ambiguous submission.

## Idle sessions

For a registered server-owned session, `prompt_async` starts asynchronous processing when the session is idle; that starts a new turn in the same persisted conversation. A status race can change an intended idle start into queued active delivery, so Claudine must recheck status and bind the user's selected operation immediately before sending. An ordinary idle TUI remains unknown because neither a PID nor history reveals its selected session or control endpoint.

## Protocol details

The candidate protocol is HTTP with directory routing and optional provider Basic Auth; the Claudine profile should require authentication even on loopback. The request is `POST /session/{sessionID}/prompt_async` with a JSON `parts` array. Global SSE and message history provide later signals, but the 204 has no correlation identifier. Session status is held in process memory and idle removes its map entry.

Normal prompt processing may involve Kilo's modes, skills, templates, commands, and plugins. This pass did not prove byte-for-byte literal interpretation. Permission holds after the message reaches conversation processing are agent/tool authorization states, not delivery states, and must not be mislabeled as queued delivery.

Kilo also exposes ACP, plugins, editor-owned servers, remote relay, and internal TUI worker RPC. None supplies evidence that an independent process can attach to and steer an arbitrary ordinary session. Terminal keystroke injection is not a provider messaging protocol and is excluded as an unsafe, focus-sensitive fallback.

## OS/version compatibility

The installed macOS binary was 7.3.45, and its package lists native artifacts for macOS, Linux, and Windows. Only macOS help/package and pinned source were inspected; no native Linux or Windows runtime was exercised. WSL, if later tested, must be labeled Linux-side evidence and cannot establish native Windows support.

The report claims source behavior only for commit `67b815466c9ab3e022f16692988673712437b881` (tag 7.3.45). Current main documentation describes a newer V2 durable-inbox/idempotency design, but that is not evidence that 7.3.45 exposes it. Read-only compatibility checks must verify version, health, OpenAPI endpoints, target directory/session, and current status; they cannot establish delivery semantics without the mandatory disposable test.

## Disposable-test proposals

Future tests should launch an authenticated, loopback-only, non-focusing server with ordinary user configuration preserved and a disposable directory/session. For each native OS and exact version/profile, test working generation, a bounded long tool, a complete multi-tool batch, idle start, FIFO ordering, concurrent send races, disconnect before response, duplicates, restart persistence, payload limits, prompt transformations, and correlation across HTTP, SSE, history, and model response.

A separate explicitly approved interruption fixture should verify what stops during generation and tools, whether pending permissions/questions clear, whether queued prompts are removed, whether context remains resumable, and the abort-success/submission-failure outcome. A bounded never-ending generation fixture should confirm that queued steering cannot rescue a loop that never reaches the drain boundary.

## Claudine integration

The preferred future profile is a Claudine-owned authenticated `kilo serve`, not stdin retention and not opportunistic attachment. It must preserve normal extensions, skills, prompt templates, context files, explicit model/provider settings, and plugins; `--pure` would silently change semantics and is unacceptable. Registration must be created at launch and cannot retrofit an already open private ordinary session.

Delivery selection should be exact and explicit: queued active-turn follow-up when busy, idle-turn start when idle, or an explained/approved interrupt sequence. Read-only checks and adapter availability are separate from live activation. If the managed RPC is absent or incompatible, the verified fallback is no delivery with a warning that ordinary sessions cannot be safely attributed; terminal injection and blind stdin writes are not fallbacks.

## Gaps

The blockers are the empty live-test gate, ordinary-session attribution, uncorrelated acknowledgments, absent active-operation guards, unknown queue persistence/limits/duplication, prompt transformations, native Linux/Windows behavior, and version migration. These remain explicit rather than being converted to unsupported claims. Kilo's current source also cannot deliver into a token loop or hung tool before the current processing boundary drains.

## Sources

- Kilo Code, [CLI Runtime Architecture](https://kilo.ai/docs/contributing/architecture/cli-runtime).
- Kilo Code, [CLI Command Reference](https://kilo.ai/docs/code-with-ai/platforms/cli-reference).
- Kilo Code repository, [server testing guide](https://github.com/Kilo-Org/kilocode/blob/main/TESTING.md).
- Kilo Code 7.3.45, [session HTTP handlers](https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts), [prompt queue](https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/kilocode/session/prompt-queue.ts), [prompt execution](https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/session/prompt.ts), and [status](https://github.com/Kilo-Org/kilocode/blob/67b815466c9ab3e022f16692988673712437b881/packages/opencode/src/session/status.ts).
- Kilo Code main, [V2 Session API specification](https://github.com/Kilo-Org/kilocode/blob/main/specs/v2/session.md) (comparison only).

## Changelog

- 2026-09-08: Created the revision-2 Kilo report, separating ordinary launches from the managed HTTP profile and pinning 7.3.45 source findings.


## Revision 3 Contract Backfill

Receipt timing, interface inventory, and case-specific discovery gaps were added
from the existing evidence on 2026-09-08. No new provider observation or live test
was performed. Unexamined profile combinations remain unknown, not unsupported.
The original fleet model/effort provenance above describes the research run;
this deterministic contract migration is a separate coordinator edit.
