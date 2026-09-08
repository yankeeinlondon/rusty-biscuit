---
$schema: ./_schema.yaml
schema_revision: 2
provider: kimi
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
  - "Kimi Code CLI 0.28.1 installed on native macOS"
  - "MoonshotAI/kimi-code tag @moonshot-ai/kimi-code@0.28.1 (commit efacf0452d46f5dbd67499eabc053869495d5213)"
launch_profiles:
  - id: ordinary_cli
    description: "Ordinary shell TUI or one-shot `kimi --prompt`; no documented peer-control endpoint. Lifetime varies by launch mode: the TUI can remain open, while a prompt invocation exits."
    endpoint_scope: none
    lifetime: unknown
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [interactive, non_interactive]
    applicable_origins: [native, claudine]
    baseline: true
    startup_requirements: ["Normal `kimi` or `kimi --prompt` launch"]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: yes
    preserves_context: yes
    evidence_ids: [E_HELP, E_SESSIONS, E_CLI_SOURCE]
  - id: web_server
    description: "Long-lived `kimi web` REST/WebSocket server with session registry and prompt queue/steer API."
    endpoint_scope: externally_reachable
    lifetime: long_lived
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [native, claudine]
    baseline: false
    startup_requirements: ["Launch `kimi web` deliberately", "Retain the server bearer token", "Register the selected instance endpoint"]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: yes
    preserves_context: yes
    evidence_ids: [E_WEB_DOC, E_WEB_PROMPTS, E_WEB_REGISTRY, E_WEB_AUTH, E_CORE_STEER]
  - id: acp_server
    description: "Long-lived `kimi acp` multi-session JSON-RPC server over retained stdin/stdout."
    endpoint_scope: retained_stdio
    lifetime: while_client_open
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [native, claudine]
    baseline: false
    startup_requirements: ["Launch `kimi acp` deliberately", "The sender must own and retain stdin/stdout", "Complete ACP initialize and load/resume the target session"]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: unknown
    preserves_context: yes
    evidence_ids: [E_ACP_DOC, E_ACP_SERVER, E_ACP_SESSION]
access_findings:
  - { mechanism_id: web_steer, profile_id: web_server, os: macos, status: setup_required, prerequisite: "A live registered web server and readable bearer token are required; setup applies only to sessions hosted by that server.", applies_to_existing_sessions: no, evidence_ids: [E_WEB_REGISTRY, E_WEB_AUTH, E_WEB_PROMPTS] }
  - { mechanism_id: web_steer, profile_id: web_server, os: linux, status: setup_required, prerequisite: "Same server registration and bearer-token setup; native Linux was not run.", applies_to_existing_sessions: no, evidence_ids: [E_WEB_REGISTRY, E_WEB_AUTH, E_WEB_PROMPTS] }
  - { mechanism_id: web_steer, profile_id: web_server, os: windows, status: setup_required, prerequisite: "Same server registration and bearer-token setup; native Windows was not run.", applies_to_existing_sessions: no, evidence_ids: [E_WEB_REGISTRY, E_WEB_AUTH, E_WEB_PROMPTS] }
  - { mechanism_id: web_submit, profile_id: web_server, os: macos, status: setup_required, prerequisite: "A live registered web server, token, and exact session ID are required.", applies_to_existing_sessions: no, evidence_ids: [E_WEB_REGISTRY, E_WEB_AUTH, E_WEB_PROMPTS] }
  - { mechanism_id: web_submit, profile_id: web_server, os: linux, status: setup_required, prerequisite: "Same setup; native Linux was not run.", applies_to_existing_sessions: no, evidence_ids: [E_WEB_REGISTRY, E_WEB_AUTH, E_WEB_PROMPTS] }
  - { mechanism_id: web_submit, profile_id: web_server, os: windows, status: setup_required, prerequisite: "Same setup; native Windows was not run.", applies_to_existing_sessions: no, evidence_ids: [E_WEB_REGISTRY, E_WEB_AUTH, E_WEB_PROMPTS] }
  - { mechanism_id: acp_cancel_submit, profile_id: acp_server, os: macos, status: setup_required, prerequisite: "The sender must own the ACP child pipes and have loaded the exact session; ordinary CLI sessions cannot be attached.", applies_to_existing_sessions: no, evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { mechanism_id: acp_cancel_submit, profile_id: acp_server, os: linux, status: setup_required, prerequisite: "Same retained-pipe setup; native Linux was not run.", applies_to_existing_sessions: no, evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { mechanism_id: acp_cancel_submit, profile_id: acp_server, os: windows, status: setup_required, prerequisite: "Same retained-pipe setup; native Windows was not run.", applies_to_existing_sessions: no, evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { mechanism_id: acp_prompt, profile_id: acp_server, os: macos, status: setup_required, prerequisite: "The sender must own the ACP child and load/resume the exact idle session.", applies_to_existing_sessions: no, evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { mechanism_id: acp_prompt, profile_id: acp_server, os: linux, status: setup_required, prerequisite: "Same retained-pipe setup; native Linux was not run.", applies_to_existing_sessions: no, evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { mechanism_id: acp_prompt, profile_id: acp_server, os: windows, status: setup_required, prerequisite: "Same retained-pipe setup; native Windows was not run.", applies_to_existing_sessions: no, evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
delivery_states:
  - { mechanism_id: web_steer, states: [accepted, queued, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "Submit returns prompt_id/user_message_id and running or queued; steer echoes selected prompt_ids; prompt.steered and prompt.completed events correlate later stages, but no signal proves model incorporation.", evidence_ids: [E_WEB_PROMPTS, E_WEB_EVENTS] }
  - { mechanism_id: web_submit, states: [accepted, queued, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "The response and WebSocket events carry prompt_id; running means scheduled, not model-consumed, and queued is distinct from later tool permission holds.", evidence_ids: [E_WEB_PROMPTS, E_WEB_EVENTS] }
  - { mechanism_id: acp_cancel_submit, states: [accepted, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "Cancel is an unacknowledged notification; the later prompt request ID correlates only its terminal JSON-RPC response and session/update notifications carry sessionId, not the cancel request.", evidence_ids: [E_ACP_SERVER, E_ACP_SESSION] }
  - { mechanism_id: acp_prompt, states: [accepted, delivered, refused, unknown], observable_by_external_sender: partial, correlation: "JSON-RPC request ID correlates the terminal PromptResponse/error; session/update carries sessionId and tool IDs, with no separate admission receipt.", evidence_ids: [E_ACP_DOC, E_ACP_SESSION] }
receipt_guarantees:
  - { mechanism_id: web_steer, request_acceptance: confirmed, persistence: unknown, scheduling: confirmed, conversation_delivery: unknown, provider_signals: ["HTTP success envelope with steered=true and prompt_ids", "prompt.steered WebSocket event", "later prompt.completed"], correlation: message_id, evidence_ids: [E_WEB_PROMPTS, E_WEB_EVENTS], limitations: "The success response proves selected queued IDs were handed to core steering; it does not prove durable persistence or that a later model request consumed them." }
  - { mechanism_id: web_submit, request_acceptance: confirmed, persistence: unknown, scheduling: confirmed, conversation_delivery: unknown, provider_signals: ["HTTP PromptItem status running or queued", "prompt.submitted", "later prompt.completed"], correlation: message_id, evidence_ids: [E_WEB_PROMPTS, E_WEB_EVENTS], limitations: "Initial status distinguishes running from queued but does not prove model incorporation; request-envelope IDs and prompt IDs are distinct." }
  - { mechanism_id: acp_cancel_submit, request_acceptance: unknown, persistence: unknown, scheduling: unknown, conversation_delivery: unknown, provider_signals: ["session/cancel has no response", "subsequent session/prompt returns stopReason or error", "session/update stream"], correlation: unknown, evidence_ids: [E_ACP_SERVER, E_ACP_SESSION], limitations: "Cancellation can succeed silently and replacement submission can then fail; no initial cancellation acknowledgment exists." }
  - { mechanism_id: acp_prompt, request_acceptance: confirmed, persistence: unknown, scheduling: unknown, conversation_delivery: unknown, provider_signals: ["PromptResponse stopReason", "session/update stream", "JSON-RPC error"], correlation: request_id, evidence_ids: [E_ACP_DOC, E_ACP_SESSION], limitations: "The response is terminal rather than an initial admission acknowledgment and does not independently prove model incorporation." }
evidence:
  - { id: E_HELP, method: local_inspection, location: "Sanitized passive output of `sniff software agents --json`, `kimi --version`, and `kimi --help`; no fixture written", version: "0.28.1", observed_on: 2026-09-08, claim: "Sniff found native macOS binary /Users/ken/.kimi-code/bin/kimi version 0.28.1; help exposes ordinary prompt mode, ACP, and web mode.", limitations: "No session was launched and no credential or session content was read." }
  - { id: E_SESSIONS, method: official_docs, location: "https://moonshotai.github.io/kimi-code/en/guides/sessions.html", version: "Documentation observed 2026-09-08; exact version applicability unknown", observed_on: 2026-09-08, claim: "Sessions persist under KIMI_CODE_HOME with provider session IDs and can be resumed; persisted history alone does not identify a live owner.", limitations: "Documentation does not provide a live ordinary-CLI registry or attach endpoint." }
  - { id: E_WEB_DOC, method: official_docs, location: "https://moonshotai.github.io/kimi-code/en/reference/kimi-web.html", version: "Documentation observed 2026-09-08; exact version applicability unknown", observed_on: 2026-09-08, claim: "`kimi web` is a separate server/UI mode and documents queued follow-ups while processing.", limitations: "Documentation does not itself prove external REST delivery or cross-OS behavior." }
  - { id: E_ACP_DOC, method: official_docs, location: "https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html", version: "Documentation observed 2026-09-08; exact version applicability unknown", observed_on: 2026-09-08, claim: "`kimi acp` is JSON-RPC over stdin/stdout with initialize, session list/load/resume, prompt, updates, and cancel.", limitations: "No documented steer/queue method and no attach endpoint for another process's pipes." }
  - { id: E_CLI_SOURCE, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/tree/efacf0452d46f5dbd67499eabc053869495d5213/apps/kimi-code/src/cli", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "Ordinary shell and prompt modes are distinct from web and ACP server entry points; prompt mode is one-shot.", limitations: "Source inspection cannot prove runtime behavior on Linux or Windows." }
  - { id: E_WEB_PROMPTS, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/kap-server/src/routes/prompts.ts", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "Authenticated REST routes list active/queued prompts, submit with stable prompt/user-message IDs, steer selected queued IDs, and abort by prompt ID.", limitations: "No live request was sent; body-size limits and crash durability were not established." }
  - { id: E_WEB_EVENTS, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/kap-server/src/protocol/events-zod.ts", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "WebSocket event schemas include prompt.submitted, prompt.steered, prompt.aborted, and prompt.completed with prompt IDs.", limitations: "Events do not attest model incorporation and delivery/reconnect persistence was not verified." }
  - { id: E_CORE_STEER, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/agent-core/src/agent/turn/index.ts", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "Steer buffers input during an active turn and flushes it before a later model step or stop-continuation decision without canceling the turn.", limitations: "A token-generation loop that never reaches a hook cannot consume the buffer; live timing was not tested." }
  - { id: E_WEB_REGISTRY, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/kap-server/src/instanceRegistry.ts", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "Each server writes a serverId/PID/host/port/start/heartbeat record under KIMI_CODE_HOME/server/instances; liveness uses PID probing and stale records are lazily removed.", limitations: "PID reuse is not guarded by process start time; registry identifies servers, not ordinary TUI owners." }
  - { id: E_WEB_AUTH, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/kap-server/src/services/auth/persistentToken.ts", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "REST/WebSocket routes use a persistent bearer token stored privately under KIMI_CODE_HOME and reload rotation.", limitations: "Reading the token is a prerequisite, not delivery verification; report does not disclose it." }
  - { id: E_ACP_SERVER, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/acp-adapter/src/server.ts", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "ACP routes prompt by sessionId and implements cancel as a notification whose unknown-session/errors are logged and swallowed because notifications cannot respond.", limitations: "No target operation ID guard and no cancellation receipt." }
  - { id: E_ACP_SESSION, method: source_code, location: "https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/acp-adapter/src/session.ts", version: "0.28.1 / efacf0452d46f5dbd67499eabc053869495d5213", observed_on: 2026-09-08, claim: "ACP prompt streams updates until turn end, maps slash skills/built-ins before ordinary prompts, and cancel can abort pre-turn compression or the active SDK turn.", limitations: "Concurrent prompt behavior and post-cancel submission ordering require live verification." }
discovery:
  - { id: D_ORD_MAC_NATIVE, profile_id: ordinary_cli, os: macos, origin: native, method: process_inspection, locator: "Sniff binary discovery plus same-user process inspection and read-only session history", identity_check: "Provider session ID may be read from history, but no evidence maps it to an ordinary live PID", liveness_check: "PID existence only; reject helpers and PID reuse cannot be fully excluded", state_detection: "Unknown without terminal inspection", observation_source: "sniff/local passive inspection", observed_at: "host", available_labels: ["binary", "version", "pid", "historical session_id"], prerequisites: ["same OS user"], evidence_ids: [E_HELP, E_SESSIONS] }
  - { id: D_ORD_LINUX_NATIVE, profile_id: ordinary_cli, os: linux, origin: native, method: process_inspection, locator: "Proposed Sniff same-user process plus history inspection", identity_check: "No live PID-to-session mapping established", liveness_check: "PID only", state_detection: "unknown", observation_source: "source/docs only", observed_at: "not observed", available_labels: ["historical session_id"], prerequisites: ["native Linux check"], evidence_ids: [E_SESSIONS] }
  - { id: D_ORD_WIN_NATIVE, profile_id: ordinary_cli, os: windows, origin: native, method: process_inspection, locator: "Proposed Sniff native-Windows same-user process plus history inspection", identity_check: "No live PID-to-session mapping established", liveness_check: "process-token-scoped PID only", state_detection: "unknown", observation_source: "source/docs only", observed_at: "not observed", available_labels: ["historical session_id"], prerequisites: ["native Windows check"], evidence_ids: [E_SESSIONS] }
  - { id: D_ORD_MAC_CLAUDINE, profile_id: ordinary_cli, os: macos, origin: claudine, method: claudine_registration, locator: "Future Claudine launch record plus provider history", identity_check: "Launch PID does not prove current provider session ID", liveness_check: "registered PID", state_detection: "wrapper output may indicate completion but not reliable working/idle state", observation_source: "inferred managed-launch design", observed_at: "not implemented", available_labels: ["launch pid", "origin"], prerequisites: ["Claudine registration"], evidence_ids: [E_CLI_SOURCE] }
  - { id: D_ORD_LINUX_CLAUDINE, profile_id: ordinary_cli, os: linux, origin: claudine, method: claudine_registration, locator: "Future Claudine launch record", identity_check: "No provider-session binding established", liveness_check: "registered PID", state_detection: "unknown", observation_source: "inference", observed_at: "not observed", available_labels: ["launch pid", "origin"], prerequisites: ["Claudine registration", "native Linux check"], evidence_ids: [E_CLI_SOURCE] }
  - { id: D_ORD_WIN_CLAUDINE, profile_id: ordinary_cli, os: windows, origin: claudine, method: claudine_registration, locator: "Future Claudine launch record", identity_check: "No provider-session binding established", liveness_check: "registered PID", state_detection: "unknown", observation_source: "inference", observed_at: "not observed", available_labels: ["launch pid", "origin"], prerequisites: ["Claudine registration", "native Windows check"], evidence_ids: [E_CLI_SOURCE] }
  - { id: D_WEB_MAC_NATIVE, profile_id: web_server, os: macos, origin: native, method: provider_registry, locator: "KIMI_CODE_HOME/server/instances/*.json then authenticated session/prompt list", identity_check: "serverId plus PID/host/port; session and prompt IDs from API", liveness_check: "registry PID probe plus authenticated HTTP health/read", state_detection: "GET prompts returns active and queued", observation_source: "pinned source", observed_at: "source-derived", available_labels: ["server_id", "pid", "host", "port", "session_id", "prompt_id", "status"], prerequisites: ["same home", "bearer token"], evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { id: D_WEB_LINUX_NATIVE, profile_id: web_server, os: linux, origin: native, method: provider_registry, locator: "Same server registry/API", identity_check: "same source-derived checks", liveness_check: "PID plus authenticated HTTP", state_detection: "GET prompts", observation_source: "pinned source; native Linux untested", observed_at: "not observed", available_labels: ["server_id", "session_id", "prompt_id", "status"], prerequisites: ["bearer token", "native Linux verification"], evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { id: D_WEB_WIN_NATIVE, profile_id: web_server, os: windows, origin: native, method: provider_registry, locator: "Same server registry/API using native paths", identity_check: "same source-derived checks", liveness_check: "PID plus authenticated HTTP", state_detection: "GET prompts", observation_source: "pinned source; native Windows untested", observed_at: "not observed", available_labels: ["server_id", "session_id", "prompt_id", "status"], prerequisites: ["bearer token", "native Windows verification"], evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { id: D_WEB_MAC_CLAUDINE, profile_id: web_server, os: macos, origin: claudine, method: claudine_registration, locator: "Claudine launch record joined to provider registry serverId", identity_check: "Require exact serverId and API sessionId", liveness_check: "registered PID plus authenticated HTTP", state_detection: "GET prompts", observation_source: "proposed managed profile", observed_at: "not implemented", available_labels: ["origin", "server_id", "session_id", "status"], prerequisites: ["managed web profile"], evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { id: D_WEB_LINUX_CLAUDINE, profile_id: web_server, os: linux, origin: claudine, method: claudine_registration, locator: "Same managed registration", identity_check: "exact serverId/sessionId", liveness_check: "PID plus HTTP", state_detection: "GET prompts", observation_source: "proposed profile", observed_at: "not observed", available_labels: ["origin", "server_id", "session_id", "status"], prerequisites: ["managed profile", "native Linux verification"], evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { id: D_WEB_WIN_CLAUDINE, profile_id: web_server, os: windows, origin: claudine, method: claudine_registration, locator: "Same managed registration", identity_check: "exact serverId/sessionId", liveness_check: "PID plus HTTP", state_detection: "GET prompts", observation_source: "proposed profile", observed_at: "not observed", available_labels: ["origin", "server_id", "session_id", "status"], prerequisites: ["managed profile", "native Windows verification"], evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { id: D_ACP_MAC_NATIVE, profile_id: acp_server, os: macos, origin: native, method: provider_api, locator: "Owning ACP client session/list plus its retained child", identity_check: "sessionId returned/listed by this ACP server", liveness_check: "child and pipes open; read-only initialize/list succeeds", state_detection: "No authoritative active/idle query found; client must track outstanding prompt", observation_source: "pinned source/docs", observed_at: "source-derived", available_labels: ["session_id", "cwd", "title"], prerequisites: ["own child pipes"], evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { id: D_ACP_LINUX_NATIVE, profile_id: acp_server, os: linux, origin: native, method: provider_api, locator: "Same ACP list/owned-child check", identity_check: "server-scoped sessionId", liveness_check: "pipes plus request", state_detection: "client-tracked only", observation_source: "source/docs; native Linux untested", observed_at: "not observed", available_labels: ["session_id"], prerequisites: ["own pipes", "native Linux verification"], evidence_ids: [E_ACP_DOC] }
  - { id: D_ACP_WIN_NATIVE, profile_id: acp_server, os: windows, origin: native, method: provider_api, locator: "Same ACP list/owned-child check", identity_check: "server-scoped sessionId", liveness_check: "pipes plus request", state_detection: "client-tracked only", observation_source: "source/docs; native Windows untested", observed_at: "not observed", available_labels: ["session_id"], prerequisites: ["own pipes", "native Windows verification"], evidence_ids: [E_ACP_DOC] }
  - { id: D_ACP_MAC_CLAUDINE, profile_id: acp_server, os: macos, origin: claudine, method: claudine_registration, locator: "Managed child registration plus ACP session/list", identity_check: "bind launch record to exact child and sessionId", liveness_check: "child/pipes plus read-only list", state_detection: "track outstanding prompt request", observation_source: "proposed managed profile", observed_at: "not implemented", available_labels: ["origin", "pid", "session_id"], prerequisites: ["managed ACP profile"], evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { id: D_ACP_LINUX_CLAUDINE, profile_id: acp_server, os: linux, origin: claudine, method: claudine_registration, locator: "Same managed registration", identity_check: "exact child/sessionId", liveness_check: "pipes/list", state_detection: "client-tracked", observation_source: "proposed profile", observed_at: "not observed", available_labels: ["origin", "session_id"], prerequisites: ["managed profile", "native Linux verification"], evidence_ids: [E_ACP_DOC] }
  - { id: D_ACP_WIN_CLAUDINE, profile_id: acp_server, os: windows, origin: claudine, method: claudine_registration, locator: "Same managed registration", identity_check: "exact child/sessionId", liveness_check: "pipes/list", state_detection: "client-tracked", observation_source: "proposed profile", observed_at: "not observed", available_labels: ["origin", "session_id"], prerequisites: ["managed profile", "native Windows verification"], evidence_ids: [E_ACP_DOC] }
mechanisms:
  - id: web_steer
    interface_status: documented
    maturity: stable
    transport: http
    initialization: "Start `kimi web`, resolve a live registry entry, authenticate, identify the exact session and active prompt, POST a second prompt to obtain queued IDs, then steer those IDs."
    operation_intent: steer_active_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: next_tool_boundary
    destination: "Exact REST session_id; queued prompt_ids are explicitly selected and merged into that session's active main-agent turn."
    authentication: "Bearer token from KIMI_CODE_HOME/server.token (or configured password); never print it."
    startup_requirements: ["web_server profile", "authenticated endpoint", "active main-agent prompt"]
    target_preconditions: ["session exists", "active prompt exists", "selected prompts are queued in that session"]
    target_guards: ["session_id", "prompt_ids" , "No expected active-prompt/turn ID guard; if the active turn ends before steer, the request is refused and queued work may auto-start"]
    request_framing: "POST /api/v1/sessions/{session_id}/prompts with JSON content, then POST /api/v1/sessions/{session_id}/prompts::steer with {prompt_ids:[...]}."
    response_framing: "JSON envelopes with request id; submission returns prompt_id, user_message_id, status, content, created_at; steer returns steered=true and prompt_ids or typed error."
    request_format: "UTF-8 JSON; content is structured parts and may carry model/thinking/permission metadata."
    response_format: "UTF-8 JSON plus optional authenticated WebSocket events."
    long_tool_behavior: "Steered text waits in memory until the core reaches beforeStep or stop-continuation handling; it does not cancel a running tool."
    tool_batch_behavior: provider_defined
    queue_behavior: "Busy submission is FIFO queued with prompt IDs; selected entries are removed, restored on definite core-steer failure, and appended to context when the active turn reaches a delivery boundary. Crash persistence is unknown."
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: "Not applicable; this mechanism does not interrupt."
    ordering: "Selected prompt IDs are concatenated in caller order; relative behavior with concurrently queued submissions requires testing."
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: "No idempotency key or duplicate suppression was found; an ambiguous network response can duplicate a submitted prompt."
    cancellation: "Queued prompts can be aborted by prompt ID; aborting the active prompt is a separate API and is not part of steering."
    limits: "Content must be a nonempty structured array; HTTP/body/message maximum and expiration are unknown."
    evidence_ids: [E_WEB_PROMPTS, E_WEB_EVENTS, E_CORE_STEER]
  - id: web_submit
    interface_status: documented
    maturity: stable
    transport: http
    initialization: "Start and authenticate to `kimi web`, resolve exact session, verify no active prompt, then submit."
    operation_intent: start_idle_turn
    conversation_effect: resume_same_conversation
    delivery_boundary: idle_turn_start
    destination: "Exact session_id and optional agent_id."
    authentication: "Bearer token or configured password."
    startup_requirements: ["web_server profile", "authenticated endpoint"]
    target_preconditions: ["session exists", "session is idle for the selected agent"]
    target_guards: ["session_id", "optional agent_id", "No compare-and-set idle guard; a race may return queued instead of running"]
    request_framing: "POST /api/v1/sessions/{session_id}/prompts with PromptSubmission JSON."
    response_framing: "JSON envelope containing prompt_id/user_message_id and status running or queued."
    request_format: "Structured UTF-8 JSON content."
    response_format: "UTF-8 JSON plus WebSocket lifecycle events."
    long_tool_behavior: "If a turn raced active, submission queues rather than interrupts."
    tool_batch_behavior: not_applicable
    queue_behavior: "Starts immediately when idle; otherwise queues FIFO for later automatic start."
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: "Not applicable."
    ordering: "Server queue order is submission order; concurrent HTTP arrival ordering is unverified."
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: "Server mints IDs after admission; no client idempotency key was found."
    cancellation: "Abort by returned prompt ID."
    limits: "Nonempty content; other bounds unknown."
    evidence_ids: [E_WEB_PROMPTS, E_WEB_EVENTS]
  - id: acp_cancel_submit
    interface_status: documented
    maturity: stable
    transport: stdio
    initialization: "Own a `kimi acp` child, initialize, load/resume session, send session/cancel, observe terminal updates, then submit session/prompt."
    operation_intent: interrupt_then_submit
    conversation_effect: cancel_turn_same_conversation
    delivery_boundary: next_turn
    destination: "ACP sessionId; cancel has no expected turn/operation guard."
    authentication: "ACP provider authentication may be required; transport itself is private only by retained child pipes."
    startup_requirements: ["acp_server profile", "owned pipes", "loaded exact session"]
    target_preconditions: ["session is loaded in this ACP server", "working state is client-tracked"]
    target_guards: ["sessionId only", "Unknown session cancel is silently logged", "No expected active turn ID"]
    request_framing: "JSON-RPC session/cancel notification followed, after settlement, by session/prompt request with content blocks."
    response_framing: "Cancel has no response; prompt streams session/update notifications and terminates with PromptResponse stopReason or JSON-RPC error."
    request_format: "ACP JSON-RPC over retained stdin/stdout."
    response_format: "ACP JSON-RPC responses and notifications."
    long_tool_behavior: "Cancellation aborts the active SDK turn and propagates to tools/subagents; already-landed external side effects are not rolled back."
    tool_batch_behavior: stop_remaining
    queue_behavior: "ACP exposes no steering/follow-up queue in 0.28.1."
    message_interpretation: skills_templates
    interruption_phases: ["Revalidate exact session and outstanding prompt", "Send unacknowledged session/cancel", "Wait for cancelled terminal state", "Revalidate session", "Send replacement session/prompt"]
    interruption_partial_failure: "If cancel succeeds and prompt submission fails, the original turn remains cancelled, its preserved history/side effects remain, and no replacement is queued."
    ordering: "Client must serialize cancel, terminal observation, revalidation, and prompt; pipelining is unsafe."
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: "JSON-RPC IDs correlate responses but are not documented idempotency keys."
    cancellation: "Cancel is idempotent at SDK level but ACP notification failures are swallowed; replacement prompt can itself be cancelled separately."
    limits: "ACP frame/content maximum unknown."
    evidence_ids: [E_ACP_SERVER, E_ACP_SESSION]
  - id: acp_prompt
    interface_status: documented
    maturity: stable
    transport: stdio
    initialization: "Own initialized ACP child and load/resume exact idle session."
    operation_intent: start_idle_turn
    conversation_effect: resume_same_conversation
    delivery_boundary: idle_turn_start
    destination: "Exact sessionId loaded in this ACP server."
    authentication: "Provider authentication through ACP; retained pipes delimit the client."
    startup_requirements: ["acp_server profile", "owned pipes"]
    target_preconditions: ["session loaded", "no outstanding prompt request"]
    target_guards: ["sessionId", "No expected idle generation"]
    request_framing: "JSON-RPC session/prompt request with sessionId and ACP content blocks."
    response_framing: "session/update notifications followed by PromptResponse stopReason or error with matching request ID."
    request_format: "ACP JSON-RPC."
    response_format: "ACP JSON-RPC."
    long_tool_behavior: "Prompt request remains outstanding until the turn ends."
    tool_batch_behavior: not_applicable
    queue_behavior: "No documented concurrent prompt queue; the client must serialize."
    message_interpretation: skills_templates
    interruption_phases: []
    interruption_partial_failure: "Not applicable."
    ordering: "One outstanding prompt per session is the conservative policy."
    sender_message_id: unsupported
    retry_policy: never_retry
    duplicate_handling: "No idempotency guarantee."
    cancellation: "Separate session/cancel notification."
    limits: "Content/frame bounds unknown."
    evidence_ids: [E_ACP_DOC, E_ACP_SERVER, E_ACP_SESSION]
compatibility:
  - { mechanism_id: web_steer, profile_id: web_server, os: macos, versions_verified: [], documented_version_bounds: "Source-derived for exactly 0.28.1; no broader bound claimed.", read_only_check: "Check binary/registry hostVersion, PID liveness, authenticated API metadata, exact session, and GET active/queued prompts.", success_criteria: "Version 0.28.1, live server, exact session, one active prompt and selected queued IDs.", failure_behavior: "Do not send, retry, interrupt, or fall back to terminal injection.", evidence_ids: [E_HELP, E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { mechanism_id: web_steer, profile_id: web_server, os: linux, versions_verified: [], documented_version_bounds: "0.28.1 source only; native Linux unverified.", read_only_check: "Same registry/version/authenticated prompt-list check on native Linux.", success_criteria: "Exact version/server/session/prompt state.", failure_behavior: "Unavailable.", evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { mechanism_id: web_steer, profile_id: web_server, os: windows, versions_verified: [], documented_version_bounds: "0.28.1 source only; native Windows unverified.", read_only_check: "Same native-Windows registry/version/authenticated prompt-list check.", success_criteria: "Exact version/server/session/prompt state.", failure_behavior: "Unavailable.", evidence_ids: [E_WEB_REGISTRY, E_WEB_PROMPTS] }
  - { mechanism_id: web_submit, profile_id: web_server, os: macos, versions_verified: [], documented_version_bounds: "Exactly 0.28.1 source; no broader bound.", read_only_check: "Authenticated GET prompts must show active=null immediately before submit.", success_criteria: "Exact session idle on expected server.", failure_behavior: "Do not send; no implicit queueing policy.", evidence_ids: [E_WEB_PROMPTS] }
  - { mechanism_id: web_submit, profile_id: web_server, os: linux, versions_verified: [], documented_version_bounds: "0.28.1 source only.", read_only_check: "Native Linux authenticated idle check.", success_criteria: "Exact session idle.", failure_behavior: "Unavailable.", evidence_ids: [E_WEB_PROMPTS] }
  - { mechanism_id: web_submit, profile_id: web_server, os: windows, versions_verified: [], documented_version_bounds: "0.28.1 source only.", read_only_check: "Native Windows authenticated idle check.", success_criteria: "Exact session idle.", failure_behavior: "Unavailable.", evidence_ids: [E_WEB_PROMPTS] }
  - { mechanism_id: acp_cancel_submit, profile_id: acp_server, os: macos, versions_verified: [], documented_version_bounds: "Exactly 0.28.1 source; no broader bound.", read_only_check: "Initialize agentInfo version, session/list, exact loaded session, and client-owned outstanding prompt state.", success_criteria: "Exact 0.28.1 child/session and one tracked active prompt.", failure_behavior: "Do not cancel or submit.", evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { mechanism_id: acp_cancel_submit, profile_id: acp_server, os: linux, versions_verified: [], documented_version_bounds: "0.28.1 source only; native Linux unverified.", read_only_check: "Same native Linux ACP checks.", success_criteria: "Exact child/session/active request.", failure_behavior: "Unavailable.", evidence_ids: [E_ACP_DOC] }
  - { mechanism_id: acp_cancel_submit, profile_id: acp_server, os: windows, versions_verified: [], documented_version_bounds: "0.28.1 source only; native Windows unverified.", read_only_check: "Same native Windows ACP checks with binary-safe pipes.", success_criteria: "Exact child/session/active request.", failure_behavior: "Unavailable.", evidence_ids: [E_ACP_DOC] }
  - { mechanism_id: acp_prompt, profile_id: acp_server, os: macos, versions_verified: [], documented_version_bounds: "Exactly 0.28.1 source; no broader bound.", read_only_check: "Initialize version, list/load session, and require no outstanding prompt request.", success_criteria: "Exact loaded idle session.", failure_behavior: "Do not submit.", evidence_ids: [E_ACP_DOC, E_ACP_SERVER] }
  - { mechanism_id: acp_prompt, profile_id: acp_server, os: linux, versions_verified: [], documented_version_bounds: "0.28.1 source only.", read_only_check: "Same native Linux checks.", success_criteria: "Exact idle session.", failure_behavior: "Unavailable.", evidence_ids: [E_ACP_DOC] }
  - { mechanism_id: acp_prompt, profile_id: acp_server, os: windows, versions_verified: [], documented_version_bounds: "0.28.1 source only.", read_only_check: "Same native Windows checks.", success_criteria: "Exact idle session.", failure_behavior: "Unavailable.", evidence_ids: [E_ACP_DOC] }
verification: []
cases:
  # ordinary_cli: exactly 24 baseline combinations
  - { profile_id: ordinary_cli, os: macos, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_ORD_MAC_NATIVE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_HELP, E_SESSIONS], reason: "No documented external endpoint maps an ordinary live TUI to its active conversation." }
  - { profile_id: ordinary_cli, os: macos, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_ORD_MAC_NATIVE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_HELP, E_SESSIONS], reason: "Writable terminal input was not established as a stable peer protocol." }
  - { profile_id: ordinary_cli, os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_ORD_MAC_CLAUDINE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "Wrapping does not create a provider control endpoint or provider-session binding." }
  - { profile_id: ordinary_cli, os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_ORD_MAC_CLAUDINE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "Managed PID knowledge alone cannot submit to the TUI safely." }
  - { profile_id: ordinary_cli, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_ORD_MAC_NATIVE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_HELP, E_CLI_SOURCE], reason: "One-shot prompt stdin is not a documented retained control channel." }
  - { profile_id: ordinary_cli, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unsupported, discovery_ids: [D_ORD_MAC_NATIVE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "The ordinary one-shot prompt process exits after its turn and cannot remain idle-open." }
  - { profile_id: ordinary_cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_ORD_MAC_CLAUDINE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "Claudine origin does not change one-shot transport." }
  - { profile_id: ordinary_cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unsupported, discovery_ids: [D_ORD_MAC_CLAUDINE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "One-shot lifetime has no idle-open state." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_ORD_LINUX_NATIVE], mechanism_ids: [], prerequisites: ["native Linux verification"], evidence_ids: [E_SESSIONS], reason: "No attach endpoint established." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_ORD_LINUX_NATIVE], mechanism_ids: [], prerequisites: ["native Linux verification"], evidence_ids: [E_SESSIONS], reason: "Terminal injection is not a provider protocol." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_ORD_LINUX_CLAUDINE], mechanism_ids: [], prerequisites: ["native Linux verification"], evidence_ids: [E_CLI_SOURCE], reason: "No control endpoint." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_ORD_LINUX_CLAUDINE], mechanism_ids: [], prerequisites: ["native Linux verification"], evidence_ids: [E_CLI_SOURCE], reason: "No safe delivery path." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_ORD_LINUX_NATIVE], mechanism_ids: [], prerequisites: ["native Linux verification"], evidence_ids: [E_CLI_SOURCE], reason: "No retained protocol." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unsupported, discovery_ids: [D_ORD_LINUX_NATIVE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "One-shot process exits." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_ORD_LINUX_CLAUDINE], mechanism_ids: [], prerequisites: ["native Linux verification"], evidence_ids: [E_CLI_SOURCE], reason: "No retained protocol." }
  - { profile_id: ordinary_cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unsupported, discovery_ids: [D_ORD_LINUX_CLAUDINE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "One-shot process exits." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_ORD_WIN_NATIVE], mechanism_ids: [], prerequisites: ["native Windows verification"], evidence_ids: [E_SESSIONS], reason: "No native-Windows attach endpoint established." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [D_ORD_WIN_NATIVE], mechanism_ids: [], prerequisites: ["native Windows verification"], evidence_ids: [E_SESSIONS], reason: "Console input injection is not a stable messaging protocol." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_ORD_WIN_CLAUDINE], mechanism_ids: [], prerequisites: ["native Windows verification"], evidence_ids: [E_CLI_SOURCE], reason: "No control endpoint." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [D_ORD_WIN_CLAUDINE], mechanism_ids: [], prerequisites: ["native Windows verification"], evidence_ids: [E_CLI_SOURCE], reason: "No safe delivery path." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [D_ORD_WIN_NATIVE], mechanism_ids: [], prerequisites: ["native Windows verification"], evidence_ids: [E_CLI_SOURCE], reason: "No retained protocol." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unsupported, discovery_ids: [D_ORD_WIN_NATIVE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "One-shot process exits." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [D_ORD_WIN_CLAUDINE], mechanism_ids: [], prerequisites: ["native Windows verification"], evidence_ids: [E_CLI_SOURCE], reason: "No retained protocol." }
  - { profile_id: ordinary_cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unsupported, discovery_ids: [D_ORD_WIN_CLAUDINE], mechanism_ids: [], prerequisites: [], evidence_ids: [E_CLI_SOURCE], reason: "One-shot process exits." }
  # web_server: 3 OS x 1 launch mode x 2 origins x 2 states
  - { profile_id: web_server, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [D_WEB_MAC_NATIVE], mechanism_ids: [web_steer], prerequisites: ["live test still required for activation"], evidence_ids: [E_WEB_PROMPTS, E_CORE_STEER], reason: "REST can queue an identified prompt then steer it into the same active turn without cancellation." }
  - { profile_id: web_server, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [D_WEB_MAC_NATIVE], mechanism_ids: [web_submit], prerequisites: ["live test still required for activation"], evidence_ids: [E_WEB_PROMPTS], reason: "Submission starts an idle session turn." }
  - { profile_id: web_server, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [D_WEB_MAC_CLAUDINE], mechanism_ids: [web_steer], prerequisites: ["future managed web profile", "live test"], evidence_ids: [E_WEB_PROMPTS, E_CORE_STEER], reason: "Provider capability is available if Claudine deliberately launches/registers web mode." }
  - { profile_id: web_server, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [D_WEB_MAC_CLAUDINE], mechanism_ids: [web_submit], prerequisites: ["future managed web profile", "live test"], evidence_ids: [E_WEB_PROMPTS], reason: "Managed web session can start an idle turn." }
  - { profile_id: web_server, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [D_WEB_LINUX_NATIVE], mechanism_ids: [web_steer], prerequisites: ["native Linux live test"], evidence_ids: [E_WEB_PROMPTS, E_CORE_STEER], reason: "Source is portable; runtime remains unverified." }
  - { profile_id: web_server, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [D_WEB_LINUX_NATIVE], mechanism_ids: [web_submit], prerequisites: ["native Linux live test"], evidence_ids: [E_WEB_PROMPTS], reason: "Source establishes idle submission candidate." }
  - { profile_id: web_server, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [D_WEB_LINUX_CLAUDINE], mechanism_ids: [web_steer], prerequisites: ["future managed profile", "native Linux live test"], evidence_ids: [E_WEB_PROMPTS, E_CORE_STEER], reason: "Requires deliberate future server launch." }
  - { profile_id: web_server, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [D_WEB_LINUX_CLAUDINE], mechanism_ids: [web_submit], prerequisites: ["future managed profile", "native Linux live test"], evidence_ids: [E_WEB_PROMPTS], reason: "Requires deliberate future server launch." }
  - { profile_id: web_server, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [D_WEB_WIN_NATIVE], mechanism_ids: [web_steer], prerequisites: ["native Windows live test"], evidence_ids: [E_WEB_PROMPTS, E_CORE_STEER], reason: "Source establishes candidate; native Windows transport is unverified." }
  - { profile_id: web_server, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [D_WEB_WIN_NATIVE], mechanism_ids: [web_submit], prerequisites: ["native Windows live test"], evidence_ids: [E_WEB_PROMPTS], reason: "Source establishes candidate." }
  - { profile_id: web_server, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [D_WEB_WIN_CLAUDINE], mechanism_ids: [web_steer], prerequisites: ["future managed profile", "native Windows live test"], evidence_ids: [E_WEB_PROMPTS, E_CORE_STEER], reason: "Requires deliberate future server launch." }
  - { profile_id: web_server, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [D_WEB_WIN_CLAUDINE], mechanism_ids: [web_submit], prerequisites: ["future managed profile", "native Windows live test"], evidence_ids: [E_WEB_PROMPTS], reason: "Requires deliberate future server launch." }
  # acp_server: 3 OS x 1 launch mode x 2 origins x 2 states
  - { profile_id: acp_server, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: interruption_required, discovery_ids: [D_ACP_MAC_NATIVE], mechanism_ids: [acp_cancel_submit], prerequisites: ["explicit user approval", "live test"], evidence_ids: [E_ACP_SERVER, E_ACP_SESSION], reason: "ACP has cancel and prompt but no non-interrupting steer method." }
  - { profile_id: acp_server, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [D_ACP_MAC_NATIVE], mechanism_ids: [acp_prompt], prerequisites: ["live test"], evidence_ids: [E_ACP_DOC, E_ACP_SESSION], reason: "ACP prompt starts a turn in its loaded idle session." }
  - { profile_id: acp_server, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: interruption_required, discovery_ids: [D_ACP_MAC_CLAUDINE], mechanism_ids: [acp_cancel_submit], prerequisites: ["future managed ACP profile", "explicit user approval", "live test"], evidence_ids: [E_ACP_SERVER, E_ACP_SESSION], reason: "Requires owned pipes and interrupt-then-submit." }
  - { profile_id: acp_server, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [D_ACP_MAC_CLAUDINE], mechanism_ids: [acp_prompt], prerequisites: ["future managed ACP profile", "live test"], evidence_ids: [E_ACP_DOC], reason: "Managed ACP can start the loaded idle session." }
  - { profile_id: acp_server, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: interruption_required, discovery_ids: [D_ACP_LINUX_NATIVE], mechanism_ids: [acp_cancel_submit], prerequisites: ["explicit approval", "native Linux live test"], evidence_ids: [E_ACP_SERVER], reason: "Source-derived interruption path; native runtime unverified." }
  - { profile_id: acp_server, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [D_ACP_LINUX_NATIVE], mechanism_ids: [acp_prompt], prerequisites: ["native Linux live test"], evidence_ids: [E_ACP_DOC], reason: "Documented idle prompt candidate." }
  - { profile_id: acp_server, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: interruption_required, discovery_ids: [D_ACP_LINUX_CLAUDINE], mechanism_ids: [acp_cancel_submit], prerequisites: ["future managed profile", "explicit approval", "native Linux live test"], evidence_ids: [E_ACP_SERVER], reason: "Requires retained pipes and interruption." }
  - { profile_id: acp_server, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [D_ACP_LINUX_CLAUDINE], mechanism_ids: [acp_prompt], prerequisites: ["future managed profile", "native Linux live test"], evidence_ids: [E_ACP_DOC], reason: "Managed ACP candidate." }
  - { profile_id: acp_server, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: interruption_required, discovery_ids: [D_ACP_WIN_NATIVE], mechanism_ids: [acp_cancel_submit], prerequisites: ["explicit approval", "native Windows live test"], evidence_ids: [E_ACP_SERVER], reason: "Source-derived path; native Windows pipes unverified." }
  - { profile_id: acp_server, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [D_ACP_WIN_NATIVE], mechanism_ids: [acp_prompt], prerequisites: ["native Windows live test"], evidence_ids: [E_ACP_DOC], reason: "Documented candidate." }
  - { profile_id: acp_server, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: interruption_required, discovery_ids: [D_ACP_WIN_CLAUDINE], mechanism_ids: [acp_cancel_submit], prerequisites: ["future managed profile", "explicit approval", "native Windows live test"], evidence_ids: [E_ACP_SERVER], reason: "Requires retained pipes and interruption." }
  - { profile_id: acp_server, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [D_ACP_WIN_CLAUDINE], mechanism_ids: [acp_prompt], prerequisites: ["future managed profile", "native Windows live test"], evidence_ids: [E_ACP_DOC], reason: "Managed ACP candidate." }
gaps:
  - { area: "Live activation", detail: "verification is empty; no mechanism is eligible for Claudine activation.", next_check: "Run disposable, non-focusing delivery tests for every enabled OS/version/profile condition." }
  - { area: "Ordinary sessions", detail: "No provider API binds ordinary TUI/one-shot PIDs to live session IDs or accepts peer messages.", next_check: "Recheck future releases for a documented attach/control endpoint; do not use keystroke injection as protocol fallback." }
  - { area: "Web acknowledgment", detail: "REST receipts do not prove persistence or model incorporation; body limits, expiry, reconnection replay, and idempotency are unknown.", next_check: "Test response loss, duplicate submission, crash/restart, size limits, event correlation, and model-visible delivery." }
  - { area: "Steer boundary", detail: "Source shows next-step buffering, but exact behavior during long tools, parallel tool batches, compaction, goals, and a never-ending token stream is unverified.", next_check: "Use bounded disposable fixtures for generation, long tool, multi-tool batch, and compaction; assert no cancellation and same conversation." }
  - { area: "ACP cancellation", detail: "Cancel is an unacknowledged notification with no expected turn ID; partial failure after cancellation is unavoidable.", next_check: "Test stale session IDs, idle cancel, long tool cancellation, terminal observation, context preservation, side effects, and replacement failure." }
  - { area: "Resource preservation", detail: "Web appears to use the ordinary configured runtime; ACP explicitly exposes skills and context, but prompt-template preservation is not documented precisely.", next_check: "Test enabled extensions/plugins, skills, templates, AGENTS/context files, MCP, and explicit provider settings in both managed profiles." }
  - { area: "OS coverage", detail: "Only native macOS binary/help was observed; Linux and native Windows are source-derived. WSL was not used and would count only as Linux-side evidence.", next_check: "Perform native Linux and native Windows passive compatibility and disposable delivery tests." }
changes:
  - "Initial Kimi steering report under schema revision 2."
  - "Separated ordinary CLI, web-server, and ACP retained-stdio launch profiles."
  - "Recorded source-derived web prompt queue/steer and ACP interrupt-then-submit candidates; left verification empty."
requires_claudine_update: true
reason: "A future managed `kimi web` profile is the preferred candidate because it exposes exact session/prompt IDs and non-interrupting steer; ACP is a retained-stdio fallback for idle prompts and explicitly approved interruption. Ordinary sessions remain undiscoverable for safe delivery. Adapters, registration, compatibility guards, and mandatory live tests do not yet exist."
---

# Kimi Code CLI steering research

## Overview

Kimi Code CLI 0.28.1 has a promising provider-native steering interface, but
only in the separately launched `kimi web` server profile. Its REST API queues a
second prompt with stable IDs and can steer selected queued prompts into the
same active conversation without cancellation. Pinned source shows that the
input is buffered until the agent reaches a later model-step boundary. This is
not immediate token-stream injection and cannot help a generation loop that
never reaches another step or stop-continuation hook.

`kimi acp` is a different retained-stdio profile. It can list, load, resume,
prompt, and cancel sessions, but 0.28.1 exposes no non-interrupting steer or
follow-up method. Steering a working ACP session therefore means an explicitly
approved cancel-then-submit sequence with a failure window after cancellation.

This was passive research by Codex using `gpt-5.6-sol` with low reasoning
effort. The model and effort provenance are launcher-supplied; no independent
resolved-execution metadata was exposed to this research turn, so none is
fabricated. The pass inspected installed 0.28.1 help on native macOS, official
documentation, and tag-pinned source. It launched no Kimi session, sent no
message, and performed no interruption. `verification` is therefore empty and
all activation remains blocked.

## Session discovery

Persisted session directories provide provider session identity and history,
not liveness. Ordinary process inspection can find candidate Kimi PIDs but does
not distinguish a TUI owner from helpers or bind a PID to the currently loaded
conversation. PID reuse remains a concern.

Web mode is stronger: each server registers `server_id`, PID, host, port,
timestamps, and version metadata beneath `KIMI_CODE_HOME/server/instances`.
Source checks PID liveness and removes dead entries lazily. An authenticated API
query then provides session IDs and active/queued prompt IDs. The registry's PID
probe does not compare process start time, so Claudine should join it to its own
managed-launch record and verify the authenticated endpoint before trusting it.

ACP discovery is scoped to the child whose pipes the sender owns. `session/list`
and load/resume identify conversations, but no attach endpoint recovers another
process's stdin/stdout. Working versus idle must be tracked from outstanding
prompt requests and terminal updates; no independent status query was found.

## Non-interrupting delivery

The web flow is submit, inspect the returned `queued` status and IDs, then steer
those exact IDs. Source preserves the active turn and appends steered user input
before the next model step. A running long tool is not cancelled. The complete
parallel/batched-tool behavior is provider-defined and needs live testing.

The initial HTTP receipt proves admission/scheduling only to the degree stated
in `receipt_guarantees`. `prompt.steered` is a later correlated signal, but no
signal proves that the model incorporated the text. Because submission has no
client idempotency key, an ambiguous network failure must never be retried.

## Interruption fallback

ACP cancellation targets only `sessionId`, not an expected turn ID. It is a
JSON-RPC notification and therefore has no response; unknown-session and
internal cancel errors are logged and swallowed. A safe manual sequence must
revalidate, notify cancel, wait for terminal cancellation, revalidate again,
and submit a new prompt. If submission fails, the original turn stays cancelled,
preserved conversation history and already-landed side effects remain, and no
replacement is queued. Automatic loop warnings must not use this fallback.

The web API also exposes prompt abort, but interruption is unnecessary for its
normal steering path and should not be silently substituted.

## Idle sessions

An idle web session accepts `POST .../prompts` and starts a turn. A race can
instead yield `queued`, so Claudine must check the returned status. An idle ACP
session accepts `session/prompt`, which starts a turn and keeps the request open
through completion. An ordinary idle TUI has no established peer channel;
terminal keystroke injection is a distinct automation technique, not a stable
provider messaging interface. Ordinary one-shot prompt mode exits and has no
idle-open state.

## Protocol details

Web mode uses bearer-authenticated HTTP JSON plus an authenticated WebSocket
event stream. Prompt IDs and user-message IDs are provider-generated. Queue
listing distinguishes active, queued, and blocked prompt states; a blocked
prompt is a delivery/scheduling state and must not be confused with a later tool
permission hold. Selected queued prompts can be steered or aborted. Ordering,
durability, expiry, maximum size, and duplicate suppression remain incomplete.

ACP is JSON-RPC over retained stdin/stdout. Prompt text can invoke advertised
skills and ACP-handled slash commands, so it is not guaranteed literal text.
The request ID correlates the terminal response, while `session/update` uses the
session ID and tool-call identifiers. No sender message ID, target operation
guard, prompt queue, or steer call was found.

## OS and version compatibility

The installed observation is native macOS 0.28.1. Pinned source is written as a
cross-platform Node application and the product ships macOS, Linux, and Windows
artifacts, but that is not runtime verification. Native Linux and native Windows
remain untested. WSL evidence, if later collected, must be labeled Linux-side and
must not be used as native-Windows proof. Compatibility bounds beyond exactly
0.28.1 are unknown.

## Disposable-test proposals

For web mode on each native OS, launch a fresh isolated home without focusing a
browser, capture the server registration without exposing its token, create one
session, and test idle start, active generation, a long tool, and a multi-tool
batch. Submit a uniquely tagged queued prompt, steer it, and assert same session,
unchanged active prompt/turn, no cancellation, correlated events, and model-visible
incorporation. Add response-loss, duplicate, stale-ID, crash/restart, ordering,
size-limit, and token-loop cases.

For ACP, own a disposable child, initialize, and create/load exactly one session.
Test idle prompt separately. For working state, test cancel during generation and
during a long tool, wait for the terminal signal, submit replacement text, and
assert conversation identity, preserved context, precise tool effects, and the
cancel-success/submit-failure outcome. These proposals are not verification
records.

## Claudine integration

Prefer a managed `kimi web` launch profile after mandatory live tests. Register
the exact server ID, endpoint, PID/start identity, version, session, and active
prompt; use read-only authenticated checks immediately before sending. Preserve
the user's normal plugins/extensions, skills, prompt templates, context files,
MCP configuration, and explicit model/permission settings. Claudine should not
install or mutate those resources during delivery.

The verified fallback is managed ACP for idle turns and, only after an explicit
interactive warning and choice, interrupt-then-submit for working turns. It
applies only to future sessions launched with retained pipes, not already-open
ordinary sessions. If web RPC is unavailable or compatibility cannot be proven,
warn that ordinary sessions cannot be steered safely; never silently fall back
to stdin or terminal-keystroke injection.

## Gaps

The structured `gaps` list is authoritative. The largest blockers are the empty
live-test record, ordinary-session identity/reachability, absence of a model-
incorporation receipt, unknown retry/idempotency and size semantics, ACP's
unacknowledged cancellation, and missing native Linux/Windows verification.

## Sources

- [Kimi Code sessions](https://moonshotai.github.io/kimi-code/en/guides/sessions.html)
- [`kimi acp` reference](https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html)
- [`kimi web` reference](https://moonshotai.github.io/kimi-code/en/reference/kimi-web.html)
- [0.28.1 prompt REST routes](https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/kap-server/src/routes/prompts.ts)
- [0.28.1 core steering boundary](https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/agent-core/src/agent/turn/index.ts)
- [0.28.1 ACP server](https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/acp-adapter/src/server.ts)
- [0.28.1 server registry](https://github.com/MoonshotAI/kimi-code/blob/efacf0452d46f5dbd67499eabc053869495d5213/packages/kap-server/src/instanceRegistry.ts)

## Changelog

- 2026-09-08: Initial revision-2 report; separated ordinary, web, and ACP
  profiles and recorded passive candidates without activating them.
