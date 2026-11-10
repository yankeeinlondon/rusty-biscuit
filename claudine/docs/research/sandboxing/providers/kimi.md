---
$schema: ./_schema.yaml
schema_revision: 5
provider: kimi
created: '2026-11-04'
last_updated: '2026-11-04'
content_policy: [ValidFor(14d)]
agent: codex
model: gpt-6.1-sol
versions_examined: ['2.5.0', '2.1.1', '1.52.0']
evidence:
  - id: e1
    method: official_docs
    location: https://github.com/MoonshotAI/kimi-cli/blob/9ab1286b8fe4e6bcd116949a27ce5e0ac3389c82/README.md
    version: '1.52.0'
    observed_on: '2026-11-04'
    claim: The archived Python Kimi CLI directs users to the replacement MoonshotAI/kimi-code product.
    limitations: Legacy identity evidence; not evidence of current sandbox support.
  - id: e2
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/docs/en/release-notes/changelog.md
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Remote environments ship in 2.5.0; earlier releases changed symlink defenses, confirmation guards, and plan review behavior.
    limitations: Release history is not runtime verification of the stated defenses.
  - id: e3
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/environment/localEnvironment.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Local execution binds directly to the host filesystem and process services rather than an OS sandbox.
    limitations: Corroborated by Node host backends linked in prose; no adversarial runtime testing.
  - id: e3-process
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/os/backends/node-local/hostProcessService.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Local children use ordinary Node spawn and inherit host environment variables plus overrides.
    limitations: Source analysis; external OS account restrictions or launch wrappers can change effective access.
  - id: e4
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/permissionPolicy/permissionPolicyService.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Permission policies check configured deny before auto approval; session approvals and ask/allow rules precede YOLO and default approvals.
    limitations: Docs describe first-match precedence more simply; no overlapping-rule runtime test performed.
  - id: e4-git
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/permissionPolicy/policies/git-cwd-write-approve.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Write and Edit can be automatically approved in POSIX Git workspaces, while this policy does not approve win32 paths.
    limitations: Earlier policies can ask or deny; this is not a complete mode specification or runtime test.
  - id: e5
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/workspace/workspaceTrust/workspaceTrustStore.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Workspace trust is a project-configuration gate with persistent trust, per-process override, and configurable requirement.
    limitations: TUI selection and print-mode handling corroborated by separate source links in prose; not isolation.
  - id: e6
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/tool/path-access.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Default file-tool policy permits absolute outside paths, rejects relative outside paths, and blocks sensitive filename patterns using lexical path handling.
    limitations: Call-site coverage and separate UI realpath checks are linked in prose; symlink behavior has not been tested at runtime.
  - id: e7
    method: official_docs
    location: https://moonshotai.github.io/kimi-code/en/reference/kimi-command.html
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Prompt mode forces auto approval; approval flags, add-dir, plan, and hidden remote-environment flags have distinct invocation scopes.
    limitations: Current docs differ from older releases; corroborated by source and limited 2.1.1 local help inspection.
  - id: e8
    method: official_docs
    location: https://moonshotai.github.io/kimi-code/en/guides/remote-environment.html
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Optional remote environments move filesystem/process tools to SSH, Docker, or command targets while retaining the controller and several tool surfaces locally, with no silent local fallback.
    limitations: Target isolation depends on provisioning; not a container or SSH runtime test.
  - id: e8-launcher
    method: source_code
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/remote/client/launchers.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Docker uses exec into an existing container without setting container security policy; SSH uses the system client and explicit authentication/host-key options.
    limitations: Does not establish security of any user-provisioned container, VM, or remote account.
  - id: e9
    method: official_docs
    location: https://moonshotai.github.io/kimi-code/en/customization/mcp.html
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Stdio MCP servers remain local unless their own environment_id explicitly names a remote environment, independently of the session binding.
    limitations: External MCP service authority is not controlled by the agent's built-in file-tool path policy.
  - id: e9-hooks
    method: official_docs
    location: https://moonshotai.github.io/kimi-code/en/customization/hooks.html
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Hooks execute locally even for remote sessions and fail open on errors/timeouts rather than supplying dependable isolation.
    limitations: Plugin hook working directories and resource validation are documented separately in the plugin guide linked in prose.
  - id: e10
    method: official_docs
    location: https://moonshotai.github.io/kimi-code/en/configuration/config-files.html
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Permission rules, global tool selection, trust, and environment declarations are separate user configuration controls; no managed OS-sandbox policy is specified.
    limitations: Negative sandbox finding additionally relies on host source; documentation's first-match rule claim conflicts with source ordering.
  - id: e10-env
    method: official_docs
    location: https://moonshotai.github.io/kimi-code/en/configuration/env-vars.html
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Proxy, trust, data-root, and shell-path variables configure clients and host behavior rather than enforcing a tool-network sandbox.
    limitations: Arbitrary shell descendants need not honor client proxy settings.
  - id: e11
    method: official_docs
    location: https://moonshotai.github.io/kimi-code/en/guides/getting-started.html
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: Native Windows is supported with Git for Windows/Git Bash; agent operation requires configuring model authentication.
    limitations: No native Windows runtime validation was performed.
  - id: e11-wsl
    method: inference
    location: https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/environment/localEnvironment.ts
    version: '2.5.0'
    observed_on: '2026-11-04'
    claim: WSL2 uses the Linux execution path and receives no additional Kimi confinement beyond the distribution's existing environment.
    limitations: Inference supported by direct local bindings and Linux installation documentation, not a WSL2 runtime observation.
  - id: e12
    method: local_inspection
    location: claudine/docs/research/sandboxing/providers/kimi.md#e12-local-observation
    version: '2.1.1'
    observed_on: '2026-11-04'
    claim: Local macOS kimi reports 2.1.1 and its help exposes approval, prompt, plan, and add-dir flags without a sandbox option.
    limitations: Metadata/help only; no model invocation, configuration inspection, or containment test; predates 2.5.0 remote execution.
summary: No default OS sandbox on any examined platform; interactive sessions use manual approval and trust, while print mode forces auto approval. Version 2.5.0 optionally relocates file/process tools into existing containers or remote hosts, leaving the agent, credentials, hooks, web tools, and default MCP execution local.
support: official_integration
default_profiles:
  - id: macos-interactive
    environment: macos
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: Clean 2.5.0 kimi uses local host authority, manual approval, and an initial trust-or-exit choice; ordinary reads and eligible POSIX Git-workspace writes auto-approve. Model authentication is required; home and network are not isolated.
    evidence_ids: [e3, e3-process, e4, e4-git, e5, e7, e11]
  - id: macos-non-interactive
    environment: macos
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: Clean 2.5.0 kimi -p forces auto without a trust dialog; configured denies and file-tool sensitive-name blocks survive; untrusted project MCP is skipped. Host authority remains and model credentials must already be configured.
    evidence_ids: [e3, e3-process, e4, e5, e6, e7, e11]
  - id: linux-interactive
    environment: linux
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: Local Linux authority with manual approval and initial trust-or-exit choice; ordinary reads and eligible POSIX Git-workspace writes auto-approve. Model authentication is required; no additional home or network boundary.
    evidence_ids: [e3, e3-process, e4, e4-git, e5, e7, e11]
  - id: linux-non-interactive
    environment: linux
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: kimi -p forces auto, skips the trust dialog and untrusted project MCP, and retains configured deny and file-tool path checks; host filesystem/process/network authority persists. Requires preconfigured model access.
    evidence_ids: [e3, e3-process, e4, e5, e6, e7, e11]
  - id: windows-interactive
    environment: windows
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: Native Windows installation requires Git Bash; local user authority, manual approval, and trust-or-exit startup apply. The POSIX Git-write automatic approval does not apply. Model authentication is required.
    evidence_ids: [e3, e3-process, e4, e4-git, e5, e7, e11]
  - id: windows-non-interactive
    environment: windows
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: Native Windows kimi -p uses auto without trust prompting and needs Git Bash for shell tools; project MCP remains trust-gated, configured deny and sensitive-name file checks remain, and no OS sandbox is added. Requires model access.
    evidence_ids: [e3, e3-process, e4, e5, e6, e7, e11]
  - id: wsl2-interactive
    environment: wsl2
    launch_mode: interactive
    status: disabled
    mechanism_ids: []
    notes: Linux CLI inside WSL2 uses manual approval and initial trust choice; distribution mounts/interoperability determine reachable Windows resources. Kimi adds no sandbox; eligible POSIX Git writes auto-approve. Requires model access; runtime behavior is untested.
    evidence_ids: [e3, e3-process, e4, e4-git, e5, e7, e11, e11-wsl]
  - id: wsl2-non-interactive
    environment: wsl2
    launch_mode: non_interactive
    status: disabled
    mechanism_ids: []
    notes: Linux print path forces auto within the WSL2 distribution, without a trust dialog or extra confinement; configured deny/file-name guards survive and untrusted project MCP is skipped. Requires model access; WSL2 mounts remain reachable subject to OS policy.
    evidence_ids: [e3, e3-process, e4, e5, e6, e7, e11, e11-wsl]
mechanisms:
  - id: docker-environment
    name: Remote environments (Docker-compatible containers)
    enforcement: container
    environments: [macos, linux, windows, wsl2]
    backend: System Docker-compatible CLI executes kimi exec-server inside an existing POSIX container; executor protocol minimum 2.5.0. This lists controller environments, not support for Windows containers.
    prerequisites:
      - Kimi Code CLI 2.5.0 controller and compatible executor installed or mounted on target.
      - Running container with usable sh, project files, and permission to use its Docker-compatible daemon/context.
      - Container mounts, identity, privileges, networking, and any supporting VM provisioned externally.
    activation: Opt in via an environments entry with type = docker, selected by --environment or environments.default; bindings persist per session. Clean default is local.
    filesystem: Ordinary file tools read/write the container filesystem with tool-specific path checks; absolute outside-workspace paths are allowed. Mounted homes, secrets, and temporary files are not hidden by Kimi. Agent-tool symlink confinement is unverified; local plans/output/attachments are exceptions.
    network: Container networking is externally configured; Kimi sets no domain, DNS, ingress, or egress policy. Web tools and model/API traffic stay on the controller.
    processes: Executor and Bash descendants run under the container's configured identity and environment. The agent loop, controller credentials, and session persistence remain on the host; local hooks and default MCP keep host access.
    coverage: File/process tools and ordinary subagents' bindings target the container; hooks, web tools, and default stdio MCP remain local. Each stdio MCP server can separately bind via environment_id. Tower is unsupported remotely.
    escape_hatches: YOLO/auto only change approval policy and do not remove the target boundary. Explicitly selecting local for a new session changes placement; no automatic unsandboxed retry. Broad mounts or privileges can weaken external containment.
    failure_behavior: fail_closed
    failure_notes: Missing/incompatible executor, stopped container, bad declaration, or connection failure prevents creation/resume or fails calls. Disconnects terminate remote work; in-flight calls are not replayed. Later calls reconnect to the same target without local fallback, including print mode.
    evidence_ids: [e2, e6, e7, e8, e8-launcher, e9, e9-hooks]
  - id: remote-environment
    name: Remote environments (SSH and custom command launchers)
    enforcement: remote_environment
    environments: [macos, linux, windows, wsl2]
    backend: System SSH or an operator-supplied command starts kimi exec-server over stdio on the chosen target; minimum executor 2.5.0. SSH relocates account authority; a custom launcher supplies only the isolation its target actually has.
    prerequisites:
      - Kimi Code CLI 2.5.0 controller and compatible target executor; installation is manual.
      - System SSH authentication/host access or a separately provisioned custom target and launcher.
      - Git Bash and process-lifecycle capability for native Windows targets.
      - OpenSSH 8.4+ only for interactive password Askpass fallback; headless needs unattended authentication.
    activation: Opt in with environments entries using type = ssh or mutually exclusive command/args, selected at session creation; no default remote account or managed sandbox entitlement is supplied.
    filesystem: Target-account readable/writable paths remain available with file-tool checks; defaultCwd is not a jail. Target home, credentials, and temporary files follow target permissions. Local plans/output/attachments remain accessible exceptions; no proven symlink confinement for agent tools.
    network: Target process networking follows target policy; SSH encrypts the connection but adds no domain firewall. Model/API and web tools stay local. Remote MCP inherits target environment plus literal env overlays.
    processes: Target executor and descendants execute as the target account; the controller retains model credentials and agent state. Custom launcher environment overlays apply locally to the launcher, not automatically to target commands.
    coverage: Selected file/process tools and ordinary subagents inherit the binding; hooks/web tools/default MCP remain local. Stdio MCP requires its own environment_id. HTTP MCP service authority is separate; remote Tower unsupported.
    escape_hatches: No silent local fallback or automatic unsandboxed retry. Explicit local selection is creation-only; approval bypasses do not alter the binding. A custom command running on the controller provides no intrinsic isolation.
    failure_behavior: fail_closed
    failure_notes: Invalid target or missing/incompatible executor fails creation/resume. Disconnections fail in-flight calls without replay; subsequent calls can reconnect to the same environment. Headless SSH uses BatchMode and has no password callback; interactive password fallback is conditional. Changed host keys fail.
    evidence_ids: [e2, e6, e7, e8, e8-launcher, e9, e9-hooks]
controls:
  - id: environment-select
    kind: cli_flag
    name: --environment
    values: [configured environment ID, local]
    effect: Selects the execution environment for a new 2.5.0 session, including print mode; selecting local deliberately returns file/process execution to host authority.
    example: kimi --environment kimi-dev -p "Run project checks"
    precedence: Overrides environments.default for creation; rejects resume/continue; restored sessions retain saved binding and later /new keeps the live binding. Hidden from normal help.
    mechanism_ids: [docker-environment, remote-environment]
    evidence_ids: [e7, e8]
  - id: environment-cwd
    kind: cli_flag
    name: --environment-cwd
    values: [absolute target directory]
    effect: Sets the new environment working directory without making it a read/write jail.
    example: kimi --environment kimi-dev --environment-cwd /workspace
    precedence: Requires --environment and overrides entry defaultCwd; creation-only, invalid target directories fail.
    mechanism_ids: [docker-environment, remote-environment]
    evidence_ids: [e7, e8]
  - id: environment-declarations
    kind: config_key
    name: environments
    values: [named SSH entries, named Docker entries, named command launcher entries]
    effect: Declares target connection, executor, and working directory; does not create or harden a container/VM. type and command are mutually exclusive.
    example: "[environments.kimi-dev]\ntype = \"docker\"\ncontainer = \"kimi-dev\"\nremoteBin = \"/usr/local/bin/kimi\"\ndefaultCwd = \"/workspace\""
    precedence: User config under KIMI_CODE_HOME; editing launcher disconnects affected work; removing entry does not rebind to local. Entry selection comes from creation flag, configured default, or persisted binding.
    mechanism_ids: [docker-environment, remote-environment]
    evidence_ids: [e8, e8-launcher, e10]
  - id: environment-default
    kind: config_key
    name: environments.default
    values: [configured environment ID]
    effect: Selects the remote/container environment for new sessions without explicit selection; referenced entry requires defaultCwd.
    example: "[environments]\ndefault = \"kimi-dev\""
    precedence: --environment overrides; absent means local, invalid configuration fails creation rather than quietly ignoring the default; /new retains current binding.
    mechanism_ids: [docker-environment, remote-environment]
    evidence_ids: [e8, e10]
  - id: mcp-environment
    kind: config_key
    name: mcpServers.<server>.environment_id
    values: [configured environment ID, local]
    effect: Binds that stdio MCP server to its own target; omitted defaults to local even during a remote session.
    example: '{"mcpServers":{"project-files":{"command":"/usr/local/bin/project-mcp","cwd":"/workspace","environment_id":"kimi-dev"}}}'
    precedence: Per-server local MCP configuration; never inherits the session binding; target environment plus explicit env overlay determines server environment.
    mechanism_ids: [docker-environment, remote-environment]
    evidence_ids: [e8, e9]
  - id: default-permission-mode
    kind: config_key
    name: default_permission_mode
    values: [manual, yolo, auto]
    effect: Adjacent approval policy only; manual is default and no value creates containment.
    example: 'default_permission_mode = "manual"'
    precedence: Default for new sessions; explicit mode flag overrides and resume keeps saved mode unless overridden; print mode forces auto.
    mechanism_ids: []
    evidence_ids: [e4, e7, e10]
  - id: yolo
    kind: cli_flag
    name: --yolo
    values: []
    effect: Relaxes routine approvals to Ask When Needed; sensitive/dangerous confirmation policies remain unless disabled. Does not change environment placement or OS enforcement.
    example: 'kimi --yolo # Relaxes approvals'
    precedence: -y, --yes and --auto-approve are aliases; overrides initial/saved permission; mutually exclusive with --auto and --prompt.
    mechanism_ids: []
    evidence_ids: [e4, e7]
  - id: auto
    kind: cli_flag
    name: --auto
    values: []
    effect: Relaxes approvals to Never Ask, including sensitive confirmations; configured deny and file-tool hard checks remain. Current plan review remains mandatory; no environment change.
    example: 'kimi --auto # Relaxes approvals'
    precedence: Overrides initial/saved mode, after configured deny in policy order; mutually exclusive with --yolo and --prompt.
    mechanism_ids: []
    evidence_ids: [e4, e6, e7]
  - id: prompt
    kind: cli_flag
    name: --prompt
    values: [prompt string]
    effect: Starts a non-interactive session in forced auto permission; does not enable isolation, and skips default plan mode for new sessions.
    example: kimi -p "Summarize the repository"
    precedence: -p alias; rejects --auto, --yolo, and --plan; configured denies remain, trust gate still skips untrusted project MCP, and resumed plans cannot be approved.
    mechanism_ids: []
    evidence_ids: [e4, e5, e7]
  - id: permission-session
    kind: session_command
    name: /permission
    values: [manual, yolo, auto]
    effect: Selects the live approval mode through the mode list; /yolo and /auto select the corresponding modes. No OS boundary change.
    example: /permission
    precedence: Live session change, with current subagents following session permission; does not change environment binding or configured deny.
    mechanism_ids: []
    evidence_ids: [e2, e4, e7]
  - id: permission-rules
    kind: config_key
    name: permission.rules
    values: [allow, deny, ask]
    effect: Tool-name/argument-pattern approval records; not an OS policy. Scope labels include turn-override, session-runtime, project, user.
    example: "[[permission.rules]]\ndecision = \"deny\"\npattern = \"Bash(rm -rf*)\""
    precedence: Source checks deny before auto, then session approvals, ask and allow before remaining approvals; documentation's first-match claim is unresolved. Auto bypasses configured ask; deny survives.
    mechanism_ids: []
    evidence_ids: [e4, e10]
  - id: confirmation-guards
    kind: config_key
    name: permission.dangerous_command_guard
    values: ['true', 'false']
    effect: Default true; false relaxes all built-in dangerous/sensitive/git confirmation guards since 2.3.0, retaining file-tool hard sensitive-name checks. No isolation change.
    example: "[permission]\ndangerous_command_guard = false # Relaxes confirmations"
    precedence: Overridden by KIMI_CODE_DANGEROUS_COMMAND_GUARD; auto already precedes these confirmation policies.
    mechanism_ids: []
    evidence_ids: [e2, e4, e6, e10]
  - id: confirmation-guards-env
    kind: environment_variable
    name: KIMI_CODE_DANGEROUS_COMMAND_GUARD
    values: ['true', 'false']
    effect: Exact lowercase false disables built-in confirmation guards for that process without changing file-tool hard checks or target isolation.
    example: 'KIMI_CODE_DANGEROUS_COMMAND_GUARD=false kimi # Relaxes confirmations'
    precedence: Overrides the user config guard boolean; source parser accepts the exact strings true/false.
    mechanism_ids: []
    evidence_ids: [e4, e10]
  - id: trust-requirement
    kind: config_key
    name: workspace.require_trust
    values: ['true', 'false']
    effect: Default true; false trusts all workspaces and enables trust-gated project configuration without prompting. Does not change tool approvals or confinement.
    example: "[workspace]\nrequire_trust = false # Relaxes project trust gating"
    precedence: 2.5.0 user setting, applied on /reload; falsy per-process trust variable cannot restore the gate when this is false.
    mechanism_ids: []
    evidence_ids: [e2, e5, e10]
  - id: trust-env
    kind: environment_variable
    name: KIMI_CODE_TRUST_WORKSPACE
    values: ['1', 'true', 'yes', 'on', '0', 'false', 'no', 'off']
    effect: Truthy value trusts the current workspace for that process without persisting trust; project configuration can then load. Does not change tool permissions.
    example: 'KIMI_CODE_TRUST_WORKSPACE=1 kimi # Relaxes trust gating'
    precedence: Independent per-process trust grant; false does not negate stored trust or require_trust = false.
    mechanism_ids: []
    evidence_ids: [e5, e10-env]
  - id: global-tools-enabled
    kind: config_key
    name: tools.enabled
    values: [array of tool names]
    effect: Nonempty global tool allowlist across agents, checked at execution; not OS isolation or a network firewall.
    example: "[tools]\nenabled = [\"Read\", \"Grep\", \"Glob\"]"
    precedence: Empty or absent imposes no constraint; followed by tools.disabled and intersected with agent tools/disallowedTools.
    mechanism_ids: []
    evidence_ids: [e10]
  - id: global-tools-disabled
    kind: config_key
    name: tools.disabled
    values: [array of tool names]
    effect: Disables listed tools across agents; disabling web tools does not disable shell or model networking and adds no OS boundary.
    example: "[tools]\ndisabled = [\"Bash\"]"
    precedence: Applied after tools.enabled, intersected with agent tools/disallowedTools and checked before execution.
    mechanism_ids: []
    evidence_ids: [e10]
  - id: additional-directory
    kind: cli_flag
    name: --add-dir
    values: [directory path, repeatable]
    effect: Adds workspace path context for the session; does not create a jail, since absolute outside paths are already accepted.
    example: kimi --add-dir /work/shared
    precedence: Relative paths resolve against startup cwd; /add-dir can remember directories; legacy project additional_dir merges only for implicit workspaces.
    mechanism_ids: []
    evidence_ids: [e5, e6, e7, e10]
  - id: plan
    kind: cli_flag
    name: --plan
    values: []
    effect: Workflow policy restricts selected mutating tools while allowing plan-file writes and regular Bash policies; does not enforce a read-only filesystem/network boundary.
    example: kimi --plan
    precedence: Cannot combine with --prompt; current plan exit review required even in auto since 2.4.0; /plan toggles mode interactively.
    mechanism_ids: []
    evidence_ids: [e2, e7]
  - id: data-root
    kind: environment_variable
    name: KIMI_CODE_HOME
    values: [data directory path]
    effect: Relocates configuration, credentials, and session data, without hiding the real home or generic .agents resources.
    example: KIMI_CODE_HOME=/work/kimi-data kimi
    precedence: Overrides default ~/.kimi-code data root for the process; does not override remote target home or create isolation.
    mechanism_ids: []
    evidence_ids: [e10-env, e11]
gaps:
  - location: default_profiles
    detail: Platform defaults are documentation/source conclusions; local observation covers only macOS 2.1.1 metadata, and WSL2 inherits Linux behavior by inference.
    next_check: Exercise clean 2.5.0 accounts with a deterministic stub model on macOS, Linux, native Windows, and WSL2, verifying approvals, trust startup, and host access.
  - location: mechanisms/docker-environment/filesystem
    detail: Release notes claim earlier symlink hardening but current agent tool resolution is lexical; UI/API realpath protection does not establish agent-tool confinement.
    next_check: Use harmless external symlinks and fixture secrets to compare Read/Write/Edit/Grep and UI/API access on pinned 2.5.0, including Windows links and local/remote targets.
  - location: controls/permission-rules/precedence
    detail: Documentation says first matching rule wins but source evaluates separate decision classes, including deny before auto and ask/allow later.
    next_check: Run overlapping allow/deny/ask fixtures with a stub model in manual/auto modes and request upstream clarification of intended ordering.
  - location: mechanisms
    detail: No 2.5.0 remote environment was executed; source/docs establish placement and fail-closed behavior but not security of any concrete target.
    next_check: Provision minimal container and Windows SSH executors and test tool/MCP placement, local hooks/web transport, missing executor, and disconnects from native and WSL2 controllers.
changes: []
---

# Kimi Code CLI: sandboxing and access boundaries

Researched **2026-11-04 UTC**. A normal Kimi Code CLI installation does **not** put the agent or its shell in an OS sandbox. It runs with the launching user's filesystem, process, and network authority. Its default protections are application-level tool checks, approval policies, and workspace trust. Starting with **2.5.0**, officially supported remote environments can move file and process operations into an existing container or onto another machine; they leave important execution surfaces on the controller. This conclusion comes from the local environment's direct host filesystem/process bindings, the Node backends, and the remote launcher implementation, rather than from silence in the documentation. [E3](#e3-host-execution) [E8](#e8-remote-environments)

## Scope and versions

The requested [documentation entry point](https://moonshotai.github.io/kimi-cli/) and [MoonshotAI/kimi-cli repository](https://github.com/MoonshotAI/kimi-cli) now lead to a migration distinction: the Python repository was archived on September 23, 2026, and identifies **MoonshotAI/kimi-code** as its replacement. This report describes the current TypeScript Kimi Code CLI, release **2.5.0**, published November 4, 2026, and source commit **`f8e15c7a9be44fe4e0361a4ce57eaf690e05476a`**, dated November 4. The archived repository's final release is **1.52.0**, and its examined commit is **`9ab1286b8fe4e6bcd116949a27ce5e0ac3389c82`**. Legacy Kaos APIs are not evidence that current CLI flags enable a sandbox. In fact, 2.2.0 removed the never-implemented MCP `executor = "kaos"` option; configurations retaining it fail validation. [E1](#e1-product-identity) [E2](#e2-release-history)

Read-only local inspection on macOS found `/Users/ken/.kimi-code/bin/kimi`, reporting **2.1.1**. `kimi --help` corroborated `--yolo`, `--auto`, `--prompt`, `--plan`, and `--add-dir`; its help did not advertise a sandbox flag. This installation predates remote environments. No model session, permission bypass, container launch, credential inspection, or access probe was run. Source inspection below establishes implementation behavior, not runtime security-test results. [E12](#e12-local-observation)

## Clean-install defaults

The table concerns **2.5.0**, with no migrated configuration, remembered trust, extra directories, or administrator-supplied containment. Native Windows has its own supported installation and requires Git for Windows/Git Bash (`KIMI_SHELL_PATH` can specify an absolute `bash.exe` path); its support is established independently of WSL2. WSL2 runs the Linux CLI within the distribution and inherits whatever Windows mounts and interoperability that distribution exposes; Kimi adds no further WSL boundary. [E3](#e3-host-execution) [E11](#e11-installation-and-data)

| Platform | Interactive `kimi` | Non-interactive `kimi -p "…"` |
| --- | --- | --- |
| macOS | No sandbox; `manual` (Always Ask); initial workspace trust confirmation | No sandbox; forced `auto` (Never Ask); no trust dialog |
| Linux | Same default policy, on the Linux host | Same unattended policy, on the Linux host |
| Native Windows | No sandbox; same modes/trust gate; Bash uses Git Bash | No sandbox; forced `auto`; Git Bash required for shell execution |
| WSL2 | No Kimi sandbox; Linux behavior inside the distribution | No Kimi sandbox; Linux unattended behavior inside the distribution |

Always Ask does not mean every read or even every write prompts. Ordinary reads, searches, web tools, and subagent dispatch are automatically approved. The inspected policy also automatically approves `Write`/`Edit` within a Git workspace on **POSIX** targets after earlier rules/guards have been considered. That policy returns no automatic approval on native Windows. This is a source-derived qualification to the documentation's simpler description that edits always ask in manual mode. [E4](#e4-permission-evaluation)

On first interactive opening, the trust dialog offers **Trust and continue** (the default selection) or **Exit**. Declining exits before session creation; trusting enables project-controlled configuration, including project MCP servers. It does not constrain the shell's authority. Print mode has no trust prompt and skips untrusted project MCP servers with a warning; that does not prevent its built-in tools from operating on the project. Current trust settings default to `[workspace] require_trust = true`; `KIMI_CODE_TRUST_WORKSPACE=1` trusts the current workspace for that process without saving a record. In 2.5.0, `require_trust = false` treats every workspace as trusted. Neither switch changes tool permissions. [E5](#e5-workspace-trust)

The non-interactive column describes `-p`, not every stdio integration. `kimi acp` exposes session mode selection and routes tool approvals/questions through the ACP client’s `session/request_permission`; it does not inherit print mode’s unconditional auto policy merely because it uses stdio. [ACP reference](https://moonshotai.github.io/kimi-code/en/reference/kimi-acp.html)

A working model connection is needed for agent activity: first-run `/login` offers Kimi Code OAuth or a Moonshot API key, and compatible providers can be configured separately. There is no separate sandbox account or paid sandbox entitlement documented for remote environments; users supply the target, authentication, and executor. The recommended binary installation needs no preinstalled Node.js; the documentation lists Node.js 22.19+ for npm installation, while the current source workspace requires Node.js 24.15+ for development. [E11](#e11-installation-and-data) [E8](#e8-remote-environments)

## What local execution can access

**Files and directories.** `Read`, `Write`, and `Edit` resolve relative paths against the workspace and accept absolute paths outside it. Relative traversal outside the workspace is rejected with `PATH_OUTSIDE_WORKSPACE`, asking the caller to use an absolute path. Consequently, the workspace is not a read jail or a mandatory write jail. `--add-dir <dir>` adds workspace context for one session; `/add-dir` can remember directories in the workspace registry. Legacy `.kimi-code/local.toml` `workspace.additional_dir` entries still apply to implicit workspaces. These settings affect path handling and approvals, not OS authority. [E6](#e6-file-tool-boundaries) [E5](#e5-workspace-trust)

The default file-tool path policy blocks sensitive names before execution (`PATH_SENSITIVE`), including `.env`, `.env.*`, `id_rsa`, `id_ed25519`, `id_ecdsa`, `credentials`, and selected backup/key variants. `.env.example`, `.env.sample`, `.env.template`, and the explicitly listed public-key names are exempt. `Read`/`Write`/`Edit` retain this check even in `auto` mode and when confirmation guards are disabled. This is a filename heuristic, not a complete credential vault boundary: `Grep` and `Glob` explicitly use `checkSensitive: false`, and arbitrary shell programs do not inherit this file-tool check. Other accessible home files and credentials under unrecognized names remain exposed. Read-only tool access can disclose secrets just as shell output can. [E6](#e6-file-tool-boundaries) [E4](#e4-permission-evaluation)

**Symlinks and protected paths.** Release 2.1.0 claimed symlink-escape hardening and rejection of additional directories resolving to home or filesystem root; 2.1.1 explicitly rolled back some defensive changes. Current agent-tool path resolution is lexical, and the examined `Read`/`Write`/`Edit` paths call host filesystem operations without a realpath confinement check. The separate workspace filesystem service used by UI/API file operations does include a realpath-based workspace check. Do not transfer that UI/API protection to the model's file tools or assume every symlink escape is blocked. This discrepancy needs an isolated runtime regression check. There is no documented private temporary filesystem or universal protected-file list for Bash; ordinary OS permissions still apply. [E2](#e2-release-history) [E6](#e6-file-tool-boundaries)

**Processes and network.** Local process execution uses Node's `spawn`, with the host environment plus overrides. Shell children have the user's host access, including inherited environment secrets, available executables, temporary directories, and ordinary network connectivity. There is no supplied Seatbelt, Landlock, namespace, seccomp, AppContainer, or container wrapper in this local execution path. Command classification and dangerous-command confirmations are policy checks before launch, not a restriction applied to the resulting process. [E3](#e3-host-execution) [E4](#e4-permission-evaluation)

Kimi supplies no general tool-network deny switch or domain firewall in the examined configuration. Web tools, MCP connections, model transport, login, updates, and telemetry have distinct network consumers. Standard `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, and `NO_PROXY` influence supported outbound clients, with loopback bypass; a proxy setting is not an enforced egress policy for arbitrary Bash descendants. Disabling a web tool does not prevent Bash from making requests or prevent model/API transport. [E10](#e10-network-and-tool-selection) [E3](#e3-host-execution)

## Optional remote execution, including containers

**2.5.0 remote environments are tool placement, with isolation supplied by the chosen target.** `[environments]` supports SSH, Docker-compatible containers, and a custom command launcher. A small `kimi exec-server` executes filesystem and process primitives on that target. The agent loop, model requests, OAuth/API credentials, approvals, and session persistence remain local. A container contains the executor and its selected subprocesses, not the complete agent. SSH relocates authority to the target account; it is not itself a filesystem sandbox on that target. [E8](#e8-remote-environments)

```mermaid
flowchart LR
  subgraph controller[Local controller]
    A[Agent loop and model transport]
    H[Hooks and web tools]
    M[Default stdio MCP servers]
    S[Session data, plans, credentials]
  end
  subgraph target[Chosen target: existing container or remote host]
    X[kimi exec-server]
    F[Filesystem tools and Bash children]
    R[Explicitly bound stdio MCP servers]
    X --> F
    X --> R
  end
  A -->|filesystem and process RPC| X
  A --> H
  A --> M
  A --> S
```

For Docker, the launcher runs `docker exec -i <container> sh -c …`; it **does not create or harden the container**. The operator must provide a running container, Docker-compatible CLI/daemon access, a usable shell, the project files, and a compatible executor installed or mounted at `remoteBin`. Mounts, container user, privileges, capabilities, network, Docker socket exposure, and any VM supporting the runtime are properties of that existing environment. Kimi sets no fixed read/write mount policy or default network isolation. A mounted home or Docker socket can defeat the intended separation. This conclusion follows from the launcher argv; this report did not test a container escape. [E8](#e8-remote-environments)

SSH needs the system SSH client, target access, and an installed executor; it honors SSH configuration and agent authentication. The protocol minimum executor version is **2.5.0**, and Windows executors must advertise the required process-lifecycle capability. The executor is not automatically installed. Native Windows targets require Git Bash and support SSH/custom launchers, **not Windows Docker containers**. A Windows controller may still use a separately provisioned POSIX container target; that is a different claim from Windows-container support. Custom launchers can connect other officially described environments, but the CLI does not manufacture their security policy. [E8](#e8-remote-environments)

Example user configuration, after provisioning a container named `kimi-dev` and installing a compatible Linux executor there:

```toml
# ~/.kimi-code/config.toml
[environments.kimi-dev]
type = "docker"
container = "kimi-dev"
remoteBin = "/usr/local/bin/kimi"
defaultCwd = "/workspace"
```

```sh
# 2.5.0: these environment flags are documented but hidden from normal help.
kimi --environment kimi-dev --environment-cwd /workspace
kimi --environment kimi-dev -p "Run the project checks"
```

Other declaration fields are `type = "ssh"` with `host`, or `command` with `args` and optional launcher-only `env`; Docker accepts optional `context`. `remoteBin` defaults to `~/.kimi-code/bin/kimi` on POSIX targets or `~/.kimi-code/bin/kimi.exe` on Windows. A custom launcher normally receives `<remoteBin> exec-server` appended to its arguments; launchers without target-shell expansion need an absolute executor path.

`--environment` overrides `[environments] default` for creation. Without either, new sessions use `local`. `--environment-cwd` requires `--environment`; both reject resume/continue because resumed sessions restore the saved binding. The entry's `defaultCwd` must be absolute unless overridden. Bindings are per session; ordinary subagents inherit them, while Tower orchestration is unsupported remotely. `/new` retains the live environment rather than reapplying the startup selection. [E8](#e8-remote-environments)

Remote file tools can access files allowed by the target account and container mounts, not only `defaultCwd`. Target home/temporary files are not automatically hidden. Model credentials are not automatically sent there, but target-side credentials and secrets deliberately supplied through MCP `env` remain exposed to their processes. Local plans, task-output artifacts, and session attachments are explicit exceptions: Kimi can still read these locally during a remote session. `WebSearch`/`FetchURL` also remain local, so a container egress restriction on Bash does not restrict those tools or the model connection. [E8](#e8-remote-environments) [E9](#e9-hooks-plugins-and-mcp)

## Coverage across tools and integrations

| Surface | Default local session | Remote/container session |
| --- | --- | --- |
| Bash and background shell tasks | User's host processes | Target executor and target processes |
| Ordinary file tools and filesystem searches | Host filesystem through tool-specific checks | Target filesystem through the same tool policy |
| Ordinary subagents | Same host authority; separate conversation context | Inherit parent environment binding |
| Plans, retained task output, attachments | Local session storage | Local storage exceptions remain available |
| WebSearch, FetchURL, model/API requests | Local network clients | Still local |
| Lifecycle hooks, including plugin hooks | Local shell commands | Still local; user hooks use the local project directory, plugin hooks use the plugin root |
| Stdio MCP servers | Local child processes | Still local unless each server specifies `environment_id` |
| HTTP/SSE MCP servers | External service authority | Same service boundary; not moved into the session target |
| Skills and agent profiles | Instructions and tool selection | No independent OS isolation |

Hooks can block selected events with exit code 2, but errors/timeouts otherwise fail open; they are not a dependable sole security boundary. Plugins can supply hooks and MCP servers and therefore acquire those execution surfaces. Plugin hooks use the plugin root as their working directory. Plugin resource paths are checked after symlink resolution to stay within that root; this validates loaded resources rather than containing the hook process. Marketplace trust labels and installation confirmations identify provenance; they do not create confinement. MCP permission approval gates a call, not everything a server might do during startup or between calls. [E9](#e9-hooks-plugins-and-mcp)

To move a stdio MCP server into the target, set its environment explicitly in the **local** MCP configuration:

```json
{
  "mcpServers": {
    "project-files": {
      "command": "/usr/local/bin/project-mcp",
      "cwd": "/workspace",
      "environment_id": "kimi-dev"
    }
  }
}
```

This ID never implicitly follows the session binding. Such servers inherit the target's base environment, plus explicit `env` overlay values. Remote-side project MCP configuration is not loaded. [E9](#e9-hooks-plugins-and-mcp)

## Controls, precedence, and failure behavior

Approval selection is separate from environment selection. Interactive defaults come from `default_permission_mode = "manual"`; `--yolo`/`-y` selects `yolo` (Ask When Needed), `--auto` selects `auto` (Never Ask), and `/permission`, `/yolo`, or `/auto` changes the live session mode. `--yes` and `--auto-approve` are hidden YOLO aliases. Mode flags override saved mode when resuming; `-p` forces `auto` and rejects combinations with `--yolo`, `--auto`, or `--plan`. It does not silently enable a sandbox. YOLO/Auto do not change an existing remote binding or its container policy. [E4](#e4-permission-evaluation) [E7](#e7-command-controls)

The current policy source evaluates configured **deny** first, then the interactive dangerous-command guard, then auto-mode approval, session approvals, configured **ask**, configured **allow**, sensitive/git-path confirmations, YOLO, default tool approvals, POSIX Git-workspace writes, and finally a prompt. Thus static deny survives `auto`, but configured ask does not force a prompt in auto. Documentation says the first matching rule wins; the implementation instead searches separately by decision class. Use non-overlapping rules and treat the implementation ordering as the examined behavior pending upstream clarification. Scope values include `turn-override`, `session-runtime`, `project`, and `user`; these are permission-record scopes, not four independent administrator configuration files. [E4](#e4-permission-evaluation)

```toml
# ~/.kimi-code/config.toml (or $KIMI_CODE_HOME/config.toml)
default_permission_mode = "manual"

[permission]
dangerous_command_guard = true

[[permission.rules]]
decision = "deny"
pattern = "Bash(rm -rf*)"
reason = "Require a different workflow for recursive deletion"

[tools]
disabled = ["Bash"]
```

The Bash rule is command-pattern matching, not a semantic prohibition on deletion through other programs or file tools. `[tools] enabled` (nonempty allowlist) and `disabled` apply globally to all agents and intersect agent-profile `tools`/`disallowedTools`; availability is checked again before execution. They reduce exposed tools but impose no OS boundary. No immutable organization-managed sandbox policy, locked administrator override, sandbox escalation protocol, or sandbox-specific environment variable was found in the examined command/config schemas and host execution code. An administrator can constrain the OS account or provision the remote environment externally; that is separate enforcement. [E4](#e4-permission-evaluation) [E10](#e10-network-and-tool-selection)

`[permission] dangerous_command_guard = false`, overridden by the exact environment value `KIMI_CODE_DANGEROUS_COMMAND_GUARD=false`, disables built-in dangerous-command, sensitive-access, and git-control confirmation guards in current releases. This broader effect dates to 2.3.0. The file tools' sensitive-name hard rejection remains. `KIMI_CODE_HOME` relocates configuration, sessions, logs, and credentials; it does not hide the real home, and generic `~/.agents` resources still use the real OS home. [E2](#e2-release-history) [E6](#e6-file-tool-boundaries) [E11](#e11-installation-and-data)

Plan mode is workflow policy. It restricts selected mutating tools, allows writing the plan, and handles Bash through regular permission rules; it is not a read-only mount or a network boundary. Since 2.4.0, exiting a plan requires review even in Never Ask. Print mode skips `default_plan_mode` for new sessions and cannot approve a plan on a resumed plan-mode session. These details should not be projected backward onto the observed 2.1.1 installation. [E2](#e2-release-history) [E7](#e7-command-controls)

Denied rules/tool checks return errors to the agent; ordinary filesystem/process failures remain host or target failures. No OS-sandbox denial can trigger an unsandboxed retry because no such local backend is selected. For remote environments, invalid declaration/default configuration fails creation; an unknown ID, invalid working directory, missing/incompatible executor, or connection failure aborts creation/resumption. Disconnects terminate associated work; an in-flight call fails without automatic replay. Later calls may reconnect to the **same** environment, but there is **no silent local fallback**. Changing launcher configuration disconnects remote commands and bound remote MCP servers. [E4](#e4-permission-evaluation) [E8](#e8-remote-environments)

SSH normally uses `-T`, `BatchMode=yes`, a 10-second connect timeout, and `StrictHostKeyChecking=accept-new`: a new key is trusted on first use, a changed key fails. Interactive TUI password fallback uses OpenSSH Askpass and needs OpenSSH 8.4+; it does not cover independent OTP/MFA or private-key passphrase prompts. Print mode has no password callback and fails promptly. Executor handshakes/platform probes have bounded timeouts. None of these failures grants host execution as a retry. [E8](#e8-remote-environments)

## Evidence and unresolved checks

The evidence records below distinguish documentation, inspected source, release history, and local observation. Source links are pinned to the examined commit so future changes do not silently rewrite the basis of this report.

### E1: Product identity

Official [archived repository and migration notice](https://github.com/MoonshotAI/kimi-cli/blob/9ab1286b8fe4e6bcd116949a27ce5e0ac3389c82/README.md), [current repository](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/README.md), and [migration guide](https://moonshotai.github.io/kimi-code/en/guides/migration.html). Documentation/source inspection, November 4, 2026.

### E2: Release history

Official [2.5.0 release](https://github.com/MoonshotAI/kimi-code/releases/tag/%40moonshot-ai/kimi-code%402.5.0), [1.52.0 legacy release](https://github.com/MoonshotAI/kimi-cli/releases/tag/1.52.0), and pinned [changelog](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/docs/en/release-notes/changelog.md). Release/source evidence, November 4, 2026.

### E3: Host execution

Inspected [LocalEnvironment](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/environment/localEnvironment.ts), [host process backend](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/os/backends/node-local/hostProcessService.ts), and [host filesystem backend](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/os/backends/node-local/hostFsService.ts). Direct host bindings and subprocess inheritance support the negative finding of no built-in local OS sandbox; no hostile runtime probes were performed.

### E4: Permission evaluation

Inspected [policy ordering](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/permissionPolicy/permissionPolicyService.ts), [rule evaluation](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/permissionPolicy/policies/user-configured-rule.ts), [default approvals](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/permissionPolicy/policies/default-tool-approve.ts), [POSIX Git write approvals](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/permissionPolicy/policies/git-cwd-write-approve.ts), and [guard configuration](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/permissionRules/configSection.ts); compared with [interaction docs](https://moonshotai.github.io/kimi-code/en/guides/interaction.html) and [permission reference](https://moonshotai.github.io/kimi-code/en/configuration/config-files.html#permission). Source-derived ordering takes precedence in this report where docs simplify it.

### E5: Workspace trust

Official [trust configuration](https://moonshotai.github.io/kimi-code/en/configuration/config-files.html#workspace-trust), inspected [trust store](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/workspace/workspaceTrust/workspaceTrustStore.ts), [TUI trust startup](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/apps/kimi-code/src/tui/kimi-tui.ts), [trust dialog](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/apps/kimi-code/src/tui/components/dialogs/trust-prompt.ts), and [print-mode trust handling](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/apps/kimi-code/src/cli/v2/run-v2-print.ts). Documentation and source, not a first-run test.

### E6: File-tool boundaries

Inspected [path-access policy](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/tool/path-access.ts), [Read](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/tools/os/read/readTool.ts), [Write](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/tools/os/write/writeTool.ts), [Edit](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/tools/edit/editTool.ts), [Grep](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/tools/os/grep/grepTool.ts), [Glob](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/agent/tools/os/glob/globTool.ts), and separate [workspace filesystem service](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/workspace/workspaceFs/fsService.ts). Source-derived findings; symlink behavior remains untested at runtime.

### E7: Command controls

Official [command reference](https://moonshotai.github.io/kimi-code/en/reference/kimi-command.html) and pinned [print-mode implementation](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/apps/kimi-code/src/cli/v2/run-v2-print.ts), including forced auto permission. Documentation/source, November 4, 2026.

### E8: Remote environments

Official [remote environment guide](https://moonshotai.github.io/kimi-code/en/guides/remote-environment.html), pinned [launcher source](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/remote/client/launchers.ts), [protocol version requirements](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/remote/protocol/methods.ts), and [executor filesystem handler](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/packages/agent-core-v2/src/remote/server/fsHandler.ts). Documentation and source; isolation quality depends on user-provisioned target and has not been exercised here.

### E9: Hooks, plugins, and MCP

Official [hooks](https://moonshotai.github.io/kimi-code/en/customization/hooks.html), [plugins](https://moonshotai.github.io/kimi-code/en/customization/plugins.html), [MCP](https://moonshotai.github.io/kimi-code/en/customization/mcp.html), and [agents](https://moonshotai.github.io/kimi-code/en/customization/agents.html) documentation, also inspected as files in the pinned source snapshot.

### E10: Network and tool selection

Official [environment variables and HTTP proxy behavior](https://moonshotai.github.io/kimi-code/en/configuration/env-vars.html), [global tools settings](https://moonshotai.github.io/kimi-code/en/configuration/config-files.html#tools), and [agent tool policies](https://moonshotai.github.io/kimi-code/en/customization/agents.html), compared with E3's direct process backend. No global enforced network filter is established by those settings.

### E11: Installation and data

Official [installation](https://moonshotai.github.io/kimi-code/en/guides/getting-started.html), [data locations](https://moonshotai.github.io/kimi-code/en/configuration/data-locations.html), and pinned [development requirements](https://github.com/MoonshotAI/kimi-code/blob/f8e15c7a9be44fe4e0361a4ce57eaf690e05476a/package.json). Native Windows/Git Bash support is documented; WSL2 behavior here is an inference from the Linux execution path, not a WSL runtime test.

### E12: Local observation

On November 4, 2026 UTC, `sniff software agents --json` located `/Users/ken/.kimi-code/bin/kimi`; `kimi --version` printed `2.1.1`. `kimi --help` showed the approval, print, plan, and additional-directory flags described above and no sandbox option. These are metadata/help observations only, with no user configuration used to infer defaults.

### Remaining checks

- **Cross-platform runtime behavior:** run 2.5.0 in disposable macOS, Linux, native Windows, and WSL2 accounts with clean data roots; exercise manual, YOLO, print, and trust gating with a stub model. Current platform findings are documentation/source conclusions, not runtime certification.
- **Symlink and secret-file scope:** directly exercise Read/Write/Edit/Grep with harmless fixture secrets, external symlinks, and absolute outside paths. Compare agent tools with UI/API filesystem requests and identify exactly what 2.1.1 rolled back; do not infer confinement from the 2.1.0 release note.
- **Rule precedence drift:** use a deterministic tool-call fixture with overlapping allow/deny/ask rules and manual/auto modes, then obtain an upstream correction to the first-match documentation or the decision-class implementation.
- **Remote boundary:** provision a minimal 2.5.0 executor/container and verify file/process placement, local hooks/web calls, explicit MCP binding, and disconnect behavior. Repeat for a Windows SSH target and WSL2 controller; the observed 2.1.1 executable cannot settle 2.5.0 behavior.

## Changelog

- **2026-11-04:** Initial report. Distinguished archived Python Kimi CLI from current Kimi Code CLI; established absence of a default OS sandbox, documented 2.5.0 optional remote tool execution, and recorded permission-order and symlink documentation/source discrepancies plus the local 2.1.1 version limitation.
