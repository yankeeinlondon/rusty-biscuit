---
"$schema": "./_schema.yaml"
schema_revision: 2
provider: antigravity
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
  - Antigravity CLI 1.1.27 (installed signed macOS arm64 binary, help, embedded changelog, and passive state layout)
  - Antigravity CLI 1.1.15 through 1.1.27 release notes embedded in 1.1.27
  - Official Antigravity CLI product page and prior provider research, with current public source revision unavailable
launch_profiles:
  - id: ordinary-cli
    description: Ordinary interactive TUI or one-shot print launch without a deliberately retained structured-input controller.
    endpoint_scope: none
    lifetime: unknown
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [interactive, non_interactive]
    applicable_origins: [native, claudine]
    baseline: true
    startup_requirements: [ordinary agy launch]
    preserves_extensions: 'yes'
    preserves_skills: 'yes'
    preserves_templates: 'yes'
    preserves_context: 'yes'
    evidence_ids: [official-product, local-cli-1127, local-state, prior-resume, wrapper-source]
  - id: managed-stream-json
    description: Future Claudine-managed print process launched with retained NDJSON stdin and stream-json stdout, exclusively owned and registered by Claudine.
    endpoint_scope: retained_stdio
    lifetime: while_client_open
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [claudine]
    baseline: false
    startup_requirements:
      - launch agy in print mode with --input-format stream-json and --output-format stream-json
      - retain and exclusively own stdin and stdout
      - capture the init event and exact conversation identity
      - register process start identity and protocol state before accepting steering requests
    preserves_extensions: unknown
    preserves_skills: unknown
    preserves_templates: unknown
    preserves_context: unknown
    evidence_ids: [local-changelog-1127, local-binary-strings]
access_findings:
  - { mechanism_id: retained-ndjson-idle, profile_id: managed-stream-json, os: macos, status: setup_required, prerequisite: "Claudine must launch and retain the structured-input process and register its exact conversation; an ordinary open process cannot be upgraded.", applies_to_existing_sessions: 'no', evidence_ids: [local-changelog-1127, local-binary-strings] }
  - { mechanism_id: retained-ndjson-idle, profile_id: managed-stream-json, os: linux, status: setup_required, prerequisite: "A future managed retained-stdio launch is required; Linux behavior is not tested.", applies_to_existing_sessions: 'no', evidence_ids: [local-changelog-1127] }
  - { mechanism_id: retained-ndjson-idle, profile_id: managed-stream-json, os: windows, status: setup_required, prerequisite: "A future managed retained-stdio launch with Windows-safe pipe handling is required; native Windows behavior is not tested.", applies_to_existing_sessions: 'no', evidence_ids: [local-changelog-1127] }
delivery_states:
  - mechanism_id: retained-ndjson-idle
    states: [accepted, delivered, refused, unknown]
    observable_by_external_sender: partial
    correlation: Output has typed init, step_update, and result events, but passive evidence did not establish a sender message ID or whether any early event acknowledges one input line before execution.
    evidence_ids: [local-changelog-1127, local-binary-strings]
receipt_guarantees:
  - mechanism_id: retained-ndjson-idle
    request_acceptance: unknown
    persistence: unknown
    scheduling: unknown
    conversation_delivery: unknown
    provider_signals: [stream-json init event, stream-json step_update events, terminal stream-json result event]
    correlation: unknown
    evidence_ids: [local-changelog-1127, local-binary-strings]
    limitations: Release notes establish multiple turns in one retained conversation, but not the meaning or timing of the first response to a line. A successful pipe write proves none of acceptance, persistence, scheduling, or conversation delivery.
evidence:
  - id: official-product
    method: official_docs
    location: https://antigravity.google/product/antigravity-cli
    version: current page observed 2026-09-08; provider version unspecified
    observed_on: 2026-09-08
    claim: Google publishes Antigravity CLI as the agy product for interactive and headless agent use.
    limitations: The dynamic page did not expose a versioned peer-steering or structured-input protocol contract to the research client.
  - id: local-cli-1127
    method: local_inspection
    location: sanitized passive output of Sniff software-agent discovery, agy --version, agy help, Mach-O inspection, and code-signature inspection
    version: 1.1.27 from agy --version; Sniff separately reported stale 1.1.5
    observed_on: 2026-09-08
    claim: The installed macOS arm64 executable is Google-signed and exposes interactive, --prompt-interactive, one-shot --print, exact --conversation, and --continue launch surfaces; no public attach, peer-message, server, socket, or interrupt command appears in help.
    limitations: Help and binary identity do not prove runtime delivery. The Sniff/version discrepancy means direct version output must be authoritative for this observation.
  - id: local-changelog-1127
    method: local_inspection
    location: sanitized output of agy changelog from installed 1.1.27
    version: release entries 1.1.15 through 1.1.27
    observed_on: 2026-09-08
    claim: Version 1.1.15 added --input-format stream-json, reading one NDJSON message per line and running one turn per message in a single conversation; later entries describe typed init, step_update, and result output, Windows fixes, state-stream fixes, and a separate Remote Control feature.
    limitations: Release notes do not provide the complete input schema, early acknowledgment, concurrent-input semantics, queue rules, authentication, protocol negotiation, or proof on this host.
  - id: local-binary-strings
    method: source_code
    location: sanitized static strings from the installed Google-signed agy 1.1.27 binary
    version: 1.1.27
    observed_on: 2026-09-08
    claim: The binary describes input-format stream-json as NDJSON stdin requiring output-format stream-json and contains internal sidecar/remote-control RPC names including StreamAgentStateUpdates, GetConversationMetadata, GetAllCascadeTrajectories, DeleteQueuedUserInputStep, and SendUserMessage-shaped data.
    limitations: Static strings do not establish externally reachable endpoints, exact framing, authentication, ordinary-session availability, or safe third-party compatibility; internal methods are not recorded as steering mechanisms.
  - id: local-state
    method: local_inspection
    location: sanitized filename-only inventory below ~/.gemini/antigravity-cli plus prior sanitized schema observations
    version: local state used by 1.1.27; individual artifact writer versions unknown
    observed_on: 2026-09-08
    claim: Antigravity maintains user state under ~/.gemini/antigravity-cli, including conversation summaries, logs, installation identity, per-conversation SQLite data, and transcript mirrors described by earlier research.
    limitations: Historical files do not prove process attachment, liveness, working/idle state, or a writable endpoint; no credential or conversation content was read.
  - id: prior-resume
    method: local_inspection
    location: claudine/docs/research/resume/antigravity.md, rechecked against 1.1.27 help and state filenames
    version: prior observations at 1.1.0; selectors rechecked at 1.1.27
    observed_on: 2026-09-08
    claim: Conversation UUIDs can be consumed by --conversation and persisted history can be resumed by a new process, while --continue is a latest-session selector unsuitable for deduplicated automation.
    limitations: Resume starts or reopens work and is not delivery to the original live process; prior content is a lead where not reverified.
  - id: wrapper-source
    method: source_code
    location: claudine/cli/src/commands/wrap/profile/antigravity.rs
    version: workspace state observed 2026-09-08
    observed_on: 2026-09-08
    claim: The current Claudine Antigravity wrapper launches one-shot print requests and does not retain structured stdin or register a peer-control endpoint.
    limitations: Claudine implementation is not evidence of intrinsic provider capability.
  - id: ordinary-no-endpoint
    method: inference
    location: comparison of 1.1.27 help, embedded changelog, static binary surfaces, and current wrapper behavior
    version: 1.1.27
    observed_on: 2026-09-08
    claim: Ordinary sessions have no established independent-sender transport; terminal keystrokes, history resume, internal sidecar RPC names, and Remote Control must not be equated with a supported prompt RPC.
    limitations: Absence of an exposed contract does not prove impossibility, so ordinary cases remain unknown rather than unsupported.
discovery:
  - { id: ordinary-macos-native, profile_id: ordinary-cli, os: macos, origin: native, method: process_inspection, locator: "Sniff process observation plus ~/.gemini/antigravity-cli conversation metadata", identity_check: "Correlate same-user executable, process start identity, CWD/project, and conversation UUID; reject PID-only or newest-history matching.", liveness_check: "Process identity can show a candidate is alive but not which conversation it owns.", state_detection: "No documented passive working/idle signal for an ordinary process.", observation_source: "sniff plus passive provider metadata", observed_at: runtime, available_labels: [pid, executable, cwd, conversation_id, project_id], prerequisites: [same OS user, readable process and provider metadata], evidence_ids: [local-cli-1127, local-state, prior-resume] }
  - { id: ordinary-macos-claudine, profile_id: ordinary-cli, os: macos, origin: claudine, method: claudine_registration, locator: "Claudine child launch record plus Sniff and provider metadata", identity_check: "Bind process start identity and captured conversation UUID; current wrapper does not capture a live endpoint.", liveness_check: "Registered child/process existence only.", state_detection: "No provider-backed working/idle signal is registered.", observation_source: "Claudine launch record, sniff, provider metadata", observed_at: runtime, available_labels: [pid, cwd, origin, conversation_id], prerequisites: [Claudine launch registration], evidence_ids: [wrapper-source, local-state] }
  - { id: ordinary-linux-native, profile_id: ordinary-cli, os: linux, origin: native, method: process_inspection, locator: "Sniff Linux process observation plus home-relative Antigravity metadata", identity_check: "Require executable/start identity, CWD/project, and conversation UUID; WSL is Linux-side evidence only.", liveness_check: "Candidate process only; conversation ownership is unresolved.", state_detection: "Unknown.", observation_source: "sniff plus provider metadata", observed_at: runtime, available_labels: [pid, executable, cwd, conversation_id], prerequisites: [same Linux user], evidence_ids: [prior-resume, ordinary-no-endpoint] }
  - { id: ordinary-linux-claudine, profile_id: ordinary-cli, os: linux, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff Linux process observation", identity_check: "Bind start identity and capture conversation UUID rather than infer from history order.", liveness_check: "Registered child identity only.", state_detection: "Unknown.", observation_source: "Claudine registry, sniff, provider metadata", observed_at: runtime, available_labels: [pid, cwd, origin, conversation_id], prerequisites: [Claudine launch registration], evidence_ids: [wrapper-source, prior-resume] }
  - { id: ordinary-windows-native, profile_id: ordinary-cli, os: windows, origin: native, method: process_inspection, locator: "Sniff native-Windows process observation plus user-profile Antigravity metadata", identity_check: "Require process-token user scope, executable/start identity, CWD/project, and conversation UUID.", liveness_check: "Candidate process only; native Windows is untested.", state_detection: "Unknown.", observation_source: "sniff plus provider metadata", observed_at: runtime, available_labels: [pid, executable, cwd, conversation_id], prerequisites: [same Windows process-token user], evidence_ids: [prior-resume, local-changelog-1127] }
  - { id: ordinary-windows-claudine, profile_id: ordinary-cli, os: windows, origin: claudine, method: claudine_registration, locator: "Claudine launch record plus Sniff native-Windows process observation", identity_check: "Bind process-token/start identity and captured conversation UUID.", liveness_check: "Registered child identity only; native Windows untested.", state_detection: "Unknown.", observation_source: "Claudine registry, sniff, provider metadata", observed_at: runtime, available_labels: [pid, cwd, origin, conversation_id], prerequisites: [Claudine launch registration], evidence_ids: [wrapper-source, prior-resume] }
  - { id: stream-macos-claudine, profile_id: managed-stream-json, os: macos, origin: claudine, method: claudine_registration, locator: "Registered retained child pipes, init event, and conversation ID", identity_check: "Bind process start identity, exclusive pipe ownership, CWD/project, and init conversation identity.", liveness_check: "Open pipes and matching child start identity; no protocol ping is documented.", state_detection: "Outstanding input turn versus terminal result in Claudine's request ledger.", observation_source: "future managed-launch registry and stream-json ledger", observed_at: runtime, available_labels: [pid, cwd, conversation_id, working, idle], prerequisites: [future retained-stream launcher], evidence_ids: [local-changelog-1127, local-binary-strings] }
  - { id: stream-linux-claudine, profile_id: managed-stream-json, os: linux, origin: claudine, method: claudine_registration, locator: "Registered retained child pipes, init event, and conversation ID", identity_check: "Bind Linux process start identity, pipe ownership, CWD/project, and conversation identity.", liveness_check: "Open pipes and matching child identity; Linux runtime untested.", state_detection: "Outstanding input versus terminal result in Claudine's ledger.", observation_source: "future managed-launch registry and stream-json ledger", observed_at: runtime, available_labels: [pid, cwd, conversation_id, working, idle], prerequisites: [future retained-stream launcher], evidence_ids: [local-changelog-1127] }
  - { id: stream-windows-claudine, profile_id: managed-stream-json, os: windows, origin: claudine, method: claudine_registration, locator: "Registered retained Windows child pipes, init event, and conversation ID", identity_check: "Bind process-token/start identity, pipe ownership, CWD/project, and conversation identity.", liveness_check: "Open pipes and matching child identity; native Windows runtime untested.", state_detection: "Outstanding input versus terminal result in Claudine's ledger.", observation_source: "future managed-launch registry and stream-json ledger", observed_at: runtime, available_labels: [pid, cwd, conversation_id, working, idle], prerequisites: [future retained-stream launcher], evidence_ids: [local-changelog-1127] }
mechanisms:
  - id: retained-ndjson-idle
    interface_status: documented
    maturity: experimental
    transport: stdio
    initialization: "Launch agy print mode with --input-format stream-json and --output-format stream-json; retain stdin/stdout; parse init before registering the conversation."
    operation_intent: start_idle_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: idle_turn_start
    destination: The single conversation owned by the retained agy child; the input line has no independently verified destination field.
    authentication: Provider authentication inherited/configured at launch; transport access relies on exclusive child-pipe ownership rather than an application-layer sender credential.
    startup_requirements: [managed retained process, exact parsed init conversation identity, idle request ledger, preserved provider configuration]
    target_preconditions: [child alive, stdin open, init accepted, no outstanding turn]
    target_guards: [Claudine must verify its registered process start identity and idle ledger, no provider expected-operation guard is established]
    request_framing: One newline-delimited JSON message on retained stdin; the complete input object schema was not available in passive documentation.
    response_framing: Newline-delimited typed init, step_update, and terminal result events on stdout.
    request_format: NDJSON message; exact required fields unknown.
    response_format: stream-json with typed init, step_update, and result events; no formal public schema was retrieved.
    long_tool_behavior: Only idle submission is claimed. Behavior when another line arrives during a long-running tool is unknown and must not be treated as immediate or queued delivery.
    tool_batch_behavior: unknown
    queue_behavior: Release notes say one turn per input message in one conversation, but buffering, ordering under concurrent writes, drain points, crash persistence, and input received while working are unknown.
    message_interpretation: provider_defined
    interruption_phases: []
    interruption_partial_failure: not applicable to the idle-only mechanism
    ordering: Claudine must serialize inputs and wait for the prior terminal result; provider ordering beyond this discipline is unknown.
    sender_message_id: unknown
    retry_policy: never_retry
    duplicate_handling: No duplicate-suppression key was established; an ambiguous write or missing result is unsafe to retry.
    cancellation: No structured-input cancel operation was documented. Closing pipes or signaling the process is process interruption, not message cancellation.
    limits: Message-size, line-size, queue-depth, timeout, and retained-session lifetime limits are unknown; model context and print timeout may apply.
    evidence_ids: [local-changelog-1127, local-binary-strings]
compatibility:
  - { mechanism_id: retained-ndjson-idle, profile_id: managed-stream-json, os: macos, versions_verified: [], documented_version_bounds: "Introduced in 1.1.15 according to the 1.1.27 embedded changelog; no upper bound or compatibility guarantee documented.", read_only_check: "Require agy --version >= 1.1.15 only as a feature lead, then inspect help/static capability markers and require an allowlisted exact version; do not send during compatibility inspection.", success_criteria: "Exact version reviewed, both input/output stream-json flags present, and a matching disposable-session verification record exists before activation.", failure_behavior: "Fail closed and offer no delivery; do not fall back to terminal injection or concurrent resume.", evidence_ids: [local-changelog-1127, local-binary-strings] }
  - { mechanism_id: retained-ndjson-idle, profile_id: managed-stream-json, os: linux, versions_verified: [], documented_version_bounds: "Introduced in 1.1.15 release notes; Linux range not independently documented.", read_only_check: "Use Sniff to resolve binary/version and require an exact reviewed allowlist entry plus Linux disposable verification.", success_criteria: "Reviewed exact version and matching Linux live-test record.", failure_behavior: "Fail closed.", evidence_ids: [local-changelog-1127] }
  - { mechanism_id: retained-ndjson-idle, profile_id: managed-stream-json, os: windows, versions_verified: [], documented_version_bounds: "Introduced in 1.1.15 release notes; later notes mention Windows fixes but no supported range.", read_only_check: "Use Sniff native-Windows discovery, direct version output, capability markers, and an exact reviewed allowlist; WSL does not qualify.", success_criteria: "Reviewed exact native-Windows version and matching live-test record with pipe framing verified.", failure_behavior: "Fail closed.", evidence_ids: [local-changelog-1127] }
verification: []
cases:
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-cli-1127, ordinary-no-endpoint], reason: "No peer delivery endpoint is established for an ordinary working TUI." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-cli-1127, ordinary-no-endpoint], reason: "Human keyboard input is not an independent sender protocol." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Wrapping does not expose a peer channel." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source, ordinary-no-endpoint], reason: "Launch registration alone is not delivery." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [local-cli-1127, ordinary-no-endpoint], reason: "One-shot print input is not retained steering unless explicitly launched in stream-json input mode." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-native], mechanism_ids: [], prerequisites: [], evidence_ids: [prior-resume, ordinary-no-endpoint], reason: "Ordinary one-shot lifetime has no established idle receiver." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "Current wrapper does not retain stdin." }
  - { profile_id: ordinary-cli, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-macos-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "Current one-shot child does not remain an idle receiver." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [ordinary-no-endpoint], reason: "No documented peer endpoint; Linux untested." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [ordinary-no-endpoint], reason: "Terminal input is not a stable protocol." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "No peer channel is registered." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "Discovery does not imply delivery." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [ordinary-no-endpoint], reason: "Ordinary print mode is not a retained receiver." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-native], mechanism_ids: [], prerequisites: [], evidence_ids: [prior-resume], reason: "A later resume is a new process, not steering the original." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "Current wrapper is one-shot." }
  - { profile_id: ordinary-cli, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-linux-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "No retained idle process is established." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [ordinary-no-endpoint], reason: "Native Windows peer delivery is undocumented and untested." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [ordinary-no-endpoint], reason: "Console keystroke injection is not provider messaging." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "No peer channel is registered." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "Discovery alone is insufficient." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [ordinary-no-endpoint], reason: "Ordinary one-shot input is not active-turn steering." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-native], mechanism_ids: [], prerequisites: [], evidence_ids: [prior-resume], reason: "Resume does not target the original live process." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "Current wrapper is one-shot." }
  - { profile_id: ordinary-cli, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [ordinary-windows-claudine], mechanism_ids: [], prerequisites: [], evidence_ids: [wrapper-source], reason: "No retained receiver exists." }
  - { profile_id: managed-stream-json, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [stream-macos-claudine], mechanism_ids: [], prerequisites: [managed retained launch, active-turn semantics research, disposable live test], evidence_ids: [local-changelog-1127], reason: "One-turn-per-line does not establish whether a line sent during generation or a long tool is accepted, queued, immediate, or canceling." }
  - { profile_id: managed-stream-json, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [stream-macos-claudine], mechanism_ids: [retained-ndjson-idle], prerequisites: [managed retained launch, exact conversation identity, idle ledger, disposable live test], evidence_ids: [local-changelog-1127, local-binary-strings], reason: "Release notes establish sequential turns in one retained conversation; activation remains blocked by empty verification and unknown receipt semantics." }
  - { profile_id: managed-stream-json, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [stream-linux-claudine], mechanism_ids: [], prerequisites: [managed retained launch, Linux disposable live test], evidence_ids: [local-changelog-1127], reason: "Active-turn semantics and Linux runtime behavior are unknown." }
  - { profile_id: managed-stream-json, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [stream-linux-claudine], mechanism_ids: [retained-ndjson-idle], prerequisites: [managed retained launch, exact conversation identity, Linux disposable live test], evidence_ids: [local-changelog-1127], reason: "Documented retained sequential-turn capability is portable evidence, but Linux is not tested and activation remains blocked." }
  - { profile_id: managed-stream-json, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [stream-windows-claudine], mechanism_ids: [], prerequisites: [managed retained launch, native Windows disposable live test], evidence_ids: [local-changelog-1127], reason: "Active-turn and native-Windows pipe behavior are unknown." }
  - { profile_id: managed-stream-json, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [stream-windows-claudine], mechanism_ids: [retained-ndjson-idle], prerequisites: [managed retained launch, exact conversation identity, native Windows disposable live test], evidence_ids: [local-changelog-1127], reason: "Capability is documented in release notes, but native Windows is not tested and activation remains blocked." }
gaps:
  - area: mandatory activation testing
    detail: verification is empty; no Antigravity steering mechanism is eligible for Claudine activation.
    next_check: Run isolated retained-stream sessions for each exact version/OS/profile/state after coordinator approval.
  - area: active-turn steering
    detail: No evidence establishes delivery during token generation or a long-running tool. The retained reader may block, buffer, queue, reject, or alter the running turn.
    next_check: Submit uniquely identified lines during generation, during a long tool, and during a multi-tool batch; assert timing, current-turn outcome, ordering, and conversation identity.
  - area: interruption fallback
    detail: Ctrl+C is documented for human interruption, but no externally guarded context-preserving interrupt-and-submit protocol was established. Process signals and terminal keys lack expected-operation guards.
    next_check: In disposable sessions, characterize first Ctrl+C during generation/tool work, preserved transcript, tool/process cleanup, and replacement submission; keep it manual-consent-only if ever adopted.
  - area: framing and receipts
    detail: Complete NDJSON input fields, acceptance acknowledgment, request/message IDs, early errors, persistence, queue depth, duplicate suppression, and size limits remain unknown.
    next_check: Obtain versioned protocol documentation or source and test malformed, oversized, duplicated, delayed, partial, and EOF-terminated inputs with correlated output.
  - area: discovery
    detail: Ordinary historical stores do not bind a conversation to a live process or reveal working/idle state. Managed registration helps only future retained launches.
    next_check: Define process-start identity, conversation-ID capture, request ledger, crash detection, PID reuse rejection, and multiple-session/process tests.
  - area: Remote Control and internal RPC
    detail: Release notes mention Remote Control and the binary contains sidecar RPC names, but endpoint discovery, auth, external-client availability, request framing, exact target guards, and non-interactive receiving behavior were not established.
    next_check: Locate official versioned Remote Control documentation/source and passively identify launch/config prerequisites before any disposable protocol test; do not reverse-engineer live credentials.
  - area: profile preservation
    detail: It is unknown whether retained stream-json mode preserves plugins/extensions, skills, prompt templates, AGENTS.md/rules/context, hooks, MCP, explicit model/effort, sandbox, and permission settings exactly like ordinary launch.
    next_check: Compare inventories and behavior between isolated ordinary and retained launches with identical explicit settings.
  - area: OS and versions
    detail: Only macOS 1.1.27 was inspected. Sniff reported stale 1.1.5, no Linux or native Windows runtime was inspected, WSL would be Linux-side only, and no formal compatibility bounds or protocol negotiation were found.
    next_check: Fix or account for Sniff version parsing, maintain an exact-version allowlist, and perform native tests on all three OS values.
  - area: token-generation loops
    detail: The idle-only retained mechanism cannot currently be claimed to help a generation loop that never reaches a tool boundary; active-input drain behavior is unknown.
    next_check: Measure whether input is read and acknowledged during uninterrupted generation without canceling it; otherwise report no non-interrupting loop relief.
changes:
  - Initial Antigravity steering report using schema revision 2.
  - Refreshed prior 1.1.0 resume and non-interactive findings against installed 1.1.27 help, state layout, and release notes.
  - Added the 1.1.15 retained stream-json input profile while keeping ordinary-session steering and active-turn behavior unknown.
requires_claudine_update: true
reason: A future managed retained-stream launch could provide same-conversation idle turns, but the current wrapper is one-shot, ordinary sessions lack a verified peer endpoint, and all activation is blocked by empty verification and unresolved framing/receipt semantics.
---

# Antigravity CLI steering research

## Overview

Antigravity CLI 1.1.27 has one credible managed control candidate: print mode with `--input-format stream-json --output-format stream-json`. Its embedded 1.1.15 release note says retained stdin accepts one NDJSON message per line and runs one turn per message in a single conversation. This is materially different from ordinary one-shot `--print`, and it is not an attach protocol for an already open TUI or process.

The candidate is limited to starting a turn while the managed conversation is known idle. Passive evidence does not establish what happens if input arrives during generation or a long-running tool, nor does it establish an early acceptance acknowledgment, sender ID, duplicate suppression, or interruption operation. `verification` is empty, so nothing is eligible for activation.

Research execution metadata records Codex `gpt-5.6-sol` with low reasoning effort. This provenance is launcher-supplied; the provider did not expose independent resolved-model/effort execution metadata to this research turn, so no stronger verification is claimed.

## Session discovery

Antigravity stores durable conversation identities and history under `~/.gemini/antigravity-cli`; exact `--conversation <ID>` resume remains present in 1.1.27. History discovery is not live-session discovery. A conversation UUID can outlive a process, a PID can be reused, helper/sidecar processes can exist, and multiple conversations or subagent trajectories can share a runtime. Neither newest history nor PID alone is a safe destination.

Ordinary native discovery can combine Sniff process observations with provider metadata, CWD/project, executable, and process start identity, but it still cannot establish attachment or working/idle state. A future managed retained profile can be exact if Claudine owns the pipes, parses the init conversation identity, records process start identity, and maintains an outstanding-turn ledger. This improves discovery only for future managed launches; it cannot enable an already open ordinary session.

## Non-interrupting delivery

For a registered idle retained process, one NDJSON message is documented to start another turn in the same conversation. That is candidate provider capability, not current Claudine support, and it needs a successful disposable-session test on each applicable OS/version before activation.

No active-turn capability is established. “One turn per message” does not say whether the reader consumes another line while the previous turn is generating or executing a tool. OS pipe buffering can make a write succeed even if the application has not accepted or scheduled the message. Until tested or documented, Claudine must serialize input after the prior terminal result. Consequently the mechanism cannot presently be claimed to help a token-generation loop that never reaches another tool call.

Terminal-keystroke injection is a distinct UI-automation approach, not a stable messaging protocol. It has no provider acknowledgment, exact conversation guard, or portable semantics and is not a verified fallback.

## Interruption fallback

Antigravity release notes describe the first interactive Ctrl+C as canceling active operations and a double press as entering exit flow. That does not establish an independent-sender interrupt API, which operation is guarded, how a long-running child tool is terminated, or whether the transcript is ready for a replacement message. Sending a process signal or injected key could race with turn completion and target the wrong operation.

No interruption mechanism is therefore recorded. A future disposable test would need explicit human approval, then ordered phases: verify expected process/conversation/operation; interrupt; observe a provider terminal cancellation state; verify the same conversation is idle; submit; correlate a terminal result. If interruption succeeded but submission failed, the prior turn would be lost and the new message absent; no queue retention is known. Automatic loop warnings must never take this path.

## Idle sessions

An ordinary idle TUI accepts its human user's keyboard entry, but no provider-supported channel for an independent sender was found. An ordinary one-shot process normally exits; resuming its UUID in a new process is continuation, not steering the original process.

The managed retained stream-json process is intentionally different: it remains open between completed turns and can receive the next NDJSON line. A submitted line starts a turn. Permission prompts that arise later during tool execution are not message-delivery holds and must not be reported as such.

## Protocol details

The documented passive framing is NDJSON stdin paired with stream-json stdout. Output uses typed `init`, `step_update`, and terminal `result` events. The exact input object fields were not available in the retrieved documentation, and no formal versioned schema was found. A pipe write has no receipt meaning by itself; the first observable provider event, correlation rules, errors, timeouts, message limits, and behavior after partial lines or EOF remain unverified.

Static strings in the signed binary include internal sidecar and Remote Control concepts, conversation metadata, agent-state streaming, queued-input deletion, and send-user-message-shaped structures. These are useful leads only. They do not reveal a supported external endpoint, authentication, request/response framing, version handshake, expected-operation guard, or availability in ordinary sessions. They are deliberately not promoted to mechanism records.

Text interpretation is provider-defined. Antigravity has slash commands, skills, plugins, rules, hooks, agents, and input transformations; a future adapter must determine whether command-looking NDJSON text is literal or invokes those surfaces. Transport selection must preserve the user's plugins, skills, prompt templates, context/rules, hooks, MCP configuration, model/effort, sandbox, and permissions rather than silently changing behavior.

## OS/version compatibility

Only native macOS and the installed Google-signed arm64 1.1.27 binary were inspected. Sniff identified the executable but reported 1.1.5, while direct `agy --version` reported 1.1.27; compatibility code must not trust that stale parse without correction. Release notes mention Windows-specific fixes, but they are not native Windows runtime proof. Linux is also untested, and WSL evidence would apply only to Linux-side behavior.

The feature was introduced in 1.1.15 according to the 1.1.27 changelog, but no upper bound or protocol compatibility promise was found. Safe selection requires passive version/capability inspection, an exact reviewed allowlist, and matching disposable verification. Unknown versions fail closed.

## Disposable-test proposals

For each exact version on macOS, Linux, and native Windows, launch an isolated managed stream-json process with test-only configuration. Parse init and capture its conversation ID; send nonce A while idle; require correlated step/result output; send nonce B; and prove same-conversation memory rather than trusting the ID alone. Close and resume separately to distinguish retained identity from persisted-history continuation.

Then test input during pure token generation, a long-running tool, and a multi-tool batch. Record whether the line is read, acknowledged, queued, reordered, duplicated, delivered in the current or next turn, or causes cancellation. Fault cases should cover malformed and partial NDJSON, oversized input, unknown fields, duplicate messages, broken pipes, output timeout, crash/restart, EOF, and ambiguous writes. A separate consented interruption experiment should measure context preservation, tool/child-process cleanup, remaining batch behavior, and the failure window before replacement submission.

Finally compare ordinary and retained launches using the same explicit plugins, skills, templates, context/rules, hooks, MCP, model, effort, sandbox, and permissions. These are proposals only; no live test was run.

## Claudine integration

The preferred future profile is a private retained child owned exclusively by Claudine. Claudine should capture the init conversation ID, bind it to process start identity and CWD/project, serialize inputs, track working versus idle from terminal results, preserve unknown output events, and refuse delivery on protocol/version drift. This is non-interactive execution: Claudine is the client, so delivery itself does not depend on a receiving human, though later provider tool approvals may still require policy handling.

Once verified, the fallback is idle-only retained NDJSON delivery. When the managed session is working, or when only an ordinary session exists, warn that Antigravity has no verified non-interrupting active-turn/peer protocol and decline. Claudine must not silently start a second `--conversation` process, inject terminal keys, enable Remote Control, install extensions, alter configuration, or interrupt work.

## Gaps

The blockers are empty live verification; unknown active-turn input behavior; no established external interruption API; incomplete input schema; unavailable acceptance acknowledgment; no sender ID/idempotency or expected-operation guard; unknown tool-batch and crash persistence behavior; no exact ordinary-session attachment/liveness signal; unverified Remote Control/internal RPC access; unknown profile preservation; and absent cross-platform/version guarantees.

## Sources

Primary public entry point: [Antigravity CLI product page](https://antigravity.google/product/antigravity-cli). Versioned facts came from the signed installed 1.1.27 executable's help and embedded changelog, including the 1.1.15 retained stream-json addition. Passive local evidence used Sniff, executable metadata, static strings, sanitized filenames, and the current Claudine wrapper. Earlier Antigravity resume research was rechecked as a lead rather than treated as current proof. No credentials, conversation content, live delivery, or focused terminal/browser windows were used.

## Changelog

- 2026-09-08: Created the schema-revision-2 report, refreshed prior 1.1.0 findings against 1.1.27, and separated ordinary launches from the managed retained stream-json candidate.
