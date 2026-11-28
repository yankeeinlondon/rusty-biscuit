---
$schema: "./_schema.yaml"
schema_revision: 5
provider: "kilo"
created: "2026-11-04"
last_updated: "2026-11-04"
content_policy:
  - "ValidFor(14d)"
agent: "codex"
model: "gpt-6.1-sol"
versions_examined:
  - "7.3.45"
  - "7.8.18"
evidence:
  -
    id: "docs"
    method: "official_docs"
    location: "https://kilo.ai/docs/getting-started/settings/sandboxing"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "Documents opt-in sandbox, defaults, CLI toggle, integration exclusions and platform support."
    limitations: "Living docs contain contradictory escalation-network wording; not runtime evidence."
  -
    id: "cli-docs"
    method: "official_docs"
    location: "https://kilo.ai/docs/code-with-ai/platforms/cli"
    version: "unknown"
    observed_on: "2026-11-04"
    claim: "CLI 1.0+ install, provider onboarding, config locations, permissions and headless behavior."
    limitations: "Broad config and auto-mode descriptions need qualification from pinned source."
  -
    id: "release"
    method: "official_docs"
    location: "https://github.com/Kilo-Org/kilocode/releases/tag/v7.8.18"
    version: "7.8.18"
    observed_on: "2026-11-04"
    claim: "Latest published release observed is v7.8.18, published 2026-10-26."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "release-backend"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/kilo-sandbox/src/backend.ts"
    version: "7.8.18"
    observed_on: "2026-11-04"
    claim: "Released tag contains sandbox backend implementation."
    limitations: "File existence does not establish release parity with every current policy or integration."
  -
    id: "local-help"
    method: "local_inspection"
    location: "local: macOS kilo --version; kilo --help; kilo run --help"
    version: "7.3.45"
    observed_on: "2026-11-04"
    claim: "Observed 7.3.45; help exposes --pure, --auto and --dangerously-skip-permissions, without sandbox launch flag."
    limitations: "Version/help only; no session or enforcement test; existing user settings not used as defaults."
  -
    id: "config"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/config.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Disabled default, deny networking, empty extra paths/hosts; local config may only tighten."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "policy"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Writable/protected paths, selected environment filtering, session precedence, refresh, inheritance, fail-closed and explicit unrestricted execution."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "backend"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/backend.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Darwin selects Seatbelt, Linux Bubblewrap, win32 unavailable; selected tool launches wrapped; unavailable confinement fails."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "seatbelt"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/seatbelt.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Uses sandbox-exec with unrestricted reads and generated write/network rules."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "seatbelt-base"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/seatbelt-base.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Default-deny policy allows process launch, same-sandbox interactions and selected runtime services."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "seatbelt-network"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/seatbelt-network.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Distinct allow, deny and scoped loopback proxy network policies."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "bubblewrap"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/bubblewrap.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Namespaces, read-only root, writable binds, probe requirements, system/verified bundled binary selection and absolute override."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "filesystem"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/filesystem.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Mutations use confined workers and checked paths; reads normal, writable handles denied."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "paths"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/path.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Canonicalizes symlink ancestors and missing descendants before matching paths."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "destinations"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/destination.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Validates exact public DNS destination entries with optional ports."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "proxy"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/proxy.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Authenticated HTTP/HTTPS proxy validates DNS/public addresses and TLS SNI; not tenant or content isolation."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "network-tools"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/network.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Gates custom, host-process, opaque and MCP tool execution separately from trusted lifecycle."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "network-inventory"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/network-tools.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Semantic search/LSP opaque; notebook/background-process tools execute on host."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "dispatcher"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/session/tools.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Wraps tools/MCP with session policy and records manual escalation provenance."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "resources"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/session/prompt.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Typed MCP resource access denied under restricted network policy."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "agents"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/agent/agent.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Most tools allowed; external paths and sensitive env reads ask, with runtime/config/skill exceptions."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "kilo-agents"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/agent/index.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Shell asks except explicit command allowlist; Ask/Plan restrict dispatch."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "run"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/cli/cmd/run.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Auto/bypass flags change ordinary approvals; headless rejects manual-only escalation; ordinary unanswered requests fail runs."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "permissions"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/permission/index.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Explicit denials remain; forced escalation requires explicit reply rather than saved/automatic grants."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "headless"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/permission/headless.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Plain-run descendants reject permission requests instead of waiting indefinitely."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "loader"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/config/config.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Scoped configuration loading; inline config is local for sandbox policy; managed settings late; trusted refresh events."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "managed"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/config/managed.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "OS-specific administrator configuration directories and macOS managed-preference domain."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "toggle"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/plugins/sandbox.tsx"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "CLI session sandbox toggle."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "git"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/git.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Classifies individual Git command text as mutating; shell parser checks each parsed subcommand."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "shell"
    method: "source_code"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/tool/shell.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Parsed Git mutations trigger explicit sandbox_escalation; approved execution runs the entire original shell command unrestricted."
    limitations: "Source inspection only; no runtime containment test or parity guarantee for older releases."
  -
    id: "wsl-selection"
    method: "inference"
    location: "https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/backend.ts"
    version: "main@c631a1da498a732ef47b97345d3c7724b8382e34"
    observed_on: "2026-11-04"
    claim: "Linux Kilo under WSL2 selects Bubblewrap by process.platform; enforcement conditional on namespace prerequisites."
    limitations: "Supported by Linux backend selector, not a WSL runtime or interop confinement test."
summary: "Clean installs have no OS sandbox. Optional Seatbelt on macOS and Bubblewrap on Linux confine tool writes and egress, with reads/model transport and trusted integrations outside. Native Windows unsupported; WSL2 conditionally selects Linux backend."
support: "native"
default_profiles:
  -
    id: "macos-interactive"
    environment: "macos"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Interactive kilo asks for unmatched shell/external actions; most workspace tools allowed. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
  -
    id: "macos-non-interactive"
    environment: "macos"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Plain kilo run rejects permission asks; optional --auto changes approvals only. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
  -
    id: "linux-interactive"
    environment: "linux"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Interactive kilo asks for unmatched shell/external actions; most workspace tools allowed. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
  -
    id: "linux-non-interactive"
    environment: "linux"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Plain kilo run rejects permission asks; optional --auto changes approvals only. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
  -
    id: "windows-interactive"
    environment: "windows"
    launch_mode: "interactive"
    status: "unavailable"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Interactive kilo asks for unmatched shell/external actions; most workspace tools allowed. Native backend unavailable. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
  -
    id: "windows-non-interactive"
    environment: "windows"
    launch_mode: "non_interactive"
    status: "unavailable"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Plain kilo run rejects permission asks; optional --auto changes approvals only. Native backend unavailable. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
  -
    id: "wsl2-interactive"
    environment: "wsl2"
    launch_mode: "interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Interactive kilo asks for unmatched shell/external actions; most workspace tools allowed. WSL2 Linux dispatch conditional on namespaces/Bubblewrap. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
      - "wsl-selection"
  -
    id: "wsl2-non-interactive"
    environment: "wsl2"
    launch_mode: "non_interactive"
    status: "disabled"
    mechanism_ids: []
    notes: "Clean-install sandbox off; no mandatory first-run sandbox choice. Plain kilo run rejects permission asks; optional --auto changes approvals only. WSL2 Linux dispatch conditional on namespaces/Bubblewrap. Home/credentials OS-readable; no default sandbox network restriction."
    evidence_ids:
      - "config"
      - "agents"
      - "kilo-agents"
      - "run"
      - "backend"
      - "cli-docs"
      - "wsl-selection"
mechanisms:
  -
    id: "seatbelt"
    name: "Sandbox (macOS)"
    enforcement: "os_sandbox"
    environments:
      - "macos"
    backend: "/usr/bin/sandbox-exec generated Seatbelt profile"
    prerequisites:
      - "sandbox-exec available; current CLI build with sandbox support"
    activation: "Opt-in sandbox.enabled true or /sandbox; saved session then project preference outranks config default."
    filesystem: "Reads unrestricted, including home/credentials. Writes allowed to active workspace/worktree, Kilo runtime data/cache/state and dedicated temp, plus global extra paths. Protected .git, global config and sandbox policy/preferences remain read-only. Canonicalization checks physical symlink targets; writable handles refused by adapter."
    network: "Default deny for tool egress. Exact public DNS host/port exceptions use authenticated HTTP/HTTPS proxy with DNS/address and SNI checks; not SSH/arbitrary TCP/UDP or tenant/DLP isolation. Network allow retains write confinement. Model transport, trusted hooks/MCP lifecycle/prompts remain outside. No independent ingress policy established."
    processes: "Kilo main process remains on host. Shell/descendants inherit Seatbelt; selected IPC/runtime services allowed. Selected Kilo config/server/broker variables removed, not all application credentials."
    coverage: "Shell descendants and file mutation workers confined; reads unconfined. Subagents/forks inherit policy. Restricted network blocks custom/opaque/MCP tools and typed resources. Plugin hooks and MCP startup/connections/prompts retain host access. Notebook/background-process tools refused while sandboxed. Direct trusted integration I/O confined only via sandbox-aware services."
    escape_hatches: "Trusted /sandbox toggle changes enabled state. Explicit manual Git escalation runs entire command/descendants unrestricted; no automatic unsandboxed retry. Auto/YOLO approval bypass retains confinement and cannot approve escalation. Escalation-network docs conflict; unrestricted source path supports bypass pending runtime test."
    failure_behavior: "fail_closed"
    failure_notes: "Missing sandbox-exec or policy execution failure refuses affected tool. No unrestricted fallback; denied file operations error; headless escalation rejected even with --auto."
    evidence_ids:
      - "config"
      - "policy"
      - "backend"
      - "seatbelt"
      - "filesystem"
      - "paths"
      - "proxy"
      - "destinations"
      - "dispatcher"
      - "resources"
      - "network-tools"
      - "network-inventory"
      - "run"
      - "permissions"
      - "git"
      - "shell"
      - "seatbelt-base"
      - "seatbelt-network"
  -
    id: "bubblewrap"
    name: "Sandbox (Linux)"
    enforcement: "os_sandbox"
    environments:
      - "linux"
      - "wsl2"
    backend: "Bubblewrap user/PID/mount namespaces; conditional network namespace; proxy relay/seccomp helper"
    prerequisites:
      - "Usable non-setuid/non-setgid bwrap at system path, verified bundled location, or absolute KILO_BWRAP_PATH"
      - "Kernel and host policy allow required namespaces; startup probes pass"
      - "Additional writable paths exist and are accessible; proxy relay and architecture-specific seccomp helper available"
      - "WSL2 Linux dispatch conditional; mounted Windows paths and interop not runtime-verified"
    activation: "Opt-in sandbox.enabled true or /sandbox; saved session then project preference outranks config default."
    filesystem: "Reads unrestricted, including home/credentials. Writes allowed to active workspace/worktree, Kilo runtime data/cache/state and dedicated temp, plus global extra paths. Protected .git, global config and sandbox policy/preferences remain read-only. Canonicalization checks physical symlink targets; writable handles refused by adapter."
    network: "Default deny for tool egress. Exact public DNS host/port exceptions use authenticated HTTP/HTTPS proxy with DNS/address and SNI checks; not SSH/arbitrary TCP/UDP or tenant/DLP isolation. Network allow retains write confinement. Model transport, trusted hooks/MCP lifecycle/prompts remain outside. No independent ingress policy established."
    processes: "Kilo main process remains on host. Tool descendants enter namespaces; read-only root and writable binds, private dev/proc. Selected Kilo config/server/broker variables removed, not all inherited credentials."
    coverage: "Shell descendants and file mutation workers confined; reads unconfined. Subagents/forks inherit policy. Restricted network blocks custom/opaque/MCP tools and typed resources. Plugin hooks and MCP startup/connections/prompts retain host access. Notebook/background-process tools refused while sandboxed. Direct trusted integration I/O confined only via sandbox-aware services."
    escape_hatches: "Trusted /sandbox toggle changes enabled state. Explicit manual Git escalation runs entire command/descendants unrestricted; no automatic unsandboxed retry. Auto/YOLO approval bypass retains confinement and cannot approve escalation. Escalation-network docs conflict; unrestricted source path supports bypass pending runtime test."
    failure_behavior: "fail_closed"
    failure_notes: "Missing usable bwrap, namespace probe failure or missing proxy dependencies refuses affected tool; native Windows unavailable. No unrestricted fallback; denied file operations error; headless escalation rejected even with --auto."
    evidence_ids:
      - "config"
      - "policy"
      - "backend"
      - "bubblewrap"
      - "filesystem"
      - "paths"
      - "proxy"
      - "destinations"
      - "dispatcher"
      - "resources"
      - "network-tools"
      - "network-inventory"
      - "run"
      - "permissions"
      - "git"
      - "shell"
      - "wsl-selection"
controls:
  -
    id: "enabled"
    kind: "config_key"
    name: "sandbox.enabled"
    values:
      - "true"
      - "false"
    effect: "Opt-in write/network confinement; unavailable configured backend fails closed."
    example: "Global kilo.jsonc: {\"sandbox\":{\"enabled\":true}}"
    precedence: "Config default seeds new sessions; saved session then project preference wins. Local config may tighten only; no immutable administrator toggle lock established."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "config"
      - "policy"
      - "backend"
  -
    id: "network-mode"
    kind: "config_key"
    name: "sandbox.network"
    values:
      - "deny"
      - "allow"
    effect: "deny blocks tool egress except proxy exceptions; allow relaxes network while retaining write policy."
    example: "Global kilo.jsonc: {\"sandbox\":{\"enabled\":true,\"network\":\"deny\"}}"
    precedence: "Default deny; local denial clears hosts; trusted refresh affects next execution, not in-flight tools."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "config"
      - "policy"
  -
    id: "hosts"
    kind: "config_key"
    name: "sandbox.allowed_hosts"
    values:
      - "array of exact public DNS hosts with optional :port; default 443"
    effect: "Adds HTTP/HTTPS egress destinations, not tenant/repository restrictions."
    example: "Global kilo.jsonc: {\"sandbox\":{\"enabled\":true,\"allowed_hosts\":[\"github.com:443\",\"api.github.com:443\"]}}"
    precedence: "Only global scope adds hosts; project denial clears list; initial child lists intersect."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "config"
      - "policy"
      - "proxy"
      - "destinations"
  -
    id: "writable"
    kind: "config_key"
    name: "sandbox.writable_paths"
    values:
      - "array of existing accessible paths; leading ~ expanded"
    effect: "Expands write authority outside built-in roots; protected paths still denied."
    example: "Global kilo.jsonc: {\"sandbox\":{\"enabled\":true,\"writable_paths\":[\"~/shared-output\"]}}"
    precedence: "Only global scope adds paths; initial child extras intersect; trusted settings refresh next tool."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "config"
      - "policy"
      - "paths"
  -
    id: "session-toggle"
    kind: "session_command"
    name: "/sandbox"
    values: []
    effect: "Toggle session confinement; save project default for future sessions."
    example: "In kilo TUI: /sandbox"
    precedence: "Config default seeds new sessions; saved session then project preference wins. Local config may tighten only; no immutable administrator toggle lock established."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "toggle"
      - "policy"
  -
    id: "bwrap-path"
    kind: "environment_variable"
    name: "KILO_BWRAP_PATH"
    values:
      - "absolute path to usable non-setuid/non-setgid bwrap"
    effect: "Select Linux backend binary without enabling sandbox policy."
    example: "KILO_BWRAP_PATH=/usr/bin/bwrap kilo"
    precedence: "Replaces system/bundled candidates; must pass probes, no unrestricted fallback."
    mechanism_ids:
      - "bubblewrap"
    evidence_ids:
      - "bubblewrap"
  -
    id: "permission-rules"
    kind: "config_key"
    name: "permission"
    values:
      - "allow"
      - "ask"
      - "deny"
      - "per-tool wildcard-pattern rule maps"
    effect: "Dispatch guardrails separate from OS sandbox; read denial does not constrain allowed shell/grep reads."
    example: "kilo.jsonc: {\"permission\":{\"bash\":{\"*\":\"ask\",\"git status*\":\"allow\"}}}"
    precedence: "Agent/user/project rules combine; hard denials and manual-only escalation retained."
    mechanism_ids: []
    evidence_ids:
      - "agents"
      - "kilo-agents"
      - "permissions"
  -
    id: "auto"
    kind: "cli_flag"
    name: "--auto"
    values: []
    effect: "Auto-answer ordinary approval requests; does not remove sandbox policy; headless manual-only escalation rejected."
    example: "kilo run --auto \"Implement the change\""
    precedence: "Explicit denials retained. Current source hides compatibility YOLO/skip flags; local older help differs."
    mechanism_ids: []
    evidence_ids:
      - "run"
      - "permissions"
      - "local-help"
  -
    id: "yolo"
    kind: "cli_flag"
    name: "--yolo"
    values: []
    effect: "Auto-answer ordinary approval requests; does not remove sandbox policy; headless manual-only escalation rejected."
    example: "kilo run --yolo \"Implement the change\""
    precedence: "Explicit denials retained. Current source hides compatibility YOLO/skip flags; local older help differs."
    mechanism_ids: []
    evidence_ids:
      - "run"
      - "permissions"
      - "local-help"
  -
    id: "skip-permissions"
    kind: "cli_flag"
    name: "--dangerously-skip-permissions"
    values: []
    effect: "Auto-answer ordinary approval requests; does not remove sandbox policy; headless manual-only escalation rejected."
    example: "kilo run --dangerously-skip-permissions \"Implement the change\""
    precedence: "Explicit denials retained. Current source hides compatibility YOLO/skip flags; local older help differs."
    mechanism_ids: []
    evidence_ids:
      - "run"
      - "permissions"
      - "local-help"
  -
    id: "pure"
    kind: "cli_flag"
    name: "--pure"
    values: []
    effect: "Disable external plugins; no OS confinement or model-network isolation."
    example: "kilo --pure"
    precedence: "Observed CLI flag independent of sandbox policy."
    mechanism_ids: []
    evidence_ids:
      - "local-help"
  -
    id: "custom-config"
    kind: "environment_variable"
    name: "KILO_CONFIG"
    values:
      - "path to config file"
    effect: "Load explicit configuration; sandbox expansion depends on source location being global."
    example: "KILO_CONFIG=/absolute/user-config/kilo.jsonc kilo"
    precedence: "After global, before project/inline/managed; location determines sandbox scope."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "loader"
      - "config"
  -
    id: "config-dir"
    kind: "environment_variable"
    name: "KILO_CONFIG_DIR"
    values:
      - "configuration directory path"
    effect: "Load explicit configuration directory; sandbox scope remains separate from substitution trust."
    example: "KILO_CONFIG_DIR=/absolute/user-config kilo"
    precedence: "Directory configuration precedes inline and managed settings; global/local scope filter applies."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "loader"
      - "config"
  -
    id: "inline-config"
    kind: "environment_variable"
    name: "KILO_CONFIG_CONTENT"
    values:
      - "JSON text"
    effect: "Local-scoped sandbox config may tighten but cannot add destinations/paths or weaken global policy."
    example: "KILO_CONFIG_CONTENT='{\"sandbox\":{\"enabled\":true}}' kilo run \"Inspect the project\""
    precedence: "After project/directory config, before managed; local sandbox filter despite trusted variable expansion."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "loader"
      - "config"
  -
    id: "disable-project-config"
    kind: "environment_variable"
    name: "KILO_DISABLE_PROJECT_CONFIG"
    values:
      - "true"
    effect: "Suppress project config loading; does not create OS confinement."
    example: "KILO_DISABLE_PROJECT_CONFIG=true kilo"
    precedence: "Global/explicit/managed config and saved session policy still matter."
    mechanism_ids: []
    evidence_ids:
      - "loader"
  -
    id: "managed-config"
    kind: "managed_policy"
    name: "system managed config and ai.opencode.managed"
    values:
      - "/Library/Application Support/kilo"
      - "/etc/kilo"
      - "%ProgramData%/kilo"
      - "macOS ai.opencode.managed preferences"
    effect: "Distribute late administrator settings; cannot supply Windows backend or immutable session toggle mandate."
    example: "Administrator kilo.jsonc: {\"sandbox\":{\"enabled\":true,\"network\":\"deny\"}}"
    precedence: "System files after ordinary/inline/console settings; macOS managed preferences last; saved session enabled choice still applies."
    mechanism_ids:
      - "seatbelt"
      - "bubblewrap"
    evidence_ids:
      - "managed"
      - "loader"
      - "policy"
gaps:
  -
    location: "versions_examined"
    detail: "First release with all current policy behavior and parity with installed 7.3.45 unresolved."
    next_check: "Compare tagged policy, filesystem/proxy and run-handler history; inspect current release help separately."
  -
    location: "mechanisms/bubblewrap/environments"
    detail: "WSL2 selects Linux backend but namespace/proxy startup and Windows-mount/interop containment untested."
    next_check: "Run current Linux build in disposable WSL2 fixtures; probe namespaces, /mnt/c writes and Windows interop subprocesses."
  -
    location: "mechanisms"
    detail: "Source inspected without runtime containment tests; adversarial path/race behavior not established."
    next_check: "On disposable macOS/Linux fixtures probe shell/file outside writes, symlinks, read visibility and restricted/proxied egress."
  -
    location: "mechanisms/seatbelt/escape_hatches"
    detail: "Docs conflict on escalation network authority; unrestricted source function supports bypass."
    next_check: "Manually approve harmless mutating Git escalation in disposable repo and compare controlled egress with sandboxed execution."
  -
    location: "mechanisms/bubblewrap/escape_hatches"
    detail: "Same escalation-network contradiction applies to Linux."
    next_check: "Repeat approved Git escalation egress probe on Linux and inspect resulting backend launch."
changes: []
---

# Kilo Code CLI sandboxing

Kilo supplies an **optional OS-enforced sandbox, disabled on a clean installation**. Its normal guardrails are application-level tool permissions. Enabling the sandbox adds filesystem write confinement and, by default, tool network denial on macOS and Linux. It does not restrict filesystem reads, isolate the main agent process, or stop model/API transport. Native Windows has no backend; WSL2 selects the Linux backend but needs working Linux namespaces and Bubblewrap. [Official sandbox overview](https://kilo.ai/docs/getting-started/settings/sandboxing), [backend selector](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/backend.ts).

## Scope and evidence

Research date: **2026-11-04 UTC**. This report concerns the standalone `kilo` CLI, not the former CLI implementation, IDE-specific controls, Cloud Agents, or third-party container wrappers. Official CLI documentation applies to CLI 1.0 and later. The latest GitHub release observed was **v7.8.18**, published **2026-10-26**. Detailed source findings below are pinned to **main commit `c631a1da498a732ef47b97345d3c7724b8382e34`**, committed **2026-11-04T16:42:26Z**; they are not assertions that every detail shipped in that release. The sandbox backend file exists at the v7.8.18 tag, corroborating that this is not solely an unreleased feature. The first release containing the complete current policy remains unresolved. [CLI documentation](https://kilo.ai/docs/code-with-ai/platforms/cli), [release](https://github.com/Kilo-Org/kilocode/releases/tag/v7.8.18), [examined commit](https://github.com/Kilo-Org/kilocode/commit/c631a1da498a732ef47b97345d3c7724b8382e34), [release backend](https://github.com/Kilo-Org/kilocode/blob/v7.8.18/packages/kilo-sandbox/src/backend.ts).

Read-only local inspection on macOS found `kilo --version` **7.3.45**. `kilo --help` and `kilo run --help` expose `--pure`, `--auto`, and `--dangerously-skip-permissions`, but no sandbox launch flag. That older executable's help is not evidence of current sandbox enforcement. No model session, sandbox escape probe, or remote platform test was run. Existing user settings were not used to infer installation defaults. Source inspection establishes intended control flow, not successful runtime confinement.

## Default protection and first use

The eight clean-install profiles are below. “No” means the CLI adds no OS sandbox; ordinary account permissions and host security still apply. The permission baseline is shared across platforms rather than an OS-specific sandbox preset. [Sandbox defaults](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/config.ts), [agent defaults](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/agent/agent.ts), [headless run handling](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/cli/cmd/run.ts).

| Platform | Session | OS sandbox by default | Approval behavior | Optional local backend |
|---|---|---|---|---|
| macOS | Interactive | No | Ask for unmatched shell actions and external paths | Seatbelt |
| macOS | Non-interactive `kilo run` | No | Reject unanswered permission requests | Seatbelt |
| Linux | Interactive | No | Same interactive permission baseline | Bubblewrap |
| Linux | Non-interactive `kilo run` | No | Same headless rejection | Bubblewrap |
| Native Windows | Interactive | No | Same interactive permission baseline | Unsupported |
| Native Windows | Non-interactive `kilo run` | No | Same headless rejection | Unsupported |
| WSL2 | Interactive | No | Same interactive permission baseline | Linux backend, conditional |
| WSL2 | Non-interactive `kilo run` | No | Same headless rejection | Linux backend, conditional |

By default most tools, including workspace file edits, are allowed. `external_directory` asks except for trusted runtime/config/skill paths. The `read` permission asks for `*.env` and `*.env.*`, with `*.env.example` allowed. Shell permission defaults to `ask` with an explicit command allowlist; it includes not only inspection commands but also `touch`, `mkdir`, `cp`, `mv`, `tsc`, and archive utilities. This is not a read-only baseline. Permission patterns decide whether Kilo dispatches a tool, not what an approved process can do internally. [Base rules](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/agent/agent.ts), [Kilo shell rules](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/agent/index.ts).

First-time setup uses `/connect` for provider credentials; the documented flow does not require a sandbox choice. Enable it explicitly in config or with `/sandbox`. No separate sandbox subscription, Kilo account entitlement, Docker installation, or VM is required by the local backend code. Model access still needs suitable provider credentials, a Kilo connection, or a configured local model. Plan/Ask agents restrict tool dispatch, and project-config trust limits what repository configuration can inject; neither constitutes OS isolation. [CLI setup](https://kilo.ai/docs/code-with-ai/platforms/cli), [agent restrictions](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/agent/index.ts), [config trust and loading](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/config/config.ts).

## What enabling the sandbox contains

The backend wraps tool processes rather than moving the entire CLI into a container. File mutations use a confined worker through Kilo's sandbox-aware filesystem service. Reads execute normally. Trusted integration code doing direct host I/O is not automatically confined. [Process wrapping](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/backend.ts), [filesystem adapter](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/filesystem.ts).

```mermaid
flowchart TD
    A[Host Kilo process and model transport] --> P[Tool permissions and session policy]
    A --> H[Trusted plugin hooks and MCP lifecycle on host]
    P --> S[Seatbelt or Bubblewrap boundary]
    S --> C[Shell and descendants]
    S --> F[File mutation worker]
    S --> N[Optional destination proxy]
    P --> R[Filesystem reads without read isolation]
```

On **macOS**, Kilo invokes `/usr/bin/sandbox-exec` with a generated Seatbelt policy. It allows file reads, process execution/forking, selected runtime services, and writes only through the computed path rules. Direct sockets are denied in restricted network modes; destination exceptions open only the scoped loopback proxy route. The backend checks that `sandbox-exec` exists; execution failure remains an error. [Seatbelt generator](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/seatbelt.ts), [base profile](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/seatbelt-base.ts), [network profile](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/seatbelt-network.ts).

On **Linux**, Bubblewrap creates user and PID namespaces, a read-only bind of `/`, writable bind exceptions, and private `/dev` and `/proc` views. Denied or proxied networking adds a network namespace. Proxy mode also needs Kilo's relay and seccomp helper to bridge the proxy while preventing direct host Unix-socket access. Selection tries `/usr/bin/bwrap`, then a bundled binary checked against its build-time SHA-256 digest. `KILO_BWRAP_PATH` selects an absolute alternative instead of that candidate list. Setuid/setgid binaries are rejected. A bounded startup probe must successfully create the namespaces; merely installing Bubblewrap is insufficient where kernel or host policy forbids them. Additional writable paths must already exist and be accessible. [Bubblewrap implementation](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/bubblewrap.ts), [path filtering](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts).

**Native Windows** explicitly returns an unavailable backend. Enabling a policy prevents affected tools from running rather than creating AppContainer isolation. **WSL2** is a source-derived conditional conclusion: a Linux Kilo binary selects Bubblewrap because selection uses `process.platform`, without a separate WSL backend. WSL2's VM is not a Kilo-created sandbox; mounted Windows paths and interoperability need runtime checks before claiming stronger containment. [Backend selection](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/backend.ts).

## Files, credentials, and writable paths

The active workspace is writable; in ordinary repositories the policy may include the detected worktree root as well as the invocation directory. Linked-worktree handling selects the active worktree and avoids implicitly widening writes to its parent checkout or siblings. Additional global writable paths expand that authority. Kilo also allows writes in its XDG data, cache, and state trees and its dedicated temporary directory, normally `~/.local/share/kilo`, `~/.cache/kilo`, `~/.local/state/kilo`, and `$TMPDIR/kilo`. `TMPDIR`, `TMP`, and `TEMP` are set to that dedicated temporary path for confined launches. Arbitrary system temporary files are not generally writable. [Policy construction](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts).

Protected rules override writable roots: `.git` metadata, saved sandbox policies/preferences, and the global config root are read-only. A permission approval for an external directory does not add it to sandbox writable paths. Writable file handles are deliberately unavailable through the adapter, so an integration requesting a read-write handle can fail even inside an otherwise writable location. Paths are canonicalized, including resolving existing symlink ancestors and not-yet-created descendants; the filesystem adapter checks physical destinations and runs mutations through the worker. A symlink under the workspace does not intentionally grant writes to its external target. This is source evidence, not a claim that all race or hard-link attacks have been tested. [Filesystem checks](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/filesystem.ts), [canonicalization](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/path.ts), [protected paths](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts).

Home files and credentials remain readable wherever the OS account can read them. The process environment removes selected Kilo config, server-auth, and browser-broker variables, but does not generally strip all provider keys or application credentials. Denying Kilo's `read` tool for a secret does not prevent an allowed shell command or another reading tool from accessing it. Read-only mounts protect integrity, not confidentiality, and readable content may enter model context. [Environment policy](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts), [unrestricted Seatbelt reads](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/seatbelt.ts).

## Network and tool coverage

`sandbox.network: "deny"` blocks direct tool-process egress. With `allowed_hosts`, the internal mode becomes `proxy`: authenticated HTTP/HTTPS proxying allows exact public DNS hosts and ports, defaulting to 443. URLs, wildcard hosts, IP literals, private/reserved destinations, and implicit subdomains are rejected. HTTPS tunnels require matching TLS SNI; DNS is resolved and validated outside confinement. Redirect destinations need their own permission. SSH, arbitrary TCP/UDP, QUIC, SOCKS, and CIDR allowlists are not supported. A host exception grants an egress destination, not a repository, tenant, HTTP path, or data-loss-prevention boundary; credentials retain their authority at that destination. Unknown NAT64 translation prefixes remain a documented limitation. `network: "allow"` removes network confinement while retaining write confinement. [Destination parser](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/destination.ts), [proxy implementation](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/proxy.ts), [config resolution](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/config.ts).

| Surface | Boundary when enabled |
|---|---|
| Shell and child processes | OS write/network policy inherited by launched subprocesses |
| Built-in file tools | Reads unconfined; mutations use sandbox-aware service and worker |
| Built-in HTTP tools | Policy-aware destination checks/proxy; not model transport |
| Subagents and forks | Inherit session policy; stricter initial parent/child combination |
| Notebook execution/background-process tool | Host-process tools refused while sandbox active |
| Semantic search/LSP and other opaque network tools | Refused while network restricted |
| Custom/plugin tools | Refused under restricted networking unless admitted as built-ins; direct trusted integration I/O otherwise depends on using sandbox-aware services |
| Plugin hooks | Trusted host execution, including hooks before internal tool permission checks |
| MCP | Local/remote tool calls and typed resource reads refused under restricted networking; server startup, remote connection/reconnection, and prompt retrieval remain trusted host activity |

The table follows the [session tool dispatcher](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/session/tools.ts), [network tool gate](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/network.ts), [host/opaque inventory](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/network-tools.ts), and [typed-resource gate](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/session/prompt.ts). Allowing tool networking does not put independently running MCP servers inside the CLI's process sandbox.

Provider/model inference remains on the host transport. Network denial is therefore not an offline or provider-privacy mode. `--pure` disables external plugins in the observed CLI help; it does not create a process sandbox or disable model networking.

## Configuration, precedence, and examples

Use global `~/.config/kilo/kilo.jsonc` (respecting the configured XDG config root) for authority-expanding settings:

```jsonc
{
  "sandbox": {
    "enabled": true,
    "network": "deny",
    "allowed_hosts": ["github.com:443", "api.github.com:443"],
    "writable_paths": ["~/shared-output"]
  },
  "permission": {
    "bash": { "*": "ask", "git status*": "allow" },
    "read": { "*": "allow", "*.env": "deny", "*.env.*": "deny" }
  }
}
```

Create `~/shared-output` yourself first. Remove `allowed_hosts` for deny-all tool networking; set `network` to `"allow"` for unrestricted tool network access with write confinement. The read rules above only gate the read tool. Run `kilo` and use `/sandbox` to change a session's enabled state; there is no verified dedicated `--sandbox` launch flag. [Config contract and scope filter](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/config.ts), [CLI sandbox command](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/plugins/sandbox.tsx).

Global defaults are `enabled: false`, `network: "deny"`, `allowed_hosts: []`, and `writable_paths: []`. Project config may enable confinement or deny networking, but cannot disable a global enabled default, permit globally denied networking, add hosts, or add writable paths. A project network denial clears global host exceptions. This sandbox-specific scope filter matters even though general CLI documentation says project config overrides global config. Session choice then takes priority: a saved session snapshot wins; otherwise a saved project preference wins over the config default. Thus a global enabled default is not an immutable administrator lock against trusted session toggles. [Scope filter](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/config.ts), [session initialization and refresh](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts).

The loader supports `KILO_CONFIG`, `KILO_CONFIG_DIR`, and `KILO_CONFIG_CONTENT`, but their security scope is significant. A custom file's scope is inferred from its location; inline `KILO_CONFIG_CONTENT` is explicitly merged as **local** for sandbox settings even though it is trusted for variable expansion. Do not rely on inline JSON to add writable paths or egress destinations. `KILO_DISABLE_PROJECT_CONFIG` suppresses project config loading. `KILO_BWRAP_PATH=/absolute/path/to/bwrap kilo` selects a Linux backend binary; it does not enable sandboxing. [Loader](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/config/config.ts), [Bubblewrap override](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/bubblewrap.ts).

Administrator-managed config loads late, after ordinary user/project/inline and console-managed organization settings. System directories are `/Library/Application Support/kilo` on macOS, `/etc/kilo` on Linux/WSL, and `%ProgramData%/kilo` on Windows. macOS managed preferences in the `ai.opencode.managed` domain load last. These can distribute settings, but do not supply a Windows backend or establish an untoggleable sandbox mandate. Saving trusted sandbox settings refreshes network/write limits for existing sessions before their next execution; in-flight tools keep their launch policy. New forks retain confinement, and subagents initially combine enabled states with OR and intersect writable paths and destination lists, with deny-all stricter than proxy access. Later trusted settings refresh those limits. [Managed config](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/config/managed.ts), [loader order](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/config/config.ts), [inheritance and refresh](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts).

## Denials, escalation, and unattended operation

An unavailable backend, failed namespace setup, or unavailable proxy dependency fails the affected execution. There is no automatic unrestricted fallback. With sandboxing off, execution deliberately uses the unrestricted path. Denied writes generally surface as tool/OS errors; a permission grant does not retry them with broader filesystem authority. [Backend errors](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/kilo-sandbox/src/backend.ts), [policy execution](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts).

Mutating Git commands need `.git` writes and can trigger a special one-command escalation prompt. Its explicit manual approval runs the **entire command and descendants outside filesystem and network confinement**. Bash allow rules and automatic permission approval do not satisfy this request. This is a deliberate escape hatch, not a wider sandbox profile or a generic automatic retry after every denial. The shell parser checks each parsed subcommand for Git mutations; approving a match lifts confinement for the entire original shell command, including accompanying commands. The lower-level Git classifier rejects compound text itself, so its check must be read together with that parser. [Shell escalation path](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/tool/shell.ts), [Git classifier](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/git.ts), [manual approval provenance](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/session/tools.ts), [unrestricted escalation function](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/sandbox/policy.ts).

The official page contradicts itself: it describes unrestricted network access after escalation, then says escalation does not itself grant network access. The unrestricted source function supports the broader interpretation for an approved escalated execution. Ordinary, non-escalated `gh` or HTTPS Git still requires destination exceptions. A runtime escalated-network probe is needed to settle the contradictory wording definitively. [Conflicting documentation](https://kilo.ai/docs/getting-started/settings/sandboxing#leaving-the-sandbox).

Plain `kilo run "Summarize this project"` cannot answer permission prompts: it rejects them, including child-agent requests, and a run with an ordinary auto-rejected permission exits 1. `kilo run --auto "Implement the change"` answers ordinary requests automatically; current source describes it as approving permissions not explicitly denied. Hidden compatibility flags `--yolo` and `--dangerously-skip-permissions` also affect approval handling. None changes the sandbox profile or removes its enforcement. The non-interactive event handler explicitly rejects sandbox-escalation and skill-shell requests even in auto mode. `kilo serve` is an API server whose clients must answer requests; merely lacking a TTY is not the same policy as the `run` frontend. [Run flags and event handler](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/cli/cmd/run.ts), [forced approval checks](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/permission/index.ts), [headless descendants](https://github.com/Kilo-Org/kilocode/blob/c631a1da498a732ef47b97345d3c7724b8382e34/packages/opencode/src/kilocode/permission/headless.ts).

## Unresolved checks

- **Release boundary:** compare tagged policy, filesystem, proxy, and run-handler history to find the first release with each behavior. The locally installed 7.3.45 was inspected only for version/help and must not be assumed to enforce the current policy.
- **WSL2 enforcement:** run a current Linux build in WSL2, verify namespace/proxy startup, then probe writes through `/mnt/c` and Windows interop subprocesses. Linux dispatch alone does not prove these cases.
- **Escalation networking:** manually approve a harmless mutating-Git escalation in a disposable repository and test a controlled egress destination from that execution; compare with an unapproved sandboxed command.
- **Runtime boundary:** source was inspected, not executed adversarially. On current macOS/Linux builds, verify shell and built-in-file outside writes, symlink redirects, read visibility, and restricted proxy egress using disposable fixtures.

## Changelog

- **2026-11-04:** Created the report. Established clean-install sandbox-off defaults, current opt-in Seatbelt/Bubblewrap confinement, host integration exclusions, headless approval behavior, and the distinction between tool egress and model transport. Recorded the installed-version mismatch and unresolved release/WSL2/escalation checks.
