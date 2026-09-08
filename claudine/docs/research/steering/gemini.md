---
"$schema": "./_schema.yaml"
schema_revision: 2
provider: gemini
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
  - Gemini CLI 0.51.0 (installed npm bundle and bundled documentation on macOS)
  - Gemini CLI official documentation and google-gemini/gemini-cli v0.51.0 source
launch_profiles:
  - id: ordinary-cli
    description: Ordinary Gemini interactive TUI or one-shot headless launch, with no deliberately retained peer-control protocol.
    endpoint_scope: none
    lifetime: unknown
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [interactive, non_interactive]
    applicable_origins: [native, claudine]
    baseline: true
    startup_requirements: [ordinary gemini launch]
    preserves_extensions: 'yes'
    preserves_skills: 'yes'
    preserves_templates: 'yes'
    preserves_context: 'yes'
    evidence_ids: [official-cli, official-sessions, local-help, wrapper-source]
  - id: managed-acp
    description: Future Claudine-managed Gemini process launched explicitly in ACP mode and retained as a private JSON-RPC stdio child.
    endpoint_scope: retained_stdio
    lifetime: while_client_open
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [claudine]
    baseline: false
    startup_requirements:
      - launch gemini --acp as a retained child before creating or loading the target ACP session
      - retain and exclusively own stdin and stdout
      - complete ACP initialize and authentication as required
      - register the ACP process and returned session ID with Claudine
    preserves_extensions: unknown
    preserves_skills: unknown
    preserves_templates: unknown
    preserves_context: unknown
    evidence_ids: [official-acp, source-acp-051, local-help]
access_findings:
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp, os: macos, status: setup_required, prerequisite: "Claudine must have launched and retained gemini --acp, initialized it, and registered the exact ACP session ID.", applies_to_existing_sessions: 'no', evidence_ids: [official-acp, source-acp-051] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp, os: linux, status: setup_required, prerequisite: "Claudine must launch and retain gemini --acp and register the ACP session; Linux runtime behavior remains untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-acp, source-acp-051] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp, os: windows, status: setup_required, prerequisite: "Claudine must launch and retain gemini --acp using binary-safe stdio and register the ACP session; native Windows remains untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-acp, source-acp-051] }
  - { mechanism_id: acp-cancel-then-prompt, profile_id: managed-acp, os: macos, status: setup_required, prerequisite: "The retained ACP child and exact session ID are required; a human must approve interruption before cancel is sent.", applies_to_existing_sessions: 'no', evidence_ids: [official-acp, source-acp-051] }
  - { mechanism_id: acp-cancel-then-prompt, profile_id: managed-acp, os: linux, status: setup_required, prerequisite: "The retained ACP child and exact session ID are required; Linux interruption behavior remains untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-acp, source-acp-051] }
  - { mechanism_id: acp-cancel-then-prompt, profile_id: managed-acp, os: windows, status: setup_required, prerequisite: "The retained ACP child and exact session ID are required; native Windows interruption behavior remains untested.", applies_to_existing_sessions: 'no', evidence_ids: [official-acp, source-acp-051] }
delivery_states:
  - mechanism_id: acp-idle-prompt
    states: [accepted, delivered, refused, unknown]
    observable_by_external_sender: partial
    correlation: JSON-RPC request ID correlates the terminal PromptResponse; session/update notifications carry sessionId, but the protocol has no sender message ID in the examined request.
    evidence_ids: [official-acp, source-acp-051]
  - mechanism_id: acp-cancel-then-prompt
    states: [accepted, delivered, refused, unknown]
    observable_by_external_sender: partial
    correlation: session/cancel is a notification without a response; the cancelled PromptResponse and later prompt request ID must be correlated separately by session ID and client state.
    evidence_ids: [source-acp-051]
receipt_guarantees:
  - mechanism_id: acp-idle-prompt
    request_acceptance: confirmed
    persistence: unknown
    scheduling: confirmed
    conversation_delivery: confirmed
    provider_signals: [session/update agent_message_chunk and tool_call updates, PromptResponse stopReason]
    correlation: request_id
    evidence_ids: [official-acp, source-acp-051]
    limitations: The response arrives after the prompt turn, not as an early acceptance acknowledgment; no live test establishes crash persistence, duplicate behavior, or correctness in 0.51.0.
  - mechanism_id: acp-cancel-then-prompt
    request_acceptance: unknown
    persistence: unknown
    scheduling: unknown
    conversation_delivery: unknown
    provider_signals: [the original session/prompt may later return stopReason cancelled, the replacement prompt has its own PromptResponse]
    correlation: unknown
    evidence_ids: [source-acp-051]
    limitations: Cancel has no acknowledgment response. Claudine must observe the old prompt's cancelled result before submitting; even then submission can fail, and no sender message ID makes ambiguous retries unsafe.
evidence:
  - id: official-acp
    method: official_docs
    location: https://geminicli.com/docs/cli/acp-mode/
    version: current documentation observed 2026-09-08
    observed_on: 2026-09-08
    claim: Gemini ACP mode is an explicit gemini --acp launch using JSON-RPC over stdio and documents initialize, authentication, new/load session, prompt, cancel, session mode, model selection, updates, and client-proxied filesystem/MCP capabilities.
    limitations: Documentation does not establish concurrent prompt semantics, live delivery, retry safety, or native behavior on all three operating systems.
  - id: official-cli
    method: official_docs
    location: https://geminicli.com/docs/cli/cli-reference/
    version: current documentation observed 2026-09-08
    observed_on: 2026-09-08
    claim: Ordinary Gemini launches are interactive by default; -p is one-shot headless input, -i starts interactively, and --resume creates a CLI continuation from stored history.
    limitations: The ordinary CLI documentation exposes no peer message endpoint for an already open process.
  - id: official-sessions
    method: official_docs
    location: https://geminicli.com/docs/cli/session-management/
    version: current documentation observed 2026-09-08
    observed_on: 2026-09-08
    claim: Project-scoped transcripts are stored below ~/.gemini/tmp/<project_hash>/chats; --list-sessions and --resume identify historical sessions, and complete history is intended to survive interruption.
    limitations: History files do not establish process liveness, ownership of a currently open conversation, working versus idle state, or a writable delivery channel.
  - id: source-acp-051
    method: source_code
    location: https://github.com/google-gemini/gemini-cli/blob/v0.51.0/packages/cli/src/acp/acpClient.ts
    version: v0.51.0
    observed_on: 2026-09-08
    claim: The release implementation dispatches session/prompt requests and session/cancel notifications by sessionId; each prompt aborts any existing pending prompt, streams session/update notifications, executes complete tool-call batches serially, and returns a terminal stopReason including cancelled.
    limitations: The installed distribution was inspected as a generated bundle, and the linked source path is the readable release source. Passive source review is not a disposable delivery test.
  - id: local-help
    method: local_inspection
    location: sanitized passive output of Sniff software-agent discovery plus gemini --version and gemini --help on the research host
    version: 0.51.0
    observed_on: 2026-09-08
    claim: Sniff found Gemini CLI 0.51.0 on macOS; help advertises --acp, interactive and headless modes, session ID/list/resume flags, extensions, skills, hooks, and stream-json output.
    limitations: No Gemini session was launched, altered, messaged, or focused; no Linux or native Windows host was inspected.
  - id: wrapper-source
    method: source_code
    location: claudine/cli/src/commands/wrap/profile/gemini.rs
    version: workspace state observed 2026-09-08
    observed_on: 2026-09-08
    claim: Claudine's current ordinary Gemini wrapper does not create or register a retained ACP peer-control connection.
    limitations: Claudine implementation is not evidence of the provider's intrinsic capability.
  - id: ordinary-no-endpoint
    method: inference
    location: Comparison of official ordinary CLI, session-management, and ACP-mode documentation observed 2026-09-08
    version: Gemini CLI documentation current on 2026-09-08
    observed_on: 2026-09-08
    claim: Stored resume identity and writable terminal input should not be treated as a supported peer messaging protocol; ACP requires a distinct startup mode and retained stdio owner.
    limitations: Absence of a documented endpoint does not prove impossibility, so ordinary cases remain unknown rather than unsupported.
discovery:
  - { id: ordinary-macos-native, profile_id: ordinary-cli, os: macos, origin: native, method: process_inspection, locator: "Sniff process discovery plus project-scoped Gemini chat metadata", identity_check: "Correlate executable ownership, launch time, CWD/project hash, and transcript session UUID; PID alone is insufficient and reusable.", liveness_check: "Sniff process observation can show a candidate process exists but not that a particular transcript is attached.", state_detection: "No documented passive working-versus-idle indicator was found.", observation_source: "sniff and passive ~/.gemini metadata", observed_at: runtime, available_labels: [pid, executable, cwd, session_uuid, project_hash], prerequisites: [same OS user, readable process and Gemini metadata], evidence_ids: [official-sessions, local-help] }
  - { id: ordinary-macos-claudine, profile_id: ordinary-cli, os: macos, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff process observation and Gemini session metadata", identity_check: "Launch record can bind child PID and CWD, but must capture the provider session UUID to avoid guessing from newest history.", liveness_check: "Child/process observation; PID reuse must be rejected with launch identity.", state_detection: "Current wrapper has no provider-backed working/idle signal.", observation_source: "Claudine registry, sniff, and passive Gemini metadata", observed_at: runtime, available_labels: [pid, cwd, origin, session_uuid], prerequisites: [Claudine launch registration], evidence_ids: [official-sessions, wrapper-source] }
  - { id: ordinary-linux-native, profile_id: ordinary-cli, os: linux, origin: native, method: process_inspection, locator: "Sniff Linux process discovery plus project-scoped Gemini chat metadata", identity_check: "Require executable/CWD/start identity and transcript UUID correlation; WSL would be Linux-side only.", liveness_check: "Candidate process existence only; transcript attachment remains unresolved.", state_detection: "Unknown without a provider state signal.", observation_source: "sniff and passive Gemini metadata", observed_at: runtime, available_labels: [pid, executable, cwd, session_uuid, project_hash], prerequisites: [same Linux user], evidence_ids: [official-sessions] }
  - { id: ordinary-linux-claudine, profile_id: ordinary-cli, os: linux, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff Linux process observation", identity_check: "Capture provider session UUID rather than infer it from PID/history order.", liveness_check: "Validate process identity against launch record; do not trust PID alone.", state_detection: "Unknown for ordinary wrapper sessions.", observation_source: "Claudine registry, sniff, and Gemini metadata", observed_at: runtime, available_labels: [pid, cwd, origin, session_uuid], prerequisites: [Claudine launch registration], evidence_ids: [official-sessions, wrapper-source] }
  - { id: ordinary-windows-native, profile_id: ordinary-cli, os: windows, origin: native, method: process_inspection, locator: "Sniff native Windows process discovery plus project-scoped Gemini chat metadata", identity_check: "Require process-token user scope, executable/start identity, CWD/project hash, and transcript UUID correlation.", liveness_check: "Candidate process existence only; native Windows behavior is untested.", state_detection: "Unknown.", observation_source: "sniff and passive Gemini metadata", observed_at: runtime, available_labels: [pid, executable, cwd, session_uuid, project_hash], prerequisites: [same Windows process-token user], evidence_ids: [official-sessions] }
  - { id: ordinary-windows-claudine, profile_id: ordinary-cli, os: windows, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff native Windows process observation", identity_check: "Bind process start identity and provider session UUID; reject PID-only matches.", liveness_check: "Validate registered child identity; native Windows remains untested.", state_detection: "Unknown for ordinary wrapper sessions.", observation_source: "Claudine registry, sniff, and Gemini metadata", observed_at: runtime, available_labels: [pid, cwd, origin, session_uuid], prerequisites: [Claudine launch registration], evidence_ids: [official-sessions, wrapper-source] }
  - { id: acp-macos-claudine, profile_id: managed-acp, os: macos, origin: claudine, method: claudine_registration, locator: "Registered retained ACP child connection and ACP sessionId", identity_check: "The registry must bind child start identity, private stdio ownership, project CWD, and exact sessionId returned by session/new or supplied to session/load.", liveness_check: "Open child pipes plus process identity and successful protocol health/initialize state; no ACP ping was documented.", state_detection: "Claudine tracks an outstanding session/prompt request as working and a completed response with retained process as idle.", observation_source: "Claudine managed-launch registry and ACP request ledger", observed_at: runtime, available_labels: [pid, cwd, session_id, request_id, working, idle], prerequisites: [future managed ACP launcher], evidence_ids: [official-acp, source-acp-051] }
  - { id: acp-linux-claudine, profile_id: managed-acp, os: linux, origin: claudine, method: claudine_registration, locator: "Registered retained ACP child connection and ACP sessionId", identity_check: "Bind process start identity, stdio ownership, CWD, and exact ACP sessionId.", liveness_check: "Open pipes and process identity; runtime behavior requires Linux verification.", state_detection: "Outstanding prompt request versus terminal response in Claudine's ledger.", observation_source: "Claudine managed-launch registry and ACP request ledger", observed_at: runtime, available_labels: [pid, cwd, session_id, request_id, working, idle], prerequisites: [future managed ACP launcher], evidence_ids: [official-acp, source-acp-051] }
  - { id: acp-windows-claudine, profile_id: managed-acp, os: windows, origin: claudine, method: claudine_registration, locator: "Registered retained ACP child connection and ACP sessionId", identity_check: "Bind native process start identity, binary-safe stdio ownership, CWD, and exact ACP sessionId.", liveness_check: "Open pipes and process identity; native Windows requires verification.", state_detection: "Outstanding prompt request versus terminal response in Claudine's ledger.", observation_source: "Claudine managed-launch registry and ACP request ledger", observed_at: runtime, available_labels: [pid, cwd, session_id, request_id, working, idle], prerequisites: [future managed ACP launcher], evidence_ids: [official-acp, source-acp-051] }
mechanisms:
  - id: acp-idle-prompt
    interface_status: documented
    maturity: experimental
    transport: stdio
    initialization: "Launch gemini --acp; exchange newline-delimited JSON-RPC initialize/authenticate as needed; create or load a session; retain the process and sessionId."
    operation_intent: start_idle_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: idle_turn_start
    destination: Exact ACP sessionId held by the retained Gemini ACP process.
    authentication: Provider authentication configured at launch or ACP authenticate; stdio access is protected by exclusive child-process ownership, not an application-layer peer credential.
    startup_requirements: [gemini --acp, retained stdin/stdout, initialized ACP client, known idle sessionId]
    target_preconditions: [ACP process alive, exact sessionId exists in that process, no pending prompt]
    target_guards: [sessionId lookup rejects an absent session, Claudine must locally require idle state because the request has no expected operation ID]
    request_framing: "One newline-delimited JSON-RPC 2.0 session/prompt request with params.sessionId and params.prompt content blocks."
    response_framing: "Interleaved session/update notifications followed by the JSON-RPC response containing stopReason and optional metadata."
    request_format: "JSON-RPC 2.0 request; prompt is an ACP content-block array."
    response_format: "ACP session/update notifications and PromptResponse stopReason."
    long_tool_behavior: A prompt remains outstanding through model/tool iterations. No separate steering queue exists; sending another prompt would abort the pending one.
    tool_batch_behavior: continue_all
    queue_behavior: No provider queue was found. The implementation processes one prompt per Session pendingPrompt slot; a new prompt aborts the previous prompt.
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: not applicable to an idle target
    ordering: Serialized by Claudine per session; provider-side concurrent ordering is unsafe because a new prompt aborts the pending prompt.
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: No duplicate-suppression key was found in the examined ACP prompt request.
    cancellation: session/cancel notification targets sessionId and aborts the pending prompt.
    limits: ACP/schema and provider/model size limits apply; no Gemini-specific message-size limit was found.
    evidence_ids: [official-acp, source-acp-051]
  - id: acp-cancel-then-prompt
    interface_status: documented
    maturity: experimental
    transport: stdio
    initialization: "Use an initialized, retained gemini --acp process and registered session with one outstanding prompt."
    operation_intent: interrupt_then_submit
    conversation_effect: cancel_turn_same_conversation
    delivery_boundary: next_turn
    destination: Exact ACP sessionId; neither cancel nor prompt carries an expected active-operation ID.
    authentication: Same retained authenticated ACP connection and exclusive stdio ownership.
    startup_requirements: [managed ACP launch, exact sessionId, outstanding prompt ledger, explicit interactive user approval]
    target_preconditions: [session exists, prompt is working, automatic warnings must not use this mechanism]
    target_guards: [sessionId lookup rejects absent session, no expected prompt/operation guard exists, Claudine must compare its own outstanding request identity]
    request_framing: "First send session/cancel as a JSON-RPC notification with sessionId; wait for the outstanding prompt response with stopReason cancelled; then send a distinct session/prompt request."
    response_framing: "Cancel itself has no response. Final updates may precede the cancelled PromptResponse. Replacement prompt has independent updates and response."
    request_format: "Two protocol operations: cancel notification, then prompt request with ACP content blocks."
    response_format: "Old PromptResponse cancelled, then replacement session/update stream and PromptResponse."
    long_tool_behavior: AbortSignal is propagated through the prompt loop and tool runner, but exact subprocess/tool termination semantics require live testing.
    tool_batch_behavior: provider_defined
    queue_behavior: There is no durable follow-up queue. Submission occurs only after interruption; if it fails, the old turn remains cancelled and the new instruction is not known delivered.
    message_interpretation: provider_defined
    interruption_phases: [obtain explicit user approval, verify registered working request, send cancel notification, observe old PromptResponse stopReason cancelled, submit replacement prompt, observe replacement completion]
    interruption_partial_failure: If cancellation succeeds but the later prompt write/request fails, work is stopped while the steering message is absent; no provider queue retains it, so surface the partial failure and do not retry ambiguously.
    ordering: Client-enforced sequence; do not exploit source behavior where session/prompt implicitly aborts pending work because that removes the observable cancellation gate.
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: No duplicate suppression is documented; a repeated prompt may create repeated work.
    cancellation: session/cancel is notification-only and a matching cancelled PromptResponse is the later observable signal.
    limits: No operation ID guard, cancel acknowledgment, durable queue, or Gemini-specific payload-size limit was found.
    evidence_ids: [official-acp, source-acp-051]
compatibility:
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp, os: macos, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Use Sniff to resolve gemini and version; require help to advertise --acp; inspect a version-matched capability/schema or conservative allowlist without launching.", success_criteria: "Known reviewed version with ACP support and exact required methods.", failure_behavior: "Block delivery and explain that a managed ACP launch/version is unsupported or unreviewed.", evidence_ids: [local-help, official-acp, source-acp-051] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp, os: linux, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Resolve with Sniff; require --acp in help and a reviewed version before launch.", success_criteria: "Reviewed Gemini version and Linux runtime test exist.", failure_behavior: "Block; do not infer Linux from macOS or WSL from native Windows.", evidence_ids: [official-acp, source-acp-051] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp, os: windows, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Resolve with Sniff on native Windows; require --acp and reviewed protocol version.", success_criteria: "Reviewed version plus native Windows stdio/runtime test.", failure_behavior: "Block.", evidence_ids: [official-acp, source-acp-051] }
  - { mechanism_id: acp-cancel-then-prompt, profile_id: managed-acp, os: macos, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Apply the idle-prompt check and additionally require version-matched source evidence for cancel and pending-prompt abort behavior.", success_criteria: "Reviewed version and disposable cancellation/tool test pass.", failure_behavior: "Disable interruption fallback.", evidence_ids: [local-help, source-acp-051] }
  - { mechanism_id: acp-cancel-then-prompt, profile_id: managed-acp, os: linux, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Require reviewed --acp version and Linux disposable cancellation test.", success_criteria: "Cancellation, tool termination, context preservation, and replacement prompt pass.", failure_behavior: "Disable interruption fallback.", evidence_ids: [source-acp-051] }
  - { mechanism_id: acp-cancel-then-prompt, profile_id: managed-acp, os: windows, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Require reviewed --acp version and native Windows disposable cancellation test.", success_criteria: "Cancellation, stdio framing, tool termination, context preservation, and replacement prompt pass.", failure_behavior: "Disable interruption fallback.", evidence_ids: [source-acp-051] }
verification: []
cases:
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "No documented peer-control endpoint; terminal keystroke injection is not a stable protocol." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "An idle TUI accepts its user, but no independent-client delivery interface is documented." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Current wrapping does not expose ACP or another peer channel." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Launch provenance improves discovery but does not create delivery." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "Headless stdin supplies initial input and is not documented as a retained active-turn channel." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "Ordinary headless execution is one-shot; an idle-but-open control lifetime is not established." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Wrapper stdin is not a provider prompt RPC." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "No retained idle process or peer endpoint is established." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "No documented peer endpoint; Linux is untested." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "No independent-client idle delivery interface is documented." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Claudine origin does not expose a peer channel." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Discovery registration alone is not delivery." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "Initial stdin is not documented as retained steering." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "One-shot lifetime has no established idle receiver." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "No prompt RPC is exposed." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "No retained idle receiver is established." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "No documented peer endpoint; native Windows is untested." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "No independent-client idle delivery interface is documented." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Claudine origin does not expose a peer channel." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Discovery registration alone is not delivery." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "Initial stdin is not documented as retained steering." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [official-cli, ordinary-no-endpoint], reason: "One-shot lifetime has no established idle receiver." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "No prompt RPC is exposed." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "No retained idle receiver is established." }
  - { profile_id: managed-acp, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: interruption_required, discovery_ids: [acp-macos-claudine], mechanism_ids: [acp-cancel-then-prompt], prerequisites: [managed ACP launch, exact session ID, explicit human approval, disposable live test], evidence_ids: [official-acp, source-acp-051], reason: "ACP has no non-interrupting steering operation; a new prompt aborts pending work, so use an explicit cancel-observe-submit sequence only after approval." }
  - { profile_id: managed-acp, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [acp-macos-claudine], mechanism_ids: [acp-idle-prompt], prerequisites: [managed ACP launch, exact idle session ID, disposable live test], evidence_ids: [official-acp, source-acp-051], reason: "session/prompt starts a turn in the same retained ACP session when no prompt is pending." }
  - { profile_id: managed-acp, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: interruption_required, discovery_ids: [acp-linux-claudine], mechanism_ids: [acp-cancel-then-prompt], prerequisites: [managed ACP launch, exact session ID, explicit human approval, Linux disposable live test], evidence_ids: [official-acp, source-acp-051], reason: "Source supports interrupt-then-submit, but Linux runtime remains unverified." }
  - { profile_id: managed-acp, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [acp-linux-claudine], mechanism_ids: [acp-idle-prompt], prerequisites: [managed ACP launch, exact idle session ID, Linux disposable live test], evidence_ids: [official-acp, source-acp-051], reason: "Documented/source-derived capability exists, with activation blocked pending Linux testing." }
  - { profile_id: managed-acp, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: interruption_required, discovery_ids: [acp-windows-claudine], mechanism_ids: [acp-cancel-then-prompt], prerequisites: [managed ACP launch, exact session ID, explicit human approval, native Windows disposable live test], evidence_ids: [official-acp, source-acp-051], reason: "Protocol capability is source-derived; native Windows runtime remains unverified." }
  - { profile_id: managed-acp, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [acp-windows-claudine], mechanism_ids: [acp-idle-prompt], prerequisites: [managed ACP launch, exact idle session ID, native Windows disposable live test], evidence_ids: [official-acp, source-acp-051], reason: "Documented/source-derived capability exists, with activation blocked pending native Windows testing." }
gaps:
  - area: mandatory activation testing
    detail: verification is empty; no mechanism is eligible for Claudine activation.
    next_check: Run isolated ACP sessions for each claimed OS/version/profile/state after coordinator approval, never against existing sessions.
  - area: active-turn non-interrupting steering
    detail: ACP 0.51.0 has one pending prompt slot and a new prompt aborts it; no queue or steer-active-turn RPC was found. It cannot help a token-generation loop without interruption.
    next_check: Watch release notes/source for an operation guarded by current prompt ID that queues or injects without aborting.
  - area: acceptance acknowledgment and idempotency
    detail: PromptResponse is terminal, cancel is notification-only, and no sender message ID or duplicate suppression was found.
    next_check: Disposable tests must characterize write failure, malformed/unknown session errors, response timeout, disconnect, duplicate prompt, and cancellation races.
  - area: tools and cancellation
    detail: AbortSignal propagation is source-derived, but whether long-running subprocesses stop promptly and whether remaining tool calls execute is not proven.
    next_check: Test generation, a long-running tool, and a multi-tool batch; record filesystem/process outcomes and final history.
  - area: session load correctness
    detail: Official issue reports indicate ACP load behavior has had conversation-restoration defects; 0.51.0 was not live-tested.
    next_check: Test new and loaded sessions separately and assert conversation identity from remembered nonce/history, not sessionId alone.
  - area: profile preservation
    detail: It is unknown whether ACP mode preserves ordinary extensions, discovered skills, prompt templates, GEMINI.md/context, hooks, explicit settings, and native tools without semantic changes; ACP can proxy filesystem/MCP through its client.
    next_check: Compare a disposable ordinary launch and managed ACP launch using the same explicit configuration and inventories.
  - area: discovery and liveness
    detail: Historical chat files identify stored sessions but not attachment, liveness, working state, or the process owning the conversation. Managed registration solves this only for future ACP launches.
    next_check: Define a registry lease/process-start identity and test crash, pipe closure, PID reuse, multiple ACP sessions per process, and stale history.
  - area: compatibility bounds
    detail: No documented version bounds or passive protocol-version negotiation sufficient for safe adapter selection were found.
    next_check: Maintain a conservative tested-version allowlist keyed by ACP capabilities and block unknown versions until reviewed.
changes:
  - Initial steering revision-2 report for Gemini CLI.
requires_claudine_update: true
reason: A future managed ACP launch/registry/adapter could support idle prompts and consented interrupt-then-submit, but ordinary sessions remain undiscoverable for exact live-conversation delivery and all activation is blocked by empty verification.
---

# Gemini CLI steering research

## Overview

Gemini CLI 0.51.0 exposes one credible control surface: its explicit ACP mode. ACP is a programmatic JSON-RPC server over retained stdio, not a socket discoverable on ordinary Gemini processes. The installed version and help were found passively with Sniff; no session was launched or contacted. Research execution metadata identifies this run as Codex `gpt-5.6-sol` with low reasoning effort. That provenance is launcher-supplied; no independent resolved-model metadata was exposed to this research turn.

The result is deliberately conservative. An idle managed ACP session can accept `session/prompt` as a candidate same-session turn start. A working ACP session cannot be steered non-interruptingly: source for 0.51.0 begins each prompt by aborting the session's existing pending prompt. Ordinary interactive and headless sessions have resumable history but no documented external delivery interface. Nothing is activated because `verification` is empty.

## Session discovery

Documented transcripts are project-scoped under `~/.gemini/tmp/<project_hash>/chats/`, and the CLI can list or resume their UUIDs. This is history discovery, not live-session discovery. A PID can be reused, helper processes can exist, and a history UUID does not show which process currently owns it or whether that conversation is working or idle. Passive native discovery therefore needs Sniff plus process start identity, CWD/project hash, and transcript correlation, and still has a documented gap.

A managed ACP profile can be exact because Claudine would own the child pipes and record the session ID returned by ACP. The record must also contain process start identity and outstanding JSON-RPC request state. One ACP process can hold more than one session, so PID is not the destination. Pipe/process liveness also does not prove a particular session is healthy; the examined protocol documents no ping.

## Non-interrupting delivery

No ordinary-session mechanism was established. Stdin in headless mode contributes initial prompt input; keeping or writing a terminal/pipe is not documented as a live prompt protocol. Terminal-keystroke injection, if attempted in a future experiment, must remain a separate unsafe UI automation fallback and cannot be labeled provider messaging.

ACP `session/prompt` is suitable only when the registered session is idle. It starts processing in that session and streams `session/update` messages until a terminal PromptResponse. It is not active-turn steering: the implementation aborts `pendingPrompt` before processing a new prompt. There is no provider follow-up queue. Consequently this interface cannot help a token-generation loop that never reaches a tool boundary unless the turn is interrupted.

## Interruption fallback

Manual fallback is a sequence, not an atomic steer. After explicit user approval, Claudine would verify its own outstanding request ledger, send `session/cancel`, wait for the old PromptResponse to report `cancelled`, and only then submit a new `session/prompt`. Automatic loop warnings must never invoke it.

The cancel operation is a JSON-RPC notification and has no direct acknowledgment. Final updates may arrive before cancellation settles. If cancel succeeds and submission fails, the old work is gone and the steering instruction is not delivered or durably queued. A new prompt also implicitly aborts pending work in source, but Claudine should not rely on that shortcut because it obscures the interruption result and worsens the race.

## Idle sessions

An idle retained ACP process can start a turn by `session/prompt`; that is the strongest candidate. An ordinary idle TUI can accept its human user's keyboard input, but there is no documented independent sender channel. An ordinary headless process normally has one-shot lifetime, so “idle but open” is not established. Resuming a stored session in a new process preserves history but is not steering the original live process.

## Protocol details

ACP framing is newline-delimited JSON-RPC 2.0 over stdio. Initialization precedes session creation/loading. Prompt requests identify `sessionId` and carry ACP content blocks; progress arrives as `session/update`, including agent chunks and tool-call state, and the response carries `stopReason`. Unknown sessions are rejected by lookup. There is no examined expected-prompt/operation guard, sender message ID, provider queue, duplicate suppression, or early acceptance receipt.

The implementation gathers a model response's tool calls and runs them serially before the next model iteration. Cancellation uses an AbortSignal through prompt and tool paths, but exact termination of a long-running external tool and disposition of a remaining tool batch are unverified. Text may be handled as provider commands when it begins with `/` or `$`, so it is not guaranteed literal; managed delivery needs policy for command-like text.

## OS and version compatibility

Only macOS with installed Gemini CLI 0.51.0 was inspected passively. Official TypeScript/source and stdio design are portable leads, not runtime proof for Linux or native Windows. WSL would count only as Linux-side evidence. Version support bounds are undocumented. A safe implementation needs Sniff-based binary/version discovery, a conservative reviewed-version allowlist, and disposable tests on macOS, Linux, and native Windows. Unknown versions must fail closed.

## Disposable-test proposals

For each OS and exact version, a harness should launch a fresh `gemini --acp` with isolated configuration and credentials suitable for testing. It should initialize, create a session, submit a nonce prompt, assert updates and terminal response, then submit a second nonce while idle and verify same-conversation memory. A separate loaded-session test must prove remembered history rather than trust the returned session ID.

Working-state tests should cover token generation, a long-running subprocess, and a multi-tool batch. They must verify that a second prompt is not non-interrupting; then explicitly cancel, observe `cancelled`, submit a replacement, and inspect process/filesystem effects. Fault tests should cover stale/unknown session IDs, cancel while idle, disconnects, partial JSON lines, timeouts, duplicate requests, and failure between cancellation and replacement submission. Tests must also compare extensions, skills, templates, hooks, MCP/native tools, explicit settings, and GEMINI.md context against an ordinary launch. These are proposals only; no test was run.

## Claudine integration

The preferred future design is a managed ACP launch profile with private, exclusively owned stdio, structured registration, serialized per-session requests, and state derived from the request ledger. It can support non-interactive receiving because Claudine is the ACP client; no receiving human must approve protocol traffic. Tool permission requests are a later agent-action state and must not be mislabeled as message delivery holds.

The verified fallback, once tests exist, is idle `session/prompt`. If an ACP session is working, the UI may offer the separately verified cancel-observe-submit flow only after explaining the partial-failure risk and receiving explicit consent. If ACP is unavailable or unverified, warn that Gemini ordinary sessions have no verified peer-delivery protocol and decline delivery. Claudine must not silently relaunch without extensions, skills, templates, context, hooks, MCP, or explicit settings.

## Gaps

The blocking gaps are the empty live-verification ledger, no acceptance acknowledgment for cancel, no idempotency key, no expected active-operation guard, unknown long-tool cancellation semantics, unverified session-load correctness, unknown ACP preservation of ordinary features, weak discovery for native sessions, and absent version bounds. Source and documentation evidence do not clear the mandatory live-test activation gate.

## Sources

Primary sources are Gemini CLI's official [ACP mode documentation](https://geminicli.com/docs/cli/acp-mode/), [CLI reference](https://geminicli.com/docs/cli/cli-reference/), [session-management documentation](https://geminicli.com/docs/cli/session-management/), and the [v0.51.0 ACP implementation](https://github.com/google-gemini/gemini-cli/blob/v0.51.0/packages/cli/src/acp/acpClient.ts). Local evidence was limited to sanitized passive Sniff/version/help and installed-bundle inspection. No credentials, session content, or live delivery observations were collected.

## Changelog

- 2026-09-08: Created the revision-2 Gemini steering report from official documentation, release source, and passive local inspection.
