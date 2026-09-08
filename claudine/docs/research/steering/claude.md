---
$schema: ./_schema.yaml
schema_revision: 1
provider: claude
created: 2026-09-08
last_updated: 2026-09-08
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
  - "Claude Code 2.1.263 (local macOS installation)"
  - "Claude Code >=2.1.224 (documented macOS/Linux cross-session baseline)"
  - "Claude Code >=2.1.234 (documented native Windows cross-session baseline)"
access_findings:
  - mechanism_id: peer-unix
    os: macos
    status: available
    prerequisite: "Same OS user, eligible non-bare live session, validated socket, and inbound policy that does not refuse; availability is researched and still requires an implemented adapter plus live verification."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session, local-registry-2-1-263, local-binary-2-1-263]
  - mechanism_id: peer-unix
    os: linux
    status: available
    prerequisite: "Same Linux user and filesystem/socket namespace, eligible non-bare live session, validated socket, and inbound policy that does not refuse; WSL 2 is Linux-side."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-windows-pipe
    os: windows
    status: blocked
    prerequisite: "The independent sender must obtain the target session's generated authentication token through a supported interface; none was found. Parent launch does not establish token access."
    applies_to_existing_sessions: unknown
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: startup-channel
    os: macos
    status: setup_required
    prerequisite: "Install and approve a channel plugin/MCP server, configure sender gating and ingress, then launch a new Claude Code session with the channel enabled."
    applies_to_existing_sessions: no
    evidence_ids: [official-channels, official-cli]
  - mechanism_id: startup-channel
    os: linux
    status: setup_required
    prerequisite: "Install and approve a channel plugin/MCP server, configure sender gating and ingress, then launch a new Claude Code session with the channel enabled."
    applies_to_existing_sessions: no
    evidence_ids: [official-channels, official-cli]
  - mechanism_id: startup-channel
    os: windows
    status: setup_required
    prerequisite: "Install and approve a channel plugin/MCP server, configure sender gating and ingress, then launch a new Claude Code session with the channel enabled."
    applies_to_existing_sessions: no
    evidence_ids: [official-channels, official-cli]
delivery_states:
  - mechanism_id: peer-unix
    states: [accepted, queued, held, delivered, refused, expired]
    observable_by_external_sender: partial
    correlation: "The built-in SendMessage path documents sender notices through held, delivered, refused, and expired outcomes. Packaged 2.1.263 source contains msg_id/orig_msg_id status correlation for raw peer transport, but the external wire response schema is not a stable documented contract and was not live-tested."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: peer-windows-pipe
    states: [accepted, queued, held, delivered, refused, expired]
    observable_by_external_sender: partial
    correlation: "Provider-owned SendMessage exposes lifecycle notices. Packaged source indicates message-ID status correlation, but an independent pipe client lacks both a demonstrated credential and a stable documented response contract."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: startup-channel
    states: [accepted, queued, delivered, unknown]
    observable_by_external_sender: partial
    correlation: "The channel ingress can acknowledge receipt and the MCP notification call can complete, but neither proves Claude consumed the event. No provider-wide delivery correlation identifier or terminal state is documented."
    evidence_ids: [official-channels]
evidence:
  - id: official-cross-session
    method: official_docs
    location: "https://code.claude.com/docs/en/cross-session-messaging"
    version: "macOS/Linux >=2.1.224; native Windows >=2.1.234; provider and feature-flag exceptions require >=2.1.248"
    observed_on: 2026-09-08
    claim: "Claude Code documents same-machine session discovery and non-interrupting delivery over per-session Unix sockets or Windows named pipes, including working, idle, and long-running -p sessions."
    limitations: "Documents provider behavior, not a stable external management API or a Claudine implementation; no disposable delivery test was run."
  - id: official-remote-control
    method: official_docs
    location: "https://code.claude.com/docs/en/remote-control"
    version: "Remote Control >=2.1.51; cross-session details on this page include later version gates"
    observed_on: 2026-09-08
    claim: "Remote Control keeps an explicitly enabled local conversation synchronized through outbound Anthropic TLS and queues prompts submitted mid-turn."
    limitations: "Requires eligible claude.ai authentication and explicit or configured activation; the public interface is a user-facing web/mobile surface, not a documented local Claudine API."
  - id: official-channels
    method: official_docs
    location: "https://code.claude.com/docs/en/channels-reference"
    version: "Channels research preview >=2.1.80"
    observed_on: 2026-09-08
    claim: "A startup-configured MCP channel subprocess can push notifications over stdio into the same running Claude Code session."
    limitations: "Requires installing and launching with a channel; it cannot discover or attach to arbitrary already-running sessions."
  - id: official-cli
    method: official_docs
    location: "https://code.claude.com/docs/en/cli-reference"
    version: "current documentation observed 2026-09-08"
    observed_on: 2026-09-08
    claim: "The CLI documents -p, stream-json input, background sessions, channels, resume, and Remote Control launch surfaces."
    limitations: "CLI options alone do not prove message delivery or persistence under every launch combination."
  - id: local-help-2-1-263
    method: local_inspection
    location: "sanitized output of claude --version, claude --help, claude agents --help, and claude agents --json on the research host"
    version: "2.1.263"
    observed_on: 2026-09-08
    claim: "The installed arm64 macOS native binary exposes stream-json input, agent listing, background, Remote Control, and related session controls; agents --json passively listed interactive and sdk-cli sessions."
    limitations: "One macOS host only; no message was sent and no Windows/Linux runtime was inspected."
  - id: local-registry-2-1-263
    method: local_inspection
    location: "sanitized field-and-file-mode inspection of ~/.claude/sessions/*.json and /tmp/cc-socks on the research host"
    version: "2.1.263"
    observed_on: 2026-09-08
    claim: "PID-named JSON records contained pid, sessionId, cwd, procStart, version, peerProtocol, peerFeatures, kind, entrypoint, pidDomain, messagingSocketPath, name, and optional status; live PID-matching mode-0600 sockets existed under a private directory."
    limitations: "Values and credentials were not retained; status was absent for active sdk-cli records; no sibling .key files existed; stale-record cleanup and atomicity were not tested."
  - id: local-binary-2-1-263
    method: source_code
    location: "sanitized static string/source inspection of the installed Claude Code 2.1.263 executable"
    version: "2.1.263"
    observed_on: 2026-09-08
    claim: "The packaged executable contains auth-first newline-delimited JSON examples, message IDs and status records, next-tool-round language, Unix socket validation, and Windows named-pipe handling consistent with the official documentation."
    limitations: "Minified packaged source is implementation evidence for one build, not a stable protocol contract; no secret values were read or printed."
discovery:
  - id: registry-macos-native
    os: macos
    origin: native
    method: provider_registry
    locator: "Enumerate same-user ~/.claude/sessions/*.json and retain entries whose messagingSocketPath names an existing same-user socket."
    identity_check: "Use sessionId as conversation identity; match pid and procStart to the live process to reject PID reuse; verify version, pidDomain, and endpoint ownership."
    liveness_check: "Match pid plus process start time and require the advertised socket to exist; a connect succeeds only as a transport probe and does not prove acceptance."
    state_detection: "Use status/statusUpdatedAt when present; absence is unknown, not working."
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-macos-claudine
    os: macos
    origin: claudine
    method: provider_registry
    locator: "Use the same provider registry; correlate the wrapper child PID/sessionId with Claudine launch metadata."
    identity_check: "Provider sessionId remains authoritative; verify child PID and procStart and deduplicate wrapper observations by sessionId."
    liveness_check: "Require the provider child process and advertised socket; wrapper liveness alone is insufficient."
    state_detection: "Use provider status if present and Claudine state only as an additional label."
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-linux-native
    os: linux
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code session registration files and validate advertised sockets, normally under /run/user/<uid>/cc-socks or an accepted private fallback."
    identity_check: "Use sessionId plus pid/procStart and endpoint ownership; treat WSL 2 as Linux-side only."
    liveness_check: "Require matching live process and same-user socket."
    state_detection: "Use status timestamps when present; otherwise unknown."
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and filesystem namespace", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-linux-claudine
    os: linux
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart; deduplicate by sessionId."
    liveness_check: "Require provider child and socket, not merely the wrapper."
    state_detection: "Provider status is primary; missing status is unknown."
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and namespace", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-native
    os: windows
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code registration files and validate the advertised per-session named pipe."
    identity_check: "Use sessionId plus pid/procStart and validate the named-pipe owner/security descriptor."
    liveness_check: "Require matching process and connectable pipe; connection alone does not establish authenticated acceptance."
    state_detection: "Use status timestamps when present; otherwise unknown."
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-claudine
    os: windows
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart and pipe ownership; deduplicate by sessionId."
    liveness_check: "Require provider child and authenticated pipe availability."
    state_detection: "Provider status is primary; missing status is unknown."
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
mechanisms:
  - id: peer-unix
    interface_status: documented
    transport: unix_socket
    conversation_effect: preserve_running_turn
    delivery_boundary: next_tool_boundary
    acknowledgment: accepted
    destination: "The target session's advertised per-session Unix socket on macOS/Linux/WSL 2."
    authentication: "An auth line with the target's per-session token is optional on macOS/Linux. Same-user endpoint permissions and inbound policy still apply. Each target exports its own token to its hooks/commands; it is never inherited from a parent."
    startup_requirements: ["Claude Code >=2.1.224, or >=2.1.248 for documented provider/feature-flag exceptions", "not --bare", "acceptable socket directory", "crossSessionInbound does not refuse"]
    request_format: "Newline-delimited JSON. The documented optional first line is {type: auth, token: <redacted>}; packaged 2.1.263 source shows a following {type: user, message: {role: user, content: <text>}} line."
    response_format: "Packaged source contains message IDs and peer-message status frames. Official docs distinguish held, refused, and delivered sender notices but do not promise a stable raw frame schema."
    long_tool_behavior: "The message queues while a tool runs and is read between tool calls; the tool is never interrupted."
    interruption_effects: "None for peer delivery. It cannot rescue token generation that never reaches another tool boundary; idle delivery starts a new turn."
    ordering: "Accepted messages queue for the recipient; exact concurrent-sender ordering is undocumented."
    duplicate_handling: "Identical repeats in a short window are dropped; repeated-sender traffic is rate-limited."
    cancellation: "No documented sender cancellation after acceptance. Held messages may later be delivered, denied, refused by policy, or expire."
    limits: "Plain text; about one million serialized characters same-machine; burst refusal; at most 50 accepted messages queued and 100 policy-held messages."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - id: peer-windows-pipe
    interface_status: documented
    transport: named_pipe
    conversation_effect: preserve_running_turn
    delivery_boundary: next_tool_boundary
    acknowledgment: accepted
    destination: "The target session's advertised per-session native Windows named pipe."
    authentication: "The first NDJSON line must authenticate with the target session's token. Claude Code exports that per-session token only inside the target session and never inherits one from a parent, so an independent Claudine process has no documented credential source."
    startup_requirements: ["Claude Code >=2.1.234, or >=2.1.248 for documented provider/feature-flag exceptions", "not --bare", "valid target token", "crossSessionInbound does not refuse"]
    request_format: "Auth-first newline-delimited JSON, followed by a user-message frame; exact user frame is source-derived for 2.1.263 rather than fully specified as a stable public wire contract."
    response_format: "Sender-visible held/refused/delivered results are documented through Claude Code; raw pipe response framing is not a stable documented API."
    long_tool_behavior: "Queues until the next tool boundary without interrupting the tool."
    interruption_effects: "None; cannot rescue a generation loop that never reaches a tool boundary."
    ordering: "Exact concurrent-sender ordering is undocumented."
    duplicate_handling: "Same documented repeat and rate-limit behavior as local peer messaging."
    cancellation: "No documented cancellation after acceptance."
    limits: "Plain text, same-machine size/burst/queue limits; direct external use additionally blocked by token access."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - id: startup-channel
    interface_status: documented
    transport: stdio
    conversation_effect: preserve_running_turn
    delivery_boundary: next_tool_boundary
    acknowledgment: queued
    destination: "A channel MCP subprocess spawned by the target Claude Code session at startup."
    authentication: "Claude Code owns the MCP stdio connection; authentication and sender gating belong to the channel's external ingress."
    startup_requirements: ["Claude Code >=2.1.80", "approved channel or explicit development-channel consent", "MCP configuration and channel capability declaration"]
    request_format: "MCP notification method notifications/claude/channel with content and optional metadata."
    response_format: "MCP notification completion confirms channel handoff, not that Claude consumed or acted on the event."
    long_tool_behavior: "Documentation establishes push into the running session but does not precisely specify long-tool timing."
    interruption_effects: "No documented tool cancellation."
    ordering: "Unknown across multiple producers."
    duplicate_handling: "Channel implementation responsibility; no general provider guarantee found."
    cancellation: "No general recall operation documented."
    limits: "Research-preview feature and preconfigured sessions only."
    evidence_ids: [official-channels, official-cli]
compatibility:
  - mechanism_id: peer-unix
    os: macos
    versions_verified: ["2.1.263 passive structure only"]
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for same-machine messaging on documented provider/feature-flag exceptions"
    read_only_check: "Check claude --version, registration version/peerProtocol, process identity, socket ownership/type, --bare absence when launch metadata is known, and applicable settings."
    success_criteria: "Version eligible, live identity match, same-user non-symlink socket, and inbound policy not refuse; live delivery remains separately required."
    failure_behavior: "Exclude the session or show an explicit compatibility/policy reason."
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - mechanism_id: peer-unix
    os: linux
    versions_verified: []
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for documented exceptions; WSL 2 is Linux-side"
    read_only_check: "Check version, registry/process start identity, socket type/owner, namespace visibility, and settings."
    success_criteria: "Eligible version and a live, same-user, validated endpoint; live delivery remains required."
    failure_behavior: "Exclude or report unavailable."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-windows-pipe
    os: windows
    versions_verified: []
    documented_version_bounds: ">=2.1.234 generally; >=2.1.248 for documented exceptions"
    read_only_check: "Check version, registry/process identity, named-pipe security, settings, and whether a supported target-token source exists."
    success_criteria: "Eligible live session plus target token and valid pipe; no target-token source is currently established for independent Claudine."
    failure_behavior: "Do not offer direct delivery; the provider's own ListAgents/SendMessage remains an agent tool, not an external authentication bridge."
    evidence_ids: [official-cross-session]
  - mechanism_id: startup-channel
    os: macos
    versions_verified: []
    documented_version_bounds: ">=2.1.80 research preview"
    read_only_check: "Inspect launch metadata and MCP/channel configuration."
    success_criteria: "Target was deliberately launched with the configured channel and its ingress is reachable."
    failure_behavior: "Cannot attach after startup; omit channel delivery."
    evidence_ids: [official-channels]
  - mechanism_id: startup-channel
    os: linux
    versions_verified: []
    documented_version_bounds: ">=2.1.80 research preview"
    read_only_check: "Inspect launch metadata and MCP/channel configuration."
    success_criteria: "Configured channel was launched with the session."
    failure_behavior: "Cannot attach after startup."
    evidence_ids: [official-channels]
  - mechanism_id: startup-channel
    os: windows
    versions_verified: []
    documented_version_bounds: ">=2.1.80 research preview"
    read_only_check: "Inspect launch metadata and MCP/channel configuration."
    success_criteria: "Configured channel was launched with the session."
    failure_behavior: "Cannot attach after startup."
    evidence_ids: [official-channels]
verification: []
cases:
  - { os: macos, launch_mode: interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-native], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Documented delivery waits for the next tool boundary and does not interrupt the running tool." }
  - { os: macos, launch_mode: interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-native], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Documented idle delivery starts a new turn." }
  - { os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-claudine], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Wrapper origin does not change the provider inbox semantics." }
  - { os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-claudine], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "An eligible idle provider child starts a turn on delivery." }
  - { os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-native], mechanism_ids: [peer-unix], prerequisites: ["long-running -p process", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Documentation says long-running -p workers bind inboxes and receive messages between tool calls." }
  - { os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-native], mechanism_ids: [peer-unix], prerequisites: ["long-running -p process remains alive", "non-bare inbox", "inbound accept avoids unavailable approval UI"], evidence_ids: [official-cross-session], reason: "A retained idle -p worker can receive, but default-held messages cannot display an approval dialog." }
  - { os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-claudine], mechanism_ids: [peer-unix], prerequisites: ["long-running -p child", "non-bare inbox", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "A passively observed sdk-cli -p child advertised an inbox; live delivery remains unverified." }
  - { os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-claudine], mechanism_ids: [peer-unix], prerequisites: ["long-running -p child remains alive", "non-bare inbox", "launch correlation", "crossSessionInbound accept"], evidence_ids: [official-cross-session], reason: "Documented non-interactive inbox behavior applies while the provider process remains alive." }
  - { os: linux, launch_mode: interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-native], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "same namespace", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented Linux behavior queues until a tool boundary." }
  - { os: linux, launch_mode: interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-native], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "same namespace", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented idle delivery starts a turn." }
  - { os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-claudine], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "same namespace", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Claudine launch does not alter documented provider delivery." }
  - { os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-claudine], mechanism_ids: [peer-unix], prerequisites: ["eligible version", "same namespace", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Eligible idle child starts a turn." }
  - { os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-native], mechanism_ids: [peer-unix], prerequisites: ["long-running -p process", "same namespace", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Long-running -p inbox support is documented." }
  - { os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-native], mechanism_ids: [peer-unix], prerequisites: ["long-running -p remains alive", "same namespace", "non-bare inbox", "crossSessionInbound accept"], evidence_ids: [official-cross-session], reason: "Idle delivery is available while the worker remains alive; accept avoids an unavailable dialog." }
  - { os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-claudine], mechanism_ids: [peer-unix], prerequisites: ["long-running -p child", "same namespace", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented non-interactive peer inbox is origin-independent." }
  - { os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-claudine], mechanism_ids: [peer-unix], prerequisites: ["long-running -p child remains alive", "same namespace", "launch correlation", "crossSessionInbound accept"], evidence_ids: [official-cross-session], reason: "A retained idle child can start a turn from peer delivery." }
  - { os: windows, launch_mode: interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-native], mechanism_ids: [peer-windows-pipe], prerequisites: ["eligible version", "non-bare inbox", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Provider capability is documented, but direct Claudine use lacks a target-token source." }
  - { os: windows, launch_mode: interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-native], mechanism_ids: [peer-windows-pipe], prerequisites: ["eligible version", "non-bare inbox", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented idle peer delivery starts a turn; Claudine authentication remains unresolved." }
  - { os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-claudine], mechanism_ids: [peer-windows-pipe], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Parent launch does not supply the target's generated token back to Claudine." }
  - { os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-claudine], mechanism_ids: [peer-windows-pipe], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Provider support exists, while a Claudine adapter is blocked on authentication." }
  - { os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-native], mechanism_ids: [peer-windows-pipe], prerequisites: ["long-running -p process", "non-bare inbox", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Long-running -p inbox support is documented on eligible native Windows." }
  - { os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-native], mechanism_ids: [peer-windows-pipe], prerequisites: ["long-running -p remains alive", "non-bare inbox", "valid target token", "crossSessionInbound accept"], evidence_ids: [official-cross-session], reason: "Idle delivery can start a turn, but there is no approval UI and Claudine lacks the token." }
  - { os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-claudine], mechanism_ids: [peer-windows-pipe], prerequisites: ["long-running -p child", "non-bare inbox", "launch correlation", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Provider capability is documented; wrapper parenthood does not expose the child's target token." }
  - { os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-claudine], mechanism_ids: [peer-windows-pipe], prerequisites: ["long-running -p child remains alive", "non-bare inbox", "launch correlation", "valid target token", "crossSessionInbound accept"], evidence_ids: [official-cross-session], reason: "Capability requires a retained worker and token; direct Claudine delivery remains blocked." }
gaps:
  - area: activation
    detail: "No disposable-session delivery test exists for any mechanism or case; verification is intentionally empty."
    next_check: "Run isolated same-user disposable sender/receiver sessions per OS/mode/origin and assert conversation identity, queue boundary, idle turn, acknowledgments, and no cancellation."
  - area: windows_authentication
    detail: "Native Windows requires the target session's generated token. It is exported into that session and never inherited from a parent; no supported external credential API was found."
    next_check: "Ask Anthropic for a supported controller API or test a provider-owned relay design; do not scrape another process environment."
  - area: raw_protocol_stability
    detail: "The socket is documented for scripts/hooks, but the complete request/status frame schemas and compatibility negotiation are not specified as a stable API."
    next_check: "Capture sanitized frames in a disposable test and gate by provider version plus peerProtocol; seek an authoritative protocol contract."
  - area: state_detection
    detail: "Active sdk-cli registry records on macOS lacked status; registry state cannot reliably distinguish working from unknown."
    next_check: "Observe disposable sessions across tool, generation, idle, and exit transitions without using absence as a state signal."
  - area: acceptance_vs_delivery
    detail: "An accepted message may still be held, refused, expired, rate-limited, duplicated away, or lost before a future tool boundary."
    next_check: "Require correlated delivered/refused status in disposable tests and expose accepted, held, and delivered separately."
  - area: token_generation_override
    detail: "Official docs state each session exports its own socket and token and never inherits a parent's socket; no evidence shows caller-supplied CLAUDE_CODE_MESSAGING_TOKEN is honored."
    next_check: "Treat caller token injection as unsupported unless Anthropic documents it; a future disposable test may confirm rejection without using real sessions."
changes:
  - "Initial passive research report created from official documentation and sanitized Claude Code 2.1.263 macOS inspection."
requires_claudine_update: true
reason: "Claudine needs registry parsing, identity/liveness checks, policy-aware acknowledgments, OS-specific adapters, compatibility gates, and mandatory disposable verification. Native Windows direct delivery additionally lacks a supported token source."
---

# Claude Code steering research

## Overview

Claude Code now documents cross-session messaging as a native feature. Same-machine sessions use a per-session Unix socket on macOS and Linux (including WSL 2) or a named pipe on native Windows. A receiving session reads a message between tool calls without canceling the running tool; an idle session starts a new turn. This is the strongest candidate for Claudine because it preserves the conversation and requires no special startup mode beyond an eligible, non-`--bare` Claude Code build.

This report was produced by the coordinator-selected Codex research agent using `gpt-5.6-sol` with low reasoning effort. The research was passive. It inspected official documentation, local help, sanitized registry shape and file modes, process identity, and packaged executable strings. It sent no messages and created no live session. Therefore every capability remains blocked from activation by `verification: []`.

## Session discovery

Official documentation says each enabled session registers on disk and binds an inbox. On the macOS research host, version 2.1.263 had PID-named records in `~/.claude/sessions/` and matching sockets under `/tmp/cc-socks/`. Records exposed a provider `sessionId`, PID, process start, working directory, version, protocol number, feature names, launch kind/entrypoint, name, socket path, and sometimes status.

Discovery must treat `sessionId` as conversation identity and PID as a liveness locator. PID plus `procStart` rejects PID reuse. Endpoint ownership/type and the record's OS domain must also match. A wrapper PID is not the Claude process, and several conversations may be resumed over time, so Claudine should deduplicate by current provider session ID after validating the active process. Missing `status` means unknown: two live `sdk-cli` records lacked it while an interactive CLI record said `idle`.

The documentation gives version and endpoint rules but does not publicly specify the registry path or its atomicity. The exact JSON shape is consequently version-sensitive local evidence. Linux and native Windows registry discovery still need passive platform confirmation. WSL 2 is a separate Linux filesystem and socket namespace and cannot discover a native Windows session.

## Non-interrupting delivery

The documented peer mechanism has the desired semantics. While Claude is active, the message waits for the next tool boundary; a running tool is never interrupted. When idle, delivery begins a new turn. Long-running `claude -p` processes bind inboxes too. `--bare` sessions do not.

This boundary cannot rescue a pure token-generation loop that never reaches another tool call. It can steer a long tool workflow after that tool returns. Acceptance is also weaker than delivery: inbound policy can hold or refuse, non-interactive sessions cannot show approval dialogs, burst and duplicate controls can drop messages, and an accepted queue is bounded.

On macOS and Linux the auth line is optional, so a same-user controller can plausibly write to a discovered socket. On native Windows it is mandatory. Each session exports its own generated token inside hooks and Bash commands, and the docs say a session's socket is never inherited from a parent. No evidence shows that a caller-supplied token is honored or that the child reveals its generated token to a Claudine parent. Therefore Windows is a researched provider capability but not currently implementable as direct independent-process delivery.

Claude's `ListAgents` and `SendMessage` tools provide a supported provider-owned bridge between independent sessions. They are tools the model invokes inside a Claude session, not an authenticated external API. Starting a separate Claude conversation merely to ask it to forward a message adds cost, latency, policy ambiguity, and a new-conversation failure point; it should not be mistaken for direct steering.

## Interruption fallback

No interruption is necessary for native peer messaging. This pilot did not test terminal control or signals. A Ctrl+C-style sequence would stop current generation or a tool before a later prompt could be submitted, creating a partial-failure window and changing semantics. Claudine should not use it for automatic loop warnings. If manual interruption is ever added, it needs a distinct design and explicit user choice.

## Idle sessions

An open idle interactive session starts a new turn when a peer message is delivered. A long-running non-interactive worker can do the same while its process remains alive. Because `-p` has no approval dialog, unattended delivery should require an effective `crossSessionInbound: accept`; otherwise a held message may wait until policy changes or expires. A completed process is history, not an idle session, and `--resume` into a new process is resume-only behavior rather than live steering.

## Protocol details

Official docs specify a newline-delimited auth frame and the endpoint security model. Packaged 2.1.263 source additionally exposes a user frame shaped as `{type: user, message: {role: user, content: ...}}`, message IDs, and peer status handling. These details are adequate for a disposable prototype but remain versioned implementation details unless Anthropic publishes the complete framing contract.

The receiver can deliver, hold, or refuse. Senders may receive follow-up notices for local interactive sessions, but raw acknowledgment/status framing is not documented as stable. Messages are plain text, roughly one million serialized characters at most on the same machine, rate-limited in bursts, duplicate-suppressed, and bounded to 50 accepted queued messages. Policy-held storage is separately bounded to 100. There is no documented cancellation after acceptance and no documented total ordering across concurrent senders.

Remote Control is another non-interrupting same-conversation mechanism, but it is user-facing and opt-in. It makes outbound TLS connections to Anthropic, requires eligible claude.ai authentication, and queues mid-turn prompts. No supported local control API for Claudine is documented. Channels are programmatic and documented, but a channel is an MCP subprocess configured when the session starts, so it is useful for deliberately managed sessions rather than arbitrary native discovery.

Setup must remain separate from sending. An eligible ordinary session already running with an inbox needs no transport setup; changing `crossSessionInbound` can release held messages in that existing session, subject to settings precedence. An existing interactive session can enable Remote Control with `/remote-control`, but that is a separate authenticated user setup flow and still exposes no Claudine API. A Channel helps only sessions launched after its plugin/MCP configuration and consent are complete. `--bare` sessions never bind a peer inbox and must be relaunched without `--bare`; a send operation must explain that prerequisite rather than mutate or relaunch the session.

## OS and version compatibility

Cross-session messaging is documented for macOS/Linux/WSL 2 from 2.1.224 and native Windows from 2.1.234. Same-machine use with named alternate providers or disabled feature fetching requires 2.1.248. Runtime checks must also reject `--bare`, unavailable/malformed endpoints, wrong owners, reused PIDs, inaccessible namespaces, and effective `crossSessionInbound: refuse`.

Only macOS 2.1.263 was passively observed. Linux paths and native Windows pipe/token behavior come from official documentation and require platform-specific disposable verification. File existence or a successful socket connection proves neither message acceptance nor delivery.

## Disposable test proposals

Use a temporary home and repository, a no-cost or tightly budgeted model configuration, explicit `crossSessionInbound: accept`, and uniquely tagged messages. Start two disposable sessions per case without focusing a terminal window. Assert target session ID remains unchanged, a long tool is not canceled, delivery appears only after the tool returns, idle delivery starts one turn, sender statuses distinguish accepted/held/refused/delivered, duplicates and size errors are visible, and processes/endpoints are cleaned up.

Run separate matrices for macOS, Linux, WSL 2, and native Windows. Cover native and Claudine launch origins, interactive and persistent `-p` modes, and working/idle states. Windows testing must first establish a supported credential path; failure to obtain the target token should record the direct adapter as unavailable, not weaken authentication. Test Channels and Remote Control only as separate mechanisms with their required startup/account conditions and explicit cost approval.

## Claudine integration

The likely macOS/Linux adapter reads the provider registry, validates session/process/endpoint identity, applies version and settings checks, then speaks the version-gated NDJSON protocol. It must expose accepted, held, delivered, refused, and unknown separately. Selection should show name, repository/working directory, launch mode, provider version, state confidence, and whether delivery waits for a tool boundary.

Claudine-launched sessions should retain child PID/session correlation for clearer labels, while still using provider `sessionId` as identity. A startup Channel could be a supported managed-session alternative. Native Windows direct transport remains blocked unless Anthropic supplies a supported target-token retrieval or controller API.

## Gaps

The largest gap is live verification: none exists. Other blockers are Windows token acquisition, stable raw response framing, reliable working/idle state, registry compatibility and cleanup semantics, and end-to-end acknowledgment correlation. The access findings now separate provider capability from an independent sender's ability to use it, and the delivery-state records preserve the known accepted/queued/held/delivered/refused/expired lifecycle together with the limited external observability.

The schema validates shape but does not enforce exactly 24 unique matrix keys, referenced-ID existence, mechanism/compatibility coverage, or evidence strength. Those checks require a separate relational reviewer or generator validation.

## Sources

- [Cross-session messaging](https://code.claude.com/docs/en/cross-session-messaging)
- [Remote Control](https://code.claude.com/docs/en/remote-control)
- [Channels reference](https://code.claude.com/docs/en/channels-reference)
- [CLI reference](https://code.claude.com/docs/en/cli-reference)

## Changelog

- 2026-09-08: Initial passive report for Claude Code 2.1.263 and the current official compatibility bounds.
