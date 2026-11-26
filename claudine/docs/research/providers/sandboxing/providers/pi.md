---
$schema: "./_schema.yaml"
schema_revision: 5
provider: "pi"
created: "2026-11-04"
last_updated: "2026-11-04"
content_policy:
  - "ValidFor(14d)"
agent: "codex"
model: "gpt-6.1-sol"
versions_examined:
  - "1.4.0"
  - "1.5.0"
evidence:
  -
    id: "security"
    method: "official_docs"
    location: "https://pi.dev/docs/latest/security"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Direct Pi uses its user permissions without per-call approval; project trust does not sandbox tools."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "trust-source"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/project-trust.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "CLI trust override precedes extension, saved decision, and defaultProjectTrust; unresolved headless ask declines trust."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "trust-choices"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/trust-manager.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Trust decisions can be session-only or saved and apply to canonical directory paths and ancestors."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "cli"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/cli.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Documents exact tool-selection, extension, MCP, trust, offline, and session flags; no core sandbox or YOLO flag."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "settings"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/settings.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "defaultTools defaults to read/bash/edit/write; defaultProjectTrust defaults to ask and is user-only; shell settings are not containment."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "configuration"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/configuration.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Agent and trusted project configuration locations, resource merging, auth.json, and pre-trust sessionDir exception."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "file-paths"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/path-utils.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Tool paths resolve relative to cwd, expand home, and accept absolute paths without workspace containment."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "read-source"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/read.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Default read uses Node readFile/access without a Pi-specific confidentiality boundary."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "write-source"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/write.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Default write uses Node mkdir/writeFile on resolved paths; filesystem errors return through tool execution."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "edit-source"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/edit.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Default edit reads and writes with Node APIs and checks ordinary OS read/write access."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "shell-source"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/bash.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Shell executes via child_process.spawn with inherited environment and optional operation/spawn hooks, not an OS sandbox."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "environment"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/environment-variables.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "PI_OFFLINE controls automatic networking; agent/session paths and codemode worker limit are process settings, not filesystem or network isolation."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "windows"
    method: "official_docs"
    location: "https://pi.dev/docs/latest/windows"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Pi supports native Windows separately from WSL; Bash discovery and optional native PowerShell are documented."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "isolation-guide"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/containerization.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Official opt-in routes are whole-process Docker, Docker Sandboxes, OpenShell, and tool-only Gondolin; none is a clean-install default."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "gondolin-source"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/gondolin/index.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Seven tools and user Bash route through ensureVm; workspace is mounted read/write and shell env forwarded; unrelated tools remain host-side."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "gondolin-platforms"
    method: "official_docs"
    location: "https://github.com/earendil-works/gondolin#readme"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Gondolin documents macOS/Linux, QEMU requirements, automatic guest assets, and optional libkrun; Pi example requires Node 23.6+."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "gondolin-security"
    method: "official_docs"
    location: "https://earendil-works.github.io/gondolin/security/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Guest Linux compute is separated by QEMU; host network/VFS mediation and internal IP blocking do not imply a Pi-configured domain allowlist."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "gondolin-vfs"
    method: "official_docs"
    location: "https://earendil-works.github.io/gondolin/vfs/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Host filesystem mounts are implemented by programmable VFS providers; Pi-specific symlink behavior still requires validation."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "docker-engine"
    method: "official_docs"
    location: "https://docs.docker.com/engine/security/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Linux namespaces, network separation, cgroups, and mount/daemon configuration define the container boundary."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "docker-wsl"
    method: "official_docs"
    location: "https://docs.docker.com/desktop/features/wsl/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Linux-container CLI works from Windows terminals and integrated WSL2 distributions; WSL 2.1.5+ is required for this backend."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "sbx-install"
    method: "official_docs"
    location: "https://docs.docker.com/ai/sandboxes/install/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Local sbx requires supported macOS/Windows/Ubuntu hosts and hypervisor setup plus Docker login; cloud guides require sbx 0.45.0+."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "sbx-accounts"
    method: "official_docs"
    location: "https://docs.docker.com/ai/sandboxes/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Local compute is free, cloud compute is paid, and organization governance is separately paid."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "sbx-security"
    method: "official_docs"
    location: "https://docs.docker.com/ai/sandboxes/security/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "MicroVM boundary with explicit workspace shares, credential proxy, network rules, SSH-agent forwarding, and external host MCP gateway."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "sbx-kit"
    method: "official_docs"
    location: "https://github.com/docker/sbx-kits-contrib/tree/main/pi"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Pi kit setup documents provider credentials including optional Anthropic subscription-token setup."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "openshell-platforms"
    method: "official_docs"
    location: "https://docs.nvidia.com/openshell/latest/about/support-matrix"
    version: "0.1.7"
    observed_on: "2026-11-04"
    claim: "OpenShell 0.1.7 supports Linux/macOS, experimental WSL2, separate native Windows branch; runtime requires Landlock/seccomp and backend prerequisites."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "openshell-denials"
    method: "official_docs"
    location: "https://docs.nvidia.com/openshell/latest/about/run-your-first-agent"
    version: "0.1.7"
    observed_on: "2026-11-04"
    claim: "Gateway policy denies unapproved destinations and supports reviewed rule proposals and sandboxed retries."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "legacy-sandbox"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/examples/extensions/sandbox/index.ts"
    version: "1.4.0"
    observed_on: "2026-11-04"
    claim: "Optional Bash-only native sandbox uses sandbox-runtime; project config overrides global; unsupported platforms and init failures fall back unsandboxed."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "release"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/CHANGELOG.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "1.5.0 on 2026-11-02 removed the native sandbox example and dependency; pinned main also includes unreleased changes."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "codemode"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/codemode.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "QuickJS has no direct Node/filesystem/network APIs but delegates real tool/model calls; completed effects are not rolled back."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "permission-example"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/permission-gate.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Optional tool_call handler prompts for selected Bash patterns and blocks those patterns without UI; not OS isolation."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "mcp"
    method: "official_docs"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/mcp.md"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Built-in stdio and HTTP MCP, trusted project overrides, and credential-command resolution operate with their respective process/service permissions."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "subagent"
    method: "source_code"
    location: "https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/subagent/index.ts"
    version: "1.5.0"
    observed_on: "2026-11-04"
    claim: "Example spawns another Pi process with selected cwd/tools and no OS isolation launcher."
    limitations: "Documentation/source inspection only; no containment or cross-platform runtime test."
  -
    id: "local-cli"
    method: "local_inspection"
    location: "claudine/docs/research/sandboxing/providers/pi.md#scope-and-evidence"
    version: "1.4.0"
    observed_on: "2026-11-04"
    claim: "sniff software agents found /Users/ken/.bun/bin/pi; pi --version returned 1.4.0 and help corroborated trust, tool, extension, offline, and MCP flags."
    limitations: "Read-only help/version observations on macOS only; no user config inspected, model request, or containment probe."
summary: "No clean-install OS isolation or per-call approval on any examined platform/session. Project trust and tool exposure are guardrails; optional official Docker/Sandboxes/OpenShell/Gondolin integrations supply external boundaries. QuickJS confines scripts only; legacy 1.4.0 Bash sandbox example fails open and was removed in 1.5.0."
support: "official_integration"
default_profiles:
  -
    id: "macos-interactive"
    environment: "macos"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. First protected project prompts under default ask; trust changes resource loading only."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "file-paths"
  -
    id: "macos-non-interactive"
    environment: "macos"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. Print/JSON/RPC cannot show built-in trust prompt; unresolved ask skips protected resources while enabled tools retain user permissions."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "file-paths"
  -
    id: "linux-interactive"
    environment: "linux"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. First protected project prompts under default ask; trust changes resource loading only."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "file-paths"
  -
    id: "linux-non-interactive"
    environment: "linux"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. Print/JSON/RPC cannot show built-in trust prompt; unresolved ask skips protected resources while enabled tools retain user permissions."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "file-paths"
  -
    id: "windows-interactive"
    environment: "windows"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. First protected project prompts under default ask; trust changes resource loading only. Native Windows Bash normally needs Git Bash; optional PowerShell is not isolation."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "windows"
  -
    id: "windows-non-interactive"
    environment: "windows"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. Print/JSON/RPC cannot show built-in trust prompt; unresolved ask skips protected resources while enabled tools retain user permissions. Native Windows Bash normally needs Git Bash; optional PowerShell is not isolation."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "windows"
  -
    id: "wsl2-interactive"
    environment: "wsl2"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. First protected project prompts under default ask; trust changes resource loading only. Linux-in-WSL environment and accessible mounts are not a Pi-imposed boundary."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "windows"
  -
    id: "wsl2-non-interactive"
    environment: "wsl2"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "No Pi OS sandbox; read/bash/edit/write use account permissions. Print/JSON/RPC cannot show built-in trust prompt; unresolved ask skips protected resources while enabled tools retain user permissions. Linux-in-WSL environment and accessible mounts are not a Pi-imposed boundary."
    evidence_ids:
      - "security"
      - "trust-source"
      - "cli"
      - "settings"
      - "shell-source"
      - "windows"
mechanisms:
  -
    id: "docker"
    name: "Plain Docker"
    enforcement: "container"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "Linux container runtime; Docker namespaces/network separation/cgroups, with Linux virtualization on non-Linux hosts."
    prerequisites:
      - "A working Linux-container runtime and permission to run containers"
      - "Build documented Node 24 Debian image with Pi and shell tools"
      - "Provider credentials or reachable compatible local model; Docker Desktop WSL backend requires WSL 2.1.5+ and integration"
    activation: "Opt-in external docker run; entire Pi runs inside container, not a core Pi flag."
    filesystem: "Container filesystem plus explicit mounts; official workspace bind is read/write, agent home is named volume. No special per-file Pi denylist; do not mount host home/credentials unless intended. Read-only shares still disclose content; symlink guarantees untested."
    network: "Recipe has no destination allowlist or credential proxy. Runtime owns egress/ingress; agent API and tool traffic originate inside container."
    processes: "Pi and local children share container boundary; passed credentials/env are readable inside. Host processes remain outside namespaces unless explicitly exposed."
    coverage: "Contains shell/file tools, Pi hooks/extensions, local subagent children and stdio MCP launched inside; remote MCP execution remains remote."
    escape_hatches: "Host chooses mounts/runtime flags or launches Pi outside. Pi --approve cannot remove container isolation; no documented automatic host retry."
    failure_behavior: "unknown"
    failure_notes: "No containment/startup-failure test or complete backend failure contract examined; docker launch is external to Pi in interactive and headless modes."
    evidence_ids:
      - "isolation-guide"
      - "docker-engine"
      - "docker-wsl"
      - "mcp"
  -
    id: "docker-sandboxes"
    name: "Docker Sandboxes"
    enforcement: "virtual_machine"
    environments:
      - "macos"
      - "linux"
      - "windows"
    backend: "sbx-managed Linux microVM with host credential/network proxy; native Windows host requires Windows Hypervisor Platform."
    prerequisites:
      - "Install sbx and sign in to Docker"
      - "macOS 14+ Apple silicon, Windows 11 x64, or Ubuntu 24.04+ x64/Arm with KVM for local compute"
      - "Configure provider secrets before creation; paid cloud compute or centralized governance are optional"
    activation: "Opt-in sbx run --kit docker.io/sbx/pi-kit:latest pi; sbx exec supports headless Pi."
    filesystem: "Direct workspace share is read/write; clone/mountless modes narrow exposure. Other explicit skills/socket shares can cross boundary. No claim of tested symlink containment."
    network: "Host proxy enforces network policy for both agent and tools; broad wildcard defaults merit review with sbx policy ls. Real provider keys can stay outside VM."
    processes: "Whole Pi process and local children inside VM; host MCP gateway services and credential proxy outside. Forwarded SSH agent permits signatures without copying keys."
    coverage: "Contains file/shell tools, hooks/extensions and local children; registered gateway stdio MCP executes on host and remote MCP remains external."
    escape_hatches: "Host changes shares/policy or runs unsandboxed outside sbx; Pi trust/approval flags do not remove VM enforcement. Explicit host integrations retain their own authority."
    failure_behavior: "unknown"
    failure_notes: "Pi documents launch and headless exec but backend-unavailable behavior was not tested; no verified Pi-specific host fallback."
    evidence_ids:
      - "isolation-guide"
      - "sbx-install"
      - "sbx-accounts"
      - "sbx-security"
      - "sbx-kit"
  -
    id: "openshell"
    name: "OpenShell"
    enforcement: "remote_environment"
    environments:
      - "macos"
      - "linux"
      - "wsl2"
    backend: "Local or remote gateway-managed Linux workload; Docker/Podman/Kubernetes/microVM runtimes. OpenShell 0.1.7 requires Landlock ABI 3+ and seccomp user-notification capabilities."
    prerequisites:
      - "Active configured gateway and Pi-containing workload image"
      - "Selected runtime prerequisites; Docker-backed setups require Docker 28+"
      - "Linux or Apple-silicon macOS; WSL2 support experimental"
      - "Provider configuration or inference-routing endpoint"
    activation: "Opt-in openshell sandbox create --name pi-sandbox --from pi -- pi after selecting gateway; Pi recipe not executed here."
    filesystem: "Policy-managed sandbox filesystem; remote gateway does not automatically mount host cwd. Upload/download or clone inside. Exact selected policy, protected/temp paths, and symlink outcomes require deployment verification."
    network: "Gateway policy controls services, credentials and inference routing; denied destinations can yield proposals. Model route must be configured; not a tool-only firewall."
    processes: "Pi process tree belongs to workload boundary; provider credentials may stay in external supervisor. Remote services keep independent privileges."
    coverage: "Pi tools, user shell commands, extensions/hooks and local children inside; external MCP services remain external."
    escape_hatches: "Operator can grant policy rules then retry inside workload. Pi --approve does not grant host execution or bypass gateway policy."
    failure_behavior: "unknown"
    failure_notes: "Policy denials documented; Pi image alias, unavailable-runtime startup and headless launch not executed against a pinned gateway."
    evidence_ids:
      - "isolation-guide"
      - "openshell-platforms"
      - "openshell-denials"
  -
    id: "gondolin"
    name: "Gondolin tool-routing example"
    enforcement: "virtual_machine"
    environments:
      - "macos"
      - "linux"
    backend: "Gondolin local Linux microVM via QEMU; programmable host VFS/network mediation. Pi example mounts RealFSProvider at /workspace."
    prerequisites:
      - "Node.js 23.6+"
      - "QEMU installed separately"
      - "Install official example extension dependencies; guest assets downloaded on first use"
    activation: "Opt-in pi -e ~/.pi/agent/extensions/gondolin after installing the example; no core sandbox flag."
    filesystem: "Mounted cwd is read/write through to host; other guest storage isolated. Host Pi can still read home/config. Symlink confinement of this example is untested."
    network: "Host-mediated HTTP/TLS and internal-range blocking; example sets no custom destination allowlist. Agent API transport stays on host; environment API keys are forwarded to guest shell."
    processes: "Only delegated operations/guest subprocesses inside VM; Pi, other extensions and MCP stay host-side. Environment forwarding is not secret filtering."
    coverage: "Overrides read/write/edit/bash/grep/find/ls and user Bash. Does not override optional PowerShell, arbitrary extension tools, independent subagents, or MCP."
    escape_hatches: "Unrelated host tools already remain outside. Removing/disabling extension removes routing; no approval flag toggles VM. Replacement tool executor has no host retry."
    failure_behavior: "conditional"
    failure_notes: "ensureVm rejection prevents replacement tool execution and has no local fallback; global extension-load/session-start error handling was not runtime-tested in headless or interactive mode."
    evidence_ids:
      - "gondolin-source"
      - "gondolin-platforms"
      - "gondolin-security"
      - "gondolin-vfs"
  -
    id: "legacy-native"
    name: "sandbox/ extension example (Pi 1.4.0; removed in 1.5.0)"
    enforcement: "os_sandbox"
    environments:
      - "macos"
      - "linux"
      - "wsl2"
    backend: "@anthropic-ai/sandbox-runtime; documented macOS sandbox-exec and Linux bubblewrap."
    prerequisites:
      - "Manually install versioned example and npm dependency"
      - "Linux needs bubblewrap, socat, ripgrep; WSL2 follows Linux detection but backend availability must be checked"
    activation: "Historical opt-in pi -e ./sandbox; not loaded by a clean installation; removed in 1.5.0."
    filesystem: "Bash defaults permit writes to cwd and /tmp, deny reads of ~/.ssh ~/.aws ~/.gnupg, deny writes to .env .env.* *.pem *.key. Other readable credentials may remain visible; file tools outside boundary."
    network: "Bash network allowedDomains defaults include npm/PyPI/GitHub destinations. Agent/model transport and unrelated tools remain outside."
    processes: "Only wrapped Bash and descendants; Pi process/extensions retain host authority and inherited environment is not a general secret boundary."
    coverage: "Bash tool and user Bash only; built-in files, extension tools, MCP and independently launched subagents outside."
    escape_hatches: "Extension --no-sandbox or enabled:false disables; unsupported platform or initialization failure automatically leaves original host Bash active."
    failure_behavior: "fallback_unsandboxed"
    failure_notes: "Source catches initialization errors and notifies, then Bash executor delegates locally if sandboxEnabled or sandboxInitialized is false; no UI approval guard on fallback."
    evidence_ids:
      - "legacy-sandbox"
      - "release"
controls:
  -
    id: "approve"
    kind: "cli_flag"
    name: "--approve"
    values:
      - "-a"
    effect: "Trust project resources for one run; no sandbox/command-approval change."
    example: "pi --approve -p \"Run tests\""
    precedence: "CLI trust override precedes extension, saved decision and global default."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "trust-source"
  -
    id: "no-approve"
    kind: "cli_flag"
    name: "--no-approve"
    values:
      - "-na"
    effect: "Skip protected project resources; tool permissions unchanged."
    example: "pi --no-approve -p \"Review project\""
    precedence: "One-run trust override; pre-trust sessionDir lookup remains."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "trust-source"
      - "configuration"
  -
    id: "project-trust"
    kind: "config_key"
    name: "defaultProjectTrust"
    values:
      - "ask"
      - "always"
      - "never"
    effect: "Resource-loading trust only; ask skips protected resources without UI."
    example: "{\"defaultProjectTrust\":\"never\"}"
    precedence: "User-level settings only; CLI, extension and nearest saved decision precede it."
    mechanism_ids: []
    evidence_ids:
      - "settings"
      - "trust-source"
  -
    id: "trust-command"
    kind: "session_command"
    name: "/trust"
    values: []
    effect: "Save project trust decision; no OS isolation."
    example: "/trust"
    precedence: "Saved canonical directory decision applies to future processes unless higher-priority override applies."
    mechanism_ids: []
    evidence_ids:
      - "trust-choices"
      - "security"
  -
    id: "tools"
    kind: "cli_flag"
    name: "--tools"
    values:
      - "comma-separated tool names or * patterns"
      - "only +name/-name entries modify defaults"
    effect: "Select model-visible tools; ordinary lists preserve MCP reach unless mcp__ entries filter it. Read-only tools do not establish confidentiality."
    example: "pi --no-mcp --tools read,grep,find,ls -p \"Review src\""
    precedence: "Overrides defaultTools; additive entries modify it; exclude-tools applies last."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "settings"
  -
    id: "default-tools"
    kind: "config_key"
    name: "defaultTools"
    values:
      - "array of names/patterns or only +name/-name entries"
    effect: "Default tool exposure, not a path or OS policy."
    example: "{\"defaultTools\":[\"read\",\"bash\",\"edit\",\"write\"]}"
    precedence: "Trusted project settings override user selection; CLI overrides; empty array does not disable extension tools."
    mechanism_ids: []
    evidence_ids:
      - "settings"
      - "cli"
  -
    id: "exclude-tools"
    kind: "cli_flag"
    name: "--exclude-tools"
    values:
      - "comma-separated names or * patterns"
    effect: "Disable tools including MCP, without isolating remaining execution."
    example: "pi --exclude-tools \"mcp__*\" -p \"Review\""
    precedence: "Applied after other tool-selection options."
    mechanism_ids: []
    evidence_ids:
      - "cli"
  -
    id: "no-tools"
    kind: "cli_flag"
    name: "--no-tools"
    values: []
    effect: "Initially disable model-callable tools; extension startup code remains privileged."
    example: "pi --no-tools -p \"Explain this text\""
    precedence: "Overrides defaultTools for invocation; not an immutable security policy."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "settings"
  -
    id: "no-builtin-tools"
    kind: "cli_flag"
    name: "--no-builtin-tools"
    values: []
    effect: "Disable default built-ins while retaining extension/custom tools."
    example: "pi --no-builtin-tools -p \"Review\""
    precedence: "Invocation selection override; does not isolate retained tools."
    mechanism_ids: []
    evidence_ids:
      - "cli"
  -
    id: "extension"
    kind: "cli_flag"
    name: "--extension"
    values:
      - "-e PATH"
      - "-e builtin:NAME"
    effect: "Load executable extension; can activate Gondolin or historical sandbox routing."
    example: "pi -e ~/.pi/agent/extensions/gondolin"
    precedence: "Repeatable explicit paths still load with --no-extensions; trusted project/user extension configuration can also load resources."
    mechanism_ids:
      - "gondolin"
      - "legacy-native"
    evidence_ids:
      - "cli"
      - "gondolin-source"
      - "legacy-sandbox"
  -
    id: "no-extensions"
    kind: "cli_flag"
    name: "--no-extensions"
    values: []
    effect: "Suppress discovery/configured/built-in extensions; can remove optional sandbox routing."
    example: "pi --no-extensions -p \"Review\""
    precedence: "Explicit -e still loads; this is not an administrator lock."
    mechanism_ids:
      - "gondolin"
      - "legacy-native"
    evidence_ids:
      - "cli"
      - "settings"
  -
    id: "no-mcp"
    kind: "cli_flag"
    name: "--no-mcp"
    values: []
    effect: "Disable built-in MCP connections/tools; does not affect an extension replacing MCP."
    example: "pi --no-mcp -p \"Review\""
    precedence: "One invocation; third-party replacement behavior is separate."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "mcp"
  -
    id: "no-context"
    kind: "cli_flag"
    name: "--no-context-files"
    values: []
    effect: "Disable instruction-file discovery; not path access enforcement."
    example: "pi --no-context-files -p \"Review\""
    precedence: "One-run override; declining trust alone does not disable context."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "security"
  -
    id: "no-skills"
    kind: "cli_flag"
    name: "--no-skills"
    values: []
    effect: "Disable discovered/configured skills; no filesystem enforcement."
    example: "pi --no-skills -p \"Review\""
    precedence: "Explicit --skill paths still load."
    mechanism_ids: []
    evidence_ids:
      - "cli"
  -
    id: "agent-dir"
    kind: "environment_variable"
    name: "PI_CODING_AGENT_DIR"
    values:
      - "directory path"
    effect: "Relocate config/trust/auth storage; does not protect old/new paths."
    example: "PI_CODING_AGENT_DIR=/tmp/pi-agent pi --help"
    precedence: "Overrides default ~/.pi/agent for process; trusted project config still applies."
    mechanism_ids: []
    evidence_ids:
      - "configuration"
      - "environment"
  -
    id: "offline"
    kind: "cli_flag"
    name: "--offline"
    values: []
    effect: "Disable startup network work; does not firewall tools or model transport."
    example: "pi --offline -p \"Review\""
    precedence: "Equivalent to process PI_OFFLINE=1; no sandbox precedence."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "environment"
  -
    id: "offline-env"
    kind: "environment_variable"
    name: "PI_OFFLINE"
    values:
      - "1"
      - "true"
      - "yes"
    effect: "Disable automatic networking; no tool-network isolation."
    example: "PI_OFFLINE=1 pi --help"
    precedence: "Process-wide; --offline also enables it."
    mechanism_ids: []
    evidence_ids:
      - "cli"
      - "environment"
  -
    id: "shell-path"
    kind: "config_key"
    name: "shellPath"
    values:
      - "executable path"
    effect: "Choose shell executable, not an enforced sandbox backend."
    example: "{\"shellPath\":\"/bin/bash\"}"
    precedence: "Trusted project ordinary setting overrides user; native Windows uses this before Git Bash discovery."
    mechanism_ids: []
    evidence_ids:
      - "settings"
      - "windows"
  -
    id: "shell-prefix"
    kind: "config_key"
    name: "shellCommandPrefix"
    values:
      - "command prefix string"
    effect: "Prefix shell commands only; not coverage for file tools/extensions."
    example: "{\"shellCommandPrefix\":\"set -o pipefail;\"}"
    precedence: "Trusted project overrides user; shell scope only."
    mechanism_ids: []
    evidence_ids:
      - "settings"
  -
    id: "codemode-mode"
    kind: "config_key"
    name: "codemode.mode"
    values:
      - "on"
      - "only"
    effect: "Change tool presentation through QuickJS; delegated tools preserve their authority."
    example: "{\"codemode\":{\"mode\":\"only\"}}"
    precedence: "Applies when codemode active; not OS/container/VM enforcement."
    mechanism_ids: []
    evidence_ids:
      - "settings"
      - "codemode"
  -
    id: "codemode-workers"
    kind: "environment_variable"
    name: "PI_CODEMODE_MAX_WORKERS"
    values:
      - "positive worker count"
    effect: "Set process-wide QuickJS worker capacity, default two; no host permission change."
    example: "PI_CODEMODE_MAX_WORKERS=4 pi --tools +codemode"
    precedence: "Process environment applies across sessions."
    mechanism_ids: []
    evidence_ids:
      - "environment"
      - "codemode"
  -
    id: "legacy-no-sandbox"
    kind: "cli_flag"
    name: "--no-sandbox"
    values: []
    effect: "Historical extension switch relaxes Bash isolation; not current core CLI."
    example: "pi -e ./sandbox --no-sandbox"
    precedence: "In 1.4.0 example overrides enabled config; flag exists only when extension loaded."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
      - "release"
  -
    id: "legacy-config"
    kind: "config_key"
    name: "enabled"
    values:
      - "true"
      - "false"
    effect: "Historical sandbox.json switch disables Bash wrapper when false."
    example: "{\"enabled\":false}"
    precedence: "Legacy defaults < agent-dir/extensions/sandbox.json < cwd/.pi/sandbox.json; --no-sandbox wins."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
  -
    id: "legacy-allow-domains"
    kind: "config_key"
    name: "network.allowedDomains"
    values:
      - "domain pattern array"
    effect: "Replace allowed Bash destinations."
    example: "{\"network\":{\"allowedDomains\":[\"github.com\"]}}"
    precedence: "Version 1.4.0 sandbox.json only; project overrides global/default; arrays replace within merged objects."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
  -
    id: "legacy-deny-domains"
    kind: "config_key"
    name: "network.deniedDomains"
    values:
      - "domain pattern array"
    effect: "Set explicit Bash destination denials."
    example: "{\"network\":{\"deniedDomains\":[\"example.com\"]}}"
    precedence: "Version 1.4.0 sandbox.json only; project overrides global/default; arrays replace within merged objects."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
  -
    id: "legacy-deny-read"
    kind: "config_key"
    name: "filesystem.denyRead"
    values:
      - "path pattern array"
    effect: "Deny selected Bash reads; file tools retain host access."
    example: "{\"filesystem\":{\"denyRead\":[\"~/.ssh\"]}}"
    precedence: "Version 1.4.0 sandbox.json only; project overrides global/default; arrays replace within merged objects."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
  -
    id: "legacy-allow-write"
    kind: "config_key"
    name: "filesystem.allowWrite"
    values:
      - "path array"
    effect: "Set Bash writable roots."
    example: "{\"filesystem\":{\"allowWrite\":[\".\",\"/tmp\"]}}"
    precedence: "Version 1.4.0 sandbox.json only; project overrides global/default; arrays replace within merged objects."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
  -
    id: "legacy-deny-write"
    kind: "config_key"
    name: "filesystem.denyWrite"
    values:
      - "path pattern array"
    effect: "Deny selected Bash writes."
    example: "{\"filesystem\":{\"denyWrite\":[\".env\"]}}"
    precedence: "Version 1.4.0 sandbox.json only; project overrides global/default; arrays replace within merged objects."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
  -
    id: "legacy-status"
    kind: "session_command"
    name: "/sandbox"
    values: []
    effect: "Display historical extension policy; does not activate or strengthen it."
    example: "/sandbox"
    precedence: "Available only with legacy extension; displayed file configuration is not a runtime containment test."
    mechanism_ids:
      - "legacy-native"
    evidence_ids:
      - "legacy-sandbox"
gaps:
  -
    location: "mechanisms/docker/failure_behavior"
    detail: "No disposable Docker unavailable-runtime or containment run; Pi recipe does not specify complete failure handling or symlink guarantees."
    next_check: "Run pinned image with missing daemon and harmless inside/outside/symlink marker probes in print mode; verify mount/network behavior."
  -
    location: "mechanisms/docker-sandboxes"
    detail: "WSL2 local support and backend-unavailable behavior unresolved; mutable kit/backend versions were not pinned or launched."
    next_check: "Obtain explicit WSL2 support statement, pin sbx and kit image, then test supported-host interactive/headless startup and policy denials."
  -
    location: "mechanisms/openshell"
    detail: "Pi image alias, headless startup, exact gateway policy and unavailable-runtime behavior unverified; native Windows is separate branch work."
    next_check: "Pin OpenShell 0.1.7 gateway/image and execute Pi recipe plus headless marker tests; separately review Windows branch and test native Pi workload."
  -
    location: "mechanisms/gondolin"
    detail: "Native Windows/WSL2 example support, symlink confidentiality, dependency-resolved backend version and CLI behavior after extension-load failure remain unverified."
    next_check: "Pin extension dependencies, confirm upstream platform support, then run disposable symlink/env markers and missing-QEMU/module tests in interactive/print/RPC modes."
  -
    location: "mechanisms/legacy-native/environments"
    detail: "Legacy example admits Linux including WSL2 by process.platform, but backend availability in WSL2 was not executed."
    next_check: "On disposable WSL2 with pinned 1.4.0 example/runtime, check dependencies and marker denial; remove bubblewrap to confirm fail-open path."
  -
    location: "controls/codemode-mode"
    detail: "Revision 5 has no language-runtime enforcement category; QuickJS is documented in prose and controls but cannot be honestly typed as an OS sandbox mechanism."
    next_check: "If language-runtime isolation is needed in machine-readable comparisons, add an explicit schema category with delegated-tool coverage; do not classify it as OS isolation."
changes: []
---

# Sandboxing in Pi

Pi has **no built-in OS sandbox enabled by a normal installation**. Its default tools read, write, and execute with the permissions of the account running Pi, without per-call approval. Project trust gates loading project resources; it does not confine tool actions. This is an explicit documented limitation, corroborated by direct filesystem calls and child-process spawning in source, rather than a conclusion based on missing documentation. [Security guide](https://pi.dev/docs/latest/security), [read implementation](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/read.ts), [Bash implementation](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/bash.ts).

## Scope and evidence

Researched on **2026-11-04 UTC**, starting at [pi.dev](https://pi.dev/). Current documentation and the **1.5.0 release dated 2026-11-02** were checked against upstream commit `707888ece685778881c40d232880c5a0b93a12f1` (2026-11-04). That checkout identifies its package as 1.5.0 but also contains unreleased changes; source-derived statements below refer to that exact commit. The locally installed macOS binary at `/Users/ken/.bun/bin/pi` returned **1.4.0** from `pi --version`; `pi --help` corroborated tool-selection, trust, extension, MCP, print/RPC, and offline flags. No local settings or secrets were read, no model was called, and no sandbox was launched. Source inspection is not runtime validation. [Package manifest](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/package.json), [release history](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/CHANGELOG.md).

The version distinction matters: 1.5.0 **removed** the native OS `sandbox/` extension example and its dependency. Current guidance recommends isolated environments or Gondolin instead. The legacy example remains relevant to the observed 1.4.0 installation, but was optional there too. [1.5.0 changelog](https://pi.dev/changelog).

For a normal npm installation, current Pi requires Node.js **22.19.0 or newer**; the macOS, Linux, and Windows installers can install Node if needed. This is separate from Gondolin's higher runtime requirement. Normal npm installation does not require dependency lifecycle scripts. [Pinned package requirements](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/package.json), [installation overview](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/README.md).

## Clean-install defaults across platforms

| Platform | Interactive session | Non-interactive print, JSON, or RPC session | Platform prerequisite/difference |
|---|---|---|---|
| macOS | No OS sandbox; project-resource trust prompt when needed | No OS sandbox; unresolved project trust skips protected resources | Local shell; no automatic Seatbelt boundary |
| Linux | Same default and trust prompt | Same default and trust skipping | Local shell; no automatic namespaces, seccomp, or Landlock boundary |
| Native Windows | Same default and trust prompt | Same default and trust skipping | Git Bash normally required; optional native `powershell` tool |
| WSL2 | Same default and trust prompt | Same default and trust skipping | Linux process inside the chosen distribution; WSL2 is not a Pi sandbox |

These are documented/source-derived defaults, not four-platform execution results. The enabled default tools are `read`, `bash`, `edit`, and `write`. On native Windows, Pi discovers Git Bash in configured `shellPath`, standard Git installation locations, then `PATH`. The optional PowerShell tool runs with `-NoProfile -NonInteractive -ExecutionPolicy Bypass`, subject to administrator-enforced policies. WSL uses Linux tools; native Windows support is documented independently. WSL-accessible Windows mounts remain available to commands under the surrounding system's permissions. [CLI tools](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/cli.md#tools), [Windows guide](https://pi.dev/docs/latest/windows).

On first use of a folder with protected project resources, the default trust setting is `ask`. Interactive users can make a session-only or saved decision; a bare `.pi` directory does not trigger the question. Without an existing decision or override, headless `ask` skips those resources rather than blocking on stdin. Declining trust does **not** turn the session into a read-only or network-restricted session. See the trust resolver and its [choice definitions](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/trust-manager.ts).

## What the default process can access

The working directory supplies a base for relative paths, not a filesystem jail. File tools accept absolute paths and home expansion; relative `..` paths are resolved normally. Reads and writes use Node filesystem APIs. There is no Pi workspace allowlist, additional-directory permission grant, or special protection for home, credentials, temporary files, `.git`, or Pi configuration. Existing OS permissions still apply. Symlinks are followed by ordinary file operations; the resolver does not enforce real-path containment within the workspace. These are source-derived conclusions from [path utilities](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/path-utils.ts), [path resolution](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/utils/paths.ts), [write](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/write.ts), and [edit](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/edit.ts).

Shell tools spawn processes with a normal inherited environment plus Pi session metadata. They can execute installed programs and use available network services. File-based API keys/OAuth credentials normally reside in `<agent-dir>/auth.json`; changing `PI_CODING_AGENT_DIR` relocates that directory but does not restrict access to the old or new location. A read-only tool selection can still disclose readable credentials to the model; write protection does not establish confidentiality. [Configuration paths](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/configuration.md), [shell environment](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/bash.ts).

Pi supplies no default tool-network domain policy. `--offline`/`PI_OFFLINE=1` disables startup/automatic network work, including catalog refreshes; it is not a firewall for shell commands, extensions, MCP, or model requests. Model/API transport originates from the Pi process, independently of any tool-only isolation. A cloud model needs its provider's API key or supported OAuth account; local compatible endpoints are also possible. No separate Pi sandbox account is required for direct execution. [Environment reference](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/environment-variables.md), [provider setup](https://pi.dev/docs/latest/providers).

## Actual guardrails and their configuration

User settings live in `~/.pi/agent/settings.json` (relocatable with `PI_CODING_AGENT_DIR`); trusted `.pi/settings.json` overrides ordinary user settings, while resource lists combine. `defaultProjectTrust` is user-level only. Trust decision precedence is CLI override, first deciding user/CLI extension's `project_trust` handler, nearest saved canonical directory/ancestor decision in `<agent-dir>/trust.json`, then `defaultProjectTrust` (`ask`, `always`, `never`). `/trust` saves decisions. Trust protects project settings, MCP configuration, extensions, skills, prompts, themes, and system-prompt resources. Context files such as `AGENTS.md` still load unless disabled. Pi reads project `sessionDir` before resolving trust, an explicit exception. [Configuration guide](https://pi.dev/docs/latest/configuration), [trust resolver](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/project-trust.ts).

For example, these are separate controls:

```sh
# Skip trust-gated project resources for this run.
pi --no-approve -p "Review this project"

# Reduce exposed model tools and remove extension/MCP/context loading.
pi --no-extensions --no-mcp --no-context-files --no-skills \
  --tools read,grep,find,ls -p "Review src"

# Explicitly trust project resources for one automated invocation.
pi --approve -p "Run the tests"
```

`--approve` (`-a`) and `--no-approve` (`-na`) control project loading, not per-command approval. There is no core YOLO or sandbox-bypass switch in the examined CLI. `--tools` replaces tool selection; lists containing only `+name`/`-name` modify defaults instead. `--exclude-tools` applies last. Crucially, ordinary `--tools read,...` selections retain MCP reach unless entries begin with `mcp__`; use `--no-mcp` or exclude `mcp__*` explicitly. `--no-tools` disables all model tools initially, but does not sandbox extension initialization. `--no-extensions` still permits explicit `-e` paths. These are configurable exposure controls, not administrator-enforced isolation. [CLI reference](https://pi.dev/docs/latest/cli).

Extensions can implement approvals through `tool_call` events. The official `permission-gate.ts` example checks selected dangerous Bash patterns, asks through the UI, and blocks matching calls when there is no UI. It is neither installed automatically nor a complete command-security policy. Plan-mode extensions likewise change available tools/workflow rather than OS permissions. Pi has no core administrator-managed sandbox policy; administrator controls belong to the host or a selected external backend. [Permission-gate source](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/permission-gate.ts), [plan-mode example](https://github.com/earendil-works/pi/tree/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/plan-mode).

The ordinary `shellPath` setting chooses a shell executable; `shellCommandPrefix` prepends text to shell commands. Neither restricts file tools or extension code. For example, `{"shellPath":"/bin/bash","shellCommandPrefix":"set -o pipefail;"}` changes shell behavior without adding containment. [Shell settings](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/settings.md#shell-and-updates).

## Which execution surfaces share a boundary?

| Surface | Normal installation | With Gondolin example | Entire Pi inside container/VM |
|---|---|---|---|
| Bash and user `!` commands | Host permissions | Routed into VM | Inside selected environment |
| `read`, `write`, `edit`, `grep`, `find`, `ls` | Host filesystem | Replaced by VM-backed operations | Inside selected environment |
| Optional native PowerShell | Host permissions | Not replaced by this example | Depends on workload OS; Linux images do not supply native Windows tool |
| Extensions and event hooks | Host, in Pi process | Still host; custom tools can bypass VM | Inside environment if loaded there |
| Subagent extension/SDK sessions | No automatic OS isolation | No automatic inheritance of routing | Children launched inside remain inside; external services keep their own boundary |
| MCP | Local stdio servers use local process permissions; HTTP servers execute remotely | Not intercepted by Gondolin overrides | Local child servers inside, except explicit host gateways; remote servers remain external |

Pi's extension subagent example spawns another Pi process and can choose its working directory/tools; it is not an OS sandbox. A parent tool wrapper is not automatically applied to that new process. [Subagent implementation](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/subagent/index.ts). MCP is now built in: user `<agent-dir>/mcp.json` and trusted project `.pi/mcp.json` define stdio or HTTP servers; project entries override same-name user entries. Configuration can execute commands to resolve credentials. These commands and extension hooks retain the Pi process's privileges unless the whole process is isolated. [MCP reference](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/mcp.md).

Codemode is a separate, narrow language-runtime boundary. It is off as an active tool by default, though MCP can activate it. `pi --tools +codemode` enables it explicitly. Its QuickJS scripts lack Node APIs, direct filesystem/network access, and timers, but can call session tools and credential-backed `models` APIs. Delegation therefore preserves those tools' existing host access; it does not isolate the agent. The script has a 256 MB limit, optional `timeout_ms`, and a default two-worker process-wide limit (`PI_CODEMODE_MAX_WORKERS`). Failures do not undo completed tool effects. [Codemode contract](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/codemode.md), [worker implementation](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/extensions/codemode/worker.ts).

## Officially documented optional isolation

These are opt-in launch environments and an official extension example, not a clean-install sandbox mode or built-in `--sandbox` flag. Their policies are configured outside Pi. [Pi isolation guide](https://pi.dev/docs/latest/containerization).

```mermaid
flowchart LR
  H[Host Pi, credentials, extensions] -->|Gondolin overrides| T[Linux VM: seven file/shell tools and user commands]
  H -->|Other tools and MCP| X[Host processes or remote servers]
  W[Whole-process launch] --> B[Container or VM: Pi, extensions, local children]
  B --> M[Explicitly shared workspace]
  B --> N[Backend-controlled network or inference proxy]
```

### Plain Docker

The official recipe builds a Linux image from `node:24-bookworm-slim`, installs Bash, Git, ripgrep and Pi, and starts the whole process inside a container. It needs a functioning Linux-container runtime; this is not a native Windows container recipe. Host filesystem exposure is determined by mounts. For a prepared `pi-sandbox` image, a headless adaptation of the documented launch is:

```sh
docker run --rm -e ANTHROPIC_API_KEY \
  -v "$PWD:/workspace" -v pi-agent-home:/root/.pi/agent \
  pi-sandbox -p "Review this project"
```

The workspace is writable through to the host. Add `:ro` to make that mount read-only, or copy files instead. The named agent-home volume avoids mounting host credentials, but the passed API key is readable inside the container. Plain Docker's recipe supplies no Pi-specific egress allowlist or credential proxy. Network and mount restrictions must come from runtime configuration; a read-only mount still discloses its contents. [Pinned Docker recipe](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/containerization.md#run-pi-in-plain-docker).

Docker supplies Linux namespaces, network separation, and cgroups; it does not turn every outbound destination into an approval decision. Docker Desktop makes Linux containers usable from Windows terminals and WSL2 distributions; its WSL backend requires WSL 2.1.5 or newer and enabled distribution integration. Thus a Windows-hosted Linux container is distinct from sandboxing Pi as a native Windows process. On macOS, the recipe likewise requires a Linux-container runtime rather than a Pi-specific macOS sandbox. [Docker isolation](https://docs.docker.com/engine/security/), [Windows and WSL2 integration](https://docs.docker.com/desktop/features/wsl/).

### Docker Sandboxes

Pi documents `sbx run --kit "docker.io/sbx/pi-kit:latest" pi`, and `sbx exec <sandbox-name> -- pi -p "list the failing tests"` for headless use. This contains the whole agent in a microVM. The current `sbx` installer requires macOS 14+ on Apple silicon, Windows 11 x64 with Windows Hypervisor Platform, or Ubuntu 24.04+ with KVM for local execution; WSL2 is not separately established by that support list. Docker sign-in is required. Local compute is free, cloud compute uses a paid subscription, and centralized governance is separately paid. [Installation](https://docs.docker.com/ai/sandboxes/install/), [service/account scope](https://docs.docker.com/ai/sandboxes/).

The host proxy can substitute real provider credentials into allowed requests, leaving placeholders inside. Pi warns against `/login` inside the VM because that stores a real credential there. Anthropic API-key setup uses `sbx secret set anthropic`; Claude Pro/Max token setup requires Claude Code's `claude setup-token` and the documented `sbx secret set-custom --host api.anthropic.com --env ANTHROPIC_OAUTH_TOKEN --placeholder 'sk-ant-oat01-{rand}'`. This is a provider-specific option, not a universal Claude subscription requirement. [Pi kit setup](https://github.com/docker/sbx-kits-contrib/tree/main/pi).

Direct workspace mounts remain writable. The proxy governs tool and agent egress, including model transport; default destination rules include broad wildcards. Inspect `sbx policy ls`; organizations can centrally manage filesystem, network, and MCP policies. A major exception is the host MCP gateway: local stdio MCP servers registered there execute **on the host**, outside the VM. SSH-agent forwarding can grant signing operations without exposing private-key bytes. Neither feature should be mistaken for inaccessible host credentials. [Docker security model](https://docs.docker.com/ai/sandboxes/security/).

Docker Sandboxes also offers clone mode, which shares the original repository read-only while Pi works on a private clone, and a mountless mode. These alternatives change host write exposure rather than merely asking Pi to avoid writes. [Workspace isolation](https://docs.docker.com/ai/sandboxes/security/#trust-boundaries).

### OpenShell

Pi's guide selects an active gateway with `openshell gateway add <gateway-url> --name <name>` and `openshell gateway select <name>`, then launches `openshell sandbox create --name pi-sandbox --from pi -- pi`. Remote gateways do not automatically mount the local project: use `sandbox upload`/`download` or clone inside. Inference routing can keep raw provider keys outside, with Pi pointed at the gateway's compatible endpoint. [Pi OpenShell recipe](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/containerization.md#run-pi-with-openshell).

OpenShell v0.1.7 documents Linux and Apple-silicon macOS support, experimental WSL2 support, and separate native Windows work on a `windows` branch. Do not equate that branch with a validated native Windows Pi recipe. Linux workloads require Landlock ABI 3+ and specific seccomp user-notification capabilities; Docker-backed setups require Docker 28+, with Podman, Kubernetes, and microVM alternatives. The selected gateway owns policies. Denied network requests can yield proposals reviewed using `openshell rule get`, `rule approve`, or `rule reject`; approval updates policy so the sandboxed operation can retry, rather than rerunning on the host. No separate Pi account is required; backend provisioning and model credentials remain necessary. [Support matrix](https://docs.nvidia.com/openshell/latest/about/support-matrix), [policy denial workflow](https://docs.nvidia.com/openshell/latest/about/run-your-first-agent).

### Gondolin tool-routing extension

The official example requires Node.js **23.6+**, QEMU, and installation of the example's dependencies. Copy `packages/coding-agent/examples/extensions/gondolin` from a source checkout into `~/.pi/agent/extensions/gondolin`, run `npm install --ignore-scripts` there, and launch `pi -e ~/.pi/agent/extensions/gondolin` from the desired workspace. Gondolin documents macOS and Linux; native Windows and WSL2 operation of this specific example remain unverified. First use can download roughly 200 MB or more of guest assets. [Example source and requirements](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/gondolin/index.ts), [Gondolin prerequisites](https://github.com/earendil-works/gondolin#readme).

The extension mounts the host working folder through `RealFSProvider` at `/workspace`, replaces seven tools, and routes user shell commands into a Linux microVM. It forwards the shell environment, including any API keys, so file-based host credential separation is not environment-secret protection. Pi's model connection and unrelated extensions/MCP stay outside. VM startup/tool errors propagate through `ensureVm`; the replacement executors do not retry the operation using their original local implementations. Extension-load failure still requires operator attention: source inspection of the tool executor is not proof that every startup error terminates the CLI. [Gondolin routing implementation](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/examples/extensions/gondolin/index.ts).

The example supplies no custom domain allowlist or secret-injection hooks. Gondolin mediates HTTP/TLS rather than providing raw host NAT, blocks internal IP ranges by default, and has programmable filesystem/network policy. These backend capabilities should not be confused with a configured Pi destination allowlist. Workspace symlink behavior and backend-version-specific defaults need separate validation before claiming full confidentiality. [Gondolin security design](https://earendil-works.github.io/gondolin/security/), [VFS providers](https://earendil-works.github.io/gondolin/vfs/).

## Legacy 1.4.0 native sandbox example and failure behavior

The versioned example used `@anthropic-ai/sandbox-runtime`, documented macOS `sandbox-exec` and Linux bubblewrap, and required bubblewrap, socat, and ripgrep on Linux. It replaced Bash and user shell execution only; file tools, Pi, extensions, and MCP remained outside. It was never enabled by installing Pi alone. [1.4.0 example](https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/examples/extensions/sandbox/index.ts).

Its precedence was built-in defaults, `<agent-dir>/extensions/sandbox.json`, then `<cwd>/.pi/sandbox.json`; nested network/filesystem objects merge but individual arrays replace. Defaults allowed writes to `.` and `/tmp`, denied reads of `~/.ssh`, `~/.aws`, `~/.gnupg`, denied writes matching `.env`, `.env.*`, `*.pem`, `*.key`, and allowed selected npm/PyPI/GitHub domains. This did not mean all credential files were unreadable. `enabled: false` or extension-defined `--no-sandbox` disabled it. Unsupported platforms and initialization errors emitted notifications and left the replacement Bash using the original **unsandboxed** executor, with no approval requirement. That fallback is present in source regardless of session UI availability. This is a verified fail-open path, not an inference about all sandbox backends. [Versioned configuration and initialization code](https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/examples/extensions/sandbox/index.ts).

In current direct Pi, OS denial returns file/shell errors; there is no built-in escalation broker or automatic sandbox-to-host retry because there is no default sandbox to escape. With external isolation, backend availability, denials, and policy changes belong to that backend. Pi approval flags cannot remove a container/VM boundary. Conversely, loading a host-capable extension or an explicit host MCP integration creates access outside a tool-only boundary without requiring a sandbox escalation prompt. [Tool error paths](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/src/core/tools/write.ts), [extension execution model](https://github.com/earendil-works/pi/blob/707888ece685778881c40d232880c5a0b93a12f1/packages/coding-agent/docs/extensions.md).

## Unresolved checks

No containment probes were run. To establish runtime guarantees, pin the Pi and backend versions, use disposable marker files inside/outside the workspace (including symlink targets), test file tools separately from Bash and MCP, verify environment-secret visibility without real secrets, and check egress separately from model transport. Remove a required backend executable to test startup failure in print and RPC modes. These checks would settle Gondolin symlink confinement, extension-load failure behavior, and platform-specific operation; absence of default Pi isolation is already established by documentation and source. Docker Sandboxes WSL2 support and Gondolin native Windows/WSL2 support need explicit upstream confirmation plus a supported-host launch. OpenShell's current Pi image alias and headless launch should be verified against a pinned gateway release; its separate native Windows branch needs its own Pi test.

The structured `mechanisms` below records OS/container/VM isolation, including optional integrations and the version-limited legacy example. QuickJS is described in prose and controls as a restricted scripting runtime, because revision 5's mechanism categories do not represent language-runtime isolation. Its presence does not change the eight unsandboxed clean-install profiles. External-backend failure behavior is left unknown where Pi documentation/source does not establish a complete launch guarantee.

## Changelog

- **2026-11-04:** Initial report. Established unsandboxed clean-install defaults, separated trust/tool exposure and QuickJS from OS isolation, documented current official integration paths, and identified the removed 1.4.0 example's unsandboxed initialization fallback.
