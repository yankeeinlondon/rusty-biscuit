---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-02
agent: open_code
model: kimi-for-coding/k2p7

homepage: https://openai.com/codex/
docs: https://developers.openai.com/codex/cli/
skills_docs: https://developers.openai.com/codex/skills/

support: first_class

locations:
  - os: all
    scope: system
    path: Bundled with Codex (e.g., ~/.codex/skills/.system/ on disk)
    notes: Built-in skills such as `plan` and `skill-creator`. Stored under `CODEX_HOME/skills/.system/`.
  - os: macos
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: Modern preferred user-scope path. Also legacy ~/.codex/skills/ is still supported.
  - os: linux
    scope: user
    path: ~/.agents/skills/<skill-name>/SKILL.md
    notes: Modern preferred user-scope path. Also legacy ~/.codex/skills/ is still supported.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\<skill-name>\\SKILL.md"
    notes: Modern preferred user-scope path. Also legacy %USERPROFILE%\.codex\skills\ is still supported.
  - os: all
    scope: repo
    path: .agents/skills/<skill-name>/SKILL.md
    notes: Discovered from CWD up to repo root, plus $CWD/../.agents/skills/ for shared parent scopes.
  - os: linux
    scope: system
    path: /etc/codex/skills/<skill-name>/SKILL.md
    notes: Machine-wide admin skills.

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
    Codex implements the Agent Skills open standard. `SKILL.md` is the required entry point.
    Supporting files live beside it in `scripts/`, `references/`, and `assets/`.
    Optional UI/policy metadata is supplied by `agents/openai.yaml` sidecar.
    The `name` must match the parent directory name and follow `a-z0-9-` constraints.

discovery:
  mechanism: |
    Codex scans configured skill directories at startup using progressive disclosure:
    only `name`, `description`, file path, and optional `agents/openai.yaml` metadata are loaded initially.
    The full `SKILL.md` body is loaded only when Codex decides to use a skill.
    New skills are detected automatically; restart Codex if changes to existing skills do not appear.
  precedence: |
    Built-in/system < admin (/etc/codex/skills) < user (~/.agents/skills or ~/.codex/skills) < repo (.agents/skills).
    Within a repo, Codex walks from CWD to root and also checks the parent directory.
    Conflicting names are not merged; both skills remain selectable.
  enable_disable: |
    Per-skill: `[[skills.config]] path = "/path/to/skill/SKILL.md" enabled = false` in ~/.codex/config.toml.
    Sidecar policy: `agents/openai.yaml` `policy.allow_implicit_invocation: false` blocks implicit invocation while keeping explicit `$skill-name` invocation.
    Feature flags: skills are enabled by default; historically required `--enable skills` behind a feature flag.
  notes: |
    Codex does not read Claude Code `.claude/skills/` directories. Symlinked skill directories are supported; individual files are not.

portability:
  portable: true
  non_portable_assets:
    - "Scripts in `scripts/` — language, interpreter, and executable availability vary by host"
    - "`agents/openai.yaml` sidecar — Codex-specific UI/policy metadata"
    - "MCP dependencies declared in `dependencies.tools` — provider-specific server availability"
    - "`allowed-tools` values referencing Codex-specific tool names"
    - "Relative file references that assume a particular repository layout"
  rewrite_needed: true
  notes: |
    The Markdown body and Agent Skills standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`)
    are portable across tools that implement the open standard. Codex-specific sidecars (`agents/openai.yaml`),
    bundled system skills, `skill-creator`/`skill-installer` built-ins, and skill-disable entries in `config.toml`
    do not map directly to other providers.

cli_params:
  - flag: --cd, -C <path>
    description: Set the working directory before processing; affects which repo-scoped `.agents/skills/` are discovered.
    example: codex --cd ./packages/api
  - flag: --add-dir <path>
    description: Grant additional directories write access; may influence repo-relative skill references.
    example: codex --add-dir ../shared
  - flag: --config, -c key=value
    description: Override configuration values including `skills.config` and feature flags.
    example: codex -c features.multi_agent=true
  - flag: --enable skills
    description: Historical feature flag to opt into skills. No longer required as skills are enabled by default.
    example: codex --enable skills
  - flag: --disable skills
    description: Force-disable the skills feature for the session.
    example: codex --disable skills
  - flag: --strict-config
    description: Error on unrecognized config.toml fields.
    example: codex --strict-config

env_vars:
  - name: CODEX_HOME
    effect: Root for Codex state including config, auth, logs, sessions, and skills. Defaults to ~/.codex. Affects legacy ~/.codex/skills/ path.
  - name: CODEX_API_KEY
    effect: Provides an API key for a single non-interactive `codex exec` run.
  - name: CODEX_ACCESS_TOKEN
    effect: ChatGPT/Codex access token for trusted automation; can be piped to `codex login --with-access-token`.
  - name: RUST_LOG
    effect: Controls Rust log filtering/verbosity for the CLI and app-server.

changes: []

requires_claudine_update: true
reason: |
  Claudine's linking module should recognize Codex's first-class Agent Skills layout (`~/.agents/skills/`, `.agents/skills/`,
  legacy `~/.codex/skills/`, and `/etc/codex/skills/`). It should treat `SKILL.md` as the canonical entry point, understand the
  `agents/openai.yaml` sidecar as Codex-specific metadata, and map skill disabling via `skills.config` in `~/.codex/config.toml`.
  Portability classification should flag scripts, MCP dependencies, and `allowed-tools` references as needing rewrite when
  moving skills between providers.
---

# OpenAI Codex CLI Skills

## Overview

OpenAI Codex CLI supports first-class, file-system-based **skills** built on the [Agent Skills open standard](https://agentskills.io/specification). A skill is a directory containing a required `SKILL.md` file with YAML frontmatter and Markdown instructions, plus optional `scripts/`, `references/`, and `assets/` subdirectories. Skills can be invoked explicitly with `$skill-name` or the `/skills` command, or implicitly when a task matches the skill's `description`. Codex CLI is open source ([github.com/openai/codex](https://github.com/openai/codex)) and stores user state under `CODEX_HOME` (default `~/.codex`).

Codex does not natively read Claude Code's `.claude/skills/` directories; to share skills, symlink the entire skill directory into Codex's scan path.

## Locations

Skill resources are stored by scope:

| Scope | Location | Notes |
|---|---|---|
| System / bundled | `CODEX_HOME/skills/.system/` | Built-in skills such as `plan` and `skill-creator`. |
| Admin | `/etc/codex/skills/<skill-name>/SKILL.md` | Machine-wide administrator skills on Linux. |
| User (modern) | `~/.agents/skills/<skill-name>/SKILL.md` | Preferred cross-platform user scope. |
| User (legacy) | `~/.codex/skills/<skill-name>/SKILL.md` | Still supported for backward compatibility. |
| Repo (CWD) | `$CWD/.agents/skills/<skill-name>/SKILL.md` | Folder-specific skills. |
| Repo (parent) | `$CWD/../.agents/skills/<skill-name>/SKILL.md` | Shared skills in a parent directory within a Git repository. |
| Repo (root) | `$REPO_ROOT/.agents/skills/<skill-name>/SKILL.md` | Repository-wide skills. |

On Windows, `~/.codex` resolves to `%USERPROFILE%\.codex` and user skills use `%USERPROFILE%\.agents\skills\`. Codex supports symlinked skill directories (not individual files).

## File Format

A skill is a directory with `SKILL.md` as the required entry point:

```text
my-skill/
├── SKILL.md            # Required: metadata + instructions
├── scripts/            # Optional: executable utilities
├── references/         # Optional: supporting documentation
├── assets/             # Optional: templates, images, data files
└── agents/
    └── openai.yaml     # Optional: UI metadata and policy
```

`SKILL.md` contains YAML frontmatter between `---` markers followed by Markdown content.

Required frontmatter:

| Field | Purpose |
|---|---|
| `name` | Skill identifier: 1-64 chars, lowercase alphanumeric and hyphens, must match directory name. |
| `description` | Routing signal for implicit invocation (1-1024 chars). |

Optional frontmatter:

| Field | Purpose |
|---|---|
| `license` | License name or bundled license file reference. |
| `compatibility` | Environment requirements (≤500 chars). |
| `metadata` | Arbitrary string-key/string-value map. |
| `allowed-tools` | Space-separated pre-approved tool names (experimental). |

The optional `agents/openai.yaml` sidecar configures UI presentation, invocation policy, and tool dependencies:

```yaml
interface:
  display_name: "User-facing name"
  short_description: "User-facing description"
  icon_small: "./assets/small-logo.svg"
  icon_large: "./assets/large-logo.png"
  brand_color: "#3B82F6"
  default_prompt: "Optional surrounding prompt"

policy:
  allow_implicit_invocation: false  # default: true

dependencies:
  tools:
    - type: "mcp"
      value: "toolName"
      description: "Tool description"
```

## Discovery and Precedence

Codex discovers skills at startup using progressive disclosure:

1. **Metadata pass** — loads `name`, `description`, file path, and `agents/openai.yaml` for every skill.
2. **Instruction pass** — loads the full `SKILL.md` body only when Codex decides to use a skill.
3. **On-demand resources** — reads `scripts/`, `references/`, and `assets/` only when referenced.

Discovery order and precedence:

1. System/built-in skills.
2. Admin `/etc/codex/skills/`.
3. User `~/.agents/skills/` (or legacy `~/.codex/skills/`).
4. Repo `.agents/skills/` from CWD up to root, plus the parent directory (`$CWD/../.agents/skills/`).

Conflicting names are not merged; both skills remain available in selectors. Codex detects new skills automatically, but restart if updates to an existing skill do not appear.

Enable/disable mechanisms:

- `agents/openai.yaml` `policy.allow_implicit_invocation: false` — blocks implicit invocation while preserving explicit `$skill-name`.
- `~/.codex/config.toml` `[[skills.config]] enabled = false` — disables a skill by path.
- `--disable skills` / `--enable skills` — feature flag toggles; skills are enabled by default.

## Portability

Skills are portable across tools that implement the Agent Skills standard. Portable assets:

- `SKILL.md` Markdown body.
- Standard frontmatter: `name`, `description`, `license`, `compatibility`, `metadata`, `allowed-tools`.

Assets that need rewriting or host gating when moving to another provider:

- `scripts/` files (language, interpreter, OS availability).
- `agents/openai.yaml` sidecar (Codex-specific UI/policy).
- `dependencies.tools` MCP declarations (server availability and transport).
- `allowed-tools` values referencing Codex-specific tool names.
- Relative file references that assume a particular repository layout.

Codex-specific built-ins (`skill-creator`, `skill-installer`, `plan`) and per-skill disable entries in `config.toml` do not map to other providers.

## Claudine Linking Notes

For Claudine's cross-provider resource linking:

- Treat `~/.agents/skills/<name>/SKILL.md` and `.agents/skills/<name>/SKILL.md` as canonical user and repo skill locations.
- Also index the legacy `~/.codex/skills/` path and `/etc/codex/skills/` for Linux admin skills.
- Recognize `SKILL.md` as the entry point and `agents/openai.yaml` as Codex-specific metadata.
- Classify each linked asset as portable when the body uses only standard Agent Skills frontmatter and Markdown; flag assets containing `scripts/`, `agents/openai.yaml`, MCP dependencies, or Codex-specific conventions as needing rewrite.
- Account for `skills.config` `enabled = false` entries and `allow_implicit_invocation: false` when deciding whether a linked skill is active or visible in Codex.
- Built-in system skills (`plan`, `skill-creator`) and the `$skill-installer` command are Codex-specific; link only if a custom override exists in a user/repo path.

## Sources

- [OpenAI Codex CLI homepage](https://openai.com/codex/)
- [Codex CLI documentation](https://developers.openai.com/codex/cli/)
- [Codex Skills documentation](https://developers.openai.com/codex/skills/)
- [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)
- [Codex environment variables](https://developers.openai.com/codex/environment-variables)
- [Codex configuration reference](https://developers.openai.com/codex/config-reference)
- [Agent Skills specification](https://agentskills.io/specification)
- [OpenAI Codex GitHub repository](https://github.com/openai/codex)
- [PR #7412: Experimental skills support](https://github.com/openai/codex/pull/7412)
- [Issue #5291: SKILL.md support request](https://github.com/openai/codex/issues/5291)
- [OpenAI Skills catalog](https://github.com/openai/skills)
