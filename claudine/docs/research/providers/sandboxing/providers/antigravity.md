---
$schema: ./_schema.yaml
schema_revision: 5
provider: antigravity
created: 2026-11-04
last_updated: 2026-11-04
content_policy: [ValidFor(14d)]
agent: codex
model: gpt-6.1-sol
versions_examined: [1.4.10, 1.5.0]
summary: >-
  Local CLI installations default to approval checks without OS isolation.
  Optional terminal sandboxing uses native OS backends; Auto mode enables it
  but can approve host execution. Eligible accounts can opt into Remote Agents.
support: native
evidence:
  - id: cli-reference
    method: official_docs
    location: https://antigravity.google/docs/cli/reference
    version: unknown
    observed_on: 2026-11-04
    claim: CLI defaults leave sandboxing off and request review; flags and settings govern workspace and execution policy.
    limitations: Mutable documentation; not runtime enforcement evidence.
  - id: sandbox-docs
    method: official_docs
    location: https://antigravity.google/docs/sandbox?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: Optional native terminal backends enforce filesystem and network restrictions, permit authorized bypass, and become ineffective when unavailable.
    limitations: No full profiles, symlink algorithm, helper containment, or WSL2-specific verification.
  - id: modes-docs
    method: official_docs
    location: https://antigravity.google/docs/cli/modes
    version: unknown
    observed_on: 2026-11-04
    claim: Default mode reviews edits; Auto enables terminal isolation and evaluates host retries with platform and personal-account conditions.
    limitations: Planning instructions and approvals do not establish containment.
  - id: headless-docs
    method: official_docs
    location: https://antigravity.google/docs/cli/headless
    version: unknown
    observed_on: 2026-11-04
    claim: Print honors persisted policies and soft-denies unobtainable approvals while continuing with exit zero.
    limitations: Bypass flag wording is broader than the explicit rule precedence documented elsewhere.
  - id: permissions-docs
    method: official_docs
    location: https://antigravity.google/docs/permissions?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: Deny beats Ask beats Allow; file and URL grants shape sandbox access, and personal versus enterprise command bypass grants differ.
    limitations: Describes policy semantics rather than implementation internals.
  - id: settings-docs
    method: official_docs
    location: https://antigravity.google/docs/settings?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: Persistent JSON settings combine scoped rules with session overrides; sandboxProxy delegates URL enforcement to a custom proxy.
    limitations: Does not settle overlapping --settings and dedicated flags or headless malformed-setting handling.
  - id: onboarding-docs
    method: official_docs
    location: https://antigravity.google/docs/getting-started?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: First-run setup includes appearance, rendering, and workspace trust before indexing.
    limitations: Trust is not an OS sandbox activation choice.
  - id: changelog
    method: source_code
    location: https://github.com/google-antigravity/antigravity-cli/blob/c994f07d241d3b3b1902d5705008092beb00e099/CHANGELOG.md
    version: 1.5.0
    observed_on: 2026-11-04
    claim: Release history records credential and Git protections, Windows backend changes, subagent Auto fixes, and Remote Agents flag exposure.
    limitations: Repository artifact inspection only; release notes are not CLI implementation source or runtime tests.
  - id: public-tree
    method: source_code
    location: https://github.com/google-antigravity/antigravity-cli/tree/c994f07d241d3b3b1902d5705008092beb00e099
    version: 1.5.0
    observed_on: 2026-11-04
    claim: The examined public tree contains documentation and examples but no CLI implementation source.
    limitations: Prevents a source audit of native backend enforcement.
  - id: local-help
    method: local_inspection
    location: claudine/docs/research/sandboxing/providers/antigravity.md#local-observation-record
    version: 1.4.10
    observed_on: 2026-11-04
    claim: Local macOS version and help expose sandbox, mode, settings, add-dir, and dangerous permission skipping; remote-agent is absent.
    limitations: Read-only help inspection; no workload, user configuration inspection, or enforcement test.
  - id: subagents-docs
    method: official_docs
    location: https://antigravity.google/docs/subagents
    version: unknown
    observed_on: 2026-11-04
    claim: Subagents inherit sandbox settings and access scopes and bubble permission requests to the parent.
    limitations: Git worktrees do not establish OS process isolation.
  - id: mcp-docs
    method: official_docs
    location: https://antigravity.google/docs/mcp?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: MCP supports local stdio and remote HTTP servers with separate Ask-default tool grants.
    limitations: Does not establish that server processes share terminal sandbox enforcement.
  - id: hooks-docs
    method: official_docs
    location: https://antigravity.google/docs/hooks?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: Hooks execute configured commands and can change tool permission decisions.
    limitations: Does not establish hook-process OS containment.
  - id: sidecars-docs
    method: official_docs
    location: https://antigravity.google/docs/sidecars
    version: unknown
    observed_on: 2026-11-04
    claim: Sidecars launch configured executables with their directory as working directory.
    limitations: Does not establish their sandbox or host access boundary.
  - id: enterprise-docs
    method: official_docs
    location: https://antigravity.google/docs/enterprise
    version: unknown
    observed_on: 2026-11-04
    claim: Enterprise guidance names bubblewrap on Linux and describes service IAM, licenses, and VPC controls.
    limitations: No verified administrator-locked local sandbox setting or separately installed bubblewrap prerequisite.
  - id: remote-docs
    method: official_docs
    location: https://antigravity.google/docs/remote-agents?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: Eligible Pro and Ultra personal accounts can run agents remotely, without local file access, using separate remote permissions; print is unsupported.
    limitations: Docs require 1.4.12+ while changelog exposes flag help in 1.5.0; rollout and underlying isolation technology remain unverified.
  - id: wsl-inference
    method: inference
    location: https://antigravity.google/docs/sandbox?tab=cli
    version: unknown
    observed_on: 2026-11-04
    claim: WSL2 Linux installations are expected to follow Linux defaults and namespace requirements.
    limitations: No WSL2-specific documented guarantee or runtime test; native Windows support is independently documented.
default_profiles:
  - id: macos-interactive
    environment: macos
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: Clean agy uses default mode and request-review; first-run trust precedes indexing and does not enable sandboxing. Approved commands run on the host.
    evidence_ids: [cli-reference, modes-docs, onboarding-docs]
  - id: macos-non-interactive
    environment: macos
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: Clean agy -p has no sandbox; unavailable approvals are soft-denied. Existing saved settings can change this.
    evidence_ids: [cli-reference, headless-docs]
  - id: linux-interactive
    environment: linux
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: Clean default mode requests review; optional namespaces are not activated by workspace trust.
    evidence_ids: [cli-reference, modes-docs, onboarding-docs]
  - id: linux-non-interactive
    environment: linux
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: Clean print mode has no sandbox and soft-denies approval-dependent tools; zero exit does not imply tool success.
    evidence_ids: [cli-reference, headless-docs]
  - id: windows-interactive
    environment: windows
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: Clean default mode requests review. Optional native isolation requires Windows 11 24H2 and the August 2026 update or later.
    evidence_ids: [cli-reference, sandbox-docs, onboarding-docs]
  - id: windows-non-interactive
    environment: windows
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: Clean print mode has no sandbox; saved settings are respected and unobtainable approvals soft-denied.
    evidence_ids: [cli-reference, headless-docs]
  - id: wsl2-interactive
    environment: wsl2
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: General clean CLI default is off; application to WSL2 is inferred and not runtime-verified. Optional Linux backend depends on distribution kernel policy.
    evidence_ids: [cli-reference, wsl-inference]
  - id: wsl2-non-interactive
    environment: wsl2
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: General print default is off with soft denial; WSL2-specific parity is inferred and needs confirmation.
    evidence_ids: [cli-reference, headless-docs, wsl-inference]
mechanisms:
  - id: seatbelt
    name: Terminal sandbox on macOS
    enforcement: os_sandbox
    environments: [macos]
    backend: Seatbelt SBPL profiles applied through sandbox-exec.
    prerequisites: []
    activation: Opt in with --sandbox or enableTerminalSandbox; personal-account Auto enables it from 1.3.5 onward.
    filesystem: Workspace, temp, common build caches, and artifact/scratch directories writable; system paths readable; grants expand access; SSH, .env, application support credentials, Docker/Kubernetes credentials and netrc protected; .git read-only. Exact lists and symlink handling unverified.
    network: Shell network denied by default; read_url domains permitted through proxy; custom sandboxProxy must enforce grants. Harness model transport is separate; exact DNS, ingress and loopback handling unverified.
    processes: Terminal commands and descendants constrained; harness remains outside. Environment-secret and inherited-handle filtering unverified.
    coverage: Shell tools and subagent shell tools inherit settings; built-in file tools have application checks. MCP, hooks and plugin sidecar OS containment unresolved.
    escape_hatches: Host retries require approval or matching grants, except Auto may approve safe retries. Permission skipping removes review but is not documented as disabling the backend itself.
    failure_behavior: fallback_unsandboxed
    failure_notes: From 1.4.4 unavailable backend makes --sandbox ineffective and proceed-in-sandbox request review; headless missing approvals soft-denied. Other setup failures unverified.
    evidence_ids: [sandbox-docs, changelog, modes-docs, permissions-docs, settings-docs, headless-docs, subagents-docs, mcp-docs, hooks-docs, sidecars-docs]
  - id: linux-namespaces
    name: Terminal sandbox on Linux
    enforcement: os_sandbox
    environments: [linux, wsl2]
    backend: Kernel filesystem, process and network namespaces; enterprise guide names bubblewrap. WSL2 application is conditional inference.
    prerequisites: [Unprivileged user namespaces permitted by kernel and host or container policy.]
    activation: Opt in with --sandbox or enableTerminalSandbox; personal-account Auto enables it from 1.3.5 onward.
    filesystem: Workspace, temp, caches and scratch writable; system paths readable; explicit read/write grants add paths; denied and unexposed paths inaccessible; sensitive paths blocked and .git read-only. Symlink behavior unverified.
    network: Namespace isolation with shell egress denied by default and read_url domain grants through proxy; custom proxy must enforce policy. Harness transport separate; exact DNS, ingress and loopback handling unverified.
    processes: Namespaces hide host processes and constrain command descendants; harness remains outside. Environment and IPC details unverified.
    coverage: Terminal tools including subagent commands; built-in file checks separate. MCP, hook and plugin helper containment unresolved.
    escape_hatches: Agent may request host retry; approval or grants authorize it, and Auto can approve safe host commands. Dangerous permission skipping can approve host requests.
    failure_behavior: fallback_unsandboxed
    failure_notes: From 1.4.4 blocked user namespaces disable sandbox requests; proceed-in-sandbox falls back to review. Headless unavailable Auto warns and uses default mode; missing approvals soft-denied.
    evidence_ids: [sandbox-docs, enterprise-docs, modes-docs, permissions-docs, settings-docs, headless-docs, changelog, subagents-docs, wsl-inference]
  - id: windows-appcontainer
    name: Terminal sandbox on Windows
    enforcement: os_sandbox
    environments: [windows]
    backend: AppContainer inside a process security environment on Windows 11 24H2 or later with August 2026 update or later.
    prerequisites: [Supported patched Windows version; administrator privileges not required since CLI 1.2.17.]
    activation: Opt in with --sandbox or enableTerminalSandbox; personal-account Auto available from 1.4.4 when backend is available.
    filesystem: Workspace, temporary directories and build caches writable; ungranted profile files and other user profiles hidden; folders outside current profile readable but not writable. Explicit grants expand access, sensitive files blocked, .git read-only; symlink handling unverified.
    network: Default shell network denied with read_url domain grants and proxy; external servers and some installs may require host retry. Harness model transport separate; exact DNS, ingress and loopback handling unverified.
    processes: Shell commands and children constrained by process security environment; complete token, IPC and inherited credential behavior unverified.
    coverage: Terminal tools and subagent terminal settings; no verified inclusion of built-in file tools, MCP, hooks or plugin sidecars in this OS boundary.
    escape_hatches: Agent can request host retry; standard review or grants apply, with Auto safety review where available. Approval bypass can authorize outside execution.
    failure_behavior: fallback_unsandboxed
    failure_notes: Older Windows versions cannot enforce sandbox; settings and flags have no effect from 1.4.4. Review fallback and headless soft-denial apply.
    evidence_ids: [sandbox-docs, modes-docs, changelog, permissions-docs, headless-docs, subagents-docs]
  - id: remote-agents
    name: Remote Agents
    enforcement: remote_environment
    environments: [macos, linux, windows]
    backend: Google-managed isolated remote execution environment; exact OS isolation technology undisclosed.
    prerequisites: [Personal Google AI Pro or Ultra subscription and rollout eligibility., CLI 1.4.12+ according to docs; 1.5.0 changelog lists flag help exposure.]
    activation: Opt in with --remote-agent and optional --project; absent from local 1.4.10 help. Print mode unsupported.
    filesystem: Agent cannot access local launch-directory files; operates on attached repositories, Drive and uploaded resources. Remote internal paths, temp files and symlink policy unverified.
    network: Remote service network boundary undisclosed; remote permission rules replace local rules and no local sandbox egress guarantee transfers.
    processes: Agent sessions and tools execute remotely rather than only shell subprocesses; local CLI is a client. VM/container and credential details undisclosed.
    coverage: Remote agent and tools; exact plugin, MCP and helper execution coverage unresolved. Print mode unavailable.
    escape_hatches: Remote permissions default to turbo and are separate from local settings; Remote Control alone runs on the user's host.
    failure_behavior: unknown
    failure_notes: Missing project names warn and select default remote project; infrastructure unavailability and isolation failure behavior unverified.
    evidence_ids: [remote-docs, changelog, local-help]
controls:
  - id: sandbox-flag
    kind: cli_flag
    name: --sandbox
    values: []
    effect: Requests native terminal containment for current local interactive or print session; ineffective on unsupported hosts.
    example: agy --sandbox -p "Run the test suite"
    precedence: Session override wins over persisted enableTerminalSandbox; cannot make an unavailable backend work.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [sandbox-docs, local-help, settings-docs]
  - id: sandbox-setting
    kind: config_key
    name: enableTerminalSandbox
    values: ['true', 'false']
    effect: Enables optional local terminal confinement; defaults false.
    example: '{"enableTerminalSandbox":true,"toolPermission":"proceed-in-sandbox"}'
    precedence: --sandbox overrides persistent setting; Auto enables sandbox automatically; unavailable backends ignore it.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [sandbox-docs, settings-docs]
  - id: mode-flag
    kind: cli_flag
    name: --mode
    values: [accept-edits, auto, plan]
    effect: Auto enables sandbox and safety-review of host commands; accept-edits and plan do not independently activate isolation.
    example: agy --mode=auto
    precedence: Overrides agentMode for session; Auto restricted by account and backend availability; strict and always-proceed policy override Auto approval behavior.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [modes-docs, permissions-docs, local-help]
  - id: mode-setting
    kind: config_key
    name: agentMode
    values: ['', accept-edits, auto, plan]
    effect: Persistent execution mode; empty default leaves sandbox off, Auto activates available local backend.
    example: '{"agentMode":"auto"}'
    precedence: --mode overrides saved mode; unavailable Auto falls back to default.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [modes-docs, settings-docs]
  - id: approval-policy
    kind: config_key
    name: toolPermission
    values: [request-review, proceed-in-sandbox, always-proceed, strict]
    effect: Controls review rather than creating isolation; proceed-in-sandbox permits confined commands and reviews host execution.
    example: '{"toolPermission":"proceed-in-sandbox","enableTerminalSandbox":true}'
    precedence: Deny applies in every mode; strict asks even with allow grants; always-proceed retains explicit Ask/Deny per settings docs. Dangerous flag interaction needs confirmation.
    mechanism_ids: []
    evidence_ids: [settings-docs, permissions-docs]
  - id: permission-rules
    kind: config_key
    name: permissions
    values: ['allow, ask, deny arrays of action(target) strings']
    effect: File grants expand sandbox mounts and URL grants expand egress; command grants can authorize host execution. Business/enterprise use unsandboxed grants for bypass.
    example: '{"permissions":{"allow":["read_file(/opt/reference)","read_url(registry.npmjs.org)"],"deny":["read_file(.env)"]}}'
    precedence: Deny > Ask > Allow; Project, Shared and Global combined; write implies read, deny read implies deny write. Explicit rules precede defaults.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [permissions-docs, sandbox-docs, settings-docs]
  - id: additional-workspace
    kind: cli_flag
    name: --add-dir
    values: [Directory path; repeatable.]
    effect: Adds another writable workspace folder to local tool and sandbox scope.
    example: agy --sandbox --add-dir /absolute/path/to/another/project
    precedence: Expands workspace rather than overriding explicit denial rules.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [cli-reference, sandbox-docs, local-help]
  - id: non-workspace-access
    kind: config_key
    name: allowNonWorkspaceAccess
    values: ['true', 'false']
    effect: Allows built-in file access beyond workspace subject to policy; not evidence of universal shell access or OS containment.
    example: 'RELAXES FILE SCOPE: {"allowNonWorkspaceAccess":true}'
    precedence: Default false; subject to agentMode, toolPermission and explicit permission rules.
    mechanism_ids: []
    evidence_ids: [cli-reference, settings-docs]
  - id: dangerous-approval-bypass
    kind: cli_flag
    name: --dangerously-skip-permissions
    values: []
    effect: Approves tools using always-proceed; may authorize host retry. Does not establish disabling an explicitly active native backend.
    example: 'RELAXES REVIEW: agy --sandbox --dangerously-skip-permissions -p "Run tests"'
    precedence: Session override of toolPermission, overriding Auto approval policy; exact explicit-rule interaction needs runtime confirmation due to documentation wording.
    mechanism_ids: []
    evidence_ids: [headless-docs, settings-docs, permissions-docs, local-help]
  - id: session-settings
    kind: cli_flag
    name: --settings
    values: [JSON object inline or file path.]
    effect: Local 1.4.10 help accepts session-only settings.json keys, including sandbox preferences.
    example: agy --settings '{"enableTerminalSandbox":true}'
    precedence: Never saved per help; precedence against overlapping dedicated flags unverified.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [local-help]
  - id: sandbox-proxy
    kind: config_key
    name: sandboxProxy.http
    values: [address and certFile required together; username and password optional paired credentials.]
    effect: Replaces built-in sandbox HTTP proxy; supplied proxy must enforce read_url grants.
    example: '{"sandboxProxy":{"http":{"address":"proxy.example.com:3128","certFile":"/etc/ssl/certs/proxy-ca.pem"}}}'
    precedence: Applies only when terminal sandbox active; incomplete field pairs cause setting to be ignored.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer]
    evidence_ids: [settings-docs]
  - id: config-panel
    kind: session_command
    name: /config
    values: []
    effect: Edits sandbox and execution preferences persistently; unavailable backend options hidden.
    example: Type /config in an interactive agy session.
    precedence: Saved changes cannot supersede enforced session flags until restart; remote session exposes separate remote settings.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer, remote-agents]
    evidence_ids: [settings-docs, sandbox-docs, remote-docs]
  - id: permissions-panel
    kind: session_command
    name: /permissions
    values: []
    effect: Edits scoped access grants; remote session edits remote rather than local rules.
    example: Type /permissions and select Project, Shared, or Global.
    precedence: Combines scopes under Deny > Ask > Allow; remote permissions replace local permissions for remote sessions.
    mechanism_ids: [seatbelt, linux-namespaces, windows-appcontainer, remote-agents]
    evidence_ids: [settings-docs, permissions-docs, remote-docs]
  - id: remote-flag
    kind: cli_flag
    name: --remote-agent
    values: []
    effect: Moves agent sessions and tools to Google-managed remote environment; requires eligible subscription and supported CLI. No print support.
    example: agy --remote-agent --project="My remote project"
    precedence: Remote permissions replace local sandbox and execution settings; new session required to change local versus remote conversation type.
    mechanism_ids: [remote-agents]
    evidence_ids: [remote-docs, changelog]
gaps:
  - location: default_profiles/wsl2-interactive
    detail: Generic CLI defaults applied to WSL2 by inference; Linux sandbox support not independently verified in a distribution.
    next_check: Inspect fresh WSL2 CLI settings and run namespace-backed nonsecret read/write/network probes.
  - location: default_profiles/wsl2-non-interactive
    detail: Print defaults and soft denial are general documentation without WSL2 observation.
    next_check: Run a clean non-interactive WSL2 fixture and verify tool denial, stderr and exit code.
  - location: mechanisms
    detail: No public implementation source or runtime enforcement tests; exact protected mounts, symlink resolution, environment and handle filtering, IPC, DNS, ingress and loopback behavior unresolved.
    next_check: Obtain backend source or trace bounded disposable per-platform probes, including symlink and inherited-resource cases.
  - location: mechanisms/linux-namespaces/prerequisites
    detail: Bubblewrap is named in enterprise guidance but packaging and external executable requirement are not established.
    next_check: Trace namespace setup in a disposable clean Linux installation with no separately installed bwrap.
  - location: mechanisms/seatbelt/coverage
    detail: Built-in file tools use policy checks; OS coverage of hooks, MCP, plugins and sidecars not established.
    next_check: Trace helper launches and nonsecret filesystem/network probes while terminal sandbox is enabled.
  - location: mechanisms/linux-namespaces/coverage
    detail: Helper and file-tool OS coverage is unresolved on Linux and WSL2.
    next_check: Trace helper namespaces and execute bounded marker probes independently on Linux and WSL2.
  - location: mechanisms/windows-appcontainer/coverage
    detail: Helper and file-tool inclusion in the process security environment is unresolved.
    next_check: Inspect process tokens and execute bounded helper marker probes on supported native Windows.
  - location: controls/dangerous-approval-bypass
    detail: Explicit-rule semantics and combined Auto activation have not been runtime-verified; docs describe the bypass more broadly than always-proceed policy.
    next_check: Test sandbox plus bypass, Auto plus bypass, and explicit Ask/Deny marker commands on disposable hosts.
  - location: controls/session-settings/precedence
    detail: Help documents session-only JSON but not conflicts with dedicated flags.
    next_check: Compare opposite enableTerminalSandbox values in --settings and --sandbox and observe effective backend.
  - location: mechanisms/remote-agents
    detail: Remote technology, internal file/network/credential boundaries, helper coverage and infrastructure failure behavior unknown; documented 1.4.12 minimum differs from 1.5.0 help announcement.
    next_check: Inspect eligible 1.4.12 and 1.5.0 help and remote sessions, obtain service architecture details, and exercise a disposable unavailable-service case.
  - location: controls
    detail: No dedicated sandbox environment variable or administrator-locked local policy verified; no exhaustive absence claim is made.
    next_check: Obtain full supported environment and managed-policy catalog or implementation source from vendor.
  - location: mechanisms/seatbelt/failure_notes
    detail: Other mid-session setup errors and malformed headless settings have no complete documented failure contract.
    next_check: Inject bounded setup failure and invalid configuration in isolated test installation and inspect effective enforcement and exit behavior.
changes: []
---

# Antigravity CLI sandboxing

Research checked **2026-11-04 UTC**. The local macOS installation reports **agy 1.4.10**. Current official documentation and the public repository at commit [`c994f07d241d3b3b1902d5705008092beb00e099`](https://github.com/google-antigravity/antigravity-cli/tree/c994f07d241d3b3b1902d5705008092beb00e099), committed November 4, were examined; its changelog reaches **1.5.0**. Documentation is mutable and includes features newer than the installed binary. No agent workload or isolation escape was executed.

## What a normal installation protects

**Antigravity CLI supplies an optional terminal sandbox, but leaves it off by default.** A clean configuration uses `agentMode: ""`, `enableTerminalSandbox: false`, `toolPermission: "request-review"`, and `allowNonWorkspaceAccess: false`. Default mode asks for shell execution and interactive review of file modifications; workspace reads are allowed. These are application permission checks, not an OS boundary around the agent. Once approved, an ordinary unsandboxed shell command has the invoking account’s host access. The desktop Antigravity App’s newer Default preset enables sandboxing; that is **not the CLI default**. [CLI reference](https://antigravity.google/docs/cli/reference#configuration-keys-settingsjson), [execution modes](https://antigravity.google/docs/cli/modes), [sandbox, CLI tab](https://antigravity.google/docs/sandbox?tab=cli).

The following table assumes no saved settings, explicit grants, startup overrides, or administrator changes. WSL2 is kept separate from native Windows; the WSL2 backend conclusion is conditional inference from the Linux requirements, not a WSL2 runtime observation.

| Platform | Interactive default | Non-interactive `agy -p` default | Optional local terminal isolation |
| --- | --- | --- | --- |
| macOS | Sandbox off; review commands and edits | Sandbox off; unavailable approvals are soft-denied | Seatbelt through `sandbox-exec` |
| Linux | Same | Same | Kernel namespaces; unprivileged user namespaces must be available |
| Native Windows | Same | Same | AppContainer inside a process security environment; Windows 11 24H2+ with August 2026 update or later |
| WSL2 | Same CLI defaults; WSL2-specific behavior unverified | Same documented print policy; WSL2-specific behavior unverified | Expected Linux backend inside the distribution if its kernel/policy permits it; not native Windows containment |

The first launch asks about appearance, rendering, and **workspace trust before indexing**. Trust records a decision about repository access; it does not enable an OS sandbox. Headless mode honors saved permission and execution settings, so an existing user’s print run need not resemble these clean defaults. A tool action whose approval cannot be obtained is denied while the run continues, normally exits zero, and names the tool and remedy on stderr. Exit zero alone therefore does not prove the requested work ran. [Getting started](https://antigravity.google/docs/getting-started?tab=cli), [headless permissions](https://antigravity.google/docs/cli/headless#permissions-in-headless-mode).

## Optional local isolation and its extent

Enable `--sandbox`, persist `enableTerminalSandbox: true`, or use `/config`. Personal accounts can also select Auto mode (`--mode=auto`), which enables sandboxing automatically: macOS/Linux require CLI 1.3.5+, Windows requires 1.4.4+. Auto is unavailable for business and enterprise accounts. It approves workspace edits and sandboxed commands, and has a separate agent evaluate commands requesting host execution; safe, reversible commands may run outside containment automatically. Auto is therefore not a promise that every command stays isolated. [Auto mode requirements](https://antigravity.google/docs/cli/modes#requirements-and-limitations).

The boundary contains **agent-launched terminal commands and their subprocesses**, rather than the entire local agent harness. macOS uses Seatbelt filesystem/socket profiles; Linux namespaces isolate filesystem, host processes, and networking. The enterprise guide names bubblewrap for Linux, but neither it nor the CLI guide establishes a separately installed `bwrap` executable requirement or its packaging. Native Windows uses a process security environment plus AppContainer, not Windows Sandbox or a VM. Windows commands no longer require administrator rights as of 1.2.17. No Docker daemon, container image, or VM setup is required for these native backends. [Sandbox mechanisms](https://antigravity.google/docs/sandbox?tab=cli#how-it-works), [enterprise execution guidance](https://antigravity.google/docs/enterprise#autonomous-tool-permissions-and-human-in-the-loop-controls), [1.2.17 release notes](https://github.com/google-antigravity/antigravity-cli/blob/c994f07d241d3b3b1902d5705008092beb00e099/CHANGELOG.md#1217).

```mermaid
flowchart LR
    U[Local agy harness and permission engine] --> M[Model or API transport]
    U --> F[Built-in file tools: application permission checks]
    U --> G{Command authorization}
    G --> S
    subgraph S[Optional native terminal boundary]
        C[Shell command and subprocesses]
    end
    G --> H[Approved host command outside sandbox]
    U --> X[MCP and customization execution: separate coverage questions]
```

### Files, credentials, and processes

With sandboxing enabled, workspace folders and `write_file` grants are writable; `read_file` grants add read-only access. Temporary directories, common build caches, and the agent’s artifact/scratch directories are also available. System directories such as `/usr` and `/etc` are readable for tool compatibility. This is broader than a workspace-only filesystem. On macOS/Linux, unexposed paths are inaccessible; on Windows, directories outside the current user profile remain readable but not writable, while other ungranted files in that profile and other users’ profiles are hidden. Read-only host access protects integrity, not confidentiality. [CLI filesystem integration](https://antigravity.google/docs/sandbox?tab=cli#cli-permissions-integration), [artifact/scratch behavior, 1.2.10](https://github.com/google-antigravity/antigravity-cli/blob/c994f07d241d3b3b1902d5705008092beb00e099/CHANGELOG.md#1210).

Documented sensitive exclusions include `~/.ssh` and `.env`. Release notes establish additional protections: 1.3.3 blocks macOS `~/Library/Application Support`; 1.4.6 blocks `~/.docker`, `~/.kube`, and `~/_netrc` unless explicitly granted. Version 1.1.10 changed repository `.git` access to read-only. These exclusions are not evidence that every credential file, environment secret, keychain operation, or inherited handle is confidential. The complete mount/cache/temporary-file lists and symlink-resolution/race protections are not published in the examined material. A workspace symlink must not be assumed to authorize its target. [Credential and Git protection changes](https://github.com/google-antigravity/antigravity-cli/blob/c994f07d241d3b3b1902d5705008092beb00e099/CHANGELOG.md).

Adding `--add-dir /path` expands the workspace; permissions can grant narrower external reads or writes. Built-in file tools use `read_file` and `write_file` application checks, with non-workspace access normally requiring approval. They are not established as running inside the terminal’s OS boundary. Plan mode prepends planning instructions and uses read-only investigation; it does not create process isolation or prevent the agent from reading authorized sensitive content. [CLI flags](https://antigravity.google/docs/cli/reference#command-line-flags), [permission resources](https://antigravity.google/docs/permissions?tab=cli), [plan mode](https://antigravity.google/docs/cli/modes#analyze-tasks-before-editing-with-plan-mode).

### Network and tool coverage

Sandboxed commands have no network access by default; `read_url(domain)` grants authorize domains and their subdomains. `sandboxProxy.http` can replace the built-in HTTP proxy; the replacement must enforce the `read_url` rules itself. This policy concerns tool traffic. It does not disable the harness’s model/API connection or establish an egress firewall around all CLI activity. Google sign-in uses the platform keyring, and `modelProvider: "gemini"` with `GEMINI_API_KEY` can select direct Gemini API transport. [Proxy configuration](https://antigravity.google/docs/settings?tab=cli#sandboxproxy), [authentication](https://antigravity.google/docs/install), [URL matching](https://antigravity.google/docs/permissions?tab=cli).

Subagents inherit parent sandbox settings, directory scopes, and terminal grants; permission requests bubble to the parent. Their branched Git worktrees separate working copies, not OS privileges. Release 1.4.3 aligned subagent Auto-mode host-command evaluation with the parent. [Subagent inheritance](https://antigravity.google/docs/subagents#permissions-and-configuration-inheritance), [release history](https://github.com/google-antigravity/antigravity-cli/blob/c994f07d241d3b3b1902d5705008092beb00e099/CHANGELOG.md#143).

MCP has its own `mcp(server/tool)` approvals, defaulting to Ask, and supports both local stdio executables and remote HTTP servers. Authorizing an MCP tool does not establish that its server is in the terminal sandbox or that its internal disk/network operations obey shell rules. Hooks run configured commands and can approve, deny, or alter tool gating; plugins can bring hooks, MCP servers, and sidecars. Sidecars launch executables from their own directories. The examined documentation does **not** establish OS containment of these helper processes. Their host access remains an unresolved coverage issue, rather than a verified assertion that they are isolated or unrestricted. [MCP](https://antigravity.google/docs/mcp?tab=cli), [hooks](https://antigravity.google/docs/hooks?tab=cli), [sidecars](https://antigravity.google/docs/sidecars), [customization files](https://antigravity.google/docs/settings?tab=cli#other-configuration-files).

## Changing permissions and handling failures

Persistent CLI preferences live at `~/.gemini/antigravity-cli/settings.json`. `/config` saves preferences immediately; command-line overrides apply to the current session and remain enforced even if the saved preference changes. Local 1.4.10 also advertises `--settings` accepting an inline JSON object or file path, for session-only settings. Its precedence relative to overlapping dedicated flags is unverified. `/permissions` exposes Project, Shared, and Global scopes; global rules come from `settings.json` and are combined with the other scopes. The documented evaluation order is **Deny > Ask > Allow**, then mode/policy defaults. [Settings lifecycle](https://antigravity.google/docs/settings?tab=cli#how-the-cli-reads-and-saves-settingsjson), [permission scope manager](https://antigravity.google/docs/cli/commands/permissions).

For example, these settings enable local containment and permit sandboxed commands automatically without selecting Auto’s automatic host-command review:

```json
{
  "enableTerminalSandbox": true,
  "toolPermission": "proceed-in-sandbox",
  "permissions": {
    "allow": ["read_file(/opt/reference)", "read_url(registry.npmjs.org)"],
    "deny": ["read_file(.env)", "write_file(.git/)"]
  }
}
```

```sh
agy --sandbox
agy --sandbox -p "Run the workspace test suite and report blocked operations"
agy --mode=auto
agy --sandbox --add-dir /absolute/path/to/another/project
```

The second directory becomes part of the writable workspace; use a `read_file` grant instead when only reference access is needed. `write_file` implies read permission; denying reads also denies writes. Commands use literal token-prefix matching or `regex:`; complex or unparseable shell constructs require full-line matching. An allowlist authorizes execution, rather than proving the command is safe. [Permissions syntax and examples](https://antigravity.google/docs/permissions?tab=cli#supported-cli-actions-and-matching-rules).

A denied permission blocks the action. A sandbox filesystem/network failure can lead the agent to request an unsandboxed retry. Outside Auto mode this requires approval unless a matching command grant exists. For personal accounts, `command(git push)` can permit execution **inside or outside** the sandbox; business/enterprise accounts instead use `unsandboxed(git push)` to grant bypass. Personal accounts ignore `unsandboxed` rules with a warning. Approvals can be one-time, conversation-wide, or persisted. This is an intentional route to host execution, not an escape exploit. [CLI bypass handling](https://antigravity.google/docs/sandbox?tab=cli#cli-permissions-integration).

`--dangerously-skip-permissions` auto-approves tool requests and selects `always-proceed`; the settings reference says explicit Ask/Deny rules still apply to that policy, while the headless guide describes the flag broadly as approving all tools. Neither proves that the flag disables an explicitly enabled sandbox. It does remove the ordinary approval hurdle for a requested host retry, so pairing it with `--sandbox` does not guarantee confinement. Exact interactions with Auto activation and explicit rules need runtime confirmation; no `--yolo` alias is advertised by local help. [Headless bypass flag](https://antigravity.google/docs/cli/headless#permissions-in-headless-mode), [policy overrides](https://antigravity.google/docs/permissions?tab=cli#execution-modes-and-the-terminal-sandbox).

**Unavailable isolation is not fail-closed.** Starting in 1.4.4, `--sandbox` and `enableTerminalSandbox` have no effect if the backend is unavailable; `proceed-in-sandbox` becomes `request-review`, and Auto is hidden. Examples include older Windows builds and Linux/container policies blocking unprivileged user namespaces. A headless request for unavailable Auto warns and continues in default mode. Saved settings remain intact for a capable host. The documents do not specify the response to every mid-session sandbox setup failure or malformed headless configuration. [Unavailable sandbox behavior](https://antigravity.google/docs/sandbox?tab=cli#if-the-sandbox-isnt-available), [mode fallback](https://antigravity.google/docs/cli/modes#requirements-and-limitations).

No dedicated sandbox environment variable or administrator-locked local sandbox policy was verified in the examined CLI references. Enterprise IAM, region selection, model enablement, and VPC Service Controls govern service access; they do not establish local shell filesystem confinement. Enterprise setup requires an eligible license/project and administrator enablement. [Enterprise administration](https://antigravity.google/docs/enterprise#administrator-setup-guide).

## Official remote alternative

Current documentation supports `agy --remote-agent --project="My remote project"`: conversations and tools run in a Google-managed isolated remote environment, without access to the local launch directory. Code must be attached as remote resources. This contains agent execution remotely rather than only local shell tools. Remote Control alone continues executing on the connected workstation. Remote Agents requires a personal Google AI Pro or Ultra account and gradual feature availability; it is unavailable for free, Plus, and business accounts. Docs specify CLI 1.4.12+, while the repository first lists the flag’s help exposure under 1.5.0; local 1.4.10 help lacks it. [Remote Agents CLI integration](https://antigravity.google/docs/remote-agents?tab=cli).

Remote permissions replace local sandbox/mode settings and default to `always-proceed (turbo)`; the remote machine’s isolation technology and complete network/credential boundaries are not disclosed. Remote sessions currently do not support print mode (`--print`). Do not infer Docker or VM technology from the phrase “isolated environment.” [Remote settings](https://antigravity.google/docs/remote-agents#remote-agent-settings).

## Evidence and remaining checks

Frontmatter evidence records distinguish **documented**, **repository-inspected**, **locally observed**, and **inferred** findings. The public repository tree contains README, changelog, issue templates, and customization examples, but **no CLI implementation source** at the examined commit. Consequently this report makes no source-audited enforcement claim. Local inspection used `sniff software agents --json`, `agy --version`, and `agy --help`; user settings and credentials were not read or changed. [Examined repository tree](https://github.com/google-antigravity/antigravity-cli/tree/c994f07d241d3b3b1902d5705008092beb00e099).

Specific unresolved checks are: run bounded filesystem/network probes on each backend (including WSL2 and native Windows separately); test symlink targets and inherited environment/handles; trace MCP, hook, plugin sidecar, and file-tool execution; test `--sandbox` with permission bypass and Auto; test overlapping `--settings`/dedicated flags; and verify Remote Agents rollout/version availability. These tests should use disposable fixtures and nonsecret markers. Missing public implementation is a limitation of verification, not evidence of missing sandbox functionality.

### Local observation record

Sanitized read-only observations on macOS, November 4, 2026 UTC (`local-help`):

```text
$ agy --version
1.4.10

$ agy --help
  --sandbox                       Run in a sandbox with terminal restrictions enabled
  --dangerously-skip-permissions  Auto-approve all tool permission requests without prompting
  --mode                          Set the agent execution mode for this session (accept-edits, auto, plan)
  --settings                      Settings for this session only, never saved: a JSON object of settings.json keys, inline or as a file path
  --add-dir                       Add a directory to the workspace (repeatable) (default [])
```

The full help output contained no `--remote-agent` or `--yolo` flag. Sniff independently identified the installed `agy` as version 1.4.10. These observations establish the installed interface, not clean-install preferences or functioning OS enforcement.

## Changelog

- **2026-11-04:** Initial report separates CLI defaults from App presets, documents optional native and remote isolation, account/version requirements, credential hardening, approval bypass, and unavailable-backend fallback. Records untested coverage explicitly.
