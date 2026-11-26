---
$schema: "./_schema.yaml"
schema_revision: 5
provider: "opencode"
created: "2026-11-04"
last_updated: "2026-11-04"
content_policy:
  - "ValidFor(14d)"
agent: "codex"
model: "gpt-6.1-sol"
versions_examined:
  - "1.19.5"
  - "1.18.35"
evidence:
  -
    id: "security-policy"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/SECURITY.md"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Upstream explicitly supplies no agent sandbox; permissions are awareness controls; containers or VMs are operator deployments; configured MCP is outside its trust boundary."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "release"
    method: "official_docs"
    location: "https://github.com/anomalyco/opencode/releases/tag/v1.19.5"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Latest-release endpoint returned 1.19.5, published 2026-10-28; release notes describe a TUI worker error fix."
    limitations: "Release identity and notes, not execution evidence; latest is time-sensitive."
  -
    id: "agent-defaults"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/agent/agent.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Build is the default; general tools allow; external paths, doom loops, and .env reads ask; temporary/skill/reference/output paths have exceptions; agent-specific permissions apply last; plan is tool policy."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "permission-engine"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/permission/index.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Last matching rule wins; deny fails before asking; once/reject/always handle pending requests; always approvals live in instance state; leading home patterns expand."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "permission-docs"
    method: "official_docs"
    location: "https://opencode.ai/docs/permissions/"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Documents permission UX and --auto; describes .env default as deny, contradicting released ask implementation."
    limitations: "Live documentation is not pinned to the executable; default contradiction resolved from tagged source."
  -
    id: "shell-host"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/tool/shell.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Shell scans selected path arguments and command patterns, then launches an ordinary host subprocess with cwd and inherited process.env plus plugin additions; Windows has native shell branches."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "file-read"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/tool/read.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Read checks supplied-path external_directory and read permissions, then uses ordinary filesystem APIs; read also warms LSP."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "file-edit"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/tool/edit.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Edit performs external-directory and edit checks around host file operations; no OS confinement boundary."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "workspace-boundary"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/project/instance-context.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Project membership includes starting directory or Git worktree; non-Git root worktree is excluded from this predicate."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "external-paths"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/tool/external-directory.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "External-directory guard is an application check on path spellings and parent-directory patterns; bypass is possible for caller-selected paths."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "path-containment"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/core/src/fs-util.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Containment uses relative path comparison, not a race-resistant filesystem capability boundary."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "run-client"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/cli/cmd/run.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Headless run rejects ask events unless --auto; hidden --yolo and --dangerously-skip-permissions use the same auto boolean; --attach runs against a server."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "config-loader"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/config/config.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Config layers merge through account/organization and managed sources; OPENCODE_PERMISSION merges later, invalid JSON warns and is skipped; legacy tools become lower-priority permissions."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "managed-config"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/config/managed.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Managed settings use OS-specific directories and macOS ai.opencode.managed preferences; OPENCODE_TEST_MANAGED_CONFIG_DIR overrides managed-directory lookup."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "config-flags"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/core/src/flag/flag.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "OPENCODE_CONFIG, OPENCODE_CONFIG_CONTENT, OPENCODE_CONFIG_DIR, OPENCODE_DISABLE_PROJECT_CONFIG, and OPENCODE_PERMISSION select configuration, not a sandbox."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "runtime-flags"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/effect/runtime-flags.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "OPENCODE_PURE suppresses external plugins; OPENCODE_EXPERIMENTAL_CODE_MODE defaults false unless broad experimental mode enables it."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "plugin-runtime"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/plugin/index.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "External plugins load in the host runtime unless pure mode suppresses them; configuration/plugin discovery is not an isolation boundary."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "custom-tools"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/tool/registry.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Custom tools execute their implementation in the host runtime and receive a cooperative ask API; experimental CodeMode registration is conditional."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "subagents"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/agent/subagent-permissions.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Task-derived child sessions carry parent session deny and external-directory rules, not all parent-agent restrictions; child agent permissions still apply."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "mcp-startup"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/mcp/index.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Local stdio MCP starts the configured command with cwd, inherited environment and overrides; remote MCP uses remote transports without a provider-created process sandbox."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "mcp-permissions"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/session/tools.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Model MCP tool invocations pass a named-tool permission check before calling the server; startup and implementation effects are not confined by it."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "webfetch"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/tool/webfetch.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Webfetch checks its tool permission before host HTTP; this is not a host egress restriction."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "permission-schema"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/core/src/v1/config/permission.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Stable 1.x webfetch/websearch permission values are scalar allow/ask/deny; read/edit/bash/external_directory accept ordered pattern objects."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "model-transport"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/session/llm.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Model inference uses a separate provider transport path; webfetch permission does not govern it."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "windows-wsl"
    method: "official_docs"
    location: "https://opencode.ai/docs/windows-wsl/"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Native Windows execution is supported; WSL is recommended and Windows files are accessible through mounted drive paths."
    limitations: "Documentation establishes support, not executed Windows/WSL2 permission behavior."
  -
    id: "official-install"
    method: "official_docs"
    location: "https://opencode.ai/docs/"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Official onboarding describes provider setup and a Docker image invocation, without a sandbox selection step."
    limitations: "Documented onboarding only; no fresh interactive installation or image execution."
  -
    id: "docker-image"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/Dockerfile"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Official image packages the Linux CLI in Alpine and declares no USER or secure mount/network profile; isolation is supplied by the chosen container runtime."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "github-integration"
    method: "official_docs"
    location: "https://opencode.ai/docs/github/"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Official integration runs in GitHub Actions with workflow/model credentials and GitHub app or caller-token authorization."
    limitations: "Runner choice determines containment; no disposable Actions run performed."
  -
    id: "worktree-naming"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/worktree/index.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Project sandboxes terminology includes Git worktree directories, not OS confinement."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "codemode"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/tool/code-mode.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Experimental execute tool interprets orchestration code with supplied host-tool authority; does not sandbox those tool implementations."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "codemode-contract"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/codemode/README.md"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "Restricted interpreter denies ambient process/filesystem/network/module authority to programs, while host tools own permissions and external side effects."
    limitations: "Source inspection only; no runtime containment or cross-platform behavior test."
  -
    id: "v2-policy-condition"
    method: "official_docs"
    location: "https://opencode.ai/docs/policies/"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "experimental.policies is an OpenCode 2 feature and is not enforced in OpenCode 1."
    limitations: "Does not establish future stable 2.x sandbox behavior."
  -
    id: "local-help"
    method: "local_inspection"
    location: "/Users/ken/.opencode/bin/opencode"
    version: "1.18.35"
    observed_on: "2026-11-04"
    claim: "Read-only --version returned 1.18.35; root/run help exposed --auto, --pure, server/attach and no sandbox flag; sniff corroborated binary/version."
    limitations: "macOS help/version inspection only; no model or permission session; user configuration not used as clean-install evidence."
  -
    id: "local-version-defaults"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/v1.18.35/packages/opencode/src/agent/agent.ts"
    version: "1.18.35"
    observed_on: "2026-11-04"
    claim: "Tagged 1.18.35 source also uses ask for .env reads."
    limitations: "Version-tag source comparison, not executed reads."
  -
    id: "workspace-adapters"
    method: "source_code"
    location: "https://github.com/anomalyco/opencode/blob/810c0eba1a7348036f8bdf61e6632d0c36c6876c/packages/opencode/src/control-plane/adapters/index.ts"
    version: "1.19.5"
    observed_on: "2026-11-04"
    claim: "The built-in workspace adapter registry contains only worktree; plugins can register other adapters. This registry does not supply a built-in container or VM backend."
    limitations: "Source inspection only; custom adapters and remote service deployments were not tested."
summary: "Stable OpenCode 1.x supplies permission and approval controls, not an agent sandbox. Host authority remains; optional Docker, VM, remote-server and Actions deployments rely on operator isolation."
support: "policy_only"
default_profiles:
  -
    id: "macos-interactive"
    environment: "macos"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode starts the build agent; most tools allow; external paths, .env reads, and doom loops ask in the UI. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
  -
    id: "macos-non-interactive"
    environment: "macos"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode run uses the build agent; allowed tools execute; permission asks automatically reject unless auto is enabled. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
  -
    id: "linux-interactive"
    environment: "linux"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode starts the build agent; most tools allow; external paths, .env reads, and doom loops ask in the UI. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
  -
    id: "linux-non-interactive"
    environment: "linux"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode run uses the build agent; allowed tools execute; permission asks automatically reject unless auto is enabled. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
  -
    id: "windows-interactive"
    environment: "windows"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode starts the build agent; most tools allow; external paths, .env reads, and doom loops ask in the UI. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Native Windows support is documented independently of WSL; native shell branches exist. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
      - "windows-wsl"
  -
    id: "windows-non-interactive"
    environment: "windows"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode run uses the build agent; allowed tools execute; permission asks automatically reject unless auto is enabled. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Native Windows support is documented independently of WSL; native shell branches exist. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
      - "windows-wsl"
  -
    id: "wsl2-interactive"
    environment: "wsl2"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode starts the build agent; most tools allow; external paths, .env reads, and doom loops ask in the UI. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Linux CLI in WSL2 can access mounted Windows drives; WSL2 is not OpenCode-created containment. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
      - "windows-wsl"
  -
    id: "wsl2-non-interactive"
    environment: "wsl2"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "opencode run uses the build agent; allowed tools execute; permission asks automatically reject unless auto is enabled. No agent sandbox; host-user files, environment, process and network authority remain. Documented first-run provider setup does not select a sandbox. Linux CLI in WSL2 can access mounted Windows drives; WSL2 is not OpenCode-created containment. Runtime matrix not exercised."
    evidence_ids:
      - "security-policy"
      - "agent-defaults"
      - "shell-host"
      - "run-client"
      - "official-install"
      - "windows-wsl"
mechanisms: []
controls:
  -
    id: "permission"
    kind: "config_key"
    name: "permission"
    values:
      - "allow"
      - "ask"
      - "deny"
      - "object of permission keys and ordered pattern rules"
    effect: "Application tool policy only; does not isolate host file/process/network access."
    example: "{\"permission\":{\"bash\":\"ask\",\"edit\":\"deny\",\"webfetch\":\"deny\"}}"
    precedence: "Merged config then agent overrides; last matching permission rule wins. Legacy tools booleans map below explicit permission."
    mechanism_ids: []
    evidence_ids:
      - "permission-engine"
      - "agent-defaults"
      - "config-loader"
      - "permission-schema"
  -
    id: "agent-permission"
    kind: "config_key"
    name: "agent.<name>.permission"
    values:
      - "permission object"
    effect: "Overrides the selected agent tool policy, including plan/subagent behavior; no OS boundary."
    example: "{\"agent\":{\"build\":{\"permission\":{\"bash\":\"deny\"}}}}"
    precedence: "Merged after defaults and top-level user permission; parent agent policy is not automatically all child policy."
    mechanism_ids: []
    evidence_ids:
      - "agent-defaults"
      - "subagents"
  -
    id: "external-directory"
    kind: "config_key"
    name: "permission.external_directory"
    values:
      - "allow"
      - "ask"
      - "deny"
      - "ordered path-pattern object"
    effect: "Application permission for additional paths outside starting directory or Git worktree; does not mount or confine paths."
    example: "{\"permission\":{\"external_directory\":{\"*\":\"ask\",\"~/shared/reference/*\":\"allow\"}}}"
    precedence: "Last matching rule; home prefix expansion; built-in temp/skill/reference/output exceptions precede user rules."
    mechanism_ids: []
    evidence_ids:
      - "external-paths"
      - "workspace-boundary"
      - "permission-engine"
      - "agent-defaults"
  -
    id: "auto"
    kind: "cli_flag"
    name: "--auto"
    values: []
    effect: "Relaxes approval only: automatically approves asks on interactive and run paths; explicit denies remain; no sandbox exists to disable."
    example: "opencode run --auto \"Run the requested checks\""
    precedence: "False by default; replies once to requests that already passed explicit deny evaluation."
    mechanism_ids: []
    evidence_ids:
      - "run-client"
      - "permission-engine"
      - "permission-docs"
      - "local-help"
  -
    id: "yolo"
    kind: "cli_flag"
    name: "--yolo"
    values: []
    effect: "Hidden run compatibility switch with the same approval-only behavior as --auto; no OS privilege change."
    example: "opencode run --yolo \"Run the requested checks\""
    precedence: "ORed into the same auto boolean; explicit deny remains enforced before the request event."
    mechanism_ids: []
    evidence_ids:
      - "run-client"
      - "permission-engine"
  -
    id: "skip-permissions"
    kind: "cli_flag"
    name: "--dangerously-skip-permissions"
    values: []
    effect: "Hidden run compatibility switch with the same approval-only behavior as --auto; no OS privilege change."
    example: "opencode run --dangerously-skip-permissions \"Run the requested checks\""
    precedence: "ORed into the same auto boolean; explicit deny remains enforced before the request event."
    mechanism_ids: []
    evidence_ids:
      - "run-client"
      - "permission-engine"
  -
    id: "permission-env"
    kind: "environment_variable"
    name: "OPENCODE_PERMISSION"
    values:
      - "JSON permission object"
    effect: "Runtime permission override; no containment; malformed JSON warns and is skipped."
    example: "OPENCODE_PERMISSION='{\"bash\":\"deny\",\"edit\":\"deny\"}' opencode run \"Explain the code\""
    precedence: "Merged after system managed files and macOS managed preferences; agent-specific overrides still apply afterward."
    mechanism_ids: []
    evidence_ids:
      - "config-loader"
      - "config-flags"
      - "agent-defaults"
  -
    id: "config-file"
    kind: "environment_variable"
    name: "OPENCODE_CONFIG"
    values:
      - "configuration file path"
    effect: "Selects an explicit merged config file, including tool guardrails; not a sandbox profile."
    example: "OPENCODE_CONFIG=/absolute/path/review.json opencode run \"Review the changes\""
    precedence: "After global config; before project and directory configs, inline, account and managed layers."
    mechanism_ids: []
    evidence_ids:
      - "config-loader"
      - "config-flags"
  -
    id: "config-directory"
    kind: "environment_variable"
    name: "OPENCODE_CONFIG_DIR"
    values:
      - "configuration directory path"
    effect: "Adds directory configuration discovery; does not restrict agent filesystem access."
    example: "OPENCODE_CONFIG_DIR=/absolute/path/opencode-config opencode"
    precedence: "Directory layer after project discovery, before inline/account/managed layers."
    mechanism_ids: []
    evidence_ids:
      - "config-loader"
      - "config-flags"
  -
    id: "config-inline"
    kind: "environment_variable"
    name: "OPENCODE_CONFIG_CONTENT"
    values:
      - "inline JSON configuration"
    effect: "Sets runtime configuration including approval policy; no execution boundary."
    example: "OPENCODE_CONFIG_CONTENT='{\"permission\":{\"bash\":\"deny\"}}' opencode run \"Review\""
    precedence: "After project/directory settings; before account/organization and managed layers; OPENCODE_PERMISSION later."
    mechanism_ids: []
    evidence_ids:
      - "config-loader"
      - "config-flags"
  -
    id: "disable-project-config"
    kind: "environment_variable"
    name: "OPENCODE_DISABLE_PROJECT_CONFIG"
    values:
      - "true"
      - "false"
    effect: "Skips project configuration discovery; not a filesystem sandbox or general code trust guarantee."
    example: "OPENCODE_DISABLE_PROJECT_CONFIG=true opencode"
    precedence: "Controls project discovery within config loading; other applicable configuration sources still merge."
    mechanism_ids: []
    evidence_ids:
      - "config-loader"
      - "config-flags"
  -
    id: "managed-files"
    kind: "managed_policy"
    name: "opencode.json / opencode.jsonc"
    values:
      - "configuration object in system managed directory"
    effect: "Admin deployment layer: macOS /Library/Application Support/opencode; Linux/WSL /etc/opencode; Windows %ProgramData%\\opencode. Tool policy, not tamper-proof containment."
    example: "/etc/opencode/opencode.json: {\"permission\":{\"bash\":\"deny\"}}"
    precedence: "After account/inline/local config, before macOS managed preferences and OPENCODE_PERMISSION; agent-specific rules apply later. OPENCODE_TEST_MANAGED_CONFIG_DIR is a test override."
    mechanism_ids: []
    evidence_ids:
      - "managed-config"
      - "config-loader"
      - "agent-defaults"
  -
    id: "macos-mdm"
    kind: "managed_policy"
    name: "ai.opencode.managed"
    values:
      - "configuration dictionary in managed plist"
    effect: "macOS managed preferences deploy tool configuration; not a sandbox."
    example: "/Library/Managed Preferences/ai.opencode.managed.plist: permission = { bash = deny; }"
    precedence: "Per-user managed plist checked first, then system plist; loaded after managed JSON. Late OPENCODE_PERMISSION and agent permissions limit claims of administrator immutability."
    mechanism_ids: []
    evidence_ids:
      - "managed-config"
      - "config-loader"
      - "agent-defaults"
  -
    id: "pure"
    kind: "cli_flag"
    name: "--pure"
    values: []
    effect: "Suppresses external plugins; leaves host authority, built-ins and configured MCP; approval-adjacent hardening only."
    example: "opencode --pure"
    precedence: "Runtime plugin-loader switch; does not replace permission policy."
    mechanism_ids: []
    evidence_ids:
      - "plugin-runtime"
      - "runtime-flags"
      - "local-help"
  -
    id: "pure-env"
    kind: "environment_variable"
    name: "OPENCODE_PURE"
    values:
      - "true"
      - "false"
    effect: "Environment form of external-plugin suppression; no isolation."
    example: "OPENCODE_PURE=true opencode"
    precedence: "Runtime flag consumed by plugin loader; distinct from tool permissions."
    mechanism_ids: []
    evidence_ids:
      - "plugin-runtime"
      - "runtime-flags"
  -
    id: "plan"
    kind: "cli_flag"
    name: "--agent"
    values:
      - "agent name, including plan"
    effect: "Selecting plan changes tool permissions and plan-file editing policy; does not establish read-only confinement or confidentiality."
    example: "opencode --agent plan"
    precedence: "Agent defaults plus user/agent overrides; no OS enforcement."
    mechanism_ids: []
    evidence_ids:
      - "agent-defaults"
      - "local-help"
  -
    id: "server-password"
    kind: "environment_variable"
    name: "OPENCODE_SERVER_PASSWORD"
    values:
      - "password string"
    effect: "Adds HTTP Basic authentication to server access; no egress or filesystem boundary."
    example: "OPENCODE_SERVER_PASSWORD=replace-me opencode serve --hostname 127.0.0.1 --port 4096"
    precedence: "Opt-in server credential; OPENCODE_SERVER_USERNAME optionally changes the username; unattested server environment owns execution."
    mechanism_ids: []
    evidence_ids:
      - "security-policy"
      - "run-client"
      - "local-help"
  -
    id: "codemode"
    kind: "environment_variable"
    name: "OPENCODE_EXPERIMENTAL_CODE_MODE"
    values:
      - "true"
      - "false"
    effect: "Enables restricted orchestration interpreter only; supplied host tools remain authoritative and unconfined; not an agent sandbox."
    example: "OPENCODE_EXPERIMENTAL_CODE_MODE=true opencode"
    precedence: "Default false unless OPENCODE_EXPERIMENTAL enables it; specific setting overrides broad experimental default."
    mechanism_ids: []
    evidence_ids:
      - "runtime-flags"
      - "codemode"
      - "codemode-contract"
gaps:
  -
    location: "default_profiles"
    detail: "Eight-case defaults are source/documentation-derived; local inspection exercised help/version only, not fresh onboarding or tool approvals."
    next_check: "Run clean-profile sentinel read/edit/shell approval fixtures in interactive and headless modes on all four environments; record exact binaries and startup choices."
  -
    location: "controls/permission"
    detail: "Documentation says .env reads deny; 1.19.5 and 1.18.35 tagged defaults say ask. Source is authoritative here, but runtime corroboration remains."
    next_check: "Use a disposable project with a non-secret .env sentinel and clean config; verify TUI asks and ordinary run rejects; recheck docs for correction."
  -
    location: "controls/external-directory"
    detail: "Symlink/junction/reparse behavior and races were not tested; path-string checks do not establish confinement."
    next_check: "Use harmless external sentinel files through symlinks on Unix and junction/reparse paths on Windows, recording permission events and actual targets."
  -
    location: "controls/managed-files"
    detail: "Late environment merge and agent overrides contradict an unrestricted reading of managed-setting immutability; runtime precedence was not exercised."
    next_check: "Load synthetic managed JSON/plist plus inline, OPENCODE_PERMISSION, and agent overrides; inspect effective config without provider credentials."
  -
    location: "mechanisms"
    detail: "No agent sandbox is supplied. Official Docker/remote/Actions deployment can move execution, but actual isolation depends on operator runtime settings and was not exercised."
    next_check: "Inspect a pinned official image and disposable configured server for user, mounts, network, secrets and MCP placement; keep deployment evidence separate from CLI defaults."
  -
    location: "support"
    detail: "This report covers stable 1.x; current 2.x-only policy documentation cannot establish future stable sandbox support."
    next_check: "When 2.x is released, inspect its versioned security policy, config schema, and process/file execution paths before updating the support classification."
changes: []
---

# Sandboxing in OpenCode CLI

OpenCode CLI **does not supply an agent sandbox** in the examined stable release. Its own security policy calls permissions a user-awareness feature rather than security isolation, and recommends running the whole application in a container or VM when isolation is needed. A normal installation runs with the launching user's filesystem, process, environment, and network authority. Permission checks can stop particular tool invocations, but they do not confine the process or everything a permitted command can do. This is a verified upstream design statement, corroborated by execution source, rather than a conclusion drawn from missing documentation. [Security policy, v1.19.5](https://github.com/anomalyco/opencode/blob/v1.19.5/SECURITY.md), [shell execution source](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/shell.ts).

## Scope and evidence

Researched **2026-11-04 UTC**. GitHub's latest-release endpoint returned **v1.19.5**, published **2026-10-28**; its release notes describe a TUI worker error fix, not a sandbox addition. Implementation findings below use that tag's downloaded source archive. Official website documentation was checked on the research date, including its explicit distinctions between OpenCode 1 and OpenCode 2. The report concerns the stable 1.x CLI; 2.x-only policy examples do not establish supported stable behavior. [Release](https://github.com/anomalyco/opencode/releases/tag/v1.19.5), [version-qualified policies documentation](https://opencode.ai/docs/policies/).

Read-only local inspection on macOS found `/Users/ken/.opencode/bin/opencode`, version **1.18.35**. Executed `opencode --version`, `opencode --help`, and `opencode run --help`; these expose `--auto`, `--pure`, server/attach operations, and no sandbox selector. Sniff independently identified the executable/version. No model session, credential inspection, permission-bypass test, or container was launched. Local help corroborates command availability, not runtime containment. Source inspection is not a runtime test on any OS. Clean-install defaults below come from released source, not this user's settings.

## Default protection by platform and session

The ordinary default agent is `build`. Its baseline allows tools, asks about external directories and repeated identical calls (`doom_loop`), and asks before built-in reads matching `*.env` or `*.env.*`; `*.env.example` is allowed. Internal output, application temporary, skill, and reference directories have external-directory exceptions. **Documentation drift:** both the tagged permissions page and the live page describe `.env` reads as denied, but **1.19.5 source says `ask`**. The 1.18.35 tagged agent source agrees with `ask`. This report uses the executable implementation for that default. [Agent defaults](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/agent/agent.ts), [1.18.35 comparison](https://github.com/anomalyco/opencode/blob/v1.18.35/packages/opencode/src/agent/agent.ts), [permissions documentation](https://opencode.ai/docs/permissions/).

| Platform | Interactive default (`opencode`) | Non-interactive default (`opencode run`) | Platform distinction |
| --- | --- | --- | --- |
| macOS | No sandbox; normally allowed tools run immediately; `ask` opens approval UI. | No sandbox; allowed tools run; `ask` is automatically rejected. | No OpenCode Seatbelt/App Sandbox boundary. |
| Linux | Same permissive tool defaults and approval UI; no sandbox. | Same immediate allow / automatic rejection of asks; no sandbox. | No OpenCode namespace, seccomp, Landlock, or container confinement. |
| Native Windows | Same permission model; no sandbox. | Same immediate allow / automatic rejection of asks; no sandbox. | Native execution is supported; there is no OpenCode AppContainer/restricted-token sandbox. |
| WSL2 | Linux CLI permission behavior; no OpenCode sandbox. | Linux CLI behavior, including automatic rejection of asks; no OpenCode sandbox. | WSL is an existing Linux environment, not a boundary created by this CLI; mounted Windows drives remain accessible. |

The no-sandbox finding is platform-independent upstream policy and is corroborated by ordinary shell spawning and direct file access. The session difference is source-derived: `run` answers permission events with `reject` unless auto mode is active. Native Windows support is established independently by upstream Windows documentation and Windows shell execution branches; it is not inferred from WSL support. WSL documentation explicitly demonstrates access through `/mnt/c/`. Ordinary OS permissions, ACLs, and host protections still apply. [Security policy](https://github.com/anomalyco/opencode/blob/v1.19.5/SECURITY.md), [run event handler](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/cli/cmd/run.ts), [native shell implementation](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/shell.ts), [Windows and WSL](https://opencode.ai/docs/windows-wsl/).

First-run setup involves installation and model/provider connection; provider credentials or a usable local model configuration are needed for inference, not for an isolation backend. The documented onboarding has no sandbox-backend selection or restrictive/YOLO security choice. Project configuration and project plugins are loaded through normal discovery; a project-trust prompt must not be assumed. This first-run conclusion is limited to documented onboarding plus the inspected config/plugin paths, not a fresh interactive-install test. [Getting started](https://opencode.ai/docs/), [config loader](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/config.ts), [plugin loader](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/plugin/index.ts).

## What the agent can access

### Files, home, credentials, and paths

Built-in `read` resolves paths and calls `external_directory`, then the `read` permission check, before returning file content. Editing uses the same external-path helper and an `edit` permission. Those checks run in the application, while actual I/O uses the host filesystem. There is no mounted workspace with a kernel-enforced writable/read-only split. Permitting reads can disclose their contents to the model even when edits are denied. [Read tool](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/read.ts), [edit tool](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/edit.ts).

The effective internal project boundary includes **the starting directory OR the Git worktree**. Starting in a repository subdirectory does not make its siblings external. Non-Git projects deliberately avoid treating `/` as the workspace. Additional paths can be admitted with `permission.external_directory` patterns, including leading `~` and `$HOME` expansion. This grants a permission exception, not a filesystem mount or confinement rule. The source definition is more precise than the docs' shorthand “working directory.” [Boundary predicate](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/project/instance-context.ts), [external-directory helper](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/external-directory.ts), [pattern expansion](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/permission/index.ts).

Home and credential files are not hidden from the process. Explicit out-of-project file-tool reads normally ask, except approved directories; this is not a comprehensive credential denylist. Shell execution inherits `process.env` plus plugin-provided additions. A permitted interpreter or script therefore retains environmental secrets and host-readable files even if a specialized `read` call would ask. No protected-file or secret-redaction guarantee follows from the `.env` rule. Similarly, there is no private temporary filesystem: application temporary/output directories are host paths, some explicitly allowed by default. [Shell environment](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/shell.ts), [default path exceptions](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/agent/agent.ts).

Symlink handling is not an isolation guarantee. The read path uses the supplied resolved spelling for checks, and the containment helper compares relative paths; it does not establish an immutable, race-resistant mapping to permitted targets. Reads then use ordinary filesystem APIs. Thus an apparent project path should not be treated as proof that its target is confined. This is a source-derived limitation; junction, reparse-point, and symlink behavior were not exercised on the four platforms. [Read path](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/read.ts), [filesystem containment](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/core/src/fs-util.ts).

### Processes and network

The shell tool retains permission key `bash`, including on Windows where the selected shell can be PowerShell or cmd. It parses command syntax, gathers command patterns, and recognizes selected commands' path arguments for external-directory approval. This scan is not syscall mediation: code launched by an allowed command can open other paths, launch children, or connect to the network. The spawn path creates ordinary child processes with the selected shell, cwd, and environment. Detached process groups and timeouts are lifecycle controls, not security isolation. [Shell implementation](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/shell.ts), [stable tool ID](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/shell/id.ts).

There is no default egress firewall or host allowlist. Built-in `webfetch` asks its own permission before using an HTTP client; in OpenCode 1 its configuration accepts only `allow`, `ask`, or `deny`, **not URL-pattern objects**. Disabling that tool does not disable shell HTTP clients, plugins, local MCP networking, or remote MCP connections. Model/API transport is a separate host-side provider operation and is not gated by `permission.webfetch`. The 2.x-only `experimental.policies` provider controls are also not a stable 1.x network sandbox. [Webfetch implementation](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/webfetch.ts), [1.x permission schema](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/core/src/v1/config/permission.ts), [provider transport](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/session/llm.ts), [policy version condition](https://opencode.ai/docs/policies/).

## Which execution surfaces share the limits?

There is no common isolation boundary to inherit:

| Surface | Actual control and execution location |
| --- | --- |
| Built-in file and shell tools | Application permission checks followed by host I/O or host child processes. |
| Subagents | Tool/session permissions, not a separate container. The task path carries parent **session** deny and external-directory rules; parent **agent** restrictions do not automatically become all child restrictions. Child agent permissions matter. |
| Plugin hooks and custom tools | Imported host-runtime code. Hooks can execute commands and change tool inputs/environment; custom tools receive an `ask` API, rather than an OS fence around arbitrary implementation code. |
| Local MCP | Configured command starts through stdio transport with cwd and inherited environment plus configured overrides. Tool-call permission checks do not sandbox server startup or the server's internals. |
| Remote MCP | Executes at the configured service under its own authentication and security controls. OpenCode does not attest or provide that service's isolation. |
| Formatters/LSP and other helpers | Host services invoked by OpenCode; file-tool permission settings do not create an encompassing process boundary. |

These are source-derived conclusions from [subagent permission derivation](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/agent/subagent-permissions.ts), [custom-tool adapter](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/registry.ts), [plugin runtime](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/plugin/index.ts), [MCP startup](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/mcp/index.ts), [MCP call gating](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/session/tools.ts), and [read-triggered LSP warm-up](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/read.ts). Upstream explicitly places configured MCP server behavior outside its trust boundary. [Security policy](https://github.com/anomalyco/opencode/blob/v1.19.5/SECURITY.md).

## Changing guardrails and handling failures

Configure `permission` in `opencode.json`/`opencode.jsonc` globally or per project. Values are `allow`, `ask`, and `deny`; ordered wildcard rules use the last match. Agent-specific rules under `agent.<name>.permission` override the agent's merged defaults. The legacy `tools` boolean settings remain compatibility input, mapped to permissions, with explicit permissions winning. [Permission evaluator](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/permission/index.ts), [config normalization](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/config.ts).

For example, this deliberately narrow built-in-tool profile permits reads and asks before shell execution while refusing edits and built-in network tools:

```json
{
  "$schema": "https://opencode.ai/config.json",
  "permission": {
    "*": "deny",
    "read": "allow",
    "bash": "ask",
    "external_directory": {
      "*": "deny",
      "~/shared/reference/*": "allow"
    },
    "edit": "deny",
    "webfetch": "deny",
    "websearch": "deny"
  }
}
```

The explicit `read: allow` replaces the built-in sensitive-name defaults for this permission; add `.env` patterns if needed. Shell approval still permits that command's actual capabilities. This profile does not constrain plugin startup, MCP startup, or model transport. For a headless run, `ask` will reject; select narrowly allowed commands in advance if they must run unattended. [Agent merge order](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/agent/agent.ts), [run handler](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/cli/cmd/run.ts).

The inspected config loader's low-to-high order is: authenticated `.well-known/opencode` defaults; global config; `OPENCODE_CONFIG`; project config; `.opencode`/`OPENCODE_CONFIG_DIR` content; `OPENCODE_CONFIG_CONTENT`; active account/organization config when present; system managed config; macOS managed preferences. **Then `OPENCODE_PERMISSION` JSON is merged into permissions.** Managed-config documentation describes its layer as overriding user/project settings, but this late environment merge and subsequent agent-specific overrides mean administrators should not interpret it as tamper-proof enforcement against a user who controls launch/configuration. Invalid `OPENCODE_PERMISSION` JSON logs a warning and is skipped, not fail-closed. [Config load sequence](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/config.ts).

Managed files are `opencode.json` or `opencode.jsonc` under `/Library/Application Support/opencode` on macOS, `/etc/opencode` on Linux/WSL, and `%ProgramData%\opencode` on Windows. macOS additionally reads `ai.opencode.managed.plist`, preferring `/Library/Managed Preferences/<user>/` before `/Library/Managed Preferences/`. These provide deployment/configuration control, not a new sandbox. The source also exposes `OPENCODE_TEST_MANAGED_CONFIG_DIR` as a test override, not a production security boundary. [Managed settings implementation](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/config/managed.ts).

Useful launch examples (POSIX syntax; PowerShell uses `$env:NAME` assignments):

```sh
# An explicit config file, still merged with other applicable layers.
OPENCODE_CONFIG=/absolute/path/review.json opencode run 'Review the changes'

# Runtime permission override; still a tool guard, not confinement.
OPENCODE_PERMISSION='{"bash":"deny","edit":"deny"}' opencode run 'Explain the code'

# Avoid external plugins; this does not isolate built-ins or configured MCP.
opencode --pure

# Approve asks automatically, preserving explicit deny rules.
opencode run --auto 'Run the requested checks'
```

`OPENCODE_CONFIG_DIR` chooses an additional configuration directory; `OPENCODE_CONFIG_CONTENT` supplies inline JSON; `OPENCODE_DISABLE_PROJECT_CONFIG=true` skips project config discovery. `--pure`/`OPENCODE_PURE=true` suppress external plugins. None changes filesystem/process authority. [Config flags](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/core/src/flag/flag.ts), [runtime flags](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/effect/runtime-flags.ts), [plugin loader](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/plugin/index.ts).

An explicit denied request fails in the permission service before executing that checked action. Rejection fails a pending request; interactive approval offers `once`, `always`, or rejection. Documentation describes `always` as lasting for the current session; source stores approvals in live instance state rather than a permanent policy file, so it should not be read as durable configuration or assumed to be a strict per-conversation security scope. Bare `serve` has no approval UI: requests wait for a client reply; ordinary `run` supplies the automatic rejection behavior described above. There is no sandbox-backend startup, unavailable-backend fallback, privilege-escalation protocol, or automatic retry outside confinement because execution was already on the host. A model might try another allowed tool; permission denial is not a global prohibition on equivalent effects. [Permission service](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/permission/index.ts), [run client](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/cli/cmd/run.ts).

`--auto` is false by default. In 1.19.5, hidden compatibility flags `--yolo` and `--dangerously-skip-permissions` on `run` feed the same auto boolean. They approve requests that reached `ask`, while explicit `deny` is evaluated earlier. They do not disable a sandbox or grant OS privileges. `permission: "allow"` is a broader configuration override that can replace permission restrictions. Plan mode likewise changes tool permissions and prompting; it is not an OS-enforced read-only environment, and its source includes allowed plan-file edits and user overrides. [Auto aliases](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/cli/cmd/run.ts), [deny evaluation](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/permission/index.ts), [plan agent](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/agent/agent.ts).

## Optional isolation and officially supported integrations

**Whole-CLI Docker deployment.** The official installation page supplies `docker run -it --rm ghcr.io/anomalyco/opencode`; the repository's CLI Dockerfile packages the Linux executable in Alpine with runtime libraries and ripgrep. This contains the CLI and its ordinary local descendants within the container environment, subject to Docker's runtime configuration. It is a separate installation/deployment choice requiring a working Docker engine and a compatible Linux-container environment, not a CLI setting that turns on a per-tool sandbox. The image declares no `USER`, so it does not itself establish a non-root application identity. Host mount permissions, credentials, Docker socket exposure, and networking must be chosen by the operator. OpenCode supplies no documented secure mount/network profile. No OpenCode paid account is required merely to run the image; model-provider access is separate. [Official Docker installation](https://opencode.ai/docs/), [CLI Dockerfile](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/Dockerfile).

**Remote server and GitHub integration.** `opencode serve` plus `opencode run --attach <url>` separates the client from the machine executing tools. If the server is inside an operator-provided VM/container, that environment can contain the agent; merely moving it to another host provides no sandbox there. Set `OPENCODE_SERVER_PASSWORD` for HTTP Basic authentication (optional `OPENCODE_SERVER_USERNAME`); this protects server access, not outbound egress or filesystem access. The official GitHub integration executes within the chosen Actions runner, requiring a repository workflow, GitHub authorization, and model access; the standard setup installs the OpenCode GitHub app, with a documented caller-token alternative. Runner isolation comes from the runner environment and its exposed secrets/resources, not a new OpenCode tool boundary. [CLI server/attach documentation](https://opencode.ai/docs/cli/), [server security](https://github.com/anomalyco/opencode/blob/v1.19.5/SECURITY.md), [GitHub integration](https://opencode.ai/docs/github/).

```mermaid
flowchart LR
  C[CLI client] --> S[OpenCode server and model transport]
  subgraph E[Execution environment: host by default; operator container or VM optionally]
    S --> P[Application permission checks]
    P --> F[Built-in file operations]
    P --> B[Shell and descendants]
    S --> H[Plugin hooks and helper processes]
    S --> M[Local MCP process]
  end
  S --> A[Model API]
  S --> R[Remote MCP service]
```

The box denotes where execution happens, **not a sandbox supplied by OpenCode**. A container around only one MCP server contains that server, not built-in shell/file tools or the agent. The upstream `sandboxes` project/worktree terminology also does not establish an OS boundary: the implementation adds Git worktree directories to a project list. The experimental workspace adapter registry ships only a `worktree` adapter, which returns a local directory target; plugins may register other adapters, but the registry supplies no built-in container or VM backend. [Worktree implementation](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/worktree/index.ts), [adapter registry](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/control-plane/adapters/index.ts), [local target](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/control-plane/adapters/worktree.ts).

**Experimental CodeMode is narrower still.** `OPENCODE_EXPERIMENTAL_CODE_MODE=true` (also enabled by the broad `OPENCODE_EXPERIMENTAL` switch unless specifically overridden) adds an `execute` tool backed by the repository's restricted JavaScript interpreter. Programs lack ambient process/filesystem/network/module authority and call only supplied host tools. Those tools retain their own authority and permission checks. This is an interpreter capability boundary for orchestration code, not an agent, shell, plugin, or MCP process sandbox; the structured agent-sandbox mechanism inventory therefore remains empty. It needs no external OS backend. Its runtime/isolation claims here are source-derived, not adversarially tested. [Flag and default](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/effect/runtime-flags.ts), [CLI adapter](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/opencode/src/tool/code-mode.ts), [CodeMode contract](https://github.com/anomalyco/opencode/blob/v1.19.5/packages/codemode/README.md).

## Remaining checks

No unresolved finding changes the established absence of a stable agent sandbox. Remaining runtime questions are narrower: repeat clean-profile first-run and harmless permission fixtures on macOS, Linux, native Windows, and WSL2; exercise symlink/junction reads with sentinel files; verify managed-settings precedence using synthetic settings and `OPENCODE_PERMISSION`; and test the official image's user, mounts, network, and MCP behavior under an explicitly recorded Docker configuration. These checks would corroborate implementation-derived behavior, not turn permissions into isolation. A future 2.x release requires a separate review rather than importing current 2.x-only documentation into these defaults.

## Changelog

- **2026-11-04:** Initial research against stable 1.19.5 and local 1.18.35 help. Established no built-in agent sandbox; distinguished optional whole-process deployments and CodeMode; recorded `.env` documentation drift, headless rejection, and managed-settings precedence limitations.
