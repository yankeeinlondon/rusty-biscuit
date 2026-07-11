---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://openai.com/codex/
docs: https://developers.openai.com/codex/cli/
skills_docs: https://developers.openai.com/codex/skills

support: first_class

locations:
  - os: macos
    scope: system
    path: $CODEX_HOME/skills/.system/
    notes: "Bundled with Codex by OpenAI. Observed on this host at ~/.codex/skills/.system/ containing imagegen, openai-docs, plugin-creator, skill-creator, skill-installer (codex-cli 0.142.5). Not enumerated by official docs as a public path; documented only as `Bundled with Codex`."
  - os: linux
    scope: system
    path: $CODEX_HOME/skills/.system/
    notes: "Bundled with Codex by OpenAI. Path pattern shared with macOS."
  - os: windows
    scope: system
    path: "%CODEX_HOME%\\skills\\.system\\"
    notes: "Bundled with Codex by OpenAI. %CODEX_HOME% defaults to %USERPROFILE%\\.codex."
  - os: linux
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: "Modern preferred user-scope path per the official Skills page."
  - os: macos
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: "Modern preferred user-scope path per the official Skills page. Not observed on this host; installed skills live under ~/.codex/skills/."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<skill-name>\\SKILL.md"
    notes: "Modern preferred user-scope path per the official Skills page."
  - os: linux
    scope: user
    path: ~/.codex/skills/<skill-name>/SKILL.md
    notes: "Legacy / default-install user-scope path under CODEX_HOME. Codex's bundled skill-creator defaults new skills here when CODEX_HOME is unset. Still actively scanned."
  - os: macos
    scope: user
    path: ~/.codex/skills/<skill-name>/SKILL.md
    notes: "Legacy / default-install user-scope path under CODEX_HOME. Observed on this host as the primary installed location (~/.codex/skills/ contains 80+ skills plus the .system subtree)."
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.codex\\skills\\<skill-name>\\SKILL.md"
    notes: "Legacy / default-install user-scope path under CODEX_HOME."
  - os: macos
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: "Repo-scoped skills. Codex walks .agents/skills/ from the launch CWD up to the Git repository root. Shared with collaborators."
  - os: linux
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: "Repo-scoped skills. Codex walks .agents/skills/ from the launch CWD up to the Git repository root. Shared with collaborators."
  - os: windows
    scope: repo
    path: ".agents\\skills\\<skill-name>\\SKILL.md"
    notes: "Repo-scoped skills. Codex walks .agents/skills/ from the launch CWD up to the Git repository root."
  - os: macos
    scope: repo
    path: ../.agents/skills/<skill-name>/SKILL.md
    notes: "Parent-directory repo skills. Discovered when CWD is inside a Git repository."
  - os: linux
    scope: repo
    path: ../.agents/skills/<skill-name>/SKILL.md
    notes: "Parent-directory repo skills. Discovered when CWD is inside a Git repository."
  - os: windows
    scope: repo
    path: "..\\.agents\\skills\\<skill-name>\\SKILL.md"
    notes: "Parent-directory repo skills. Discovered when CWD is inside a Git repository."
  - os: linux
    scope: other
    path: /etc/codex/skills/<skill-name>/SKILL.md
    notes: "Admin / machine-wide location on Linux. Documented as `/etc/codex/skills`."
  - os: macos
    scope: extension
    path: <plugin>/skills/<skill-name>/SKILL.md
    notes: "Skills bundled inside Codex plugins. Loaded only when the plugin is enabled (see [plugins docs](https://developers.openai.com/codex/plugins/build)). Path is relative to the plugin root and travels with the plugin."
  - os: linux
    scope: extension
    path: <plugin>/skills/<skill-name>/SKILL.md
    notes: "Skills bundled inside Codex plugins. Loaded only when the plugin is enabled. Path is relative to the plugin root."
  - os: windows
    scope: extension
    path: "<plugin>\\skills\\<skill-name>\\SKILL.md"
    notes: "Skills bundled inside Codex plugins. Loaded only when the plugin is enabled. Path is relative to the plugin root."

format:
  file_names:
    - SKILL.md
  frontmatter: true
  required_fields:
    - name
    - description
  optional_fields:
    - license
    - compatibility
    - metadata
    - allowed-tools
  body_format: markdown
  notes: |
    Codex implements the Agent Skills open standard (https://agentskills.io/specification).
    SKILL.md is the required entry point. The `name` field must be 1-64 characters,
    lowercase letters, numbers, and hyphens only; no leading/trailing hyphen; no
    consecutive hyphens; and must match the parent directory name. The `description`
    field is 1-1024 characters and is the routing signal for implicit invocation —
    Codex shortens descriptions when the initial skill list exceeds its context budget
    (≈2% of the model context, or 8000 characters when unknown), so authors should
    front-load trigger words.

    A skill directory may also include:

    - `scripts/` — executable code (Python, Bash, etc.); language availability is
      host-dependent.
    - `references/` — documentation loaded on demand; keep references one level deep.
    - `assets/` — templates, images, data files used in outputs.
    - `agents/openai.yaml` — Codex-specific sidecar with `interface:`,
      `policy.allow_implicit_invocation`, and `dependencies.tools` MCP declarations.

    Codex supports symlinked skill *directories* (the symlink target is followed when
    scanning); individual files inside a skill folder are not separately symlinked.

discovery:
  mechanism: |
    Codex uses progressive disclosure to keep skill metadata cheap. At session start
    Codex scans every configured scope and reads only `name`, `description`, file path,
    and (when present) `agents/openai.yaml` for each skill. The full `SKILL.md` body
    is loaded only when Codex decides the skill applies. Files under `scripts/`,
    `references/`, and `assets/` are loaded only when the skill body references them.

    Codex follows symlinked skill directories (not individual files). New skills are
    detected automatically; if a change to an existing skill does not appear, the
    Codex session must be restarted.

    Skills are activated in two ways:

    1. Explicit invocation — `$skill-name` mention in a prompt, or `/skills` to list
       available skills.
    2. Implicit invocation — Codex may choose a skill when the task matches its
       `description`. Sidecar `policy.allow_implicit_invocation: false` blocks this
       while still allowing explicit invocation.
  precedence: |
    Discovery order from lowest to highest priority:

    1. System / bundled (`$CODEX_HOME/skills/.system/`) — OpenAI-shipped defaults.
    2. Admin (`/etc/codex/skills/`) — Linux machine-wide admin skills.
    3. User (`~/.agents/skills/` per official docs; `~/.codex/skills/` legacy /
       default-install location). Both are scanned.
    4. Repo (`.agents/skills/` from the launch CWD up to the Git repository root,
       plus `$CWD/../.agents/skills/` for a shared parent scope).
    5. Plugins (`<plugin>/skills/<skill-name>/SKILL.md`) — loaded only when the
       plugin is enabled.

    Conflicts at the same name are NOT merged: both skills remain available in the
    selector. A skill folder may also be a symlink whose target sits under another
    scope; Codex follows the link and reads SKILL.md from the target.
  enable_disable: |
    - Per-skill disable: `[[skills.config]] path = "/absolute/path/to/SKILL.md"
      enabled = false` in `~/.codex/config.toml`. Path matches the SKILL.md file
      itself, not the directory. Restart Codex after editing the file.
    - Implicit-only disable: `policy.allow_implicit_invocation: false` in the
      skill's `agents/openai.yaml` keeps explicit `$skill-name` invocation working
      but blocks the model from selecting it automatically.
    - Feature flag: `--enable <FEATURE>` / `--disable <FEATURE>` accept the
      `skills` feature; skills are enabled by default in current Codex releases.
    - Session-scoped trust: Codex runs in untrusted mode by default in unfamiliar
      directories; certain capabilities (e.g. hook execution) require workspace
      trust before they activate, but skill discovery itself does not gate on
      trust.
  notes: |
    Codex does not read Claude Code's `.claude/skills/` directories; cross-provider
    reuse requires symlinking the skill directory into one of Codex's scan paths.
    Plugin skills can layer on top of file-system skills and are managed through
    the plugin system, not the skills scanner.

portability:
  portable: true
  non_portable_assets:
    - "Files in `scripts/` — language interpreter and binary availability vary by host"
    - "`agents/openai.yaml` sidecar — Codex-specific UI/policy/dependencies metadata"
    - "`dependencies.tools[]` MCP entries (Codex-flavored) — `type`, `value`, optional `transport`, `url`"
    - "`policy.allow_implicit_invocation` — Codex-specific invocation policy"
    - "`allowed-tools` values referencing Codex-specific tool names"
    - "Per-skill disable entries in `~/.codex/config.toml` (`[[skills.config]]` blocks)"
    - "Bundled system skills (`skill-creator`, `skill-installer`, `plan`, etc.) which have no user-space counterpart"
    - "Relative file references that assume a particular repository layout"
  rewrite_needed: true
  notes: |
    The Markdown body and the standard Agent Skills frontmatter (`name`,
    `description`, `license`, `compatibility`, `metadata`, `allowed-tools`) travel
    unchanged to any provider that implements the open standard. Codex-specific
    sidecars (`agents/openai.yaml`), Codex-flavored MCP `dependencies.tools`
    declarations, `[[skills.config]]` disable entries, and Codex-only tool names
    in `allowed-tools` need rewriting or removal before linking into another
    provider.

cli_params:
  - flag: "-c, --config <key=value>"
    description: "Override configuration values that would otherwise come from `~/.codex/config.toml`. Can target nested keys including `skills.config` entries used to disable skills."
    example: codex -c skills.config.path=/path/to/skill/SKILL.md -c skills.config.enabled=false
  - flag: "--enable <FEATURE>"
    description: "Enable a feature flag for the session. Accepts `skills` (enabled by default)."
    example: codex --enable skills
  - flag: "--disable <FEATURE>"
    description: "Disable a feature flag for the session. Accepts `skills` to turn off skill discovery entirely."
    example: codex --disable skills
  - flag: "--strict-config"
    description: "Error out when `~/.codex/config.toml` contains unrecognized fields. Affects validation of `[[skills.config]]` blocks."
    example: codex --strict-config
  - flag: "-C, --cd <DIR>"
    description: "Set the working directory before processing. Changes which repo-scoped `.agents/skills/` directories are discovered."
    example: codex --cd ./packages/api
  - flag: "--add-dir <DIR>"
    description: "Grant additional writable directories. May influence repo-relative file references inside skill bodies."
    example: codex --add-dir ../shared
  - flag: "-p, --profile <CONFIG_PROFILE_V2>"
    description: "Layer `$CODEX_HOME/<name>.config.toml` on top of the base user config. Profiles can ship their own `[[skills.config]]` entries."
    example: codex -p work
  - flag: "--dangerously-bypass-approvals-and-sandbox"
    description: "Skip all confirmation prompts and the sandbox. Indirectly affects scripts referenced by skills (scripts run with full host access)."
    example: codex --dangerously-bypass-approvals-and-sandbox

env_vars:
  - name: CODEX_HOME
    effect: "Root for Codex state including config, auth, logs, sessions, skills, and the bundled `~/.codex/skills/.system/` subtree. Default: `~/.codex`. Setting it changes every path that resolves from CODEX_HOME, including both legacy `~/.codex/skills/` and the official `$HOME/.agents/skills/` location is independent of CODEX_HOME on the official Skills page."
  - name: CODEX_API_KEY
    effect: "Provides an API key for a single non-interactive `codex exec` run. Does not influence skill discovery."
  - name: CODEX_ACCESS_TOKEN
    effect: "ChatGPT / Codex access token for trusted automation. Pipe to `codex login --with-access-token` to persist. Does not influence skill discovery."
  - name: CODEX_CA_CERTIFICATE
    effect: "PEM CA bundle path for HTTPS / login / WebSocket. Used by skill installer scripts that fetch from GitHub."
  - name: SSL_CERT_FILE
    effect: "Fallback PEM CA bundle path when `CODEX_CA_CERTIFICATE` is unset."
  - name: RUST_LOG
    effect: "Controls Rust log filtering for the Codex CLI and app-server (e.g. `codex_core=debug`). Useful when debugging skill discovery."

changes:
  - "Split location records from `os: all` into per-OS records (macOS, Linux, Windows) to satisfy the schema's `os` enum."
  - "Recorded two user-scope paths: the official `~/.agents/skills/` and the legacy / default-install `~/.codex/skills/`. Confirmed both are scanned; the bundled `skill-creator` defaults new skills to `~/.codex/skills/` when CODEX_HOME is unset, and the on-disk install on this host uses `~/.codex/skills/` as the primary location."
  - "Added `system` scope records for `$CODEX_HOME/skills/.system/` based on local inspection (`~/.codex/skills/.system/` contains `imagegen`, `openai-docs`, `plugin-creator`, `skill-creator`, `skill-installer` on codex-cli 0.142.5)."
  - "Confirmed CLI flags from `codex --help` on v0.142.5: `-c, --config`; `--enable`; `--disable`; `--strict-config`; `-C, --cd`; `--add-dir`; `-p, --profile`; `--dangerously-bypass-approvals-and-sandbox`. The previously listed `--enable skills` / `--disable skills` form still works as the values passed to the generic `--enable`/`--disable` flags."
  - "Verified env-var list against the current Codex environment-variables page (no new skill-related vars)."
  - "Updated `portability.non_portable_assets` to call out the Codex-flavored `dependencies.tools` shape (`type`, `value`, `transport`, `url`) and `policy.allow_implicit_invocation`."
  - "Updated agent/model fields to current run metadata (`open_code`, `minimax/MiniMax-M3`)."

requires_claudine_update: true
reason: |
  Claudine's linking module should treat both `~/.agents/skills/` (the official
  modern path) and `~/.codex/skills/` (the legacy / default-install path under
  CODEX_HOME) as canonical Codex user-scope skill roots, plus
  `$CODEX_HOME/skills/.system/` for bundled system skills and `/etc/codex/skills/`
  for admin skills on Linux. Repo-scope discovery walks `.agents/skills/` from
  CWD up to the Git repository root and includes the parent directory. The
  linker should recognize `SKILL.md` as the entry point, treat
  `agents/openai.yaml` as a Codex-specific sidecar, and honor
  `[[skills.config]] path = "..." enabled = false` blocks in
  `~/.codex/config.toml` when deciding whether a Codex-linked skill is active.
  Portability classification should mark the standard Agent Skills frontmatter
  and Markdown body as portable; scripts, `agents/openai.yaml`,
  `dependencies.tools`, Codex-only tool names in `allowed-tools`, and
  `allow_implicit_invocation: false` as provider-specific and requiring rewrite.
---

# OpenAI Codex CLI Skills

## Overview

OpenAI Codex CLI ships **first-class, file-system-based Agent Skills** that follow the [Agent Skills open standard](https://agentskills.io/specification). A skill is a directory containing a required `SKILL.md` entry point with YAML frontmatter and Markdown instructions, plus optional `scripts/`, `references/`, and `assets/` subdirectories. Codex activates skills explicitly (with `$skill-name` or `/skills`) or implicitly when a task matches the skill's `description`. The CLI is open source ([github.com/openai/codex](https://github.com/openai/codex)) and persists state under `CODEX_HOME` (default `~/.codex`). Codex 0.142.5 is installed on this host with bundled skills at `~/.codex/skills/.system/`.

Codex does **not** read Claude Code's `.claude/skills/` directories. To reuse a Claude-authored skill, symlink its directory into one of Codex's scan paths.

## Locations

Skill resources are organized by scope. The official Codex Skills page enumerates `system`, `admin`, `user`, and `repo` scopes; plugins add a fifth, `extension`.

| Scope | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| System / bundled | `$CODEX_HOME/skills/.system/` | `$CODEX_HOME/skills/.system/` | `%CODEX_HOME%\skills\.system\` | Shipped by OpenAI. Local host contains `imagegen`, `openai-docs`, `plugin-creator`, `skill-creator`, `skill-installer`. The official Skills page lists this only as "Bundled with Codex". |
| Admin | n/a | `/etc/codex/skills/<skill-name>/SKILL.md` | n/a | Machine-wide admin location. Linux-only per current docs. |
| User (modern) | `~/.agents/skills/<skill-name>/SKILL.md` | `~/.agents/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.agents\skills\<skill-name>\SKILL.md` | Official Skills page. |
| User (legacy / default-install) | `~/.codex/skills/<skill-name>/SKILL.md` | `~/.codex/skills/<skill-name>/SKILL.md` | `%USERPROFILE%\.codex\skills\<skill-name>\SKILL.md` | `$CODEX_HOME/skills/<skill-name>/`. Codex's bundled `skill-creator` defaults here when CODEX_HOME is unset, and the on-disk install on this host lives here. Still actively scanned. |
| Repo (CWD) | `$CWD/.agents/skills/<skill-name>/SKILL.md` | `$CWD/.agents/skills/<skill-name>/SKILL.md` | `$CWD\.agents\skills\<skill-name>\SKILL.md` | Folder-specific, team-shareable. |
| Repo (parent) | `$CWD/../.agents/skills/<skill-name>/SKILL.md` | `$CWD/../.agents/skills/<skill-name>/SKILL.md` | `$CWD\..\.agents\skills\<skill-name>\SKILL.md` | Shared parent scope when CWD is inside a Git repository. |
| Repo (root) | `$REPO_ROOT/.agents/skills/<skill-name>/SKILL.md` | `$REPO_ROOT/.agents/skills/<skill-name>/SKILL.md` | `$REPO_ROOT\.agents\skills\<skill-name>\SKILL.md` | Top-of-repo skills available to any subfolder. |
| Extension / plugin | `<plugin>/skills/<skill-name>/SKILL.md` | `<plugin>/skills/<skill-name>/SKILL.md` | `<plugin>\skills\<skill-name>\SKILL.md` | Bundled inside a Codex plugin. Loads only when the plugin is enabled. |

Observed on this host: `/Users/ken/.codex/skills/.system/` contains the bundled `imagegen`, `openai-docs`, `plugin-creator`, `skill-creator`, and `skill-installer` skills; `/Users/ken/.codex/skills/` is heavily populated (≈80 entries, mostly symlinks to external `/Users/ken/.research/library/<topic>/skill` directories plus a few real directories). No `~/.agents/skills/` directory exists on this host — every user-installed skill lives under `~/.codex/skills/`.

Codex supports symlinked skill directories: when scanning, Codex follows the symlink and reads `SKILL.md` from the target. Individual files inside a skill folder are not separately symlinked.

## File Format

A skill is a directory whose `SKILL.md` is the required entry point:

```text
my-skill/
├── SKILL.md            # Required: metadata + instructions
├── scripts/            # Optional: executable code (Python/Bash/etc.)
├── references/         # Optional: documentation loaded on demand
├── assets/             # Optional: templates, images, data files
└── agents/
    └── openai.yaml     # Optional: Codex-specific UI/policy/dependencies
```

`SKILL.md` is YAML frontmatter between `---` markers followed by Markdown content.

### Standard frontmatter

Required fields follow the [Agent Skills spec](https://agentskills.io/specification):

| Field | Required | Constraints |
|---|---|---|
| `name` | Yes | 1-64 characters; lowercase letters, numbers, hyphens; no leading/trailing hyphen; no consecutive hyphens; must match the parent directory name. |
| `description` | Yes | 1-1024 characters; describes what the skill does and when to use it. The routing signal for implicit invocation. |

Optional fields:

| Field | Purpose |
|---|---|
| `license` | License name or reference to a bundled license file. |
| `compatibility` | Environment requirements (max 500 characters). |
| `metadata` | Arbitrary string-key/string-value map for additional metadata. Codex's bundled skills use `metadata.short-description`. |
| `allowed-tools` | Space-separated pre-approved tool names (experimental in the standard). |

A minimal `SKILL.md`:

```markdown
---
name: pdf-processing
description: Extract PDF text, fill forms, merge files. Use when handling PDFs.
license: Apache-2.0
metadata:
  author: example-org
  version: "1.0"
---

# PDF processing

Steps Codex should follow when handling PDFs …
```

### `agents/openai.yaml` sidecar

Codex-specific metadata for UI presentation, invocation policy, and MCP dependencies:

```yaml
interface:
  display_name: "User-facing name"
  short_description: "User-facing description"
  icon_small: "./assets/small-logo.svg"
  icon_large: "./assets/large-logo.png"
  brand_color: "#3B82F6"
  default_prompt: "Optional surrounding prompt"

policy:
  allow_implicit_invocation: false   # default true; false blocks model auto-selection while keeping $skill-name usable

dependencies:
  tools:
    - type: "mcp"
      value: "openaiDeveloperDocs"
      description: "OpenAI Docs MCP server"
      transport: "streamable_http"
      url: "https://developers.openai.com/mcp"
```

Observed on this host: `~/.codex/skills/.system/skill-creator/agents/openai.yaml` and `~/.codex/skills/.system/skill-installer/agents/openai.yaml` contain only `interface:` blocks with `display_name`, `short_description`, `icon_small`, and `icon_large`. No `policy:` or `dependencies:` block in the shipped sidecars.

## Discovery and Precedence

Codex uses progressive disclosure to keep skill metadata cheap:

```mermaid
flowchart LR
    A[Session start] --> B[Scan all scopes]
    B --> C[Load name + description + path + openai.yaml]
    C --> D{Initial skill list fits context budget?}
    D -- Yes --> E[Expose skills to model]
    D -- No --> F[Shorten descriptions; drop overflow with warning]
    E --> G{User or model picks a skill?}
    G -- Yes --> H[Load full SKILL.md body]
    G -- No --> I[Continue without skill]
    H --> J{Skill body references a script or asset?}
    J -- Yes --> K[Load scripts/references/assets on demand]
    J -- No --> L[Execute skill]
```

### Discovery order (lowest → highest priority)

1. **System** — `$CODEX_HOME/skills/.system/` (OpenAI-bundled).
2. **Admin** — `/etc/codex/skills/` (Linux only).
3. **User** — `~/.agents/skills/` (official) and `~/.codex/skills/` (legacy / default-install under `CODEX_HOME`).
4. **Repo** — `.agents/skills/` walked from the launch CWD up to the Git repository root, plus `$CWD/../.agents/skills/` for a shared parent scope.
5. **Extension** — `<plugin>/skills/<skill-name>/SKILL.md` for plugins that ship skills.

Same-name conflicts are **not merged**: both skills remain available in the selector. The context budget for the initial skill list is roughly 2% of the model context, capped at 8,000 characters when the context is unknown; descriptions are shortened first, then individual skills are dropped with a warning.

### Enable / disable

| Mechanism | Effect |
|---|---|
| `[[skills.config]] path = "/abs/path/SKILL.md" enabled = false` in `~/.codex/config.toml` | Disables that specific skill (path matches `SKILL.md`, not the directory). Restart Codex after editing. |
| `policy.allow_implicit_invocation: false` in `agents/openai.yaml` | Blocks the model from auto-selecting the skill; explicit `$skill-name` still works. |
| `--disable skills` / `--enable skills` (feature flag values) | Globally toggles skill discovery for the session. Skills are enabled by default. |
| Plugin enabled state | Skills bundled in a disabled plugin do not load, even if their directories exist. |
| `--strict-config` | Surfaces unknown config keys, useful when validating `[[skills.config]]` entries. |

## Portability

Skills are portable across tools that implement the Agent Skills standard.

**Portable as-is:**

- `SKILL.md` Markdown body.
- Standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`).

**Non-portable — must be rewritten or removed:**

- `scripts/` contents (language interpreter, executable bit, host availability).
- `agents/openai.yaml` sidecar (`interface`, `policy`, `dependencies` are Codex-specific).
- `dependencies.tools[]` MCP declarations in the Codex shape (`type`, `value`, optional `transport`, `url`).
- `policy.allow_implicit_invocation` — no equivalent outside Codex.
- `allowed-tools` values referencing Codex-specific tool names.
- `[[skills.config]]` disable entries in `~/.codex/config.toml` — provider-specific.
- Bundled system skills (`skill-creator`, `skill-installer`, `plan`, etc.) — Codex-shipped and not portable to other providers.
- File references that assume a particular repository layout.

## Claudine Linking Notes

For Claudine's cross-provider resource linking:

- Treat **both** `~/.agents/skills/` (official) and `~/.codex/skills/` (legacy / default-install) as canonical user-scope Codex skill roots, since Codex still installs into `~/.codex/skills/` by default and the bundled `skill-creator` defaults there when `CODEX_HOME` is unset.
- Recognize `$CODEX_HOME/skills/.system/` as the bundled system scope; link these only when a user/repo override exists.
- Index `/etc/codex/skills/` for Linux admin skills and the `.agents/skills/` walk-from-CWD pattern for repo skills.
- Treat `SKILL.md` as the canonical entry point and `agents/openai.yaml` as Codex-specific metadata. When linking into another provider, either drop the sidecar or convert the `dependencies.tools` and `policy.allow_implicit_invocation` fields to that provider's equivalent.
- Honor `[[skills.config]] path = "..." enabled = false` blocks when deciding whether a Codex-linked skill is active; treat them as provider-specific and do not propagate to other providers.
- Built-in system skills (`plan`, `skill-creator`, `skill-installer`, `plugin-creator`, `imagegen`, `openai-docs`) are Codex-shipped and should not be linked into other providers unless a user-space override exists.
- Plugin-bundled skills are namespaced inside their plugin; do not link plugin skills without also linking the surrounding plugin manifest.

## Changelog

- **2026-07-03** — Split location records from `os: all` into per-OS records (macOS, Linux, Windows) to satisfy the schema's `os` enum. Documented two user-scope paths (`~/.agents/skills/` per the official Skills page, `~/.codex/skills/` as the legacy / default-install location under `CODEX_HOME`) and confirmed both are scanned; the bundled `skill-creator` defaults new skills to `~/.codex/skills/` when `CODEX_HOME` is unset, and the on-disk install on this host uses `~/.codex/skills/` as the primary location. Added `system` scope records based on local inspection of `~/.codex/skills/.system/` (contains `imagegen`, `openai-docs`, `plugin-creator`, `skill-creator`, `skill-installer` on `codex-cli 0.142.5`). Refreshed `cli_params` against `codex --help` on v0.142.5 (`-c`, `--enable`, `--disable`, `--strict-config`, `-C`, `--add-dir`, `-p`, `--dangerously-bypass-approvals-and-sandbox`). Expanded `portability.non_portable_assets` to call out the Codex-flavored `dependencies.tools` shape and `policy.allow_implicit_invocation`. Updated `agent` / `model` frontmatter to current run metadata.

## Sources

- [Codex — Agent Skills](https://developers.openai.com/codex/skills)
- [Codex — CLI overview](https://developers.openai.com/codex/cli/)
- [Codex — CLI command reference](https://developers.openai.com/codex/cli/reference)
- [Codex — CLI slash commands](https://developers.openai.com/codex/cli/slash-commands)
- [Codex — Plugins](https://developers.openai.com/codex/plugins/build)
- [Codex — Environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex — Config reference](https://developers.openai.com/codex/config-reference)
- [Codex — Config basics](https://developers.openai.com/codex/config-basic)
- [Agent Skills specification](https://agentskills.io/specification)
- [Agent Skills reference validator](https://github.com/agentskills/agentskills/tree/main/skills-ref)
- [OpenAI Codex GitHub repository](https://github.com/openai/codex)
- [OpenAI curated skills catalog](https://github.com/openai/skills)
- [Local host evidence — `~/.codex/skills/.system/{skill-creator,skill-installer,plugin-creator,openai-docs,imagegen}` and `~/.codex/config.toml` (codex-cli 0.142.5)]