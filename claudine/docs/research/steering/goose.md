---
$schema: ./_schema.yaml
schema_revision: 3
provider: goose
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined: ["Goose v1.49.0 source at tag commit 71fc4be1ed729e26b1dc0a4466abdd03be548a53 (release published 2026-09-03)", "No Goose binary installed on the inspected macOS host"]
launch_profiles:
  - id: ordinary-cli
    description: "Ordinary `goose session` or `goose run`; neither exposes a peer control endpoint."
    endpoint_scope: none
    lifetime: unknown
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [interactive, non_interactive]
    applicable_origins: [native, claudine]
    baseline: true
    startup_requirements: ["ordinary Goose launch"]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: yes
    preserves_context: yes
    evidence_ids: [official-cli, source-cli, local-host, claudine-wrapper]
  - id: managed-acp-server
    description: "A deliberately retained `goose serve` process exposing authenticated ACP over HTTP/WebSocket; this is a separate future managed profile, not an attachment to an ordinary CLI."
    endpoint_scope: externally_reachable
    lifetime: long_lived
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [native, claudine]
    baseline: false
    startup_requirements: ["launch `goose serve`", "set GOOSE_SERVER__SECRET_KEY", "retain endpoint and expected server identity", "initialize with Goose custom notifications", "create or load the target ACP session"]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: yes
    preserves_context: yes
    evidence_ids: [official-acp, source-cli, source-steer, source-sessions, source-auth]
access_findings:
  - { mechanism_id: acp-steer, profile_id: managed-acp-server, os: macos, status: setup_required, prerequisite: "Prelaunch authenticated server and retain session/run metadata; no local binary was available to verify.", applies_to_existing_sessions: no, evidence_ids: [source-steer, source-auth, local-host] }
  - { mechanism_id: acp-steer, profile_id: managed-acp-server, os: linux, status: setup_required, prerequisite: "Prelaunch authenticated server and retain session/run metadata; native Linux untested.", applies_to_existing_sessions: no, evidence_ids: [official-acp, source-steer, source-auth] }
  - { mechanism_id: acp-steer, profile_id: managed-acp-server, os: windows, status: setup_required, prerequisite: "Prelaunch authenticated server and retain session/run metadata; native Windows untested.", applies_to_existing_sessions: no, evidence_ids: [official-install, source-steer, source-auth] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp-server, os: macos, status: setup_required, prerequisite: "Managed ACP session known to be idle; no local binary was available to verify.", applies_to_existing_sessions: no, evidence_ids: [source-prompt, local-host] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp-server, os: linux, status: setup_required, prerequisite: "Managed ACP session known to be idle; native Linux untested.", applies_to_existing_sessions: no, evidence_ids: [source-prompt] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp-server, os: windows, status: setup_required, prerequisite: "Managed ACP session known to be idle; native Windows untested.", applies_to_existing_sessions: no, evidence_ids: [official-install, source-prompt] }
  - { mechanism_id: acp-cancel-submit, profile_id: managed-acp-server, os: macos, status: setup_required, prerequisite: "Managed ACP session, explicit interactive choice, and revalidation between cancel and prompt; untested.", applies_to_existing_sessions: no, evidence_ids: [source-cancel, source-prompt] }
  - { mechanism_id: acp-cancel-submit, profile_id: managed-acp-server, os: linux, status: setup_required, prerequisite: "Managed ACP session and explicit interactive choice; native Linux untested.", applies_to_existing_sessions: no, evidence_ids: [source-cancel, source-prompt] }
  - { mechanism_id: acp-cancel-submit, profile_id: managed-acp-server, os: windows, status: setup_required, prerequisite: "Managed ACP session and explicit interactive choice; native Windows untested.", applies_to_existing_sessions: no, evidence_ids: [official-install, source-cancel, source-prompt] }
delivery_states:
  - { mechanism_id: acp-steer, states: [accepted, queued, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "JSON-RPC request id correlates the response; response messageId correlates queuedSteer and the later steer-marked UserMessageChunk when custom notifications are enabled.", evidence_ids: [source-steer, source-steer-test] }
  - { mechanism_id: acp-idle-prompt, states: [accepted, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "JSON-RPC request id remains outstanding until PromptResponse; session/update notifications carry sessionId but are not all tied to that request id. Later tool-permission waits are execution state, not incoming-message holds.", evidence_ids: [source-prompt, source-permissions] }
  - { mechanism_id: acp-cancel-submit, states: [accepted, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "Cancel is a notification without a correlated result; the replacement prompt has its own request id and final PromptResponse.", evidence_ids: [source-cancel, source-prompt] }
receipt_guarantees:
  - mechanism_id: acp-steer
    request_acceptance: confirmed
    persistence: not_persisted
    scheduling: confirmed
    conversation_delivery: unknown
    provider_signals: ["SteerSessionResponse {runId,messageId}", "queuedSteer session-info update", "steer-marked user_message_chunk at pickup"]
    correlation: message_id
    evidence_ids: [source-steer, source-steer-test]
    limitations: "The initial response proves guarded in-memory enqueue, not pickup or model incorporation. Pickup is a later signal. Live delivery remains unverified."
  - mechanism_id: acp-idle-prompt
    request_acceptance: confirmed
    persistence: unknown
    scheduling: unknown
    conversation_delivery: confirmed
    provider_signals: [session/update, PromptResponse]
    correlation: request_id
    evidence_ids: [source-prompt]
    limitations: "The first successful response is the terminal PromptResponse, which confirms acceptance and completion. There is no early acceptance acknowledgment; until that response or another proven delivery signal, acceptance remains unconfirmed to the sender."
  - mechanism_id: acp-cancel-submit
    request_acceptance: unknown
    persistence: unknown
    scheduling: unknown
    conversation_delivery: unknown
    provider_signals: ["PromptResponse stopReason=cancelled for the old request", "separate PromptResponse for replacement"]
    correlation: unknown
    evidence_ids: [source-cancel, source-prompt]
    limitations: "The cancel notification has no response; submission is a separate operation and may fail after cancellation."
evidence:
  - { id: official-cli, method: official_docs, location: "https://block.github.io/goose/docs/guides/goose-cli-commands/", version: "documentation observed 2026-09-08", observed_on: 2026-09-08, claim: "Documents ordinary interactive session and non-interactive run entrypoints.", limitations: "Does not document peer steering for ordinary sessions." }
  - { id: official-acp, method: official_docs, location: "https://block.github.io/goose/docs/guides/acp-clients/", version: "documentation observed 2026-09-08", observed_on: 2026-09-08, claim: "Documents `goose acp`, `goose serve`, persisted ACP sessions, and secret-key protection for the server endpoint.", limitations: "Does not fully specify the custom steering extension or receipt semantics." }
  - { id: official-install, method: official_docs, location: "https://block.github.io/goose/docs/getting-started/installation/", version: "documentation observed 2026-09-08", observed_on: 2026-09-08, claim: "Documents native macOS, Linux, and Windows CLI installation and labels WSL separately.", limitations: "Installation support is not runtime steering verification." }
  - { id: official-release, method: official_docs, location: "https://github.com/aaif-goose/goose/releases/tag/v1.49.0", version: "v1.49.0", observed_on: 2026-09-08, claim: "v1.49.0 is the latest release found and was published 2026-09-03.", limitations: "Release recency does not establish compatibility." }
  - { id: source-cli, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose-cli/src/cli.rs", version: "v1.49.0", observed_on: 2026-09-08, claim: "Defines separate `acp` stdio and `serve` HTTP/WebSocket modes; ordinary session/run do not expose these endpoints.", limitations: "Source inspection is not a runtime test." }
  - { id: source-steer, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose/src/acp/server.rs#L1899-L2393", version: "v1.49.0", observed_on: 2026-09-08, claim: "Tracks one active run per session, publishes activeRunId, rejects absent/stale expectedRunId, queues steering, and returns runId/messageId.", limitations: "Undocumented custom API; no live test." }
  - { id: source-types, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose-sdk-types/src/custom_requests.rs#L208-L234", version: "v1.49.0", observed_on: 2026-09-08, claim: "Defines `_goose/unstable/session/steer` request and response framing.", limitations: "The unstable namespace has no documented compatibility range." }
  - { id: source-loop, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose/src/agents/state_machine/ops_steer.rs", version: "v1.49.0", observed_on: 2026-09-08, claim: "Drains all queued steer messages FIFO only between turns, after an assistant turn end or a tool message.", limitations: "Exact timing with every provider/tool was not live-tested." }
  - { id: source-steer-test, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose/tests/acp_custom_requests_test.rs#L535-L626", version: "v1.49.0", observed_on: 2026-09-08, claim: "Provider tests correlate response messageId with queuedSteer and the later steer-marked UserMessageChunk.", limitations: "An upstream test is not this report's mandatory disposable live test." }
  - { id: source-prompt, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose/src/acp/server.rs#L2244-L2351", version: "v1.49.0", observed_on: 2026-09-08, claim: "`session/prompt` creates an active run, streams the turn, persists session messages, and returns a final stop reason; concurrent prompt is rejected in favor of steer.", limitations: "No separate prompt-acceptance receipt and no live test." }
  - { id: source-cancel, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose/src/acp/server.rs#L2395-L2421", version: "v1.49.0", observed_on: 2026-09-08, claim: "`session/cancel` cancels the active run token but is a notification and does not submit replacement text.", limitations: "Tool subprocess termination details were not established." }
  - { id: source-sessions, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose/src/acp/server/list_sessions.rs#L175-L225", version: "v1.49.0", observed_on: 2026-09-08, claim: "ACP session/list pages persisted User, Scheduled, and ACP sessions with optional cwd/type/keyword filters.", limitations: "History listing does not prove liveness, ownership, or an active loaded conversation." }
  - { id: source-auth, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose-cli/src/cli.rs#L855-L897", version: "v1.49.0", observed_on: 2026-09-08, claim: "`goose serve` defaults to loopback and requires GOOSE_SERVER__SECRET_KEY unless explicitly started dangerously unauthenticated.", limitations: "Authentication handshake and transport were not live-tested." }
  - { id: source-permissions, method: source_code, location: "https://github.com/aaif-goose/goose/blob/71fc4be1ed729e26b1dc0a4466abdd03be548a53/crates/goose/src/acp/server.rs", version: "v1.49.0", observed_on: 2026-09-08, claim: "ACP may issue permission reverse requests; provider tool approval is distinct from message delivery.", limitations: "Headless policy combinations were not exercised." }
  - { id: local-host, method: local_inspection, location: "Sanitized Sniff agent inventory and shell lookup on macOS", version: "Goose absent", observed_on: 2026-09-08, claim: "Sniff reported Goose not installed and no executable path/version; no process or session was touched.", limitations: "Negative host inventory says nothing about other OSes or provider capability." }
  - { id: claudine-wrapper, method: source_code, location: "claudine/cli/src/commands/wrap/profile/goose.rs", version: "workspace 2026-09-08", observed_on: 2026-09-08, claim: "Current Claudine launches ordinary `goose run -t` and does not start or retain an ACP server.", limitations: "Claudine implementation is not provider capability." }
  - { id: provenance, method: local_inspection, location: "Fleet launcher contract supplied in this task", version: "unknown resolved execution metadata", observed_on: 2026-09-08, claim: "The launcher supplied agent=codex, model=gpt-5.6-sol, and low effort; no independent resolved-model metadata was exposed to this researcher.", limitations: "Launcher-supplied provenance is not an independently verified resolved model." }
discovery:
  - { id: ordinary-macos-native, profile_id: ordinary-cli, os: macos, origin: native, method: process_inspection, locator: "Sniff process/program facts plus persisted-history hints.", identity_check: "PID and history ID cannot prove which conversation is loaded; require process creation identity to mitigate PID reuse.", liveness_check: "Process existence only.", state_detection: "unknown", observation_source: "Sniff and source/docs", observed_at: "2026-09-08; Goose absent", available_labels: [provider, pid, command, cwd], prerequisites: [], evidence_ids: [local-host, source-sessions] }
  - { id: ordinary-macos-claudine, profile_id: ordinary-cli, os: macos, origin: claudine, method: claudine_registration, locator: "Wrapper child record.", identity_check: "Claudine child identity does not prove Goose session identity.", liveness_check: "Child/process handle only.", state_detection: "Output activity is insufficient to distinguish generation, tools, and idle.", observation_source: "wrapper source", observed_at: "2026-09-08", available_labels: [provider, pid, claudine_session_id], prerequisites: [], evidence_ids: [claudine-wrapper] }
  - { id: ordinary-linux-native, profile_id: ordinary-cli, os: linux, origin: native, method: process_inspection, locator: "Process/history hints.", identity_check: "Unknown; PID/history are insufficient.", liveness_check: "PID only.", state_detection: "unknown", observation_source: "cross-platform inference", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Linux inspection"], evidence_ids: [source-sessions] }
  - { id: ordinary-linux-claudine, profile_id: ordinary-cli, os: linux, origin: claudine, method: claudine_registration, locator: "Wrapper child record.", identity_check: "No provider session guard.", liveness_check: "Child handle only.", state_detection: "unknown", observation_source: "wrapper source", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Linux inspection"], evidence_ids: [claudine-wrapper] }
  - { id: ordinary-windows-native, profile_id: ordinary-cli, os: windows, origin: native, method: process_inspection, locator: "Native process/history hints; WSL is excluded.", identity_check: "PID plus creation time still does not prove loaded conversation.", liveness_check: "PID only.", state_detection: "unknown", observation_source: "official platform docs and source inference", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Windows inspection"], evidence_ids: [official-install, source-sessions] }
  - { id: ordinary-windows-claudine, profile_id: ordinary-cli, os: windows, origin: claudine, method: claudine_registration, locator: "Wrapper child record; WSL is excluded.", identity_check: "No provider session guard.", liveness_check: "Child handle plus creation identity.", state_detection: "unknown", observation_source: "wrapper source", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Windows inspection"], evidence_ids: [claudine-wrapper] }
  - { id: acp-macos-native, profile_id: managed-acp-server, os: macos, origin: native, method: provider_api, locator: "Registered server endpoint plus ACP session/list and sessionId.", identity_check: "Authenticate endpoint, reconcile registered server identity and exact sessionId, then use activeRunId as the working-run guard.", liveness_check: "Successful authenticated initialize/list round trip; list alone describes persisted history.", state_detection: "activeRunId update distinguishes a managed active run; absence is a snapshot and can race.", observation_source: "v1.49.0 source", observed_at: "source-derived; not runtime observed", available_labels: [session_id, cwd, title, updated_at, active_run_id], prerequisites: ["managed server registry", "secret", "custom notifications"], evidence_ids: [source-sessions, source-steer, source-auth] }
  - { id: acp-macos-claudine, profile_id: managed-acp-server, os: macos, origin: claudine, method: claudine_registration, locator: "Future Claudine launch registration plus ACP list/updates.", identity_check: "Reconcile registered endpoint/process creation identity, sessionId, and expected activeRunId.", liveness_check: "Authenticated ACP round trip plus managed process identity.", state_detection: "activeRunId and final PromptResponse.", observation_source: "source and implementation gap", observed_at: "2026-09-08 source-derived", available_labels: [provider, pid, endpoint, session_id, active_run_id], prerequisites: ["future managed profile"], evidence_ids: [source-steer, source-sessions, claudine-wrapper] }
  - { id: acp-linux-native, profile_id: managed-acp-server, os: linux, origin: native, method: provider_api, locator: "Registered endpoint plus ACP session/list.", identity_check: "Endpoint identity, exact sessionId, activeRunId.", liveness_check: "Authenticated ACP round trip.", state_detection: "activeRunId; race remains.", observation_source: "cross-platform source", observed_at: "not observed", available_labels: [session_id, cwd, active_run_id], prerequisites: ["native Linux check"], evidence_ids: [source-steer, source-sessions] }
  - { id: acp-linux-claudine, profile_id: managed-acp-server, os: linux, origin: claudine, method: claudine_registration, locator: "Future launch registration plus ACP.", identity_check: "Registration/sessionId/activeRunId.", liveness_check: "Process and authenticated ACP round trip.", state_detection: "activeRunId.", observation_source: "source-derived", observed_at: "not observed", available_labels: [pid, endpoint, session_id, active_run_id], prerequisites: ["future profile", "native Linux check"], evidence_ids: [source-steer, claudine-wrapper] }
  - { id: acp-windows-native, profile_id: managed-acp-server, os: windows, origin: native, method: provider_api, locator: "Registered TCP endpoint plus ACP session/list; not WSL evidence.", identity_check: "Endpoint identity, exact sessionId, activeRunId.", liveness_check: "Authenticated ACP round trip.", state_detection: "activeRunId.", observation_source: "source-derived", observed_at: "not observed", available_labels: [session_id, cwd, active_run_id], prerequisites: ["native Windows check"], evidence_ids: [official-install, source-steer, source-sessions] }
  - { id: acp-windows-claudine, profile_id: managed-acp-server, os: windows, origin: claudine, method: claudine_registration, locator: "Future Windows launch registration plus ACP.", identity_check: "Creation identity, endpoint, sessionId, activeRunId.", liveness_check: "Process and authenticated ACP round trip.", state_detection: "activeRunId.", observation_source: "source-derived", observed_at: "not observed", available_labels: [pid, endpoint, session_id, active_run_id], prerequisites: ["future profile", "native Windows check"], evidence_ids: [official-install, source-steer, claudine-wrapper] }
mechanisms:
  - id: acp-steer
    interface_status: undocumented
    maturity: experimental
    transport: http
    initialization: "Launch authenticated `goose serve`; initialize with `_meta.goose.customNotifications=true`; create/load session; observe non-null activeRunId. WebSocket is an alternate transport for the same server profile but was not independently framed here."
    operation_intent: steer_active_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: next_tool_boundary
    destination: "Exact ACP sessionId and its currently active run."
    authentication: "GOOSE_SERVER__SECRET_KEY presented as the server's supported secret-key credential; transport details require compatibility probing."
    startup_requirements: ["managed serve process", "authenticated connection", "custom notifications", "known sessionId and activeRunId"]
    target_preconditions: ["session has active run", "expectedRunId is current", "non-empty steerable prompt"]
    target_guards: ["empty expectedRunId rejected", "no active run rejected", "stale run rejected with expected/actual IDs", "concurrent ordinary prompt rejected"]
    request_framing: "ACP JSON-RPC request over the server's HTTP transport."
    response_framing: "Correlated JSON-RPC result; later session/update notifications."
    request_format: '{"jsonrpc":"2.0","id":"request-id","method":"_goose/unstable/session/steer","params":{"sessionId":"...","expectedRunId":"run_...","prompt":[{"type":"text","text":"..."}]}}'
    response_format: '{"jsonrpc":"2.0","id":"request-id","result":{"runId":"run_...","messageId":"steer_..."}}'
    long_tool_behavior: "Queued guidance is not injected into a running tool. It is eligible after a tool message boundary; exact behavior for a never-returning tool is no pickup."
    tool_batch_behavior: provider_defined
    queue_behavior: "Per-session in-memory FIFO; all pending messages drain together at the next between-turn boundary. Normal run cleanup discards leftovers; source tests show lower-level cancellation can preserve them for resume, but ACP cleanup may then discard them."
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: "Not applicable; steering does not cancel."
    ordering: "FIFO within the mutex-protected session queue; cross-client arrival ordering is scheduler-defined."
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: "No caller-supplied idempotency key or duplicate suppression; provider creates messageId after enqueue."
    cancellation: "No per-message queue removal API found. Canceling/disconnecting the run can discard pending ACP steers during cleanup."
    limits: "No documented message-size, queue-count, expiry, or persistence bounds; HTTP body limits and text transformations need a live/version probe."
    evidence_ids: [source-types, source-steer, source-loop, source-steer-test]
  - id: acp-idle-prompt
    interface_status: documented
    maturity: stable
    transport: http
    initialization: "Managed authenticated ACP server with an exact loaded or newly created idle session."
    operation_intent: start_idle_turn
    conversation_effect: resume_same_conversation
    delivery_boundary: idle_turn_start
    destination: "Exact ACP sessionId."
    authentication: "Authenticated server connection."
    startup_requirements: ["managed serve process", "known sessionId", "idle revalidation"]
    target_preconditions: ["session exists", "no activeRunId", "provider credentials and non-interactive tool policy are usable"]
    target_guards: ["one active run per session", "unknown/closed session errors", "idle observation can race before request"]
    request_framing: "ACP JSON-RPC session/prompt request."
    response_framing: "Streaming session/update notifications followed by correlated PromptResponse."
    request_format: '{"jsonrpc":"2.0","id":"request-id","method":"session/prompt","params":{"sessionId":"...","prompt":[{"type":"text","text":"..."}]}}'
    response_format: '{"jsonrpc":"2.0","id":"request-id","result":{"stopReason":"end_turn|cancelled|..."}}'
    long_tool_behavior: "The prompt request stays open while tools run; permission reverse requests can hold tool execution after message delivery."
    tool_batch_behavior: continue_all
    queue_behavior: "Starts immediately if idle; a racing active run causes rejection directing the client to steer."
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: "Not applicable."
    ordering: "One active prompt per session; concurrent prompt rejected."
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: "No idempotency contract; ambiguous failure may duplicate the user turn."
    cancellation: "session/cancel can cancel the resulting active run."
    limits: "No documented prompt/body bound found."
    evidence_ids: [official-acp, source-prompt, source-permissions]
  - id: acp-cancel-submit
    interface_status: documented
    maturity: stable
    transport: http
    initialization: "Managed session after an explicit interactive interruption choice."
    operation_intent: interrupt_then_submit
    conversation_effect: cancel_turn_same_conversation
    delivery_boundary: next_turn
    destination: "Cancel exact session's active run, wait for its original prompt response/update to settle, revalidate, then submit a separate session/prompt."
    authentication: "Authenticated server connection."
    startup_requirements: ["explicit user choice", "known sessionId", "activeRunId evidence", "ability to observe settlement"]
    target_preconditions: ["working managed session", "same session remains available after cancellation"]
    target_guards: ["cancel carries sessionId but no expectedRunId", "unknown session only logs warning", "replacement prompt has ordinary one-active-run guard"]
    request_framing: "ACP cancel notification, settlement wait, then separate JSON-RPC prompt request."
    response_framing: "No cancel response; old prompt eventually returns cancelled; replacement has its own response."
    request_format: "session/cancel(sessionId); await old PromptResponse/activeRunId null; revalidate; session/prompt(sessionId,text)"
    response_format: "old PromptResponse stopReason=cancelled; independent replacement PromptResponse"
    long_tool_behavior: "Cancellation token stops the agent stream; termination semantics of an already-running native tool subprocess were not established."
    tool_batch_behavior: stop_remaining
    queue_behavior: "Do not rely on queued steers surviving: ACP run cleanup discards pending steers."
    message_interpretation: provider_defined
    interruption_phases: ["explain effect and obtain explicit choice", "identify session and active run", "send cancel", "await old turn settlement", "revalidate same session is idle", "submit replacement prompt", "observe replacement result"]
    interruption_partial_failure: "Cancellation can succeed and replacement submission can fail; the original work remains canceled, unpicked queued steers may be discarded, and there is no rollback."
    ordering: "Never pipeline cancel and replacement; wait and revalidate between them."
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: "No idempotency for replacement prompt."
    cancellation: "Destructive manual fallback only; automatic loop warnings must not use it."
    limits: "No established cancellation timeout or tool-child cleanup bound."
    evidence_ids: [source-cancel, source-prompt, source-steer]
compatibility:
  - { mechanism_id: acp-steer, profile_id: managed-acp-server, os: macos, versions_verified: [], documented_version_bounds: "unknown; custom method is unstable", read_only_check: "Require initialize agentInfo.version=1.49.0 (or an allowlisted tested version), customNotifications opt-in, observed activeRunId, and a non-mutating diagnostics/list round trip; method support cannot be proven read-only.", success_criteria: "Exact tested version plus live disposable steering fixture is registered.", failure_behavior: "Disable steering; do not probe by sending to a real session.", evidence_ids: [source-types, source-steer] }
  - { mechanism_id: acp-steer, profile_id: managed-acp-server, os: linux, versions_verified: [], documented_version_bounds: "unknown; custom method is unstable", read_only_check: "Same handshake/version checks; native Linux transport remains untested.", success_criteria: "Allowlisted exact version and matching Linux fixture.", failure_behavior: "Disable.", evidence_ids: [source-types, source-steer] }
  - { mechanism_id: acp-steer, profile_id: managed-acp-server, os: windows, versions_verified: [], documented_version_bounds: "unknown; custom method is unstable", read_only_check: "Same handshake/version checks on native Windows, not WSL.", success_criteria: "Allowlisted exact version and matching native Windows fixture.", failure_behavior: "Disable.", evidence_ids: [official-install, source-types, source-steer] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp-server, os: macos, versions_verified: [], documented_version_bounds: "ACP support documented; exact bounds unknown", read_only_check: "Initialize, verify load/list capabilities and exact agentInfo.version; no mutation-free idle-prompt support probe exists.", success_criteria: "Exact version has matching disposable idle fixture.", failure_behavior: "Do not submit.", evidence_ids: [official-acp, source-prompt] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp-server, os: linux, versions_verified: [], documented_version_bounds: "ACP support documented; exact bounds unknown", read_only_check: "Handshake and capabilities on native Linux.", success_criteria: "Matching Linux fixture.", failure_behavior: "Do not submit.", evidence_ids: [official-acp, source-prompt] }
  - { mechanism_id: acp-idle-prompt, profile_id: managed-acp-server, os: windows, versions_verified: [], documented_version_bounds: "ACP support documented; exact bounds unknown", read_only_check: "Handshake and capabilities on native Windows, not WSL.", success_criteria: "Matching native Windows fixture.", failure_behavior: "Do not submit.", evidence_ids: [official-install, source-prompt] }
  - { mechanism_id: acp-cancel-submit, profile_id: managed-acp-server, os: macos, versions_verified: [], documented_version_bounds: "ACP cancellation documented by schema; exact Goose bounds unknown", read_only_check: "Handshake/version only; cancellation behavior cannot be established read-only.", success_criteria: "Matching working-turn and long-tool disposable fixtures.", failure_behavior: "Disable interruption fallback.", evidence_ids: [source-cancel] }
  - { mechanism_id: acp-cancel-submit, profile_id: managed-acp-server, os: linux, versions_verified: [], documented_version_bounds: "unknown", read_only_check: "Handshake/version only on native Linux.", success_criteria: "Matching Linux fixtures.", failure_behavior: "Disable interruption fallback.", evidence_ids: [source-cancel] }
  - { mechanism_id: acp-cancel-submit, profile_id: managed-acp-server, os: windows, versions_verified: [], documented_version_bounds: "unknown", read_only_check: "Handshake/version only on native Windows, not WSL.", success_criteria: "Matching native Windows fixtures.", failure_behavior: "Disable interruption fallback.", evidence_ids: [official-install, source-cancel] }
verification: []
cases:
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [source-cli, local-host], reason: "No peer endpoint or safe same-conversation input contract was found; stdin/terminal keystrokes are not treated as messaging." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [source-cli, local-host], reason: "An open TUI may accept human input, but no independent protocol or acceptance receipt was established." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [claudine-wrapper], reason: "Current wrapper does not retain ACP or a writable provider prompt channel." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [claudine-wrapper], reason: "No independent idle delivery interface." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [source-cli], reason: "`goose run` is one-shot and no retained peer endpoint was found." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [source-cli], reason: "A completed one-shot has no live idle process; retained-stdio was not assumed." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [claudine-wrapper], reason: "Wrapper launches one-shot run without ACP." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [claudine-wrapper], reason: "No live idle endpoint after one-shot completion." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [source-cli], reason: "No ordinary peer protocol established." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [source-cli], reason: "No independent idle protocol established." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [claudine-wrapper], reason: "No retained ACP channel." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [claudine-wrapper], reason: "No independent idle delivery." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [source-cli], reason: "One-shot without endpoint." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [source-cli], reason: "Completed one-shot is not an idle session process." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [claudine-wrapper], reason: "One-shot wrapper without endpoint." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["native Linux check"], evidence_ids: [claudine-wrapper], reason: "No retained idle process." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [official-install, source-cli], reason: "Native Windows is supported for install, but ordinary peer steering is unestablished; WSL is not proof." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [official-install, source-cli], reason: "No independent idle protocol." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [claudine-wrapper], reason: "No retained ACP channel." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [claudine-wrapper], reason: "No independent idle delivery." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [official-install, source-cli], reason: "One-shot without endpoint." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [official-install, source-cli], reason: "Completed one-shot is not a live idle process." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [claudine-wrapper], reason: "One-shot wrapper without endpoint." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["native Windows check"], evidence_ids: [claudine-wrapper], reason: "No retained idle process." }
  - { profile_id: managed-acp-server, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [acp-macos-native], mechanism_ids: [acp-steer], prerequisites: ["managed server", "exact sessionId", "current activeRunId", "live activation test"], evidence_ids: [source-steer, source-loop, source-steer-test], reason: "Source establishes guarded same-run queueing without cancellation; activation remains blocked by empty verification." }
  - { profile_id: managed-acp-server, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [acp-macos-native], mechanism_ids: [acp-idle-prompt], prerequisites: ["managed server", "idle revalidation", "live activation test"], evidence_ids: [source-prompt], reason: "Documented ACP prompt starts a turn in the same managed session." }
  - { profile_id: managed-acp-server, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [acp-macos-claudine], mechanism_ids: [acp-steer], prerequisites: ["future managed profile", "live activation test"], evidence_ids: [source-steer, claudine-wrapper], reason: "Provider capability exists but current Claudine does not implement it." }
  - { profile_id: managed-acp-server, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [acp-macos-claudine], mechanism_ids: [acp-idle-prompt], prerequisites: ["future managed profile", "live activation test"], evidence_ids: [source-prompt, claudine-wrapper], reason: "Same managed session can start a turn; adapter absent today." }
  - { profile_id: managed-acp-server, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [acp-linux-native], mechanism_ids: [acp-steer], prerequisites: ["native Linux disposable test"], evidence_ids: [source-steer, source-loop], reason: "Cross-platform source supports the mechanism; native runtime unverified." }
  - { profile_id: managed-acp-server, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [acp-linux-native], mechanism_ids: [acp-idle-prompt], prerequisites: ["native Linux disposable test"], evidence_ids: [source-prompt], reason: "ACP idle prompt capability is source-established; runtime unverified." }
  - { profile_id: managed-acp-server, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [acp-linux-claudine], mechanism_ids: [acp-steer], prerequisites: ["future profile", "native Linux disposable test"], evidence_ids: [source-steer, claudine-wrapper], reason: "Capability exists; integration and live gate remain absent." }
  - { profile_id: managed-acp-server, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [acp-linux-claudine], mechanism_ids: [acp-idle-prompt], prerequisites: ["future profile", "native Linux disposable test"], evidence_ids: [source-prompt, claudine-wrapper], reason: "Capability exists; integration and live gate remain absent." }
  - { profile_id: managed-acp-server, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [acp-windows-native], mechanism_ids: [acp-steer], prerequisites: ["native Windows disposable test"], evidence_ids: [official-install, source-steer, source-loop], reason: "Source and native install support make this a candidate; WSL is not evidence and runtime is unverified." }
  - { profile_id: managed-acp-server, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [acp-windows-native], mechanism_ids: [acp-idle-prompt], prerequisites: ["native Windows disposable test"], evidence_ids: [official-install, source-prompt], reason: "Candidate only until native Windows delivery verification." }
  - { profile_id: managed-acp-server, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [acp-windows-claudine], mechanism_ids: [acp-steer], prerequisites: ["future profile", "native Windows disposable test"], evidence_ids: [official-install, source-steer, claudine-wrapper], reason: "Provider capability exists; no current adapter or native Windows test." }
  - { profile_id: managed-acp-server, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [acp-windows-claudine], mechanism_ids: [acp-idle-prompt], prerequisites: ["future profile", "native Windows disposable test"], evidence_ids: [official-install, source-prompt, claudine-wrapper], reason: "Provider capability exists; no current adapter or native Windows test." }
gaps:
  - { area: activation, detail: "verification is empty; no mechanism may be enabled.", next_check: "Run disposable sessions for exact v1.49.0 on each native OS and each claimed launch/origin/state tuple." }
  - { area: ordinary sessions, detail: "No provider registry or peer endpoint ties an ordinary process to its loaded conversation; stdin and terminal injection are not accepted as protocols.", next_check: "Passively inspect a future installed ordinary session and upstream docs; if no interface exists, retain unknown until affirmative negative evidence supports unsupported." }
  - { area: token loop, detail: "Steering drains only between model/tool turns, so a generation loop that never reaches either boundary will not receive it promptly.", next_check: "Disposable test a long token stream and establish whether any provider stream checkpoint can drain steering." }
  - { area: long tools, detail: "A steer waits for the tool boundary, while cancellation's child-process semantics are unestablished.", next_check: "Use a harmless bounded long-running tool fixture; assert no cancellation for steer and exact cleanup for explicit cancel." }
  - { area: receipts, detail: "Steer has strong queued/pickup correlation, but idle prompt has no separate initial acceptance acknowledgment and provider/model incorporation remains unobservable.", next_check: "Capture wire events and assert request response, queued notice, pickup chunk, model-visible nonce, and final response separately." }
  - { area: compatibility, detail: "The custom steering namespace is unstable and cannot be safely feature-probed without mutation; version allowlisting is required.", next_check: "Establish first/last tested releases and reject untested agentInfo.version values." }
  - { area: transport, detail: "HTTP and WebSocket framing, secret header/query behavior, reconnects, timeouts, and body limits were not live-inspected.", next_check: "Test both transports without real credentials using a deterministic provider fixture." }
  - { area: retained stdio, detail: "`goose acp` exposes the same ACP agent over owned stdio but cannot be attached to after launch; it was investigated but intentionally not merged with the exposed-server profile.", next_check: "If Claudine chooses an owned-child profile, define a separate retained-stdio launch profile and repeat every case/access/compatibility/test record." }
  - { area: interruption race, detail: "Cancel lacks expectedRunId and a direct acknowledgment; the wrong newly active run could be canceled after a race.", next_check: "Request an upstream guarded cancellation operation or prove a safe serialization strategy in a disposable server." }
  - { area: model provenance, detail: "The task supplied gpt-5.6-sol/low, but resolved execution metadata was not exposed.", next_check: "Have the sequence launcher retain resolved model and effort metadata and audit it before accepting fleet output." }
changes:
  - "Created the first Goose steering report under schema revision 2; no prior goose.md existed to refresh and therefore created is today's date."
  - "Reverified prior ACP leads against v1.49.0 and replaced the older generic ACP view with the guarded custom steering request, correlated pickup, and explicit activation block."
  - "Separated ordinary CLI coverage from the managed authenticated ACP server profile and left retained stdio as a separately identified future profile gap."
requires_claudine_update: true
reason: "A future adapter should manage authenticated `goose serve`, preserve the user's normal Goose config/resources, register endpoint/session/run identity, implement guarded steer and idle prompt, and enforce exact-version plus disposable-test gates. Current Claudine only launches ordinary one-shot `goose run`."
discovery_gaps: []
interface_inventory:
- disposition: included
  evidence_ids:
  - official-cli
  - source-cli
  - local-host
  - claudine-wrapper
  id: profile-ordinary-cli
  profile_ids:
  - ordinary-cli
  reason: Ordinary `goose session` or `goose run`; neither exposes a peer control endpoint.
- disposition: included
  evidence_ids:
  - official-acp
  - source-cli
  - source-steer
  - source-sessions
  - source-auth
  id: profile-managed-acp-server
  profile_ids:
  - managed-acp-server
  reason: A deliberately retained `goose serve` process exposing authenticated ACP over HTTP/WebSocket; this is a separate future managed profile, not an attachment to an ordinary CLI.
- disposition: unknown
  evidence_ids:
  - official-acp
  - source-cli
  - source-steer
  - source-sessions
  - source-auth
  id: coverage-review-managed-acp-server
  profile_ids:
  - managed-acp-server
  reason: The existing profile does not cover other client launch modes. This migration does not establish that these combinations are impossible. Review interface ownership, lifetime, and discovery before expanding coverage; do not infer exclusion from current wrapper behavior.
receipt_observations:
- evidence_ids:
  - source-steer
  - source-steer-test
  mechanism_id: acp-steer
  signal: '{"jsonrpc":"2.0","id":"request-id","result":{"runId":"run_...","messageId":"steer_..."}} Initial receipt only; later processing and settlement have separate signals.'
  timing: early
- evidence_ids:
  - source-prompt
  mechanism_id: acp-idle-prompt
  signal: Successful session/prompt PromptResponse after the turn; no separate early acceptance response is established.
  timing: terminal
- evidence_ids:
  - source-cancel
  - source-prompt
  mechanism_id: acp-cancel-submit
  signal: Cancellation and replacement have separate outcomes. old PromptResponse stopReason=cancelled; independent replacement PromptResponse The cancel notification has no response; submission is a separate operation and may fail after cancellation.
  timing: multi_phase

---

# Goose steering research

## Overview

Goose v1.49.0 has a promising non-interrupting steering capability, but only in a deliberately exposed ACP server session. The source-derived custom request `_goose/unstable/session/steer` targets an exact `sessionId`, requires the caller's current `expectedRunId`, queues the guidance without canceling the turn, and returns a provider-generated `messageId`. A later marked user-message update can confirm pickup. This is candidate provider capability, not enabled Claudine support: there is no disposable-session verification record, the custom method is explicitly unstable, and the inspected macOS host has no Goose binary.

Ordinary `goose session` and `goose run` sessions expose no discovered peer interface. A process PID is not a conversation identity; persisted history is not liveness; and a write to stdin or terminal keystroke injection would be UI automation, not an acknowledged messaging protocol. `goose acp` is a useful owned-child alternative, but its retained stdio is available only from process launch and cannot enable an already-open ordinary session. It must become a separate profile if selected.

Research provenance is launcher-supplied: this task identifies the researcher as Codex using `gpt-5.6-sol` with low reasoning effort. The environment did not expose independently resolved execution metadata, so this report does not fabricate independent verification.

## Session discovery

`session/list` enumerates persisted User, Scheduled, and ACP sessions and supplies durable provider session IDs plus metadata. It does not say which ordinary TUI process has loaded a conversation, whether a listed session is alive, or which helper/server process owns it. Under a managed `goose serve`, Claudine can combine its launch registration, authenticated endpoint health, exact `sessionId`, and the custom `activeRunId` session-info update. That supports deduplication by server identity plus session ID and guards a working run against stale targeting. The absence of `activeRunId` is only a racing snapshot, so idle delivery must be serialized or revalidated at submission.

Multiple conversations can exist behind one long-lived server, while one ordinary process can resume different histories over its lifetime. Process IDs—especially reused IDs—must remain labels, never provider session identity. Helper tools and server processes must be classified by launch profile rather than counted as conversations.

## Non-interrupting delivery

The custom steering request is source-derived in v1.49.0. Goose rejects an empty prompt, an empty guard, an absent active run, or a stale `expectedRunId`; stale errors include expected and actual run IDs. On success it enqueues the message in a per-session in-memory FIFO and returns `runId` and `messageId`. Custom notifications announce `queuedSteer`, and the picked-up `UserMessageChunk` carries the same message ID with steer metadata.

Pickup is not immediate during token generation or a running tool. The state machine drains steering between turns, after an assistant response ends or after a tool message. It drains the whole pending queue. Therefore the mechanism cannot help a token-generation loop that never reaches a model/tool boundary, and a never-ending tool also prevents pickup. A successful response proves enqueue under the guarded run; it does not prove pickup or model incorporation.

## Interruption fallback

ACP `session/cancel` cancels the active run token, but it is a notification with no correlated response and no `expectedRunId`. Replacement text requires a later, separate `session/prompt`. The safe manual sequence is: explain the effect, obtain an explicit choice, identify the target, send cancel, await the old prompt's canceled completion and a cleared active-run signal, revalidate the same session, then submit the replacement.

This sequence has an irreversible partial-failure state: cancellation can succeed and submission can fail. The original work remains canceled, ACP cleanup may discard queued steers, and there is no rollback. Automatic loop warnings must never use this path. Tool-child termination and cancellation timeout behavior remain unverified.

## Idle sessions

An idle managed ACP session accepts `session/prompt`, which starts a turn in that same session. The call streams updates and returns only when the turn completes; no distinct initial acceptance receipt was found. Provider permission requests can hold later tool work, but that is not a message-delivery state. An idle ordinary TUI may accept human keyboard input, but no stable external protocol, conversation guard, or acknowledgment was established.

## Protocol details

`goose serve` exposes ACP over HTTP and WebSocket and defaults to loopback. The official guidance requires `GOOSE_SERVER__SECRET_KEY` unless the operator explicitly opts into dangerous unauthenticated mode. This report models HTTP framing; WebSocket must be verified separately before use. The standard prompt request and the custom steer request are JSON-RPC operations, while cancel is a notification. There is no caller-supplied message idempotency key. An ambiguous timeout must never be retried because duplicate user content is possible.

Steering text is converted through Goose's ACP content conversion and re-enters the ordinary user-message/hook pipeline; exact skill, prompt-template, slash-command, and extension-command interpretation was not exhaustively established. The managed profile can preserve configured extensions, skills, prompt templates, and context because it uses the same Goose configuration and session construction, but Claudine must pass explicit provider settings and must not replace this with a stripped transport-specific home. Non-interactive operation also needs a permission mode or reverse-request handler that cannot wait for a receiving human.

## OS and version compatibility

Official installation material covers native macOS, Linux, and Windows and describes WSL separately. This run inspected source on macOS only; no provider binary was installed, and there is no native Linux or native Windows runtime evidence. WSL would be Linux-side evidence and is not accepted as native Windows proof.

The exact examined release is v1.49.0. The custom method lives under `_goose/unstable`, and no documented first/last compatible release was found. Read-only compatibility can verify authenticated handshake, exact `agentInfo.version`, advertised list/load capabilities, and active-run notifications. It cannot prove that the mutating custom method is implemented or semantically compatible. Claudine must use an exact tested-version allowlist rather than speculative version bounds.

## Disposable-test proposals

For every OS/profile/origin/state claim, launch a fresh server with a deterministic test provider, isolated config/session storage, harmless tools, and no real credentials. For active steering, capture the initial prompt request, wait for `activeRunId`, send a nonce-bearing steer with that guard, and separately assert response acceptance, queued notification, pickup chunk correlation, same session/run identity, no cancellation, and model-visible incorporation. Repeat during a long token stream, a bounded long tool, and a multi-tool batch; record the actual pickup boundary and FIFO order.

For idle delivery, load a known session, prove no active run, submit one nonce prompt, assert one new turn and no new conversation, and test an ambiguous transport failure without retry. For interruption, test explicit cancel during generation and during a child tool, assert exactly what stops and what context persists, then deliberately fail replacement submission to capture the partial outcome. Repeat on native macOS, Linux, and Windows; keep WSL labeled Linux-side. Test HTTP and WebSocket independently, including authentication failure, stale guard, absent run, reconnect, timeout, maximum accepted payload, duplicate behavior, and custom-notification opt-in.

## Claudine integration

The preferred future profile is an authenticated, loopback-bound `goose serve` managed by Claudine. Registration should include process creation identity, endpoint, exact Goose version, profile ID, session ID, and latest active run ID. Selection must distinguish historical sessions from live managed sessions and display working/idle state as a racing observation. Delivery should prefer guarded `acp-steer` for a working session and `session/prompt` for a revalidated idle session. The manual cancel-submit fallback remains disabled until separately verified and must always require an explicit interactive choice.

For an already-running ordinary Goose session without a reachable control interface, report that steering is unavailable and explain the future managed ACP launch requirement. This does not establish a tested fallback or authorize replacing the running session. `goose acp` could support an owned retained-stdio profile, but only for future launches and only after receiving its own complete case matrix and live tests. Setup cannot retrofit an already-open ordinary session. Claudine must preserve the user's extensions, skills, templates, context files, provider/model settings, and permission posture; transport selection must not silently disable them.

## Gaps

The activation gate is intentionally blocked by `verification: []`. Major blockers are the unstable interface's missing compatibility bounds, absence of a safe read-only support probe, no local/native cross-OS runtime evidence, no independent prompt-acceptance acknowledgment, unknown payload limits, and incomplete cancellation/tool-child semantics. Ordinary-session discovery and delivery remain unknown rather than unsupported because affirmative provider evidence of impossibility is incomplete.

## Sources

Primary sources are the official Goose documentation and the commit-pinned v1.49.0 files identified in the evidence catalog above. The local observation came from Sniff's sanitized agent inventory plus a non-interactive executable lookup. Prior Claudine ACP and topic research was used only as a lead; claims in this report were rechecked against v1.49.0 source.

## Changelog

- 2026-09-08: Created schema-revision-2 Goose steering research, pinned claims to v1.49.0, identified guarded non-interrupting ACP steering, separated ordinary and managed-server coverage, and left live activation blocked.


## Revision 3 Contract Backfill

Receipt timing, interface inventory, and case-specific discovery gaps were added
from the existing evidence on 2026-09-08. No new provider observation or live test
was performed. Unexamined profile combinations remain unknown, not unsupported.
The original fleet model/effort provenance above describes the research run;
this deterministic contract migration is a separate coordinator edit.
