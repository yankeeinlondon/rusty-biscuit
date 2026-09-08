---
"$schema": "./_schema.yaml"
schema_revision: 3
provider: qwen
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
  - Qwen Code 0.19.8 (installed Homebrew package, bundled documentation, and generated JavaScript on macOS)
  - Qwen Code official documentation and QwenLM/qwen-code v0.19.8 source distribution
launch_profiles:
  - id: ordinary-cli
    description: Ordinary Qwen interactive TUI or one-shot prompt launch without a peer-control server.
    endpoint_scope: none
    lifetime: unknown
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [interactive, non_interactive]
    applicable_origins: [native, claudine]
    baseline: true
    startup_requirements: [ordinary qwen launch]
    preserves_extensions: 'yes'
    preserves_skills: 'yes'
    preserves_templates: 'yes'
    preserves_context: 'yes'
    evidence_ids: [official-overview, local-help, wrapper-source]
  - id: managed-http-daemon
    description: Future Claudine-managed qwen serve process with registered HTTP endpoint, client identity, session IDs, and SSE subscriptions.
    endpoint_scope: externally_reachable
    lifetime: long_lived
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [claudine]
    baseline: false
    startup_requirements: [launch qwen serve before creating or attaching sessions, bind loopback with required bearer authentication, register endpoint workspace process identity and session IDs, retain SSE subscriptions]
    preserves_extensions: 'yes'
    preserves_skills: 'yes'
    preserves_templates: partial
    preserves_context: 'yes'
    evidence_ids: [official-serve, source-sdk-0198, local-help]
access_findings:
  - { mechanism_id: daemon-follow-up, profile_id: managed-http-daemon, os: macos, status: setup_required, prerequisite: "Claudine must deliberately launch/authenticate the daemon, create or attach the session, retain its clientId and SSE stream, and register the endpoint.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve, source-sdk-0198] }
  - { mechanism_id: daemon-follow-up, profile_id: managed-http-daemon, os: linux, status: setup_required, prerequisite: "The same managed daemon setup is required; Linux runtime is untested and WSL would be Linux-side evidence only.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve] }
  - { mechanism_id: daemon-follow-up, profile_id: managed-http-daemon, os: windows, status: setup_required, prerequisite: "The same managed daemon setup is required; native Windows runtime is untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve] }
  - { mechanism_id: daemon-idle-prompt, profile_id: managed-http-daemon, os: macos, status: setup_required, prerequisite: "A registered live daemon session whose status reports hasActivePrompt false is required.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve, source-sdk-0198] }
  - { mechanism_id: daemon-idle-prompt, profile_id: managed-http-daemon, os: linux, status: setup_required, prerequisite: "A registered live daemon session is required; Linux runtime is untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve] }
  - { mechanism_id: daemon-idle-prompt, profile_id: managed-http-daemon, os: windows, status: setup_required, prerequisite: "A registered live daemon session is required; native Windows runtime is untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve] }
  - { mechanism_id: daemon-cancel-then-prompt, profile_id: managed-http-daemon, os: macos, status: setup_required, prerequisite: "Managed session plus explicit interactive consent and observation of the old turn's terminal event are required.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve, source-sdk-0198] }
  - { mechanism_id: daemon-cancel-then-prompt, profile_id: managed-http-daemon, os: linux, status: setup_required, prerequisite: "Managed session and explicit consent are required; Linux behavior is untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve] }
  - { mechanism_id: daemon-cancel-then-prompt, profile_id: managed-http-daemon, os: windows, status: setup_required, prerequisite: "Managed session and explicit consent are required; native Windows behavior is untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-serve] }
delivery_states:
  - { mechanism_id: daemon-follow-up, states: [accepted, queued, delivered, refused, expired, unknown], observable_by_external_sender: partial, correlation: "HTTP 202 returns promptId and lastEventId; SSE turn_complete or turn_error correlates by promptId. Queue position and durable persistence are not established.", evidence_ids: [official-serve, source-sdk-0198] }
  - { mechanism_id: daemon-idle-prompt, states: [accepted, delivered, refused, expired, unknown], observable_by_external_sender: partial, correlation: "The 202 acceptance and terminal SSE event correlate by promptId; no distinct delivered-before-complete acknowledgment exists.", evidence_ids: [official-serve, source-sdk-0198] }
  - { mechanism_id: daemon-cancel-then-prompt, states: [accepted, queued, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "Cancel's 204 and the old turn terminal event are separate from the replacement promptId and its SSE result.", evidence_ids: [official-serve, source-sdk-0198] }
receipt_guarantees:
  - { mechanism_id: daemon-follow-up, request_acceptance: confirmed, persistence: not_persisted, scheduling: confirmed, conversation_delivery: unknown, provider_signals: [HTTP 202 promptId and lastEventId, SSE turn_complete, SSE turn_error, HTTP 503 prompt_queue_full], correlation: message_id, evidence_ids: [official-serve, source-sdk-0198], limitations: "Initial 202 proves bounded admission into the live in-memory FIFO, not model receipt or completion; daemon restart loses in-flight state." }
  - { mechanism_id: daemon-idle-prompt, request_acceptance: confirmed, persistence: not_persisted, scheduling: confirmed, conversation_delivery: unknown, provider_signals: [HTTP 202 promptId and lastEventId, SSE turn_complete, SSE turn_error], correlation: message_id, evidence_ids: [official-serve, source-sdk-0198], limitations: "The initial receipt is admission only; correlated terminal SSE is later evidence, and no live test exists." }
  - { mechanism_id: daemon-cancel-then-prompt, request_acceptance: unknown, persistence: unknown, scheduling: unknown, conversation_delivery: unknown, provider_signals: [HTTP 204 cancel response, old turn terminal SSE, replacement HTTP 202 and terminal SSE], correlation: unknown, evidence_ids: [official-serve, source-sdk-0198], limitations: "Cancel and replacement are non-atomic; a 204 does not prove tool termination or replacement admission." }
evidence:
  - { id: official-overview, method: official_docs, location: "https://qwenlm.github.io/qwen-code-docs/en/users/overview", version: "documentation observed 2026-09-08", observed_on: 2026-09-08, claim: "Qwen Code documents ordinary interactive and non-interactive CLI use, configuration, context, extensions, skills, hooks, MCP, and session resume.", limitations: "It does not expose an independent peer endpoint for an already-open ordinary process." }
  - { id: official-serve, method: official_docs, location: "https://qwenlm.github.io/qwen-code-docs/en/users/qwen-serve", version: "v0.19.8-bundled documentation and current site observed 2026-09-08", observed_on: 2026-09-08, claim: "Experimental qwen serve provides HTTP plus SSE, live session status, bounded FIFO prompt admission, promptId correlation, cancellation, replay, authentication, and session load/resume.", limitations: "Documentation/source evidence is not a disposable delivery test; production-grade and cross-restart guarantees are explicitly limited." }
  - { id: source-sdk-0198, method: source_code, location: "https://github.com/QwenLM/qwen-code/blob/v0.19.8/packages/sdk/src/daemon/client.ts", version: v0.19.8, observed_on: 2026-09-08, claim: "The SDK handles 202 prompt admission, correlates turn_complete/turn_error by promptId over SSE, exposes nonblocking prompt and cancel, and treats prompt admission separately from completion.", limitations: "The release bundle was inspected locally; source inspection does not establish runtime behavior." }
  - { id: local-help, method: local_inspection, location: "sanitized passive Sniff/program and qwen --version/--help/serve --help inspection on the research host", version: 0.19.8, observed_on: 2026-09-08, claim: "Qwen Code 0.19.8 is installed on macOS; help advertises ordinary prompt modes, sessions, experimental serve HTTP bridge, queue caps, auth, workspace binding, and SSE replay controls.", limitations: "No Qwen process was launched, focused, attached, or messaged; Linux and native Windows were not inspected." }
  - { id: wrapper-source, method: source_code, location: "claudine/cli/src/commands/wrap/profile/qwen.rs", version: "workspace state observed 2026-09-08", observed_on: 2026-09-08, claim: "Claudine's ordinary Qwen wrapper launches prompt modes and does not create/register a qwen serve endpoint.", limitations: "Current Claudine behavior is not evidence of provider impossibility." }
  - { id: ordinary-inference, method: inference, location: "comparison of official overview, serve documentation, local help, and wrapper source", version: "Qwen Code 0.19.8", observed_on: 2026-09-08, claim: "Ordinary TUI/stdin/history interfaces should not be treated as peer steering; qwen serve is a distinct launch profile and cannot enable already-open ordinary sessions.", limitations: "Absence of a documented ordinary endpoint does not prove impossibility, so ordinary cases remain unknown." }
discovery:
  - { id: ordinary-macos-native, profile_id: ordinary-cli, os: macos, origin: native, method: process_inspection, locator: "Sniff same-user process discovery plus Qwen project/session metadata", identity_check: "Correlate executable, process start identity, CWD, and provider session ID; PID and newest history alone are insufficient.", liveness_check: "Process existence identifies only a candidate and must reject PID reuse.", state_detection: "No documented passive working/idle signal for ordinary sessions.", observation_source: "sniff and passive Qwen metadata", observed_at: runtime, available_labels: [pid, executable, cwd, session_id], prerequisites: [same OS user], evidence_ids: [official-overview, local-help] }
  - { id: ordinary-macos-claudine, profile_id: ordinary-cli, os: macos, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff process and Qwen metadata", identity_check: "Capture provider session ID and process start identity; do not infer from PID/history order.", liveness_check: "Validate child start identity; conversation attachment remains unresolved.", state_detection: "Unknown.", observation_source: "Claudine registry, sniff, Qwen metadata", observed_at: runtime, available_labels: [pid, cwd, origin, session_id], prerequisites: [Claudine launch registration], evidence_ids: [wrapper-source] }
  - { id: ordinary-linux-native, profile_id: ordinary-cli, os: linux, origin: native, method: process_inspection, locator: "Sniff Linux process discovery plus Qwen metadata", identity_check: "Require executable/CWD/start identity and session-ID correlation.", liveness_check: "Candidate process only; WSL is Linux-side evidence.", state_detection: "Unknown.", observation_source: "sniff and Qwen metadata", observed_at: runtime, available_labels: [pid, executable, cwd, session_id], prerequisites: [same Linux user], evidence_ids: [official-overview] }
  - { id: ordinary-linux-claudine, profile_id: ordinary-cli, os: linux, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff Linux process discovery", identity_check: "Bind session ID to process start identity.", liveness_check: "Validate launch identity; attachment unresolved.", state_detection: "Unknown.", observation_source: "Claudine registry and sniff", observed_at: runtime, available_labels: [pid, cwd, origin, session_id], prerequisites: [Claudine launch registration], evidence_ids: [wrapper-source] }
  - { id: ordinary-windows-native, profile_id: ordinary-cli, os: windows, origin: native, method: process_inspection, locator: "Sniff native-Windows same-token process discovery plus Qwen metadata", identity_check: "Require executable/CWD/start identity and session-ID correlation.", liveness_check: "Candidate process only; native Windows untested.", state_detection: "Unknown.", observation_source: "sniff and Qwen metadata", observed_at: runtime, available_labels: [pid, executable, cwd, session_id], prerequisites: [same Windows process-token user], evidence_ids: [official-overview] }
  - { id: ordinary-windows-claudine, profile_id: ordinary-cli, os: windows, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff native-Windows process discovery", identity_check: "Bind session ID to process start identity.", liveness_check: "Validate launch identity; native Windows untested.", state_detection: "Unknown.", observation_source: "Claudine registry and sniff", observed_at: runtime, available_labels: [pid, cwd, origin, session_id], prerequisites: [Claudine launch registration], evidence_ids: [wrapper-source] }
  - { id: daemon-macos-claudine, profile_id: managed-http-daemon, os: macos, origin: claudine, method: provider_api, locator: "registered authenticated endpoint; GET /capabilities, GET /daemon/status, and GET /session/:id/status", identity_check: "Exact daemon endpoint/workspace plus provider sessionId and Claudine clientId; one ACP child may host multiple sessions.", liveness_check: "HTTP health/capabilities and session status 200; session 404 is stale.", state_detection: "hasActivePrompt distinguishes working from idle for a live session.", observation_source: "Qwen daemon API and Claudine launch registry", observed_at: runtime, available_labels: [session_id, workspace, created_at, display_name, client_count, has_active_prompt], prerequisites: [managed launch, endpoint registration, bearer token], evidence_ids: [official-serve] }
  - { id: daemon-linux-claudine, profile_id: managed-http-daemon, os: linux, origin: claudine, method: provider_api, locator: "registered authenticated daemon endpoint and status APIs", identity_check: "Endpoint/workspace/sessionId/clientId tuple.", liveness_check: "Health/capabilities/session status; Linux untested.", state_detection: "hasActivePrompt.", observation_source: "Qwen daemon API and Claudine registry", observed_at: runtime, available_labels: [session_id, workspace, client_count, has_active_prompt], prerequisites: [managed launch, bearer token], evidence_ids: [official-serve] }
  - { id: daemon-windows-claudine, profile_id: managed-http-daemon, os: windows, origin: claudine, method: provider_api, locator: "registered authenticated daemon endpoint and status APIs", identity_check: "Endpoint/workspace/sessionId/clientId tuple.", liveness_check: "Health/capabilities/session status; native Windows untested.", state_detection: "hasActivePrompt.", observation_source: "Qwen daemon API and Claudine registry", observed_at: runtime, available_labels: [session_id, workspace, client_count, has_active_prompt], prerequisites: [managed launch, bearer token], evidence_ids: [official-serve] }
mechanisms:
  - id: daemon-follow-up
    interface_status: documented
    maturity: experimental
    transport: http
    initialization: Launch qwen serve, authenticate, preflight capabilities/workspace, create or attach a session, and subscribe to its SSE stream.
    operation_intent: queue_follow_up
    conversation_effect: preserve_running_turn
    delivery_boundary: next_turn
    destination: Exact live daemon sessionId at the registered endpoint/workspace.
    authentication: Bearer token should be mandatory even on loopback; clientId identifies the attached client but is not authenticated identity.
    startup_requirements: [special qwen serve launch, registered endpoint and session, retained SSE subscription]
    target_preconditions: [live session status 200, hasActivePrompt true, prompt_queue capability, queue below advertised cap]
    target_guards: [sessionId, workspace binding, clientId; no expected active prompt or operation ID guard]
    request_framing: "POST /session/:id/prompt JSON with prompt content and client identity; successful admission returns HTTP 202."
    response_framing: "202 {promptId,lastEventId}; later SSE turn_complete or turn_error carries promptId."
    request_format: "JSON text content blocks; daemon body limit is 10 MiB."
    response_format: "JSON HTTP envelope plus SSE envelopes {id,v,type,data,originatorClientId?}."
    long_tool_behavior: Accepted prompts remain FIFO behind the active prompt and therefore do not interrupt a long-running tool; they are not immediate delivery into that tool or generation.
    tool_batch_behavior: continue_all
    queue_behavior: Per-session FIFO includes active and queued prompts; default cap five; overflow is synchronous 503 prompt_queue_full with Retry-After 5; in-memory queue is lost on daemon restart.
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: Not applicable; this mechanism does not interrupt.
    ordering: FIFO per session; cross-session scheduling shares one ACP child and exact fairness is not established.
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: promptId is daemon-assigned after acceptance; no caller-supplied idempotency key or duplicate suppression is documented.
    cancellation: A separate session cancel operation targets current work; automatic warnings must not use it.
    limits: Default five accepted unsettled prompts per session, 10 MiB JSON body, bounded SSE replay and subscriber queues; optional deadlines/rate limits may expire/refuse work.
    evidence_ids: [official-serve, source-sdk-0198]
  - id: daemon-idle-prompt
    interface_status: documented
    maturity: experimental
    transport: http
    initialization: Same daemon/session/SSE initialization as daemon-follow-up; verify hasActivePrompt false immediately before admission.
    operation_intent: start_idle_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: idle_turn_start
    destination: Exact live idle daemon sessionId.
    authentication: Bearer token plus clientId as above.
    startup_requirements: [special qwen serve launch, registered live session]
    target_preconditions: [status 200, hasActivePrompt false]
    target_guards: [sessionId and workspace; no atomic expected-idle guard]
    request_framing: "POST /session/:id/prompt with JSON text content."
    response_framing: "HTTP 202 promptId/lastEventId, then correlated SSE terminal event."
    request_format: JSON text content blocks.
    response_format: JSON and SSE envelopes.
    long_tool_behavior: Not applicable to a correctly observed idle session; state can race before admission.
    tool_batch_behavior: not_applicable
    queue_behavior: Starts immediately if still idle; otherwise joins FIFO, so the idle check is advisory rather than atomic.
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: Not applicable.
    ordering: FIFO per session.
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: No caller idempotency key or suppression documented.
    cancellation: Separate session cancel is available after admission.
    limits: Same prompt/body/rate/deadline limits as daemon-follow-up.
    evidence_ids: [official-serve, source-sdk-0198]
  - id: daemon-cancel-then-prompt
    interface_status: documented
    maturity: experimental
    transport: http
    initialization: Managed daemon session and SSE state ledger; require explicit interactive user approval.
    operation_intent: interrupt_then_submit
    conversation_effect: cancel_turn_same_conversation
    delivery_boundary: next_turn
    destination: Current work and then the same exact sessionId.
    authentication: Bearer token and registered clientId.
    startup_requirements: [special qwen serve launch, explicit human consent]
    target_preconditions: [live working session, correlated outstanding prompt]
    target_guards: [sessionId only; no expected promptId guard for cancel]
    request_framing: "POST /session/:id/cancel, observe old terminal state, then POST /session/:id/prompt."
    response_framing: "Cancel returns 204; replacement returns 202 plus later correlated SSE."
    request_format: Empty cancel request then JSON prompt.
    response_format: HTTP status plus SSE envelopes.
    long_tool_behavior: Exact subprocess/tool termination and context checkpoint behavior are unverified.
    tool_batch_behavior: unknown
    queue_behavior: Existing queued prompts may remain; cancellation scope and queue retention require a disposable test before use.
    message_interpretation: provider_defined
    interruption_phases: [confirm exact target and obtain user approval, submit cancel, observe old prompt terminal event and side effects, submit replacement prompt, observe correlated replacement terminal event]
    interruption_partial_failure: If cancel succeeds but replacement fails, the original turn is stopped and the steering message is neither accepted nor known durable; queued-prompt disposition is unknown.
    ordering: Replacement must not be submitted until the old terminal event; existing queued entries may precede it.
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: No caller idempotency key.
    cancellation: Provider cancellation is explicit but only session-scoped in the examined interface.
    limits: Same admission limits; cancel race and queue disposition unverified.
    evidence_ids: [official-serve, source-sdk-0198]
compatibility:
  - { mechanism_id: daemon-follow-up, profile_id: managed-http-daemon, os: macos, versions_verified: [], documented_version_bounds: "Introduced experimentally in v0.16-alpha; no stable upper compatibility bound documented.", read_only_check: "Sniff qwen version; authenticate GET /capabilities and require daemon envelope v1 plus prompt queue/SSE/status features and matching workspace.", success_criteria: "Reviewed version and required feature tags/limits are present.", failure_behavior: "Fail closed; do not send or retry.", evidence_ids: [official-serve, local-help] }
  - { mechanism_id: daemon-follow-up, profile_id: managed-http-daemon, os: linux, versions_verified: [], documented_version_bounds: "v0.16-alpha introduction; runtime range unverified.", read_only_check: "Same version/capabilities/workspace checks; label WSL as Linux only.", success_criteria: "Reviewed native Linux version and required features.", failure_behavior: "Fail closed.", evidence_ids: [official-serve] }
  - { mechanism_id: daemon-follow-up, profile_id: managed-http-daemon, os: windows, versions_verified: [], documented_version_bounds: "v0.16-alpha introduction; native Windows range unverified.", read_only_check: "Same version/capabilities/workspace checks on native Windows.", success_criteria: "Reviewed native Windows version and required features.", failure_behavior: "Fail closed.", evidence_ids: [official-serve] }
  - { mechanism_id: daemon-idle-prompt, profile_id: managed-http-daemon, os: macos, versions_verified: [], documented_version_bounds: "No stable bounds.", read_only_check: "Require reviewed version/capabilities and status.hasActivePrompt=false.", success_criteria: "Exact live session reports idle immediately before send.", failure_behavior: "Do not send as idle; it could queue.", evidence_ids: [official-serve, local-help] }
  - { mechanism_id: daemon-idle-prompt, profile_id: managed-http-daemon, os: linux, versions_verified: [], documented_version_bounds: "No stable bounds.", read_only_check: "Same checks on native Linux.", success_criteria: "Reviewed version and exact idle session.", failure_behavior: "Fail closed.", evidence_ids: [official-serve] }
  - { mechanism_id: daemon-idle-prompt, profile_id: managed-http-daemon, os: windows, versions_verified: [], documented_version_bounds: "No stable bounds.", read_only_check: "Same checks on native Windows.", success_criteria: "Reviewed version and exact idle session.", failure_behavior: "Fail closed.", evidence_ids: [official-serve] }
  - { mechanism_id: daemon-cancel-then-prompt, profile_id: managed-http-daemon, os: macos, versions_verified: [], documented_version_bounds: "No stable bounds.", read_only_check: "Require cancel capability, exact outstanding prompt ledger, working status, and explicit consent.", success_criteria: "Reviewed version and correlated target are present.", failure_behavior: "Never interrupt automatically.", evidence_ids: [official-serve, source-sdk-0198] }
  - { mechanism_id: daemon-cancel-then-prompt, profile_id: managed-http-daemon, os: linux, versions_verified: [], documented_version_bounds: "No stable bounds.", read_only_check: "Same checks on native Linux.", success_criteria: "Reviewed version, target, and consent.", failure_behavior: "Never interrupt automatically.", evidence_ids: [official-serve] }
  - { mechanism_id: daemon-cancel-then-prompt, profile_id: managed-http-daemon, os: windows, versions_verified: [], documented_version_bounds: "No stable bounds.", read_only_check: "Same checks on native Windows.", success_criteria: "Reviewed version, target, and consent.", failure_behavior: "Never interrupt automatically.", evidence_ids: [official-serve] }
verification: []
cases:
  # The ordinary baseline is the exact 3 OS × 2 launch modes × 2 origins × 2 states product.
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "No independent peer endpoint is documented for an ordinary active TUI." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "Keyboard input is not an external messaging protocol." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "Wrapping does not expose qwen serve." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "Launch registration improves discovery, not delivery." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-help, ordinary-inference], reason: "Initial stdin/prompt is not retained steering." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-help, ordinary-inference], reason: "One-shot idle lifetime is not established." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "Wrapper stdin is not a prompt RPC." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "No retained idle receiver." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "No documented peer endpoint; Linux untested." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "No external idle delivery interface." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "No peer channel." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "Discovery is not delivery." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "Initial prompt channel is not retained." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "One-shot idle lifetime is unknown." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "No prompt RPC." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "No retained receiver." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "No documented peer endpoint; native Windows untested." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "No external idle delivery interface." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "No peer channel." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "Discovery is not delivery." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "Initial prompt channel is not retained." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-overview, ordinary-inference], reason: "One-shot idle lifetime is unknown." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "No prompt RPC." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-inference], reason: "No retained receiver." }
  - { profile_id: managed-http-daemon, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [daemon-macos-claudine], mechanism_ids: [daemon-follow-up], prerequisites: [managed authenticated daemon, exact session, disposable live test], evidence_ids: [official-serve, source-sdk-0198], reason: "The documented FIFO accepts a follow-up without canceling current work; activation remains blocked." }
  - { profile_id: managed-http-daemon, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [daemon-macos-claudine], mechanism_ids: [daemon-idle-prompt], prerequisites: [managed authenticated daemon, exact idle session, disposable live test], evidence_ids: [official-serve, source-sdk-0198], reason: "Prompt admission starts a turn in the same live idle session." }
  - { profile_id: managed-http-daemon, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [daemon-linux-claudine], mechanism_ids: [daemon-follow-up], prerequisites: [managed authenticated daemon, Linux disposable live test], evidence_ids: [official-serve], reason: "Documented capability; Linux runtime unverified." }
  - { profile_id: managed-http-daemon, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [daemon-linux-claudine], mechanism_ids: [daemon-idle-prompt], prerequisites: [managed authenticated daemon, Linux disposable live test], evidence_ids: [official-serve], reason: "Documented capability; Linux runtime unverified." }
  - { profile_id: managed-http-daemon, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [daemon-windows-claudine], mechanism_ids: [daemon-follow-up], prerequisites: [managed authenticated daemon, native Windows disposable live test], evidence_ids: [official-serve], reason: "Documented capability; native Windows runtime unverified." }
  - { profile_id: managed-http-daemon, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [daemon-windows-claudine], mechanism_ids: [daemon-idle-prompt], prerequisites: [managed authenticated daemon, native Windows disposable live test], evidence_ids: [official-serve], reason: "Documented capability; native Windows runtime unverified." }
gaps:
  - { area: mandatory activation testing, detail: "verification is empty; no candidate mechanism is enabled.", next_check: "Run isolated daemon delivery tests for each exact OS/version/profile/state after coordinator approval." }
  - { area: queue semantics, detail: "FIFO admission is documented, but exact long-tool behavior, multi-tool batch completion, cancellation scope, queue retention, and cross-session fairness lack live proof.", next_check: "Test generation, long subprocess, multi-tool batch, multiple queued prompts, cancel races, and filesystem/process outcomes." }
  - { area: acceptance and retry, detail: "202 confirms admission/scheduling but not model receipt; no caller idempotency key exists.", next_check: "Test disconnects, ambiguous responses, duplicates, overflow, deadlines, daemon/child crash, and SSE replay gaps; never retry ambiguity automatically." }
  - { area: ordinary discovery, detail: "History and PID cannot prove live conversation attachment or state.", next_check: "Inspect sanitized session metadata and define process-start/session-ID correlation without mutation." }
  - { area: profile preservation, detail: "Daemon docs say boot configuration/context/extensions/skills are loaded, but local-only TUI commands/templates are absent and equivalence is untested.", next_check: "Compare inventories and behavior against an ordinary launch with identical explicit settings." }
  - { area: compatibility, detail: "No stable version range or completed native Linux/Windows runtime verification exists.", next_check: "Use a reviewed-version allowlist plus capability negotiation and test native macOS, Linux, and Windows; WSL counts only as Linux." }
changes:
  - Initial Qwen Code steering report using schema revision 2.
requires_claudine_update: true
reason: A future managed qwen serve profile could provide discoverable FIFO follow-up and idle-turn delivery, while ordinary sessions remain unknown and all activation is blocked by empty verification.
discovery_gaps: []
interface_inventory:
- disposition: included
  evidence_ids:
  - official-overview
  - local-help
  - wrapper-source
  id: profile-ordinary-cli
  profile_ids:
  - ordinary-cli
  reason: Ordinary Qwen interactive TUI or one-shot prompt launch without a peer-control server.
- disposition: included
  evidence_ids:
  - official-serve
  - source-sdk-0198
  - local-help
  id: profile-managed-http-daemon
  profile_ids:
  - managed-http-daemon
  reason: Future Claudine-managed qwen serve process with registered HTTP endpoint, client identity, session IDs, and SSE subscriptions.
- disposition: unknown
  evidence_ids:
  - official-serve
  - source-sdk-0198
  - local-help
  id: coverage-review-managed-http-daemon
  profile_ids:
  - managed-http-daemon
  reason: The existing profile does not cover other client launch modes, other launch origins. This migration does not establish that these combinations are impossible. Review interface ownership, lifetime, and discovery before expanding coverage; do not infer exclusion from current wrapper behavior.
receipt_observations:
- evidence_ids:
  - official-serve
  - source-sdk-0198
  mechanism_id: daemon-follow-up
  signal: JSON HTTP envelope plus SSE envelopes {id,v,type,data,originatorClientId?}. Initial receipt only; later processing and settlement have separate signals.
  timing: early
- evidence_ids:
  - official-serve
  - source-sdk-0198
  mechanism_id: daemon-idle-prompt
  signal: JSON and SSE envelopes. Initial receipt only; later processing and settlement have separate signals.
  timing: early
- evidence_ids:
  - official-serve
  - source-sdk-0198
  mechanism_id: daemon-cancel-then-prompt
  signal: Cancellation and replacement have separate outcomes. HTTP status plus SSE envelopes. Cancel and replacement are non-atomic; a 204 does not prove tool termination or replacement admission.
  timing: multi_phase

---

# Qwen Code steering research

## Overview

Qwen Code 0.19.8 has a credible but experimental steering surface: `qwen serve`. It is a distinct long-lived HTTP+SSE daemon that hosts ACP sessions and accepts multiple clients; it is not present in ordinary TUI or one-shot launches. Passive Sniff/local inspection found 0.19.8 on macOS. No process was launched or contacted. Research execution metadata identifies Codex `gpt-5.6-sol` with low reasoning effort; this provenance is launcher-supplied because the provider exposed no independent resolved-model/effort metadata to this turn.

The daemon can admit follow-up prompts into the same session FIFO while a turn is running without canceling it, and can start a turn when idle. This is queued delivery at the next turn, not immediate injection into current token generation or a running tool. `verification: []` keeps activation blocked.

## Session discovery

Ordinary Qwen history/session IDs identify stored conversations, not which live process owns one. PID reuse, helper processes, multiple histories, and one-shot lifetime prevent exact liveness/state claims. Sniff can narrow candidates using OS-user scope, executable, CWD, and process-start identity, but a provider session ID must be captured rather than guessed.

A managed daemon is stronger: Claudine can register endpoint, workspace, daemon start identity, client ID, and provider `sessionId`. `GET /session/:id/status` returns live existence, client count, and `hasActivePrompt`; 404 marks that live handle stale. One shared ACP child may host multiple sessions, so PID is never the destination.

## Non-interrupting delivery

`POST /session/:id/prompt` admits prompts into a bounded per-session FIFO. A successful nonblocking request returns `202` with `promptId` and `lastEventId`; terminal `turn_complete` or `turn_error` arrives on SSE with that `promptId`. When work is active, the new prompt waits for the next turn without canceling generation or the current tool. Thus it can preserve a token-generation loop but cannot influence or break a loop that never finishes; automatic loop warnings may queue, never interrupt.

No equivalent is established for ordinary stdin, hooks, extensions, or terminal keystrokes. Hooks observe lifecycle and extensions customize launches; neither documentation nor passive inspection established an external sender path into an already-open ordinary conversation. Keystroke injection would be fragile UI automation, not provider messaging.

## Interruption fallback

The daemon exposes session cancellation, but the preferred fallback is still FIFO follow-up. Manual interruption is a separate, non-atomic sequence: obtain explicit choice, cancel, observe the old correlated terminal event and side effects, then submit the replacement and observe its prompt ID. A cancel response does not prove a long-running subprocess stopped. If replacement submission fails, the original work may already be gone and the message is not delivered; existing queued-prompt disposition remains unverified. Automatic warnings must never take this path.

## Idle sessions

For a managed session, `hasActivePrompt: false` supports an idle check and a prompt starts a turn in that same session. The check and admission are not atomic; a race may turn an intended idle start into a queued follow-up. An ordinary idle TUI accepts its user's keyboard, but no independent sender protocol is documented. Resuming history in another process is not steering the original process.

## Protocol details

The daemon uses JSON HTTP requests and SSE envelopes. Loopback is auth-free by default, but a safe Claudine profile should require a bearer token even on loopback and bind the exact workspace. Client IDs aid attachment/correlation but are self-declared rather than authenticated identity. The daemon-assigned prompt ID correlates later completion; there is no caller-supplied idempotency key, so an ambiguous request must never be retried.

The default unsettled-prompt cap is five and overflow is synchronous `503 prompt_queue_full` with `Retry-After: 5`. JSON bodies are limited to 10 MiB. SSE replay and subscriber queues are bounded; daemon restart loses live queues, while stored sessions may be loaded/resumed separately. Permission holds happen after delivery and are distinct from queued delivery. Text supports provider-defined interpretation; daemon mode lacks several terminal-only slash-command/template UI flows, so transport must not claim perfect ordinary-TUI equivalence.

## OS/version compatibility

Only macOS and installed 0.19.8 were inspected passively. Official TypeScript and optional native dependencies indicate cross-platform intent, not verified Linux/native-Windows behavior. WSL would be Linux-side evidence only. The daemon began experimentally in v0.16-alpha, but no stable compatibility bounds exist. Claudine should combine Sniff version discovery with authenticated `/capabilities`, workspace matching, required feature tags, and a reviewed exact-version allowlist; unknowns fail closed.

## Disposable-test proposals

For each exact version on native macOS, Linux, and Windows, launch an isolated authenticated loopback daemon and create a nonce-bearing session. Verify idle prompt admission, SSE completion, remembered nonce, exact session identity, and state transitions. During generation, a long subprocess, and a multi-tool batch, enqueue distinct prompts and prove FIFO order, no cancellation, tool completion, and same-conversation memory.

Fault tests should cover queue overflow, stale session/client IDs, malformed bodies, deadlines, rate limits, disconnect before 202, disconnect after 202, SSE replay gaps/eviction, daemon and ACP-child crashes, duplicate submissions, cancellation while active/idle, queued-message retention, and failure between cancel and replacement. Compare extensions, skills, context files, explicit settings, MCP, hooks, and prompt/template behavior to an ordinary launch. These are proposals only.

## Claudine integration

Define a managed profile that starts `qwen serve --hostname 127.0.0.1 --require-auth --token … --workspace … --no-web`, preserves the user's explicit provider settings and customization directories, registers process-start identity and endpoint without logging the token, negotiates capabilities, creates/attaches sessions, and retains one correlated SSE consumer. It can only help future managed sessions, never already-open ordinary ones.

Preferred delivery is FIFO follow-up for working sessions and prompt start for idle sessions. UI should explain that follow-up is not immediate injection. Interruption remains opt-in and manual. Claudine should not install extensions or alter Qwen configuration during delivery, and should refuse unsupported versions/capabilities rather than silently fall back to terminal injection or a new conversation.

## Gaps

The principal blockers are empty live verification, no safe retry/idempotency contract, incomplete cancellation/tool semantics, unverified preservation parity, no exact ordinary-session discovery, and no native Linux/Windows results. Read-only capability checks establish interface shape but cannot prove delivery. A `202` establishes admission only; confirmed same-conversation delivery requires later correlated evidence and a nonce-based disposable test.

## Sources

- [Qwen Code overview](https://qwenlm.github.io/qwen-code-docs/en/users/overview)
- [Qwen Code daemon mode](https://qwenlm.github.io/qwen-code-docs/en/users/qwen-serve)
- [Qwen Code v0.19.8 source](https://github.com/QwenLM/qwen-code/tree/v0.19.8)

## Changelog

- 2026-09-08: Created the Qwen Code schema-revision-2 steering report from official documentation, v0.19.8 source distribution, and passive macOS inspection.


## Revision 3 Contract Backfill

Receipt timing, interface inventory, and case-specific discovery gaps were added
from the existing evidence on 2026-09-08. No new provider observation or live test
was performed. Unexamined profile combinations remain unknown, not unsupported.
The original fleet model/effort provenance above describes the research run;
this deterministic contract migration is a separate coordinator edit.
