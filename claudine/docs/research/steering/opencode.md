---
"$schema": "./_schema.yaml"
schema_revision: 2
provider: opencode
created: '2026-09-08'
last_updated: '2026-09-08'
agent: codex
model: gpt-5.6-sol
reasoning_effort: low
versions_examined:
- OpenCode CLI 1.18.29 (passive local macOS inspection and tag v1.18.29)
- OpenCode development commit d6855b6b47a8433462ac6aeeba882ccf734cb7f1 (source comparison
  only)
access_findings:
- mechanism_id: http-prompt-async-active
  profile_id: exposed-tui-http
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: &1
  - official-server
  - source-api-1-18-29
  - source-runner-1-18-29
- mechanism_id: http-prompt-sync-active
  profile_id: exposed-tui-http
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: &2
  - official-server
  - source-api-1-18-29
  - source-runner-1-18-29
- mechanism_id: http-prompt-async-idle
  profile_id: exposed-tui-http
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: &3
  - official-server
  - source-api-1-18-29
  - source-runner-1-18-29
- mechanism_id: http-prompt-sync-idle
  profile_id: exposed-tui-http
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: &4
  - official-server
  - source-api-1-18-29
- mechanism_id: http-abort-then-prompt
  profile_id: exposed-tui-http
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch; interruption requires explicit interactive approval.
  applies_to_existing_sessions: 'no'
  evidence_ids: &5
  - official-server
  - source-api-1-18-29
  - source-runner-1-18-29
- mechanism_id: http-prompt-async-active
  profile_id: exposed-tui-http
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch; interruption requires explicit interactive approval.
  applies_to_existing_sessions: 'no'
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: exposed-tui-http
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch; interruption requires explicit interactive approval.
  applies_to_existing_sessions: 'no'
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: exposed-tui-http
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch; interruption requires explicit interactive approval.
  applies_to_existing_sessions: 'no'
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: exposed-tui-http
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch; interruption requires explicit interactive approval.
  applies_to_existing_sessions: 'no'
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: exposed-tui-http
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch; interruption requires explicit interactive approval.
  applies_to_existing_sessions: 'no'
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: exposed-tui-http
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: exposed-tui-http
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: exposed-tui-http
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: exposed-tui-http
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: exposed-tui-http
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: retained-server-run-attach
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: retained-server-run-attach
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: retained-server-run-attach
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: retained-server-run-attach
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: retained-server-run-attach
  os: macos
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: retained-server-run-attach
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: retained-server-run-attach
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: retained-server-run-attach
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: retained-server-run-attach
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: retained-server-run-attach
  os: linux
  status: setup_required
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: retained-server-run-attach
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: retained-server-run-attach
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: retained-server-run-attach
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: retained-server-run-attach
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: retained-server-run-attach
  os: windows
  status: unknown
  prerequisite: Deliberately expose and retain the authenticated endpoint, workspace
    routing, and native provider session identity before launch.
  applies_to_existing_sessions: 'no'
  evidence_ids: *5
delivery_states:
- mechanism_id: http-prompt-async-active
  states:
  - accepted
  - queued
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: Caller-selected messageID can correlate message reads/SSE, but 204
    or an open stream does not independently expose persistence, scheduling, or terminal
    delivery. Held permission/question state is post-delivery execution state, not
    an incoming queue acknowledgment.
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  states:
  - accepted
  - queued
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: Caller-selected messageID can correlate message reads/SSE, but 204
    or an open stream does not independently expose persistence, scheduling, or terminal
    delivery. Held permission/question state is post-delivery execution state, not
    an incoming queue acknowledgment.
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  states:
  - accepted
  - queued
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: Caller-selected messageID can correlate message reads/SSE, but 204
    or an open stream does not independently expose persistence, scheduling, or terminal
    delivery. Held permission/question state is post-delivery execution state, not
    an incoming queue acknowledgment.
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  states:
  - accepted
  - queued
  - delivered
  - refused
  - unknown
  observable_by_external_sender: partial
  correlation: Caller-selected messageID can correlate message reads/SSE, but 204
    or an open stream does not independently expose persistence, scheduling, or terminal
    delivery. Held permission/question state is post-delivery execution state, not
    an incoming queue acknowledgment.
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  states:
  - accepted
  - refused
  - delivered
  - unknown
  observable_by_external_sender: partial
  correlation: Abort boolean and later status/SSE are separate from the replacement
    messageID; there is no atomic cross-phase correlation.
  evidence_ids: *5
evidence:
- id: official-server
  method: official_docs
  location: https://opencode.ai/docs/server/
  version: current documentation observed 2026-09-08; explicit historical bounds not
    stated
  observed_on: '2026-09-08'
  claim: OpenCode documents its HTTP/OpenAPI server, session list/status/message/prompt_async/abort
    endpoints, SSE events, TUI-plus-server architecture, explicit endpoint launch,
    and optional HTTP Basic authentication.
  limitations: Documentation does not define busy prompt queuing, concurrency ordering,
    external endpoint discovery, or OS-specific runtime guarantees.
- id: official-cli
  method: official_docs
  location: https://opencode.ai/docs/cli/
  version: current documentation observed 2026-09-08
  observed_on: '2026-09-08'
  claim: The CLI distinguishes ordinary TUI, attach, non-interactive run, run --attach,
    and serve; run accepts a server URL and Basic credentials and otherwise uses a
    local server.
  limitations: The broad interactive/non-interactive labels do not identify endpoint
    visibility or server ownership.
- id: local-help-1-18-29
  method: local_inspection
  location: sanitized output of opencode --version and help for the root, serve, run,
    and attach commands; Sniff software agent inventory
  version: 1.18.29 on macOS
  observed_on: '2026-09-08'
  claim: The installed binary exposes TUI, serve, attach, run --attach, --hostname,
    --port, --password, --username, session continuation, and random-port defaults.
  limitations: Passive inspection only; no server was launched and no message was
    sent.
- id: source-launch-1-18-29
  method: source_code
  location: https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/cli/cmd/tui.ts
    and packages/opencode/src/cli/cmd/run.ts
  version: v1.18.29, commit 16747470f976aca3d362ad730bcd3fe82ecc2c9a
  observed_on: '2026-09-08'
  claim: The ordinary TUI selects internal worker RPC/event transport unless network
    flags are explicit; run without --attach owns an in-process server while attached
    run uses the supplied server.
  limitations: Source-derived launch behavior for one version; no native Windows/Linux
    launch observation.
- id: source-api-1-18-29
  method: source_code
  location: https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/server/routes/instance/httpapi/handlers/session.ts
    and packages/sdk/openapi.json
  version: v1.18.29, commit 16747470f976aca3d362ad730bcd3fe82ecc2c9a
  observed_on: '2026-09-08'
  claim: The API lists sessions/status, accepts message IDs and typed parts, returns
    204 for prompt_async, streams a response for prompt, and exposes abort as a separate
    operation.
  limitations: The response schemas do not prove semantic delivery during a busy turn.
- id: source-runner-1-18-29
  method: source_code
  location: https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/session/prompt.ts
    and packages/opencode/src/effect/runner.ts
  version: v1.18.29, commit 16747470f976aca3d362ad730bcd3fe82ecc2c9a
  observed_on: '2026-09-08'
  claim: Prompt handling persists the user message before ensureRunning; when already
    Running, ensureRunning awaits the existing run rather than scheduling a second
    run. The loop rereads messages between model/tool iterations.
  limitations: This proves neither reliable next-tool absorption nor a post-run queue.
    Generation-only stranding is an inference from control flow, not an observed outcome,
    so busy delivery remains unknown pending a disposable test.
- id: source-auth-1-18-29
  method: source_code
  location: https://github.com/anomalyco/opencode/blob/16747470f976aca3d362ad730bcd3fe82ecc2c9a/packages/opencode/src/server/auth.ts
  version: v1.18.29, commit 16747470f976aca3d362ad730bcd3fe82ecc2c9a
  observed_on: '2026-09-08'
  claim: Server authentication is optional HTTP Basic using OPENCODE_SERVER_PASSWORD
    and an optional username; without a password the server is unsecured.
  limitations: Loopback reachability does not create a per-user trust boundary and
    the API does not advertise a peer-identity credential.
- id: source-head-2026-09-08
  method: source_code
  location: https://github.com/anomalyco/opencode/tree/d6855b6b47a8433462ac6aeeba882ccf734cb7f1
  version: development commit d6855b6b47a8433462ac6aeeba882ccf734cb7f1
  observed_on: '2026-09-08'
  claim: Current development source retains the HTTP prompt/status/abort surfaces
    and has substantial server/runtime restructuring beyond v1.18.29.
  limitations: An unreleased commit is not a compatibility bound and must not be mixed
    with the installed-version runtime gate.
- id: local-sniff-1-18-29
  method: local_inspection
  location: sanitized Sniff software-agent and OS inventory plus non-interactive OpenCode
    version/help output
  version: OpenCode 1.18.29; macOS 27.0 host
  observed_on: '2026-09-08'
  claim: Sniff found OpenCode 1.18.29 at /Users/ken/.opencode/bin/opencode on native
    macOS; passive help confirms TUI, serve, attach, run --attach, random-port defaults,
    explicit hostname/port, mDNS flags, and Basic-auth options.
  limitations: No OpenCode session or server was launched; process/service output
    was not used to inspect or alter existing agent sessions.
discovery:
- id: discover-ordinary-tui-macos-native
  os: macos
  origin: native
  method: process_inspection
  locator: Ordinary interactive OpenCode TUI using its internal worker transport and
    no deliberately exposed HTTP endpoint. Sniff may label candidate processes, but
    no external endpoint registry was found; this profile is not externally discoverable
    by provider API.
  identity_check: Use provider session id from GET /session as conversation identity;
    validate project directory/workspace and recent messages. PID identifies only
    a process and one server may host many sessions.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: &6
  - session_id
  - title
  - directory
  - project_id
  - parent_id
  - version
  - status
  prerequisites: &7
  - known endpoint or explicit launch correlation
  - Basic credential if configured
  - same local host/current OS user policy
  evidence_ids: &8
  - official-server
  - local-help-1-18-29
  - source-api-1-18-29
  profile_id: ordinary-tui
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-tui-macos-claudine
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Ordinary interactive OpenCode TUI using its internal worker transport and
    no deliberately exposed HTTP endpoint. Sniff may label candidate processes, but
    no external endpoint registry was found; this profile is not externally discoverable
    by provider API.
  identity_check: Correlate wrapper launch to the session returned by that exact endpoint
    and directory; deduplicate by endpoint identity plus provider session ID.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: &9
  - session_id
  - title
  - directory
  - launch_profile
  - endpoint
  - pid
  - version
  - status
  prerequisites: &10
  - Claudine retained endpoint and provider-session correlation
  evidence_ids: &11
  - official-server
  - source-launch-1-18-29
  profile_id: ordinary-tui
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-tui-linux-native
  os: linux
  origin: native
  method: process_inspection
  locator: Ordinary interactive OpenCode TUI using its internal worker transport and
    no deliberately exposed HTTP endpoint. Sniff may label candidate processes, but
    no external endpoint registry was found; this profile is not externally discoverable
    by provider API.
  identity_check: Use endpoint plus provider session ID and directory/workspace; reject
    PID-only identity.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: &12
  - session_id
  - title
  - directory
  - project_id
  - version
  - status
  prerequisites: &13
  - known endpoint
  - Basic credential if configured
  - same Linux user/network namespace
  evidence_ids: &14
  - official-server
  - source-api-1-18-29
  profile_id: ordinary-tui
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-tui-linux-claudine
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Ordinary interactive OpenCode TUI using its internal worker transport and
    no deliberately exposed HTTP endpoint. Sniff may label candidate processes, but
    no external endpoint registry was found; this profile is not externally discoverable
    by provider API.
  identity_check: Endpoint plus provider session ID is authoritative; correlate but
    do not replace it with CLAUDINE_SESSION_ID.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: &15
  - session_id
  - title
  - directory
  - launch_profile
  - endpoint
  - pid
  - version
  - status
  prerequisites: &16
  - Claudine retained endpoint and provider-session correlation
  evidence_ids: &17
  - official-server
  - source-launch-1-18-29
  profile_id: ordinary-tui
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-tui-windows-native
  os: windows
  origin: native
  method: process_inspection
  locator: Ordinary interactive OpenCode TUI using its internal worker transport and
    no deliberately exposed HTTP endpoint. Sniff may label candidate processes, but
    no external endpoint registry was found; this profile is not externally discoverable
    by provider API.
  identity_check: Use endpoint plus provider session ID and directory/workspace, never
    PID alone.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: &18
  - session_id
  - title
  - directory
  - project_id
  - version
  - status
  prerequisites: &19
  - known endpoint
  - Basic credential if configured
  - same native Windows user
  evidence_ids: &20
  - official-server
  profile_id: ordinary-tui
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-tui-windows-claudine
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Ordinary interactive OpenCode TUI using its internal worker transport and
    no deliberately exposed HTTP endpoint. Sniff may label candidate processes, but
    no external endpoint registry was found; this profile is not externally discoverable
    by provider API.
  identity_check: Endpoint plus provider session ID with Windows process-token user
    checks; CLAUDINE_SESSION_ID is only correlation metadata.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: &21
  - session_id
  - title
  - directory
  - launch_profile
  - endpoint
  - pid
  - version
  - status
  prerequisites: &22
  - Claudine retained endpoint and provider-session correlation
  - native Windows verification
  evidence_ids: &23
  - official-server
  profile_id: ordinary-tui
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-run-macos-native
  os: macos
  origin: native
  method: process_inspection
  locator: Ordinary one-shot opencode run with its in-process random-port server and
    no retained external endpoint. Sniff may label candidate processes, but no external
    endpoint registry was found; this profile is not externally discoverable by provider
    API.
  identity_check: Use provider session id from GET /session as conversation identity;
    validate project directory/workspace and recent messages. PID identifies only
    a process and one server may host many sessions.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: *6
  prerequisites: *7
  evidence_ids: *8
  profile_id: ordinary-run
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-run-macos-claudine
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Ordinary one-shot opencode run with its in-process random-port server and
    no retained external endpoint. Sniff may label candidate processes, but no external
    endpoint registry was found; this profile is not externally discoverable by provider
    API.
  identity_check: Correlate wrapper launch to the session returned by that exact endpoint
    and directory; deduplicate by endpoint identity plus provider session ID.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: *9
  prerequisites: *10
  evidence_ids: *11
  profile_id: ordinary-run
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-run-linux-native
  os: linux
  origin: native
  method: process_inspection
  locator: Ordinary one-shot opencode run with its in-process random-port server and
    no retained external endpoint. Sniff may label candidate processes, but no external
    endpoint registry was found; this profile is not externally discoverable by provider
    API.
  identity_check: Use endpoint plus provider session ID and directory/workspace; reject
    PID-only identity.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: *12
  prerequisites: *13
  evidence_ids: *14
  profile_id: ordinary-run
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-run-linux-claudine
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Ordinary one-shot opencode run with its in-process random-port server and
    no retained external endpoint. Sniff may label candidate processes, but no external
    endpoint registry was found; this profile is not externally discoverable by provider
    API.
  identity_check: Endpoint plus provider session ID is authoritative; correlate but
    do not replace it with CLAUDINE_SESSION_ID.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: *15
  prerequisites: *16
  evidence_ids: *17
  profile_id: ordinary-run
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-run-windows-native
  os: windows
  origin: native
  method: process_inspection
  locator: Ordinary one-shot opencode run with its in-process random-port server and
    no retained external endpoint. Sniff may label candidate processes, but no external
    endpoint registry was found; this profile is not externally discoverable by provider
    API.
  identity_check: Use endpoint plus provider session ID and directory/workspace, never
    PID alone.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: *18
  prerequisites: *19
  evidence_ids: *20
  profile_id: ordinary-run
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-ordinary-run-windows-claudine
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Ordinary one-shot opencode run with its in-process random-port server and
    no retained external endpoint. Sniff may label candidate processes, but no external
    endpoint registry was found; this profile is not externally discoverable by provider
    API.
  identity_check: Endpoint plus provider session ID with Windows process-token user
    checks; CLAUDINE_SESSION_ID is only correlation metadata.
  liveness_check: Sniff can check process liveness, but that does not establish conversation
    liveness or a reachable steering endpoint.
  state_detection: Unavailable to an external sender for this internal profile; process
    state cannot distinguish working from idle conversation state.
  available_labels: *21
  prerequisites: *22
  evidence_ids: *23
  profile_id: ordinary-run
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-exposed-tui-http-macos-native
  os: macos
  origin: native
  method: process_inspection
  locator: Use retained launch metadata for exposed-tui-http; verify the explicit
    endpoint before listing provider sessions. Process discovery alone is insufficient.
  identity_check: Use provider session id from GET /session as conversation identity;
    validate project directory/workspace and recent messages. PID identifies only
    a process and one server may host many sessions.
  liveness_check: GET /global/health plus a fresh GET /session/:id on the revalidated
    endpoint; TCP connect alone is insufficient.
  state_detection: GET /session/status and SSE session.status/session.idle, checked
    against message/event progress because idle entries are omitted from the in-memory
    map.
  available_labels: *6
  prerequisites: *7
  evidence_ids: *8
  profile_id: exposed-tui-http
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-exposed-tui-http-macos-claudine
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Use retained launch metadata for exposed-tui-http; verify the explicit
    endpoint before listing provider sessions. Process discovery alone is insufficient.
  identity_check: Correlate wrapper launch to the session returned by that exact endpoint
    and directory; deduplicate by endpoint identity plus provider session ID.
  liveness_check: Require live child/server ownership and successful authenticated
    health/session reads.
  state_detection: Use provider status/SSE with Claudine process state as secondary
    evidence.
  available_labels: *9
  prerequisites: *10
  evidence_ids: *11
  profile_id: exposed-tui-http
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-exposed-tui-http-linux-native
  os: linux
  origin: native
  method: process_inspection
  locator: Use retained launch metadata for exposed-tui-http; verify the explicit
    endpoint before listing provider sessions. Process discovery alone is insufficient.
  identity_check: Use endpoint plus provider session ID and directory/workspace; reject
    PID-only identity.
  liveness_check: Health and session reads on the revalidated endpoint.
  state_detection: Status/SSE plus transcript progress; absence from status map means
    idle only according to current source, but stale endpoints and races remain possible.
  available_labels: *12
  prerequisites: *13
  evidence_ids: *14
  profile_id: exposed-tui-http
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-exposed-tui-http-linux-claudine
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Use retained launch metadata for exposed-tui-http; verify the explicit
    endpoint before listing provider sessions. Process discovery alone is insufficient.
  identity_check: Endpoint plus provider session ID is authoritative; correlate but
    do not replace it with CLAUDINE_SESSION_ID.
  liveness_check: Health/session reads plus provider child/server liveness.
  state_detection: Provider status/SSE, with wrapper state only supplementary.
  available_labels: *15
  prerequisites: *16
  evidence_ids: *17
  profile_id: exposed-tui-http
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-exposed-tui-http-windows-native
  os: windows
  origin: native
  method: process_inspection
  locator: Use retained launch metadata for exposed-tui-http; verify the explicit
    endpoint before listing provider sessions. Process discovery alone is insufficient.
  identity_check: Use endpoint plus provider session ID and directory/workspace, never
    PID alone.
  liveness_check: Authenticated health and session reads; native Windows process-start
    validation remains to be designed with Sniff.
  state_detection: Status/SSE semantics are source-derived and need native Windows
    verification.
  available_labels: *18
  prerequisites: *19
  evidence_ids: *20
  profile_id: exposed-tui-http
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-exposed-tui-http-windows-claudine
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Use retained launch metadata for exposed-tui-http; verify the explicit
    endpoint before listing provider sessions. Process discovery alone is insufficient.
  identity_check: Endpoint plus provider session ID with Windows process-token user
    checks; CLAUDINE_SESSION_ID is only correlation metadata.
  liveness_check: Authenticated health/session reads and child/server liveness.
  state_detection: Provider status/SSE after native verification.
  available_labels: *21
  prerequisites: *22
  evidence_ids: *23
  profile_id: exposed-tui-http
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-retained-server-run-attach-macos-native
  os: macos
  origin: native
  method: process_inspection
  locator: Use retained launch metadata for retained-server-run-attach; verify the
    explicit endpoint before listing provider sessions. Process discovery alone is
    insufficient.
  identity_check: Use provider session id from GET /session as conversation identity;
    validate project directory/workspace and recent messages. PID identifies only
    a process and one server may host many sessions.
  liveness_check: GET /global/health plus a fresh GET /session/:id on the revalidated
    endpoint; TCP connect alone is insufficient.
  state_detection: GET /session/status and SSE session.status/session.idle, checked
    against message/event progress because idle entries are omitted from the in-memory
    map.
  available_labels: *6
  prerequisites: *7
  evidence_ids: *8
  profile_id: retained-server-run-attach
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-retained-server-run-attach-macos-claudine
  os: macos
  origin: claudine
  method: claudine_registration
  locator: Use retained launch metadata for retained-server-run-attach; verify the
    explicit endpoint before listing provider sessions. Process discovery alone is
    insufficient.
  identity_check: Correlate wrapper launch to the session returned by that exact endpoint
    and directory; deduplicate by endpoint identity plus provider session ID.
  liveness_check: Require live child/server ownership and successful authenticated
    health/session reads.
  state_detection: Use provider status/SSE with Claudine process state as secondary
    evidence.
  available_labels: *9
  prerequisites: *10
  evidence_ids: *11
  profile_id: retained-server-run-attach
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-retained-server-run-attach-linux-native
  os: linux
  origin: native
  method: process_inspection
  locator: Use retained launch metadata for retained-server-run-attach; verify the
    explicit endpoint before listing provider sessions. Process discovery alone is
    insufficient.
  identity_check: Use endpoint plus provider session ID and directory/workspace; reject
    PID-only identity.
  liveness_check: Health and session reads on the revalidated endpoint.
  state_detection: Status/SSE plus transcript progress; absence from status map means
    idle only according to current source, but stale endpoints and races remain possible.
  available_labels: *12
  prerequisites: *13
  evidence_ids: *14
  profile_id: retained-server-run-attach
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-retained-server-run-attach-linux-claudine
  os: linux
  origin: claudine
  method: claudine_registration
  locator: Use retained launch metadata for retained-server-run-attach; verify the
    explicit endpoint before listing provider sessions. Process discovery alone is
    insufficient.
  identity_check: Endpoint plus provider session ID is authoritative; correlate but
    do not replace it with CLAUDINE_SESSION_ID.
  liveness_check: Health/session reads plus provider child/server liveness.
  state_detection: Provider status/SSE, with wrapper state only supplementary.
  available_labels: *15
  prerequisites: *16
  evidence_ids: *17
  profile_id: retained-server-run-attach
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-retained-server-run-attach-windows-native
  os: windows
  origin: native
  method: process_inspection
  locator: Use retained launch metadata for retained-server-run-attach; verify the
    explicit endpoint before listing provider sessions. Process discovery alone is
    insufficient.
  identity_check: Use endpoint plus provider session ID and directory/workspace, never
    PID alone.
  liveness_check: Authenticated health and session reads; native Windows process-start
    validation remains to be designed with Sniff.
  state_detection: Status/SSE semantics are source-derived and need native Windows
    verification.
  available_labels: *18
  prerequisites: *19
  evidence_ids: *20
  profile_id: retained-server-run-attach
  observation_source: Sniff process/program observation plus provider HTTP reads only
    when an endpoint is deliberately exposed.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
- id: discover-retained-server-run-attach-windows-claudine
  os: windows
  origin: claudine
  method: claudine_registration
  locator: Use retained launch metadata for retained-server-run-attach; verify the
    explicit endpoint before listing provider sessions. Process discovery alone is
    insufficient.
  identity_check: Endpoint plus provider session ID with Windows process-token user
    checks; CLAUDINE_SESSION_ID is only correlation metadata.
  liveness_check: Authenticated health/session reads and child/server liveness.
  state_detection: Provider status/SSE after native verification.
  available_labels: *21
  prerequisites: *22
  evidence_ids: *23
  profile_id: retained-server-run-attach
  observation_source: Claudine launch registration plus provider HTTP reads where
    reachable.
  observed_at: 2026-09-08 passive research snapshot; runtime observations must be
    refreshed before delivery
mechanisms:
- id: http-prompt-async-active
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Launch an HTTP-exposed TUI or retained serve instance, authenticate,
    resolve the exact workspace and session, and observe current status before submission.
  operation_intent: steer_active_turn
  conversation_effect: preserve_running_turn
  delivery_boundary: unknown
  destination: Exact provider session ID on the owning OpenCode HTTP server/workspace.
  authentication: Optional HTTP Basic; Claudine-managed profiles should require a
    password because loopback is not a same-user boundary.
  startup_requirements: &24
  - known reachable HTTP endpoint
  - correct directory/workspace routing
  - Basic credentials if configured
  - native provider session ID
  target_preconditions:
  - session is busy on the selected endpoint
  - endpoint and directory still match the selected conversation
  target_guards:
  - No expected operation/turn ID guard is documented
  - Absent guard permits a final-state race; revalidate status but do not assume atomicity
  request_framing: HTTP POST /session/:id/prompt_async; JSON body.
  response_framing: HTTP 204 No Content on initial acceptance; later reads/SSE are
    separate.
  request_format: Optional caller-selected messageID plus typed parts such as text;
    directory/workspace routing identifies the instance.
  response_format: 204 contains no delivery object. Correlate the chosen messageID
    through message reads and SSE when available.
  long_tool_behavior: v1.18.29 source can reread a persisted message between loop
    iterations, but reliable absorption after a long tool is unverified.
  tool_batch_behavior: unknown
  queue_behavior: No documented durable follow-up queue; ensureRunning waits on an
    existing run and does not establish a second scheduled run.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable; this mechanism does not intentionally
    interrupt.
  ordering: Concurrent persistence and incorporation ordering are undocumented.
  sender_message_id: supported
  retry_policy: never_retry
  duplicate_handling: Caller-selected messageID exists, but duplicate suppression/idempotency
    is undocumented.
  cancellation: No selected-message recall is documented; session abort affects the
    whole active run.
  limits: Body, queue, and expiry limits are undocumented.
  evidence_ids: *1
- id: http-prompt-sync-active
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Same endpoint, authentication, workspace, session, and status initialization
    as async prompt.
  operation_intent: steer_active_turn
  conversation_effect: preserve_running_turn
  delivery_boundary: unknown
  destination: Exact provider session ID on the owning server/workspace.
  authentication: Optional HTTP Basic; managed launches should require it.
  startup_requirements: *24
  target_preconditions:
  - session is busy on the selected endpoint
  target_guards:
  - No expected operation ID guard is documented
  request_framing: HTTP POST /session/:id/message with streamed response.
  response_framing: Streamed JSON assistant message or HTTP error; timeout can be
    semantically ambiguous.
  request_format: Optional messageID/model/agent/noReply/system/tools plus required
    parts.
  response_format: Assistant message stream; a response after waiting on prior work
    does not alone prove a distinct follow-up turn.
  long_tool_behavior: Uses the same persisted-before-ensureRunning path; busy delivery
    boundary remains unverified.
  tool_batch_behavior: unknown
  queue_behavior: No documented independent busy-session queue.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable.
  ordering: Undocumented for concurrent callers.
  sender_message_id: supported
  retry_policy: never_retry
  duplicate_handling: Idempotency is undocumented.
  cancellation: Canceling the HTTP request is not documented to cancel provider work.
  limits: Request and concurrency limits are undocumented.
  evidence_ids: *2
- id: http-prompt-async-idle
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Reach the exposed server, authenticate, select an idle provider
    session in the correct workspace, then submit.
  operation_intent: start_idle_turn
  conversation_effect: resume_same_conversation
  delivery_boundary: idle_turn_start
  destination: Exact idle provider session ID on the owning server/workspace.
  authentication: Optional HTTP Basic; managed launches should require it.
  startup_requirements: *24
  target_preconditions:
  - session is idle immediately before submission
  target_guards:
  - No atomic idle-state or expected operation guard is documented
  request_framing: HTTP POST /session/:id/prompt_async with JSON body.
  response_framing: HTTP 204 No Content; later SSE/message/status observations establish
    progress.
  request_format: Optional caller-selected messageID plus typed parts.
  response_format: 204 acknowledges request handling only, not completed conversation
    delivery.
  long_tool_behavior: Not applicable at initial submission because the selected session
    is idle; an idle/busy race remains possible.
  tool_batch_behavior: not_applicable
  queue_behavior: Starts prompt processing for an idle session; persistence/scheduling
    timing relative to 204 remains unverified.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable.
  ordering: Concurrent idle submissions are undocumented.
  sender_message_id: supported
  retry_policy: never_retry
  duplicate_handling: Idempotency is undocumented.
  cancellation: Session abort is the only documented run-level cancellation.
  limits: Body and concurrency limits are undocumented.
  evidence_ids: *3
- id: http-prompt-sync-idle
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Reach, authenticate, and select an idle exact session/workspace.
  operation_intent: start_idle_turn
  conversation_effect: resume_same_conversation
  delivery_boundary: idle_turn_start
  destination: Exact idle provider session ID.
  authentication: Optional HTTP Basic; managed launches should require it.
  startup_requirements: *24
  target_preconditions:
  - session is idle immediately before submission
  target_guards:
  - No atomic idle-state guard is documented
  request_framing: HTTP POST /session/:id/message with streamed response.
  response_framing: Streamed JSON assistant message or HTTP error.
  request_format: Typed prompt request with parts and optional messageID.
  response_format: Assistant result stream; completion can be correlated with message
    identifiers.
  long_tool_behavior: Not applicable at idle submission; races require a test.
  tool_batch_behavior: not_applicable
  queue_behavior: Starts work on the idle conversation; concurrent ordering is undocumented.
  message_interpretation: provider_defined
  interruption_phases: []
  interruption_partial_failure: Not applicable.
  ordering: Concurrent caller ordering undocumented.
  sender_message_id: supported
  retry_policy: never_retry
  duplicate_handling: Idempotency is undocumented.
  cancellation: Abort is separate.
  limits: Request and response limits undocumented.
  evidence_ids: *4
- id: http-abort-then-prompt
  interface_status: documented
  maturity: stable
  transport: http
  initialization: Reach and revalidate the exact busy session; obtain explicit interactive
    approval; retain endpoint/authentication for both requests.
  operation_intent: interrupt_then_submit
  conversation_effect: cancel_turn_same_conversation
  delivery_boundary: next_turn
  destination: Abort and then prompt the same provider session ID on the same revalidated
    server/workspace.
  authentication: Same Basic credentials for both independent operations.
  startup_requirements:
  - known reachable HTTP endpoint
  - correct directory/workspace routing
  - Basic credentials if configured
  - native provider session ID
  - explicit interactive user approval
  target_preconditions:
  - selected session is still busy
  - user approved the described interruption effect
  target_guards:
  - No expected operation ID guard is documented
  - Revalidate session state between phases; stale/absent busy state means do not
    abort
  request_framing: POST /session/:id/abort, observe quiescence, then separately POST
    prompt_async or message.
  response_framing: Abort returns a boolean; replacement has its own independent HTTP
    response.
  request_format: Two or more separately framed operations with no transaction.
  response_format: Boolean abort result plus later prompt acknowledgment/result and
    SSE/status observations.
  long_tool_behavior: Cancellation is intended to stop the session runner; exact child-tool
    cleanup is unverified on every OS.
  tool_batch_behavior: stop_remaining
  queue_behavior: No atomic replacement queue; after successful abort, failure to
    submit leaves the conversation idle with the original work canceled.
  message_interpretation: provider_defined
  interruption_phases:
  - revalidate exact endpoint/session and busy state
  - explain effect and obtain explicit interactive approval
  - submit abort
  - observe abort/idle evidence
  - revalidate same conversation
  - submit replacement prompt
  - observe delivery
  interruption_partial_failure: Abort may succeed while replacement submission fails;
    the prior turn remains canceled and no replacement is guaranteed or retained.
  ordering: Claudine must serialize phases, but other senders can race because no
    operation guard exists.
  sender_message_id: supported
  retry_policy: never_retry
  duplicate_handling: No combined idempotency contract; replacement IDs do not make
    abort atomic.
  cancellation: An accepted abort cannot be undone; a submitted replacement requires
    another session abort to stop.
  limits: Manual fallback only; never use for automatic warnings.
  evidence_ids: *5
compatibility:
- mechanism_id: http-prompt-async-active
  profile_id: exposed-tui-http
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: exposed-tui-http
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: exposed-tui-http
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: exposed-tui-http
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: exposed-tui-http
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Do not abort or send.
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: exposed-tui-http
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: exposed-tui-http
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: exposed-tui-http
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: exposed-tui-http
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: exposed-tui-http
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Do not abort or send.
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: exposed-tui-http
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: exposed-tui-http
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: exposed-tui-http
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: exposed-tui-http
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: exposed-tui-http
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Do not abort or send.
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: retained-server-run-attach
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: retained-server-run-attach
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: retained-server-run-attach
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: retained-server-run-attach
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: retained-server-run-attach
  os: macos
  versions_verified:
  - 1.18.29 passive help/source only; no delivery test
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Do not abort or send.
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: retained-server-run-attach
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: retained-server-run-attach
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: retained-server-run-attach
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: retained-server-run-attach
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: retained-server-run-attach
  os: linux
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Do not abort or send.
  evidence_ids: *5
- mechanism_id: http-prompt-async-active
  profile_id: retained-server-run-attach
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *1
- mechanism_id: http-prompt-sync-active
  profile_id: retained-server-run-attach
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *2
- mechanism_id: http-prompt-async-idle
  profile_id: retained-server-run-attach
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *3
- mechanism_id: http-prompt-sync-idle
  profile_id: retained-server-run-attach
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Report unavailable/unknown and send nothing; never retry an ambiguous
    response.
  evidence_ids: *4
- mechanism_id: http-abort-then-prompt
  profile_id: retained-server-run-attach
  os: windows
  versions_verified: []
  documented_version_bounds: unknown
  read_only_check: Confirm binary/health version, authenticated endpoint ownership,
    OpenAPI route and schema, exact directory/workspace and session identity, status/SSE
    availability, and profile registration. Native OS process ownership must also
    match.
  success_criteria: All read-only checks agree on endpoint, route, profile, and exact
    session; a matching disposable delivery test is still mandatory.
  failure_behavior: Do not abort or send.
  evidence_ids: *5
verification: []
cases:
- profile_id: ordinary-tui
  os: macos
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-macos-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: &25
  - official-cli
  - source-launch-1-18-29
  - local-help-1-18-29
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: macos
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-macos-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-macos-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-macos-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: linux
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-linux-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: linux
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-linux-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-linux-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-linux-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: windows
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-windows-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: windows
  launch_mode: interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-windows-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-windows-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-tui
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-tui-windows-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *25
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: macos
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-run-macos-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: &26
  - official-cli
  - source-launch-1-18-29
  - local-help-1-18-29
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: macos
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-run-macos-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-run-macos-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-run-macos-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: linux
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-run-linux-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: linux
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-run-linux-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-run-linux-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-run-linux-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: windows
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-run-windows-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: windows
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-run-windows-native
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-ordinary-run-windows-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: ordinary-run
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: unknown
  discovery_ids:
  - discover-ordinary-run-windows-claudine
  mechanism_ids: []
  prerequisites: []
  evidence_ids: *26
  reason: Ordinary launch keeps its server transport internal or one-shot; no documented
    external attachment/registry exposes this existing conversation to an independent
    sender.
- profile_id: exposed-tui-http
  os: macos
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-exposed-tui-http-macos-native
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: &27
  - launch with an explicit stable hostname and port
  - configure Basic authentication
  - retain endpoint and native session correlation
  evidence_ids: &28
  - official-server
  - official-cli
  - source-launch-1-18-29
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: exposed-tui-http
  os: macos
  launch_mode: interactive
  origin: native
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-exposed-tui-http-macos-native
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *27
  evidence_ids: *28
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: exposed-tui-http
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-exposed-tui-http-macos-claudine
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *27
  evidence_ids: *28
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: exposed-tui-http
  os: macos
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-exposed-tui-http-macos-claudine
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *27
  evidence_ids: *28
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: exposed-tui-http
  os: linux
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-exposed-tui-http-linux-native
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *27
  evidence_ids: *28
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: exposed-tui-http
  os: linux
  launch_mode: interactive
  origin: native
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-exposed-tui-http-linux-native
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *27
  evidence_ids: *28
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: exposed-tui-http
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-exposed-tui-http-linux-claudine
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *27
  evidence_ids: *28
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: exposed-tui-http
  os: linux
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-exposed-tui-http-linux-claudine
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *27
  evidence_ids: *28
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: exposed-tui-http
  os: windows
  launch_mode: interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-exposed-tui-http-windows-native
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *27
  evidence_ids: *28
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: exposed-tui-http
  os: windows
  launch_mode: interactive
  origin: native
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-exposed-tui-http-windows-native
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *27
  evidence_ids: *28
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: exposed-tui-http
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-exposed-tui-http-windows-claudine
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *27
  evidence_ids: *28
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: exposed-tui-http
  os: windows
  launch_mode: interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-exposed-tui-http-windows-claudine
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *27
  evidence_ids: *28
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: retained-server-run-attach
  os: macos
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-retained-server-run-attach-macos-native
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: &29
  - start and retain opencode serve with a known endpoint
  - configure Basic authentication
  - launch run with --attach and retain native session correlation
  evidence_ids: &30
  - official-server
  - official-cli
  - local-help-1-18-29
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: retained-server-run-attach
  os: macos
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-retained-server-run-attach-macos-native
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *29
  evidence_ids: *30
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: retained-server-run-attach
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-retained-server-run-attach-macos-claudine
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *29
  evidence_ids: *30
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: retained-server-run-attach
  os: macos
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-retained-server-run-attach-macos-claudine
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *29
  evidence_ids: *30
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: retained-server-run-attach
  os: linux
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-retained-server-run-attach-linux-native
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *29
  evidence_ids: *30
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: retained-server-run-attach
  os: linux
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-retained-server-run-attach-linux-native
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *29
  evidence_ids: *30
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: retained-server-run-attach
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-retained-server-run-attach-linux-claudine
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *29
  evidence_ids: *30
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: retained-server-run-attach
  os: linux
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-retained-server-run-attach-linux-claudine
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *29
  evidence_ids: *30
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: retained-server-run-attach
  os: windows
  launch_mode: non_interactive
  origin: native
  session_state: working
  support: unknown
  discovery_ids:
  - discover-retained-server-run-attach-windows-native
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *29
  evidence_ids: *30
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: retained-server-run-attach
  os: windows
  launch_mode: non_interactive
  origin: native
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-retained-server-run-attach-windows-native
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *29
  evidence_ids: *30
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
- profile_id: retained-server-run-attach
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: working
  support: unknown
  discovery_ids:
  - discover-retained-server-run-attach-windows-claudine
  mechanism_ids:
  - http-prompt-async-active
  - http-prompt-sync-active
  - http-abort-then-prompt
  prerequisites: *29
  evidence_ids: *30
  reason: HTTP prompt and abort routes exist, but busy-session delivery boundary,
    token-only generation behavior, expected-operation guarding, and interruption
    effects lack disposable verification.
- profile_id: retained-server-run-attach
  os: windows
  launch_mode: non_interactive
  origin: claudine
  session_state: idle
  support: non_interrupting
  discovery_ids:
  - discover-retained-server-run-attach-windows-claudine
  mechanism_ids:
  - http-prompt-async-idle
  - http-prompt-sync-idle
  prerequisites: *29
  evidence_ids: *30
  reason: The documented prompt API addresses the same retained idle session and starts
    work without canceling a turn; activation remains blocked by the empty live-verification
    set and OS-specific gaps.
gaps:
- area: ordinary-session external access
  detail: Revision 2 now separates ordinary internal launches from exposed-server
    profiles, but no documented registry or attachment mechanism makes an already-open
    ordinary TUI or one-shot run externally steerable.
  next_check: Keep ordinary profiles unknown; search future release notes/source for
    an authenticated peer endpoint or launch registry and require a compatible read-only
    probe.
- area: server identity and discovery
  detail: No official per-user registry maps ordinary processes to random/internal
    endpoints. An endpoint may host multiple sessions and an OpenCode session may
    outlive a client process.
  next_check: Design Claudine-owned registration for future launches and disposable
    tests for config-derived ports and stale/reused endpoints.
- area: busy prompt semantics
  detail: v1.18.29 source persists before ensureRunning and does not schedule a second
    run when busy. Tool-boundary absorption and generation-only stranding are control-flow
    inferences; prompt versus prompt_async behavior requires direct evidence.
  next_check: Test long token generation, long tool, post-delivery permission blocking,
    and multi-tool turns with unique message IDs on disposable servers.
- area: delivery lifecycle vocabulary
  detail: HTTP accepted, persisted, absorbed by current loop, scheduled as a new run,
    assistant-started, completed, and possibly stranded are distinct. A later tool/question
    approval block is an execution state, not an incoming-message delivery state.
    The current delivery_states vocabulary cannot cleanly represent persisted-but-unscheduled
    or absorbed-at-boundary.
  next_check: Add persisted, scheduled, processing/absorbed, completed, canceled,
    and provider-detail states while keeping post-delivery approval state separate.
- area: message identity
  detail: The schema has no first-class request/message correlation field or idempotency
    capability; OpenCode accepts caller-selected messageID but semantics are unverified.
  next_check: Add correlation/idempotency fields and test duplicate message IDs, retries
    after timeout, and concurrent senders.
- area: session state confidence
  detail: Status exposes idle/retry/busy and idle sessions are absent from the current
    in-memory map; status can race transcript/event progress and permission holds
    are separate resources.
  next_check: Model raw provider state, observation timestamp/source, confidence,
    and pending permission/question state.
- area: authentication boundary
  detail: HTTP Basic is optional and loopback is not a same-user security boundary.
    Credentials must not be disclosed in discovery output.
  next_check: Require Claudine-owned authenticated launches or prove same-user endpoint
    controls; threat-model local cross-user/container access.
- area: interruption partial failure
  detail: Abort and replacement are separate operations; abort may succeed and delivery
    fail. Tool subprocess cleanup and history consistency are unverified.
  next_check: Add operation-phase outcomes and disposable interruption tests per OS.
- area: native OS evidence
  detail: Only passive macOS 1.18.29 inspection was performed; Linux and native Windows
    have documentation/source evidence only.
  next_check: Run non-focusing disposable tests on macOS, Linux, and native Windows.
- area: version gating
  detail: Official docs state no historical version bounds, and current development
    source has diverged substantially from v1.18.29.
  next_check: Derive feature probes from /global/health and /doc rather than guessing
    semantic version ranges; pin live results.
- area: mandatory activation gate
  detail: No disposable-session delivery record exists.
  next_check: Keep every OpenCode adapter disabled until matching live verification
    proves destination identity, busy/idle outcome, acknowledgment, and interruption
    effects.
- area: acceptance acknowledgment
  detail: Async HTTP 204 confirms initial request acceptance only; persistence, scheduling,
    and same-conversation delivery remain independently unknown.
  next_check: Implement correlated message/SSE/status observation and keep delivery
    outcome unknown on timeout or disconnect; do not retry without proven idempotency.
changes:
- Refreshed the OpenCode passive pilot from steering schema revision 1 to revision
  2.
- Split ordinary TUI, ordinary one-shot run, exposed TUI HTTP, and retained serve/run-attach
  into stable launch profiles.
- Separated active-turn, idle-turn, synchronous, asynchronous, and interrupt-then-submit
  intent; added receipt guarantees, target guards, framing, retry policy, and complete
  profile/OS/origin discovery and compatibility relationships.
- Reconfirmed local OpenCode 1.18.29 and launch/help surfaces with Sniff and passive
  CLI inspection; no live delivery test was run.
requires_claudine_update: true
reason: OpenCode has a candidate HTTP control surface for deliberately exposed profiles,
  but ordinary sessions are not externally reachable and safe activation requires
  launch registration, authenticated endpoints, read-only compatibility probes, exact
  session correlation, and matching disposable tests.
launch_profiles:
- id: ordinary-tui
  description: Ordinary interactive OpenCode TUI using its internal worker transport
    and no deliberately exposed HTTP endpoint.
  endpoint_scope: internal
  lifetime: while_client_open
  applicable_os: &31
  - macos
  - linux
  - windows
  applicable_launch_modes:
  - interactive
  applicable_origins: &32
  - native
  - claudine
  baseline: true
  startup_requirements: []
  preserves_extensions: 'yes'
  preserves_skills: 'yes'
  preserves_templates: 'yes'
  preserves_context: 'yes'
  evidence_ids: *25
- id: ordinary-run
  description: Ordinary one-shot opencode run with its in-process random-port server
    and no retained external endpoint.
  endpoint_scope: internal
  lifetime: one_shot
  applicable_os: *31
  applicable_launch_modes:
  - non_interactive
  applicable_origins: *32
  baseline: true
  startup_requirements: []
  preserves_extensions: 'yes'
  preserves_skills: 'yes'
  preserves_templates: 'yes'
  preserves_context: 'yes'
  evidence_ids: *26
- id: exposed-tui-http
  description: Interactive TUI deliberately launched with a known HTTP hostname/port
    and optional Basic authentication.
  endpoint_scope: externally_reachable
  lifetime: while_client_open
  applicable_os: *31
  applicable_launch_modes:
  - interactive
  applicable_origins: *32
  baseline: false
  startup_requirements: *27
  preserves_extensions: 'yes'
  preserves_skills: 'yes'
  preserves_templates: 'yes'
  preserves_context: 'yes'
  evidence_ids: *28
- id: retained-server-run-attach
  description: Long-lived headless OpenCode server with a non-interactive run client
    attached by URL.
  endpoint_scope: externally_reachable
  lifetime: long_lived
  applicable_os: *31
  applicable_launch_modes:
  - non_interactive
  applicable_origins: *32
  baseline: false
  startup_requirements: *29
  preserves_extensions: 'yes'
  preserves_skills: 'yes'
  preserves_templates: 'yes'
  preserves_context: 'yes'
  evidence_ids: *30
receipt_guarantees:
- mechanism_id: http-prompt-async-active
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - message read/list
  - session status
  - SSE message/session events
  - streamed assistant response for synchronous route
  correlation: message_id
  evidence_ids: *1
  limitations: A successful synchronous HTTP response confirms request acceptance,
    but it can arrive only after waiting on prior work. A timeout or disconnect
    before that response is ambiguous and must not be retried; acceptance does not
    independently prove durable persistence, scheduling, or conversation delivery.
- mechanism_id: http-prompt-sync-active
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - message read/list
  - session status
  - SSE message/session events
  - streamed assistant response for synchronous route
  correlation: message_id
  evidence_ids: *2
  limitations: The initial acknowledgment does not independently prove durable persistence,
    scheduling, or delivery into the intended conversation; later signals must be
    correlated.
- mechanism_id: http-prompt-async-idle
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - message read/list
  - session status
  - SSE message/session events
  - streamed assistant response for synchronous route
  correlation: message_id
  evidence_ids: *3
  limitations: The initial acknowledgment does not independently prove durable persistence,
    scheduling, or delivery into the intended conversation; later signals must be
    correlated.
- mechanism_id: http-prompt-sync-idle
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - message read/list
  - session status
  - SSE message/session events
  - streamed assistant response for synchronous route
  correlation: message_id
  evidence_ids: *4
  limitations: A successful synchronous HTTP response confirms request acceptance,
    but response latency may include the provider run. A timeout or disconnect
    before that response is ambiguous and must not be retried; acceptance does not
    independently prove durable persistence, scheduling, or conversation delivery.
- mechanism_id: http-abort-then-prompt
  request_acceptance: confirmed
  persistence: unknown
  scheduling: unknown
  conversation_delivery: unknown
  provider_signals:
  - abort boolean
  - session.status/session.idle SSE
  - replacement message/status events
  correlation: unknown
  evidence_ids: *5
  limitations: Initial abort acknowledgment proves neither completed cancellation
    nor replacement acceptance/delivery.
---
# Steering Research: OpenCode CLI

## Overview

OpenCode 1.18.29 has a documented HTTP/OpenAPI control surface that can address a
native session ID, submit prompts, observe events and status, and abort a run.
That is a plausible steering substrate only when Claudine knows the exact server
endpoint, workspace/directory routing, authentication, and provider session ID.
The ordinary TUI and ordinary `run` path do not provide those facts to an
independent process: the TUI normally uses internal worker transport, while
`run` owns an in-process server unless it attaches to a separately retained one.

This passive refresh was performed by agent `codex` with launcher-supplied model
`gpt-5.6-sol` and reasoning effort `low`. The execution surface did not expose
independent resolved-model metadata, so model and effort provenance is
launcher-supplied rather than independently verified. No session was launched,
messaged, or interrupted. `verification` is therefore empty and activation
remains blocked.

## Session discovery

The provider API can list sessions after a server is known. It is not a discovery
mechanism for the server itself. No official same-user registry was found that
maps an ordinary OpenCode process to its internal/random endpoint. Process
inspection can identify candidates, but a PID is neither a conversation ID nor
proof that a server is externally reachable. One server may own many sessions.

For Claudine-launched sessions, the durable solution is launch-time registration:
record server ownership/profile, URL, child process identity, directory/workspace,
and the OpenCode `ses...` identifier returned by the owning endpoint. The current
`CLAUDINE_SESSION_ID` must remain correlation metadata unless explicitly mapped to
that native ID.

## Non-interrupting delivery

`POST /session/:id/prompt_async` returns 204 and the OpenAPI description calls
that “accepted.” The handler forks prompt work and returns without awaiting it,
so 204 precedes any proven persistence. Inside the fork, 1.18.29 prompt handling
first persists the user message and then calls `ensureRunning`. If a run already exists, the runner waits
for it instead of scheduling another run. The active loop rereads messages
between iterations, so a message might be absorbed after a tool boundary. The
source does not establish a durable next-turn queue. Generation-only stranding
is a control-flow inference that requires a disposable test. Thus 204 proves neither queued execution
nor delivery. The synchronous `/message` route uses the same core path and can
block on the old run without proving a distinct follow-up turn occurred.

This means neither endpoint is yet proven suitable for automatic loop warnings.
A token-generation loop that never reaches another tool boundary cannot be
assumed to receive the message.

## Interruption fallback

`POST /session/:id/abort` and the later prompt are separate operations. Current
source cancels the session runner and related background jobs, then leaves the
conversation history available. Claudine would have to obtain explicit manual
approval, revalidate the same endpoint/session, abort, observe bounded quiescence,
and submit a new prompt. If the last request fails, the original work has already
been stopped. Automatic loop intervention cannot use this fallback.

## Idle sessions

The API documents prompting an existing session and starting it if needed, so an
idle session on a retained reachable server is a credible candidate. An ordinary
completed `opencode run` process is not an idle open session. The 24-case matrix
mixes these distinct shapes, so its cases remain `unknown` until launch profile is
represented. The documented capability for a retained reachable server is
separate from the mandatory live-test activation gate.

## Protocol details

The transport is HTTP with JSON bodies and SSE events. A caller can choose a
`messageID`, which is promising for correlation, but idempotency and retry rules
are undocumented. `prompt_async` returns no body. `/session/status` reports
`busy`, `retry`, or `idle`; current source removes idle sessions from its active
map. Pending permission and question resources must be inspected separately.
Non-interactive operation cannot provide human approval. Such a request can block
execution after the message was already consumed; it is not evidence that message
delivery itself was held or queued.

Server authentication is optional HTTP Basic. A password should be required for
a Claudine-owned endpoint. Binding to loopback limits network reach but does not
authenticate the current OS user and must not be treated as a private per-user
channel by itself.

## OS and version compatibility

The installed macOS binary and pinned tag are 1.18.29. Official docs present the
same TCP interface for supported platforms, but no Linux or native Windows
runtime was inspected. WSL would count only as Linux-side evidence. No reliable
documented introduction versions were found. Compatibility should probe
`/global/health` and the live `/doc` schema, then still require a version/profile
matched disposable delivery test. The current development commit was inspected
only to detect drift; it is not a released compatibility claim.

## Disposable-test proposals

For each OS and launch profile, start an authenticated disposable server without
focusing a terminal window and capture sanitized message/event fixtures. Test:

1. idle prompt and unique message correlation;
2. long token generation with no tools;
3. a long-running tool and a multi-tool turn;
4. post-delivery tool/question approval blocking with no human approval;
5. concurrent senders, duplicate message IDs, client timeout, and retry;
6. abort during generation and tool execution, followed by both successful and
   failed replacement submission;
7. process exit, stale endpoint, reused PID/port, wrong directory/workspace, and
   wrong session ID.

Success requires proof that the selected conversation received the exact message,
plus a terminal delivery outcome. HTTP status, persistence, or SSE connection
alone is insufficient.

## Claudine integration

Future managed launches should make the server profile explicit. A safe adapter
would register an authenticated loopback endpoint, server ownership, health
version, provider session ID, and routing context, then revalidate all of them
before each send. Existing ordinary sessions without such registration should
remain visible but unavailable or unknown. `serve`, exposed TUI, embedded `run`,
and `run --attach` need separate profile records because their lifetimes and
reachability differ.

## Gaps

Revision 2 resolves the earlier launch-profile ambiguity, but provider and
implementation gaps remain: ordinary sessions expose no documented external
endpoint registry, async acceptance does not prove persistence or delivery,
busy-turn incorporation is unresolved, and message idempotency is undocumented.
Native Linux and Windows behavior, interruption cleanup, and every live delivery
condition still require the proposed disposable tests. The frontmatter `gaps`
records the corresponding blockers and next checks.

## Sources

- [OpenCode server documentation](https://opencode.ai/docs/server/)
- [OpenCode CLI documentation](https://opencode.ai/docs/cli/)
- [OpenCode v1.18.29 source](https://github.com/anomalyco/opencode/tree/16747470f976aca3d362ad730bcd3fe82ecc2c9a)
- [OpenCode development source inspected on 2026-09-08](https://github.com/anomalyco/opencode/tree/d6855b6b47a8433462ac6aeeba882ccf734cb7f1)

## Changelog

- 2026-09-08: Refreshed revision 1 into schema revision 2; separated ordinary and HTTP-exposed launch profiles, expanded protocol/receipt metadata, and completed relational coverage.
- 2026-09-08: Preserved the initial passive findings for OpenCode 1.18.29; no live delivery evidence was added.
