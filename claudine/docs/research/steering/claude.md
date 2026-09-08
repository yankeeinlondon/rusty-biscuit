---
$schema: ./_schema.yaml
schema_revision: 3
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
launch_profiles:
  - id: ordinary-interactive
    description: "Ordinary interactive Claude Code session using the provider-managed same-machine peer inbox."
    endpoint_scope: externally_reachable
    lifetime: while_client_open
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [interactive]
    applicable_origins: [native, claudine]
    baseline: true
    startup_requirements: [eligible long-running -p process, messaging enabled, not --bare, process remains open]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: yes
    preserves_context: yes
    evidence_ids: [official-cross-session, official-cli, local-help-2-1-263]
  - id: ordinary-one-shot
    description: "Ordinary one-shot non-interactive Claude Code process. It is active only while running and cannot be an idle active session after exit."
    endpoint_scope: externally_reachable
    lifetime: one_shot
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [native, claudine]
    baseline: true
    startup_requirements: [ordinary non-interactive launch]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: yes
    preserves_context: yes
    evidence_ids: [official-cross-session, official-cli, local-help-2-1-263]
  - id: retained-noninteractive
    description: "Deliberately retained long-running non-interactive Claude Code process with a provider-managed peer inbox."
    endpoint_scope: externally_reachable
    lifetime: long_lived
    applicable_os: [macos, linux, windows]
    applicable_launch_modes: [non_interactive]
    applicable_origins: [native, claudine]
    baseline: false
    startup_requirements: [eligible long-running -p process, messaging enabled, not --bare, process remains open]
    preserves_extensions: yes
    preserves_skills: yes
    preserves_templates: yes
    preserves_context: yes
    evidence_ids: [official-cross-session, official-cli, local-help-2-1-263]
access_findings:
  - mechanism_id: peer-unix-active
    profile_id: ordinary-interactive
    os: macos
    status: available
    prerequisite: "Same OS user, eligible non-bare live session, validated socket, and inbound policy that does not refuse; availability is researched and still requires an implemented adapter plus live verification."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session, local-registry-2-1-263, local-binary-2-1-263]
  - mechanism_id: peer-unix-idle
    profile_id: ordinary-interactive
    os: macos
    status: available
    prerequisite: "Same OS user, eligible non-bare live session, validated socket, and inbound policy that does not refuse; availability is researched and still requires an implemented adapter plus live verification."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session, local-registry-2-1-263, local-binary-2-1-263]
  - mechanism_id: peer-unix-active
    profile_id: retained-noninteractive
    os: macos
    status: available
    prerequisite: "Same OS user, eligible non-bare live session, validated socket, and inbound policy that does not refuse; availability is researched and still requires an implemented adapter plus live verification."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session, local-registry-2-1-263, local-binary-2-1-263]
  - mechanism_id: peer-unix-idle
    profile_id: retained-noninteractive
    os: macos
    status: available
    prerequisite: "Same OS user, eligible non-bare live session, validated socket, and inbound policy that does not refuse; availability is researched and still requires an implemented adapter plus live verification."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session, local-registry-2-1-263, local-binary-2-1-263]
  - mechanism_id: peer-unix-active
    profile_id: ordinary-interactive
    os: linux
    status: available
    prerequisite: "Same Linux user and filesystem/socket namespace, eligible non-bare live session, validated socket, and inbound policy that does not refuse; WSL 2 is Linux-side."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-unix-idle
    profile_id: ordinary-interactive
    os: linux
    status: available
    prerequisite: "Same Linux user and filesystem/socket namespace, eligible non-bare live session, validated socket, and inbound policy that does not refuse; WSL 2 is Linux-side."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-unix-active
    profile_id: retained-noninteractive
    os: linux
    status: available
    prerequisite: "Same Linux user and filesystem/socket namespace, eligible non-bare live session, validated socket, and inbound policy that does not refuse; WSL 2 is Linux-side."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-unix-idle
    profile_id: retained-noninteractive
    os: linux
    status: available
    prerequisite: "Same Linux user and filesystem/socket namespace, eligible non-bare live session, validated socket, and inbound policy that does not refuse; WSL 2 is Linux-side."
    applies_to_existing_sessions: yes
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-windows-pipe-active
    profile_id: ordinary-interactive
    os: windows
    status: blocked
    prerequisite: "The independent sender must obtain the target session's generated authentication token through a supported interface; none was found. Parent launch does not establish token access."
    applies_to_existing_sessions: unknown
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: peer-windows-pipe-idle
    profile_id: ordinary-interactive
    os: windows
    status: blocked
    prerequisite: "The independent sender must obtain the target session's generated authentication token through a supported interface; none was found. Parent launch does not establish token access."
    applies_to_existing_sessions: unknown
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: peer-windows-pipe-active
    profile_id: retained-noninteractive
    os: windows
    status: blocked
    prerequisite: "The independent sender must obtain the target session's generated authentication token through a supported interface; none was found. Parent launch does not establish token access."
    applies_to_existing_sessions: unknown
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: peer-windows-pipe-idle
    profile_id: retained-noninteractive
    os: windows
    status: blocked
    prerequisite: "The independent sender must obtain the target session's generated authentication token through a supported interface; none was found. Parent launch does not establish token access."
    applies_to_existing_sessions: unknown
    evidence_ids: [official-cross-session, local-binary-2-1-263]
delivery_states:
  - mechanism_id: peer-unix-active
    states: [accepted, queued, held, delivered, refused, expired]
    observable_by_external_sender: partial
    correlation: "The built-in SendMessage path documents sender notices through held, delivered, refused, and expired outcomes. Packaged 2.1.263 source contains msg_id/orig_msg_id status correlation for raw peer transport, but the external wire response schema is not a stable documented contract and was not live-tested."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: peer-unix-idle
    states: [accepted, queued, held, delivered, refused, expired]
    observable_by_external_sender: partial
    correlation: "The built-in SendMessage path documents sender notices through held, delivered, refused, and expired outcomes. Packaged 2.1.263 source contains msg_id/orig_msg_id status correlation for raw peer transport, but the external wire response schema is not a stable documented contract and was not live-tested."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: peer-windows-pipe-active
    states: [accepted, queued, held, delivered, refused, expired]
    observable_by_external_sender: partial
    correlation: "Provider-owned SendMessage exposes lifecycle notices. Packaged source indicates message-ID status correlation, but an independent pipe client lacks both a demonstrated credential and a stable documented response contract."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - mechanism_id: peer-windows-pipe-idle
    states: [accepted, queued, held, delivered, refused, expired]
    observable_by_external_sender: partial
    correlation: "Provider-owned SendMessage exposes lifecycle notices. Packaged source indicates message-ID status correlation, but an independent pipe client lacks both a demonstrated credential and a stable documented response contract."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
receipt_guarantees:
  - mechanism_id: peer-unix-active
    request_acceptance: unknown
    persistence: unknown
    scheduling: unknown
    conversation_delivery: unknown
    provider_signals: ["Provider-owned interactive SendMessage can later report held, delivered, refused, or expired; the independent raw-socket response contract is not documented as stable."]
    correlation: message_id
    evidence_ids: [official-cross-session, local-binary-2-1-263]
    limitations: "A successful socket write or initial raw response must not be treated as acceptance, persistence, scheduling, or delivery until a disposable test establishes the exact frame and correlation behavior."
  - mechanism_id: peer-unix-idle
    request_acceptance: unknown
    persistence: unknown
    scheduling: unknown
    conversation_delivery: unknown
    provider_signals: ["Provider-owned interactive SendMessage can later report held, delivered, refused, or expired; the independent raw-socket response contract is not documented as stable."]
    correlation: message_id
    evidence_ids: [official-cross-session, local-binary-2-1-263]
    limitations: "A successful socket write or initial raw response must not be treated as acceptance, persistence, scheduling, or delivery until a disposable test establishes the exact frame and correlation behavior."
  - mechanism_id: peer-windows-pipe-active
    request_acceptance: unknown
    persistence: unknown
    scheduling: unknown
    conversation_delivery: unknown
    provider_signals: ["Provider-owned SendMessage can expose later lifecycle notices after authenticated transport admission."]
    correlation: message_id
    evidence_ids: [official-cross-session, local-binary-2-1-263]
    limitations: "No supported independent source for the target token and no live pipe exchange were established."
  - mechanism_id: peer-windows-pipe-idle
    request_acceptance: unknown
    persistence: unknown
    scheduling: unknown
    conversation_delivery: unknown
    provider_signals: ["Provider-owned SendMessage can expose later lifecycle notices after authenticated transport admission."]
    correlation: message_id
    evidence_ids: [official-cross-session, local-binary-2-1-263]
    limitations: "No supported independent source for the target token and no live pipe exchange were established."
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
    claim: "The prior passive observation found PID-named JSON records with pid, sessionId, cwd, procStart, version, peerProtocol, peerFeatures, kind, entrypoint, pidDomain, messagingSocketPath, name, and optional status; live PID-matching mode-0600 sockets existed under a private directory. The refresh found no registry directory and no live socket or sibling .key file, consistent with there being no currently registered session."
    limitations: "Values and credentials were not retained; the refresh could not re-observe record shape because no session was registered, and no session was launched under the passive-only constraint. Stale-record cleanup and atomicity remain untested."
  - id: local-binary-2-1-263
    method: source_code
    location: "sanitized static string/source inspection of the installed Claude Code 2.1.263 executable"
    version: "2.1.263"
    observed_on: 2026-09-08
    claim: "The packaged executable contains auth-first newline-delimited JSON examples, message IDs and status records, next-tool-round language, Unix socket validation, and Windows named-pipe handling consistent with the official documentation."
    limitations: "Minified packaged source is implementation evidence for one build, not a stable protocol contract; no secret values were read or printed."
discovery:
  - id: registry-macos-native-interactive
    profile_id: ordinary-interactive
    os: macos
    origin: native
    method: provider_registry
    locator: "Enumerate same-user ~/.claude/sessions/*.json and retain entries whose messagingSocketPath names an existing same-user socket."
    identity_check: "Use sessionId as conversation identity; match pid and procStart to the live process to reject PID reuse; verify version, pidDomain, and endpoint ownership."
    liveness_check: "Match pid plus process start time and require the advertised socket to exist; a connect succeeds only as a transport probe and does not prove acceptance."
    state_detection: "Use status/statusUpdatedAt when present; absence is unknown, not working."
    observation_source: "Provider registry plus passive process/socket inspection."
    observed_at: "2026-09-08 passive macOS observation"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-macos-native-one-shot
    profile_id: ordinary-one-shot
    os: macos
    origin: native
    method: provider_registry
    locator: "Enumerate same-user ~/.claude/sessions/*.json and retain entries whose messagingSocketPath names an existing same-user socket."
    identity_check: "Use sessionId as conversation identity; match pid and procStart to the live process to reject PID reuse; verify version, pidDomain, and endpoint ownership."
    liveness_check: "Match pid plus process start time and require the advertised socket to exist; a connect succeeds only as a transport probe and does not prove acceptance."
    state_detection: "Use status/statusUpdatedAt when present; absence is unknown, not working."
    observation_source: "Provider registry plus passive process/socket inspection."
    observed_at: "2026-09-08 passive macOS observation"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-macos-native-retained
    profile_id: retained-noninteractive
    os: macos
    origin: native
    method: provider_registry
    locator: "Enumerate same-user ~/.claude/sessions/*.json and retain entries whose messagingSocketPath names an existing same-user socket."
    identity_check: "Use sessionId as conversation identity; match pid and procStart to the live process to reject PID reuse; verify version, pidDomain, and endpoint ownership."
    liveness_check: "Match pid plus process start time and require the advertised socket to exist; a connect succeeds only as a transport probe and does not prove acceptance."
    state_detection: "Use status/statusUpdatedAt when present; absence is unknown, not working."
    observation_source: "Provider registry plus passive process/socket inspection."
    observed_at: "2026-09-08 passive macOS observation"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-macos-claudine-interactive
    profile_id: ordinary-interactive
    os: macos
    origin: claudine
    method: provider_registry
    locator: "Use the same provider registry; correlate the wrapper child PID/sessionId with Claudine launch metadata."
    identity_check: "Provider sessionId remains authoritative; verify child PID and procStart and deduplicate wrapper observations by sessionId."
    liveness_check: "Require the provider child process and advertised socket; wrapper liveness alone is insufficient."
    state_detection: "Use provider status if present and Claudine state only as an additional label."
    observation_source: "Provider registry correlated with Claudine launch metadata."
    observed_at: "2026-09-08 passive macOS observation"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-macos-claudine-one-shot
    profile_id: ordinary-one-shot
    os: macos
    origin: claudine
    method: provider_registry
    locator: "Use the same provider registry; correlate the wrapper child PID/sessionId with Claudine launch metadata."
    identity_check: "Provider sessionId remains authoritative; verify child PID and procStart and deduplicate wrapper observations by sessionId."
    liveness_check: "Require the provider child process and advertised socket; wrapper liveness alone is insufficient."
    state_detection: "Use provider status if present and Claudine state only as an additional label."
    observation_source: "Provider registry correlated with Claudine launch metadata."
    observed_at: "2026-09-08 passive macOS observation"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-macos-claudine-retained
    profile_id: retained-noninteractive
    os: macos
    origin: claudine
    method: provider_registry
    locator: "Use the same provider registry; correlate the wrapper child PID/sessionId with Claudine launch metadata."
    identity_check: "Provider sessionId remains authoritative; verify child PID and procStart and deduplicate wrapper observations by sessionId."
    liveness_check: "Require the provider child process and advertised socket; wrapper liveness alone is insufficient."
    state_detection: "Use provider status if present and Claudine state only as an additional label."
    observation_source: "Provider registry correlated with Claudine launch metadata."
    observed_at: "2026-09-08 passive macOS observation"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same OS user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - id: registry-linux-native-interactive
    profile_id: ordinary-interactive
    os: linux
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code session registration files and validate advertised sockets, normally under /run/user/<uid>/cc-socks or an accepted private fallback."
    identity_check: "Use sessionId plus pid/procStart and endpoint ownership; treat WSL 2 as Linux-side only."
    liveness_check: "Require matching live process and same-user socket."
    state_detection: "Use status timestamps when present; otherwise unknown."
    observation_source: "Official documentation; no Linux host observation."
    observed_at: "Not observed on Linux; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and filesystem namespace", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-linux-native-one-shot
    profile_id: ordinary-one-shot
    os: linux
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code session registration files and validate advertised sockets, normally under /run/user/<uid>/cc-socks or an accepted private fallback."
    identity_check: "Use sessionId plus pid/procStart and endpoint ownership; treat WSL 2 as Linux-side only."
    liveness_check: "Require matching live process and same-user socket."
    state_detection: "Use status timestamps when present; otherwise unknown."
    observation_source: "Official documentation; no Linux host observation."
    observed_at: "Not observed on Linux; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and filesystem namespace", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-linux-native-retained
    profile_id: retained-noninteractive
    os: linux
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code session registration files and validate advertised sockets, normally under /run/user/<uid>/cc-socks or an accepted private fallback."
    identity_check: "Use sessionId plus pid/procStart and endpoint ownership; treat WSL 2 as Linux-side only."
    liveness_check: "Require matching live process and same-user socket."
    state_detection: "Use status timestamps when present; otherwise unknown."
    observation_source: "Official documentation; no Linux host observation."
    observed_at: "Not observed on Linux; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and filesystem namespace", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-linux-claudine-interactive
    profile_id: ordinary-interactive
    os: linux
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart; deduplicate by sessionId."
    liveness_check: "Require provider child and socket, not merely the wrapper."
    state_detection: "Provider status is primary; missing status is unknown."
    observation_source: "Official documentation and proposed Claudine launch correlation."
    observed_at: "Not observed on Linux; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and namespace", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-linux-claudine-one-shot
    profile_id: ordinary-one-shot
    os: linux
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart; deduplicate by sessionId."
    liveness_check: "Require provider child and socket, not merely the wrapper."
    state_detection: "Provider status is primary; missing status is unknown."
    observation_source: "Official documentation and proposed Claudine launch correlation."
    observed_at: "Not observed on Linux; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and namespace", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-linux-claudine-retained
    profile_id: retained-noninteractive
    os: linux
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart; deduplicate by sessionId."
    liveness_check: "Require provider child and socket, not merely the wrapper."
    state_detection: "Provider status is primary; missing status is unknown."
    observation_source: "Official documentation and proposed Claudine launch correlation."
    observed_at: "Not observed on Linux; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same Linux user and namespace", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-native-interactive
    profile_id: ordinary-interactive
    os: windows
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code registration files and validate the advertised per-session named pipe."
    identity_check: "Use sessionId plus pid/procStart and validate the named-pipe owner/security descriptor."
    liveness_check: "Require matching process and connectable pipe; connection alone does not establish authenticated acceptance."
    state_detection: "Use status timestamps when present; otherwise unknown."
    observation_source: "Official documentation; no native Windows host observation."
    observed_at: "Not observed on native Windows; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-native-one-shot
    profile_id: ordinary-one-shot
    os: windows
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code registration files and validate the advertised per-session named pipe."
    identity_check: "Use sessionId plus pid/procStart and validate the named-pipe owner/security descriptor."
    liveness_check: "Require matching process and connectable pipe; connection alone does not establish authenticated acceptance."
    state_detection: "Use status timestamps when present; otherwise unknown."
    observation_source: "Official documentation; no native Windows host observation."
    observed_at: "Not observed on native Windows; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-native-retained
    profile_id: retained-noninteractive
    os: windows
    origin: native
    method: provider_registry
    locator: "Enumerate the same-user Claude Code registration files and validate the advertised per-session named pipe."
    identity_check: "Use sessionId plus pid/procStart and validate the named-pipe owner/security descriptor."
    liveness_check: "Require matching process and connectable pipe; connection alone does not establish authenticated acceptance."
    state_detection: "Use status timestamps when present; otherwise unknown."
    observation_source: "Official documentation; no native Windows host observation."
    observed_at: "Not observed on native Windows; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-claudine-interactive
    profile_id: ordinary-interactive
    os: windows
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart and pipe ownership; deduplicate by sessionId."
    liveness_check: "Require provider child and authenticated pipe availability."
    state_detection: "Provider status is primary; missing status is unknown."
    observation_source: "Official documentation and proposed Claudine launch correlation."
    observed_at: "Not observed on native Windows; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-claudine-one-shot
    profile_id: ordinary-one-shot
    os: windows
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart and pipe ownership; deduplicate by sessionId."
    liveness_check: "Require provider child and authenticated pipe availability."
    state_detection: "Provider status is primary; missing status is unknown."
    observation_source: "Official documentation and proposed Claudine launch correlation."
    observed_at: "Not observed on native Windows; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
  - id: registry-windows-claudine-retained
    profile_id: retained-noninteractive
    os: windows
    origin: claudine
    method: provider_registry
    locator: "Use the provider registry and correlate its child PID/sessionId with Claudine launch metadata."
    identity_check: "Use provider sessionId and validate pid/procStart and pipe ownership; deduplicate by sessionId."
    liveness_check: "Require provider child and authenticated pipe availability."
    state_detection: "Provider status is primary; missing status is unknown."
    observation_source: "Official documentation and proposed Claudine launch correlation."
    observed_at: "Not observed on native Windows; documentation reviewed 2026-09-08"
    available_labels: [name, cwd, sessionId, pid, kind, entrypoint, version, status]
    prerequisites: ["same native Windows user", "Claudine retained launch correlation", "messaging-enabled non-bare session"]
    evidence_ids: [official-cross-session]
mechanisms:
  - id: peer-unix-active
    interface_status: undocumented
    maturity: unknown
    transport: unix_socket
    initialization: "The ordinary non-bare Claude process registers the session and binds its provider-managed inbox at startup."
    operation_intent: steer_active_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: next_tool_boundary
    destination: "The target session's advertised per-session Unix socket on macOS/Linux/WSL 2."
    authentication: "An auth line with the target's per-session token is optional on macOS/Linux. Same-user endpoint permissions and inbound policy still apply. Each target exports its own token to its hooks/commands; it is never inherited from a parent."
    startup_requirements: ["Claude Code >=2.1.224, or >=2.1.248 for documented provider/feature-flag exceptions", "not --bare", "acceptable socket directory", "crossSessionInbound does not refuse"]
    target_preconditions: ["same OS user", "live matching provider process", "same filesystem/socket namespace", "eligible inbox-bearing session"]
    target_guards: ["sessionId is conversation identity", "pid plus procStart rejects PID reuse", "socket must be expected type, owner, and non-symlink", "stale or absent endpoint is excluded"]
    request_framing: "One complete newline-delimited JSON record per frame; optional auth frame first, then one user frame."
    response_framing: "Versioned newline-delimited status frames are source-derived; the complete independent-client contract is not documented."
    request_format: "Newline-delimited JSON. The documented optional first line is {type: auth, token: <redacted>}; packaged 2.1.263 source shows a following {type: user, message: {role: user, content: <text>}} line."
    response_format: "Packaged source contains message IDs and peer-message status frames. Official docs distinguish held, refused, and delivered sender notices but do not promise a stable raw frame schema."
    long_tool_behavior: "The message queues while a tool runs and is read between tool calls; the tool is never interrupted."
    tool_batch_behavior: continue_all
    ordering: "Accepted messages queue for the recipient; exact concurrent-sender ordering is undocumented."
    queue_behavior: "Working sessions drain at a later tool boundary; idle sessions start a turn. Accepted queue capacity is 50; held storage is a separate 100-message bound. Queue persistence across process exit is not established."
    message_interpretation: literal
    interruption_phases: []
    interruption_partial_failure: "Not applicable; this mechanism does not interrupt."
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: "Identical repeats in a short window are dropped; repeated-sender traffic is rate-limited."
    cancellation: "No documented sender cancellation after acceptance. Held messages may later be delivered, denied, refused by policy, or expire."
    limits: "Plain text; about one million serialized characters same-machine; burst refusal; at most 50 accepted messages queued and 100 policy-held messages."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - id: peer-unix-idle
    interface_status: undocumented
    maturity: unknown
    transport: unix_socket
    initialization: "The ordinary non-bare Claude process registers the session and binds its provider-managed inbox at startup."
    operation_intent: start_idle_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: idle_turn_start
    destination: "The target session's advertised per-session Unix socket on macOS/Linux/WSL 2."
    authentication: "An auth line with the target's per-session token is optional on macOS/Linux. Same-user endpoint permissions and inbound policy still apply. Each target exports its own token to its hooks/commands; it is never inherited from a parent."
    startup_requirements: ["Claude Code >=2.1.224, or >=2.1.248 for documented provider/feature-flag exceptions", "not --bare", "acceptable socket directory", "crossSessionInbound does not refuse"]
    target_preconditions: ["same OS user", "live matching provider process", "same filesystem/socket namespace", "eligible inbox-bearing session"]
    target_guards: ["sessionId is conversation identity", "pid plus procStart rejects PID reuse", "socket must be expected type, owner, and non-symlink", "stale or absent endpoint is excluded"]
    request_framing: "One complete newline-delimited JSON record per frame; optional auth frame first, then one user frame."
    response_framing: "Versioned newline-delimited status frames are source-derived; the complete independent-client contract is not documented."
    request_format: "Newline-delimited JSON. The documented optional first line is {type: auth, token: <redacted>}; packaged 2.1.263 source shows a following {type: user, message: {role: user, content: <text>}} line."
    response_format: "Packaged source contains message IDs and peer-message status frames. Official docs distinguish held, refused, and delivered sender notices but do not promise a stable raw frame schema."
    long_tool_behavior: "Not applicable to an idle target; a working/idle race remains possible."
    tool_batch_behavior: not_applicable
    ordering: "Accepted messages queue for the recipient; exact concurrent-sender ordering is undocumented."
    queue_behavior: "Working sessions drain at a later tool boundary; idle sessions start a turn. Accepted queue capacity is 50; held storage is a separate 100-message bound. Queue persistence across process exit is not established."
    message_interpretation: literal
    interruption_phases: []
    interruption_partial_failure: "Not applicable; this mechanism does not interrupt."
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: "Identical repeats in a short window are dropped; repeated-sender traffic is rate-limited."
    cancellation: "No documented sender cancellation after acceptance. Held messages may later be delivered, denied, refused by policy, or expire."
    limits: "Plain text; about one million serialized characters same-machine; burst refusal; at most 50 accepted messages queued and 100 policy-held messages."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - id: peer-windows-pipe-active
    interface_status: undocumented
    maturity: unknown
    transport: named_pipe
    initialization: "The ordinary non-bare native Windows Claude process registers the session and binds its provider-managed named pipe at startup."
    operation_intent: steer_active_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: next_tool_boundary
    destination: "The target session's advertised per-session native Windows named pipe."
    authentication: "The first NDJSON line must authenticate with the target session's token. Claude Code exports that per-session token only inside the target session and never inherits one from a parent, so an independent Claudine process has no documented credential source."
    startup_requirements: ["Claude Code >=2.1.234, or >=2.1.248 for documented provider/feature-flag exceptions", "not --bare", "valid target token", "crossSessionInbound does not refuse"]
    target_preconditions: ["same native Windows user", "live matching provider process", "eligible inbox-bearing session", "valid per-session target token"]
    target_guards: ["sessionId is conversation identity", "pid plus procStart rejects PID reuse", "named-pipe security descriptor and process must match", "missing or invalid auth closes the connection without delivery"]
    request_framing: "Auth-first newline-delimited JSON followed by one user frame."
    response_framing: "Versioned status frames are source-derived; the independent-client pipe response contract is not documented as stable."
    request_format: "Auth-first newline-delimited JSON, followed by a user-message frame; exact user frame is source-derived for 2.1.263 rather than fully specified as a stable public wire contract."
    response_format: "Sender-visible held/refused/delivered results are documented through Claude Code; raw pipe response framing is not a stable documented API."
    long_tool_behavior: "Queues until the next tool boundary without interrupting the tool."
    tool_batch_behavior: continue_all
    ordering: "Exact concurrent-sender ordering is undocumented."
    queue_behavior: "Same accepted and held queue behavior as peer Unix delivery; persistence across process exit is not established."
    message_interpretation: literal
    interruption_phases: []
    interruption_partial_failure: "Not applicable; this mechanism does not interrupt."
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: "Same documented repeat and rate-limit behavior as local peer messaging."
    cancellation: "No documented cancellation after acceptance."
    limits: "Plain text, same-machine size/burst/queue limits; direct external use additionally blocked by token access."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
  - id: peer-windows-pipe-idle
    interface_status: undocumented
    maturity: unknown
    transport: named_pipe
    initialization: "The ordinary non-bare native Windows Claude process registers the session and binds its provider-managed named pipe at startup."
    operation_intent: start_idle_turn
    conversation_effect: preserve_running_turn
    delivery_boundary: idle_turn_start
    destination: "The target session's advertised per-session native Windows named pipe."
    authentication: "The first NDJSON line must authenticate with the target session's token. Claude Code exports that per-session token only inside the target session and never inherits one from a parent, so an independent Claudine process has no documented credential source."
    startup_requirements: ["Claude Code >=2.1.234, or >=2.1.248 for documented provider/feature-flag exceptions", "not --bare", "valid target token", "crossSessionInbound does not refuse"]
    target_preconditions: ["same native Windows user", "live matching provider process", "eligible inbox-bearing session", "valid per-session target token"]
    target_guards: ["sessionId is conversation identity", "pid plus procStart rejects PID reuse", "named-pipe security descriptor and process must match", "missing or invalid auth closes the connection without delivery"]
    request_framing: "Auth-first newline-delimited JSON followed by one user frame."
    response_framing: "Versioned status frames are source-derived; the independent-client pipe response contract is not documented as stable."
    request_format: "Auth-first newline-delimited JSON, followed by a user-message frame; exact user frame is source-derived for 2.1.263 rather than fully specified as a stable public wire contract."
    response_format: "Sender-visible held/refused/delivered results are documented through Claude Code; raw pipe response framing is not a stable documented API."
    long_tool_behavior: "Not applicable to an idle target; a working/idle race remains possible."
    tool_batch_behavior: not_applicable
    ordering: "Exact concurrent-sender ordering is undocumented."
    queue_behavior: "Same accepted and held queue behavior as peer Unix delivery; persistence across process exit is not established."
    message_interpretation: literal
    interruption_phases: []
    interruption_partial_failure: "Not applicable; this mechanism does not interrupt."
    sender_message_id: supported
    retry_policy: never_retry
    duplicate_handling: "Same documented repeat and rate-limit behavior as local peer messaging."
    cancellation: "No documented cancellation after acceptance."
    limits: "Plain text, same-machine size/burst/queue limits; direct external use additionally blocked by token access."
    evidence_ids: [official-cross-session, local-binary-2-1-263]
compatibility:
  - mechanism_id: peer-unix-active
    profile_id: ordinary-interactive
    os: macos
    versions_verified: ["2.1.263 passive structure only"]
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for same-machine messaging on documented provider/feature-flag exceptions"
    read_only_check: "Check claude --version, registration version/peerProtocol, process identity, socket ownership/type, --bare absence when launch metadata is known, and applicable settings."
    success_criteria: "Version eligible, live identity match, same-user non-symlink socket, and inbound policy not refuse; live delivery remains separately required."
    failure_behavior: "Exclude the session or show an explicit compatibility/policy reason."
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - mechanism_id: peer-unix-idle
    profile_id: ordinary-interactive
    os: macos
    versions_verified: ["2.1.263 passive structure only"]
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for same-machine messaging on documented provider/feature-flag exceptions"
    read_only_check: "Check claude --version, registration version/peerProtocol, process identity, socket ownership/type, --bare absence when launch metadata is known, and applicable settings."
    success_criteria: "Version eligible, live identity match, same-user non-symlink socket, and inbound policy not refuse; live delivery remains separately required."
    failure_behavior: "Exclude the session or show an explicit compatibility/policy reason."
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - mechanism_id: peer-unix-active
    profile_id: retained-noninteractive
    os: macos
    versions_verified: ["2.1.263 passive structure only"]
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for same-machine messaging on documented provider/feature-flag exceptions"
    read_only_check: "Check claude --version, registration version/peerProtocol, process identity, socket ownership/type, --bare absence when launch metadata is known, and applicable settings."
    success_criteria: "Version eligible, live identity match, same-user non-symlink socket, and inbound policy not refuse; live delivery remains separately required."
    failure_behavior: "Exclude the session or show an explicit compatibility/policy reason."
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - mechanism_id: peer-unix-idle
    profile_id: retained-noninteractive
    os: macos
    versions_verified: ["2.1.263 passive structure only"]
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for same-machine messaging on documented provider/feature-flag exceptions"
    read_only_check: "Check claude --version, registration version/peerProtocol, process identity, socket ownership/type, --bare absence when launch metadata is known, and applicable settings."
    success_criteria: "Version eligible, live identity match, same-user non-symlink socket, and inbound policy not refuse; live delivery remains separately required."
    failure_behavior: "Exclude the session or show an explicit compatibility/policy reason."
    evidence_ids: [official-cross-session, local-registry-2-1-263]
  - mechanism_id: peer-unix-active
    profile_id: ordinary-interactive
    os: linux
    versions_verified: []
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for documented exceptions; WSL 2 is Linux-side"
    read_only_check: "Check version, registry/process start identity, socket type/owner, namespace visibility, and settings."
    success_criteria: "Eligible version and a live, same-user, validated endpoint; live delivery remains required."
    failure_behavior: "Exclude or report unavailable."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-unix-idle
    profile_id: ordinary-interactive
    os: linux
    versions_verified: []
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for documented exceptions; WSL 2 is Linux-side"
    read_only_check: "Check version, registry/process start identity, socket type/owner, namespace visibility, and settings."
    success_criteria: "Eligible version and a live, same-user, validated endpoint; live delivery remains required."
    failure_behavior: "Exclude or report unavailable."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-unix-active
    profile_id: retained-noninteractive
    os: linux
    versions_verified: []
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for documented exceptions; WSL 2 is Linux-side"
    read_only_check: "Check version, registry/process start identity, socket type/owner, namespace visibility, and settings."
    success_criteria: "Eligible version and a live, same-user, validated endpoint; live delivery remains required."
    failure_behavior: "Exclude or report unavailable."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-unix-idle
    profile_id: retained-noninteractive
    os: linux
    versions_verified: []
    documented_version_bounds: ">=2.1.224 generally; >=2.1.248 for documented exceptions; WSL 2 is Linux-side"
    read_only_check: "Check version, registry/process start identity, socket type/owner, namespace visibility, and settings."
    success_criteria: "Eligible version and a live, same-user, validated endpoint; live delivery remains required."
    failure_behavior: "Exclude or report unavailable."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-windows-pipe-active
    profile_id: ordinary-interactive
    os: windows
    versions_verified: []
    documented_version_bounds: ">=2.1.234 generally; >=2.1.248 for documented exceptions"
    read_only_check: "Check version, registry/process identity, named-pipe security, settings, and whether a supported target-token source exists."
    success_criteria: "Eligible live session plus target token and valid pipe; no target-token source is currently established for independent Claudine."
    failure_behavior: "Do not offer direct delivery; the provider's own ListAgents/SendMessage remains an agent tool, not an external authentication bridge."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-windows-pipe-idle
    profile_id: ordinary-interactive
    os: windows
    versions_verified: []
    documented_version_bounds: ">=2.1.234 generally; >=2.1.248 for documented exceptions"
    read_only_check: "Check version, registry/process identity, named-pipe security, settings, and whether a supported target-token source exists."
    success_criteria: "Eligible live session plus target token and valid pipe; no target-token source is currently established for independent Claudine."
    failure_behavior: "Do not offer direct delivery; the provider's own ListAgents/SendMessage remains an agent tool, not an external authentication bridge."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-windows-pipe-active
    profile_id: retained-noninteractive
    os: windows
    versions_verified: []
    documented_version_bounds: ">=2.1.234 generally; >=2.1.248 for documented exceptions"
    read_only_check: "Check version, registry/process identity, named-pipe security, settings, and whether a supported target-token source exists."
    success_criteria: "Eligible live session plus target token and valid pipe; no target-token source is currently established for independent Claudine."
    failure_behavior: "Do not offer direct delivery; the provider's own ListAgents/SendMessage remains an agent tool, not an external authentication bridge."
    evidence_ids: [official-cross-session]
  - mechanism_id: peer-windows-pipe-idle
    profile_id: retained-noninteractive
    os: windows
    versions_verified: []
    documented_version_bounds: ">=2.1.234 generally; >=2.1.248 for documented exceptions"
    read_only_check: "Check version, registry/process identity, named-pipe security, settings, and whether a supported target-token source exists."
    success_criteria: "Eligible live session plus target token and valid pipe; no target-token source is currently established for independent Claudine."
    failure_behavior: "Do not offer direct delivery; the provider's own ListAgents/SendMessage remains an agent tool, not an external authentication bridge."
    evidence_ids: [official-cross-session]
verification: []
cases:
  - {profile_id: ordinary-interactive, os: macos, launch_mode: interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-native-interactive], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Documented delivery waits for the next tool boundary and does not interrupt the running tool."}
  - {profile_id: retained-noninteractive, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-native-retained], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Documented delivery waits for the next tool boundary and does not interrupt the running tool."}
  - {profile_id: ordinary-one-shot, os: macos, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [registry-macos-native-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "The evidence establishes inbox behavior for retained long-running non-interactive workers, not arbitrary ordinary one-shot delivery while working."}
  - {profile_id: ordinary-interactive, os: macos, launch_mode: interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-native-interactive], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Documented idle delivery starts a new turn."}
  - {profile_id: retained-noninteractive, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-native-retained], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Documented idle delivery starts a new turn."}
  - {profile_id: ordinary-one-shot, os: macos, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [registry-macos-native-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "A completed one-shot process is history, not an active idle session; no retained endpoint is assumed."}
  - {profile_id: ordinary-interactive, os: macos, launch_mode: interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-claudine-interactive], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Wrapper origin does not change the provider inbox semantics."}
  - {profile_id: retained-noninteractive, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-macos-claudine-retained], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "Wrapper origin does not change the provider inbox semantics."}
  - {profile_id: ordinary-one-shot, os: macos, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [registry-macos-claudine-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session, local-registry-2-1-263], reason: "The evidence establishes inbox behavior for retained long-running non-interactive workers, not arbitrary ordinary one-shot delivery while working."}
  - {profile_id: ordinary-interactive, os: macos, launch_mode: interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-claudine-interactive], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "An eligible idle provider child starts a turn on delivery."}
  - {profile_id: retained-noninteractive, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-macos-claudine-retained], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "An eligible idle provider child starts a turn on delivery."}
  - {profile_id: ordinary-one-shot, os: macos, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [registry-macos-claudine-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "A completed one-shot process is history, not an active idle session; no retained endpoint is assumed."}
  - {profile_id: ordinary-interactive, os: linux, launch_mode: interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-native-interactive], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "same namespace", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented Linux behavior queues until a tool boundary."}
  - {profile_id: retained-noninteractive, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-native-retained], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "same namespace", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented Linux behavior queues until a tool boundary."}
  - {profile_id: ordinary-one-shot, os: linux, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [registry-linux-native-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "The evidence establishes inbox behavior for retained long-running non-interactive workers, not arbitrary ordinary one-shot delivery while working."}
  - {profile_id: ordinary-interactive, os: linux, launch_mode: interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-native-interactive], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "same namespace", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented idle delivery starts a turn."}
  - {profile_id: retained-noninteractive, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-native-retained], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "same namespace", "non-bare inbox", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented idle delivery starts a turn."}
  - {profile_id: ordinary-one-shot, os: linux, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [registry-linux-native-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "A completed one-shot process is history, not an active idle session; no retained endpoint is assumed."}
  - {profile_id: ordinary-interactive, os: linux, launch_mode: interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-claudine-interactive], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "same namespace", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Claudine launch does not alter documented provider delivery."}
  - {profile_id: retained-noninteractive, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-linux-claudine-retained], mechanism_ids: [peer-unix-active], prerequisites: ["eligible version", "same namespace", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Claudine launch does not alter documented provider delivery."}
  - {profile_id: ordinary-one-shot, os: linux, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [registry-linux-claudine-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "The evidence establishes inbox behavior for retained long-running non-interactive workers, not arbitrary ordinary one-shot delivery while working."}
  - {profile_id: ordinary-interactive, os: linux, launch_mode: interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-claudine-interactive], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "same namespace", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Eligible idle child starts a turn."}
  - {profile_id: retained-noninteractive, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-linux-claudine-retained], mechanism_ids: [peer-unix-idle], prerequisites: ["eligible version", "same namespace", "launch correlation", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Eligible idle child starts a turn."}
  - {profile_id: ordinary-one-shot, os: linux, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [registry-linux-claudine-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "A completed one-shot process is history, not an active idle session; no retained endpoint is assumed."}
  - {profile_id: ordinary-interactive, os: windows, launch_mode: interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-native-interactive], mechanism_ids: [peer-windows-pipe-active], prerequisites: ["eligible version", "non-bare inbox", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Provider capability is documented, but direct Claudine use lacks a target-token source."}
  - {profile_id: retained-noninteractive, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-native-retained], mechanism_ids: [peer-windows-pipe-active], prerequisites: ["eligible version", "non-bare inbox", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Provider capability is documented, but direct Claudine use lacks a target-token source."}
  - {profile_id: ordinary-one-shot, os: windows, launch_mode: non_interactive, origin: native, session_state: working, support: unknown, discovery_ids: [registry-windows-native-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "The evidence establishes inbox behavior for retained long-running non-interactive workers, not arbitrary ordinary one-shot delivery while working."}
  - {profile_id: ordinary-interactive, os: windows, launch_mode: interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-native-interactive], mechanism_ids: [peer-windows-pipe-idle], prerequisites: ["eligible version", "non-bare inbox", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented idle peer delivery starts a turn; Claudine authentication remains unresolved."}
  - {profile_id: retained-noninteractive, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-native-retained], mechanism_ids: [peer-windows-pipe-idle], prerequisites: ["eligible version", "non-bare inbox", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Documented idle peer delivery starts a turn; Claudine authentication remains unresolved."}
  - {profile_id: ordinary-one-shot, os: windows, launch_mode: non_interactive, origin: native, session_state: idle, support: unknown, discovery_ids: [registry-windows-native-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "A completed one-shot process is history, not an active idle session; no retained endpoint is assumed."}
  - {profile_id: ordinary-interactive, os: windows, launch_mode: interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-claudine-interactive], mechanism_ids: [peer-windows-pipe-active], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Parent launch does not supply the target's generated token back to Claudine."}
  - {profile_id: retained-noninteractive, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: non_interrupting, discovery_ids: [registry-windows-claudine-retained], mechanism_ids: [peer-windows-pipe-active], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Parent launch does not supply the target's generated token back to Claudine."}
  - {profile_id: ordinary-one-shot, os: windows, launch_mode: non_interactive, origin: claudine, session_state: working, support: unknown, discovery_ids: [registry-windows-claudine-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "The evidence establishes inbox behavior for retained long-running non-interactive workers, not arbitrary ordinary one-shot delivery while working."}
  - {profile_id: ordinary-interactive, os: windows, launch_mode: interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-claudine-interactive], mechanism_ids: [peer-windows-pipe-idle], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Provider support exists, while a Claudine adapter is blocked on authentication."}
  - {profile_id: retained-noninteractive, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: non_interrupting, discovery_ids: [registry-windows-claudine-retained], mechanism_ids: [peer-windows-pipe-idle], prerequisites: ["eligible version", "non-bare inbox", "launch correlation", "valid target token", "inbound policy permits"], evidence_ids: [official-cross-session], reason: "Provider support exists, while a Claudine adapter is blocked on authentication."}
  - {profile_id: ordinary-one-shot, os: windows, launch_mode: non_interactive, origin: claudine, session_state: idle, support: unknown, discovery_ids: [registry-windows-claudine-one-shot], mechanism_ids: [], prerequisites: [ordinary one-shot process], evidence_ids: [official-cross-session], reason: "A completed one-shot process is history, not an active idle session; no retained endpoint is assumed."}
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
  - "Refreshed the revision 1 pilot into schema revision 2 with an explicit ordinary-peer launch profile, profile-linked discovery and compatibility, full protocol intent/guard/queue fields, and separate initial-receipt guarantees."
  - "Revalidated Claude Code 2.1.263 through Sniff and local help, rechecked official cross-session, Remote Control, Channels, CLI, and overview documentation, and recorded that no session registry or socket was live during the refresh."
  - Split ordinary interactive, ordinary one-shot non-interactive, and retained non-interactive launch profiles; split active-turn steering from idle-turn start mechanisms.
  - Split ordinary interactive, ordinary one-shot non-interactive, and retained non-interactive launch profiles; split active-turn steering from idle-turn start mechanisms.
requires_claudine_update: true
reason: "Claudine needs registry parsing, identity/liveness checks, policy-aware acknowledgments, OS-specific adapters, compatibility gates, and mandatory disposable verification. Native Windows direct delivery additionally lacks a supported token source."
discovery_gaps: []
interface_inventory:
- disposition: included
  evidence_ids:
  - official-cross-session
  - official-cli
  - local-help-2-1-263
  id: profile-ordinary-interactive
  profile_ids:
  - ordinary-interactive
  reason: Ordinary interactive Claude Code session using the provider-managed same-machine peer inbox.
- disposition: included
  evidence_ids:
  - official-cross-session
  - official-cli
  - local-help-2-1-263
  id: profile-ordinary-one-shot
  profile_ids:
  - ordinary-one-shot
  reason: Ordinary one-shot non-interactive Claude Code process. It is active only while running and cannot be an idle active session after exit.
- disposition: included
  evidence_ids:
  - official-cross-session
  - official-cli
  - local-help-2-1-263
  id: profile-retained-noninteractive
  profile_ids:
  - retained-noninteractive
  reason: Deliberately retained long-running non-interactive Claude Code process with a provider-managed peer inbox.
- disposition: unknown
  evidence_ids:
  - official-cross-session
  - official-cli
  - local-help-2-1-263
  id: coverage-review-retained-noninteractive
  profile_ids:
  - retained-noninteractive
  reason: The existing profile does not cover other client launch modes. This migration does not establish that these combinations are impossible. Review interface ownership, lifetime, and discovery before expanding coverage; do not infer exclusion from current wrapper behavior.
receipt_observations:
- evidence_ids:
  - official-cross-session
  - local-binary-2-1-263
  mechanism_id: peer-unix-active
  signal: No independently established initial acknowledgment timing. A successful socket write or initial raw response must not be treated as acceptance, persistence, scheduling, or delivery until a disposable test establishes the exact frame and correlation behavior.
  timing: unknown
- evidence_ids:
  - official-cross-session
  - local-binary-2-1-263
  mechanism_id: peer-unix-idle
  signal: No independently established initial acknowledgment timing. A successful socket write or initial raw response must not be treated as acceptance, persistence, scheduling, or delivery until a disposable test establishes the exact frame and correlation behavior.
  timing: unknown
- evidence_ids:
  - official-cross-session
  - local-binary-2-1-263
  mechanism_id: peer-windows-pipe-active
  signal: No independently established initial acknowledgment timing. No supported independent source for the target token and no live pipe exchange were established.
  timing: unknown
- evidence_ids:
  - official-cross-session
  - local-binary-2-1-263
  mechanism_id: peer-windows-pipe-idle
  signal: No independently established initial acknowledgment timing. No supported independent source for the target token and no live pipe exchange were established.
  timing: unknown

---

# Claude Code steering research

Claude Code documents provider-managed peer inboxes for eligible, non-bare
sessions. This revision separates launch and operation shapes that affect
steering eligibility. Ordinary interactive sessions are baseline active-session
targets. Ordinary one-shot non-interactive launches are also baseline, but the
evidence does not establish arbitrary mid-run access, and a process that exits is
history rather than an idle active session. Long-running non-interactive workers
use the separate retained profile because continued process lifetime is required.

Unix-socket and native-Windows named-pipe transports each have separate active
and idle operations. Active steering preserves the current turn and waits for a
tool boundary. Idle delivery starts a new turn in the same conversation. Neither
operation interrupts a running tool. Incoming-policy holds are delivery states;
later tool permissions remain execution state.

The provider feature is documented, while independent raw-client framing remains
source-derived and is marked undocumented with unknown maturity. Windows also
lacks a supported source for the target session token. Every candidate remains
unavailable for activation because this passive run has no disposable delivery
verification.


## Revision 3 Contract Backfill

Receipt timing, interface inventory, and case-specific discovery gaps were added
from the existing evidence on 2026-09-08. No new provider observation or live test
was performed. Unexamined profile combinations remain unknown, not unsupported.
The original fleet model/effort provenance above describes the research run;
this deterministic contract migration is a separate coordinator edit.
