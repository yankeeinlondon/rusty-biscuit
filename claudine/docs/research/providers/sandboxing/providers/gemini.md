---
$schema: "./_schema.yaml"
schema_revision: 5
provider: "gemini"
agent: "codex"
model: "gpt-6.1-sol"
created: "2026-11-04"
last_updated: "2026-11-04"
content_policy:
  - "ValidFor(14d)"
changes: []
versions_examined:
  - "0.66.0"
summary: "Clean installations enable neither full-process nor tool-level OS isolation on macOS, Linux, native Windows, or WSL2. Approval policy, workspace path validation, and environment sanitization remain. Opt-in native tool managers and full-process external runtimes provide different, incomplete boundaries."
support: "native"
evidence:
  - id: "e1"
    method: "official_docs"
    location: "https://github.com/google-gemini/gemini-cli/releases/tag/v0.66.0"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Stable release on October 27; matches local 0.66.0."
    limitations: "Release metadata does not test isolation."
  - id: "e2"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/sandboxConfig.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Unset/false full-process sandbox is disabled; selectors and environment precedence are explicit."
    limitations: "Must also account for CLI startup environment rewriting."
  - id: "e3"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settingsSchema.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Tool sandboxing defaults false; sandbox and approval configuration keys are declared."
    limitations: "Declared keys alone do not prove runtime wiring."
  - id: "e4"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/config.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Positive sandbox flag rewrites GEMINI_SANDBOX; siblings augment partial config; tool sandboxing defaults false."
    limitations: "Inspection does not demonstrate successful backend startup."
  - id: "e5"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManager.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Disabled manager passes commands through with sanitized environment; path aliases and secret/governance constants exist."
    limitations: "Noop does not enforce filesystem or network confinement."
  - id: "e6"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/policy-engine.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Headless ask_user becomes deny; policy gates tool admission."
    limitations: "Policy is not OS isolation and does not prove reactive expansion termination."
  - id: "e7"
    method: "official_docs"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/cli/trusted-folders.md"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Folder trust is opt-in; headless untrusted workspaces fail unless session trust is supplied."
    limitations: "Trust does not create a sandbox."
  - id: "e8"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/gemini.tsx"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Host bootstrap precedes optional full-process relaunch; stdin is forwarded."
    limitations: "Does not confine earlier host bootstrap."
  - id: "e9"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManagerFactory.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Enabled per-tool isolation selects native OS manager; disabled mode is Noop."
    limitations: "Factory selection does not establish prerequisites are installed."
  - id: "e10"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox-macos-permissive-open.sb"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Opted-in outer Seatbelt profile allows broad reads and network, scoped writes, and selected credential/container denials."
    limitations: "Profile existence does not mean it is enabled by default."
  - id: "e11"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Outer runtime launcher defines profile lookup, container mounts, environment, network, LXC branch and fatal startup failures."
    limitations: "Native Windows full-process branch is unresolved; container auth untested."
  - id: "e12"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/config/config.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Workspace/temp validation and native filesystem-service selection coexist with host orchestrator access."
    limitations: "File-service selection does not cover every built-in read."
  - id: "e13"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/utils/fileUtils.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "processSingleFileContent ignores the injected file service and performs direct host reads."
    limitations: "No runtime exploit or universal read-surface audit performed."
  - id: "e14"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/hooks/hookRunner.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Command hooks spawn directly with sanitized environment."
    limitations: "Outer process confinement may still apply."
  - id: "e15"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/mcp-client.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Stdio MCP starts with SDK transport; remote transports execute outside local per-tool confinement."
    limitations: "Remote service security is not examined."
  - id: "e16"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/agents/local-executor.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Local subagent execution context inherits sandboxManager."
    limitations: "Remote agents are not confined by this inheritance."
  - id: "e17"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settings.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "System/user/workspace settings and system-path overrides determine loaded configuration."
    limitations: "User-controlled launch environment can alter selection."
  - id: "e18"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/sandboxPolicyManager.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "YOLO enables native network/write policy; user sandbox.toml stores mode and persistent command approvals."
    limitations: "User policy parse failure falls back to built-in rules; no runtime test."
  - id: "e19"
    method: "official_docs"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/admin/enterprise-controls.md"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Managed strict mode blocks YOLO and enterprise controls govern MCP/extensions."
    limitations: "Account controls are not OS sandbox enforcement."
  - id: "e20"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/89e9d25d67e7dc7cb0deb84ad2c3474b7a392d31/packages/cli/src/utils/sandbox.ts"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Main adds rootless Podman and IDE bridge changes after stable 0.66.0."
    limitations: "Not presented as stable release behavior."
  - id: "e21"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/linux/LinuxSandboxManager.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Linux tool isolation uses Bubblewrap and ptrace-denying seccomp."
    limitations: "Mount details are separately recorded; Linux/WSL runtime not tested."
  - id: "e22"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/macos/seatbeltArgsBuilder.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Generated tool Seatbelt grants workspace/runtime/temp access, protects selected paths, and conditionally permits network."
    limitations: "Not the legacy full-process profile and not runtime-tested."
  - id: "e23"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/windows/GeminiSandbox.cs"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Native helper uses restricted Low token and Job Object; network off attempts bandwidth throttling and can continue after warning."
    limitations: "Not a firewall or proven confidentiality boundary; native Windows not runtime-tested."
  - id: "e24"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/shell.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Shell denial/proactive paths request granular sandbox expansion."
    limitations: "Scheduler and headless reactive paths require further runtime checks."
  - id: "e25"
    method: "official_docs"
    location: "https://geminicli.com/docs/cli/sandbox/"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Officially documents optional Seatbelt, Docker/Podman, gVisor, LXC, Windows and tool sandboxing."
    limitations: "Unversioned guidance differs from source on precedence and coverage."
  - id: "local-help"
    method: "local_inspection"
    location: "gemini --version; gemini --help (read-only macOS observation)"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Local CLI returned 0.66.0 and exposed sandbox, approval, policy, trust and include-directory flags."
    limitations: "Existing settings were not inspected; no model session or sandbox launched."
  - id: "linux-mounts"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/linux/bwrapArgsBuilder.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Root is read-only, selected paths writable, includes read-only, private tmp/proc/dev and finite-depth secret masks."
    limitations: "Broad host reads remain; secret scanning errors are nonfatal."
  - id: "windows-manager"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/windows/WindowsSandboxManager.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Tool backend uses native helper, tries C# compilation if needed, and prepares ACL/path manifests."
    limitations: "Full-process windows-native selector is not established by this manager."
  - id: "file-service"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxedFileSystemService.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Canonical scope validation precedes manager-mediated file reads/writes."
    limitations: "Other built-in read paths bypass this service."
  - id: "scheduler"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/scheduler/scheduler.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Reactive expansion asks for confirmation and retries with permission arguments."
    limitations: "Headless termination of all reactive paths is untested."
  - id: "mount-safety"
    method: "source_code"
    location: "https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandboxUtils.ts"
    version: "0.66.0"
    observed_on: "2026-11-04"
    claim: "Sensitive-mount rejection resolves real paths; UID/GID override is supported."
    limitations: "Pattern-based sensitive detection is not a complete secret inventory."
default_profiles:
  - id: "macos-interactive"
    environment: "macos"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Normal gemini session uses default approval policy; first-run auth/theme do not activate isolation. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. "
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
  - id: "macos-non-interactive"
    environment: "macos"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "gemini -p or non-TTY does not activate isolation; policy asks become denials. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. "
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
  - id: "linux-interactive"
    environment: "linux"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Normal gemini session uses default approval policy; first-run auth/theme do not activate isolation. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. "
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
  - id: "linux-non-interactive"
    environment: "linux"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "gemini -p or non-TTY does not activate isolation; policy asks become denials. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. "
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
  - id: "windows-interactive"
    environment: "windows"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Normal gemini session uses default approval policy; first-run auth/theme do not activate isolation. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. "
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
  - id: "windows-non-interactive"
    environment: "windows"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "gemini -p or non-TTY does not activate isolation; policy asks become denials. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. "
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
  - id: "wsl2-interactive"
    environment: "wsl2"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Normal gemini session uses default approval policy; first-run auth/theme do not activate isolation. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. WSL2 uses Linux guest behavior, not native Windows."
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
  - id: "wsl2-non-interactive"
    environment: "wsl2"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "gemini -p or non-TTY does not activate isolation; policy asks become denials. Host shell permissions remain. Built-in files have workspace/temp checks. Folder trust is disabled by default; preconfigured credentials are required for model use. WSL2 uses Linux guest behavior, not native Windows."
    evidence_ids:
      - "e2"
      - "e3"
      - "e4"
      - "e5"
      - "e6"
      - "e7"
mechanisms:
  - id: "seatbelt-full"
    name: "macOS Seatbelt full-process sandbox"
    enforcement: "os_sandbox"
    environments:
      - "macos"
    backend: "Apple sandbox-exec and selected SBPL profile"
    prerequisites:
      - "sandbox-exec available"
    activation: "Opt-in --sandbox or GEMINI_SANDBOX=sandbox-exec; permissive-open is opted-in profile default."
    filesystem: "Broad host reads except selected credentials/.env and daemon paths; workspace, temp/cache and selected includes writable; home Gemini writes denied. Canonical mount/path handling does not imply all symlink races tested."
    network: "permissive-open permits outbound, inbound and binding; proxied profiles available; model transport shares outer network policy."
    processes: "CLI is relaunched; fork/exec descendants inherit Seatbelt; bootstrap remains host. Broad readable home data can be exposed."
    coverage: "Relaunched CLI, its built-in tools and ordinary child hooks/local MCP/subagents inherit outer boundary; host bootstrap and remote execution are outside it."
    escape_hatches: "Operator can choose custom profiles or disable outer mode; YOLO does not replace outer profile."
    failure_behavior: "fail_closed"
    failure_notes: "Missing requested backend/profile or process launch failure is not silently retried on host; headless uses same relaunch."
    evidence_ids:
      - "e2"
      - "e8"
      - "e10"
      - "e11"
      - "e25"
  - id: "docker"
    name: "Docker full-process sandbox"
    enforcement: "container"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "Docker Linux container engine; runtime supplies Linux environment on non-Linux hosts"
    prerequisites:
      - "Docker CLI and working engine"
      - "Pullable image with Gemini/runtime dependencies"
      - "Model credentials available inside container"
    activation: "Explicit GEMINI_SANDBOX=docker or tools.sandbox backend; true detects Docker before Podman after macOS Seatbelt."
    filesystem: "Workspace RW, temporary isolated config home, allowedPaths RO; explicit SANDBOX_MOUNTS can add RW. Host home not mounted wholesale; selected gcloud/ADC reads and env keys forwarded; realpath-sensitive-mount checks apply."
    network: "Ordinary container networking available; internal-network/proxy options can restrict egress; model/API transport inside container also affected."
    processes: "CLI and children run in container; mounted files and forwarded credentials remain exposed; host launcher is outside."
    coverage: "Relaunched CLI, its built-in tools and ordinary child hooks/local MCP/subagents inherit outer boundary; host bootstrap and remote execution are outside it."
    escape_hatches: "SANDBOX_FLAGS, mounts, env, images can weaken isolation; disable mode explicitly; no general automatic unsandboxed retry."
    failure_behavior: "fail_closed"
    failure_notes: "Missing engine/image or launch failure raises errors; container auth may fail despite host auth; native Windows mount/auth behavior not runtime-tested."
    evidence_ids:
      - "e2"
      - "e8"
      - "e11"
      - "e25"
      - "mount-safety"
  - id: "podman"
    name: "Podman full-process sandbox"
    enforcement: "container"
    environments:
      - "macos"
      - "linux"
      - "windows"
      - "wsl2"
    backend: "Podman Linux container engine; runtime supplies Linux environment on non-Linux hosts"
    prerequisites:
      - "Podman CLI and working engine"
      - "Pullable image with Gemini/runtime dependencies"
      - "Model credentials available inside container"
    activation: "Explicit GEMINI_SANDBOX=podman or tools.sandbox backend; true detects Docker before Podman after macOS Seatbelt."
    filesystem: "Workspace RW, temporary isolated config home, allowedPaths RO; explicit SANDBOX_MOUNTS can add RW. Host home not mounted wholesale; selected gcloud/ADC reads and env keys forwarded; realpath-sensitive-mount checks apply."
    network: "Ordinary container networking available; internal-network/proxy options can restrict egress; model/API transport inside container also affected."
    processes: "CLI and children run in container; mounted files and forwarded credentials remain exposed; host launcher is outside."
    coverage: "Relaunched CLI, its built-in tools and ordinary child hooks/local MCP/subagents inherit outer boundary; host bootstrap and remote execution are outside it."
    escape_hatches: "SANDBOX_FLAGS, mounts, env, images can weaken isolation; disable mode explicitly; no general automatic unsandboxed retry."
    failure_behavior: "fail_closed"
    failure_notes: "Missing engine/image or launch failure raises errors; container auth may fail despite host auth; native Windows mount/auth behavior not runtime-tested."
    evidence_ids:
      - "e2"
      - "e8"
      - "e11"
      - "e25"
      - "mount-safety"
  - id: "runsc"
    name: "gVisor / runsc"
    enforcement: "container"
    environments:
      - "linux"
      - "wsl2"
    backend: "Docker --runtime=runsc with gVisor user-space kernel"
    prerequisites:
      - "Linux-compatible runsc installation"
      - "Docker configured with runsc runtime"
      - "Usable image and credentials"
    activation: "Explicit GEMINI_SANDBOX=runsc; never auto-detected. WSL2 needs Linux runtime prerequisites, not merely WSL installation."
    filesystem: "Docker mount/config rules apply; user-space kernel does not hide deliberately mounted workspace or credentials."
    network: "Container network rules also constrain inference; not a per-tool-only network switch."
    processes: "Whole relaunched CLI and descendants execute through gVisor; bootstrap outside."
    coverage: "Relaunched CLI, its built-in tools and ordinary child hooks/local MCP/subagents inherit outer boundary; host bootstrap and remote execution are outside it."
    escape_hatches: "Runtime flags/mounts and explicit disabling remain operator controls; YOLO does not remove gVisor."
    failure_behavior: "fail_closed"
    failure_notes: "Non-Linux rejected; missing runsc or Docker fatal; runtime failure does not authorize host fallback. WSL2 runtime compatibility requires disposable test."
    evidence_ids:
      - "e2"
      - "e8"
      - "e11"
      - "e25"
  - id: "lxc"
    name: "LXC/LXD sandbox"
    enforcement: "container"
    environments:
      - "linux"
      - "wsl2"
    backend: "lxc exec in existing running LXD-managed Linux container"
    prerequisites:
      - "LXC/LXD CLI and service"
      - "Precreated running container with dependencies and credentials"
    activation: "Experimental opt-in GEMINI_SANDBOX=lxc; default container name gemini-sandbox; GEMINI_SANDBOX_IMAGE overrides name."
    filesystem: "Workspace bind-mounted at same absolute path; additional allowed paths RO; container root and configured shares define rest. Symlink and credential behavior need runtime matrix."
    network: "Existing container networking controls CLI/API transport; not documented as a default offline environment."
    processes: "Full Linux system container can run services; CLI and descendants inside; host launcher outside."
    coverage: "Relaunched CLI, its built-in tools and ordinary child hooks/local MCP/subagents inherit outer boundary; host bootstrap and remote execution are outside it."
    escape_hatches: "Operator changes container setup and mounts or disables selector; no general unsandboxed retry."
    failure_behavior: "fail_closed"
    failure_notes: "Missing CLI/container, unavailable exec or mount failure is surfaced; WSL2 LXD prerequisites must be verified in disposable environment."
    evidence_ids:
      - "e2"
      - "e11"
      - "e25"
  - id: "seatbelt-tools"
    name: "macOS tool sandboxing"
    enforcement: "os_sandbox"
    environments:
      - "macos"
    backend: "Generated Seatbelt policy through sandbox-exec"
    prerequisites:
      - "sandbox-exec available"
    activation: "Opt-in security.toolSandboxing=true; independent of full-process selector."
    filesystem: "Workspace and authorized scope; system/Node/PATH reads, shared tmp writes; included paths RW; canonical aliases considered; .env and forbidden paths blocked in wrapped commands, not all host read tools."
    network: "Default native mode denies tool network; authorized expansion or YOLO permits network. Host API/web/MCP transport unaffected."
    processes: "Selected subprocesses inherit SBPL; orchestrator and direct spawns remain host; sanitized command environment."
    coverage: "Tool-only mode wraps shell descendants and filesystem-service writes; some built-in reads/search, hooks, local MCP startup, and API transport retain host access. Local subagents inherit manager; remote services are not confined."
    escape_hatches: "Granular permissions, sandbox.toml, approval modes and included directories expand access; YOLO enables writes/network without removing manager."
    failure_behavior: "conditional"
    failure_notes: "Launch errors propagate; shell denial can request expansion and retry with approved permissions; headless reactive termination untested."
    evidence_ids:
      - "e3"
      - "e9"
      - "e12"
      - "e13"
      - "e14"
      - "e15"
      - "e16"
      - "e18"
      - "e22"
      - "e24"
      - "file-service"
  - id: "bubblewrap"
    name: "Linux tool sandboxing"
    enforcement: "os_sandbox"
    environments:
      - "linux"
      - "wsl2"
    backend: "Bubblewrap namespaces and ptrace-denying seccomp filter"
    prerequisites:
      - "bwrap and shell/filesystem utilities"
      - "Kernel permits required namespaces and seccomp"
    activation: "Opt-in security.toolSandboxing=true; selected for Linux including WSL2."
    filesystem: "Root read-only with broad home reads, workspace/authorized writes, includes RO; private tmp/proc/dev. Canonical aliases included; finite-depth .env masks and ignored paths not universal confidentiality."
    network: "Unshared network by default; authorized network adds share-net; host model/API and direct web/MCP unaffected."
    processes: "New session, PID/other namespaces, die-with-parent, minimal dev; ptrace denied; selected subprocess descendants inherit."
    coverage: "Tool-only mode wraps shell descendants and filesystem-service writes; some built-in reads/search, hooks, local MCP startup, and API transport retain host access. Local subagents inherit manager; remote services are not confined."
    escape_hatches: "Additional permissions grant paths/network; YOLO enables network/workspace writes; backend remains."
    failure_behavior: "conditional"
    failure_notes: "Missing bwrap or namespace/seccomp failure propagates; permission denials can request expansion. No general automatic unsandboxed fallback; headless reactive path untested."
    evidence_ids:
      - "e9"
      - "e12"
      - "e13"
      - "e14"
      - "e15"
      - "e16"
      - "e18"
      - "e21"
      - "linux-mounts"
      - "e24"
      - "file-service"
  - id: "windows-tools"
    name: "Windows Native tool sandboxing"
    enforcement: "os_sandbox"
    environments:
      - "windows"
    backend: "GeminiSandbox.exe, restricted Low token, filesystem ACL/integrity labels and Job Object"
    prerequisites:
      - "Shipped helper or C# compiler to build supplied helper"
      - "Filesystem permits required ACL/integrity operations"
    activation: "Opt-in security.toolSandboxing=true; full-process windows-native selector usability unresolved."
    filesystem: "Low integrity restricts writes with allowed-path labels; broad readable user files are not generally hidden. Forbidden manifests and aliases are processed; ACL/label changes persist and failures can warn."
    network: "Network off attempts MaxBandwidth=1 throttling, can warn and continue; no proven deny-all egress. Host inference/MCP transport unaffected."
    processes: "Helper launches restricted process tree in Job Object; no VM or AppContainer; orchestrator stays host."
    coverage: "Tool-only mode wraps shell descendants and filesystem-service writes; some built-in reads/search, hooks, local MCP startup, and API transport retain host access. Local subagents inherit manager; remote services are not confined."
    escape_hatches: "YOLO permits network/write expansion; operator can disable tool mode; labels may require explicit manual reset after session."
    failure_behavior: "conditional"
    failure_notes: "Token/launch failures return errors; ACL and network-control failures can warn and continue. Helper compilation/startup and native full-process selection require runtime checks."
    evidence_ids:
      - "e9"
      - "e12"
      - "e13"
      - "e14"
      - "e15"
      - "e16"
      - "e18"
      - "e23"
      - "windows-manager"
      - "file-service"
controls:
  - id: "sandbox-flag"
    kind: "cli_flag"
    name: "--sandbox"
    values:
      - "true"
      - "false"
    effect: "Opt in to full-process auto-selection; --no-sandbox disables only when environment does not override."
    example: "gemini --sandbox -p \"Analyze project\""
    precedence: "loadCliConfig makes positive --sandbox set GEMINI_SANDBOX=true; selector then prefers nonempty environment over argv/file. Negative flag does not clear environment. Loaded file settings merge system defaults < user < trusted workspace < system overrides."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
    evidence_ids:
      - "e2"
      - "e4"
      - "e8"
      - "local-help"
  - id: "sandbox-setting"
    kind: "config_key"
    name: "tools.sandbox"
    values:
      - "boolean"
      - "docker"
      - "podman"
      - "sandbox-exec"
      - "runsc"
      - "lxc"
      - "windows-native"
      - "object {enabled,command,allowedPaths,networkAccess,image}"
    effect: "Select full-process backend; windows-native accepted but native relaunch path unresolved. Object networkAccess defaults true."
    example: "{\"tools\":{\"sandbox\":{\"enabled\":true,\"command\":\"docker\",\"networkAccess\":true}}}"
    precedence: "loadCliConfig makes positive --sandbox set GEMINI_SANDBOX=true; selector then prefers nonempty environment over argv/file. Negative flag does not clear environment. Loaded file settings merge system defaults < user < trusted workspace < system overrides."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
    evidence_ids:
      - "e2"
      - "e3"
      - "e11"
  - id: "sandbox-env"
    kind: "environment_variable"
    name: "GEMINI_SANDBOX"
    values:
      - "true"
      - "1"
      - "false"
      - "0"
      - "docker"
      - "podman"
      - "sandbox-exec"
      - "runsc"
      - "lxc"
      - "windows-native"
    effect: "Enable, disable, or select full-process backend; Windows full-process value unresolved."
    example: "GEMINI_SANDBOX=docker gemini -p \"Analyze project\""
    precedence: "loadCliConfig makes positive --sandbox set GEMINI_SANDBOX=true; selector then prefers nonempty environment over argv/file. Negative flag does not clear environment. Loaded file settings merge system defaults < user < trusted workspace < system overrides."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
    evidence_ids:
      - "e2"
      - "e4"
  - id: "tool-mode"
    kind: "config_key"
    name: "security.toolSandboxing"
    values:
      - "true"
      - "false"
    effect: "Enable selected native per-tool operations; false default; restart required."
    example: "{\"security\":{\"toolSandboxing\":true}}"
    precedence: "Merged settings; independent of full-process enablement. Does not confine every host tool path."
    mechanism_ids:
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e3"
      - "e4"
      - "e9"
      - "e12"
  - id: "included-paths"
    kind: "cli_flag"
    name: "--include-directories"
    values:
      - "comma-separated paths or repeated flag"
    effect: "Expand workspace/application scope and backend-dependent included-path access."
    example: "gemini --include-directories /path/to/reference"
    precedence: "CLI supplied additional workspace scope; native Linux RO versus native macOS RW; not interchangeable with container mounts."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "local-help"
      - "e10"
      - "e12"
      - "e22"
      - "linux-mounts"
  - id: "seatbelt-profile"
    kind: "environment_variable"
    name: "SEATBELT_PROFILE"
    values:
      - "permissive-open"
      - "restrictive-open"
      - "strict-open"
      - "permissive-proxied"
      - "restrictive-proxied"
      - "strict-proxied"
      - "custom name"
    effect: "Choose opted-in outer SBPL file policy; not generated native tool profile."
    example: "GEMINI_SANDBOX=sandbox-exec SEATBELT_PROFILE=strict-open gemini"
    precedence: "Default permissive-open; custom name resolved user .gemini before project .gemini."
    mechanism_ids:
      - "seatbelt-full"
    evidence_ids:
      - "e10"
      - "e11"
      - "e25"
  - id: "image"
    kind: "environment_variable"
    name: "GEMINI_SANDBOX_IMAGE"
    values:
      - "image reference or LXC container name"
    effect: "Select container image; for LXC selects existing container name."
    example: "GEMINI_SANDBOX=docker GEMINI_SANDBOX_IMAGE=your-approved-image:tag gemini"
    precedence: "GEMINI_SANDBOX_IMAGE > GEMINI_SANDBOX_IMAGE_DEFAULT > object image > package image."
    mechanism_ids:
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
    evidence_ids:
      - "e2"
      - "e11"
  - id: "default-image"
    kind: "environment_variable"
    name: "GEMINI_SANDBOX_IMAGE_DEFAULT"
    values:
      - "image reference or LXC container name"
    effect: "Override image default when primary image environment is absent."
    example: "GEMINI_SANDBOX=docker GEMINI_SANDBOX_IMAGE_DEFAULT=your-approved-image:tag gemini"
    precedence: "GEMINI_SANDBOX_IMAGE > GEMINI_SANDBOX_IMAGE_DEFAULT > object image > package image."
    mechanism_ids:
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
    evidence_ids:
      - "e2"
      - "e11"
  - id: "mounts"
    kind: "environment_variable"
    name: "SANDBOX_MOUNTS"
    values:
      - "comma-separated absolute from:to:opts; default opts ro"
    effect: "Grant extra container filesystem access; RW relaxes confinement; sensitive host paths rejected."
    example: "GEMINI_SANDBOX=docker SANDBOX_MOUNTS=/path/to/data:/data:ro gemini"
    precedence: "Extra explicit mounts on top of workspace; realpath-sensitive-path checks."
    mechanism_ids:
      - "docker"
      - "podman"
      - "runsc"
    evidence_ids:
      - "e11"
      - "mount-safety"
      - "e25"
  - id: "runtime-flags"
    kind: "environment_variable"
    name: "SANDBOX_FLAGS"
    values:
      - "space-separated runtime arguments"
    effect: "Inject Docker/Podman runtime flags; privileged flags can weaken boundary."
    example: "GEMINI_SANDBOX=podman SANDBOX_FLAGS=\"--security-opt label=disable\" gemini"
    precedence: "Operator runtime arguments; not restricted by ordinary tool approval prompts."
    mechanism_ids:
      - "docker"
      - "podman"
      - "runsc"
    evidence_ids:
      - "e11"
      - "e25"
  - id: "forward-env"
    kind: "environment_variable"
    name: "SANDBOX_ENV"
    values:
      - "comma-separated key=value pairs"
    effect: "Forward extra environment to sandbox; can expose secrets."
    example: "GEMINI_SANDBOX=docker SANDBOX_ENV=BUILD_MODE=debug gemini"
    precedence: "Explicit additions to selected built-in forwarded environment."
    mechanism_ids:
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
    evidence_ids:
      - "e11"
  - id: "uid-gid"
    kind: "environment_variable"
    name: "SANDBOX_SET_UID_GID"
    values:
      - "true"
      - "false"
    effect: "Override distro-sensitive host UID/GID mapping."
    example: "GEMINI_SANDBOX=docker SANDBOX_SET_UID_GID=true gemini"
    precedence: "Explicit environment override beats distro detection."
    mechanism_ids:
      - "docker"
      - "podman"
      - "runsc"
    evidence_ids:
      - "mount-safety"
      - "e25"
  - id: "proxy"
    kind: "environment_variable"
    name: "GEMINI_SANDBOX_PROXY_COMMAND"
    values:
      - "proxy startup command"
    effect: "Launch optional proxy integration; requires matching profile/network setup and affects full-process transport."
    example: "GEMINI_SANDBOX=sandbox-exec SEATBELT_PROFILE=permissive-proxied GEMINI_SANDBOX_PROXY_COMMAND=\"your-proxy-command\" gemini"
    precedence: "Operator-supplied proxy plus selected profile; not native tool policy."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
    evidence_ids:
      - "e11"
      - "e25"
  - id: "already-inside"
    kind: "environment_variable"
    name: "SANDBOX"
    values:
      - "nonempty runtime marker"
    effect: "Suppress outer relaunch; manually setting it is a bypass marker, not proof of containment."
    example: "SANDBOX=sandbox:none gemini"
    precedence: "Checked before outer backend selection; separate tool enablement remains independent."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
    evidence_ids:
      - "e2"
      - "e8"
  - id: "sandbox-policy"
    kind: "config_key"
    name: "~/.gemini/policies/sandbox.toml"
    values:
      - "modes.plan/default/accepting_edits: network,readonly,approvedTools,allowOverrides"
      - "commands.<name>: allowed_paths,allow_network"
    effect: "Set native mode defaults and persistent permission expansions; persistent allowed_paths grants read/write."
    example: "[commands.curl]\nallow_network = true"
    precedence: "Valid custom file replaces built-in policy; malformed file falls back; YOLO mode is synthesized separately."
    mechanism_ids:
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e18"
  - id: "approval-mode"
    kind: "cli_flag"
    name: "--approval-mode"
    values:
      - "default"
      - "auto_edit"
      - "plan"
      - "yolo"
    effect: "Tool admission policy; native sandbox mode also changes write/network permission; not universal OS isolation."
    example: "gemini --approval-mode plan -p \"Review project\""
    precedence: "CLI overrides ordinary default mode; administrator disableYoloMode/strict controls constrain YOLO."
    mechanism_ids:
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "local-help"
      - "e6"
      - "e18"
      - "e19"
  - id: "yolo"
    kind: "cli_flag"
    name: "--yolo"
    values: []
    effect: "Relaxation: bypass ordinary approvals and permit native sandbox network/workspace writes; keep native manager and outer boundary."
    example: "gemini --yolo -p \"Run task\""
    precedence: "Alias -y or --approval-mode yolo; managed strict mode and disableYoloMode can block."
    mechanism_ids:
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "local-help"
      - "e18"
      - "e19"
  - id: "default-approval"
    kind: "config_key"
    name: "general.defaultApprovalMode"
    values:
      - "default"
      - "auto_edit"
      - "plan"
    effect: "Choose initial approval mode; not OS isolation when sandbox is disabled."
    example: "{\"general\":{\"defaultApprovalMode\":\"plan\"}}"
    precedence: "Merged settings; CLI approval mode overrides; YOLO available through CLI, not this setting."
    mechanism_ids:
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e3"
      - "e18"
  - id: "disable-yolo"
    kind: "config_key"
    name: "security.disableYoloMode"
    values:
      - "true"
      - "false"
    effect: "Disallow YOLO despite flag; does not itself enable sandbox."
    example: "{\"security\":{\"disableYoloMode\":true}}"
    precedence: "System settings can set; enterprise strict control is separately managed."
    mechanism_ids:
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e3"
      - "e19"
  - id: "folder-trust"
    kind: "config_key"
    name: "security.folderTrust.enabled"
    values:
      - "true"
      - "false"
    effect: "Opt-in project configuration trust checks; disabled default; not OS isolation."
    example: "{\"security\":{\"folderTrust\":{\"enabled\":true}}}"
    precedence: "Merged user/system settings control trust discovery; untrusted workspace settings not loaded."
    mechanism_ids: []
    evidence_ids:
      - "e7"
      - "e17"
  - id: "skip-trust"
    kind: "cli_flag"
    name: "--skip-trust"
    values: []
    effect: "Trust workspace for current session; no change to OS boundary."
    example: "gemini --skip-trust -p \"Analyze project\""
    precedence: "Session trust override; does not bypass sandbox enforcement."
    mechanism_ids: []
    evidence_ids:
      - "local-help"
      - "e7"
  - id: "trust-env"
    kind: "environment_variable"
    name: "GEMINI_CLI_TRUST_WORKSPACE"
    values:
      - "true"
    effect: "Trust workspace for current headless/session invocation; not isolation."
    example: "GEMINI_CLI_TRUST_WORKSPACE=true gemini -p \"Analyze project\""
    precedence: "Session trust override; distinct from remembered trust file."
    mechanism_ids: []
    evidence_ids:
      - "e7"
  - id: "policy-files"
    kind: "cli_flag"
    name: "--policy"
    values:
      - "policy file/directory paths; repeated or comma-separated"
    effect: "Load admission rules; approval decisions are not OS isolation."
    example: "gemini --policy /path/to/policies -p \"Analyze project\""
    precedence: "Policy engine evaluates applicable priorities; headless asks become denials."
    mechanism_ids: []
    evidence_ids:
      - "local-help"
      - "e6"
  - id: "admin-policy-files"
    kind: "cli_flag"
    name: "--admin-policy"
    values:
      - "policy file/directory paths; repeated or comma-separated"
    effect: "Load admission rules; approval decisions are not OS isolation."
    example: "gemini --admin-policy /path/to/policies -p \"Analyze project\""
    precedence: "Policy engine evaluates applicable priorities; headless asks become denials."
    mechanism_ids: []
    evidence_ids:
      - "local-help"
      - "e6"
  - id: "tool-allowlist"
    kind: "config_key"
    name: "tools.core"
    values:
      - "array of built-in tool identifiers/command patterns"
    effect: "Restrict tools available to model; not a process sandbox."
    example: "{\"tools\":{\"core\":[\"ReadFileTool\",\"GlobTool\",\"ShellTool(ls)\"]}}"
    precedence: "Merged settings; tools omitted from availability cannot be invoked through registry."
    mechanism_ids: []
    evidence_ids:
      - "e3"
      - "e4"
  - id: "legacy-allow"
    kind: "cli_flag"
    name: "--allowed-tools"
    values:
      - "tool names"
    effect: "Deprecated approval bypass for selected tools; does not remove OS checks."
    example: "gemini --allowed-tools read_file -p \"Analyze project\""
    precedence: "Policy Engine recommended instead; cannot establish isolation."
    mechanism_ids: []
    evidence_ids:
      - "local-help"
      - "e6"
  - id: "managed-strict"
    kind: "managed_policy"
    name: "Strict Mode"
    values:
      - "enabled"
      - "disabled"
    effect: "Enterprise cloud control blocks YOLO; MCP/extensions have separate managed switches; not OS isolation."
    example: "Management Console: Strict Mode = enabled"
    precedence: "Documented globally enforced for managed enterprise account; no local override."
    mechanism_ids:
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e19"
  - id: "system-settings"
    kind: "environment_variable"
    name: "GEMINI_CLI_SYSTEM_SETTINGS_PATH"
    values:
      - "absolute configuration file path"
    effect: "Replace system settings/defaults source; system file policy alone is not immutable against user-controlled launch. "
    example: "GEMINI_CLI_SYSTEM_SETTINGS_PATH=/path/to/corporate-settings.json gemini"
    precedence: "System defaults < user < trusted workspace < system overrides; sandbox flag/env selection can supersede sandbox file choice."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e17"
      - "e2"
      - "e4"
  - id: "system-defaults"
    kind: "environment_variable"
    name: "GEMINI_CLI_SYSTEM_DEFAULTS_PATH"
    values:
      - "absolute configuration file path"
    effect: "Replace system settings/defaults source; system file policy alone is not immutable against user-controlled launch. "
    example: "GEMINI_CLI_SYSTEM_DEFAULTS_PATH=/path/to/corporate-settings.json gemini"
    precedence: "System defaults < user < trusted workspace < system overrides; sandbox flag/env selection can supersede sandbox file choice."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e17"
      - "e2"
      - "e4"
  - id: "config-home"
    kind: "environment_variable"
    name: "GEMINI_CLI_HOME"
    values:
      - "configuration root path"
    effect: "Redirect Gemini user configuration root, not OS HOME identity or host filesystem access."
    example: "GEMINI_CLI_HOME=/path/to/job-config gemini"
    precedence: "Process configuration-location override, not an isolation boundary."
    mechanism_ids: []
    evidence_ids:
      - "e17"
  - id: "additional-paths"
    kind: "config_key"
    name: "tools.sandboxAllowedPaths"
    values:
      - "array of absolute paths"
    effect: "Augment loadCliConfig sandbox allowed paths; separate outer relaunch loader does not read this sibling."
    example: "{\"tools\":{\"sandboxAllowedPaths\":[\"/path/to/reference\"]}}"
    precedence: "Merged settings; appended/deduplicated into partial CLI sandbox config, not nested object replacement."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e3"
      - "e4"
      - "e8"
  - id: "network-sibling"
    kind: "config_key"
    name: "tools.sandboxNetworkAccess"
    values:
      - "true"
      - "false"
    effect: "OR network permission into loadCliConfig sandbox config; false cannot revoke true; outer loader uses nested networkAccess."
    example: "{\"tools\":{\"sandboxNetworkAccess\":false}}"
    precedence: "Default false; OR augmentation is scoped to loadCliConfig, not the outer loader."
    mechanism_ids:
      - "seatbelt-full"
      - "docker"
      - "podman"
      - "runsc"
      - "lxc"
      - "seatbelt-tools"
      - "bubblewrap"
      - "windows-tools"
    evidence_ids:
      - "e3"
      - "e4"
      - "e8"
  - id: "source-image-build"
    kind: "environment_variable"
    name: "BUILD_SANDBOX"
    values:
      - "nonempty value"
    effect: "Request custom sandbox image build through linked source checkout; unsupported for ordinary installed binary and Seatbelt."
    example: "GEMINI_SANDBOX=docker BUILD_SANDBOX=1 gemini"
    precedence: "Requires linked source setup; .gemini/sandbox.Dockerfile can supply build definition."
    mechanism_ids:
      - "docker"
      - "podman"
      - "runsc"
    evidence_ids:
      - "e11"
      - "e25"
gaps:
  - location: "evidence"
    detail: "Live sandbox documentation has no release version, and the compared main commit is not identified as an exact packaged release; those evidence versions remain unknown."
    next_check: "Consult versioned release documentation or match main commit to a published package; retain stable 0.66.0 as the behavioral baseline."
  - location: "controls/sandbox-setting"
    detail: "windows-native is accepted but outer launcher lacks explicit native relaunch branch."
    next_check: "Disposable native Windows run of selector versus tool-only configuration; inspect packaged helper and startup result."
  - location: "mechanisms/windows-tools"
    detail: "Network off throttles rather than denies; ACL enforcement and helper behavior are not runtime-proven."
    next_check: "Disposable Windows test with sentinel files, process descendants and bounded external/loopback network probes; inspect ACL changes and warnings."
  - location: "mechanisms"
    detail: "No runtime confinement matrix or nested container/native-tool test was run; Linux under WSL2 requires usable namespaces/runtimes."
    next_check: "Run disposable macOS/Linux/Windows/WSL2 matrix for tool, file, home, symlink, secret and network boundaries; test bwrap inside configured container."
  - location: "controls/sandbox-policy"
    detail: "Reactive sandbox expansion may enter scheduler confirmation path in headless; prompt termination untested."
    next_check: "Bounded non-TTY denial/expansion task; assert terminal denial/exit without host retry."
  - location: "mechanisms/docker"
    detail: "Host auth success does not prove sanitized container has functional credentials."
    next_check: "Use disposable API/OAuth/Vertex credentials in fresh container and verify model connection without broad host home mounts."
  - location: "mechanisms/podman"
    detail: "Stable lacks later rootless user namespace fix; native Windows and WSL2 runtime behavior not observed."
    next_check: "Run pinned 0.66.0 and the documented nightly in disposable rootless environments; compare ownership/startup behavior."
---
# Gemini CLI sandboxing

Gemini CLI **does not enable an OS sandbox in a clean installation** on macOS, Linux, native Windows, or WSL2. Interactive approval prompts and application-level file checks are the ordinary protection. Optional full-process sandboxing and a separate tool-level sandbox implementation exist, with different coverage and defaults. In particular, installing on macOS does not automatically activate Seatbelt: the backend selector returns before platform detection when sandboxing is unset or false. Both the settings schema and CLI configuration set `security.toolSandboxing` to false. These are source-derived findings, not confinement tests. [e2](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/sandboxConfig.ts), [e3](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settingsSchema.ts), [e4](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/config.ts)

## Scope and evidence

Researched on **2026-11-04 UTC**. The official website was the starting point. The release API identified **v0.66.0**, published October 27, 2026, as stable; its source archive resolves to commit `9704132`. Read-only local `gemini --version` returned `0.66.0`; `gemini --help` confirmed the flags used below. No agent task, authentication flow, sandbox launch, credential read, or denial probe was run. Local configuration was not used to infer defaults. [e1](https://github.com/google-gemini/gemini-cli/releases/tag/v0.66.0)

The source snapshot is the main implementation authority here. The backend selector and manager factory are unchanged at main commit `89e9d25d67e7dc7cb0deb84ad2c3474b7a392d31` (November 3 UTC). That main snapshot adds rootless Podman user namespace handling and an IDE connection bridge; these are **post-stable** changes, not demonstrated 0.66.0 behavior. The November 4 nightly release notes mention the Podman change. Live documentation is unversioned and occasionally describes a stronger or simpler boundary than the examined code. [e20](https://github.com/google-gemini/gemini-cli/blob/89e9d25d67e7dc7cb0deb84ad2c3474b7a392d31/packages/cli/src/utils/sandbox.ts), [release history](https://github.com/google-gemini/gemini-cli/releases)

Installation recommends Node.js 20+, macOS 15+, Windows 11 24H2+, or Ubuntu 20.04+. Sandboxing has no separate paid-account entitlement in its local selection code. Model use still requires authentication: Google login, an AI Studio `GEMINI_API_KEY`, or Vertex AI credentials. Organization login and Vertex AI can require a Cloud project. Headless automation needs credentials established beforehand; isolation does not provide an account or remove API quotas. [installation](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/get-started/installation.mdx), [authentication](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/get-started/authentication.mdx)

## Defaults by platform and session

“None” means no Gemini-enforced OS isolation, not absence of all tool checks. This table assumes no administrator policy, environment override, project configuration, or explicit flags.

| Platform | Interactive clean default | Non-interactive clean default | Optional isolation and prerequisites |
| --- | --- | --- | --- |
| macOS | None; default approval mode | None; requests requiring unavailable confirmation are denied | Full CLI under Seatbelt with `-s`; tool-level Seatbelt with `security.toolSandboxing`; containers require their runtime |
| Linux | None; default approval mode | None; same policy behavior | Tool-level Bubblewrap requires `bwrap`, usable namespaces, and seccomp; full CLI supports Docker/Podman, explicit Docker + runsc, or experimental LXC/LXD |
| Native Windows | None; default approval mode | None; same policy behavior | Tool-level native helper uses restricted tokens, Low integrity, ACL changes, and a Job Object; Linux containers require a configured Docker/Podman environment |
| WSL2 | None in the Linux guest | None in the Linux guest | Linux implementation and prerequisites; neither WSL2 nor Docker availability proves the native Windows backend works |

The defaults follow the disabled flags and `NoopSandboxManager`, which passes the requested program and arguments through after environment sanitization. Non-interactive policy converts `ask_user` to `deny`; this is a software decision before execution, not an OS containment boundary. Known-safe commands and read tools can be allowed without a prompt. [e3](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settingsSchema.ts), [e5](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManager.ts), [e6](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/policy-engine.ts)

First-run theme/authentication choices do not select an OS sandbox. Folder trust is also **disabled by default**. If enabled with `security.folderTrust.enabled`, the first visit offers trust for the folder, its parent, or neither. An untrusted project loses executable configuration and other capabilities; headless startup exits with `FatalUntrustedWorkspaceError` rather than displaying that dialog. `--skip-trust` or `GEMINI_CLI_TRUST_WORKSPACE=true` trusts the workspace for that session. Neither switch enables nor disables OS isolation. [e7](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/cli/trusted-folders.md)

## Two different process boundaries

Full-process mode relaunches Gemini itself, after host bootstrap and authentication preparation. A sandboxed CLI's ordinary child processes inherit its outer boundary. Tool-level mode leaves the orchestrator on the host and wraps selected operations instead. The factory chooses the native platform manager only when sandboxing is enabled; it does not choose a container backend for tool-level execution. [e8](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/gemini.tsx), [e9](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManagerFactory.ts)

```mermaid
flowchart LR
  H[Host bootstrap] --> F
  subgraph F[Optional full-process boundary]
    C[Gemini orchestrator] --> S[Shell and file tools]
    C --> M[Local MCP and hooks]
    C --> A[Local subagents]
  end
  O[Host orchestrator in tool-only mode] --> T
  O --> U[Host reads, MCP startup, hooks, API transport]
  subgraph T[Selected per-operation boundary]
    X[Shell subprocess and filesystem-service operations]
  end
```

This is a coverage model, not a claim that every bootstrap action is contained. A remote MCP server or remote subagent executes wherever that service is deployed; a local sandbox cannot confine that remote machine.

### Full-process Seatbelt

With sandboxing explicitly enabled, macOS selects Apple's `sandbox-exec` before looking for containers. The default **profile within this opted-in mode** is `permissive-open`. It starts with deny-default, permits broad host reads, process execution and forking, and allows outbound networking, local binds, and inbound connections. Writes include the workspace, selected temporary/cache locations, `~/.npm`, `~/.cache`, and up to five included directories. Explicit rules block several credential filenames, `.env` files, writes to `~/.gemini`, and known container-daemon escape surfaces. This is not workspace-only confidentiality. [e10](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox-macos-permissive-open.sb)

`SEATBELT_PROFILE` selects profiles such as `restrictive-open`, `strict-open`, and the `*-proxied` variants. The sandbox guide's profile names are not all equivalent, and a profile's network rules must be read separately from its file rules. Custom profiles resolve as `sandbox-macos-<name>.sb`, preferring `~/.gemini` over the project's `.gemini`. `GEMINI_SANDBOX_PROXY_COMMAND` configures an optional proxy integration. Neither these environment variables nor the permissive profile control the separate generated tool-level Seatbelt policy. [profile guide](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/cli/sandbox.md), [e11](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox.ts)

### Containers and experimental LXC

Docker and Podman need a working engine, not just a CLI binary. Linux containers on macOS or Windows depend on the runtime's Linux environment; Gemini does not install a VM. Explicit `runsc` requires Linux, Docker, and a configured gVisor runtime, and adds `--runtime=runsc`. LXC requires an already running LXD-managed container, defaults to `gemini-sandbox`, and is documented experimental. Its operating system and services are supplied by that container. None is auto-enabled by installation. [e2](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/sandboxConfig.ts), [sandbox guide](https://geminicli.com/docs/cli/sandbox/)

The Docker/Podman launcher mounts the workspace read/write at its corresponding absolute path, creates a temporary configuration home, and selectively forwards environment/authentication material. Extra `allowedPaths` mounts are read-only; `SANDBOX_MOUNTS` supports explicit read/write mounts. Container networking is normally available for the CLI's API transport; disabling it can also disable inference. An optional internal-network/proxy arrangement can mediate egress. Container confinement protects unmounted host files, not files deliberately mounted or credentials forwarded into the container. [e11](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox.ts)

The default release image is `us-docker.pkg.dev/gemini-code-dev/gemini-cli/sandbox:0.66.0` ([package metadata](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/package.json)). The launcher excludes the host Gemini credential directory from ordinary settings mounts and sanitizes copied settings. Nevertheless it can expose a read-only gcloud configuration directory, `GOOGLE_APPLICATION_CREDENTIALS`, and API-key environment variables. Read-only credentials are still readable credentials. Sensitive-path rejection follows canonical paths, including symlink targets; it is not a universal secret detector. [mount sanitization](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandboxUtils.ts)

### Native per-tool mechanisms

Enable tool-level mode independently with `security.toolSandboxing: true`. It does **not** require `tools.sandbox` to be set. The native manager receives the workspace, included paths, ignored/forbidden paths, and an approval-mode-specific sandbox policy. [e12](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/config/config.ts)

On macOS, the tool manager generates an SBPL policy and executes through `sandbox-exec`. Read access includes the workspace, system runtime locations, Node's installation, and PATH-derived directories. Shared system/user temporary directories are writable; the default is not a private temporary filesystem. It permits subprocess execution under the same policy. Network access is separately added when authorized. Broad PATH entries can therefore broaden readable files. [generated policy](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/macos/seatbeltArgsBuilder.ts), [base policy](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/macos/baseProfile.ts)

On Linux and WSL2, the tool manager uses **Bubblewrap**, not Docker. It creates namespaces with `--unshare-all`, a new session and private `/proc`, a minimal `/dev`, and a private writable `/tmp`; network permission adds `--share-net`. It initially binds `/` read-only, then overlays permitted writable paths and masks selected forbidden/secret paths. Thus most host files, including home files outside the masks, remain readable. The seccomp filter specifically denies `ptrace`; it is not a general syscall allowlist. `bwrap`, `sh`, and filesystem utilities must be available and the environment must permit the namespace operations. [Linux manager](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/linux/LinuxSandboxManager.ts), [mount builder](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/linux/bwrapArgsBuilder.ts)

Native Windows support is present in source and is distinct from WSL. The tool manager invokes `GeminiSandbox.exe`, compiling the shipped C# source through `csc.exe` if necessary. The helper creates a restricted token with privileges disabled, lowers integrity to Low, and assigns external processes to a Job Object. It adjusts filesystem ACLs/integrity labels for allowed paths, with persistent effects; the guide gives `icacls <path> /setintegritylevel Medium` as a manual reset. This is not Windows Sandbox, Hyper-V, or an AppContainer. Low integrity principally restricts writes; it does not hide every readable user file. [Windows manager](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/windows/WindowsSandboxManager.ts), [native helper](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/windows/GeminiSandbox.cs)

A significant Windows limitation: “network off” attempts Job Object rate control with `MaxBandwidth = 1`. If setting it fails, the helper warns and continues. It does not install a firewall rule or create a disconnected network namespace. Treat this as attempted throttling, **not verified network denial**. Native helper runtime behavior remains untested in this research. [native network implementation](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/windows/GeminiSandbox.cs#L292)

## Files, credentials, and tool coverage

Without an OS sandbox, the shell runs with the host user's effective permissions. It can read home files, launch programs, write outside the project, and contact network services when tool policy permits the command. Environment sanitization removes selected sensitive variables, but cannot stop a permitted shell from reading accessible credential files. Built-in file tools separately validate workspace membership; additional directories and project temporary paths expand that application-level scope. The read tool resolves real paths before validating access. These checks do not constrain arbitrary shell programs. [e5](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManager.ts), [e12](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/config/config.ts), [read tool](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/read-file.ts)

In tool mode, canonical workspace and permission paths are resolved together with symlink aliases. `SandboxedFileSystemService` rejects canonical paths outside the workspace or included paths, then performs text reads/writes through the manager. This prevents treating a workspace symlink as automatic permission for an outside target in that service. It is source evidence, not proof against all races or alternate read paths. [filesystem service](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxedFileSystemService.ts), [path resolution](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManager.ts)

The generated native policies distinguish `.git`, ignore files, `.env`/`.env.*`, and `.geminiignore` exclusions. `.git` is normally protected from writes but a writable Git command can receive explicit permission. Ignore files can be writable when the workspace is writable. Linux secret discovery searches only three levels and prunes build/dependency directories; failure to scan is logged rather than fatal. macOS uses path regex rules. These protections are neither uniform nor a complete credential inventory. Included paths also differ: Linux globally included directories are read-only; macOS's generated profile allows both reads and writes to them. [Linux mounts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/linux/bwrapArgsBuilder.ts), [macOS policy](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/macos/seatbeltArgsBuilder.ts)

| Execution surface | Default / tool-only behavior | With an outer full-process sandbox |
| --- | --- | --- |
| Shell tool and descendants | Host by default; platform manager when enabled | Inherits outer boundary, possibly with further per-tool restrictions |
| File writes and edits | Host validation by default; filesystem service can sandbox writes | Outer boundary also applies |
| Built-in reads/search/listing | Not uniformly sandboxed in tool mode; application path checks remain | Host reads are limited by outer boundary |
| Local subagents | Execution context passes the same sandbox manager; no separate private machine | Share the outer process environment |
| Command hooks | Spawn directly from the orchestrator; environment sanitization is not confinement | Child inherits outer boundary; bootstrap timing still matters |
| Extensions | No universal separate plugin sandbox; coverage depends on the hook/tool/MCP surface supplied | Code executed inside the relaunched CLI inherits its boundary; installation/bootstrap are separate |
| Local stdio MCP | Direct SDK transport startup, not the per-tool sandbox manager | Child process inherits outer boundary |
| Remote MCP / remote agents | Remote execution and host-side network transport | Outer boundary controls local connection only, not the remote service |

The read coverage exception is concrete: `processSingleFileContent` explicitly does **not use** its `FileSystemService` parameter and reads through host filesystem utilities. Search also has direct filesystem reads. Consequently enabling tool sandboxing does not prove that secret masks protect all model-visible reads. Hooks call `spawn` directly; MCP stdio uses `StdioClientTransport`; local subagents receive `sandboxManager` in their execution context. [e13](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/utils/fileUtils.ts), [search](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/grep.ts), [e14](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/hooks/hookRunner.ts), [e15](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/mcp-client.ts), [e16](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/agents/local-executor.ts)

Tool-level network restrictions apply to wrapped subprocesses, not the host orchestrator's Gemini API, built-in web-fetch/search, or remote MCP transport. With full-process isolation, the model connection is inside the boundary and needs an allowed route. A network-denied shell therefore does not imply that model transport or every tool is offline. [factory](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManagerFactory.ts), [web fetch](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/web-fetch.ts), [MCP transports](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/mcp-client.ts)

## Configuration, precedence, and approvals

These examples describe supported configuration surfaces; they were not executed as sandbox tests.

```sh
# Opt into full-process isolation; on macOS this selects Seatbelt.
gemini --sandbox -p "Analyze this project"

# Explicit container provider, including on macOS.
GEMINI_SANDBOX=docker gemini -p "Run the tests"

# Explicit gVisor integration on a configured Linux host.
GEMINI_SANDBOX=runsc gemini -p "Run the tests"

# Add workspace scope; this does not mean identical OS permissions on all backends.
gemini --include-directories /path/to/shared-library
```

For native per-tool isolation, put this in `~/.gemini/settings.json` and restart:

```json
{
  "security": { "toolSandboxing": true },
  "general": { "defaultApprovalMode": "default" }
}
```

For an explicit full-process container configuration:

```json
{
  "tools": {
    "sandbox": {
      "enabled": true,
      "command": "docker",
      "networkAccess": true,
      "allowedPaths": ["/path/to/reference"],
      "image": "your-approved-image:tag"
    }
  }
}
```

The object form's network default is true; tool-only native policies start with network false. `tools.sandboxAllowedPaths` and `tools.sandboxNetworkAccess` augment the configuration constructed by `loadCliConfig` (network uses logical OR). However, the outer relaunch separately calls `loadSandboxConfig`, which reads the nested object's `allowedPaths` and `networkAccess` instead of those siblings. They are not interchangeable across the two startup paths. [e2](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/sandboxConfig.ts), [e3](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settingsSchema.ts)

Settings merge from system defaults, user settings, trusted workspace settings, then system overrides. System paths are `/Library/Application Support/GeminiCli/settings.json` on macOS, `/etc/gemini-cli/settings.json` on Linux/WSL, and `C:\ProgramData\gemini-cli\settings.json` on Windows; `system-defaults.json` sits beside it. `GEMINI_CLI_SYSTEM_SETTINGS_PATH` and `GEMINI_CLI_SYSTEM_DEFAULTS_PATH` replace those locations. `GEMINI_CLI_HOME` redirects Gemini's user configuration root, not the OS user's identity or shell permissions. [e17](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settings.ts), [enterprise configuration](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/cli/enterprise.md)

**Backend-selection precedence has an important asymmetry:** `loadCliConfig` first rewrites `GEMINI_SANDBOX=true` when `--sandbox` is truthy. The selector then takes `argv.sandbox ?? settings.tools.sandbox` and replaces it with any nonempty `GEMINI_SANDBOX`. Thus `--sandbox` forces auto-selection even over an explicit backend environment value, but `--no-sandbox` does not clear an existing environment override. Without the positive flag, the environment wins over file settings and the negative flag. Environment `false`/`0` disables the full-process selector, `true`/`1` requests auto-selection, and explicit strings select a backend. `SANDBOX` suppresses relaunch as an internal already-sandboxed marker; setting it is not proof of actual confinement. Tool-level enablement is independent, so disabling full-process mode alone does not disable an explicitly enabled tool manager. The CLI rewrite is in [CLI config](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/config.ts). Image precedence is `GEMINI_SANDBOX_IMAGE`, `GEMINI_SANDBOX_IMAGE_DEFAULT`, object `image`, package default. [e2](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/sandboxConfig.ts)

Other container controls include `SANDBOX_MOUNTS` (comma-separated `from:to:opts`, default `ro`), `SANDBOX_FLAGS` (runtime arguments), `SANDBOX_ENV` (explicit environment pairs), and `SANDBOX_SET_UID_GID`. These are operator-granted capabilities: extra mounts or privileged runtime flags can materially weaken isolation. The guide describes custom `.gemini/sandbox.Dockerfile` images; the stable launcher's `BUILD_SANDBOX` path requires a linked source checkout, so building a custom image externally and selecting it explicitly is clearer for installed binaries. [sandbox guide](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/cli/sandbox.md), [launcher](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox.ts)

Approval policy is separate from confinement. `--approval-mode default`, `auto_edit`, `plan`, and `yolo`, plus `--yolo`/`-y`, control tool admission. `tools.core` limits available built-in tools; policy files supplied with `--policy` or `--admin-policy` can allow, ask, or deny. Deprecated `--allowed-tools` bypasses selected confirmations, not OS checks. Plan mode is not a universal read-only OS environment when sandboxing is off. With tool sandboxing on, its sandbox mode also requests a read-only workspace. [local help observation](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/config.ts), [policy engine](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/policy-engine.ts)

YOLO is more than prompt suppression **inside the native tool policies**: the sandbox policy manager sets network true, readonly false, and `yolo: true`. It keeps the native backend, but expands workspace writes and networking; it does not remove every forbidden-path rule or outer container restriction. Full-process Seatbelt's selected profile is not replaced by YOLO. A custom `~/.gemini/policies/sandbox.toml` supplies `modes.plan`, `modes.default`, `modes.accepting_edits`, and per-command `allowed_paths` / `allow_network`. Default native modes block tool networking; malformed user sandbox TOML logs an error and falls back to the built-in policy. [e18](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/sandboxPolicyManager.ts)

Administrators can set `security.disableYoloMode`, restrict tools/MCP/extensions, and deploy system settings. The enterprise guide calls system `tools.sandbox` enforcement, but the source's environment/argv overrides mean it is not an immutable OS security authority against a user controlling process launch. Cloud enterprise admin controls are separately documented as locally non-overridable; strict mode blocks YOLO and managed controls can disable extensions/MCP. Those account policies do not themselves create an OS sandbox. [enterprise guide](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/cli/enterprise.md), [e19](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/admin/enterprise-controls.md)

## Denials, expansion, and failure behavior

An invalid backend, missing explicitly selected executable, unsupported platform for runsc/windows-native, or `true` with no detected backend raises `FatalSandboxError`. Auto-selection tries macOS Seatbelt, Docker, then Podman; it does not fall back to ordinary host execution after an explicitly requested backend fails. Tool-level Linux/native launch errors similarly propagate from prepared subprocess execution; the no-op manager is the disabled path, not a documented denial-retry fallback. [e2](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/sandboxConfig.ts), [shell execution](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/shellExecutionService.ts)

Shell denial detection is heuristic. On a failed operation it can return `sandbox_expansion_required`, carrying proposed filesystem or network permissions. The scheduler requests confirmation and retries the tool with the expanded permission arguments; cancellation preserves a denial. Proactive suggestions can ask before network-heavy commands. Approval can apply once, for the session, or persist per command; persistent `allowed_paths` collapse requested read/write access into paths usable for both. This is a permission expansion, not a general automatic unsandboxed retry. [shell expansion](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/shell.ts), [scheduler](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/scheduler/scheduler.ts), [e18](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/sandboxPolicyManager.ts)

Headless sessions have no approval dialog and must arrange permitted operations beforehand. The policy engine denies decisions still requiring a user. Whether every reactive expansion path terminates promptly in the current headless scheduler was not runtime-tested; do not rely on interactive expansion in CI. Full-process sandbox options work with `-p`; piped stdin is forwarded during relaunch. No code examined establishes an automatic headless switch to YOLO or automatic removal of a failed boundary. [policy engine](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/policy-engine.ts), [relaunch](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/gemini.tsx)

## Remaining checks

1. **Native Windows full-process selector:** `windows-native` is accepted, but `start_sandbox` has explicit Seatbelt/LXC branches followed by container-style startup, without a corresponding native relaunch branch. Use the tool-level setting for the source-established native manager. A disposable native Windows launch should establish whether the full-process selector is usable or defective; acceptance of a config value alone is insufficient.
2. **Runtime boundary matrix:** on disposable macOS, Linux, Windows, and WSL2 environments, enable tool mode and test shell/file reads, outside-workspace writes, symlink targets, home credentials, secret masks, subprocess inheritance, and loopback/external networking. This research establishes implementation intent and specific limitations, not runtime security certification.
3. **Headless expansion:** run a bounded failing shell task requiring new permissions without a TTY and confirm denial/exit, including the reactive scheduler path.
4. **Layer composition:** test full-process containers containing native tool mode, especially whether their image includes `bwrap` and permits nested namespaces. Documentation's contrast between modes does not prove that one automatically disables the other.
5. **Container authentication:** verify each auth method using disposable credentials. Sanitized settings and withheld OAuth cache files mean host login success does not establish successful model authentication inside a fresh container.

These checks are unresolved behavior, not evidence that the respective mechanism is absent. Conversely, disabled clean defaults and host-side direct read/hook/MCP paths are supported negative findings in the examined source.

## Evidence records

All remote records were accessed 2026-11-04 UTC. `source` means code inspection, not execution; `documentation` means the official description. Evidence identifiers in frontmatter refer to this inventory.

| ID | Kind | Examined artifact and principal use |
| --- | --- | --- |
| e1 | release + local observation | [v0.66.0](https://github.com/google-gemini/gemini-cli/releases/tag/v0.66.0); local version/help, stable baseline |
| e2 | source | [sandboxConfig.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/sandboxConfig.ts); defaults, selection, precedence, fatal validation |
| e3 | source | [settingsSchema.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settingsSchema.ts); disabled tool mode and settings |
| e4 | source | [CLI config.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/config.ts); CLI flags and toolSandboxing default |
| e5 | source | [sandboxManager.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManager.ts); no-op behavior, path resolution |
| e6 | source | [policy-engine.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/policy-engine.ts); headless denies asks |
| e7 | documentation | [Trusted Folders](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/cli/trusted-folders.md); opt-in trust and headless errors |
| e8 | source | [gemini.tsx](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/gemini.tsx); outer relaunch |
| e9 | source | [sandboxManagerFactory.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/services/sandboxManagerFactory.ts); native factory and disabled manager |
| e10 | source | [permissive-open.sb](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox-macos-permissive-open.sb); outer Seatbelt rules |
| e11 | source | [sandbox.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/utils/sandbox.ts); containers, profiles, mounts, transport |
| e12 | source | [core config.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/config/config.ts); file service and workspace validation |
| e13 | source | [fileUtils.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/utils/fileUtils.ts); direct read exception |
| e14 | source | [hookRunner.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/hooks/hookRunner.ts); direct host spawn |
| e15 | source | [mcp-client.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/mcp-client.ts); stdio and remote transports |
| e16 | source | [local-executor.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/agents/local-executor.ts); inherited manager |
| e17 | source | [settings.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/cli/src/config/settings.ts); configuration layers |
| e18 | source | [sandboxPolicyManager.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/policy/sandboxPolicyManager.ts); modes, YOLO, persistent expansions |
| e19 | documentation | [Enterprise Admin Controls](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/docs/admin/enterprise-controls.md); managed-account guardrails |
| e20 | source | [main sandbox.ts](https://github.com/google-gemini/gemini-cli/blob/89e9d25d67e7dc7cb0deb84ad2c3474b7a392d31/packages/cli/src/utils/sandbox.ts); post-stable differences |
| e21 | source | [Linux manager](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/linux/LinuxSandboxManager.ts) and [mount builder](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/linux/bwrapArgsBuilder.ts); Bubblewrap boundaries |
| e22 | source | [macOS manager](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/macos/MacOsSandboxManager.ts) and [profile builder](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/macos/seatbeltArgsBuilder.ts); tool Seatbelt |
| e23 | source | [Windows manager](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/windows/WindowsSandboxManager.ts) and [C# helper](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/sandbox/windows/GeminiSandbox.cs); native support and network limitation |
| e24 | source | [shell.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/tools/shell.ts) and [scheduler.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/scheduler/scheduler.ts); expansions and retry |
| e25 | documentation | [sandbox guide](https://geminicli.com/docs/cli/sandbox/); supported optional integrations and configuration examples |

## Changelog

- **2026-11-04:** Initial report. Distinguished disabled defaults from opted-in profiles; documented native tool sandboxing, partial tool coverage, source/documentation precedence differences, and Windows network throttling limits.
