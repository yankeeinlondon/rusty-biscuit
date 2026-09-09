---
$schema: ./_schema.yaml
schema_revision: 3
provider: pi
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined: ["Pi 0.84.4 on macOS", "earendil-works/pi v0.84.4 commit b79e4cc834970cca69daebffab7df1da7d1e52c4"]
launch_profiles:
  - { id: ordinary-cli, description: "Ordinary Pi TUI or print/JSON invocation without a retained peer endpoint.", endpoint_scope: none, lifetime: unknown, applicable_os: [macos, linux, windows], applicable_launch_modes: [interactive, non_interactive], applicable_origins: [native, claudine], baseline: true, startup_requirements: ["ordinary pi launch"], preserves_extensions: yes, preserves_skills: yes, preserves_templates: yes, preserves_context: yes, evidence_ids: [local-pi, official-home, wrapper] }
  - { id: retained-rpc, description: "Pi child deliberately launched in RPC mode with controller-owned stdin/stdout.", endpoint_scope: retained_stdio, lifetime: while_client_open, applicable_os: [macos, linux, windows], applicable_launch_modes: [non_interactive], applicable_origins: [native, claudine], baseline: false, startup_requirements: ["pi --mode rpc", "retain and protect pipes", "register expected session identity"], preserves_extensions: yes, preserves_skills: yes, preserves_templates: yes, preserves_context: yes, evidence_ids: [official-rpc, source-rpc] }
access_findings:
  - { mechanism_id: rpc-steer, profile_id: retained-rpc, os: macos, status: setup_required, prerequisite: "Owned RPC child, retained pipes, fresh get_state.", applies_to_existing_sessions: no, evidence_ids: [official-rpc, source-rpc, local-pi] }
  - { mechanism_id: rpc-steer, profile_id: retained-rpc, os: linux, status: setup_required, prerequisite: "Owned RPC child; native Linux unverified.", applies_to_existing_sessions: no, evidence_ids: [official-rpc, source-rpc] }
  - { mechanism_id: rpc-steer, profile_id: retained-rpc, os: windows, status: setup_required, prerequisite: "Owned RPC child and binary-safe pipes; native Windows unverified.", applies_to_existing_sessions: no, evidence_ids: [official-rpc, source-rpc] }
  - { mechanism_id: rpc-idle-prompt, profile_id: retained-rpc, os: macos, status: setup_required, prerequisite: "Owned RPC child and expected idle session.", applies_to_existing_sessions: no, evidence_ids: [official-rpc, source-rpc] }
  - { mechanism_id: rpc-idle-prompt, profile_id: retained-rpc, os: linux, status: setup_required, prerequisite: "Owned RPC child; native Linux unverified.", applies_to_existing_sessions: no, evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-idle-prompt, profile_id: retained-rpc, os: windows, status: setup_required, prerequisite: "Owned RPC child; native Windows unverified.", applies_to_existing_sessions: no, evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-abort-submit, profile_id: retained-rpc, os: macos, status: setup_required, prerequisite: "Owned RPC child, explicit interactive approval, revalidation between phases.", applies_to_existing_sessions: no, evidence_ids: [official-rpc, source-rpc] }
  - { mechanism_id: rpc-abort-submit, profile_id: retained-rpc, os: linux, status: setup_required, prerequisite: "Managed RPC and explicit approval; native Linux unverified.", applies_to_existing_sessions: no, evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-abort-submit, profile_id: retained-rpc, os: windows, status: setup_required, prerequisite: "Managed RPC and explicit approval; native Windows unverified.", applies_to_existing_sessions: no, evidence_ids: [official-rpc] }
delivery_states:
  - { mechanism_id: rpc-steer, states: [accepted, queued, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "Command id correlates only its response; queue_update and user message events have no originating id and duplicate text is ambiguous.", evidence_ids: [official-rpc, source-rpc, source-session] }
  - { mechanism_id: rpc-idle-prompt, states: [accepted, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "Command id correlates preflight response; later lifecycle events are uncorrelated.", evidence_ids: [official-rpc, source-rpc] }
  - { mechanism_id: rpc-abort-submit, states: [accepted, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "Abort and replacement have separate ids and outcomes.", evidence_ids: [official-rpc, source-rpc] }
receipt_guarantees:
  - { mechanism_id: rpc-steer, request_acceptance: confirmed, persistence: not_persisted, scheduling: confirmed, conversation_delivery: unknown, provider_signals: [queue_update, message_start, message_end, agent_settled], correlation: request_id, evidence_ids: [official-rpc, source-rpc, source-session], limitations: "Success proves in-memory enqueue, not exact model incorporation or a race-safe target." }
  - { mechanism_id: rpc-idle-prompt, request_acceptance: confirmed, persistence: unknown, scheduling: unknown, conversation_delivery: unknown, provider_signals: [agent_start, turn_start, message_start, message_end, agent_end, agent_settled], correlation: request_id, evidence_ids: [official-rpc, source-rpc, source-settled], limitations: "Success is preflight acceptance or immediate extension handling, so it does not universally establish a scheduled model turn; later events lack the request id." }
  - { mechanism_id: rpc-abort-submit, request_acceptance: confirmed, persistence: unknown, scheduling: unknown, conversation_delivery: unknown, provider_signals: ["abort response after idle", "separate prompt response", agent_settled], correlation: request_id, evidence_ids: [official-rpc, source-rpc], limitations: "Abort acknowledgment proves idle, not replacement acceptance." }
evidence:
  - claim: RPC abort removes the marked built-in bash external process; killing Pi leaves it running until fixture cleanup. Context, skill, and template discovery remain enabled.
    id: live-rpc-bash-cleanup-0844
    limitations: Real Pi 0.84.4/macOS with deterministic local model and built-in bash executing one marked external process. Checked after a 750 ms delay; provider abort removed it, provider SIGKILL did not, and fixture cleanup then removed it. No additional descendants, native Windows/Linux, arbitrary extensions, or cloud inference tested. Not production activation.
    location: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-bash-cleanup.json
    method: disposable_test
    observed_on: 2026-09-08
    version: Pi 0.84.4 macOS

  - claim: Steering accepted during a gated session switch disappears without transcript persistence or model consumption; steering accepted after the switch reaches the new session. Abrupt provider termination after acknowledgment loses queued steering before history or model consumption.
    id: live-rpc-switch-crash-0844
    limitations: Real Pi 0.84.4/macOS with deterministic local model and fixture extensions. Controlled idle switch interleaving and kill after acknowledged steering during cooperative tool waits; no exhaustive scheduling, cloud inference, host crash, filesystem durability, or restart. Passing loss assertions do not activate delivery support.
    location: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-switch-crash.json
    method: disposable_test
    observed_on: 2026-09-08
    version: Pi 0.84.4 macOS

  - claim: Abort preserves queued steering as saved user history without immediate model consumption; explicit next prompt consumes it. clear_queue before abort removes it. stdin EOF while tools are held loses even acknowledged steering without saved user history despite exit code zero.
    id: live-rpc-failures-0844
    limitations: Real Pi with local deterministic model and cooperative in-process tools on macOS only. EOF tests close stdin while retaining stdout observation; no cloud provider, OS subprocess cleanup, full-duplex loss, host crash, or concurrent target race. Expected failure behavior is a passing regression assertion, not successful message delivery.
    location: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-failures.json
    method: disposable_test
    observed_on: 2026-09-08
    version: Pi 0.84.4 macOS
  - claim: Real Pi RPC with deterministic local model passed correlated admission, full two-tool batch delivery, same-session continuity, duplicate request IDs, resource expansion, extension-handled input, mutable session identity, and unanswered UI checks.
    id: live-rpc-fixture-0844
    limitations: Isolated fixture resources and local deterministic model; no cloud backend, production Claudine adapter, concurrent targeting race, disconnect, abort cleanup, Linux or Windows verification.
    location: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4.json
    method: disposable_test
    observed_on: 2026-09-08
    version: Pi 0.84.4 macOS

  - { id: official-home, method: official_docs, location: "https://pi.dev/", version: "observed 2026-09-08", observed_on: 2026-09-08, claim: "Pi supports terminal and programmatic operation.", limitations: "Not a protocol specification." }
  - { id: official-rpc, method: official_docs, location: "https://pi.dev/docs/latest/rpc", version: "latest observed 2026-09-08; pinned separately to v0.84.4 source", observed_on: 2026-09-08, claim: "Documents JSONL RPC prompt, steer, follow_up, abort, clear_queue, get_state, responses, queue events, agent_end, and agent_settled.", limitations: "Does not prove live or cross-OS behavior." }
  - { id: official-sdk, method: official_docs, location: "https://pi.dev/docs/latest/sdk", version: "latest observed 2026-09-08", observed_on: 2026-09-08, claim: "In-process AgentSession exposes prompting, queues, abort, state, and event subscriptions.", limitations: "Does not attach to another CLI process." }
  - { id: official-sessions, method: official_docs, location: "https://pi.dev/docs/latest/sessions", version: "latest observed 2026-09-08", observed_on: 2026-09-08, claim: "Pi persists conversation trees as project-scoped JSONL sessions.", limitations: "History is not liveness, loaded identity, or reachability." }
  - { id: source-rpc, method: source_code, location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/modes/rpc/rpc-mode.ts", version: "v0.84.4", observed_on: 2026-09-08, claim: "RPC subscribes to AgentSession events; steer/follow_up enqueue, prompt responds after preflight, abort awaits idle, and get_state exposes current session/activity.", limitations: "No session or operation guard; not a live test." }
  - { id: source-session, method: source_code, location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session.ts", version: "v0.84.4", observed_on: 2026-09-08, claim: "AgentSession maintains in-memory text queues, emits queue_update, expands skills/templates, and rejects extension commands in direct steering.", limitations: "Text matching is not message identity." }
  - { id: source-loop, method: source_code, location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/agent/src/agent-loop.ts", version: "v0.84.4", observed_on: 2026-09-08, claim: "Steering drains after the assistant response and complete tool batch; follow-ups drain when the agent would otherwise stop.", limitations: "Source timing is not delivery verification." }
  - { id: source-settled, method: source_code, location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session.ts", version: "v0.84.4", observed_on: 2026-09-08, claim: "AgentSession defines and emits agent_settled after session-level retries, compaction recovery, and queued continuations; agent_end is lower-level.", limitations: "Settlement edge cases were not live-tested." }
  - { id: local-pi, method: local_inspection, location: "Sanitized Sniff software inventory, pi version/help, installed 0.84.4 package/docs on macOS", version: "0.84.4", observed_on: 2026-09-08, claim: "Sniff found Pi 0.84.4; help advertises text/json/rpc and resource flags; installed RPC forwards agent_settled.", limitations: "No session was launched, focused, messaged, or interrupted." }
  - { id: wrapper, method: source_code, location: "claudine/cli/src/commands/wrap/profile/pi.rs", version: "workspace 2026-09-08", observed_on: 2026-09-08, claim: "Claudine currently launches one-shot JSON and does not retain RPC.", limitations: "Claudine behavior is not provider capability." }
discovery:
  - { id: ordinary-macos-native, profile_id: ordinary-cli, os: macos, origin: native, method: process_inspection, locator: "Process/session-file hints.", identity_check: "PID/history cannot prove loaded session.", liveness_check: "PID only.", state_detection: unknown, observation_source: "Sniff and session docs", observed_at: "2026-09-08 macOS", available_labels: [provider, pid, command, cwd], prerequisites: [], evidence_ids: [local-pi, official-sessions] }
  - { id: ordinary-macos-claudine, profile_id: ordinary-cli, os: macos, origin: claudine, method: claudine_registration, locator: "Wrapper child record.", identity_check: "No Pi-session validation.", liveness_check: "Child handle.", state_detection: "Output activity only.", observation_source: "wrapper source", observed_at: "2026-09-08", available_labels: [provider, pid, claudine_session_id], prerequisites: [], evidence_ids: [wrapper] }
  - { id: ordinary-linux-native, profile_id: ordinary-cli, os: linux, origin: native, method: process_inspection, locator: "Process/history hints.", identity_check: "Unknown.", liveness_check: "PID only.", state_detection: unknown, observation_source: "source/docs inference", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Linux check"], evidence_ids: [official-sessions] }
  - { id: ordinary-linux-claudine, profile_id: ordinary-cli, os: linux, origin: claudine, method: claudine_registration, locator: "Wrapper child record.", identity_check: "No provider check.", liveness_check: "Child handle.", state_detection: unknown, observation_source: "wrapper source", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Linux check"], evidence_ids: [wrapper] }
  - { id: ordinary-windows-native, profile_id: ordinary-cli, os: windows, origin: native, method: process_inspection, locator: "Process/history hints.", identity_check: "PID requires creation-time guard but still lacks session identity.", liveness_check: "PID only.", state_detection: unknown, observation_source: "source/docs inference", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Windows check"], evidence_ids: [official-sessions] }
  - { id: ordinary-windows-claudine, profile_id: ordinary-cli, os: windows, origin: claudine, method: claudine_registration, locator: "Wrapper child record.", identity_check: "No provider check.", liveness_check: "Child handle/creation identity.", state_detection: unknown, observation_source: "wrapper source", observed_at: "not observed", available_labels: [provider, pid], prerequisites: ["native Windows check"], evidence_ids: [wrapper] }
  - { id: rpc-macos-native, profile_id: retained-rpc, os: macos, origin: native, method: provider_api, locator: "Controller registry plus get_state.", identity_check: "Process creation identity, sessionId, sessionFile immediately before send.", liveness_check: "Correlated get_state.", state_detection: "isStreaming/isCompacting plus events; snapshot races.", observation_source: "v0.84.4 source", observed_at: "2026-09-08", available_labels: [pid, session_id, session_file, session_name, state], prerequisites: ["owned RPC child"], evidence_ids: [source-rpc] }
  - { id: rpc-macos-claudine, profile_id: retained-rpc, os: macos, origin: claudine, method: claudine_registration, locator: "Future launch registration plus get_state.", identity_check: "Reconcile registration and state.", liveness_check: "Child handle/get_state.", state_detection: "state plus agent_settled.", observation_source: "source and wrapper gap", observed_at: "2026-09-08", available_labels: [pid, session_id, session_file, state, profile], prerequisites: ["future RPC wrapper"], evidence_ids: [source-rpc, wrapper] }
  - { id: rpc-linux-native, profile_id: retained-rpc, os: linux, origin: native, method: provider_api, locator: "Owned RPC plus get_state.", identity_check: "Fresh state and process identity.", liveness_check: get_state, state_detection: "state/events", observation_source: "cross-platform source", observed_at: "not observed", available_labels: [pid, session_id, state], prerequisites: ["native Linux check"], evidence_ids: [source-rpc] }
  - { id: rpc-linux-claudine, profile_id: retained-rpc, os: linux, origin: claudine, method: claudine_registration, locator: "Future RPC registration.", identity_check: "Registration/get_state.", liveness_check: get_state, state_detection: "state/events", observation_source: "source-derived", observed_at: "not observed", available_labels: [pid, session_id, state], prerequisites: ["future wrapper", "native Linux check"], evidence_ids: [source-rpc, wrapper] }
  - { id: rpc-windows-native, profile_id: retained-rpc, os: windows, origin: native, method: provider_api, locator: "Owned child pipes plus get_state.", identity_check: "Fresh state and creation identity.", liveness_check: get_state, state_detection: "state/events", observation_source: "cross-platform source", observed_at: "not observed", available_labels: [pid, session_id, state], prerequisites: ["native Windows check"], evidence_ids: [source-rpc] }
  - { id: rpc-windows-claudine, profile_id: retained-rpc, os: windows, origin: claudine, method: claudine_registration, locator: "Future RPC registration.", identity_check: "Registration/get_state.", liveness_check: get_state, state_detection: "state/events", observation_source: "source-derived", observed_at: "not observed", available_labels: [pid, session_id, state], prerequisites: ["future wrapper", "native Windows check"], evidence_ids: [source-rpc, wrapper] }
mechanisms:
  - { id: rpc-steer, interface_status: documented, maturity: stable, transport: stdio, initialization: "Launch RPC, retain pipes, continuously drain stdout, correlate get_state.", operation_intent: steer_active_turn, conversation_effect: preserve_running_turn, delivery_boundary: end_of_tool_batch, destination: "Mutable current AgentSession in that child; request has no session id.", authentication: "Possession/protection of child pipes.", startup_requirements: ["retained pipes", "registered expected session"], target_preconditions: ["expected session", "streaming", "not compacting"], target_guards: ["no expected session/operation id", "session replacement can retarget; macOS 0.84.4 gated switch also discards acknowledged steering"], request_framing: "LF-delimited UTF-8 JSON stdin.", response_framing: "LF-delimited response interleaved with events.", request_format: '{"id":"id","type":"steer","message":"text"}', response_format: '{"id":"id","type":"response","command":"steer","success":true|false}', long_tool_behavior: "Waits through long tool and entire batch; hangs indefinitely if boundary never arrives.", tool_batch_behavior: continue_all, queue_behavior: "In-memory FIFO; all or one-at-a-time drain; clear_queue removes pending text.", message_interpretation: skills_templates, interruption_phases: [], interruption_partial_failure: "Not applicable.", ordering: "FIFO; concurrent sender ordering untested.", sender_message_id: unsupported, retry_policy: never_retry, duplicate_handling: "No suppression; request id is correlation, not idempotency.", cancellation: "clear_queue before insertion; no per-message recall after insertion; abort retains queues.", limits: "No established size/count/expiry/persistence bounds; extension commands rejected.", evidence_ids: [official-rpc, source-rpc, source-session, source-loop, live-rpc-switch-crash-0844] }
  - { id: rpc-idle-prompt, interface_status: documented, maturity: stable, transport: stdio, initialization: "Retained RPC with expected idle current session.", operation_intent: start_idle_turn, conversation_effect: resume_same_conversation, delivery_boundary: idle_turn_start, destination: "Mutable current idle AgentSession.", authentication: "Possession/protection of pipes.", startup_requirements: ["retained RPC", "stdout reader"], target_preconditions: ["expected session", "isStreaming false", "not compacting"], target_guards: ["no expected session id", "idle snapshot can race", "busy prompt without streamingBehavior is refused"], request_framing: "LF-delimited JSON.", response_framing: "Correlated preflight response then uncorrelated events.", request_format: '{"id":"id","type":"prompt","message":"text"}', response_format: "success after acceptance, queueing, or extension handling", long_tool_behavior: "Not applicable if idle.", tool_batch_behavior: not_applicable, queue_behavior: "Starts a turn; streamingBehavior can delegate to steer/follow-up.", message_interpretation: input_extension, interruption_phases: [], interruption_partial_failure: "Not applicable.", ordering: "Concurrent idle submissions unverified.", sender_message_id: unsupported, retry_policy: never_retry, duplicate_handling: "No idempotency.", cancellation: "abort cancels resulting run.", limits: "Extensions may handle/transform; skills/templates expand; size unknown.", evidence_ids: [official-rpc, source-rpc, source-session] }
  - { id: rpc-abort-submit, interface_status: documented, maturity: stable, transport: stdio, initialization: "Retained RPC after explicit manual choice.", operation_intent: interrupt_then_submit, conversation_effect: cancel_turn_same_conversation, delivery_boundary: next_turn, destination: "Abort current operation, then separately prompt revalidated current session.", authentication: "Possession/protection of pipes.", startup_requirements: ["manual approval", "fresh state before both phases"], target_preconditions: ["expected working session", "same idle session before replacement"], target_guards: ["no expected operation/session id", "get_state can detect a changed identity but cannot atomically guard a later send"], request_framing: "Two separate LF-delimited commands.", response_framing: "Separate correlated responses and events.", request_format: "abort; await idle; get_state; prompt", response_format: "abort success after idle; separate prompt preflight response", long_tool_behavior: "Abort signals current run; macOS 0.84.4 fixture confirms one built-in bash external process stops. Provider kill leaves that process alive; other trees/platforms unverified.", tool_batch_behavior: stop_remaining, queue_behavior: "Abort retains queues; clear only under explicit policy.", message_interpretation: input_extension, interruption_phases: ["approve", "identify", "optionally clear", "abort", "await idle", "revalidate", "prompt"], interruption_partial_failure: "Abort may succeed while prompt fails; original stays canceled and cleared queues stay cleared.", ordering: "Never pipeline phases.", sender_message_id: unsupported, retry_policy: never_retry, duplicate_handling: "No idempotency.", cancellation: "Destructive with no rollback.", limits: "Manual only; forbidden for automatic warnings.", evidence_ids: [official-rpc, source-rpc, source-session, live-rpc-bash-cleanup-0844] }
compatibility:
  - { mechanism_id: rpc-steer, profile_id: retained-rpc, os: macos, versions_verified: ["0.84.4 passive only"], documented_version_bounds: unknown, read_only_check: "Version/help plus installed steer/get_state/queue/settled types.", success_criteria: "Exact vocabulary/framing present.", failure_behavior: "Block send.", evidence_ids: [local-pi, source-rpc, source-settled] }
  - { mechanism_id: rpc-steer, profile_id: retained-rpc, os: linux, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Inspect exact installed types.", success_criteria: "Exact match.", failure_behavior: "Block send.", evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-steer, profile_id: retained-rpc, os: windows, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Inspect exact types and pipe framing.", success_criteria: "Exact match.", failure_behavior: "Block send.", evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-idle-prompt, profile_id: retained-rpc, os: macos, versions_verified: ["0.84.4 passive only"], documented_version_bounds: unknown, read_only_check: "Inspect prompt/get_state preflight types.", success_criteria: "Exact match.", failure_behavior: "Do not submit.", evidence_ids: [local-pi, source-rpc] }
  - { mechanism_id: rpc-idle-prompt, profile_id: retained-rpc, os: linux, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Inspect prompt/get_state types.", success_criteria: "Exact match.", failure_behavior: "Do not submit.", evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-idle-prompt, profile_id: retained-rpc, os: windows, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Inspect types and native pipe framing.", success_criteria: "Exact match.", failure_behavior: "Do not submit.", evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-abort-submit, profile_id: retained-rpc, os: macos, versions_verified: ["0.84.4 passive only"], documented_version_bounds: unknown, read_only_check: "Inspect abort/prompt/clear/get_state shapes.", success_criteria: "All phases match.", failure_behavior: "Disable before abort.", evidence_ids: [local-pi, source-rpc] }
  - { mechanism_id: rpc-abort-submit, profile_id: retained-rpc, os: linux, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Inspect all phase types.", success_criteria: "Exact match.", failure_behavior: "Disable fallback.", evidence_ids: [official-rpc] }
  - { mechanism_id: rpc-abort-submit, profile_id: retained-rpc, os: windows, versions_verified: [], documented_version_bounds: unknown, read_only_check: "Inspect all types and Windows behavior.", success_criteria: "Exact match.", failure_behavior: "Disable fallback.", evidence_ids: [official-rpc] }
verification:
- assertions:
  - abort acknowledgment followed by absence of marked external tool process at observation
  - provider SIGKILL leaves marked external tool process alive at observation
  - fixture cleanup removes surviving owned process
  - fixture context, skill, and template discovery remain enabled
  evidence_ids:
  - live-rpc-bash-cleanup-0844
  fixture: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-bash-cleanup.json
  launch_mode: non_interactive
  limitations: Real Pi 0.84.4/macOS with deterministic local model and built-in bash executing one marked external process. Checked after a 750 ms delay; provider abort removed it, provider SIGKILL did not, and fixture cleanup then removed it. No additional descendants, native Windows/Linux, arbitrary extensions, or cloud inference tested. Not production activation.
  mechanism_id: rpc-abort-submit
  origin: native
  os: macos
  outcome: passed
  profile_id: retained-rpc
  provider_version: 0.84.4
  session_state: working
  tested_on: 2026-09-08

- assertions:
  - old session remains observable during a pending switch
  - steering acknowledged during switch is lost when switch completes
  - steering queued after switch reaches the new session once
  evidence_ids:
  - live-rpc-switch-crash-0844
  fixture: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-switch-crash.json
  launch_mode: non_interactive
  limitations: Real Pi 0.84.4/macOS with deterministic local model and fixture extensions. Controlled idle switch interleaving and kill after acknowledged steering during cooperative tool waits; no exhaustive scheduling, cloud inference, host crash, filesystem durability, or restart. Passing loss assertions do not activate delivery support.
  mechanism_id: rpc-steer
  origin: native
  os: macos
  outcome: passed
  profile_id: retained-rpc
  provider_version: 0.84.4
  session_state: idle
  tested_on: 2026-09-08
- assertions:
  - provider killed after observed steering acknowledgment
  - queued steering neither saved in transcript nor consumed by model before abrupt termination
  evidence_ids:
  - live-rpc-switch-crash-0844
  fixture: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-switch-crash.json
  launch_mode: non_interactive
  limitations: Real Pi 0.84.4/macOS with deterministic local model and fixture extensions. Controlled idle switch interleaving and kill after acknowledged steering during cooperative tool waits; no exhaustive scheduling, cloud inference, host crash, filesystem durability, or restart. Passing loss assertions do not activate delivery support.
  mechanism_id: rpc-steer
  origin: native
  os: macos
  outcome: passed
  profile_id: retained-rpc
  provider_version: 0.84.4
  session_state: working
  tested_on: 2026-09-08

- assertions:
  - same session and idle after abort
  - cooperative tool waits canceled without normal completion
  - uncleared steering persists once in transcript and reaches explicit next model turn
  - clear_queue before abort prevents persistence and later delivery
  - malformed replacement rejected independently after abort
  evidence_ids:
  - live-rpc-failures-0844
  fixture: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-failures.json
  launch_mode: non_interactive
  limitations: Real Pi with local deterministic model and cooperative in-process tools on macOS only. EOF tests close stdin while retaining stdout observation; no cloud provider, OS subprocess cleanup, full-duplex loss, host crash, or concurrent target race. Expected failure behavior is a passing regression assertion, not successful message delivery.
  mechanism_id: rpc-abort-submit
  origin: native
  os: macos
  outcome: passed
  profile_id: retained-rpc
  provider_version: 0.84.4
  session_state: working
  tested_on: 2026-09-08
- assertions:
  - acknowledged steering not consumed or persisted after stdin EOF while tools are held
  - successful process exit does not prove delivery
  - no automatic replay after unobserved acknowledgment
  evidence_ids:
  - live-rpc-failures-0844
  fixture: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4-failures.json
  launch_mode: non_interactive
  limitations: Real Pi with local deterministic model and cooperative in-process tools on macOS only. EOF tests close stdin while retaining stdout observation; no cloud provider, OS subprocess cleanup, full-duplex loss, host crash, or concurrent target race. Expected failure behavior is a passing regression assertion, not successful message delivery.
  mechanism_id: rpc-steer
  origin: native
  os: macos
  outcome: passed
  profile_id: retained-rpc
  provider_version: 0.84.4
  session_state: working
  tested_on: 2026-09-08

- assertions:
  - acceptance precedes tool completion
  - both tool calls finish before subsequent model input contains steering
  - same session after settlement
  - duplicate request ID creates two user messages
  evidence_ids:
  - live-rpc-fixture-0844
  fixture: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4.json
  launch_mode: non_interactive
  limitations: Isolated fixture resources and local deterministic model; no cloud backend, production Claudine adapter, concurrent targeting race, disconnect, abort cleanup, Linux or Windows verification.
  mechanism_id: rpc-steer
  origin: native
  os: macos
  outcome: passed
  profile_id: retained-rpc
  provider_version: 0.84.4
  session_state: working
  tested_on: 2026-09-08
- assertions:
  - template and skill expansion reach model
  - context loaded
  - same conversation across idle prompts
  - extension-handled prompt succeeds without a model invocation
  - extension confirmation remains pending without an answer
  evidence_ids:
  - live-rpc-fixture-0844
  fixture: claudine/features/2026-09-08-steering/verification/pi-macos-0.84.4.json
  launch_mode: non_interactive
  limitations: Isolated fixture resources and local deterministic model; no cloud backend, production Claudine adapter, concurrent targeting race, disconnect, abort cleanup, Linux or Windows verification.
  mechanism_id: rpc-idle-prompt
  origin: native
  os: macos
  outcome: passed
  profile_id: retained-rpc
  provider_version: 0.84.4
  session_state: idle
  tested_on: 2026-09-08
cases:
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-pi], reason: "No documented peer channel; keystroke injection is distinct and untested." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-pi], reason: "Terminal input is not an external protocol." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: ["future interactive wrapper"], evidence_ids: [wrapper], reason: "Not currently produced or attachable." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: ["future interactive wrapper"], evidence_ids: [wrapper], reason: "Not currently produced." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-pi], reason: "Print/JSON stdin is initial input; RPC is separate." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-pi], reason: "One-shot exits." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper], reason: "Current wrapper lacks RPC input." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper], reason: "Child exits." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "No peer protocol/evidence." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "Terminal input is not peer messaging." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["future wrapper"], evidence_ids: [wrapper], reason: "Impossible with current wrapper." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["future wrapper"], evidence_ids: [wrapper], reason: "Impossible with current wrapper." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "Ordinary stdin undocumented for steering." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "One-shot expected to exit." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [wrapper], reason: "JSON, not RPC." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [wrapper], reason: "Child exits." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "No native evidence; WSL is Linux-side only." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "Keystrokes are not provider messaging." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["future wrapper"], evidence_ids: [wrapper], reason: "Impossible currently." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["future wrapper"], evidence_ids: [wrapper], reason: "Impossible currently." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "Ordinary stdin undocumented." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [official-home], reason: "No reachable idle evidence." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [wrapper], reason: "JSON, not RPC." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: ["native check"], evidence_ids: [wrapper], reason: "Child exits." }
  - { profile_id: retained-rpc, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [rpc-macos-native], mechanism_ids: [rpc-steer], prerequisites: ["managed RPC", "fresh state", "live gate"], evidence_ids: [official-rpc, source-loop], reason: "Documented same-session queue preserves work but waits for batch end." }
  - { profile_id: retained-rpc, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [rpc-macos-native], mechanism_ids: [rpc-idle-prompt], prerequisites: ["managed RPC", "confirmed idle", "live gate"], evidence_ids: [official-rpc, source-rpc], reason: "Prompt starts a turn in retained session." }
  - { profile_id: retained-rpc, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [rpc-macos-claudine], mechanism_ids: [rpc-steer], prerequisites: ["future RPC wrapper", "live gate"], evidence_ids: [official-rpc, wrapper], reason: "Provider capability exists; Claudine implementation/activation do not." }
  - { profile_id: retained-rpc, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [rpc-macos-claudine], mechanism_ids: [rpc-idle-prompt], prerequisites: ["future RPC wrapper", "live gate"], evidence_ids: [official-rpc, wrapper], reason: "Capability documented; not implemented/activated." }
  - { profile_id: retained-rpc, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [rpc-linux-native], mechanism_ids: [rpc-steer], prerequisites: ["native/live tests"], evidence_ids: [official-rpc], reason: "No native runtime evidence." }
  - { profile_id: retained-rpc, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [rpc-linux-native], mechanism_ids: [rpc-idle-prompt], prerequisites: ["native/live tests"], evidence_ids: [official-rpc], reason: "No native runtime evidence." }
  - { profile_id: retained-rpc, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [rpc-linux-claudine], mechanism_ids: [rpc-steer], prerequisites: ["future wrapper", "native/live tests"], evidence_ids: [official-rpc, wrapper], reason: "Unimplemented/unverified." }
  - { profile_id: retained-rpc, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [rpc-linux-claudine], mechanism_ids: [rpc-idle-prompt], prerequisites: ["future wrapper", "native/live tests"], evidence_ids: [official-rpc, wrapper], reason: "Unimplemented/unverified." }
  - { profile_id: retained-rpc, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [rpc-windows-native], mechanism_ids: [rpc-steer], prerequisites: ["native/live tests"], evidence_ids: [official-rpc], reason: "Native pipes/delivery unverified; WSL is insufficient." }
  - { profile_id: retained-rpc, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [rpc-windows-native], mechanism_ids: [rpc-idle-prompt], prerequisites: ["native/live tests"], evidence_ids: [official-rpc], reason: "No native evidence." }
  - { profile_id: retained-rpc, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [rpc-windows-claudine], mechanism_ids: [rpc-steer], prerequisites: ["future wrapper", "native/live tests"], evidence_ids: [official-rpc, wrapper], reason: "Unimplemented/unverified." }
  - { profile_id: retained-rpc, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [rpc-windows-claudine], mechanism_ids: [rpc-idle-prompt], prerequisites: ["future wrapper", "native/live tests"], evidence_ids: [official-rpc, wrapper], reason: "Unimplemented/unverified." }
gaps:
  - { area: activation, detail: "Two scoped macOS deterministic-model RPC records now pass; production adapter and broader profile verification remain absent.", next_check: "Keep disabled until matching tests pass." }
  - { area: attachment, detail: "No ordinary-session peer endpoint; files/PIDs do not provide reachability.", next_check: "Recheck releases; otherwise managed launches only." }
  - { area: targeting, detail: "RPC has no expected session/operation guard.", next_check: "Test switch/new/fork races and fail closed." }
  - { area: receipts, detail: "Acceptance is not correlated model delivery; retry/limits/dedup are unknown.", next_check: "Test ordering, duplicate text/ids, disconnects, limits, and retries." }
  - { area: loop-rescue, detail: "Steer waits for generation and full tool batch; it cannot rescue a boundary that never arrives.", next_check: "Test bounded stalled-generation and long-tool fixtures." }
  - { area: interruption, detail: "Abort retains queues and replacement can fail after cancellation.", next_check: "Test cleanup, queues, races, and partial failure with approval." }
  - { area: transformations, detail: "Skills/templates and extensions can change interpretation.", next_check: "Test slash text, extensions, UI requests, and escaping." }
  - { area: OS, detail: "Only macOS passive evidence; Linux/native Windows absent.", next_check: "Run native passive checks and later disposable tests." }
  - { area: model_provenance, detail: "Model/effort are launcher-supplied; resolved execution metadata is not exposed here.", next_check: "Persist resolved metadata when backend exposes it." }
changes:
  - "Refreshed schema revision 1 pilot to revision 2 with launch profiles and complete case products."
  - "Added access, receipt, framing, compatibility, queue, and interruption records."
  - "Corrected stale claim: v0.84.4 AgentSession emits agent_settled and RPC forwards it; agent_end is lower-level."
requires_claudine_update: true
reason: "Safe support needs a new retained-RPC launch profile, guarded registration, resource preservation, native OS and disposable verification; current Claudine JSON sessions and ordinary Pi sessions are not steerable."
discovery_gaps: []
interface_inventory:
- disposition: included
  evidence_ids:
  - local-pi
  - official-home
  - wrapper
  id: profile-ordinary-cli
  profile_ids:
  - ordinary-cli
  reason: Ordinary Pi TUI or print/JSON invocation without a retained peer endpoint.
- disposition: included
  evidence_ids:
  - official-rpc
  - source-rpc
  id: profile-retained-rpc
  profile_ids:
  - retained-rpc
  reason: Pi child deliberately launched in RPC mode with controller-owned stdin/stdout.
- disposition: unknown
  evidence_ids:
  - official-rpc
  - source-rpc
  id: coverage-review-retained-rpc
  profile_ids:
  - retained-rpc
  reason: The existing profile does not cover other client launch modes. This migration does not establish that these combinations are impossible. Review interface ownership, lifetime, and discovery before expanding coverage; do not infer exclusion from current wrapper behavior.
receipt_observations:
- evidence_ids:
  - official-rpc
  - source-rpc
  - source-session
  mechanism_id: rpc-steer
  signal: '{"id":"id","type":"response","command":"steer","success":true|false} Initial receipt only; later processing and settlement have separate signals.'
  timing: early
- evidence_ids:
  - official-rpc
  - source-rpc
  - source-settled
  mechanism_id: rpc-idle-prompt
  signal: success after acceptance, queueing, or extension handling Initial receipt only; later processing and settlement have separate signals.
  timing: early
- evidence_ids:
  - official-rpc
  - source-rpc
  mechanism_id: rpc-abort-submit
  signal: Cancellation and replacement have separate outcomes. abort success after idle; separate prompt preflight response Abort acknowledgment proves idle, not replacement acceptance.
  timing: multi_phase

---

# Steering Research: Pi

## Overview

Pi 0.84.4 has a documented steering candidate only in a deliberately retained RPC child or an in-process SDK object. RPC `steer` preserves the running turn, but waits for the current assistant response and its complete tool-call batch before insertion. Ordinary TUI, print, and JSON sessions expose no documented peer-attachment channel.

Agent `codex` performed this passive refresh. `gpt-5.6-sol` and low effort are launcher-supplied because resolved execution metadata is not exposed to this session. The original passive refresh launched no live Pi session. A subsequent coordinator-run disposable fixture is recorded below; its limited passing results do not activate production adapters.

## Session discovery

Session JSONL identifies history, not liveness or the conversation currently loaded by a process. PIDs can be reused and do not distinguish provider session identity, helpers, or multiple conversations over one process lifetime. Retained RPC `get_state` adds session ID/file/name and activity, but the send request has no expected-session/operation guard. Claudine must register child creation identity and pipes at launch, then reconcile a fresh state immediately before sending.

## Non-interrupting delivery

`steer` enqueues in memory and success confirms admission/scheduling, not durable persistence or model incorporation. Queue and user-message events expose later progress without originating request IDs. Pi drains steering after the full tool batch; sequential remaining calls continue and parallel batches finish. A hung tool or endless token stream never reaches delivery. `follow_up` waits until tools and steering are exhausted, so it is next work, not active correction.

## Interruption fallback

After explicit user choice: identify the working session, optionally clear queues only under an explicit policy, abort, await idle, revalidate the same session, then submit a separate prompt. Abort retains queues. If abort succeeds but prompt fails, original work stays canceled and the replacement was not accepted. Automatic warnings must never use this path.

## Idle sessions

An expected idle session in a retained RPC child accepts `prompt` and starts a turn. Prompt success means preflight acceptance, queueing, or immediate extension handling. An ordinary completed one-shot exits; relaunching saved history is not steering its former process.

## Protocol details

RPC is LF-delimited UTF-8 JSON over child stdin/stdout. Responses and events interleave; command IDs correlate responses only. Pipe possession is the access boundary. No sender message ID, idempotency, duplicate suppression, queue persistence/expiry, or size limit was established, so ambiguous requests are never retried.

Steer expands skills/templates and rejects extension commands. Prompt may execute extension commands immediately, while input extensions may transform or consume it. A managed profile must preserve extensions, skills, templates, context, explicit settings, and trust behavior, with unattended extension-UI policy.

The refresh correction is confirmed in pinned v0.84.4: `agent_end` is a low-level run boundary and may precede retry, compaction recovery, or queued continuation. `AgentSession` emits `agent_settled` only after session-level continuation is exhausted, and RPC forwards session events. Settlement is still a later uncorrelated signal, not a per-message receipt.

## OS/version compatibility

Sniff found Pi 0.84.4 on macOS; installed source/help match tag commit `b79e4cc834970cca69daebffab7df1da7d1e52c4`. Linux and native Windows were not run; WSL would be Linux-side only. Read-only checks can verify installed protocol shapes but cannot prove delivery, cancellation cleanup, or race behavior.

## Disposable-test proposals

Using non-focusing isolated RPC children on each native OS, test working/idle delivery, conversation identity, streaming, long and multi-tool batches, follow-ups, duplicate/concurrent writes, disconnects, queue clearing/restart, transformations/extensions/UI, session replacement races, and `agent_end` versus `agent_settled` across retry/compaction. Separately test abort during generation/tools, child cleanup, retained queues, and failed replacement. These are proposals, not verification.

## Claudine integration

Prefer a Claudine-owned retained RPC child registered at launch: `steer` for working state and `prompt` for idle. Report accepted, queued, observed insertion, and settled separately. Current Claudine uses one-shot JSON and cannot upgrade existing sessions. There is no verified fallback when RPC is unavailable: warn that ordinary sessions are unreachable and terminal-keystroke injection is not a stable protocol. Interruption remains manual-only.

## Gaps

Remaining blockers are production-profile live verification, absent ordinary attach, mutable unguarded target, incomplete correlation/idempotency/limits, non-immediate batch boundary, unverified abort cleanup, transformations, and missing Linux/native-Windows evidence.

## Changelog

Revision 2 adds stable profiles, full Cartesian cases, receipts, access, compatibility, and interruption partial failures. It corrects the stale pilot by recording pinned `agent_settled` definition/emission and RPC forwarding, distinct from `agent_end`.

## Sources

- [Pi](https://pi.dev/)
- [RPC mode](https://pi.dev/docs/latest/rpc)
- [SDK](https://pi.dev/docs/latest/sdk)
- [Sessions](https://pi.dev/docs/latest/sessions)
- [v0.84.4 RPC source](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/modes/rpc/rpc-mode.ts)
- [v0.84.4 AgentSession](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session.ts)
- [v0.84.4 agent loop](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/agent/src/agent-loop.ts)


## Revision 3 Contract Backfill

Receipt timing, interface inventory, and case-specific discovery gaps were added
from the existing evidence on 2026-09-08. No new provider observation or live test
was performed. Unexamined profile combinations remain unknown, not unsupported.
The original fleet model/effort provenance above describes the research run;
this deterministic contract migration is a separate coordinator edit.


## First Disposable Verification Pass

On 2026-09-08, real Pi 0.84.4 RPC passed the opt-in Rust harness on native macOS,
using a deterministic local model registered by an extension. The test retained
normal resource discovery in an isolated project; it passed context, skill, and
template sentinels without resource-disabling flags. Full assertions and sanitized
event ordering are in [the verification artifact](../../../features/2026-09-08-steering/verification/pi-macos-0.84.4.json).

Steering was acknowledged while a tool remained blocked and reached the next
model request only after both tools completed. Duplicate correlation IDs did not
suppress duplicate user messages. Extension-handled input returned success without
a model invocation. An extension confirmation stayed pending without an answer.
The harness confirmed that new_session changes identity in the same process; it
did not test concurrent switch/submit races.

These two passing records apply only to this fixture, OS, and version. Arbitrary
extensions, cloud providers, generation stalls, queue persistence, disconnects,
abort cleanup, concurrent senders, and native Linux/Windows remain unverified.
No production adapter or fallback was enabled.


## Abort and Input-Disconnect Verification

The [second disposable pass](../../../features/2026-09-08-steering/verification/pi-macos-0.84.4-failures.json)
passed four failure scenarios on Pi 0.84.4/macOS using the same local model and
cooperative tool fixture. Abort canceled both waiting tools. Without prior queue
clearing, the steering text was saved once in conversation history but was not
processed by the model until an explicit next prompt. Clearing before abort
prevented both persistence and later consumption. A malformed replacement was
rejected separately while the session remained idle.

Closing stdin while tools were held exited Pi with code zero. In both the
acknowledgment-observed and not-waited-for cases, steering was neither consumed
nor saved in the session transcript. The latter case is not proof that the
provider had not accepted it: the experiment drained one acknowledgment afterward.
This is input EOF, not a full-duplex connection failure or a process/host crash.

Claudine must keep owned RPC input open through settlement. Abort is not message
withdrawal; clearing pending messages is an explicit separate action, and a later
turn may consume steering already moved into history. Acceptance and successful
process exit establish neither model consumption nor durable storage. These
regression results do not enable a production adapter.

## Controlled Switch and Provider-Kill Verification

The [third disposable pass](../../../features/2026-09-08-steering/verification/pi-macos-0.84.4-switch-crash.json)
held a session switch in a fixture extension. The old session remained observable,
and steering was acknowledged while the switch was pending. Completing the
switch discarded that steering without saving it or sending it to the model.
By contrast, steering submitted after the switch reached the new session once.
Killing Pi after acknowledgment during a held tool batch also lost queued text
before transcript persistence or model consumption.

These exact macOS 0.84.4 outcomes require coordination of session changes and
submissions. State snapshots are not atomic target guards. A wrapper cannot
serialize independent extension actions merely by locking its own requests.
The controlled interleaving is not exhaustive race testing, and provider kill
does not establish host-failure or restart behavior. No production adapter is
activated by these additional records.

## Built-In Bash Cleanup Verification

The [external-process experiment](../../../features/2026-09-08-steering/verification/pi-macos-0.84.4-bash-cleanup.json)
uses the real built-in bash tool with a marked external process and a deterministic
local model. After a 750 ms observation delay, normal RPC abort had removed the
process, while SIGKILL of Pi had left it alive. The fixture then removed and
verified absence of that owned process. This distinguishes provider cancellation
from cleanup supplied by the test harness after provider failure.

Context, skill, and template discovery remained enabled and observable. This
scope covers one exec-replaced subprocess on macOS 0.84.4; additional descendants,
detached processes, native Windows/Linux, and cloud inference remain untested.
Managed execution therefore needs its own process-ownership cleanup guarantee,
independent of a provider's normal abort handling.
