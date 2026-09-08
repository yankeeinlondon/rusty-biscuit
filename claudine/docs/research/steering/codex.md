---
"$schema": "./_schema.yaml"
schema_revision: 3
provider: codex
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
- Codex CLI 0.153.4 (local macOS installation and generated experimental app-server
  schema)
- openai/codex main app-server protocol documentation observed 2026-09-08
launch_profiles:
- id: ordinary-cli
  description: Ordinary Codex TUI or one-shot exec/resume launch without a deliberately
    exposed peer-control endpoint.
  endpoint_scope: none
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
  - ordinary codex or codex exec launch
  preserves_extensions: 'yes'
  preserves_skills: 'yes'
  preserves_templates: 'yes'
  preserves_context: 'yes'
  evidence_ids:
  - official-cli
  - local-help-0-153-4
  - wrapper-inspection
- id: managed-app-server
  description: Future Claudine-managed client, including non-interactive execution,
    backed by a deliberately retained and registered Codex app-server.
  endpoint_scope: externally_reachable
  lifetime: while_client_open
  applicable_os:
  - macos
  - linux
  - windows
  applicable_launch_modes:
  - interactive
  - non_interactive
  applicable_origins:
  - claudine
  baseline: false
  startup_requirements:
  - launch and retain app-server
  - initialize client
  - register endpoint and provider thread identity
  preserves_extensions: unknown
  preserves_skills: unknown
  preserves_templates: unknown
  preserves_context: 'yes'
  evidence_ids:
  - official-app-server
  - local-help-0-153-4
access_findings:
- mechanism_id: app-server-steer
  os: macos
  status: setup_required
  prerequisite: A deliberately managed app-server endpoint, initialized client connection,
    loaded thread, and exact active turn ID; ordinary TUI/exec stdin is not this channel.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  - local-help-0-153-4
  - local-schema-0-153-4
  profile_id: managed-app-server
- mechanism_id: app-server-steer
  os: linux
  status: setup_required
  prerequisite: A deliberately managed app-server endpoint, initialized client connection,
    loaded thread, and exact active turn ID.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-steer
  os: windows
  status: unknown
  prerequisite: A deliberately managed reachable app-server transport, initialized
    client connection, loaded thread, exact active turn ID, and native Windows verification.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-turn-start
  os: macos
  status: setup_required
  prerequisite: A deliberately managed app-server endpoint and known/resumed thread
    ID with no active regular turn.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  - local-help-0-153-4
  profile_id: managed-app-server
- mechanism_id: app-server-turn-start
  os: linux
  status: setup_required
  prerequisite: A deliberately managed app-server endpoint and known/resumed thread
    ID with no active regular turn.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-turn-start
  os: windows
  status: unknown
  prerequisite: A deliberately managed reachable app-server transport, known/resumed
    idle thread, and native Windows verification.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-interrupt-then-start
  os: macos
  status: setup_required
  prerequisite: Managed app-server, exact thread and active turn IDs, manual approval,
    observed interrupted completion, then a separately accepted turn/start.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  profile_id: managed-app-server
- mechanism_id: app-server-interrupt-then-start
  os: linux
  status: setup_required
  prerequisite: Managed app-server, exact thread and active turn IDs, manual approval,
    observed interrupted completion, then a separately accepted turn/start.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-interrupt-then-start
  os: windows
  status: unknown
  prerequisite: Managed reachable app-server, exact thread and active turn IDs, manual
    approval, and native Windows verification.
  applies_to_existing_sessions: 'no'
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
delivery_states:
- mechanism_id: app-server-steer
  states:
  - accepted
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: JSON-RPC correlates the request response; optional clientUserMessageId
    is echoed on the userMessage item. Success returns the active turn ID, but terminal
    processing requires later item/turn notifications. Revision 1 cannot represent
    persisted, processing, completed, stale-turn rejection, or the final-turn race
    precisely.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
- mechanism_id: app-server-turn-start
  states:
  - accepted
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: The response identifies the new turn and clientUserMessageId can correlate
    the user item; turn/started and turn/completed establish later execution states.
    Request acceptance is not completion.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
- mechanism_id: app-server-interrupt-then-start
  states:
  - accepted
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: Interrupt's empty response acknowledges a cancellation request; turn/completed
    with interrupted status establishes termination. Replacement turn/start has a
    distinct request, turn ID, and lifecycle, so partial failure remains possible.
  evidence_ids:
  - official-app-server
evidence:
- id: official-cli
  method: official_docs
  location: https://developers.openai.com/codex/cli
  version: current documentation observed 2026-09-08
  observed_on: 2026-09-08
  claim: Codex CLI documents interactive and exec workflows and session resume behavior.
  limitations: Resume starts or connects a client workflow around stored conversation
    identity; it does not document arbitrary live exec attachment or steering through
    exec stdin.
- id: official-app-server
  method: source_code
  location: https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/app-server/README.md
  version: rust-v0.153.4
  observed_on: 2026-09-08
  claim: The source pinned to the installed release documents initialize, thread start/resume,
    turn/start, turn/steer with expectedTurnId, turn/interrupt, lifecycle notifications,
    approvals, correlation, and transport framing.
  limitations: Official release source and passive schema evidence are not a live
    delivery test. Several surfaces are explicitly experimental or unsupported for
    production.
- id: local-help-0-153-4
  method: local_inspection
  location: sanitized output of codex --version, codex --help, codex exec --help,
    codex exec resume --help, codex resume --help, and codex app-server --help on
    the research host
  version: 0.153.4
  observed_on: 2026-09-08
  claim: The installed macOS CLI reads an exec initial prompt from an argument or
    stdin; resume is a new CLI invocation; app-server is an explicit experimental
    mode with stdio, Unix-socket, or WebSocket listeners and daemon/proxy commands.
  limitations: Passive inspection on one macOS host; no process was launched and no
    Linux or Windows runtime was inspected.
- id: local-schema-0-153-4
  method: local_inspection
  location: sanitized temporary output of codex app-server generate-json-schema --experimental
  version: 0.153.4
  observed_on: 2026-09-08
  claim: The installed protocol schema includes separate threadId, expectedTurnId,
    input, and optional clientUserMessageId for turn steering, plus distinct turn
    start and interrupt requests.
  limitations: Shape inspection does not establish runtime delivery, ordering, retries,
    or cross-platform behavior.
- id: wrapper-inspection
  method: source_code
  location: claudine/cli/src/commands/wrap/profile/codex.rs
  version: workspace state observed 2026-09-08
  observed_on: 2026-09-08
  claim: Claudine currently launches codex exec, supplies the initial non-interactive
    prompt through stdin, and resumes with codex exec resume; it does not establish
    an app-server control endpoint.
  limitations: Local wrapper behavior is not provider capability and may change independently.
- id: final-race-inference
  method: inference
  location: Inference from expectedTurnId precondition and separate turn/start lifecycle
    in the official app-server protocol
  version: openai/codex main observed 2026-09-08
  observed_on: 2026-09-08
  claim: A steer racing active-turn completion must fail or be handled separately;
    silently creating a new turn would change the operation's meaning.
  limitations: Exact error code, persistence behavior, and retry contract were not
    live-tested or pinned to 0.153.4 source.
discovery:
- id: discover-macos-native
  os: macos
  origin: native
  method: process_inspection
  locator: Process inspection can label codex candidates, but only an explicitly recorded
    app-server endpoint plus thread/loaded/list and thread/read can identify a currently
    available destination without loading one.
  identity_check: Validate endpoint ownership/configuration, initialize the connection,
    read the exact thread without resuming it, and obtain its current active turn
    ID from protocol state/events; notLoaded is unavailable for immediate steering.
  liveness_check: Successful initialized JSON-RPC round trip plus passive thread/read;
    PID alone is insufficient.
  state_detection: Use thread/read status and observed turn lifecycle events; snapshot
    state can race.
  available_labels:
  - pid
  - command
  - cwd
  - threadId
  - turnId
  prerequisites:
  - explicit app-server launch/registration
  evidence_ids:
  - official-app-server
  - local-help-0-153-4
  profile_id: ordinary-cli
  observation_source: process inspection plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-macos-claudine
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Future managed launches record endpoint, transport ownership, server process,
    thread ID, and active-turn observations.
  identity_check: Provider threadId is authoritative; CLAUDINE_SESSION_ID and wrapper
    PID are correlation only. Require expectedTurnId immediately before steering.
  liveness_check: Initialized protocol request plus matching server/thread registration.
  state_detection: Track turn notifications and reconcile with thread read; reject
    stale active-turn identity.
  available_labels:
  - threadId
  - turnId
  - cwd
  - server_pid
  - endpoint
  - launch_profile
  prerequisites:
  - new app-server-managed Claudine launch
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  profile_id: ordinary-cli
  observation_source: launch registration plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-linux-native
  os: linux
  origin: native
  method: process_inspection
  locator: Process inspection is candidate labeling only; require an explicitly known
    app-server endpoint and passive thread/loaded/list plus thread/read results.
  identity_check: Initialize, read the exact thread without resuming, require loaded
    state, and match the current active turn.
  liveness_check: Successful initialized protocol round trip plus passive thread/read.
  state_detection: thread/read status and protocol turn lifecycle events; snapshot
    state is race-prone.
  available_labels:
  - pid
  - command
  - cwd
  - threadId
  - turnId
  prerequisites:
  - same namespace and explicit endpoint
  evidence_ids:
  - official-app-server
  profile_id: ordinary-cli
  observation_source: process inspection plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-linux-claudine
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Future launch-time registration of endpoint, ownership, thread ID, and
    active turns.
  identity_check: Validate provider IDs and launch record before every operation.
  liveness_check: Initialized protocol round trip and thread lookup.
  state_detection: Tracked turn notifications plus reconciliation.
  available_labels:
  - threadId
  - turnId
  - cwd
  - server_pid
  - endpoint
  - launch_profile
  prerequisites:
  - new app-server-managed Claudine launch
  evidence_ids:
  - official-app-server
  profile_id: ordinary-cli
  observation_source: launch registration plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-windows-native
  os: windows
  origin: native
  method: process_inspection
  locator: Process inspection is candidate labeling only; a deliberately reachable
    app-server endpoint and passive thread/loaded/list plus thread/read results are
    still required.
  identity_check: Initialize, read without resuming, require loaded state, and validate
    the exact active turn; native transport/security behavior remains unverified.
  liveness_check: Successful initialized protocol round trip plus passive thread/read.
  state_detection: thread/read status and protocol notifications when available; otherwise
    unknown.
  available_labels:
  - pid
  - command
  - cwd
  - threadId
  - turnId
  prerequisites:
  - explicit endpoint
  - native Windows verification
  evidence_ids:
  - official-app-server
  profile_id: ordinary-cli
  observation_source: process inspection plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-windows-claudine
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Future launch-time registration of a supported endpoint, server ownership,
    thread ID, and active turns.
  identity_check: Validate provider IDs and endpoint security before every operation.
  liveness_check: Initialized protocol round trip and thread lookup.
  state_detection: Tracked turn notifications plus reconciliation.
  available_labels:
  - threadId
  - turnId
  - cwd
  - server_pid
  - endpoint
  - launch_profile
  prerequisites:
  - new app-server-managed Claudine launch
  - native Windows verification
  evidence_ids:
  - official-app-server
  profile_id: ordinary-cli
  observation_source: launch registration plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-managed-macos-claudine
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Future managed launches record endpoint, transport ownership, server process,
    thread ID, and active-turn observations.
  identity_check: Provider threadId is authoritative; CLAUDINE_SESSION_ID and wrapper
    PID are correlation only. Require expectedTurnId immediately before steering.
  liveness_check: Initialized protocol request plus matching server/thread registration.
  state_detection: Track turn notifications and reconcile with thread read; reject
    stale active-turn identity.
  available_labels:
  - threadId
  - turnId
  - cwd
  - server_pid
  - endpoint
  - launch_profile
  prerequisites:
  - new app-server-managed Claudine launch
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  profile_id: managed-app-server
  observation_source: launch registration plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-managed-linux-claudine
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Future launch-time registration of endpoint, ownership, thread ID, and
    active turns.
  identity_check: Validate provider IDs and launch record before every operation.
  liveness_check: Initialized protocol round trip and thread lookup.
  state_detection: Tracked turn notifications plus reconciliation.
  available_labels:
  - threadId
  - turnId
  - cwd
  - server_pid
  - endpoint
  - launch_profile
  prerequisites:
  - new app-server-managed Claudine launch
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
  observation_source: launch registration plus provider API
  observed_at: 2026-09-08 passive research snapshot
- id: discover-managed-windows-claudine
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Future launch-time registration of a supported endpoint, server ownership,
    thread ID, and active turns.
  identity_check: Validate provider IDs and endpoint security before every operation.
  liveness_check: Initialized protocol round trip and thread lookup.
  state_detection: Tracked turn notifications plus reconciliation.
  available_labels:
  - threadId
  - turnId
  - cwd
  - server_pid
  - endpoint
  - launch_profile
  prerequisites:
  - new app-server-managed Claudine launch
  - native Windows verification
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
  observation_source: launch registration plus provider API
  observed_at: 2026-09-08 passive research snapshot
mechanisms:
- id: app-server-steer
  interface_status: documented
  transport: other
  conversation_effect: preserve_running_turn
  delivery_boundary: unknown
  destination: An app-server-managed thread's exact active regular turn, addressed
    by both threadId and expectedTurnId over its owning initialized JSON-RPC connection.
  authentication: Stdio is inherited from the process owner; Unix socket security
    depends on path permissions; WebSocket listeners have configured token modes.
    Release 0.153.4 calls TCP WebSocket experimental/unsupported. Do not expose credentials
    in discovery.
  startup_requirements:
  - explicit app-server or managed daemon
  - initialize/initialized handshake
  request_format: 'JSON-RPC request: {"method":"turn/steer","id":<request-id>,"params":{"threadId":"<thread>","expectedTurnId":"<active-turn>","input":[{"type":"text","text":"<message>"}],"clientUserMessageId":"<optional-correlation>"}}.'
  response_format: Success identifies the same turn; userMessage items can echo clientUserMessageId.
    Later item and turn notifications establish processing/completion. Protocol errors
    reject invalid, stale, idle, or ineligible targets.
  long_tool_behavior: The protocol accepts input for the active turn rather than starting
    a new one. Exact incorporation timing during token generation or a long command/tool
    requires disposable tests. A pending approval after the user message is accepted
    is execution state, not a delivery hold.
  ordering: Submission ordering follows the app-server/core queue, but concurrent-client
    ordering and behavior when multiple steers race are not documented sufficiently
    for activation.
  duplicate_handling: clientUserMessageId provides correlation; no documented idempotency
    or duplicate-suppression guarantee was found.
  cancellation: No recall operation for accepted steer input was found; turn/interrupt
    cancels the whole active turn and is a separate operation.
  limits: Regular directly controlled active turns only; review/manual-compaction
    turns and parent-owned Multi-Agent V2 subagents reject steering. Requires exact
    active-turn identity and a managed app-server profile. Ingress overload returns
    JSON-RPC -32001 and is retryable with backoff, but retry safety for a possibly
    accepted request remains unproven.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  - final-race-inference
  maturity: experimental
  initialization: Launch or connect to the retained app-server, complete initialize/initialized,
    then identify or resume the target thread.
  operation_intent: steer_active_turn
  target_preconditions:
  - explicit app-server or managed daemon
  - registered reachable transport
  - initialize/initialized handshake
  - known loaded threadId
  - fresh exact expectedTurnId
  - experimental API capability where required by the installed protocol
  target_guards:
  - required expectedTurnId must equal active turn; absent, stale, idle, review, and
    manual-compaction targets reject
  request_framing: JSONL on stdio; one JSON-RPC message per WebSocket text frame on
    socket transports.
  response_framing: Matching JSON-RPC response followed by asynchronous lifecycle
    notifications.
  tool_batch_behavior: unknown
  queue_behavior: Same-turn input, not a documented next-turn queue; exact incorporation
    timing is unverified.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable.
  sender_message_id: supported
  retry_policy: unsafe_possible_duplicate
- id: app-server-turn-start
  interface_status: documented
  transport: other
  conversation_effect: resume_same_conversation
  delivery_boundary: idle_turn_start
  destination: A known app-server thread with no active regular turn. thread/resume
    loads or rejoins the conversation; turn/start creates a distinct new turn.
  authentication: Same transport boundary as app-server-steer.
  startup_requirements:
  - managed app-server
  - initialized connection
  request_format: JSON-RPC turn/start with threadId, input, and optional clientUserMessageId;
    thread/resume is separate and does not itself deliver the prompt.
  response_format: Initial turn response followed by turn/started, item lifecycle
    notifications, and turn/completed.
  long_tool_behavior: Not applicable when the idle precondition is true. Starting
    while another regular turn is active needs explicit provider behavior testing
    and must not be treated as steering.
  ordering: Each accepted new turn has its own ID; concurrent admission and idle-to-busy
    races require testing.
  duplicate_handling: Optional clientUserMessageId correlates a user item but is not
    documented as an idempotency key.
  cancellation: Use turn/interrupt with the returned turn ID; completion is asynchronous.
  limits: Idle-thread delivery only for this research classification; an ordinary
    completed exec process is not a reachable idle app-server.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  maturity: experimental
  initialization: Launch or connect to the retained app-server, complete initialize/initialized,
    then identify or resume the target thread.
  operation_intent: start_idle_turn
  target_preconditions:
  - managed app-server
  - initialized connection
  - known threadId
  - confirmed idle thread
  target_guards:
  - known threadId and confirmed idle state; idle observation is race-prone
  request_framing: JSONL on stdio; one JSON-RPC message per WebSocket text frame on
    socket transports.
  response_framing: Matching JSON-RPC response followed by asynchronous lifecycle
    notifications.
  tool_batch_behavior: not_applicable
  queue_behavior: Starts an idle turn; concurrent idle admission behavior is unverified.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable.
  sender_message_id: supported
  retry_policy: unsafe_possible_duplicate
- id: app-server-interrupt-then-start
  interface_status: documented
  transport: other
  conversation_effect: cancel_turn_same_conversation
  delivery_boundary: next_turn
  destination: First the exact active (threadId, turnId), then the same thread for
    a separate replacement turn/start.
  authentication: Same managed app-server transport boundary.
  startup_requirements: []
  request_format: turn/interrupt with threadId and turnId, followed only after terminal
    observation by a separate turn/start request.
  response_format: Interrupt returns {}; later turn/completed with interrupted status
    proves termination. Replacement has its own turn response and lifecycle.
  long_tool_behavior: Requests cancellation of the active turn. Background terminals
    may survive and require separate cleanup; command/tool cleanup effects need live
    verification.
  ordering: Wait for terminal interruption and revalidate idle state before replacement;
    final-turn and new-turn races can otherwise target the wrong lifecycle.
  duplicate_handling: Repeated interrupt or replacement behavior is not an idempotency
    contract; clientUserMessageId only correlates replacement input.
  cancellation: The interruption is itself cancellation; no rollback is promised.
  limits: Manual fallback only; cannot satisfy non-interrupting automatic loop-warning
    requirements.
  evidence_ids:
  - official-app-server
  - final-race-inference
  maturity: experimental
  initialization: Launch or connect to the retained app-server, complete initialize/initialized,
    then identify or resume the target thread.
  operation_intent: interrupt_then_submit
  target_preconditions:
  - manual approval
  - fresh exact active turn ID
  - observe matching turn/completed status interrupted
  - revalidate thread idle
  - submit replacement turn/start
  target_guards:
  - turn/interrupt addresses exact threadId and turnId; revalidate idle before replacement
  request_framing: JSONL on stdio; one JSON-RPC message per WebSocket text frame on
    socket transports.
  response_framing: Matching JSON-RPC response followed by asynchronous lifecycle
    notifications.
  tool_batch_behavior: provider_defined
  queue_behavior: No replacement is queued atomically; wait for interrupted completion,
    then separately start.
  message_interpretation: provider_defined
  interruption_phases:
  - request exact-turn interruption
  - observe matching interrupted completion
  - revalidate same thread is idle
  - submit separate turn/start
  - observe replacement lifecycle
  interruption_partial_failure: Interruption may complete while replacement submission
    fails, leaving the same conversation idle; background terminals may remain and
    no rollback is promised.
  sender_message_id: supported
  retry_policy: unsafe_possible_duplicate
compatibility:
- mechanism_id: app-server-steer
  os: macos
  versions_verified:
  - 0.153.4 passive schema only
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Check codex --version, generate the installed experimental app-server
    schema, require turn/steer fields, validate endpoint ownership/auth, initialize,
    and confirm thread/turn identity.
  success_criteria: Installed schema contains compatible turn/steer and managed endpoint/thread/active-turn
    checks pass; live delivery remains separately required.
  failure_behavior: Do not offer steering; report unsupported profile/version or unknown
    active turn.
  evidence_ids:
  - local-help-0-153-4
  - local-schema-0-153-4
  profile_id: managed-app-server
- mechanism_id: app-server-steer
  os: linux
  versions_verified: []
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Same installed-schema and managed-endpoint feature probe on Linux.
  success_criteria: Compatible schema, initialized endpoint, exact active turn; live
    test still required.
  failure_behavior: Disable mechanism with explicit reason.
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-steer
  os: windows
  versions_verified: []
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Generate installed schema and verify a supported authenticated
    transport on native Windows.
  success_criteria: Compatible schema and secure initialized endpoint with exact turn
    identity; live test still required.
  failure_behavior: Disable mechanism.
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-turn-start
  os: macos
  versions_verified:
  - 0.153.4 passive schema only
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Probe installed schema for thread/resume and turn/start; validate
    registered endpoint and idle thread.
  success_criteria: Compatible methods and identity/state checks pass; live test still
    required.
  failure_behavior: Do not create a turn.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  profile_id: managed-app-server
- mechanism_id: app-server-turn-start
  os: linux
  versions_verified: []
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Probe installed schema and managed idle thread.
  success_criteria: Compatible methods and identity/state checks pass.
  failure_behavior: Do not create a turn.
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-turn-start
  os: windows
  versions_verified: []
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Probe installed schema and secure native transport.
  success_criteria: Compatible methods and identity/state checks pass.
  failure_behavior: Do not create a turn.
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-interrupt-then-start
  os: macos
  versions_verified:
  - 0.153.4 passive schema only
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Probe exact-ID interrupt and turn/start schemas; validate current
    active turn before asking for approval.
  success_criteria: Compatible methods and identity checks pass; live interruption/replacement
    test still required.
  failure_behavior: Do not interrupt.
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  profile_id: managed-app-server
- mechanism_id: app-server-interrupt-then-start
  os: linux
  versions_verified: []
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Probe installed schemas and active-turn identity.
  success_criteria: Compatible methods and identity checks pass.
  failure_behavior: Do not interrupt.
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
- mechanism_id: app-server-interrupt-then-start
  os: windows
  versions_verified: []
  documented_version_bounds: No reliable introduction bound found.
  read_only_check: Probe installed schemas, secure native transport, and active-turn
    identity.
  success_criteria: Compatible methods and identity checks pass.
  failure_behavior: Do not interrupt.
  evidence_ids:
  - official-app-server
  profile_id: managed-app-server
verification: []
cases:
- os: macos
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-macos-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - local-help-0-153-4
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: macos
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-macos-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: macos
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-macos-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: macos
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-macos-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: macos
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-macos-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - official-cli
  - local-help-0-153-4
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: macos
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-macos-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - official-cli
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-macos-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - wrapper-inspection
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-macos-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - wrapper-inspection
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-linux-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-linux-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-linux-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-linux-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-linux-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - official-cli
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-linux-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-linux-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-linux-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-windows-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-windows-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-windows-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-windows-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-windows-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - official-cli
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-windows-native
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-windows-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-windows-claudine
  mechanism_ids: []
  prerequisites:
  - No externally reachable prompt RPC is established for an ordinary launch.
  evidence_ids:
  - official-app-server
  - wrapper-inspection
  reason: Ordinary Codex launch discovery and external delivery are not established;
    app-server capability belongs to a separate managed profile.
  profile_id: ordinary-cli
- profile_id: managed-app-server
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: non_interrupting
  discovery_ids:
  - discover-managed-macos-claudine
  mechanism_ids:
  - app-server-steer
  - app-server-interrupt-then-start
  prerequisites:
  - retained registered app-server
  - exact active regular turn ID
  - matching disposable test before activation
  evidence_ids:
  - official-app-server
  reason: Documented turn/steer admits input to the exact active regular turn without
    canceling it; runtime activation remains blocked by empty verification.
- profile_id: managed-app-server
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-managed-macos-claudine
  mechanism_ids:
  - app-server-turn-start
  prerequisites:
  - retained registered app-server
  - known idle thread
  - matching disposable test before activation
  evidence_ids:
  - official-app-server
  reason: Documented turn/start can begin a turn in the same managed idle thread;
    runtime activation remains blocked by empty verification.
- profile_id: managed-app-server
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: non_interrupting
  discovery_ids:
  - discover-managed-linux-claudine
  mechanism_ids:
  - app-server-steer
  - app-server-interrupt-then-start
  prerequisites:
  - retained registered app-server
  - exact active regular turn ID
  - matching disposable test before activation
  evidence_ids:
  - official-app-server
  reason: Documented turn/steer admits input to the exact active regular turn without
    canceling it; runtime activation remains blocked by empty verification.
- profile_id: managed-app-server
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-managed-linux-claudine
  mechanism_ids:
  - app-server-turn-start
  prerequisites:
  - retained registered app-server
  - known idle thread
  - matching disposable test before activation
  evidence_ids:
  - official-app-server
  reason: Documented turn/start can begin a turn in the same managed idle thread;
    runtime activation remains blocked by empty verification.
- profile_id: managed-app-server
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: non_interrupting
  discovery_ids:
  - discover-managed-windows-claudine
  mechanism_ids:
  - app-server-steer
  - app-server-interrupt-then-start
  prerequisites:
  - retained registered app-server
  - exact active regular turn ID
  - matching disposable test before activation
  evidence_ids:
  - official-app-server
  reason: Documented turn/steer admits input to the exact active regular turn without
    canceling it; runtime activation remains blocked by empty verification.
- profile_id: managed-app-server
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-managed-windows-claudine
  mechanism_ids:
  - app-server-turn-start
  prerequisites:
  - retained registered app-server
  - known idle thread
  - matching disposable test before activation
  evidence_ids:
  - official-app-server
  reason: Documented turn/start can begin a turn in the same managed idle thread;
    runtime activation remains blocked by empty verification.
- profile_id: managed-app-server
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: non_interrupting
  discovery_ids: [discover-managed-macos-claudine]
  mechanism_ids: [app-server-steer, app-server-interrupt-then-start]
  prerequisites: [retained registered app-server, exact active regular turn ID, matching disposable test before activation]
  evidence_ids: [official-app-server, local-help-0-153-4, local-schema-0-153-4]
  reason: App-server is a headless protocol process, and documented turn/steer does not require a TTY or human client; a future Claudine-managed non-interactive client can preserve the active turn, pending live verification.
- profile_id: managed-app-server
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids: [discover-managed-macos-claudine]
  mechanism_ids: [app-server-turn-start]
  prerequisites: [retained registered app-server, known idle thread, matching disposable test before activation]
  evidence_ids: [official-app-server, local-help-0-153-4, local-schema-0-153-4]
  reason: Documented turn/start can start work in the same managed idle thread through the headless protocol; no human client is required, pending live verification.
- profile_id: managed-app-server
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: non_interrupting
  discovery_ids: [discover-managed-linux-claudine]
  mechanism_ids: [app-server-steer, app-server-interrupt-then-start]
  prerequisites: [retained registered app-server, exact active regular turn ID, matching disposable test before activation]
  evidence_ids: [official-app-server]
  reason: Versioned source defines app-server as headless and turn/steer as protocol input independent of a TTY; native Linux runtime remains unverified.
- profile_id: managed-app-server
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids: [discover-managed-linux-claudine]
  mechanism_ids: [app-server-turn-start]
  prerequisites: [retained registered app-server, known idle thread, matching disposable test before activation]
  evidence_ids: [official-app-server]
  reason: Versioned source defines turn/start over the headless protocol without a human client; native Linux runtime remains unverified.
- profile_id: managed-app-server
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids: [discover-managed-windows-claudine]
  mechanism_ids: [app-server-steer, app-server-interrupt-then-start]
  prerequisites: [retained registered app-server, exact active regular turn ID, matching native Windows disposable test before activation]
  evidence_ids: [official-app-server]
  reason: The protocol is headless and does not inherently require a human, but transport reachability and native Windows behavior remain unverified.
- profile_id: managed-app-server
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids: [discover-managed-windows-claudine]
  mechanism_ids: [app-server-turn-start]
  prerequisites: [retained registered app-server, known idle thread, matching native Windows disposable test before activation]
  evidence_ids: [official-app-server]
  reason: The protocol can express idle turn/start without a human client, but transport reachability and native Windows behavior remain unverified.
gaps:
- area: schema launch profiles
  detail: 'Already identified by OpenCode: interactive/non_interactive cannot distinguish
    ordinary TUI, one-shot exec, remote TUI, stdio app-server, Unix-socket server,
    WebSocket server, or managed daemon. Codex access changes completely across these
    profiles.'
  next_check: Add launch_profile and transport-ownership records referenced by cases.
- area: active-turn identity and concurrency
  detail: 'NEW: revision 1 models session_state but has no active turn ID, identity
    freshness, observed-at value, or compare-and-submit precondition. Codex turn/steer
    requires expectedTurnId, and a final-turn race can turn a valid snapshot stale
    without changing conversation identity.'
  next_check: Add provider conversation ID and active operation/turn identity, source/timestamp/confidence,
    expected-ID precondition, and stale-target outcome.
- area: operation intent
  detail: 'NEW: revision 1 cannot distinguish steer-current-turn, start-idle-turn,
    resume/load-thread, and interrupt-then-start. Codex exposes these as separate
    operations with different IDs and effects.'
  next_check: Add normalized operation kind and multi-phase operation records instead
    of overloading conversation_effect and delivery_boundary.
- area: target eligibility and ownership role
  detail: 'NEW: active is insufficient. Review/manual-compaction turns and parent-owned
    Multi-Agent V2 subagents reject direct steering. Experimental thread/read canAcceptDirectInput
    and thread/loaded/list expose useful but version-gated eligibility facts that
    revision 1 cannot store.'
  next_check: Add target kind, owner/controller role, direct-input eligibility, source,
    and confidence to destination state.
- area: delivery lifecycle vocabulary
  detail: 'Already identified by OpenCode and reinforced here: accepted, persisted,
    injected/processing, user-item observed, completed, rejected-stale, canceled,
    and stranded cannot be represented. A later approval/tool hold is execution state
    after delivery, not held delivery.'
  next_check: Expand delivery states and model post-delivery execution/approval separately.
- area: acknowledgment and terminal observation
  detail: Codex steer/start responses establish admission and identity, while notifications
    establish item and turn progress. Interrupt's empty response acknowledges only
    the cancellation request; matching turn/completed establishes termination.
  next_check: Represent acknowledgment level, terminal observation channel, timeout,
    and unknown-after-disconnect explicitly.
- area: message identity and idempotency
  detail: 'Already identified by OpenCode: clientUserMessageId offers correlation
    but no verified retry/idempotency guarantee. Revision 1 stores correlation only
    as prose.'
  next_check: Add structured client message identity, duplicate policy, and retry
    safety; test timeout and concurrent submissions.
- area: server ownership and attachment
  detail: No evidence shows a fresh app-server process can seize or attach to an arbitrary
    ordinary exec/TUI process. thread/resume addresses conversation history or a thread
    already owned by that app-server, not process attachment.
  next_check: Define Claudine-owned app-server lifecycle/registration and treat unmanaged
    sessions as unavailable unless an official attachment contract is found.
- area: authentication and OS transport
  detail: The schema's transport enum cannot express JSON-RPC over several selectable
    transports cleanly. Stdio ownership, Unix socket permissions, WebSocket authentication,
    daemon discovery, and native Windows support require separate evidence.
  next_check: Model protocol independently from transport and authentication; verify
    supported endpoints on macOS, Linux, and native Windows.
- area: wire framing and support policy
  detail: 'NEW: revision 1 has one transport enum and one interface-status enum. In
    0.153.4, stdio carries JSONL, unix:// carries WebSocket frames after an HTTP Upgrade
    over UDS, and ws:// uses the same framing but is explicitly experimental/unsupported.
    Socket kind, framing, stability, and production-support policy are independent
    facts.'
  next_check: Add protocol/framing and stability/support-policy fields separate from
    carrier transport and documented status.
- area: peer-control exposure
  detail: 'NEW: an ordinary TUI can have internal app-server machinery while externally
    usable peer control is disabled, enabled, or deliberately exposed at startup.
    Process presence does not reveal that access boundary, and revision 1 cannot represent
    it.'
  next_check: Add peer_control/exposure mode to launch profiles and require launch-time
    registration; never infer external reachability from an ordinary TUI process.
- area: provider-native next-turn queue
  detail: The pinned 0.153.4 protocol also documents experimental thread/queue/add
    with stable submission and client message IDs, FIFO idle submission, update/delete/reorder,
    and a 100-message limit. Revision 1 cannot distinguish this durable next-turn
    queue from same-turn steer, and this bounded pilot did not activate or live-test
    it.
  next_check: Evaluate queue/add as a separate mechanism in the revised schema and
    test admission, persistence, duplicate IDs, interruption-paused queues, and idle
    races.
- area: interruption partial failure
  detail: 'Already identified by OpenCode and sharper with exact turn IDs: interrupt
    and replacement are separate. The target can finish first, interruption can complete
    without replacement, and background terminals can survive.'
  next_check: Model phase-specific outcomes and test final-turn races, long tools,
    background terminals, and failed replacement.
- area: mandatory activation gate
  detail: No disposable-session delivery record exists for any Codex mechanism or
    OS.
  next_check: Keep Codex steering disabled until matching non-focusing tests prove
    destination, active-turn guard, long-generation/tool behavior, acknowledgment,
    retry behavior, and interruption effects.
changes:
- Refreshed the revision 1 Codex pilot into steering schema revision 2.
- Separated the 24 ordinary baseline combinations from the managed app-server profile.
- Added all six Claudine-managed non-interactive app-server cases; app-server is a
  headless protocol and does not inherently require a TTY or human client.
- Added structured receipt guarantees, launch-profile access, exact-turn guards, interruption
  phases, and relational coverage.
- Preserved verification as empty; no live delivery test was performed.
requires_claudine_update: true
reason: Codex app-server provides candidate exact-turn and idle-turn RPCs, but ordinary
  launches expose no evidenced peer endpoint and all activation remains blocked pending
  disposable tests and cross-platform verification.
receipt_guarantees:
- mechanism_id: app-server-steer
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - turn/steer response returns accepting turnId
  - later userMessage item may echo clientUserMessageId
  - later turn/completed reports turn outcome
  correlation: message_id
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  limitations: Initial success proves admission to the named active turn, not durable
    persistence, model incorporation, or completion.
- mechanism_id: app-server-turn-start
  request_acceptance: confirmed
  persistence: unknown
  scheduling: confirmed
  conversation_delivery: unknown
  provider_signals:
  - turn/start response returns a turn
  - turn/started confirms execution began
  - item and turn/completed notifications provide later lifecycle
  correlation: message_id
  evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  limitations: Initial response identifies an admitted turn; only later notifications
    establish start and completion.
- mechanism_id: app-server-interrupt-then-start
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - interrupt response is empty success
  - matching turn/completed with interrupted status confirms cancellation
  - replacement turn has separate lifecycle
  correlation: operation_id
  evidence_ids:
  - official-app-server
  limitations: Initial interrupt acknowledgment proves only cancellation request acceptance;
    replacement delivery can fail separately.
discovery_gaps: []
interface_inventory:
- disposition: included
  evidence_ids:
  - official-cli
  - local-help-0-153-4
  - wrapper-inspection
  id: profile-ordinary-cli
  profile_ids:
  - ordinary-cli
  reason: Ordinary Codex TUI or one-shot exec/resume launch without a deliberately exposed peer-control endpoint.
- disposition: included
  evidence_ids:
  - official-app-server
  - local-help-0-153-4
  id: profile-managed-app-server
  profile_ids:
  - managed-app-server
  reason: Future Claudine-managed client, including non-interactive execution, backed by a deliberately retained and registered Codex app-server.
- disposition: unknown
  evidence_ids:
  - official-app-server
  - local-help-0-153-4
  id: coverage-review-managed-app-server
  profile_ids:
  - managed-app-server
  reason: The existing profile does not cover other launch origins. This migration does not establish that these combinations are impossible. Review interface ownership, lifetime, and discovery before expanding coverage; do not infer exclusion from current wrapper behavior.
receipt_observations:
- evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  mechanism_id: app-server-steer
  signal: Success identifies the same turn; userMessage items can echo clientUserMessageId. Later item and turn notifications establish processing/completion. Protocol errors reject invalid, stale, idle, or ineligible targets. Initial receipt only; later processing and settlement have separate signals.
  timing: early
- evidence_ids:
  - official-app-server
  - local-schema-0-153-4
  mechanism_id: app-server-turn-start
  signal: Initial turn response followed by turn/started, item lifecycle notifications, and turn/completed. Initial receipt only; later processing and settlement have separate signals.
  timing: early
- evidence_ids:
  - official-app-server
  mechanism_id: app-server-interrupt-then-start
  signal: Cancellation and replacement have separate outcomes. Interrupt returns {}; later turn/completed with interrupted status proves termination. Replacement has its own turn response and lifecycle. Initial interrupt acknowledgment proves only cancellation request acceptance; replacement delivery can fail separately.
  timing: multi_phase

---
# Steering Research: Codex CLI

## Overview

Codex CLI 0.153.4 exposes a promising non-interrupting mechanism through its
experimental app-server protocol: `turn/steer` targets the exact active turn in a
thread. That capability belongs to a deliberately managed app-server connection.
It does not make an ordinary `codex` TUI or `codex exec` process attachable.
The managed client can itself be non-interactive: `codex app-server` is a headless
protocol process, and initialize, thread, turn, notification, and approval traffic
do not require a human TTY. A future integration may replace Claudine's one-shot
`codex exec` launch with a retained app-server while preserving the non-interactive
user-facing mode.

This passive refresh was performed by agent `codex` with launcher-supplied model
`gpt-5.6-sol` and reasoning effort `low`. The provider did not expose independent
resolved-model execution metadata to this report, so launcher configuration is
the provenance and is not presented as an independent runtime verification. No
provider session was launched, messaged, or interrupted. Revision 2 separates
ordinary launches from managed app-server launches; the empty verification set
still blocks activation for every mechanism.

## Session discovery

A Codex conversation/thread ID and its active turn ID are different identities.
`turn/steer` requires both `threadId` and `expectedTurnId`. This compare-and-submit
shape prevents a stale client from steering a later turn after the intended turn
finishes, but it also means a thread list or rollout file alone is insufficient.
The sender must own or reach the app-server, initialize its JSON-RPC connection,
inspect an already-loaded exact thread, and maintain fresh turn lifecycle state.

Pinned 0.153.4 also exposes `thread/loaded/list` and an experimental
`canAcceptDirectInput` field on `thread/read`. Those help distinguish loaded and
directly controllable targets, but they are version-gated eligibility signals,
not substitutes for the exact active-turn guard.

Process inspection can label candidate Codex processes. It cannot establish that
an ordinary TUI or exec process exposes an app-server endpoint. Future Claudine
managed launches should record server ownership, transport, process identity,
thread ID, and active-turn observations. Claudine's own session ID remains
correlation metadata unless explicitly mapped to the provider thread ID.

Installed help also shows that a user can explicitly launch app-server with stdio,
Unix-socket, or WebSocket listener options. That supports a candidate native-exposed
profile in principle, but this pass does not establish endpoint discovery,
authentication, ownership checks, or native Windows transport behavior. It remains
an explicit gap rather than being merged into ordinary native CLI cases.

## Non-interrupting delivery

For a managed active regular turn, `turn/steer` is the provider-native candidate.
The request carries input and the expected active turn ID; an optional
`clientUserMessageId` is echoed on the resulting user-message item. The response
acknowledges admission to that turn. Later item and turn notifications are needed
to prove processing and completion.

The exact-turn guard creates an unavoidable final-turn race: the turn can finish
between observation and request handling. A stale-target response should be
surfaced, not retried against a newly active turn. Whether a retry should instead
start a new idle turn is user intent, not a transport decision.

Source documentation establishes acceptance for an active turn, but it does not
prove rescue during uninterrupted token generation. The installed schema
confirms the request shape; incorporation during long generation or a long tool
remains unknown until source or disposable-test evidence establishes its
boundary. This remains researched capability rather than activated support.

## Interruption fallback

`turn/interrupt` requires the exact `(threadId, turnId)` and returns an empty
JSON-RPC result when the cancellation request is accepted. The matching later
`turn/completed` notification with `interrupted` status establishes completion.
Only then may a client safely revalidate idle state and submit `turn/start`.

Those are separate operations. The original turn may finish before interruption,
interruption may succeed while replacement fails, and background terminals may
survive. This fallback therefore requires manual approval and cannot implement an
automatic non-interrupting warning.

## Idle sessions

For a retained managed thread with no active turn, `turn/start` creates a new
turn in the same conversation. `thread/resume` loads or rejoins a thread; it does
not itself deliver input. An ordinary completed `codex exec` process is not an
idle reachable server. Running `codex exec resume <id>` starts another CLI
invocation around stored conversation identity and must not be described as
attaching a live sender to the earlier exec process.

## Protocol details

App-server uses JSON-RPC after an `initialize` request and `initialized`
notification. Framing varies by carrier: stdio uses JSONL; `unix://` performs an
HTTP Upgrade and carries WebSocket frames over a Unix-domain socket; `ws://`
carries one JSON-RPC message per WebSocket text frame but release 0.153.4 calls
that TCP transport experimental and unsupported. Installed help also exposes
managed daemon/proxy commands. Protocol, carrier, framing, stability, and
authentication need separate metadata fields.

An ordinary TUI may use internal app-server machinery without enabling a peer
control listener. Peer control being off, explicitly enabled, or exposed through
a managed daemon is launch state. A process record cannot prove which applies.

Turn and item notifications provide lifecycle evidence. Server-initiated approval
requests can pause execution after steering input was delivered. That state is
not a queued or held incoming message. The app-server client must service or
decline approvals according to policy; a delivery report must keep this separate
from admission and processing of the steering message.

No idempotency guarantee was found for `clientUserMessageId`. Concurrent senders,
timeouts, disconnects, duplicate IDs, and stale expected-turn retries require
disposable tests.

Pinned 0.153.4 source also documents experimental `thread/queue/add`, a FIFO
next-turn queue with a stable server submission ID and required client message
ID. It can persist follow-up input while a turn is active and start it when the
thread becomes idle. That is distinct from same-turn steering and deserves its
own mechanism after the schema revision; this pilot records it as a gap because
it was neither live-tested nor adequately expressible in revision 1.

## OS and version compatibility

The installed macOS CLI and generated protocol schema are 0.153.4. Official
openai/codex source documents the current protocol but provides no reliable
historical version bound for these semantics. Linux and native Windows were not
run. Compatibility should generate or inspect the installed app-server schema,
validate the selected transport and authentication, initialize the connection,
and verify thread/turn identity. That read-only probe still does not replace live
delivery verification.

## Disposable-test proposals

Use non-focusing, isolated, authenticated managed app-servers on macOS, Linux,
and native Windows. Test idle `turn/start`; steering during long token generation,
a long command, and multi-tool work; steering while an approval is pending;
concurrent sends; duplicate client message IDs; disconnect, timeout, and retry;
the exact final-turn race; stale and wrong turn IDs; interrupt during generation
and tools; surviving background terminals; and replacement failure after a
successful interrupt. Capture sanitized requests, responses, notifications, and
provider version/schema fixtures.

## Claudine integration

The current wrapper launches `codex exec`, sends the initial prompt through
stdin, and resumes with `codex exec resume`. Stdin in that contract is initial
prompt input, not a bidirectional steering transport. Safe steering requires a
new explicit managed app-server launch profile and durable registration. The
adapter must preserve thread and active-turn identities, reject stale turn
snapshots, expose admission separately from terminal processing, and never turn a
failed steer into an implicit new turn.
This managed profile applies to both interactive and non-interactive Claudine
invocations; implementation is missing, but provider support is setup-required
rather than unsupported.

## Gaps

The OpenCode pilot already exposed launch-profile, delivery-state, message-ID,
server-ownership, authentication, interruption, OS-evidence, and activation-gate
gaps. Codex confirms those and adds two concrete requirements: first-class active
turn identity with compare-and-submit concurrency semantics, and an operation
kind separating steer, idle turn creation, thread resume, and interrupt/replace.
Revision 2 now carries those distinctions structurally. Remaining blockers are
runtime verification, native Linux and Windows evidence, ordinary-session
attachment evidence, and an implemented Claudine adapter.
The explicitly launched native app-server surface also needs its own profile once
endpoint ownership, authentication, and discovery have evidence.

## Changelog

The revision 1 pilot was refreshed to revision 2. The refresh partitions all 24
ordinary baseline combinations from a managed app-server profile, adds receipt
guarantees and exact-turn guards, and retains the prior passive findings without
claiming that any live-delivery activation gate passed.
It now includes managed non-interactive cases because app-server is headless and
does not require a human client.

## Sources

- [Codex CLI documentation](https://developers.openai.com/codex/cli)
- [Codex 0.153.4 app-server protocol in the official repository](https://github.com/openai/codex/blob/rust-v0.153.4/codex-rs/app-server/README.md)
- [Official openai/codex repository](https://github.com/openai/codex)


## Revision 3 Contract Backfill

Receipt timing, interface inventory, and case-specific discovery gaps were added
from the existing evidence on 2026-09-08. No new provider observation or live test
was performed. Unexamined profile combinations remain unknown, not unsupported.
The original fleet model/effort provenance above describes the research run;
this deterministic contract migration is a separate coordinator edit.
