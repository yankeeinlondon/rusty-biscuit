---
$schema: "./_schema.yaml"
schema_revision: 5
provider: "claude"
created: "2026-11-04"
last_updated: "2026-11-04"
content_policy:
  - "ValidFor(14d)"
agent: "codex"
model: "gpt-6.1-sol"
versions_examined:
  - "2.1.318"
evidence:
  - id: "e1"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/overview"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Local CLI installation and supported authentication/provider choices."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e2"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/sandboxing"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Built-in shell sandbox is off by default, unsupported on native Windows, and has documented boundaries, dependencies and escape hatches."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e3"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/permission-modes"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Current interactive Auto and headless Manual defaults, first-run choices and approval-bypass behavior."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e4"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/permissions"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Application-level tool permission rules, working directories and workspace trust."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e5"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/sandbox-environments"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Official optional whole-process sandbox-runtime, container and cloud deployments and their scope."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e6"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/settings-reference"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Exact sandbox and file-tool restriction keys, defaults, scopes and version-specific exceptions."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e7"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/env-vars"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Subprocess credential scrub, restricted mode and temp/config environment variables."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e8"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/settings"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Settings source precedence and list merging."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e9"
    method: "source_code"
    location: "https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/CHANGELOG.md"
    version: "2.1.318"
    observed_on: "2026-11-04"
    claim: "Published release notes identify current release and historical sandbox hardening changes."
    limitations: "Source inspection is not a runtime test; the embedded CLI runtime revision is unverified."
  - id: "e10"
    method: "source_code"
    location: "https://github.com/anthropic-experimental/sandbox-runtime/blob/98a4016d27609ed34dd1fb43d23ed6efb6eeeb77/README.md"
    version: "0.0.81"
    observed_on: "2026-11-04"
    claim: "Pinned runtime README and adjacent backend source describe Seatbelt, bubblewrap, Windows alpha, prerequisites and residual limitations."
    limitations: "Source inspection is not a runtime test; the embedded CLI runtime revision is unverified."
  - id: "e11"
    method: "source_code"
    location: "https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/.devcontainer/init-firewall.sh"
    version: "2.1.318"
    observed_on: "2026-11-04"
    claim: "Reference container source has default-drop firewall plus DNS, SSH, loopback and host-network exceptions; adjacent configuration defines mounts and capabilities."
    limitations: "Source inspection is not a runtime test; the embedded CLI runtime revision is unverified."
  - id: "e12"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/remote-control"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Remote Control continues local execution rather than placing the agent in a remote sandbox."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e13"
    method: "local_inspection"
    location: "claude.md#local-observation"
    version: "2.1.318"
    observed_on: "2026-11-04"
    claim: "Read-only version/help and sniff inspection identify installed macOS CLI and exposed flags."
    limitations: "No sandbox or model session was launched; no local policy or credentials were inspected."
  - id: "e14"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/managed-settings"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Administrative policy delivery paths and managed bypass restrictions."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e15"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/devcontainer"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Official container feature and hardened reference have distinct setup, persistence and firewall requirements."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
  - id: "e16"
    method: "official_docs"
    location: "https://code.claude.com/docs/en/claude-code-on-the-web#security-and-isolation"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "CLI cloud launch uses an isolated hosted VM with credential proxies and retained API transport; eligibility and self-hosted exceptions apply."
    limitations: "Living vendor documentation accessed on the research date; not a runtime isolation test."
summary: "Local Claude Code has no enabled OS sandbox by default. An optional native shell sandbox restricts selected tool subprocesses on macOS, Linux and WSL2, leaving file tools, hooks and MCP outside. Native Windows lacks that built-in sandbox. Official whole-process deployments are separate opt-ins."
support: "native"
default_profiles:
  - id: "macos-interactive"
    environment: "macos"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "claude starts in Auto when eligible, otherwise Manual; trust, Auto notices and outside-read choices do not enable sandboxing. Opt-in shell isolation uses built-in Seatbelt."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "macos-non-interactive"
    environment: "macos"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "claude -p starts in Manual with no override; calls needing unanswered approval are denied. Trust dialog is skipped; no OS sandbox is enabled. Opt-in shell isolation uses built-in Seatbelt."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "linux-interactive"
    environment: "linux"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "claude starts in Auto when eligible, otherwise Manual; trust, Auto notices and outside-read choices do not enable sandboxing. Opt-in shell isolation needs bubblewrap, socat and usable namespaces; WSL2 is separate from native Windows."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "linux-non-interactive"
    environment: "linux"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "claude -p starts in Manual with no override; calls needing unanswered approval are denied. Trust dialog is skipped; no OS sandbox is enabled. Opt-in shell isolation needs bubblewrap, socat and usable namespaces; WSL2 is separate from native Windows."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "windows-interactive"
    environment: "windows"
    launch_mode: "interactive"
    status: "unavailable"
    mechanism_ids: []
    notes: "claude starts in Auto when eligible, otherwise Manual; trust, Auto notices and outside-read choices do not enable sandboxing. Built-in shell isolation is explicitly unsupported; WSL2 and standalone runtime Windows alpha are separate cases."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "windows-non-interactive"
    environment: "windows"
    launch_mode: "non_interactive"
    status: "unavailable"
    mechanism_ids: []
    notes: "claude -p starts in Manual with no override; calls needing unanswered approval are denied. Trust dialog is skipped; no OS sandbox is enabled. Built-in shell isolation is explicitly unsupported; WSL2 and standalone runtime Windows alpha are separate cases."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "wsl2-interactive"
    environment: "wsl2"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "claude starts in Auto when eligible, otherwise Manual; trust, Auto notices and outside-read choices do not enable sandboxing. Opt-in shell isolation needs bubblewrap, socat and usable namespaces; WSL2 is separate from native Windows."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "wsl2-non-interactive"
    environment: "wsl2"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "claude -p starts in Manual with no override; calls needing unanswered approval are denied. Trust dialog is skipped; no OS sandbox is enabled. Opt-in shell isolation needs bubblewrap, socat and usable namespaces; WSL2 is separate from native Windows."
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
mechanisms:
  - id: "shell-seatbelt"
    name: "Sandboxed Bash tool (macOS)"
    enforcement: "os_sandbox"
    environments:
      - "macos"
    backend: "Seatbelt profiles invoked through sandbox-exec; runtime source confirms the entry point. Exact CLI-embedded runtime revision unknown."
    prerequisites: []
    activation: "Opt in with /sandbox or sandbox.enabled=true; auto-allow and regular-permissions approval modes share the boundary."
    filesystem: "Reads broadly include home and credential files unless denied; writes are limited to working/additional directories, per-user temp and required shared Git metadata, with protected configuration writes denied. Symlink and rule behavior is version-sensitive."
    network: "No unrestricted direct outbound route; host proxy checks allowed/denied domains. Unknown hosts may be approved unless strict allowlist applies. Built-in web tools and parent model/API traffic are outside this shell policy."
    processes: "Parent agent and proxy stay on host. Child processes inherit OS restrictions; inherited secrets are not removed by default. Apple Events are blocked by default."
    coverage: "Shell tools and ordinary descendants only. Subagents share shell policy; built-in file/web tools, hooks, local/remote MCP, LSP and plugin helper code remain outside. Independent background sessions load their own settings."
    escape_hatches: "dangerouslyDisableSandbox tool parameter permits approved unsandboxed retries by default; excludedCommands exemptions also leave the boundary. Approval bypass does not disable the OS policy but can authorize retries and unknown hosts. Strict settings can prevent those paths; IPC and weaker-mode grants can defeat isolation."
    failure_behavior: "conditional"
    failure_notes: "Default unavailable-backend behavior permits unsandboxed execution. failIfUnavailable=true exits on failed startup checks or refuses sandboxed calls after later initialization failure. OS denials surface as operation errors; retry approval depends on mode."
    evidence_ids:
      - "e2"
      - "e6"
      - "e10"
  - id: "shell-bubblewrap"
    name: "Sandboxed Bash tool (Linux and WSL2)"
    enforcement: "os_sandbox"
    environments:
      - "linux"
      - "wsl2"
    backend: "Bubblewrap mount/PID/network namespaces, read-only and writable mounts, socat proxy relays; optional seccomp Unix-socket filtering. Exact CLI-embedded runtime revision unknown."
    prerequisites:
      - "bubblewrap"
      - "socat"
      - "Usable user namespaces and compatible AppArmor policy"
      - "Optional seccomp helper for Unix socket blocking; native CLI bundles ripgrep"
    activation: "Opt in through /sandbox or sandbox.enabled=true; not supported under WSL1."
    filesystem: "Reads broadly include home and credential files unless denied; writes are limited to working/additional directories, per-user temp and required shared Git metadata, with protected configuration writes denied. Symlink and rule behavior is version-sensitive. Mount-based denials and bounded nested protected-path discovery differ from macOS."
    network: "No unrestricted direct outbound route; host proxy checks allowed/denied domains. Unknown hosts may be approved unless strict allowlist applies. Built-in web tools and parent model/API traffic are outside this shell policy. UDP, ICMP and proxy-ignorant clients have no ordinary outbound route."
    processes: "Shell descendants inherit restrictions. Optional seccomp blocks Unix sockets; absent filter or allowAllUnixSockets can expose WSL Windows interoperability. Enabling nested user namespaces weakens filesystem containment."
    coverage: "Shell tools and ordinary descendants only. Subagents share shell policy; built-in file/web tools, hooks, local/remote MCP, LSP and plugin helper code remain outside. Independent background sessions load their own settings."
    escape_hatches: "dangerouslyDisableSandbox tool parameter permits approved unsandboxed retries by default; excludedCommands exemptions also leave the boundary. Approval bypass does not disable the OS policy but can authorize retries and unknown hosts. Strict settings can prevent those paths; IPC and weaker-mode grants can defeat isolation."
    failure_behavior: "conditional"
    failure_notes: "Missing dependencies/namespaces can cause unsandboxed fallback unless failIfUnavailable=true. Startup refusal differs from runtime read-only filesystem denial; headless calls still need their permission policy."
    evidence_ids:
      - "e2"
      - "e6"
      - "e9"
      - "e10"
  - id: "whole-process-runtime"
    name: "Standalone @anthropic-ai/sandbox-runtime"
    enforcement: "os_sandbox"
    environments:
      - "macos"
      - "linux"
      - "wsl2"
    backend: "Experimental srt 0.0.81 using Seatbelt or bubblewrap; distinct from the CLI-embedded shell sandbox."
    prerequisites:
      - "Node.js >=22.12.0"
      - "Install @anthropic-ai/sandbox-runtime outside writable roots"
      - "Linux/WSL2 require bubblewrap, socat and ripgrep; existing write targets"
      - "Explicit settings granting required project/state/temp and API access"
    activation: "Optional official integration: srt --settings FILE claude; ~/.srt-settings.json is the default config."
    filesystem: "Reads broadly allowed unless narrowed; explicit write grants for project, CLI state and temp. Mandatory protected writes differ by backend; Linux discovery is bounded and does not cover all future nested files."
    network: "Outer HTTP/SOCKS proxy restricts all descendant traffic including model/auth transport; necessary endpoints must be allowed. Readable context still leaves through approved inference transport."
    processes: "CLI and ordinary local descendants run inside; proxy remains outside. Writable state can persist changes affecting a later unsandboxed launch."
    coverage: "Includes built-in tools, subagents, local hooks/plugins/LSP and stdio MCP descendants. Remote MCP service actions remain outside."
    escape_hatches: "Inner CLI permission bypass does not remove outer OS policy. Broad grants, service access, writable launcher code and later unsandboxed launches remain risks."
    failure_behavior: "conditional"
    failure_notes: "Absent default settings file starts with restricted built-in writes and no network; explicit missing or invalid settings file refuses launch. No runtime tests performed; embedded CLI revision remains unknown."
    evidence_ids:
      - "e5"
      - "e10"
  - id: "runtime-windows-alpha"
    name: "Standalone sandbox-runtime Windows alpha"
    enforcement: "os_sandbox"
    environments:
      - "windows"
    backend: "Runtime 0.0.81 documents dedicated srt-sandbox account, restricted token, job object, NTFS ACLs and Windows Filtering Platform egress fence; end-to-end Claude Code compatibility unknown."
    prerequisites:
      - "Node.js >=22.12.0"
      - "Standalone runtime installation"
      - "One-time elevated windows-install setup with UAC and account/ACL changes"
    activation: "Experimental standalone opt-in, not native Claude Code /sandbox; not installed or tested in this research."
    filesystem: "Dedicated account and ACL grants/denies; default installer profile-read grants have credential exclusions. Exact Claude Code state/auth requirements remain unverified."
    network: "WFP fences sandbox-account connections to proxy ports; documented alternate-token and system-service residuals remain. Parent inference traffic would also need permission."
    processes: "Dedicated sandbox account and job object, unlike the host-user CLI; source-documented limitations do not establish tested containment."
    coverage: "Intended outer process-tree boundary; native Claude Code, its file tools, hooks and MCP compatibility require disposable Windows validation."
    escape_hatches: "Documented token/service residuals; installation changes persistent accounts and some grants. Does not make built-in Windows shell sandbox available."
    failure_behavior: "conditional"
    failure_notes: "README documents startup setup/fence validation and refusal when invalid. Actual Claude Code startup, auth and subprocess behavior are untested."
    evidence_ids:
      - "e2"
      - "e10"
  - id: "reference-devcontainer"
    name: "Claude Code reference dev container"
    enforcement: "container"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "Docker development container with optional reference iptables/ipset firewall; node user, writable project bind mount and config volume."
    prerequisites:
      - "Docker and compatible Dev Containers tooling"
      - "Supported CLI account/provider authentication"
      - "NET_ADMIN and NET_RAW for reference firewall; not required by CLI itself"
    activation: "Copy official reference .devcontainer files or install official feature; feature alone does not supply reference firewall."
    filesystem: "Only container-visible mounts/state are available; project bind mount is writable. Config volume persists authentication/settings. Injected secrets remain readable inside."
    network: "Reference default-drop firewall admits named destination IPs plus DNS, TCP 22, loopback and host-network traffic. API transport must remain reachable."
    processes: "Whole CLI and ordinary descendants use container environment; exposed mounts, credentials and services determine host reach."
    coverage: "Shell, file tools, subagents, hooks, extensions and local MCP inside container; remote services outside."
    escape_hatches: "Approval bypass cannot remove outer mounts/network boundary, but writable host bind mounts and deliberately exposed services remain accessible."
    failure_behavior: "conditional"
    failure_notes: "Container startup and firewall setup depend on deployment; no automatic conversion of an ordinary host CLI session into a container. Template source inspected, not built or run."
    evidence_ids:
      - "e5"
      - "e11"
      - "e15"
  - id: "hosted-cloud"
    name: "Anthropic-hosted cloud session"
    enforcement: "virtual_machine"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "CLI delegates execution to an isolated Anthropic-managed VM with network and Git credential proxies; host environments listed are launch clients."
    prerequisites:
      - "Eligible Claude subscription and organizational cloud-session permission"
      - "GitHub access for clone/push, or supported CLI local repository bundle upload"
    activation: "Optional claude --cloud handoff; self-hosted --environment is deployment-owned isolation instead."
    filesystem: "Remote VM filesystem instead of arbitrary local home access; uploaded/cloned code and session state accessible. Git credentials/signing keys remain outside VM."
    network: "Default restricted egress can be disabled for tools, but Anthropic API transport remains; cloud environment policy differs from local sandbox allowedDomains."
    processes: "Entire remote session executes in VM; self-hosted deployments supply their own isolation and credentials."
    coverage: "Remote shell/file tools, subagents, hooks and local-to-VM MCP descendants within VM; external MCP actions outside."
    escape_hatches: "Approval changes do not remove VM; --teleport resumes locally under local protections. Network/API access can disclose readable data."
    failure_behavior: "conditional"
    failure_notes: "Cloud startup needs account/network/policy eligibility. Self-hosted isolation is not supplied by the hosted-VM guarantee; no cloud session was launched."
    evidence_ids:
      - "e5"
      - "e16"
controls:
  - id: "sandbox-panel"
    kind: "session_command"
    name: "/sandbox"
    values: []
    effect: "Interactive opt-in, approval mode selection, strict retry control and resolved configuration/dependency inspection."
    example: "/sandbox"
    precedence: "Saves project-local settings; higher-priority policy may prevent changes. Not a headless configuration interface."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
  - id: "enable"
    kind: "config_key"
    name: "sandbox.enabled"
    values:
      - "true"
      - "false"
    effect: "Default false; enables optional shell sandbox on supported platforms."
    example: "{\"sandbox\":{\"enabled\":true}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "unavailable"
    kind: "config_key"
    name: "sandbox.failIfUnavailable"
    values:
      - "true"
      - "false"
    effect: "Default false; true refuses startup when unavailable, or refuses shell calls after later initialization failure."
    example: "{\"sandbox\":{\"failIfUnavailable\":true}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "auto-allow"
    kind: "config_key"
    name: "sandbox.autoAllowBashIfSandboxed"
    values:
      - "true"
      - "false"
    effect: "Default true; approval shortcuts for sandboxed Bash, with deny/ask/plan/Monitor exceptions. OS policy unchanged."
    example: "{\"sandbox\":{\"autoAllowBashIfSandboxed\":false}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "retry"
    kind: "config_key"
    name: "sandbox.allowUnsandboxedCommands"
    values:
      - "true"
      - "false"
    effect: "Default true; false ignores model-requested unsandboxed retry parameter. Trusted exclusions are separate."
    example: "{\"sandbox\":{\"allowUnsandboxedCommands\":false}}"
    precedence: "False from user/CLI/managed scope holds over repository true from 2.1.285. Managed/CLI false makes sandbox admin-required; user false alone does not. Exclusions remain separate."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "exclusions"
    kind: "config_key"
    name: "sandbox.excludedCommands"
    values:
      - "array of command patterns"
    effect: "Relaxes isolation: matching whole calls run outside filesystem/network boundary; compound-call matching rules apply."
    example: "{\"sandbox\":{\"excludedCommands\":[\"docker compose *\"]}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "fs-disable"
    kind: "config_key"
    name: "sandbox.filesystem.disabled"
    values:
      - "true"
      - "false"
    effect: "Relaxes isolation: disables filesystem layer and protected paths while preserving network filtering."
    example: "{\"sandbox\":{\"filesystem\":{\"disabled\":true}}}"
    precedence: "Managed filesystem policy or credential-file deny prevents lower scopes disabling filesystem isolation; otherwise documented source rules apply."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "write-grants"
    kind: "config_key"
    name: "sandbox.filesystem.allowWrite"
    values:
      - "array of sandbox paths"
    effect: "Adds shell writable roots; broad grants can expose persistent code/configuration."
    example: "{\"sandbox\":{\"filesystem\":{\"allowWrite\":[\"/tmp/build\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "write-denies"
    kind: "config_key"
    name: "sandbox.filesystem.denyWrite"
    values:
      - "array of sandbox paths"
    effect: "Denies writes within otherwise writable regions."
    example: "{\"sandbox\":{\"filesystem\":{\"denyWrite\":[\"./secrets\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "read-denies"
    kind: "config_key"
    name: "sandbox.filesystem.denyRead"
    values:
      - "array of sandbox paths"
    effect: "Restricts broad default shell reads; does not restrict Read tool."
    example: "{\"sandbox\":{\"filesystem\":{\"denyRead\":[\"~/.ssh\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "read-grants"
    kind: "config_key"
    name: "sandbox.filesystem.allowRead"
    values:
      - "array of sandbox paths"
    effect: "Reopens a narrower region inside read denies, subject to managed/read-block locks."
    example: "{\"sandbox\":{\"filesystem\":{\"allowRead\":[\"./src\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "domains"
    kind: "config_key"
    name: "sandbox.network.allowedDomains"
    values:
      - "array of host patterns with optional ports"
    effect: "Adds tool network destinations; does not constrain parent inference or WebFetch."
    example: "{\"sandbox\":{\"network\":{\"allowedDomains\":[\"registry.npmjs.org\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "domain-denies"
    kind: "config_key"
    name: "sandbox.network.deniedDomains"
    values:
      - "array of host patterns with optional ports"
    effect: "Blocks destinations even under broader wildcard grants."
    example: "{\"sandbox\":{\"network\":{\"deniedDomains\":[\"example.com\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "strict-domains"
    kind: "config_key"
    name: "sandbox.network.strictAllowlist"
    values:
      - "true"
      - "false"
    effect: "Requires allowlist match in every approval mode; repository settings cannot enable it."
    example: "{\"sandbox\":{\"network\":{\"strictAllowlist\":true}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "managed-domains"
    kind: "managed_policy"
    name: "sandbox.network.allowManagedDomainsOnly"
    values:
      - "true"
      - "false"
    effect: "Managed-only lock restricts network grants and proxy-port authority to managed settings."
    example: "{\"sandbox\":{\"network\":{\"allowManagedDomainsOnly\":true}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "managed-reads"
    kind: "managed_policy"
    name: "sandbox.filesystem.allowManagedReadPathsOnly"
    values:
      - "true"
      - "false"
    effect: "Managed-only read allowlist lock; also changes read-block shell interaction."
    example: "{\"sandbox\":{\"filesystem\":{\"allowManagedReadPathsOnly\":true}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "unix-paths"
    kind: "config_key"
    name: "sandbox.network.allowUnixSockets"
    values:
      - "array of Unix socket paths"
    effect: "Relaxes macOS IPC access; ignored on Linux/WSL2. Docker socket can effectively expose the host."
    example: "{\"sandbox\":{\"network\":{\"allowUnixSockets\":[\"/var/run/docker.sock\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "unix-all"
    kind: "config_key"
    name: "sandbox.network.allowAllUnixSockets"
    values:
      - "true"
      - "false"
    effect: "Relaxes IPC broadly; WSL Windows binary interop can become available."
    example: "{\"sandbox\":{\"network\":{\"allowAllUnixSockets\":true}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "local-bind"
    kind: "config_key"
    name: "sandbox.network.allowLocalBinding"
    values:
      - "true"
      - "false"
    effect: "macOS only: permits local listening and direct localhost connections; non-loopback binding admits remote clients. No effect on Linux/WSL2 private loopback."
    example: "{\"sandbox\":{\"network\":{\"allowLocalBinding\":true}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "mach-services"
    kind: "config_key"
    name: "sandbox.network.allowMachLookup"
    values:
      - "array of Mach service names"
    effect: "macOS service access can delegate work beyond sandbox."
    example: "{\"sandbox\":{\"network\":{\"allowMachLookup\":[\"com.apple.trustd.agent\"]}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "apple-events"
    kind: "config_key"
    name: "sandbox.allowAppleEvents"
    values:
      - "true"
      - "false"
    effect: "Relaxes macOS code-execution isolation by permitting unsandboxed app launches. Repository cannot enable it."
    example: "{\"sandbox\":{\"allowAppleEvents\":true}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "nested-namespaces"
    kind: "config_key"
    name: "sandbox.allowNestedUserNamespaces"
    values:
      - "true"
      - "false"
    effect: "Relaxes Linux/WSL2 containment; permits namespace operations that can defeat filesystem protections."
    example: "{\"sandbox\":{\"allowNestedUserNamespaces\":true}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "weak-container"
    kind: "config_key"
    name: "sandbox.enableWeakerNestedSandbox"
    values:
      - "true"
      - "false"
    effect: "Weaker Linux sandbox for constrained container environments."
    example: "{\"sandbox\":{\"enableWeakerNestedSandbox\":true}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "weak-network"
    kind: "config_key"
    name: "sandbox.enableWeakerNetworkIsolation"
    values:
      - "true"
      - "false"
    effect: "Relaxes macOS trust-service isolation for compatibility."
    example: "{\"sandbox\":{\"enableWeakerNetworkIsolation\":true}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "tls"
    kind: "config_key"
    name: "sandbox.network.tlsTerminate"
    values:
      - "object"
    effect: "Experimental TLS termination for credential injection, without content filtering; trusted scopes only."
    example: "{\"sandbox\":{\"network\":{\"tlsTerminate\":{}}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "http-port"
    kind: "config_key"
    name: "sandbox.network.httpProxyPort"
    values:
      - "localhost port number"
    effect: "Replaces built-in proxy for HTTP traffic; custom proxy becomes responsible for filtering."
    example: "{\"sandbox\":{\"network\":{\"httpProxyPort\":8080}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "socks-port"
    kind: "config_key"
    name: "sandbox.network.socksProxyPort"
    values:
      - "localhost port number"
    effect: "Replaces built-in SOCKS proxy; custom proxy becomes responsible for filtering."
    example: "{\"sandbox\":{\"network\":{\"socksProxyPort\":8081}}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "bwrap-path"
    kind: "managed_policy"
    name: "sandbox.bwrapPath"
    values:
      - "absolute executable path"
    effect: "Managed-only Linux/WSL2 backend executable selection instead of PATH."
    example: "{\"sandbox\":{\"bwrapPath\":\"/opt/admin/bwrap\"}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "socat-path"
    kind: "managed_policy"
    name: "sandbox.socatPath"
    values:
      - "absolute executable path"
    effect: "Managed-only Linux/WSL2 socat executable selection instead of PATH."
    example: "{\"sandbox\":{\"socatPath\":\"/opt/admin/socat\"}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "credential-files"
    kind: "config_key"
    name: "sandbox.credentials.files"
    values:
      - "array of {path, mode: deny|mask, optional masking fields}"
    effect: "Deny file reads or substitute Linux masked contents; macOS masks become denies with filesystem isolation. No default secret deny list."
    example: "{\"sandbox\":{\"credentials\":{\"files\":[{\"path\":\"~/.ssh\",\"mode\":\"deny\"}]}}}"
    precedence: "Deny entries accumulate; masking authorization only user/managed/CLI. File protection depends on filesystem layer; masking has documented no-match/fallback conditions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "credential-env"
    kind: "config_key"
    name: "sandbox.credentials.envVars"
    values:
      - "array of {name, mode: deny|mask, optional injectHosts and masking fields}"
    effect: "Removes named secrets or substitutes per-session placeholders; masked injection needs TLS termination and allowed destinations."
    example: "{\"sandbox\":{\"credentials\":{\"envVars\":[{\"name\":\"GITHUB_TOKEN\",\"mode\":\"deny\"}]}}}"
    precedence: "Deny wins across scopes; mask authorization only user/managed/CLI. Excluding sources has documented credential-specific exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "file-read-block"
    kind: "config_key"
    name: "permissions.blockReadsOutsideWorkingDirectories"
    values:
      - "true"
      - "false"
    effect: "Rejects outside file-tool reads even in bypass mode, with own-file exceptions. Also denies selected user-data roots to sandboxed shells, except documented cases. Available from 2.1.257."
    example: "{\"permissions\": {\"blockReadsOutsideWorkingDirectories\": true}}"
    precedence: "Read block true in any source holds; added readable directories and repository grants have additional trust restrictions. File-tool controls are distinct from OS policy."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e6"
  - id: "file-write-block"
    kind: "config_key"
    name: "permissions.blockWritesOutsideWorkingDirectories"
    values:
      - "true"
      - "false"
    effect: "Rejects outside file-tool writes even in bypass mode, with own-file exceptions; does not limit hooks, MCP or shell commands. Available from 2.1.314."
    example: "{\"permissions\": {\"blockWritesOutsideWorkingDirectories\": true}}"
    precedence: "Read block true in any source holds; added readable directories and repository grants have additional trust restrictions. File-tool controls are distinct from OS policy."
    mechanism_ids: []
    evidence_ids:
      - "e6"
  - id: "settings"
    kind: "cli_flag"
    name: "--settings"
    values:
      - "path to JSON settings file"
      - "inline JSON object"
    effect: "Loads session settings, including shell sandbox policy."
    example: "claude --settings policy.json"
    precedence: "Session-only; below managed and above ordinary files."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e8"
      - "e13"
  - id: "sources"
    kind: "cli_flag"
    name: "--setting-sources"
    values:
      - "comma-separated user,project,local"
    effect: "Chooses ordinary settings sources; managed policy and special credential retention rules still apply."
    example: "claude --setting-sources user"
    precedence: "Does not remove managed policy. Source exclusion behavior changed in 2.1.246."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e8"
      - "e13"
  - id: "additional-dirs"
    kind: "cli_flag"
    name: "--add-dir"
    values:
      - "directories..."
    effect: "Adds working/tool roots and normal shell sandbox write grants; read-block and admin-required policy impose source restrictions."
    example: "claude --add-dir /absolute/path/shared"
    precedence: "CLI scope; /add-dir and permissions.additionalDirectories are related surfaces described in prose."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
      - "e13"
  - id: "additional-dir-setting"
    kind: "config_key"
    name: "permissions.additionalDirectories"
    values:
      - "array of directory paths"
    effect: "Adds roots for file tools and normal sandbox writes; repository-supplied entries cannot widen some locked sandbox policies."
    example: "{\"permissions\":{\"additionalDirectories\":[\"/absolute/path/shared\"]}}"
    precedence: "Managed > --settings > local > project > user for scalar keys; lists merge and security-specific source locks apply. See prose for exceptions."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "additional-dir-session"
    kind: "session_command"
    name: "/add-dir"
    values:
      - "directory path"
    effect: "Adds a directory during an interactive session."
    example: "/add-dir /absolute/path/shared"
    precedence: "Session grant; admin-required sandbox policy can limit its resulting write grants."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e6"
  - id: "permission-mode"
    kind: "cli_flag"
    name: "--permission-mode"
    values:
      - "manual"
      - "acceptEdits"
      - "auto"
      - "bypassPermissions"
      - "dontAsk"
      - "plan"
    effect: "Changes tool approval, not OS isolation. Mode determines new-domain and unsandboxed-retry decisions unless stricter sandbox policy holds."
    example: "claude --permission-mode dontAsk"
    precedence: "Explicit launch choice overrides starting default; managed restrictions and hard sandbox settings still apply."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "default-mode"
    kind: "config_key"
    name: "permissions.defaultMode"
    values:
      - "default"
      - "manual"
      - "acceptEdits"
      - "auto"
      - "bypassPermissions"
      - "dontAsk"
      - "plan"
    effect: "Selects saved starting approval mode without enabling OS sandboxing."
    example: "{\"permissions\":{\"defaultMode\":\"dontAsk\"}}"
    precedence: "Explicit launch flag takes priority; repository Auto and bypass values have documented special handling. Managed restrictions and mode eligibility still apply."
    mechanism_ids: []
    evidence_ids:
      - "e3"
      - "e6"
  - id: "bypass"
    kind: "cli_flag"
    name: "--dangerously-skip-permissions"
    values: []
    effect: "Approval bypass; does not itself disable enabled sandbox, but can authorize retries and unknown domains."
    example: "claude --dangerously-skip-permissions"
    precedence: "Managed bypass prohibition, responsibility-acceptance context and independent sandbox locks still apply."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "enable-bypass"
    kind: "cli_flag"
    name: "--allow-dangerously-skip-permissions"
    values: []
    effect: "Makes bypass mode selectable without activating it by default."
    example: "claude --allow-dangerously-skip-permissions"
    precedence: "Launch-only capability; managed restriction still applies. Not an isolation switch."
    mechanism_ids: []
    evidence_ids:
      - "e13"
  - id: "ban-bypass"
    kind: "managed_policy"
    name: "permissions.disableBypassPermissionsMode"
    values:
      - "disable"
    effect: "Prevents bypass approval mode; does not itself enable a sandbox."
    example: "{\"permissions\":{\"disableBypassPermissionsMode\":\"disable\"}}"
    precedence: "Deliver through managed settings for an administrative restriction."
    mechanism_ids: []
    evidence_ids:
      - "e14"
      - "e3"
  - id: "retry-ask"
    kind: "config_key"
    name: "permissions.ask"
    values:
      - "Bash(dangerouslyDisableSandbox:true)"
    effect: "Can force an unsandboxed retry approval even in Auto and bypass mode."
    example: "{\"permissions\":{\"ask\":[\"Bash(dangerouslyDisableSandbox:true)\"]}}"
    precedence: "Ask rule takes precedence over matching allow rule; allowUnsandboxedCommands=false still disables the retry entirely."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
  - id: "tool-allow"
    kind: "cli_flag"
    name: "--allowedTools"
    values:
      - "tool names or scoped permission patterns"
    effect: "Preapproves tool calls; matching Bash allow rules can also approve an unsandboxed retry."
    example: "claude -p --allowedTools \"Bash(npm test)\" \"Run npm test\""
    precedence: "Permission rules, modes and managed policy apply; an allow rule is not OS containment."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e3"
      - "e13"
  - id: "tool-deny"
    kind: "cli_flag"
    name: "--disallowedTools"
    values:
      - "tool names or scoped permission patterns"
    effect: "Restricts available tool calls; application enforcement separate from sandbox."
    example: "claude --disallowedTools Bash"
    precedence: "Launch tool restriction; does not place remaining tools behind an OS boundary."
    mechanism_ids: []
    evidence_ids:
      - "e13"
  - id: "tool-set"
    kind: "cli_flag"
    name: "--tools"
    values:
      - "default"
      - "empty string"
      - "list of built-in tool names"
    effect: "Selects available built-in tools; can restore tools normally removed by --restricted."
    example: "claude -p --tools \"Read\" \"Summarize README.md\""
    precedence: "Explicit tool-set selection; managed restrictions still apply. Not OS isolation."
    mechanism_ids: []
    evidence_ids:
      - "e13"
  - id: "prompt-routing"
    kind: "cli_flag"
    name: "--permission-prompts"
    values:
      - "host"
      - "none"
    effect: "none denies requests that would prompt, while permission mode still decides other actions."
    example: "claude -p --permission-mode dontAsk --permission-prompts none \"Summarize README.md\""
    precedence: "Headless approval routing only; not an OS enforcement control."
    mechanism_ids: []
    evidence_ids:
      - "e13"
  - id: "restricted"
    kind: "cli_flag"
    name: "--restricted"
    values: []
    effect: "Removes code-running tools/WebFetch unless restored, confines file tools, ignores ordinary settings and refuses bypass. Not an OS jail."
    example: "claude -p --restricted --strict-mcp-config \"Summarize README.md\""
    precedence: "Managed settings and --settings remain; --strict-mcp-config separately avoids ordinary MCP configuration."
    mechanism_ids: []
    evidence_ids:
      - "e3"
      - "e13"
  - id: "restricted-env"
    kind: "environment_variable"
    name: "CLAUDE_CODE_RESTRICTED"
    values:
      - "1"
    effect: "Equivalent to --restricted; available from 2.1.248."
    example: "CLAUDE_CODE_RESTRICTED=1 claude -p \"Summarize README.md\""
    precedence: "Ignored in settings env blocks; not an OS sandbox."
    mechanism_ids: []
    evidence_ids:
      - "e7"
  - id: "strict-mcp"
    kind: "cli_flag"
    name: "--strict-mcp-config"
    values: []
    effect: "Loads only explicit --mcp-config servers; complements restricted mode without containing server processes."
    example: "claude -p --restricted --strict-mcp-config \"Summarize README.md\""
    precedence: "Launch-only MCP source control; local servers still outside built-in shell sandbox."
    mechanism_ids: []
    evidence_ids:
      - "e13"
  - id: "safe-mode"
    kind: "cli_flag"
    name: "--safe-mode"
    values: []
    effect: "Troubleshooting switch disables customizations; does not establish isolation."
    example: "claude --safe-mode"
    precedence: "Managed policy remains; built-in tools and permissions still operate normally."
    mechanism_ids: []
    evidence_ids:
      - "e13"
  - id: "env-scrub"
    kind: "environment_variable"
    name: "CLAUDE_CODE_SUBPROCESS_ENV_SCRUB"
    values:
      - "1"
    effect: "Strips recognized credentials from child environments, including hooks and stdio MCP; leaves GitHub tokens/proxy settings. Linux typed-command strict-mode coverage also changes."
    example: "CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1 claude"
    precedence: "Does not scrub files or the parent. Separate from sandbox.credentials rules."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
      - "e7"
  - id: "tmpdir"
    kind: "environment_variable"
    name: "CLAUDE_CODE_TMPDIR"
    values:
      - "directory path"
    effect: "Overrides internal temp base, with per-user suffix and possible short shell fallback; does not itself isolate temp."
    example: "CLAUDE_CODE_TMPDIR=/absolute/path/tmp claude"
    precedence: "Shell/user/managed setting supported; project/local env settings ignored."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e7"
  - id: "config-dir"
    kind: "environment_variable"
    name: "CLAUDE_CONFIG_DIR"
    values:
      - "directory path"
    effect: "Relocates CLI state/configuration and relevant protected paths; useful for container persistence."
    example: "CLAUDE_CONFIG_DIR=/absolute/path/claude-config claude"
    precedence: "Launch environment sets configuration root; no isolation by relocation alone."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
      - "whole-process-runtime"
      - "reference-devcontainer"
    evidence_ids:
      - "e2"
      - "e7"
      - "e15"
  - id: "http-proxy"
    kind: "environment_variable"
    name: "HTTP_PROXY"
    values:
      - "http:// or https:// proxy URL"
    effect: "Routes outbound traffic through an upstream corporate proxy; built-in sandbox allow checks still apply unless custom local proxy ports replace them."
    example: "HTTP_PROXY=http://proxy.example:8080 claude"
    precedence: "Launch environment or settings env; custom proxy-port configuration has separate responsibility/scope rules."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
  - id: "https-proxy"
    kind: "environment_variable"
    name: "HTTPS_PROXY"
    values:
      - "http:// or https:// proxy URL"
    effect: "Routes outbound traffic through an upstream corporate proxy; built-in sandbox allow checks still apply unless custom local proxy ports replace them."
    example: "HTTPS_PROXY=http://proxy.example:8080 claude"
    precedence: "Launch environment or settings env; custom proxy-port configuration has separate responsibility/scope rules."
    mechanism_ids:
      - "shell-seatbelt"
      - "shell-bubblewrap"
    evidence_ids:
      - "e2"
  - id: "srt-launch"
    kind: "cli_flag"
    name: "srt --settings"
    values:
      - "runtime settings file path"
    effect: "Outer whole-process sandbox uses its own filesystem/network schema, not Claude Code sandbox nesting."
    example: "srt --settings /absolute/path/srt-settings.json claude"
    precedence: "External runtime settings; explicit missing/invalid file refuses launch. Does not use Claude Code settings precedence."
    mechanism_ids:
      - "whole-process-runtime"
      - "runtime-windows-alpha"
    evidence_ids:
      - "e5"
      - "e10"
  - id: "cloud-launch"
    kind: "cli_flag"
    name: "--cloud"
    values:
      - "description"
      - "session ID"
      - "session URL"
    effect: "Starts or attaches to a hosted session; requires eligible subscription/policy. Changes execution location."
    example: "claude --cloud \"Run the project tests\""
    precedence: "Cloud environment and organization policy govern; not an enable switch for local sandbox."
    mechanism_ids:
      - "hosted-cloud"
    evidence_ids:
      - "e16"
      - "e13"
  - id: "self-hosted"
    kind: "cli_flag"
    name: "--environment"
    values:
      - "environment ID"
    effect: "Selects self-hosted cloud execution whose isolation is the deployment responsibility."
    example: "claude --environment ccpool_example"
    precedence: "Replaces assumption of Anthropic-hosted VM isolation; environment deployment must be reviewed."
    mechanism_ids:
      - "hosted-cloud"
    evidence_ids:
      - "e13"
      - "e16"
  - id: "teleport"
    kind: "cli_flag"
    name: "--teleport"
    values:
      - "session identifier"
    effect: "Resumes cloud work locally; VM boundary is not preserved on host."
    example: "claude --teleport session-id"
    precedence: "Local launch settings/permissions govern resumed local execution."
    mechanism_ids:
      - "hosted-cloud"
    evidence_ids:
      - "e13"
      - "e16"
gaps:
  - location: "mechanisms/shell-seatbelt/backend"
    detail: "Exact runtime revision embedded in Claude Code 2.1.318 is unknown; source inspection does not prove installed enforcement."
    next_check: "Obtain upstream build manifest/SBOM, then test the documented boundary in disposable macOS environments."
  - location: "mechanisms/shell-bubblewrap/backend"
    detail: "Exact embedded runtime revision and live namespace/socket/headless behavior were not verified."
    next_check: "Obtain build metadata and run disposable Linux/WSL2 deny, symlink, socket, unavailable-backend and retry tests; verify native Windows failIfUnavailable startup refusal separately."
  - location: "mechanisms/runtime-windows-alpha"
    detail: "Standalone 0.0.81 Windows alpha is source documented; end-to-end native Claude Code startup/auth/tool compatibility remains unknown."
    next_check: "Use disposable Windows VM, review installer ACL/account changes, then test CLI 2.1.318 startup, credential access, filesystem denial, child coverage and documented network residuals."
changes: []
---

# Claude Code sandboxing

Research checked **2026-11-04 UTC**. A normal local Claude Code installation supplies **no enabled OS sandbox**. Its optional built-in sandbox contains shell tool commands and their descendants, not the agent process. Reads remain broad even after enabling it. For a boundary around file tools, hooks, and local MCP servers as well, Anthropic documents launching the whole CLI through its experimental sandbox runtime, a dev container, or a cloud session. These are separate deployment choices, not clean-install defaults. [e1], [e2], [e5]

## What was examined

The starting point was the [official overview](https://code.claude.com/docs/en/overview), followed by the current sandboxing, permissions, settings, deployment, and CLI documentation. Read-only local inspection on macOS found `/Users/ken/.local/bin/claude`, reporting **2.1.318 (Claude Code)** through both `sniff software agents --json` and `claude --version`. `claude --help` was also examined. No model session, sandbox probe, package installation, permission change, or credential inspection was performed. Existing user settings were not used to infer defaults. [e1], [e13]

Release notes were examined at Claude Code repository commit `26382c08758260c9f5deb2019d8fe851f89fa9ae`; the current top release was 2.1.318. Public sandbox-runtime source was examined at `98a4016d27609ed34dd1fb43d23ed6efb6eeeb77`, whose package version is **0.0.81**, also the npm `latest` version observed during research. That package requires Node.js **22.12.0 or later**. This establishes the standalone runtime's source behavior, not the exact runtime revision embedded in the installed native CLI. Claude Code's complete permission implementation was not source-audited. [e9], [e10], [e11], [e13]

## Defaults across platforms and session types

The following assumes no managed policy, saved permission choice, settings override, or command-line override. “Auto” means a classifier reviews actions; it does not mean sandboxed. Current interactive Auto defaults require an eligible model/session and can fall back to Manual. [e2], [e3]

| Platform | Interactive terminal, current CLI | Non-interactive `claude -p`, current CLI | Optional built-in shell sandbox |
| --- | --- | --- | --- |
| macOS | Sandbox off; Auto when available, otherwise Manual | Sandbox off; Manual; unanswered approval calls denied | Seatbelt; no separate sandbox package needed |
| Linux | Sandbox off; same permission defaults | Sandbox off; same headless defaults | Bubblewrap and socat; usable user namespaces required |
| Native Windows | No built-in shell isolation; same permission defaults | No built-in shell isolation; same headless defaults | Explicitly unsupported; shell commands run unsandboxed |
| WSL2 | Sandbox off; same permission defaults inside the distribution | Sandbox off; same headless defaults | Linux backend inside WSL2; not native Windows isolation |

The sandbox-off default and native Windows absence are explicit statements in the [sandbox guide](https://code.claude.com/docs/en/sandboxing#what-the-sandbox-restricts), not deductions from missing documentation. Current [permission-mode documentation](https://code.claude.com/docs/en/permission-modes#which-mode-a-session-starts-in) says interactive Auto became the built-in default across plans/providers in **2.1.283**; older versions had plan and feature-flag conditions. HIPAA-eligible sessions default to Manual from 2.1.285. An organization can disable Auto, and unsupported models or server-side availability can cause Manual fallback. **2.1.302 and later** explicitly default headless/SDK runs to Manual when no mode is set. Earlier defaults should not be inferred from this table. [e2], [e3]

First use involves authentication and interactive workspace trust. The local CLI supports Claude subscription/Console authentication and documented third-party inference providers; the shell sandbox itself has no separately documented paid entitlement. On the first session that uses the built-in Auto default, a notice explains the mode. A user with a saved non-Auto default may instead receive an opt-in question; declining preserves that choice. None of these enables the sandbox. `/sandbox` is a separate opt-in, offering automatic approval inside the boundary or regular permission checks. It normally saves the choice to `.claude/settings.local.json`; setting `sandbox.enabled` in user settings applies across projects. [e1], [e2], [e3]

Current Auto mode also has an interactive first-read choice for paths outside working directories: allow broadly, block future outside reads, or decide again next time. Choosing the persistent block sets `permissions.blockReadsOutsideWorkingDirectories: true`. The prompt does not occur in `-p` or background sessions. Thus a previous user's answer can materially change an installation without changing the clean-install default. `-p`, or non-TTY output, skips workspace trust according to locally observed help; this is not an isolation mechanism. [e3], [e13]

## The built-in boundary and what bypasses it

With `sandbox.enabled: true`, Bash, PowerShell, and Monitor tool commands run under OS restrictions, as do their ordinary child processes. On macOS the runtime generates a Seatbelt profile and invokes `/usr/bin/sandbox-exec`. On Linux/WSL2 it constructs bubblewrap mount, PID, and network namespaces, read-only mounts and writable exceptions, with socat relaying traffic to a host-side proxy. This is per-command isolation on the local kernel, not a VM containing Claude Code. The agent and proxy remain outside. [e2]; source-derived details: [e10]

```mermaid
flowchart TB
  Model[Model or API endpoint] <--> Agent[Claude Code on the host]
  Agent --> Files[Read / Edit / Write / WebFetch]
  Agent --> Helpers[Hooks / local MCP / LSP / plugin code]
  Agent --> Shell
  subgraph Boundary[Optional OS shell sandbox]
    Shell[Bash / PowerShell / Monitor] --> Children[Builds, scripts, ordinary descendants]
  end
  Children --> Proxy[Host-side domain proxy]
  Proxy --> Dest[Approved destinations]
```

The [documented scope](https://code.claude.com/docs/en/sandboxing#what-runs-outside-the-sandbox) is narrower than “Claude is sandboxed”:

| Surface | Under built-in shell sandbox? | Actual control |
| --- | --- | --- |
| Shell tools and ordinary descendants | Yes, when enabled and not exempted | OS filesystem/network policy plus tool permissions |
| Read, Edit, Write, search and built-in web tools | No | Claude Code's application permission checks |
| Subagents | Their shell calls share the parent's sandbox | Same-process subagents use the same sandbox configuration; other tools retain their normal scope |
| Hooks, local stdio MCP, LSP, status-line and authentication helpers | No | Host-user authority; separate configuration/approval controls |
| Plugins and extensions | Depends on execution surface | A skill's shell call uses shell policy; plugin hooks, monitors, and in-process mods do not acquire that boundary |
| Remote MCP services | No local process containment | Service-side authority and Claude Code's MCP permissions |
| Commands typed with `!` | Usually no | User-directed host execution; limited strict-mode exceptions below |

An ordinary shell child cannot opt out just by ignoring a prompt or proxy variable. However, allowed host services can execute work outside the boundary. Docker socket access effectively exposes host authority; macOS `allowAppleEvents: true` permits launching unsandboxed applications. Computer use operates on the real desktop. Independent background Claude sessions are separate processes and use their own loaded sandbox settings. Remote Control steers a local process; it does not move it into a remote sandbox. [e2], [e12]

Linux requires **bubblewrap and socat**. Native Claude Code bundles ripgrep. `/sandbox` reports missing dependencies and may show a missing optional seccomp component, installable through `@anthropic-ai/sandbox-runtime`. The optional filter blocks Unix socket creation; without it, do not claim Unix IPC isolation. Ubuntu's AppArmor user-namespace restriction may prevent startup; the guide gives an AppArmor profile for `/usr/bin/bwrap` and an alternative system policy change. WSL1 is unsupported. In WSL2, Windows executable launches use an interoperability Unix socket: blocking it requires the optional filter; `allowAllUnixSockets` permits those launches and much more. [e2]

## Files, credentials, processes, and network

### Filesystem access

Without the sandbox, OS access remains that of the host user. Tool prompts, read/write rules, and modes constrain what Claude Code dispatches; they do not create a read-only mount or separate credential store. In Manual mode, ordinary in-workspace reads and recognized read-only commands can run without approval; edits and most shell commands need approval. Auto substitutes classifier decisions for many prompts. Neither mode makes readable secrets confidential. [e3], [e4]

With the sandbox enabled, shell writes are allowed in the working directory, added directories (`--add-dir`, `/add-dir`, `permissions.additionalDirectories`), and a per-user temporary directory. Linked worktrees also receive access to the shared Git metadata needed for commits. **Reads normally cover most of the machine**, including `~/.ssh`, `~/.aws/credentials`, and inherited environment secrets. There is no built-in credential deny list. [e2]

Use `sandbox.filesystem.allowWrite`, `denyWrite`, `denyRead`, and `allowRead` to adjust shell access. Narrower read paths override broader ones: a project can be reopened under a home-directory read deny, while a narrower secret deny survives a broad read allowance. Filesystem path syntax differs from permission-rule syntax: `/tmp/build` is an absolute sandbox path, `~` means home, and relative paths resolve against the settings file's directory. In a project settings file, `.` means that project; in `~/.claude/settings.json`, it means `~/.claude`. [e2], [e6]

Protected writes remain denied inside writable regions. The CLI protects settings, skills, agents, commands, hooks, MCP configuration, relevant workflow/task files, Git hooks/configuration, shell startup files, and its configuration/credential store. The scope differs between the working directory, ancestors, and user configuration directory; it is not a universal ban on every similarly named file. `allowWrite` does not override these protections. The guide also documents cleanup of newly created top-level `HEAD`, `objects`, and `refs` entries, including deletion of some populated folders: these names have special bare-repository protections and should not be assumed safe scratch locations. [e2]

Symlinks do not establish a separate security boundary. Current macOS denials also cover resolved destinations, including supported wildcard-prefix cases, and settings symlink targets are reconsidered for subsequent commands. Release notes document fixes in **2.1.299** for denied paths through symlinks and **2.1.309** for write-deny globs at real destinations. Linux uses mount-based policy; the standalone runtime scans existing nested protected paths to a bounded depth rather than watching all future nested repositories. These are version-sensitive claims, not results of adversarial tests here. [e2], [e9], [e10]

`permissions.blockReadsOutsideWorkingDirectories: true` is an additional application control, available from **2.1.257**. It rejects outside file-tool reads even in bypass mode, with exceptions for files Claude needs. When filesystem sandboxing applies, it also denies shell reads under user-data roots such as `/Users`, `/home`, `/Volumes`, and `/mnt`, then reopens working directories, session temp, and needed configuration. It is not a blanket deny of the entire root filesystem. Shell enforcement has documented exceptions: disabled filesystem isolation, `allowManagedReadPathsOnly`, or a starting directory containing glob characters. Global Git configuration is reopened and may itself contain secrets. **2.1.314** adds `permissions.blockWritesOutsideWorkingDirectories` for file tools, with own-file exceptions; it does not constrain hooks, MCP, or shells. [e6]

### Secrets and temporary storage

For sandboxed commands, `sandbox.credentials.files` and `envVars` accept `mode: "deny"`: files become unreadable and variables are removed. `mode: "mask"` supplies placeholders and allows proxy-side substitution on allowed destinations. Masking requires `sandbox.network.tlsTerminate`; each `injectHosts` destination must also be network-allowed. Only user, managed, and `--settings` scopes may authorize masking/TLS termination. Linux file masks use substituted file contents; macOS file masks become read denies while filesystem isolation is enabled. File masking and AWS SigV4 handling require **2.1.221** or later; additional AWS pairing controls require 2.1.224. A pattern mask with the default no-match warning can leave a file unmasked: use a deny when failure to match must remain confidential. [e2], [e6]

`CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` separately strips recognized credentials from environments of subprocesses including hooks and stdio MCP. It deliberately leaves GitHub tokens and proxy settings; it does not remove credentials from files or the parent agent. `CLAUDE_CONFIG_DIR` relocates CLI configuration; `CLAUDE_CODE_TMPDIR` changes internal temp placement. Sandboxed shell `$TMPDIR` normally points to the writable per-user sandbox temp and may differ from unsandboxed `$TMPDIR`. A temp directory is writable storage, not a promise of isolation from every process of the same user. [e2], [e7]

### Network and process execution

Sandboxed tools have no unrestricted direct outbound route. Linux uses a network namespace; Seatbelt limits macOS network access. The host-side HTTP/SOCKS proxy evaluates `sandbox.network.allowedDomains` and `deniedDomains`; the allowed list starts empty and also incorporates supported `WebFetch(domain:...)` permission rules. Proxy-aware tools work once allowed. Tools that ignore the proxy, such as plain SSH or many database clients, fail even for allowed hosts until explicitly configured to use it. UDP/QUIC/ICMP do not get a direct outbound path. [e2]

An unknown hostname does not necessarily stay denied: interactive Manual/acceptEdits sessions can approve it, bypass mode ordinarily allows it, Auto can approve per-command requested domains from **2.1.271**, and dontAsk refuses it. `strictAllowlist: true` or managed `allowManagedDomainsOnly: true` prevents that widening; explicit denied domains remain denied. The proxy also checks resolved local addresses on direct hostname connections, with exceptions including explicit IP grants and upstream proxies. An allowed domain is not content-level authorization: broad GitHub access can still exfiltrate readable files. Default proxying does not inspect TLS payloads, and TLS termination alone adds no content filter. [e2], [e6]

This network boundary applies to **tool subprocess traffic**. The parent agent still communicates with Anthropic or its configured inference provider; Read output can enter those model requests. `allowedDomains: []` does not make the session offline or prevent model-mediated disclosure. External whole-process isolation must separately permit model/auth endpoints and still transmits supplied context. [e2], [e5]

`network.allowUnixSockets`, `allowAllUnixSockets`, `allowLocalBinding`, and macOS `allowMachLookup` open further IPC/server access. The per-path `allowUnixSockets` list applies only on macOS; Linux/WSL2 ignore it and use the all-or-nothing socket setting. `allowLocalBinding` also applies only on macOS: it permits listening on local addresses and direct localhost connections, including accepting other machines when binding a non-loopback address. Linux/WSL2 instead have private loopback interfaces. `allowAppleEvents` and Linux `allowNestedUserNamespaces` can undermine code-execution containment; the latter, added in **2.1.305**, permits operations that can defeat filesystem protections. `enableWeakerNestedSandbox` supports constrained container environments with weaker isolation. `enableWeakerNetworkIsolation` relaxes macOS trust-service restrictions for compatibility. These are security changes, not harmless fixes for a failing build. [e2], [e6], [e9], [e10]

## Configuration, approvals, and failures

For a local supported-platform session, this example enables the shell boundary, prevents startup fallback and model-requested unsandboxed retries, protects two credential paths, and admits one package registry. It still leaves file tools and hooks outside the OS boundary and keeps the ordinary broad read default for other shell paths. [e2], [e6], [e13]

```sh
claude --settings '{"sandbox":{"enabled":true,"failIfUnavailable":true,"allowUnsandboxedCommands":false,"filesystem":{"denyRead":["~/.ssh","~/.aws"]},"network":{"allowedDomains":["registry.npmjs.org"],"strictAllowlist":true}}}'
```

The following can be saved as a settings file and supplied with `--settings policy.json`. It retains normal permission approval for sandboxed calls and applies explicit file-tool boundaries as well. [e2], [e6]

With the default `autoAllowBashIfSandboxed: true`, matching deny rules and content-specific ask rules still apply. A bare Bash ask rule is normally skipped for sandboxed calls, but plan mode does not gain those approvals. Monitor calls remain sandboxed without receiving the same automatic approval. Certain interpreter/environment-assignment forms also return to the ordinary permission flow. These exceptions affect approval, not the OS boundary. [e2]

```json
{
  "sandbox": {
    "enabled": true,
    "failIfUnavailable": true,
    "autoAllowBashIfSandboxed": false,
    "allowUnsandboxedCommands": false,
    "network": { "allowedDomains": [], "strictAllowlist": true },
    "credentials": {
      "files": [{ "path": "~/.ssh", "mode": "deny" }],
      "envVars": [{ "name": "GITHUB_TOKEN", "mode": "deny" }]
    }
  },
  "permissions": {
    "blockReadsOutsideWorkingDirectories": true,
    "blockWritesOutsideWorkingDirectories": true
  }
}
```

For unattended execution, specify the approval behavior too; the sandbox does not answer every tool permission request:

```sh
claude -p --settings policy.json --permission-mode dontAsk \
  --permission-prompts none --allowedTools 'Bash(npm test)' \
  'Run npm test and summarize the result.'
```

Observed 2.1.318 help describes `--permission-prompts none` as denying anything that would prompt. Without a host callback/prompt tool, headless Manual calls needing approval are denied; allow rules, hooks, or an SDK permission callback can authorize them. Auto or bypass modes are explicit alternatives, not implicit consequences of `-p`. A headless run should preconfigure required domains and avoid dependence on an interactive network decision. [e3], [e13]

### Scope and administration

The documented settings precedence is managed policy, then `--settings`, project-local settings, shared project settings, and user settings. User settings are normally `~/.claude/settings.json`; project files are `.claude/settings.local.json` and `.claude/settings.json`. Lists generally merge; they are not simply replaced by the highest file. `--setting-sources user,project,local` selects ordinary sources. From **2.1.246**, excluded project/local sources no longer feed sandbox filesystem or credential rules; some user credential denies remain protective even when that user source is excluded. Environment variables have per-variable semantics rather than constituting a universal highest precedence tier. [e2], [e8]

Administrators can deploy the sandbox JSON through managed settings. File locations are `/Library/Application Support/ClaudeCode/managed-settings.json` on macOS, `/etc/claude-code/managed-settings.json` on Linux/WSL, and `C:\Program Files\ClaudeCode\managed-settings.json` on native Windows. Server-managed policy and OS management are other supported delivery routes. The Windows path does not imply Windows sandbox support: `failIfUnavailable: true` makes a required built-in sandbox refuse startup there. Managed-only `sandbox.bwrapPath` and `sandbox.socatPath` pin backend executables instead of trusting PATH. [e2], [e6], [e14]

From **2.1.285**, an admin-required sandbox ignores repository settings that loosen exclusions, network access, write grants, or certain read rules. It becomes admin-required through managed/CLI `allowUnsandboxedCommands: false`, or managed `allowManagedDomainsOnly: true`; these keys do not themselves enable it. User/CLI grants can still widen many lists unless a specific managed lock covers them. `allowManagedReadPathsOnly` and `allowManagedDomainsOnly` restrict their respective allowlists. Managed filesystem policy or credential-file denies also prevent a lower scope disabling the filesystem layer. There is no basis for treating one `enabled: true` as an immutable complete policy. [e2], [e6]

`sandbox.filesystem.disabled: true` deliberately leaves network isolation but removes filesystem isolation, including protected-write and file-deny enforcement. `autoAllowBashIfSandboxed` still defaults true, making this especially significant. `excludedCommands` sends matching calls outside both shell boundaries; compound calls require every command to match, and matching is not a transitive exemption for programs invoked inside scripts. A custom `httpProxyPort`/`socksProxyPort` replaces built-in network filtering for that traffic: the custom proxy must enforce the policy itself. `HTTPS_PROXY`/`HTTP_PROXY` instead configure an upstream corporate proxy behind the built-in allow checks. [e2], [e6]

### Denial, escape hatches, and approval bypass

OS-denied writes usually surface as `Operation not permitted` on macOS or `Read-only file system` on Linux/WSL2. Blocked network results identify the destination. Claude can adjust the command or request a retry with the **tool parameter** `dangerouslyDisableSandbox`; this is not a top-level CLI flag. With the default `allowUnsandboxedCommands: true`, approval can permit a completely unsandboxed retry. A matching Bash allow rule can approve that retry without a fresh prompt. Manual and acceptEdits otherwise prompt; bypass runs it without prompting; Auto evaluates it; dontAsk denies it. An ask rule `Bash(dangerouslyDisableSandbox:true)` can force a retry prompt even in bypass mode. `allowUnsandboxedCommands: false` disables the parameter, but trusted explicit exclusions remain possible. [e2]

When enabled sandboxing cannot initialize, the default `failIfUnavailable: false` allows unsandboxed execution. Set it true for fail-closed startup. Recent releases also have narrower command refusal paths reported as `Sandbox is required`; they refuse the retry parameter too. An unusable backend should not be confused with a successfully sandboxed command that gets an access error. Missing dependencies, unavailable namespaces, and container restrictions require repair or an explicit deployment change. `claude doctor` and `/sandbox` expose diagnostics; `/sandbox` is interactive and does not configure a headless run. [e2], [e6]

More precisely, `failIfUnavailable: true` exits when the startup platform/dependency check fails; if that check passes but later initialization fails, the session may remain open while each sandboxed command is refused. [e6]

“YOLO” is not the examined CLI's flag name. `--dangerously-skip-permissions`/`--permission-mode bypassPermissions` bypass approval checks; `--allow-dangerously-skip-permissions` only makes that mode available. These do **not themselves disable an already enabled OS sandbox**, but they can approve unknown hosts and unsandboxed retries unless the stricter controls above prevent them. The first interactive bypass launch asks for responsibility acceptance; ordinary `-p` shows no such dialog. Background bypass sessions have separate saved-acceptance requirements. `permissions.disableBypassPermissionsMode: "disable"` lets administrators forbid the mode. [e2], [e3], [e13], [e14]

Plan mode, tool allowlists/denylists, workspace trust, and `--restricted` are application controls. `--allowedTools`, `--disallowedTools`, and `--tools` select or preapprove tool calls rather than isolating execution. `permissions.defaultMode` selects a saved starting mode, subject to documented restrictions on repository-supplied Auto/bypass settings. `CLAUDE_CODE_RESTRICTED=1` is equivalent to `--restricted` and is ignored in a settings `env` block. Current `--restricted` removes code-running tools and WebFetch unless explicitly restored with `--tools`, confines file tools, ignores ordinary user/project/local settings, and refuses bypass mode; managed settings and `--settings` still apply. Add `--strict-mcp-config` to avoid ordinary configured MCP servers. It is available from **2.1.248**, not an OS jail. `--safe-mode` is a troubleshooting customization switch, also not a sandbox. [e3], [e7], [e13]

Commands a person types at `!` ordinarily remain unsandboxed even with strict retries disabled. Documented exceptions are strict background sessions and Linux sessions using `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`; prior to **2.1.260** strict mode had broader coverage of typed commands. Therefore “strict” must be read in its documented scope. [e2]

## Official optional deployments around the whole CLI

### Standalone Anthropic sandbox runtime

The [official deployment guide](https://code.claude.com/docs/en/sandbox-environments#sandbox-runtime) documents `npm install -g @anthropic-ai/sandbox-runtime`, configuring `~/.srt-settings.json` or `srt --settings FILE`, then launching `srt claude`. For a pinned examined version:

```sh
npm install -g @anthropic-ai/sandbox-runtime@0.0.81
srt --settings /absolute/path/srt-settings.json claude
```

Configure the runtime's own `filesystem` and `network` objects, without Claude Code's outer `sandbox` key. Allow the project, required CLI state paths, and runtime temp writes; permit the configured model endpoint and required login/refresh hosts. Linux/WSL2 need bubblewrap, socat, and ripgrep; create Linux write-grant targets first. Install the runtime outside writable project paths so sandboxed code cannot replace the next launcher. An absent default settings file starts with restricted built-in writes and blocked network; an explicitly missing, empty, unreadable, or invalid settings file refuses launch. Ordinary read access remains broad unless narrowed. [e5], [e10], [e11]

The outer boundary now covers file tools, hooks, and local MCP descendants too, and its network policy also affects the agent's API traffic. It does not contain a remote MCP service's own actions. Linux automatic protected-path discovery is bounded and does not protect all future nested files; state deliberately left writable can affect a later unsandboxed launch. The integration is experimental and may change. The pinned README's macOS dependency subsection still lists ripgrep, but the pinned `sandbox-manager.ts` explicitly checks it only on Linux and says macOS uses profile globs instead. This report follows that implementation and the official deployment guide; the README line appears stale. [e5], [e10]

There is a meaningful **native Windows distinction**: current Claude Code explicitly lacks a built-in Windows sandbox, but standalone runtime **0.0.81 has an alpha Windows backend**. Its source/README describe a dedicated `srt-sandbox` local account, write-restricted token, job object, NTFS ACLs, and a Windows Filtering Platform outbound fence. `windows-install` requires an elevated UAC setup step and makes substantial account/ACL changes, including default profile-read grants with credential exceptions. It also documents residual token and system-service network gaps. This research did not install or test it, and does not establish end-to-end Claude Code compatibility on that alpha backend. It is not evidence that `/sandbox` works on native Windows. [e2], [e10]; unresolved check G2

### Dev container and cloud sessions

Anthropic supplies a [reference dev container](https://code.claude.com/docs/en/devcontainer), requiring Docker and compatible Dev Containers tooling. It runs the entire local CLI inside the container, with the project bind-mounted writable and configuration persisted in a named volume. Authentication still needs a normal supported account/provider. The optional reference firewall requires `NET_ADMIN`/`NET_RAW`; merely installing the Dev Container Feature does not install that firewall policy. [e15]

Source inspection of the reference `init-firewall.sh` shows a default-drop policy with **exceptions beyond the named domain set**, including DNS, TCP port 22, loopback, and a host-network range. Thus “default-deny firewall” is not “all traffic is restricted to those HTTPS hostnames.” The reference uses the non-root `node` user and permits its firewall initialization through sudo. These observations describe the published template, not a built/tested container. Bind-mounted code and any injected credentials remain accessible inside it. [e11]

`claude --cloud` is an official CLI handoff to an Anthropic-hosted isolated VM, not an enhancement to local `/sandbox`. It requires a suitable Claude subscription: the current documentation lists Pro, Max, Ultra, Team, and eligible Enterprise seats, subject to organizational policy. GitHub connectivity supports clone/push, but CLI uploads can send a local repository without a GitHub connection. A network proxy constrains VM egress; Git credentials/signing keys remain outside the VM and are attached by a separate proxy. Even network-disabled cloud sessions retain Anthropic API transport. Self-hosted `--environment` execution instead depends on the organization's own isolation. `--teleport` returns work to a local execution context and should not be assumed to preserve the VM boundary. [e16]

## Evidence and remaining checks

Evidence records below were accessed **2026-11-04 UTC**. Documentation establishes vendor claims; source records establish implementation details at a pinned revision; local help establishes exposed options. None constitutes a runtime isolation test.

| ID | Evidence and scope |
| --- | --- |
| e1 | [Official overview](https://code.claude.com/docs/en/overview): local CLI installation/authentication and supported inference options. |
| e2 | [Sandbox guide](https://code.claude.com/docs/en/sandboxing): explicit off default, unsupported native Windows, boundaries, controls, dependencies, escape/failure behavior. |
| e3 | [Permission modes](https://code.claude.com/docs/en/permission-modes): current starting modes, version conditions, outside-read choice, bypass acceptance and headless behavior. |
| e4 | [Permissions](https://code.claude.com/docs/en/permissions): application tool rules, working directories and trust; distinct from OS isolation. |
| e5 | [Sandbox environments](https://code.claude.com/docs/en/sandbox-environments): officially documented whole-process runtime, containers, VMs and cloud choices. |
| e6 | [All settings](https://code.claude.com/docs/en/settings-reference): precise sandbox and permission key defaults, scopes, read-block exceptions and version requirements. |
| e7 | [Environment variables](https://code.claude.com/docs/en/env-vars): subprocess scrub, temp/config paths, restricted/safe mode variables. |
| e8 | [Settings files and precedence](https://code.claude.com/docs/en/settings): sources, scopes, list merging and precedence. |
| e9 | [Pinned Claude Code changelog](https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/CHANGELOG.md): 2.1.318 top release and historical sandbox fixes. |
| e10 | [Pinned runtime README](https://github.com/anthropic-experimental/sandbox-runtime/blob/98a4016d27609ed34dd1fb43d23ed6efb6eeeb77/README.md), [sandbox manager](https://github.com/anthropic-experimental/sandbox-runtime/blob/98a4016d27609ed34dd1fb43d23ed6efb6eeeb77/src/sandbox/sandbox-manager.ts), [macOS backend](https://github.com/anthropic-experimental/sandbox-runtime/blob/98a4016d27609ed34dd1fb43d23ed6efb6eeeb77/src/sandbox/macos-sandbox-utils.ts), [Linux backend](https://github.com/anthropic-experimental/sandbox-runtime/blob/98a4016d27609ed34dd1fb43d23ed6efb6eeeb77/src/sandbox/linux-sandbox-utils.ts), and [package manifest](https://github.com/anthropic-experimental/sandbox-runtime/blob/98a4016d27609ed34dd1fb43d23ed6efb6eeeb77/package.json): public runtime implementation and prerequisites, including Windows alpha claims. |
| e11 | [Pinned reference container configuration](https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/.devcontainer/devcontainer.json), [Dockerfile](https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/.devcontainer/Dockerfile), and [firewall](https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/.devcontainer/init-firewall.sh): mounts, capabilities and actual network exceptions. |
| e12 | [Remote Control](https://code.claude.com/docs/en/remote-control): local execution remains local. |
| e13 | Local read-only commands `sniff software agents --json`, `sniff os --json`, `claude --version`, and `claude --help`: macOS installation, 2.1.318, observed flag descriptions; no boundary probes. |
| e14 | [Managed settings](https://code.claude.com/docs/en/managed-settings): administrative delivery, OS file locations and bypass restrictions. |
| e15 | [Dev containers](https://code.claude.com/docs/en/devcontainer): official feature/reference setup, authentication, persistence and optional firewall. |
| e16 | [Cloud sessions](https://code.claude.com/docs/en/claude-code-on-the-web#security-and-isolation): CLI launch, eligibility, VM boundary, credential proxies and API transport exception. |

### Local observation

Sanitized excerpts captured from the local CLI on 2026-11-04 UTC:

```text
$ claude --version
2.1.318 (Claude Code)
$ claude --help
  -p, --print
  --settings <file-or-json>
  --permission-mode <mode>
    choices: acceptEdits, auto, bypassPermissions, manual, dontAsk, plan
  --permission-prompts <target>
    choices: host, none; default: host
  --restricted
  --strict-mcp-config
  --cloud [description|session_id|url]
```

The flag list is excerpted, not a complete help transcript. `sniff` independently reported the installed version and macOS host. No local configuration or secret values are reproduced.

**G1 — embedded revision and runtime enforcement:** the exact sandbox-runtime revision compiled into Claude Code 2.1.318 was not established. Obtain an upstream build manifest/SBOM and compare it with pinned runtime source. Then use disposable macOS, Linux, and WSL2 environments to test writes, read denies, symlink retargeting, socket access, startup fallback, headless domain denial, and a rejected unsandboxed retry; on native Windows verify required-sandbox startup refusal. This research used documentation/source inspection only.

**G2 — standalone Windows alpha:** establish whether `srt` 0.0.81 can run the native Claude Code 2.1.318 binary end to end. Test in a disposable Windows VM after reviewing the installer account/ACL changes, including credential availability, filesystem denies, subprocess coverage, and the README's documented network residuals. Keep this question separate from the verified absence of the built-in native Windows sandbox.

## Changelog

- **2026-11-04:** Created the report from current documentation, pinned release notes/runtime/container source, and local 2.1.318 version/help inspection. Recorded the sandbox-off default, current interactive Auto/headless Manual split, shell-only coverage, whole-process alternatives, and the standalone Windows alpha distinction. No earlier report existed at this path.

[e1]: https://code.claude.com/docs/en/overview
[e2]: https://code.claude.com/docs/en/sandboxing
[e3]: https://code.claude.com/docs/en/permission-modes
[e4]: https://code.claude.com/docs/en/permissions
[e5]: https://code.claude.com/docs/en/sandbox-environments
[e6]: https://code.claude.com/docs/en/settings-reference
[e7]: https://code.claude.com/docs/en/env-vars
[e8]: https://code.claude.com/docs/en/settings
[e9]: https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/CHANGELOG.md
[e10]: https://github.com/anthropic-experimental/sandbox-runtime/blob/98a4016d27609ed34dd1fb43d23ed6efb6eeeb77/README.md
[e11]: https://github.com/anthropics/claude-code/blob/26382c08758260c9f5deb2019d8fe851f89fa9ae/.devcontainer/init-firewall.sh
[e12]: https://code.claude.com/docs/en/remote-control
[e13]: claude.md#local-observation
[e14]: https://code.claude.com/docs/en/managed-settings
[e15]: https://code.claude.com/docs/en/devcontainer
[e16]: https://code.claude.com/docs/en/claude-code-on-the-web#security-and-isolation
