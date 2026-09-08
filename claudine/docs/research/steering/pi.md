---
$schema: ./_schema.yaml
schema_revision: 1
provider: pi
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
  - "Pi 0.84.4 installed on macOS"
  - "earendil-works/pi tag v0.84.4 (commit b79e4cc834970cca69daebffab7df1da7d1e52c4)"
access_findings:
  - { mechanism_id: rpc_steer, os: macos, status: setup_required, prerequisite: "Pi must have been launched as a managed `pi --mode rpc` child and the sender must retain that child's stdin/stdout pipes; there is no documented attach endpoint for an already-open process.", applies_to_existing_sessions: no, evidence_ids: [E_RPC_DOC, E_RPC_MODE, E_LOCAL_HELP] }
  - { mechanism_id: rpc_steer, os: linux, status: setup_required, prerequisite: "Launch and retain a managed `pi --mode rpc` child; native Linux was not run in this pass.", applies_to_existing_sessions: no, evidence_ids: [E_RPC_DOC, E_RPC_MODE] }
  - { mechanism_id: rpc_steer, os: windows, status: setup_required, prerequisite: "Launch and retain a managed `pi --mode rpc` child with binary stdin/stdout JSONL handling; native Windows was not run in this pass.", applies_to_existing_sessions: no, evidence_ids: [E_RPC_DOC, E_RPC_MODE] }
  - { mechanism_id: rpc_prompt, os: macos, status: setup_required, prerequisite: "The target conversation must already be loaded in a managed RPC child whose pipes the sender owns.", applies_to_existing_sessions: no, evidence_ids: [E_RPC_DOC, E_RPC_MODE] }
  - { mechanism_id: rpc_prompt, os: linux, status: setup_required, prerequisite: "The target conversation must already be loaded in a managed RPC child whose pipes the sender owns; native Linux remains untested.", applies_to_existing_sessions: no, evidence_ids: [E_RPC_DOC, E_RPC_MODE] }
  - { mechanism_id: rpc_prompt, os: windows, status: setup_required, prerequisite: "The target conversation must already be loaded in a managed RPC child whose pipes the sender owns; native Windows remains untested.", applies_to_existing_sessions: no, evidence_ids: [E_RPC_DOC, E_RPC_MODE] }
delivery_states:
  - mechanism_id: rpc_steer
    states: [accepted, queued, delivered, refused, unknown]
    observable_by_external_sender: partial
    correlation: "The optional RPC command `id` correlates only the immediate response. `queue_update` exposes full pending text arrays and `message_start`/`message_end` show later user-message insertion, but they carry no originating command ID; duplicate text also makes text matching ambiguous. There is no separately named consumed-by-model state."
    evidence_ids: [E_RPC_DOC, E_RPC_MODE, E_AGENT_SESSION]
  - mechanism_id: rpc_prompt
    states: [accepted, queued, delivered, refused, unknown]
    observable_by_external_sender: partial
    correlation: "The command ID correlates the prompt preflight response. A busy prompt with streamingBehavior may be queued; a nonbusy prompt starts processing. Later lifecycle events are not correlated to that command ID, and `success: true` does not establish model incorporation."
    evidence_ids: [E_RPC_DOC, E_RPC_MODE, E_AGENT_SESSION]
evidence:
  - id: E_RPC_DOC
    method: official_docs
    location: "https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/docs/rpc.md"
    version: "v0.84.4 / b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "RPC is LF-delimited JSON over child stdin/stdout; steer, follow_up, prompt, abort, clear_queue, state, queue modes, responses, and lifecycle events are documented."
    limitations: "Documentation does not prove live delivery on this host or cross-OS behavior, and does not expose an attach endpoint."
  - id: E_AGENT_LOOP
    method: source_code
    location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/agent/src/agent-loop.ts"
    version: "b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "Steering is polled only after an assistant response and its entire tool-call batch complete, then inserted before the next LLM request; follow-ups drain only when the agent would otherwise stop."
    limitations: "Source inspection is not a disposable delivery test."
  - id: E_AGENT_CORE
    method: source_code
    location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/agent/src/agent.ts"
    version: "b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "steer and followUp enqueue in-memory messages; all drains the full FIFO queue, one-at-a-time drains its head; abort signals the current run and does not itself clear either queue."
    limitations: "The revision-1 schema cannot represent queue durability, drain cardinality, or abort-with-queue continuation as structured fields."
  - id: E_AGENT_SESSION
    method: source_code
    location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session.ts"
    version: "b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "AgentSession records pending queue text, expands skill/template syntax in direct steer/followUp, rejects registered extension commands there, allows literal extension sendUserMessage with expansion disabled, and replaces sessions through runtime APIs."
    limitations: "The literal sendUserMessage option is available to in-process extension code, not as an RPC steer option. Input extension handlers can also handle or transform RPC prompts."
  - id: E_RPC_MODE
    method: source_code
    location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/modes/rpc/rpc-mode.ts"
    version: "b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "RPC steer/follow_up target the process's mutable current AgentSession without a session or active-operation guard; response IDs correlate commands, while session replacement rebinds the same RPC connection."
    limitations: "No concurrent-command or stale-target experiment was run."
  - id: E_RUNTIME
    method: source_code
    location: "https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session-runtime.ts"
    version: "b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "new_session and switch_session abort and dispose the old AgentSession, create a replacement, and rebind RPC; old in-memory queues are not copied to the new session."
    limitations: "Exact behavior under concurrently pipelined steer and session-switch commands was not tested."
  - id: E_SDK_DOC
    method: official_docs
    location: "https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/docs/sdk.md"
    version: "v0.84.4 / b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "The in-process AgentSession SDK exposes prompt, steer, followUp, abort, queues, and event subscriptions; prompt preflight acceptance is distinct from full-run completion."
    limitations: "SDK references address the AgentSession object held by the embedding process and do not establish access to arbitrary Pi processes."
  - id: E_SESSION_DOC
    method: official_docs
    location: "https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/docs/sessions.md"
    version: "v0.84.4 / b79e4cc834970cca69daebffab7df1da7d1e52c4"
    observed_on: 2026-09-08
    claim: "Pi persists conversation trees as per-project JSONL session files under the Pi agent directory."
    limitations: "A persisted file is history, not proof of a live process, current loaded conversation, busy/idle state, or reachable RPC pipe."
  - id: E_LOCAL_HELP
    method: local_inspection
    location: "`pi --version`, `pi --help`, installed package metadata, and installed dist/docs under /Users/ken/.bun/install/global/node_modules/@earendil-works/pi-coding-agent"
    version: "0.84.4 on macOS"
    observed_on: 2026-09-08
    claim: "The installed roster-authoritative Pi binary is @earendil-works/pi-coding-agent 0.84.4 and advertises text, json, and rpc modes plus session selectors."
    limitations: "No running session was messaged, interrupted, resumed, or loaded during inspection."
  - id: E_CLAUDINE_WRAP
    method: source_code
    location: "claudine/cli/src/commands/wrap/profile/pi.rs"
    version: "workspace state on 2026-09-08"
    observed_on: 2026-09-08
    claim: "Claudine currently sends an initial prompt on stdin to Pi's `-p --mode json` one-shot profile and uses --session-id for relaunch; it does not launch or retain RPC mode."
    limitations: "Workspace source is evidence about Claudine integration, not provider runtime delivery."
discovery:
  - { id: D_MAC_NATIVE, os: macos, origin: native, method: process_inspection, locator: "Passively enumerate same-user Pi candidates and historical session files; only a controller-owned registration can map a candidate to retained RPC pipes.", identity_check: "For managed RPC, correlate child PID plus `get_state.sessionId` and sessionFile; for ordinary processes identity remains unavailable.", liveness_check: "Process liveness plus successful correlated get_state for managed RPC; files alone never establish liveness.", state_detection: "Managed RPC get_state.isStreaming; ordinary process state is unknown.", available_labels: [provider, pid, session_id, session_name, cwd, working_state], prerequisites: ["No discovery action may load or resume a session", "Managed controller registration for reachability"], evidence_ids: [E_RPC_DOC, E_SESSION_DOC, E_LOCAL_HELP] }
  - { id: D_MAC_CLAUDINE, os: macos, origin: claudine, method: claudine_registration, locator: "Future Claudine RPC launches can register child PID, pipe ownership, sessionId, sessionFile, and launch profile; current wrapper has no such launch.", identity_check: "Compare registration to correlated get_state immediately before send.", liveness_check: "Child handle plus successful get_state.", state_detection: "get_state.isStreaming, with compaction and tool lifecycle kept separate.", available_labels: [provider, pid, session_id, session_name, cwd, working_state, launch_profile], prerequisites: ["Future managed-RPC wrapper and per-user registration"], evidence_ids: [E_RPC_DOC, E_CLAUDINE_WRAP] }
  - { id: D_LINUX_NATIVE, os: linux, origin: native, method: process_inspection, locator: "Same design as D_MAC_NATIVE; no native Linux observation was made.", identity_check: "Managed RPC get_state only; ordinary identity unknown.", liveness_check: "Process plus correlated get_state.", state_detection: "get_state.isStreaming for managed RPC only.", available_labels: [provider, pid, session_id, session_name, cwd, working_state], prerequisites: ["Managed RPC registration", "Native Linux verification"], evidence_ids: [E_RPC_DOC, E_SESSION_DOC] }
  - { id: D_LINUX_CLAUDINE, os: linux, origin: claudine, method: claudine_registration, locator: "Future Claudine-managed RPC registration; current wrapper is JSON mode.", identity_check: "Registration plus correlated get_state.", liveness_check: "Child handle plus get_state.", state_detection: "get_state.isStreaming for managed RPC only.", available_labels: [provider, pid, session_id, session_name, cwd, working_state, launch_profile], prerequisites: ["Future managed-RPC wrapper", "Native Linux verification"], evidence_ids: [E_RPC_DOC, E_CLAUDINE_WRAP] }
  - { id: D_WINDOWS_NATIVE, os: windows, origin: native, method: process_inspection, locator: "Same-user process candidates and historical files are hints; no native Windows observation was made.", identity_check: "Managed RPC get_state only; ordinary identity unknown.", liveness_check: "Owned child plus get_state; session files alone are insufficient.", state_detection: "get_state.isStreaming for managed RPC only.", available_labels: [provider, pid, session_id, session_name, cwd, working_state], prerequisites: ["Managed RPC registration", "Native Windows verification"], evidence_ids: [E_RPC_DOC, E_SESSION_DOC] }
  - { id: D_WINDOWS_CLAUDINE, os: windows, origin: claudine, method: claudine_registration, locator: "Future Claudine-managed RPC child registration; current wrapper is JSON mode.", identity_check: "Registration plus correlated get_state.", liveness_check: "Child handle plus get_state.", state_detection: "get_state.isStreaming for managed RPC only.", available_labels: [provider, pid, session_id, session_name, cwd, working_state, launch_profile], prerequisites: ["Future managed-RPC wrapper", "Native Windows verification"], evidence_ids: [E_RPC_DOC, E_CLAUDINE_WRAP] }
mechanisms:
  - id: rpc_steer
    interface_status: documented
    transport: stdio
    conversation_effect: preserve_running_turn
    delivery_boundary: unknown
    acknowledgment: queued
    destination: "The current mutable AgentSession loaded inside the specific managed RPC child; the request carries no sessionId or expected operation ID."
    authentication: "No protocol credential. Access control is exclusive ownership/protection of the child process stdin/stdout handles and any Claudine registration; do not expose them across OS users."
    startup_requirements: ["Launch `pi --mode rpc` before the session needs steering", "Retain exclusive bidirectional pipes", "Load the intended session at launch or through an explicitly serialized switch", "Verify get_state sessionId and isStreaming immediately before steering"]
    request_format: "Strict LF-delimited JSON object: {\"id\":\"sender-id\",\"type\":\"steer\",\"message\":\"...\"}. Images are optional. Direct steer expands /skill and prompt-template syntax and rejects registered extension commands; no literal/raw flag exists on the RPC steer command."
    response_format: "LF-delimited {type:\"response\", command:\"steer\", success:true, id?} after enqueue, or a correlated error before acceptance. queue_update and later message events are asynchronous and lack the command ID."
    long_tool_behavior: "Version 0.84.4 finishes the complete current assistant tool-call batch before draining steering. Sequential calls not yet started still run; parallel calls are prepared/executed as a batch. Delivery therefore cannot rescue a hung tool and can be delayed indefinitely by it."
    interruption_effects: "None. steer does not signal abort, cancel token generation, or skip remaining tool calls. Separate abort ends the active run and waits for idle; if queues remain, Pi may continue them, so interruption intent requires an explicit clear/abort/revalidate/submit sequence and has partial-failure states."
    ordering: "FIFO within the in-memory steering queue. all drains every pending message together at one boundary; one-at-a-time (default) drains one per completed assistant turn. Cross-command ordering during concurrent RPC writes/session replacement is unverified."
    duplicate_handling: "No idempotency or duplicate suppression is documented. RPC IDs correlate responses but are not message IDs; replay can enqueue duplicate text."
    cancellation: "clear_queue removes both queue classes and returns their text. abort alone leaves queued messages eligible to continue. Session replacement disposes the old AgentSession; its in-memory queues are not transferred, but concurrent transition behavior is unverified."
    limits: "No message-size, queue-size, retention-time, or expiration limit found. Queues are process memory and are not established as restart-persistent. Direct steer can transform /skill and template-looking text."
    evidence_ids: [E_RPC_DOC, E_AGENT_LOOP, E_AGENT_CORE, E_AGENT_SESSION, E_RPC_MODE, E_RUNTIME]
  - id: rpc_prompt
    interface_status: documented
    transport: stdio
    conversation_effect: preserve_running_turn
    delivery_boundary: next_turn
    acknowledgment: accepted
    destination: "The current mutable AgentSession in the managed RPC child; prompt has no target identity field."
    authentication: "Same retained-pipe boundary as rpc_steer."
    startup_requirements: ["Managed RPC child", "Immediate get_state identity check", "For busy delivery, streamingBehavior must explicitly be steer or followUp"]
    request_format: "Strict LF-delimited JSON prompt. Default prompt processing may execute a registered extension command immediately, allows input extensions to handle/transform text, and expands skills/templates. RPC exposes no expandPromptTemplates=false/raw-text member."
    response_format: "Correlated preflight response. success true combines accepted, queued, and handled-immediately outcomes; later failures use the event stream."
    long_tool_behavior: "With streamingBehavior=steer it inherits the complete-tool-batch boundary. With followUp it waits until the agent otherwise stops. When idle it starts a new run in the loaded conversation."
    interruption_effects: "No implicit abort."
    ordering: "Busy behavior inherits the selected queue mode; idle prompts are serialized by the owning client, but concurrent command ordering was not verified."
    duplicate_handling: "No idempotency guarantee; command IDs do not suppress replay."
    cancellation: "Queued variants use clear_queue. Once an idle prompt starts, abort is separate."
    limits: "Acceptance cannot distinguish extension-handled, transformed, queued, or ordinary prompt processing. Literal manual-message preservation is not guaranteed through this RPC surface."
    evidence_ids: [E_RPC_DOC, E_AGENT_SESSION, E_RPC_MODE]
compatibility:
  - { mechanism_id: rpc_steer, os: macos, versions_verified: [], documented_version_bounds: "Documented and source-derived for exactly v0.84.4; no historical or future bound claimed.", read_only_check: "Verify binary version 0.84.4, launch profile registration, child ownership, then correlate get_state sessionId/isStreaming immediately before send without switching sessions.", success_criteria: "Expected version/profile and exact loaded session match; get_state succeeds over retained pipes and reports isStreaming=true.", failure_behavior: "Mark unavailable; do not resume, switch, attach, or fall back to stdin/TUI injection.", evidence_ids: [E_RPC_DOC, E_LOCAL_HELP] }
  - { mechanism_id: rpc_steer, os: linux, versions_verified: [], documented_version_bounds: "v0.84.4 source only; native Linux unverified.", read_only_check: "Same managed-child version/profile/get_state check on native Linux.", success_criteria: "Exact loaded session and busy state confirmed.", failure_behavior: "Unavailable without mutation.", evidence_ids: [E_RPC_DOC] }
  - { mechanism_id: rpc_steer, os: windows, versions_verified: [], documented_version_bounds: "v0.84.4 source only; native Windows unverified.", read_only_check: "Same managed-child version/profile/get_state check using binary-safe pipe handling on native Windows.", success_criteria: "Exact loaded session and busy state confirmed.", failure_behavior: "Unavailable without mutation.", evidence_ids: [E_RPC_DOC] }
  - { mechanism_id: rpc_prompt, os: macos, versions_verified: [], documented_version_bounds: "Documented and source-derived for exactly v0.84.4.", read_only_check: "Version/profile/get_state check; require isStreaming=false and exact session identity for an idle new-turn prompt.", success_criteria: "Exact current session and idle state confirmed.", failure_behavior: "Do not send or silently choose busy streaming behavior.", evidence_ids: [E_RPC_DOC, E_LOCAL_HELP] }
  - { mechanism_id: rpc_prompt, os: linux, versions_verified: [], documented_version_bounds: "v0.84.4 source only; native Linux unverified.", read_only_check: "Native Linux managed-child version/profile/get_state check.", success_criteria: "Exact current session and idle state confirmed.", failure_behavior: "Unavailable.", evidence_ids: [E_RPC_DOC] }
  - { mechanism_id: rpc_prompt, os: windows, versions_verified: [], documented_version_bounds: "v0.84.4 source only; native Windows unverified.", read_only_check: "Native Windows managed-child version/profile/get_state check.", success_criteria: "Exact current session and idle state confirmed.", failure_behavior: "Unavailable.", evidence_ids: [E_RPC_DOC] }
verification: []
cases:
  - { os: macos, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_MAC_NATIVE], mechanism_ids: [rpc_steer], prerequisites: ["Only a separately managed RPC launch is reachable; ordinary TUI is not"], evidence_ids: [E_RPC_DOC, E_LOCAL_HELP], reason: "Revision 1 conflates ordinary TUI and managed RPC profiles. RPC supports queued steering, but no attach path to the ordinary TUI and no live test were established." }
  - { os: macos, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_MAC_NATIVE], mechanism_ids: [rpc_prompt], prerequisites: ["Managed RPC profile with exact loaded session"], evidence_ids: [E_RPC_DOC], reason: "Managed RPC can prompt its idle current session, while an ordinary idle TUI has no documented external channel." }
  - { os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_MAC_CLAUDINE], mechanism_ids: [rpc_steer], prerequisites: ["Future Claudine managed-RPC launch"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current Claudine interactive behavior does not register retained RPC pipes; a future managed profile is a different launch." }
  - { os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_MAC_CLAUDINE], mechanism_ids: [rpc_prompt], prerequisites: ["Future Claudine managed-RPC launch"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Idle delivery is available only inside a managed RPC profile, which current Claudine does not create." }
  - { os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_MAC_NATIVE], mechanism_ids: [rpc_steer], prerequisites: ["Must be `--mode rpc`, not -p or --mode json"], evidence_ids: [E_RPC_DOC, E_LOCAL_HELP], reason: "RPC is a candidate; ordinary print/JSON processes expose no retained command channel. The broad case cannot express the profile split." }
  - { os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_MAC_NATIVE], mechanism_ids: [rpc_prompt], prerequisites: ["Long-lived managed RPC child"], evidence_ids: [E_RPC_DOC], reason: "A managed RPC child can remain idle; one-shot print/JSON processes exit and cannot." }
  - { os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_MAC_CLAUDINE], mechanism_ids: [rpc_steer], prerequisites: ["New managed-RPC wrapper profile"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Claudine currently launches -p --mode json, so its stdin is initial prompt data rather than an RPC control channel." }
  - { os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_MAC_CLAUDINE], mechanism_ids: [rpc_prompt], prerequisites: ["New long-lived managed-RPC wrapper profile"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current one-shot wrapper exits; only a future RPC child could remain reachable while idle." }
  - { os: linux, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_LINUX_NATIVE], mechanism_ids: [rpc_steer], prerequisites: ["Managed RPC profile", "Native Linux verification"], evidence_ids: [E_RPC_DOC], reason: "Same profile ambiguity as macOS, with no native Linux run." }
  - { os: linux, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_LINUX_NATIVE], mechanism_ids: [rpc_prompt], prerequisites: ["Managed RPC profile", "Native Linux verification"], evidence_ids: [E_RPC_DOC], reason: "Ordinary TUI reachability is unknown; managed RPC idle prompting remains unverified on Linux." }
  - { os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_LINUX_CLAUDINE], mechanism_ids: [rpc_steer], prerequisites: ["Future managed-RPC wrapper", "Native Linux verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current Claudine launch is not RPC and Linux was not run." }
  - { os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_LINUX_CLAUDINE], mechanism_ids: [rpc_prompt], prerequisites: ["Future managed-RPC wrapper", "Native Linux verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Requires a future retained child profile and verification." }
  - { os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_LINUX_NATIVE], mechanism_ids: [rpc_steer], prerequisites: ["--mode rpc rather than print/JSON", "Native Linux verification"], evidence_ids: [E_RPC_DOC], reason: "The schema combines reachable RPC and unreachable one-shot profiles." }
  - { os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_LINUX_NATIVE], mechanism_ids: [rpc_prompt], prerequisites: ["Long-lived managed RPC child", "Native Linux verification"], evidence_ids: [E_RPC_DOC], reason: "One-shot processes cannot remain idle; RPC can by design but is not live-tested." }
  - { os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_LINUX_CLAUDINE], mechanism_ids: [rpc_steer], prerequisites: ["New managed-RPC wrapper", "Native Linux verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current wrapper uses JSON mode and offers no external control channel." }
  - { os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_LINUX_CLAUDINE], mechanism_ids: [rpc_prompt], prerequisites: ["New managed-RPC wrapper", "Native Linux verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current wrapper exits; future managed RPC requires verification." }
  - { os: windows, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_WINDOWS_NATIVE], mechanism_ids: [rpc_steer], prerequisites: ["Managed RPC profile", "Native Windows verification"], evidence_ids: [E_RPC_DOC], reason: "Ordinary TUI has no documented external endpoint; RPC pipe behavior was not run on native Windows." }
  - { os: windows, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_WINDOWS_NATIVE], mechanism_ids: [rpc_prompt], prerequisites: ["Managed RPC profile", "Native Windows verification"], evidence_ids: [E_RPC_DOC], reason: "Managed idle prompting and ordinary TUI reachability differ and are not representable in one case." }
  - { os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_WINDOWS_CLAUDINE], mechanism_ids: [rpc_steer], prerequisites: ["Future managed-RPC wrapper", "Native Windows verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current Claudine profile is not RPC; Windows is untested." }
  - { os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_WINDOWS_CLAUDINE], mechanism_ids: [rpc_prompt], prerequisites: ["Future managed-RPC wrapper", "Native Windows verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Requires future profile and live evidence." }
  - { os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_WINDOWS_NATIVE], mechanism_ids: [rpc_steer], prerequisites: ["--mode rpc rather than print/JSON", "Native Windows verification"], evidence_ids: [E_RPC_DOC], reason: "Broad case mixes distinct profiles; Windows pipe/framing behavior is unverified." }
  - { os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_WINDOWS_NATIVE], mechanism_ids: [rpc_prompt], prerequisites: ["Long-lived managed RPC child", "Native Windows verification"], evidence_ids: [E_RPC_DOC], reason: "One-shot modes exit; RPC can remain idle but lacks native Windows verification." }
  - { os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_WINDOWS_CLAUDINE], mechanism_ids: [rpc_steer], prerequisites: ["New managed-RPC wrapper", "Native Windows verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current Claudine JSON profile does not expose RPC commands." }
  - { os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_WINDOWS_CLAUDINE], mechanism_ids: [rpc_prompt], prerequisites: ["New managed-RPC wrapper", "Native Windows verification"], evidence_ids: [E_CLAUDINE_WRAP, E_RPC_DOC], reason: "Current one-shot profile exits; future RPC support remains unverified." }
gaps:
  - { area: "Live activation", detail: "No disposable-session delivery record exists, so no Pi mechanism is eligible for activation.", next_check: "Run non-focusing disposable RPC tests on each native OS and exact version/profile after the schema is revised." }
  - { area: "Launch profiles (previously identified)", detail: "Revision 1 still merges ordinary TUI, one-shot JSON/print, managed RPC, and in-process SDK launches, forcing broad cases to unknown.", next_check: "Adopt explicit provider launch-profile IDs before generation." }
  - { area: "Acknowledgment lifecycle (previously identified)", detail: "RPC success confirms enqueue/preflight but lifecycle events lack request correlation and no state proves model incorporation.", next_check: "Represent admission, persistence, queue drain, conversation insertion, and model incorporation separately." }
  - { area: "Full tool-batch delivery boundary (new from Pi)", detail: "Pi steering waits for the complete assistant tool-call batch, not the next individual tool boundary; revision 1 offers only next_tool_boundary and cannot say that queued steering leaves every remaining call running.", next_check: "Add batch boundary and pending-tool scheduling effects to the contract; test sequential, parallel, permission-delayed, and hung tools." }
  - { area: "Abort and queue coupling (new from Pi)", detail: "abort does not clear queued work and can be followed by automatic queue continuation. clear_queue and abort are separate, and replacement submission can still fail.", next_check: "Represent queue retention across interruption and the clear/abort/revalidate/submit partial outcomes." }
  - { area: "Mutable implicit target (new from Pi)", detail: "steer/follow_up carry neither session identity nor expected active-operation identity; they address whichever AgentSession is current when handled. RPC request IDs do not guard target freshness.", next_check: "Require serialized ownership plus a just-in-time get_state identity check, and test pipelined session replacement races." }
  - { area: "Message interpretation (new from Pi)", detail: "RPC direct steer expands skill/template syntax; RPC prompt may execute extension commands and input extensions may handle or transform it. No RPC raw/literal option was found, although in-process sendUserMessage defaults expansion off.", next_check: "Add message-interpretation metadata and test literal strings beginning with slash syntax and active input extensions before defining an escaping policy." }
  - { area: "Queue identity, persistence, and retry", detail: "Queue entries have text and FIFO position but no stable message ID; duplicate suppression, safe retry, size limits, expiration, and crash persistence are absent or unverified.", next_check: "Test duplicate command IDs/text, disconnect-before-response, restart, limits, and retry behavior." }
  - { area: "Session replacement queues", detail: "Source shows old queues are not transferred when new/switch replaces AgentSession, but concurrent queued commands and response ordering were not traced exhaustively.", next_check: "Test queued steer followed by switch/new_session, including pipelined and extension-cancelled transitions." }
  - { area: "Discovery and attach", detail: "History files and process inspection cannot identify a live loaded conversation or recover another process's private pipes; no documented attach/listener mechanism was found.", next_check: "Use managed-launch registration for future sessions and keep ordinary existing sessions unavailable unless Pi adds an authenticated attach API." }
  - { area: "OS evidence", detail: "Only installed macOS help/source was inspected; Linux and native Windows transport behavior is unverified.", next_check: "Run native Linux and Windows managed-RPC compatibility and delivery tests without focusing windows." }
  - { area: "Tool approval versus delivery", detail: "Extensions can implement tool/input gates, but queue admission or message insertion says nothing about later tool authorization.", next_check: "Keep delivery state separate from extension/tool approval state and test both independently." }
  - { area: "RPC migration lifecycle", detail: "Claudine's current Pi stream parser treats agent_end as completion, while RPC remains alive and can continue through retry, compaction recovery, or queued follow-up work until agent_settled.", next_check: "Test JSON/RPC event parity, route responses separately from events, and define agent_settled, EOF, shutdown, and final-success boundaries." }
  - { area: "RPC feature and UI policy", detail: "A managed RPC profile must retain user-enabled extensions, skills, prompt templates, and context files. Enabled extensions may require unattended extension_ui_response handling; blanket --no-* disabling is not acceptable. Project approval is separate.", next_check: "Test enabled resources, extension input transformation and UI requests, explicit provider settings, and --no-approve independently." }
  - { area: "Token-loop rescue", detail: "Steering is polled only after the current LLM response and complete tool batch, so a token-generation loop that never ends cannot consume it.", next_check: "Verify with a bounded disposable streaming fixture; automatic intervention must retain the existing hard stop." }
changes:
  - "Initial Pi steering pilot under schema revision 1."
  - "Recorded documented RPC steer, follow-up, prompt, abort, clear-queue, queue-mode, and event semantics for pinned v0.84.4."
  - "Added Pi-specific gaps for complete tool-batch scheduling, abort-retained queues, mutable implicit targets, and message interpretation."
requires_claudine_update: true
reason: "Pi has a documented candidate only when Claudine deliberately owns a long-lived RPC or in-process session. The current wrapper is one-shot JSON and cannot steer it. A future managed RPC profile must retain user-enabled resources, handle unattended extension UI, and adopt settled-session lifecycle semantics. Schema revision 1 also cannot safely express Pi's launch profiles, complete-tool-batch boundary, queue/interruption coupling, mutable current-session targeting, or message transformation. No mechanism has passed the required live activation gate."
---

# Pi steering research

## Overview

Pi 0.84.4 has a documented, non-interrupting steering queue in managed RPC and
in-process SDK sessions. Its meaning is narrower than “interrupt”: a steering
message waits until the current assistant response and its **entire tool-call
batch** finish, then enters the same conversation before the next model request.
It does not stop generation, cancel a running tool, or skip remaining tools.

This was a passive pilot performed by Codex using `gpt-5.6-sol` with low
reasoning effort. It inspected the roster-authoritative Pi repository, installed
0.84.4 help/package/source, and Claudine's wrapper. It sent no messages, launched
no agent sessions, loaded no saved conversations, and performed no interruption.
Consequently `verification` is empty and activation remains blocked.

## Session discovery

Pi's JSONL session files provide durable conversation identity and history, but
they do not prove that a process is alive, that it currently has that conversation
loaded, or that its private stdin is reachable. Process inspection adds a PID,
not the current Pi session or its state. A reliable future integration therefore
needs Claudine to register a managed RPC child at launch and retain its pipe
handles. Immediately before sending, it must correlate `get_state.sessionId` and
the expected working/idle state. Discovery must never switch, resume, or create a
session merely to make it reachable.

The request itself has no session ID or expected operation ID. `steer` always
addresses the RPC process's mutable current `AgentSession`. This differs from a
provider protocol with an exact turn guard: a request racing `switch_session`,
`new_session`, fork, or extension-driven replacement could target changing state.
RPC command IDs correlate responses but do not make the destination race-safe.

## Non-interrupting delivery

`steer` appends an in-memory FIFO queue entry and returns a success response. Pi
then finishes all tool calls from the current assistant message. In sequential
mode, calls that have not started still execute; in parallel mode, the batch is
prepared and executed together. Only after `turn_end` does the loop drain
steering and place it before the next model call. The default one-at-a-time mode
drains one entry per completed assistant turn; all mode drains every pending
entry at the same boundary.

This preserves the current turn in the broad sense, but it has a scheduling
effect revision 1 cannot encode: every remaining tool call continues before the
message. A hung long tool delays delivery indefinitely. A model token loop that
never finishes the assistant response never reaches the polling boundary, so Pi
steering cannot be treated as immediate loop rescue. Queue removal plus
`message_start`/`message_end` can show conversation insertion, but neither proves
that a later provider request incorporated the message.

`follow_up` is a different intent. It drains only after the agent would otherwise
stop, after tools and steering are exhausted. It is queued next work, rather than
course correction of the current work.

## Interruption fallback

RPC `abort` signals the current run and waits for the session to become idle.
It is separate from both queue clearing and replacement input. Official docs
explicitly state that queued messages continue after abort when they remain.
Interactive Escape behavior is modeled as `clear_queue` before `abort`, with the
returned text restored to the editor.

For Claudine, an interruption fallback would need explicit user approval and a
multi-phase record: clear or retain queue intentionally, abort, observe idle,
revalidate the session identity, then submit input. Abort may succeed while the
replacement fails, and clearing may remove unrelated pending user work. This
cannot be used by automatic repetition warnings.

## Idle sessions

A managed RPC child can remain alive with `get_state.isStreaming=false`.
`prompt` then starts a new run in the currently loaded conversation. Calling
low-level steer while idle only queues; it does not itself start work, and later
continuation semantics are not a substitute for an explicit idle prompt intent.
An ordinary print/JSON process exits after completion. An idle ordinary TUI is
open for keyboard input, but no documented external RPC or attach channel was
found.

Starting another Pi invocation with `--session` or `--session-id` is transcript
resumption in another process, not delivery to the original live process.
`--session-id` can also create a missing session, making it unsafe as discovery
or an attachment test.

## Protocol details

RPC uses strict LF-delimited JSON on a child process's stdin/stdout. Each command
may carry an ID echoed by its response. A successful steer response establishes
enqueue, while `queue_update` exposes full pending steering/follow-up text arrays.
Those later events do not carry the request ID. There is no message idempotency
key, and replaying a command after a lost response may enqueue a duplicate.

The pipe is also the access boundary. Pi documents no socket, named pipe,
network listener, authentication token, or attach command for taking control of
an arbitrary ordinary process. The SDK methods act on the `AgentSession` object
owned by the embedding process; they are not a remote API to other Pi sessions.

Message text is not necessarily literal. Direct RPC `steer` and `follow_up`
expand skill commands and prompt templates, and reject registered extension
commands. RPC `prompt` may execute a registered extension command immediately;
input extensions can handle or transform text before skill/template expansion.
The in-process extension API `sendUserMessage` defaults expansion off, but RPC
does not expose that option. A message beginning with slash syntax therefore
needs explicit compatibility testing before Claudine can promise exact delivery.

## OS and version compatibility

The installed macOS Pi is 0.84.4 and matches tag `v0.84.4`, commit
`b79e4cc834970cca69daebffab7df1da7d1e52c4`. The documented transport is Node
child stdio and contains platform-aware newline guidance, but no native Linux or
Windows session was run. No historical compatibility bound was found. WSL was
not used and would not establish native Windows behavior.

On every OS, retained process handles must remain confined to the same OS user.
The provider protocol supplies no authentication of its own. Windows clients
must preserve JSON records over child pipes without treating console input or
ordinary TUI stdin as the RPC channel. macOS/Linux likewise gain no Unix socket
merely because RPC exists.

## Disposable-test proposals

After schema revision, launch non-focusing disposable RPC children under the
same OS user on macOS, Linux, and native Windows. Assert exact session identity,
enqueue response, queue update, drain, user-message insertion, and later model
context separately. Cover token generation, a long sequential tool, multiple
sequential tools, parallel tools, permission-delayed tools, a hung tool, and the
final-turn race. Confirm that steering does not skip any remaining call.

Test one-at-a-time and all ordering, duplicate text, repeated command IDs,
disconnect-before-response, queue clear, abort with retained queues, clear then
abort, replacement failure, and process restart. Race steer/follow_up against
new_session and switch_session while checking which session receives each
message. Include literal `/skill:`, prompt-template, and registered extension
command strings with and without input-transforming extensions. Capture only
sanitized JSONL fixtures and never focus a terminal window.

## Claudine integration

Claudine currently invokes Pi as `-p --mode json`, writes the initial prompt to
stdin, and resumes by launching another process with `--session-id`. That stdin
is initial prompt input, not RPC framing, and the process is not a retained idle
server. Steering therefore requires a distinct managed-RPC profile plus durable
per-user registration, exclusive command serialization, and immediate identity
revalidation. It cannot retrofit sessions already open in an ordinary TUI or
one-shot mode.

This is a managed-launch migration, not a direct swap from JSON output to RPC.
The controller must keep stdin open, separate correlated command responses from
asynchronous events, own shutdown/EOF, and answer extension UI requests safely.
It must use `agent_settled` for the point after retries, compaction recovery, and
queued follow-ups; Claudine's current Pi parser instead treats `agent_end` as
terminal, so final-success behavior needs explicit compatibility work.

The managed profile must retain user-enabled extensions, skills, prompt
templates, context files, and explicit provider settings. It must not carry over
the current generated `--no-extensions`, `--no-skills`,
`--no-prompt-templates`, or `--no-context-files` flags as a blanket workaround.
Enabled resources make input transformation and unattended
`extension_ui_request` handling part of the test matrix. Project trust and
`--no-approve` remain separate permission decisions and must not change
implicitly with transport.

The adapter must report enqueue rather than delivery, preserve queue mode, avoid
automatic retry after an ambiguous response, and distinguish message admission
from later tool permission waits. It also needs a defined literal-text policy;
provider-side skill, template, extension-command, and input-handler behavior must
not silently reinterpret a manual warning.

## Gaps

OpenCode and Codex already exposed the need for launch profiles, layered
acknowledgments, message correlation, target preconditions, and partial
interruption outcomes. Pi confirms those and adds four material gaps:

1. Its delivery boundary is the end of the complete tool-call batch, while the
   current schema can express only a vague next-tool boundary.
2. Abort retains queued work, so cancellation and queue disposition are coupled
   but separate operations.
3. Commands target a mutable current session without an expected session/turn
   guard, requiring sender-side serialization and a freshness check.
4. RPC can transform or execute command-like message text and offers no raw-text
   option, so the contract needs message-interpretation semantics.

Queue durability, expiry, size bounds, idempotency, concurrent session-switch
ordering, native Linux/Windows behavior, and ordinary-process attach remain
unresolved. RPC event parity, settled/EOF success semantics, enabled-resource
policy, and extension UI responses also require migration tests. These gaps are
independent of the mandatory live-test gate.

## Sources

- [Pi RPC documentation, v0.84.4](https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/docs/rpc.md)
- [Pi SDK documentation, v0.84.4](https://github.com/earendil-works/pi/blob/v0.84.4/packages/coding-agent/docs/sdk.md)
- [Agent loop at pinned commit](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/agent/src/agent-loop.ts)
- [Agent queue implementation at pinned commit](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/agent/src/agent.ts)
- [AgentSession at pinned commit](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session.ts)
- [RPC mode at pinned commit](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/modes/rpc/rpc-mode.ts)
- [Session runtime at pinned commit](https://github.com/earendil-works/pi/blob/b79e4cc834970cca69daebffab7df1da7d1e52c4/packages/coding-agent/src/core/agent-session-runtime.ts)

## Changelog

- 2026-09-08: Initial passive Pi pilot under schema revision 1.
