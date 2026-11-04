---
$schema: "./_schema.yaml"
schema_revision: 5
provider: "goose"
created: "2026-11-04"
last_updated: "2026-11-04"
content_policy:
  - "ValidFor(14d)"
agent: "codex"
model: "gpt-6.1-sol"
versions_examined:
  - "1.57.0"
evidence:
  -
    id: "mode-default"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose-provider-types/src/goose_mode.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Auto is the default mode; the alternatives control approval and tool availability, not an OS boundary."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "first-run"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose-cli/src/commands/configure.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "First-time provider setup selects a mode when unset, with Auto selected; provider credentials depend on setup choice."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "shell-host"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/platform_extensions/developer/shell.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Developer launches ordinary host shells with no sandbox wrapper; Unix defaults to bash/sh, Windows to cmd; GOOSE_SHELL overrides."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "file-host"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/platform_extensions/developer/edit.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Absolute paths and working-directory-relative paths use ordinary filesystem reads/writes without a workspace containment check."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "platform-registry"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/platform_extensions/mod.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Developer is a default-enabled in-process platform extension."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "extension-placement"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/extension_manager/mod.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Platform extensions bypass container placement; remaining stdio/legacy built-ins use their selected placement, HTTP remains remote; failed launches are retained as load errors."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "docker-exec"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/extension_manager/stdio.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Selected stdio extensions use docker exec -i with configured environment values; there is no host retry or container-creation policy."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "legacy-builtin"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/extension_manager/builtin.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Non-platform built-ins run goose mcp inside the selected container, requiring an installed container-side Goose executable."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "permission-order"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/permission/permission_inspector.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Auto allows calls without consulting per-tool policy; Approve and SmartApprove consult explicit permissions first, then mode-specific classification."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "permission-file"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/config/permission.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "permission.yaml contains user and smart_approve sections with always_allow, ask_before, and never_allow lists."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "headless"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose-cli/src/session/mod.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Headless confirmation errors in configured Approve/SmartApprove; other mode confirmation fallback selects AllowOnce."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "config-layers"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/config/base.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "System, additional, then user config layers merge; environment GOOSE_MODE overrides; secret storage is not isolated from same-user tools."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "config-paths"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/config/paths.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Absolute GOOSE_PATH_ROOT relocates config/data/state and plugin discovery; it does not constrain filesystem execution."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "subagents"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/platform_extensions/summon.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Subagent creation inherits mode and resolves extension selection but does not copy the parent container."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "session-init"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/session/session_manager.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "New sessions initialize container to None; container is an explicit stored session field."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "hooks"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/hooks/mod.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Plugin hook commands use a separate shell runner without extension container placement."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "egress-log"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/security/egress_inspector.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Egress inspection logs recognized shell/web destinations and returns Allow rather than enforcing a network policy."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "compose"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/documentation/docs/docker/docker-compose.yml"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Official whole-process example mounts repository read-write and SSH keys/gitconfig read-only and supplies provider credentials/environment."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "codex-adapter"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/providers/codex.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Deprecated codex provider maps Auto to --yolo, SmartApprove to --full-auto, Chat to read-only sandbox, and recommends codex-acp."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "allowlist-desktop"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/ui/desktop/src/allowlistFetcher.ts"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Desktop allowlist fetcher requires HTTPS, validates redirects, and rejects fetch/parse errors; this does not establish CLI enforcement."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "security-policy"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/SECURITY.md"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Upstream describes local action risks and recommends a dedicated VM/container with limited privileges."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "install-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/getting-started/installation/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Official installation covers native Windows separately from WSL and permits skipping installer configuration."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "docker-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/tutorials/goose-in-docker/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Official integration offers extension-only --container and separately whole-process Docker deployment; setup requires runtime and container-side dependencies."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "container-use-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/mcp/container-use-mcp/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Optional Container Use extension requires Docker; remote bridge additionally uses Node.js; local command is container-use stdio."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "mode-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/guides/managing-tools/goose-permissions/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Auto is default; configure and /mode select approval behavior; existing sessions retain their mode."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "config-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/guides/config-files/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Documents platform config locations and disabled-by-default prompt detection/classification settings."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "roots-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/guides/mcp-roots/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "MCP Roots advertises the current working directory to cooperative servers."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "allowlist-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/guides/allowlist/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Guide advertises GOOSE_ALLOWLIST for Desktop and CLI, conflicting with the examined implementation placement."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "adversary-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/guides/security/adversary-mode/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "adversary.md enables model review, shell-only by default; reviewer failure allows execution."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "injection-doc"
    method: "official_docs"
    location: "https://goose-docs.ai/docs/guides/security/prompt-injection-detection/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Optional pattern/ML inspection prompts for detected risks but allowed commands retain full authority."
    limitations: "Current unversioned documentation; source discrepancies are identified in the report."
  -
    id: "current-main"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/bfe8ccd23de1d2c47a4cb2970d522f093ae7f2ba/crates/goose/src/agents/extension_manager/mod.rs"
    version: "main@bfe8ccd23de1d2c47a4cb2970d522f093ae7f2ba"
    observed_on: "2026-11-04"
    claim: "Targeted comparison confirms platform extensions still bypass Docker placement."
    limitations: "Unreleased snapshot; only key Developer/placement paths compared."
  -
    id: "local-absence"
    method: "local_inspection"
    location: "Local macOS shell: sniff software agents --json; goose --version </dev/null; goose --help </dev/null"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Sniff reported Goose uninstalled; version/help returned command not found."
    limitations: "No CLI runtime observation, configuration inspection, or backend test was possible."
  -
    id: "release"
    method: "official_docs"
    location: "https://github.com/aaif-goose/goose/releases/tag/v1.57.0"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Latest published release examined was 1.57.0, published October 28, including allowlist-fetch hardening and Windows/shell-cleanup fixes."
    limitations: "Release notes do not establish runtime sandbox coverage."
  -
    id: "extension-config"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/config/extensions.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Explicit enabled state controls configured extension selection; persistent selection can differ from recipe/session selection."
    limitations: "Source inspection, not an executed containment or cross-platform test."
  -
    id: "extension-types"
    method: "source_code"
    location: "https://github.com/aaif-goose/goose/blob/30bba95105e59bc3e7df624cf6b4d433690cfe12/crates/goose/src/agents/extension.rs"
    version: "1.57.0"
    observed_on: "2026-11-04"
    claim: "Platform extension config is in-process; available_tools empty means all tools and nonempty values filter by exact tool name."
    limitations: "Source inspection, not an executed containment or cross-platform test."
summary: "No default OS sandbox; Auto mode normally permits host shell/file access. Official opt-in Docker integration contains selected MCP processes, while whole-process Docker also contains Developer and hooks. --container alone leaves platform Developer tools on the host."
support: "official_integration"
default_profiles:
  -
    id: "macos-interactive"
    environment: "macos"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose session. Unix shell and ordinary macOS process permissions; Keychain/privacy controls are external. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
  -
    id: "macos-non-interactive"
    environment: "macos"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose run --text. Unix shell and ordinary macOS process permissions; Keychain/privacy controls are external. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
  -
    id: "linux-interactive"
    environment: "linux"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose session. Unix shell with ordinary user permissions; no Goose-enabled filesystem/network sandbox. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
  -
    id: "linux-non-interactive"
    environment: "linux"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose run --text. Unix shell with ordinary user permissions; no Goose-enabled filesystem/network sandbox. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
  -
    id: "windows-interactive"
    environment: "windows"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose session. Official native Windows CLI; cmd shell by default; no Goose AppContainer boundary. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
  -
    id: "windows-non-interactive"
    environment: "windows"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose run --text. Official native Windows CLI; cmd shell by default; no Goose AppContainer boundary. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
  -
    id: "wsl2-interactive"
    environment: "wsl2"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose session. Linux CLI within preexisting WSL; mounted Windows paths and interoperability are not fenced by Goose. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
  -
    id: "wsl2-non-interactive"
    environment: "wsl2"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "goose run --text. Linux CLI within preexisting WSL; mounted Windows paths and interoperability are not fenced by Goose. Auto is the unset default; first-time configuration offers approval-mode selection, not isolation. A configured provider/authentication is required."
    evidence_ids:
      - "mode-default"
      - "first-run"
      - "shell-host"
      - "file-host"
      - "platform-registry"
      - "install-doc"
      - "headless"
mechanisms:
  -
    id: "docker-extensions"
    name: "--container"
    enforcement: "container"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "Existing Docker container via docker exec -i; Linux-container kernel/backend supplied by the operator. No Goose minimum Docker version established."
    prerequisites:
      - "Docker CLI and reachable daemon with permission to exec into a running container."
      - "Extension executables at configured paths inside the container; Goose inside it for legacy built-ins."
      - "Platform/backend combinations require independent runtime verification; native Windows CLI is not evidence of Windows-container support."
    activation: "Opt-in per session/run using --container <id-or-name>; off by default."
    filesystem: "Only contained extension processes use container filesystem/mounts. No automatic workspace/home mount or protected-path policy. Mounted secrets remain readable; symlinks resolve within the container namespace and mounted trees. Goose and Developer retain host files and temporary/state paths."
    network: "Existing container network policy applies to contained tools; Goose adds no destination/port/DNS restriction. Model transport and remote MCP client remain on host."
    processes: "Stdio/legacy MCP subprocesses execute in the selected container with configured -e values; no container user/capability/IPC policy is created. Host-side orchestration remains privileged to use Docker."
    coverage: "Contains stdio MCP and non-platform built-ins only. Platform Developer shell/write/edit, hooks, remote HTTP client, and Goose remain on host. Subagent sessions do not copy parent container."
    escape_hatches: "Omit --container for host placement; Auto changes approvals without removing Docker policy. Host platform tools are a separate execution path, not a denied-command retry."
    failure_behavior: "conditional"
    failure_notes: "A selected MCP Docker launch fails without host fallback when runtime/container/binary is unavailable; failed extension slots expose load errors, but other host tools may still run. Source-derived for interactive and headless launches."
    evidence_ids:
      - "docker-doc"
      - "docker-exec"
      - "legacy-builtin"
      - "extension-placement"
      - "platform-registry"
      - "subagents"
      - "session-init"
      - "hooks"
      - "current-main"
  -
    id: "docker-whole-process"
    name: "Running goose Inside Docker"
    enforcement: "container"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "Operator-built Docker image and Docker/Compose runtime; Linux image in the official example, with a runtime-provided Linux kernel/VM where needed."
    prerequisites:
      - "Installed compatible Docker backend; source checkout/build dependencies or a prepared Goose image."
      - "Configured provider/model and credentials; keyring alternatives for headless/container use."
      - "Operator-reviewed mounts, capabilities, network, and user; no Goose cloud account required by isolation itself."
    activation: "Opt-in whole-process deployment following official Docker tutorial, not a built-in sandbox flag."
    filesystem: "Container filesystem and explicit mounts apply to Goose and local tools, including temporary files. Official example mounts repository read-write and SSH keys/gitconfig read-only; read-only permits disclosure. No implicit confidentiality guarantee."
    network: "Container policy applies to local tool and model/API traffic together. Example grants network; disabling it also blocks external provider transport. Remote MCP retains its server-side policy."
    processes: "Goose, local subprocesses, platform tools, and hooks execute under outer container permissions. Bind mounts, sockets, privileges, and supplied credentials determine remaining host exposure."
    coverage: "Contains main agent, Developer shell/file tools, local hooks/extensions/subagents unless explicitly routed elsewhere; does not relocate remote services."
    escape_hatches: "Changing container creation options changes isolation. Auto bypasses approvals but does not bypass outer container enforcement. Docker socket/host mounts can expose host capabilities."
    failure_behavior: "conditional"
    failure_notes: "Goose cannot run in a container that cannot start; exact image build compatibility and runtime behavior across Docker backends were not tested. There is no Goose-managed host retry in this deployment."
    evidence_ids:
      - "docker-doc"
      - "compose"
      - "security-policy"
  -
    id: "container-use"
    name: "Container Use Extension"
    enforcement: "container"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "Separately installed Container Use MCP server orchestrating Docker task environments; remote bridge is a separate documented option."
    prerequisites:
      - "Docker and separately installed container-use for the local MCP command."
      - "Node.js for documented npx mcp-remote bridge."
      - "Remote account/authentication and version-specific OS guarantees unresolved; no backend version pinned by the Goose guide."
    activation: "Opt-in extension: container-use stdio, or documented remote MCP bridge. Not installed or used by default."
    filesystem: "Selected task operations run in extension-managed environments. This does not restrict host Goose/Developer paths. Exact mounts, protected paths, credential handling, temporary paths, and symlink policy require extension-specific verification."
    network: "No Goose-wide network limit is added. Task and remote-service network policy is delegated to the installed extension/backend; model transport remains with Goose."
    processes: "Container task commands are isolated by the external runtime; MCP orchestration and other host tools remain outside it. Environment/credential policy depends on extension configuration."
    coverage: "Only Container Use tool operations; other extensions, Developer, hooks, agent process, and subagents do not become universally contained."
    escape_hatches: "Host Developer remains available unless independently disabled; enabling this extension is not exclusive routing. Approval mode does not alter Docker enforcement."
    failure_behavior: "unknown"
    failure_notes: "Goose setup page does not establish unavailable-runtime fallback or remote-service failure semantics; inspect and test the installed Container Use version."
    evidence_ids:
      - "container-use-doc"
      - "docker-doc"
      - "extension-placement"
  -
    id: "delegated-codex"
    name: "Codex CLI provider sandbox"
    enforcement: "os_sandbox"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "External installed Codex CLI selected through the legacy codex provider; --full-auto or --sandbox read-only requests its sandbox. Exact OS backend is not established by Goose source."
    prerequisites:
      - "Separately installed, authenticated, compatible Codex CLI; legacy Goose codex provider is deprecated in favor of codex-acp."
      - "Actual OS sandbox availability, backend prerequisites, filesystem/network details, and native Windows behavior depend on installed Codex version and remain outside this verified mapping."
    activation: "Conditional opt-in delegated provider. Goose SmartApprove passes --full-auto; Chat passes --sandbox read-only. Goose Auto instead passes --yolo."
    filesystem: "Delegated workspace-write/read-only request, enforced by the external provider when supported. Goose source does not establish exact read roots, home/credential exclusions, temporary paths, or symlink enforcement; no confidentiality claim follows from read-only."
    network: "Provider sandbox policy depends on installed provider/configuration; this mapping does not establish tool egress restrictions or constrain other Goose clients/model transport."
    processes: "Only actions executed inside the delegated provider sandbox can inherit its boundary. Goose process, its hooks, and independent extension execution are not contained by this adapter."
    coverage: "Delegated Codex execution only; not the general Goose shell/file/MCP runtime. Replacement ACP integrations must be checked separately."
    escape_hatches: "Auto explicitly passes --yolo, bypassing delegated approval and sandbox. Approve adds no sandbox flag and leaves provider default behavior. Does not bypass an independent outer Docker container."
    failure_behavior: "unknown"
    failure_notes: "Argument mapping is source-verified; unavailable OS backend, failure/escalation, and compatibility with current installed Codex releases are unresolved and require provider-specific tests."
    evidence_ids:
      - "codex-adapter"
controls:
  -
    id: "container-flag"
    kind: "cli_flag"
    name: "--container"
    values:
      - "Docker container ID or name"
    effect: "Places selected stdio/legacy built-in MCP processes in an existing container; platform tools retain host access."
    example: "goose run --container goose-dev --text \"Inspect the project\""
    precedence: "Per session/run placement; does not create/override container policy and is not automatically copied to subagents."
    mechanism_ids:
      - "docker-extensions"
    evidence_ids:
      - "docker-exec"
      - "legacy-builtin"
      - "extension-placement"
      - "subagents"
  -
    id: "mode-env"
    kind: "environment_variable"
    name: "GOOSE_MODE"
    values:
      - "auto"
      - "approve"
      - "smart_approve"
      - "chat"
    effect: "Adjacent approval control, not Goose OS isolation. Auto skips per-tool policy; deprecated codex adapter additionally maps modes to its own sandbox/YOLO flags."
    example: "GOOSE_MODE=approve goose session"
    precedence: "Environment overrides merged default config; persisted session mode can differ. Does not override Docker policy."
    mechanism_ids:
      - "delegated-codex"
    evidence_ids:
      - "mode-default"
      - "config-layers"
      - "permission-order"
      - "codex-adapter"
  -
    id: "mode-config"
    kind: "config_key"
    name: "GOOSE_MODE"
    values:
      - "auto"
      - "approve"
      - "smart_approve"
      - "chat"
    effect: "Persistent default for approval and tool use; does not establish filesystem or network containment."
    example: "GOOSE_MODE: smart_approve"
    precedence: "System < additional files < user < environment at mode resolution; defaults do not rewrite existing sessions."
    mechanism_ids: []
    evidence_ids:
      - "mode-doc"
      - "config-layers"
  -
    id: "mode-session"
    kind: "session_command"
    name: "/mode"
    values:
      - "auto"
      - "approve"
      - "smart_approve"
      - "chat"
    effect: "Changes permission mode for an interactive session without creating an OS boundary."
    example: "/mode approve"
    precedence: "Session-scoped; independent of outer container enforcement."
    mechanism_ids: []
    evidence_ids:
      - "mode-doc"
      - "permission-order"
  -
    id: "user-permissions"
    kind: "config_key"
    name: "user"
    values:
      - "always_allow: [tool names]"
      - "ask_before: [tool names]"
      - "never_allow: [tool names]"
    effect: "permission.yaml governs dispatch in Approve/SmartApprove; Auto bypasses these rules. Rules constrain named tool calls, not OS capabilities."
    example: "user:\n  always_allow: []\n  ask_before: [shell, write, edit]\n  never_allow: [computercontroller__computer_control]"
    precedence: "Explicit user rule precedes smart annotations/model classification in approval modes; Auto skips this lookup."
    mechanism_ids: []
    evidence_ids:
      - "permission-file"
      - "permission-order"
  -
    id: "config-layers"
    kind: "environment_variable"
    name: "GOOSE_ADDITIONAL_CONFIG_FILES"
    values:
      - "OS-separated path list"
    effect: "Adds default configuration layers; adjacent policy configuration, not a mandatory security boundary."
    example: "GOOSE_ADDITIONAL_CONFIG_FILES=/opt/goose/team.yaml goose session"
    precedence: "Loaded after system config and before user config; OS path separator; environment setting values can override."
    mechanism_ids: []
    evidence_ids:
      - "config-layers"
  -
    id: "path-root"
    kind: "environment_variable"
    name: "GOOSE_PATH_ROOT"
    values:
      - "Absolute filesystem path"
    effect: "Relocates Goose config/data/state; does not restrict tool paths."
    example: "GOOSE_PATH_ROOT=/tmp/goose-state goose session"
    precedence: "Absolute only; config/data/state become children; normal environment default resolution still applies."
    mechanism_ids: []
    evidence_ids:
      - "config-paths"
  -
    id: "shell-choice"
    kind: "environment_variable"
    name: "GOOSE_SHELL"
    values:
      - "Shell executable name or path"
    effect: "Selects shell interpreter; does not sandbox execution."
    example: "GOOSE_SHELL=/bin/zsh goose session"
    precedence: "Overrides platform default shell; no change to permissions/mounts/network."
    mechanism_ids: []
    evidence_ids:
      - "shell-host"
  -
    id: "no-keyring"
    kind: "environment_variable"
    name: "GOOSE_DISABLE_KEYRING"
    values:
      - "Any set value"
    effect: "Avoids interactive keyring access; permits file-based credentials, not credential isolation."
    example: "GOOSE_DISABLE_KEYRING=1 goose run --text \"Inspect the project\""
    precedence: "Process-scoped environment or config influences storage; provider credentials still required."
    mechanism_ids: []
    evidence_ids:
      - "config-layers"
  -
    id: "allowlist-env"
    kind: "environment_variable"
    name: "GOOSE_ALLOWLIST"
    values:
      - "HTTPS URL of extension-command YAML policy"
    effect: "Docs advertise CLI installation restriction; examined enforcement is Desktop-only, so CLI efficacy remains unresolved. Does not sandbox installed tools."
    example: "GOOSE_ALLOWLIST=https://example.com/goose-allowlist.yaml goose session"
    precedence: "Optional; absent means unrestricted installation in documentation. CLI application and failure precedence unverified."
    mechanism_ids: []
    evidence_ids:
      - "allowlist-doc"
      - "allowlist-desktop"
      - "release"
  -
    id: "security-enabled"
    kind: "config_key"
    name: "SECURITY_PROMPT_ENABLED"
    values:
      - "true"
      - "false"
    effect: "Enables argument-pattern security inspection and prompts; no OS boundary. Defaults false."
    example: "SECURITY_PROMPT_ENABLED: true"
    precedence: "Other security inspectors can intervene independently of approval; headless Auto confirmation fallback can allow."
    mechanism_ids: []
    evidence_ids:
      - "config-doc"
      - "injection-doc"
      - "headless"
  -
    id: "security-threshold"
    kind: "config_key"
    name: "SECURITY_PROMPT_THRESHOLD"
    values:
      - "Number from 0.01 through 1.0"
    effect: "Configures inspection threshold; default 0.8. Does not restrict process authority."
    example: "SECURITY_PROMPT_THRESHOLD: 0.8"
    precedence: "Used by optional detection when enabled; not Docker policy."
    mechanism_ids: []
    evidence_ids:
      - "config-doc"
      - "injection-doc"
  -
    id: "classifier-enabled"
    kind: "config_key"
    name: "SECURITY_PROMPT_CLASSIFIER_ENABLED"
    values:
      - "true"
      - "false"
    effect: "Enables optional ML inspection, default false. This is detection, not enforced isolation."
    example: "SECURITY_PROMPT_CLASSIFIER_ENABLED: true"
    precedence: "Requires configured optional classifier; normal config/environment resolution applies."
    mechanism_ids: []
    evidence_ids:
      - "config-doc"
      - "injection-doc"
  -
    id: "classifier-endpoint"
    kind: "config_key"
    name: "SECURITY_PROMPT_CLASSIFIER_ENDPOINT"
    values:
      - "Classification service URL"
    effect: "Selects additional classification transport, potentially sending tool/context data. This is detection, not enforced isolation."
    example: "SECURITY_PROMPT_CLASSIFIER_ENDPOINT: https://api.example.com/classify"
    precedence: "Requires configured optional classifier; normal config/environment resolution applies."
    mechanism_ids: []
    evidence_ids:
      - "config-doc"
      - "injection-doc"
  -
    id: "classifier-token"
    kind: "config_key"
    name: "SECURITY_PROMPT_CLASSIFIER_TOKEN"
    values:
      - "Authentication token"
    effect: "Authenticates classification endpoint when required. This is detection, not enforced isolation."
    example: "SECURITY_PROMPT_CLASSIFIER_TOKEN: YOUR_TOKEN"
    precedence: "Requires configured optional classifier; normal config/environment resolution applies."
    mechanism_ids: []
    evidence_ids:
      - "config-doc"
      - "injection-doc"
  -
    id: "delegated-provider"
    kind: "environment_variable"
    name: "GOOSE_PROVIDER"
    values:
      - "codex"
    effect: "Selects the deprecated delegated CLI provider; sandbox requests then depend on GOOSE_MODE, with Auto passing --yolo."
    example: "GOOSE_PROVIDER=codex GOOSE_MODE=smart_approve goose session"
    precedence: "Provider environment overrides configured provider; requires installed/authenticated external CLI. No equivalent replacement ACP mapping is assumed."
    mechanism_ids:
      - "delegated-codex"
    evidence_ids:
      - "codex-adapter"
  -
    id: "container-use-extension"
    kind: "cli_flag"
    name: "--with-extension"
    values:
      - "container-use stdio"
    effect: "Adds the separately installed Container Use MCP server; only its selected task operations use managed containers."
    example: "goose session --with-extension \"container-use stdio\""
    precedence: "Session-added stdio extension; does not disable host Developer or other extensions. An additional --container places this MCP launcher in that selected container."
    mechanism_ids:
      - "container-use"
      - "docker-extensions"
    evidence_ids:
      - "container-use-doc"
      - "docker-doc"
      - "docker-exec"
      - "extension-placement"
  -
    id: "adversary-tools"
    kind: "config_key"
    name: "tools"
    values:
      - "Comma-separated tool names in adversary.md header"
    effect: "Creating config-directory adversary.md enables optional model review; tools header expands default shell-only coverage. It fails open on reviewer failure and adds no OS isolation."
    example: "tools: shell, computercontroller__computer_control\n---\nBLOCK if the command exfiltrates data. ALLOW normal development."
    precedence: "Optional config-directory file; deleting it disables reviewer. Review is independent of Docker placement and uses configured model/provider."
    mechanism_ids: []
    evidence_ids:
      - "adversary-doc"
  -
    id: "developer-enabled"
    kind: "config_key"
    name: "extensions.developer.enabled"
    values:
      - "true"
      - "false"
    effect: "Disables or enables host Developer tool surface; disabling it does not isolate the agent or other extensions."
    example: "extensions:\n  developer:\n    type: platform\n    name: developer\n    enabled: false"
    precedence: "Persistent default selection may differ from explicit recipe/session extension selection; no OS access policy is created."
    mechanism_ids: []
    evidence_ids:
      - "extension-config"
      - "extension-types"
  -
    id: "tool-selection"
    kind: "config_key"
    name: "extensions.<name>.available_tools"
    values:
      - "List of exact tool names; empty means all"
    effect: "Filters extension tool availability; does not restrict filesystem paths, subprocesses, or network for remaining tools."
    example: "extensions:\n  developer:\n    type: platform\n    name: developer\n    enabled: true\n    available_tools: [tree]"
    precedence: "Per-extension selection; independent of the OS boundary and approval mode."
    mechanism_ids: []
    evidence_ids:
      - "extension-types"
gaps:
  -
    location: "evidence/local-absence"
    detail: "No installed Goose; runtime help/version, file boundaries, backend denial, and native OS behavior were not exercised."
    next_check: "In disposable credential-free installations, capture version/help and sentinel-file/process tests on macOS, Linux, native Windows, and WSL2; test missing Docker/container without other extensions."
  -
    location: "mechanisms/docker-extensions/environments"
    detail: "Official Docker approach is not a runtime compatibility matrix; exact native Windows Docker mode and WSL2 backend behavior remain untested."
    next_check: "Run pinned Linux-container backend tests from each OS and distinguish native CLI, WSL CLI, Linux containers, and Windows containers."
  -
    location: "mechanisms/docker-whole-process/failure_notes"
    detail: "Official source-build example was inspected but not built; no minimum Docker version or all-platform image compatibility established."
    next_check: "Build the pinned Dockerfile and run the minimal CLI workload on the supported Docker backends, recording versions and errors."
  -
    location: "mechanisms/container-use"
    detail: "Exact backend platform guarantees, mounts/symlinks, network/credentials, remote account requirements, and unavailable-backend fallback are unresolved."
    next_check: "Inspect the installed Container Use version and remote-service authentication terms; run disposable denied-path/network and stopped-runtime tests."
  -
    location: "controls/allowlist-env"
    detail: "Guide claims CLI support, but source search located policy fetch/enforcement in Desktop TypeScript and no CLI consumer."
    next_check: "With a pinned CLI, exclude a harmless test MCP server in an HTTPS allowlist and attempt configure/--with-extension launch; repeat with fetch failure and verify whether any CLI code consumes the policy."
  -
    location: "controls/mode-env/effect"
    detail: "Legacy codex permission-flag mapping is verified, but actual sandbox backend/platform support and replacement ACP mappings depend on separately installed provider versions."
    next_check: "Inspect pinned codex-acp and other selected ACP adapters plus delegated CLI versions, capture child arguments, and exercise denied-path/network operations."
  -
    location: "mechanisms/docker-extensions/coverage"
    detail: "Host platform dispatch and missing subagent container propagation are source-derived; no runtime sentinel test confirms observed effects."
    next_check: "Under --container, compare Developer and stdio MCP process locations and outside-workspace writes, then repeat through a Summon subagent."
  -
    location: "mechanisms/delegated-codex"
    detail: "Environment list identifies conditional integration targets, not verified backend support. The inspected adapter does not establish OS facility, native Windows support, exact filesystem/network policy, failures, or current Codex/ACP compatibility."
    next_check: "Pin the delegated CLI and adapter versions on each OS, capture launch flags, verify backend availability, and run denied file/network and escalation tests without accessing real credentials."
changes: []
---

# Sandboxing in Goose CLI

Goose CLI supplies **no default OS sandbox** for its agent, shell, or file tools. A normal installation runs with the launching user's authority and defaults to `GOOSE_MODE=auto`, which approves tool calls automatically. Optional official integrations can put selected MCP processes, or the entire CLI, in Docker. These are materially different boundaries: **`--container` does not contain the current Developer shell and file tools.** This conclusion comes from execution-path inspection, not merely the absence of a documented sandbox switch. [Default mode](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose-provider-types/src/goose_mode.rs), [Developer execution](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/platform_extensions/developer/shell.rs), [extension dispatch](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension_manager/mod.rs).

## Scope, versions, and method

Research date: **2026-11-04 UTC**. The requested [Block documentation](https://block.github.io/goose/) redirects to [goose-docs.ai](https://goose-docs.ai/), and [block/goose](https://github.com/block/goose) redirects to AAIF Goose. The latest published release examined was **v1.57.0**, published **2026-10-28**, source commit **`30bba95105e59bc3e7df624cf6b4d433690cfe12`**. Its release notes include stricter extension-allowlist fetching, Windows build fixes, and shell process-tree cleanup; those changes do not introduce a general agent sandbox. [Release](https://github.com/aaif-goose/goose/releases/tag/v1.57.0), [release tree](https://github.com/aaif-goose/goose/tree/30bba95105e59bc3e7df624cf6b4d433690cfe12).

The release source was downloaded for read-only inspection. Key Developer and extension-placement paths were also checked at current `main`, **`bfe8ccd23de1d2c47a4cb2970d522f093ae7f2ba`**, dated **2026-11-04 07:57:41 UTC**; the platform-extension exception persists there. This was a targeted comparison, not a full audit of every unreleased change. [Current dispatch](https://github.com/aaif-goose/goose/blob/bfe8ccd23de1d2c47a4cb2970d522f093ae7f2ba/crates/goose/src/agents/extension_manager/mod.rs).

Local observation: `sniff software agents --json` reported Goose absent on this macOS host; both `goose --version </dev/null` and `goose --help </dev/null` returned `command not found`. No provider session, credential lookup, extension, or container was executed. Examples below are documentation/source-derived, not runtime validation. No existing report was present to refresh.

## Normal-install defaults

The table assumes provider setup is complete, no explicit mode override, and no externally imposed container or OS policy. “Host access” always means access permitted to the process identity, not administrator authority.

| Platform | Interactive `goose session` | Non-interactive `goose run --text …` | Meaningful platform difference |
| --- | --- | --- | --- |
| macOS | No sandbox; Auto by default | Same boundary and default | Native Unix shell; macOS permissions and Keychain remain external controls |
| Linux | No sandbox; Auto by default | Same boundary and default | Native Unix shell; no Goose-enabled namespace/seccomp filesystem boundary |
| Native Windows | No sandbox; Auto by default | Same boundary and default | Official native installation exists; shell defaults to `cmd`, not a WSL sandbox |
| WSL2 | No Goose sandbox; Auto by default | Same boundary and default | Linux process inside the existing WSL environment; mounted Windows files and enabled interoperability are not fenced by Goose |

Native Windows and WSL are separately documented installation options; the table does not infer Windows support from WSL. Unix uses `bash` if available, otherwise `sh`; `GOOSE_SHELL` can change the executable. Windows also supports PowerShell or a supplied POSIX shell, with invocation flags chosen automatically. Process-tree cancellation is cleanup, not containment. [Installation](https://goose-docs.ai/docs/getting-started/installation/), [environment variables](https://goose-docs.ai/docs/guides/environment-variables/), [shell implementation](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/platform_extensions/developer/shell.rs).

**First run is a choice, not an enforced protection tier.** After successful provider setup, `goose configure` asks for a mode if none is already set, with Auto selected. Existing configuration or `GOOSE_MODE` can suppress that choice. Provider setup offers OpenRouter login, Tetrate login, or manual configuration; credentials/account requirements depend on the selected provider, and a local-model provider avoids a mandatory Goose cloud account. Skipping installer configuration with `CONFIGURE=false` does not enable a sandbox; a headless invocation still needs a working provider configuration. [Setup implementation](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose-cli/src/commands/configure.rs), [installation](https://goose-docs.ai/docs/getting-started/installation/).

## What the uncontained agent can reach

Developer is enabled by default and executes in the Goose process. Its shell launches an ordinary child process; `write` and `edit` use ordinary filesystem operations. Absolute paths are accepted; relative paths are joined to the session working directory without a containment check. Consequently, the workspace is a starting directory, not a filesystem jail. Additional paths do not need a Goose sandbox grant. `..` and symlink targets remain subject to normal OS resolution rather than a workspace fence. Directory-tree filtering and advice to use gitignore-aware search reduce context noise, not direct-read capability. [Platform registry](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/platform_extensions/mod.rs), [path resolution and edits](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/platform_extensions/developer/edit.rs), [Developer tools](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/platform_extensions/developer/mod.rs).

Home directories, dotfiles, repositories outside the workspace, temporary files, and credentials in readable files have no general Goose deny rule. OS ACLs, ownership, macOS privacy permissions, and credential-store authorization can still deny access. File-based `secrets.yaml` permissions protect against other identities; they do not isolate tools running as its owner. Keyring access can require separate authorization and must not be described as automatically readable merely because file access is broad. Likewise, protected system files remain protected by the OS, not a Goose-specific protected-path list. [Secret storage](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/config/base.rs), [security guidance](https://github.com/aaif-goose/goose/blob/v1.57.0/SECURITY.md).

Shell programs can execute other programs and access the network with the user's ordinary permissions. Goose supplies no default tool destination allowlist or deny-all network namespace. Its `EgressInspector` recognizes some destinations in shell/web arguments and logs them while returning `InspectionAction::Allow`; it is observation, not a firewall. Model/API transport is a separate connection from the Goose process to its selected provider. Blocking a tool through approval does not block that transport or prevent already collected context from being sent. [Egress implementation](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/security/egress_inspector.rs), [provider configuration](https://goose-docs.ai/docs/guides/config-files/).

MCP Roots advertises the session working directory to cooperative servers. It does not change the OS authority of their processes. A server may implement stricter rules itself; that is server-specific enforcement. No universal project-trust prompt establishes a filesystem boundary in the examined launch and tool paths. Recipe warnings, hints, plan instructions, and “Chat” mode must likewise not be read as a read-only OS sandbox. [Roots](https://goose-docs.ai/docs/guides/mcp-roots/), [CLI launch](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose-cli/src/cli.rs), [permission modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions/).

## Optional isolation and its coverage

### Attach selected extensions to an existing Docker container

Both session and run accept an existing container ID or name:

```sh
goose session --container goose-dev
goose run --container goose-dev --text "Inspect the project"
goose session --container goose-dev --with-extension "/usr/local/bin/my-mcp-server"
```

Docker must be installed, its daemon reachable, and the named container running. Goose does not create a container or set its mounts, user, capabilities, network, or read-only root filesystem here. Stdio extension executables must already exist inside it at their configured paths. Legacy built-ins require `goose` inside the container to serve `goose mcp <name>`. There is no additional Goose account requirement; Docker distribution/licensing and model credentials are separate prerequisites. [Docker tutorial](https://goose-docs.ai/docs/tutorials/goose-in-docker/), [stdio launch](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension_manager/stdio.rs), [built-in launch](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension_manager/builtin.rs).

**The documentation overstates current coverage when it says configured extensions automatically run inside the container.** The release dispatch first recognizes platform-extension names and constructs their clients in process. This includes Developer even when old configuration labels it `type: builtin`. `Placement::for_config` supplies no container placement for these clients. Only remaining stdio and legacy built-in branches use Docker. Remote Streamable HTTP servers remain remote; the Goose HTTP client stays on the host. [Dispatch and placement](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension_manager/mod.rs), [current-main dispatch](https://github.com/aaif-goose/goose/blob/bfe8ccd23de1d2c47a4cb2970d522f093ae7f2ba/crates/goose/src/agents/extension_manager/mod.rs).

```mermaid
flowchart LR
  subgraph Host
    G[Goose CLI and model transport]
    D[Platform Developer shell and file tools]
    H[Plugin hooks]
    G --> D
    G --> H
    G --> X[docker exec -i]
  end
  subgraph Existing_Docker_Container
    M[Stdio MCP servers and legacy built-ins]
    T[Their subprocesses and files]
    M --> T
  end
  X --> M
  G --> R[Remote HTTP MCP server]
  G --> P[Model provider]
```

For contained extensions, the container's filesystem, mounts, identity, and network determine access. No host workspace or home is automatically mounted by `--container`; conversely, any existing bind mounts, secrets, agent sockets, or Docker socket remain accessible according to the container configuration. Symlinks resolve in that filesystem and its mounted trees. Temporary files created by an extension belong to its environment, while Goose's own session/configuration files remain on the host. `docker exec` forwards configured extension environment values with `-e`, not a sanitized confidentiality policy. It does not add `--network none`, a mount policy, or an explicit container working directory. [Command construction](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension_manager/stdio.rs).

### Run the entire CLI inside Docker

The official tutorial also supplies a source-build Docker/Compose workflow. Here Goose, its in-process Developer tools, hooks, and locally spawned extension processes run inside the container, subject to mounts and privileges. Remote services still execute in their own environments. Linux containers use Linux isolation; on macOS or Windows their Linux kernel is supplied by the chosen Docker backend, not by a Goose-managed VM. Native Windows CLI availability does not mean Windows-container images or every Docker backend were tested for this report. [Whole-process tutorial](https://goose-docs.ai/docs/tutorials/goose-in-docker/), [Dockerfile](https://github.com/aaif-goose/goose/blob/v1.57.0/documentation/docs/docker/Dockerfile).

The shipped Compose example mounts the repository read-write, `~/.gitconfig` read-only, and **`~/.ssh` read-only**. Read-only SSH keys can still be read and transmitted. It also passes model credentials and socket-related environment names. Adapt this example before treating it as a confidentiality boundary; it is a development environment, not a minimal hardened preset. The tutorial requires explicit model/provider/API configuration and discusses keyring limitations in containers. [Compose file](https://github.com/aaif-goose/goose/blob/v1.57.0/documentation/docs/docker/docker-compose.yml).

A minimal illustrative alternative, using an image you have built with Goose and its dependencies, is:

```sh
docker run --rm --network bridge \
  --mount "type=bind,src=$PWD,dst=/workspace" --workdir /workspace \
  --env OPENAI_API_KEY --env GOOSE_PROVIDER=openai --env GOOSE_MODEL \
  --env GOOSE_MODE=auto --env GOOSE_DISABLE_KEYRING=1 \
  goose-local goose run --text "Inspect this workspace"
```

This is an operator-created container policy using the officially documented whole-process approach, not an extra Goose flag or a tested image. It deliberately grants workspace writes and network access for model transport. Change mounts/user/network at container creation to change those limits. `--network none` would also block an external model API unless another transport arrangement is provided.

### Container Use and delegated CLI providers

The official extension catalog documents **Container Use**, installed separately, with Docker required and Node.js additionally required for the documented remote bridge. The local launch command is `container-use stdio`; the remote example uses `npx -y mcp-remote https://container-use.com/mcp`. It creates task environments through its tools; enabling it does not relocate Goose, Developer, hooks, or other MCP servers. Host-side orchestration still requires trust. Remote-service authentication, retention, and platform guarantees are not established by the Goose setup page, so no account-free remote execution claim is made here. [Container Use setup](https://goose-docs.ai/docs/mcp/container-use-mcp/).

Goose can also delegate inference/actions to another agent CLI. In the deprecated `codex` provider, source maps Goose Auto to `--yolo`, Smart Approve to `--full-auto`, Approve to no permission flag, and Chat to `--sandbox read-only`. Here switching approval mode **can change the delegated provider's sandbox**, because the adapter explicitly passes those flags. That boundary belongs to the installed Codex version, not to Goose's general tool runtime. The provider metadata recommends `codex-acp`; do not assume the legacy mapping applies unchanged to ACP or all providers. A working delegated CLI installation and its authentication are required. [Adapter and deprecation](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/providers/codex.rs), [documented provider integration](https://goose-docs.ai/docs/guides/cli-providers/).

For a session intended to use only Container Use, explicitly disable host Developer in the persistent extension configuration and inspect the remaining enabled tools:

```yaml
extensions:
  developer:
    type: platform
    name: developer
    enabled: false
```

Then a session can add the separately installed server with `goose session --with-extension "container-use stdio"`. Disabling one extension removes that tool surface; it does not isolate Goose or block another enabled extension from executing host code. `extensions.<name>.available_tools` filters tool names; an empty list means **all tools**, not none. Recipe/session extension selections can differ from persistent defaults. [Extension configuration](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/config/extensions.rs), [tool-filter semantics](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension.rs).

### Coverage summary

| Execution surface | Normal host launch | With `--container` | Entire Goose process in Docker |
| --- | --- | --- | --- |
| Developer shell, `write`, `edit`, other platform tools | Host | Host | Container |
| Stdio MCP and legacy built-in MCP processes | Host | Selected existing container | Container, unless separately directed elsewhere |
| Remote HTTP MCP | Remote server; host client | Same | Remote server; container client |
| Subagents | Host session/tool runtime | No inherited container guarantee | Within outer container for local execution |
| Plugin hooks | Host shell process | Host shell process | Container shell process |
| Provider/API client and Goose state | Host | Host | Container, plus explicitly mounted state |

Subagents inherit the parent's permission mode and selected extensions unless overridden, but the examined Summon child-session creation does not copy `container`; new sessions initialize it to `None`. This is source-derived evidence against relying on inherited Docker placement, not a tested exploit. Plugin hooks use a separate command runner (`sh -c`, with Flatpak host-spawn handling) and do not consult extension container placement. Native Windows hook execution additionally depends on an available `sh`; this report does not claim every plugin hook works in a clean Windows shell. [Summon](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/platform_extensions/summon.rs), [session initialization](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/session/session_manager.rs), [hooks](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/hooks/mod.rs).

## Approval, configuration, and failure behavior

Use these controls for oversight, not OS isolation:

```sh
# Process-scoped mode override; requires a configured provider.
GOOSE_MODE=approve goose session
GOOSE_MODE=smart_approve goose session
GOOSE_MODE=auto goose run --text "Run the checks"
```

Within an interactive session, `/mode auto`, `/mode approve`, `/mode smart_approve`, and `/mode chat` change the session's mode. Persistent defaults can be selected through `goose configure` or `GOOSE_MODE: approve` in `config.yaml`. Chat suppresses agent tool use; it does not remove Goose's own filesystem, provider, or extension-startup authority. Auto bypasses permission approval, not Docker's kernel-enforced mounts or permissions. There is no independent Goose-wide YOLO sandbox toggle in the examined CLI parser. [Modes](https://goose-docs.ai/docs/guides/managing-tools/goose-permissions/), [CLI parser](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose-cli/src/cli.rs).

Configuration normally lives at `~/.config/goose/config.yaml` on macOS/Linux/WSL and `%APPDATA%\Block\goose\config\config.yaml` on Windows. Source loads system defaults first (`/etc/goose/config.yaml`, or `%PROGRAMDATA%\goose\config.yaml`), then `GOOSE_ADDITIONAL_CONFIG_FILES` in OS path-list order, then the user file. Later layers override earlier values; `GOOSE_MODE` in the environment wins during mode resolution. Session mode is persisted, so a changed default need not rewrite existing sessions. `GOOSE_PATH_ROOT`, when absolute, relocates config/data/state under that root; it does not chroot tools. These system settings are defaults, not an immutable administrator security policy. [Configuration layering](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/config/base.rs), [path relocation](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/config/paths.rs), [configuration guide](https://goose-docs.ai/docs/guides/config-files/).

Per-tool policy is stored in `permission.yaml`; for example:

```yaml
user:
  always_allow: []
  ask_before: [shell, write, edit]
  never_allow: [computercontroller__computer_control]
```

Use actual tool names from the installed extensions; current Developer tools are unprefixed. In Approve and Smart Approve, explicit user rules take priority. Without such a rule, Approve asks; Smart Approve allows read-only annotations and can ask the model to classify other calls. **Auto skips these per-tool permission rules**, including `never_allow`, in the inspected permission inspector. Other security inspectors can still intervene. Classifying an operation as read-only does not restrict which files it reads or guarantee it cannot disclose their contents. [Permission storage](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/config/permission.rs), [decision order](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/permission/permission_inspector.rs).

Interactive confirmation supports allowing or denying a request; permission denial keeps the selected tool from executing. In headless mode, if a confirmation is requested while the effective configured mode is Approve or Smart Approve, the CLI cancels the run and returns an error requiring an interactive terminal. This is conditional on a confirmation request: pre-allowed calls may already have executed. The fallback branch for confirmations in other modes warns and selects `AllowOnce`; therefore even an optional security prompt is not a reliable unattended containment policy under Auto. No general sandbox-escalation handshake or automatic retry outside Docker appears in the examined extension-launch paths. [Headless confirmation handling](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose-cli/src/session/mod.rs).

A missing Docker command, unavailable daemon, stopped/missing container, absent in-container binary, or MCP initialization failure becomes an extension setup failure. The selected Docker branch does not retry its command on the host; extension slots retain a failed state and expose a load error. This does not imply a fail-closed session: host platform tools and other successfully loaded extensions remain separate capabilities. An OS-denied file operation is returned as a tool error, without a Goose privilege grant; the agent can propose another action, which goes through its applicable tool controls. [Launch errors](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension_manager/stdio.rs), [extension failure state](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/extension_manager/mod.rs), [file errors](https://github.com/aaif-goose/goose/blob/v1.57.0/crates/goose/src/agents/platform_extensions/developer/edit.rs).

Headless credential access is another failure surface: OS keyring authorization may require a GUI. `GOOSE_DISABLE_KEYRING=1` avoids that lookup and permits file-based secret storage; provide credentials through process environment where appropriate. It changes credential storage/access, not tool isolation. [Unattended CLI guidance](https://goose-docs.ai/docs/guides/goose-cli-commands/).

Optional prompt-injection inspection uses `SECURITY_PROMPT_ENABLED: true`, with `SECURITY_PROMPT_THRESHOLD` defaulting to `0.8`. ML classification additionally uses `SECURITY_PROMPT_CLASSIFIER_ENABLED`, `SECURITY_PROMPT_CLASSIFIER_ENDPOINT`, and optional `SECURITY_PROMPT_CLASSIFIER_TOKEN`. Both detection features default off. They inspect arguments/context and can request approval; allowed commands still have full process authority. Classification can itself send context to an additional endpoint. [Security configuration](https://goose-docs.ai/docs/guides/config-files/), [detection guide](https://goose-docs.ai/docs/guides/security/prompt-injection-detection/).

Adversary mode is enabled by creating `adversary.md` in the Goose config directory; its default reviewed tool is `shell`, expandable through a `tools:` header. It uses the configured provider for contextual review and **fails open** if the reviewer fails. This is an optional model judgment, not kernel enforcement. [Adversary mode](https://goose-docs.ai/docs/guides/security/adversary-mode/).

The official extension-allowlist guide advertises `GOOSE_ALLOWLIST=https://…/allowlist.yaml` for both Desktop and CLI. However, the release's fetch/check implementation is in **Desktop TypeScript**, and repository searches found no corresponding CLI/Rust consumer. The v1.57.0 HTTPS/redirect/fail-closed release note describes that fetcher. Treat CLI enforcement as **unresolved and unsafe to rely on** until a CLI-specific runtime check proves rejection; an installation allowlist would still not constrain an already running shell. [Allowlist guide](https://goose-docs.ai/docs/guides/allowlist/), [fetcher](https://github.com/aaif-goose/goose/blob/v1.57.0/ui/desktop/src/allowlistFetcher.ts), [release notes](https://github.com/aaif-goose/goose/releases/tag/v1.57.0).

## Evidence limits and checks to resolve them

The default absence is supported by ordinary file/process execution, explicit Auto defaults, and extension placement code, plus upstream's recommendation to supply a dedicated VM/container. It is not a claim that every server, provider, or operator environment lacks isolation. Browser iframe/CSP sandboxes for MCP Apps and Desktop packaging are different surfaces from CLI tool execution. [Upstream security policy](https://github.com/aaif-goose/goose/blob/v1.57.0/SECURITY.md).

No cross-platform runtime containment tests were performed. The most useful follow-up is a credential-free disposable setup with sentinel files inside/outside the workspace: test `write`/`edit` and shell under `--container`, repeat through a subagent, and record process locations on macOS, Linux, native Windows, and WSL2. Separately test stopped-container failure without other tools enabled. For the allowlist discrepancy, set an HTTPS list excluding a harmless test server and attempt to add/start it through CLI configuration and `--with-extension`; inspect both rejection and fetch-failure behavior. For Container Use remote service and ACP provider boundaries, inspect the exact installed service/provider versions and authentication before claiming their enforcement or account requirements.
