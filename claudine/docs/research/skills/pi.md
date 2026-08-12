---
$schema: ./_schema.yaml
created: 2026-07-02
last_updated: 2026-07-03
agent: open_code
model: minimax/MiniMax-M3

homepage: https://pi.dev/
docs: https://pi.dev/docs/latest
skills_docs: https://pi.dev/docs/latest/skills

support: first_class

locations:
  - os: macos
    scope: user
    path: ~/.pi/agent/skills/
    notes: Default user skill directory. Root `.md` files and recursive `SKILL.md` directories are both accepted. Replaceable via `PI_CODING_AGENT_DIR`.
  - os: linux
    scope: user
    path: ~/.pi/agent/skills/
    notes: Default user skill directory. Root `.md` files and recursive `SKILL.md` directories are both accepted. Replaceable via `PI_CODING_AGENT_DIR`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\skills\\"
    notes: Default user skill directory. Root `.md` files and recursive `SKILL.md` directories are both accepted. Replaceable via `PI_CODING_AGENT_DIR`.
  - os: macos
    scope: user
    path: ~/.agents/skills/
    notes: Agent-compatible global skill directory. Recursive `SKILL.md` directories only — direct root `.md` files are ignored.
  - os: linux
    scope: user
    path: ~/.agents/skills/
    notes: Agent-compatible global skill directory. Recursive `SKILL.md` directories only — direct root `.md` files are ignored.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.agents\\skills\\"
    notes: Agent-compatible global skill directory. Recursive `SKILL.md` directories only — direct root `.md` files are ignored.
  - os: macos
    scope: repo
    path: .pi/skills/
    notes: Project skill directory. Loaded only after the project is trusted. Root `.md` files and recursive `SKILL.md` directories are both accepted.
  - os: linux
    scope: repo
    path: .pi/skills/
    notes: Project skill directory. Loaded only after the project is trusted. Root `.md` files and recursive `SKILL.md` directories are both accepted.
  - os: windows
    scope: repo
    path: ".pi\\skills\\"
    notes: Project skill directory. Loaded only after the project is trusted. Root `.md` files and recursive `SKILL.md` directories are both accepted.
  - os: macos
    scope: repo
    path: .agents/skills/
    notes: Project agent-compatible skill directory discovered in `cwd` and ancestor directories up to the git repository root, or filesystem root when not in a repo. Loaded only after project trust. Recursive `SKILL.md` directories only.
  - os: linux
    scope: repo
    path: .agents/skills/
    notes: Project agent-compatible skill directory discovered in `cwd` and ancestor directories up to the git repository root, or filesystem root when not in a repo. Loaded only after project trust. Recursive `SKILL.md` directories only.
  - os: windows
    scope: repo
    path: ".agents\\skills\\"
    notes: Project agent-compatible skill directory discovered in `cwd` and ancestor directories up to the git repository root, or filesystem root when not in a repo. Loaded only after project trust. Recursive `SKILL.md` directories only.
  - os: macos
    scope: user
    path: ~/.pi/agent/settings.json
    notes: Global `skills` array of paths (file or directory, with glob/exclusion/force-include/force-exclude modifiers). Resolution base is `~/.pi/agent`.
  - os: linux
    scope: user
    path: ~/.pi/agent/settings.json
    notes: Global `skills` array of paths (file or directory, with glob/exclusion/force-include/force-exclude modifiers). Resolution base is `~/.pi/agent`.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\settings.json"
    notes: Global `skills` array of paths (file or directory, with glob/exclusion/force-include/force-exclude modifiers). Resolution base is `~/.pi/agent`.
  - os: macos
    scope: repo
    path: .pi/settings.json
    notes: Project `skills` array and project package entries load only after project trust. Resolution base is `.pi`.
  - os: linux
    scope: repo
    path: .pi/settings.json
    notes: Project `skills` array and project package entries load only after project trust. Resolution base is `.pi`.
  - os: windows
    scope: repo
    path: ".pi\\settings.json"
    notes: Project `skills` array and project package entries load only after project trust. Resolution base is `.pi`.
  - os: macos
    scope: user
    path: ~/.pi/agent/npm/<package>/skills/ and ~/.pi/agent/git/<host>/<repo>/skills/
    notes: Installed user package resources. Conventional `skills/` directory is auto-discovered, or `package.json` `pi.skills` entries can point elsewhere inside the package. Package objects also accept `skills` array filters.
  - os: linux
    scope: user
    path: ~/.pi/agent/npm/<package>/skills/ and ~/.pi/agent/git/<host>/<repo>/skills/
    notes: Installed user package resources. Conventional `skills/` directory is auto-discovered, or `package.json` `pi.skills` entries can point elsewhere inside the package. Package objects also accept `skills` array filters.
  - os: windows
    scope: user
    path: "%USERPROFILE%\\.pi\\agent\\npm\\<package>\\skills\\ and %USERPROFILE%\\.pi\\agent\\git\\<host>\\<repo>\\skills\\"
    notes: Installed user package resources. Conventional `skills/` directory is auto-discovered, or `package.json` `pi.skills` entries can point elsewhere inside the package. Package objects also accept `skills` array filters.
  - os: macos
    scope: repo
    path: .pi/npm/<package>/skills/ and .pi/git/<host>/<repo>/skills/
    notes: Installed project package resources; loaded only after project trust.
  - os: linux
    scope: repo
    path: .pi/npm/<package>/skills/ and .pi/git/<host>/<repo>/skills/
    notes: Installed project package resources; loaded only after project trust.
  - os: windows
    scope: repo
    path: ".pi\\npm\\<package>\\skills\\ and .pi\\git\\<host>\\<repo>\\skills\\"
    notes: Installed project package resources; loaded only after project trust.
  - os: macos
    scope: extension
    path: "<extension-contributed runtime paths>"
    notes: Extensions can contribute skill paths at runtime via the `resources_discover` hook (no stable provider-owned filesystem location).
  - os: linux
    scope: extension
    path: "<extension-contributed runtime paths>"
    notes: Extensions can contribute skill paths at runtime via the `resources_discover` hook (no stable provider-owned filesystem location).
  - os: windows
    scope: extension
    path: "<extension-contributed runtime paths>"
    notes: Extensions can contribute skill paths at runtime via the `resources_discover` hook (no stable provider-owned filesystem location).
  - os: macos
    scope: other
    path: ~/.pi/agent/trust.json
    notes: Project trust store. Records per-cwd or per-parent decisions that gate loading of project-local `.pi/` resources and `.agents/skills` ancestors.
  - os: linux
    scope: other
    path: ~/.pi/agent/trust.json
    notes: Project trust store. Records per-cwd or per-parent decisions that gate loading of project-local `.pi/` resources and `.agents/skills` ancestors.
  - os: windows
    scope: other
    path: "%USERPROFILE%\\.pi\\agent\\trust.json"
    notes: Project trust store. Records per-cwd or per-parent decisions that gate loading of project-local `.pi/` resources and `.agents/skills` ancestors.
  - os: macos
    scope: other
    path: "--skill <path>"
    notes: Repeatable session-only skill file or directory path. Additive even when `--no-skills` is set; path de-duplication uses canonical paths.
  - os: linux
    scope: other
    path: "--skill <path>"
    notes: Repeatable session-only skill file or directory path. Additive even when `--no-skills` is set; path de-duplication uses canonical paths.
  - os: windows
    scope: other
    path: "--skill <path>"
    notes: Repeatable session-only skill file or directory path. Additive even when `--no-skills` is set; path de-duplication uses canonical paths.

format:
  file_names:
    - SKILL.md
    - "*.md (Pi-native root files in ~/.pi/agent/skills/, .pi/skills/, package skills/, settings entries, and CLI --skill paths)"
  frontmatter: true
  required_fields:
    - description
    - name
  optional_fields:
    - license
    - compatibility
    - metadata
    - allowed-tools
    - disable-model-invocation
  body_format: markdown
  notes: |
    Pi implements the Agent Skills standard and treats `name` and `description` as required by the standard. The loader is otherwise lenient: unknown frontmatter fields are ignored, name violations produce warnings but still load, and descriptions longer than 1024 characters warn but still load. The one hard requirement is a non-blank `description` — without it the file is rejected as a warning and not loaded. `name` defaults to the parent directory name when omitted, and the directory-name requirement is intentionally NOT enforced (Pi documents this as a deliberate deviation for cross-tool shared directories).

    A directory that contains a `SKILL.md` is treated as a single skill root and Pi does not recurse deeper below it for additional skills. Otherwise Pi recurses into subdirectories to find nested `SKILL.md` files. Direct root `*.md` files are loaded as individual skills only in Pi-native roots (`~/.pi/agent/skills/`, `.pi/skills/`, package `skills/`, configured skill directories, explicit directory paths); they are ignored in `~/.agents/skills/` and project `.agents/skills/`. Sibling files in a skill directory (scripts, references, assets) are freeform — the system prompt tells the model to resolve relative paths against the skill directory.

    Directory scans honor `.gitignore`, `.ignore`, and `.fdignore`. Dot-prefixed entries and `node_modules` are skipped during recursion. Valid symlinks are followed; broken symlinks are silently skipped. The loader dedupes skills by canonical path so symlinked duplicates are not loaded twice.

discovery:
  mechanism: |
    Pi's `DefaultResourceLoader` resolves skills at startup or reload from: (a) the configured global and project `skills` arrays in `settings.json`, (b) installed npm/git packages that declare `pi.skills` in their `package.json` or ship a conventional `skills/` directory, (c) the built-in `~/.pi/agent/skills/` and `.pi/skills/` directories, (d) the agent-compatible `~/.agents/skills/` and project `.agents/skills/` ancestor directories (only after project trust), (e) explicit `--skill` CLI paths, and (f) extension-contributed paths. Frontmatter metadata is parsed up front; the full body loads on demand. Visible skills (`disable-model-invocation: false`) are appended to the system prompt as `<available_skills>` entries with `name`, `description`, and absolute `location`, but only when the `read` tool is available. The model is told to use `read` to load the full `SKILL.md` on demand.

    Users can force a skill through `/skill:<name> [args]` in any mode (interactive, RPC, JSON, print). The command reads the file, strips frontmatter, wraps the body in a `<skill name="..." location="...">` block, adds `References are relative to <baseDir>.`, and appends arguments after the block. `/skill:name` commands are toggleable via `enableSkillCommands: false` in settings; `disable-model-invocation: true` only hides a skill from `<available_skills>`, it does not remove the slash command.

    Extensions can supply additional skill paths through the `resources_discover` hook, and SDK consumers can filter, merge, or replace the loaded skill list via `skillsOverride` on `DefaultResourceLoader`.
  precedence: |
    Collision rule: first writer wins across the resource accumulator. Same-name collisions produce a `ResourceDiagnostic` of type `collision` carrying the winner and loser paths; the later skill is not loaded and entries are not merged. The resource accumulator evaluates resources in the order project `.pi/skills` (when trusted) → project `.agents/skills` ancestors (when trusted) → user `~/.pi/agent/skills` → user `~/.agents/skills`, then merges CLI `--skill` and extension paths into the effective path list before loading. Canonical-path de-duplication prevents symlinked duplicates. The `resourcePrecedenceRank` function used by the package manager ranks project + local-settings (rank 0) above project + auto (rank 1) above user + local-settings (rank 2) above user + auto (rank 3) above package resources (rank 4). Package identity deduplication makes project packages override user packages for the same identity.

    Project settings override global settings through standard merge rules. Resource arrays in settings support glob patterns, `!pattern` exclusions, `+path` force-includes, and `-path` force-excludes. Paths in `~/.pi/agent/settings.json` resolve relative to `~/.pi/agent`; paths in `.pi/settings.json` resolve relative to `.pi`; absolute paths and `~` are supported.
  enable_disable: |
    `--no-skills` / `-ns` disables normal skill discovery and loading, but explicit `--skill <path>` entries still load (the path list is built from CLI skill paths plus no other defaults). Removing or excluding a path disables that skill; package filters in settings (the `skills` array on a package object) can selectively include or exclude skills from a package. `enableSkillCommands: false` disables `/skill:name` command registration but does not remove model-visible skills from the system prompt. `disable-model-invocation: true` hides a skill from the system prompt and forces explicit `/skill:name` invocation.

    Project-local resources (`.pi/skills`, `.pi/settings.json`, project packages, and project `.agents/skills` ancestors) require project trust. Interactive mode prompts according to `defaultProjectTrust`; non-interactive modes do not prompt and instead obey `defaultProjectTrust` from global settings (`ask`/`never` skip project resources, `always` loads them). `--approve` / `-a` trusts project-local resources for one run; `--no-approve` / `-na` ignores them for one run. Saved trust decisions live in `~/.pi/agent/trust.json`, keyed by canonical path, with nearest-current-or-parent lookup.
  notes: |
    Pi has no built-in permission sandbox. Trust controls whether project resources are loaded; it does not restrict what loaded skills, extensions, tools, or model output can ask the process to do. Extensions can affect skills in two ways: the `resources_discover` hook can return skill paths during startup and reload, and SDK callers can pass `skillsOverride` to filter, merge, or replace the loaded skill list. Package resources can be enabled or disabled through `pi config`, global/project settings `packages`, or per-package `skills` filters. `pi config` opens a TUI that lists installed package resources and lets the user toggle them per scope. The `pi list` subcommand prints configured packages and their installed paths.

portability:
  portable: true
  non_portable_assets:
    - "Pi-native direct root `*.md` skills; many Agent Skills consumers require `SKILL.md` inside a directory."
    - "`disable-model-invocation` semantics; other providers may use a different auto-invocation control or none."
    - "Experimental `allowed-tools` semantics and tool names."
    - "Pi settings/package wiring: `settings.json` `skills` array, package `pi.skills` manifest entries, package filters, `pi config` enabled/disabled state, and installed package roots under `.pi` or `~/.pi/agent`."
    - "Project trust state in `~/.pi/agent/trust.json` and one-shot trust flags."
    - "Extension-discovered skill paths via the `resources_discover` hook and SDK `skillsOverride` virtual skills."
    - "Relative scripts/assets that assume Pi's prompt wording, built-in `read`/`bash` tools, local executable availability, or package-installed dependencies."
  rewrite_needed: true
  notes: |
    A directory skill with `SKILL.md`, YAML frontmatter, Markdown body, `name`, and `description` is linkable as an Agent Skills artifact. Claudine can preserve sibling files and map the directory into another provider's skill root. Direct root `*.md` skills should be rewritten into `<name>/SKILL.md` form for stricter providers. Standard frontmatter (`name`, `description`, `license`, `compatibility`, `metadata`) ports cleanly; `allowed-tools`, `disable-model-invocation`, package manifests, settings arrays, trust files, and extension/SDK-provided skills are provider-specific. Skill bodies that mention Pi-specific commands, `pi config`, `PI_CODING_AGENT_DIR`, Pi packages, or Pi's built-in tool names need review before linking elsewhere.

cli_params:
  - flag: "--skill <path>"
    description: "Load a skill file or directory for the current session. Repeatable and additive even with `--no-skills`. Source: `cli/args.ts` (`--skill` branch) and `core/skills.ts` `loadSkills()` `skillPaths`."
    example: pi --skill ./skills/review/SKILL.md "review this repo"
  - flag: "--no-skills, -ns"
    description: "Disable normal skill discovery and loading. Explicit `--skill` paths still load. Source: `cli/args.ts` and `core/resource-loader.ts` `noSkills` constructor option."
    example: pi --no-skills --skill ./one-off/SKILL.md -p "use only this skill"
  - flag: "--approve, -a"
    description: "Trust project-local resources for this run, including `.pi/skills`, `.pi/settings.json`, project `.agents/skills` ancestors, and project package skills. Maps to `projectTrustOverride: true` in `cli/args.ts`. Source: `core/trust-manager.ts` and `core/resource-loader.ts` `resolveProjectTrust`."
    example: pi --approve "use project skills"
  - flag: "--no-approve, -na"
    description: "Ignore project-local resources for this run, including project skills. Maps to `projectTrustOverride: false`."
    example: pi --no-approve -p "summarize safely"
  - flag: "--extension <path>, -e <path>"
    description: Load a session-only extension. Extensions can contribute skill paths at runtime through the `resources_discover` hook.
    example: pi -e ./extensions/resources.ts "run with extension resources"
  - flag: "--no-extensions, -ne"
    description: Disable extension discovery. This can indirectly prevent extension-contributed skill paths; explicit `-e` paths still work.
    example: pi --no-extensions "ignore installed extension resources"
  - flag: "--tools <tools>, -t <tools>"
    description: "Tool allowlist. If `read` is absent, Pi does not append available skills to the system prompt. Source: `core/system-prompt.ts` `customPromptHasRead` check."
    example: pi --tools read,bash "load matching skills"
  - flag: "--exclude-tools <tools>, -xt <tools>"
    description: Tool denylist. Excluding `read` prevents available-skill prompt injection.
    example: pi --exclude-tools read "do not expose skill metadata"
  - flag: "--no-tools, -nt"
    description: Disable all tools by default. This removes `read`, so model-visible skill listings are not appended.
    example: pi --no-tools -p "answer without tools or skills"
  - flag: "--no-builtin-tools, -nbt"
    description: Disable built-in tools by default but keep extension/custom tools enabled. This removes built-in `read` unless an extension provides a replacement.
    example: pi --no-builtin-tools -e ./read-extension.ts "use extension tools"
  - flag: "pi install <source> [-l]"
    description: Install a Pi package (npm or git source) and persist it in `settings.json` (`-l` writes project settings). The installed package can contribute skills through `skills/` or package `pi.skills`.
    example: pi install npm:@scope/package
  - flag: "pi remove <source> [-l]"
    description: Remove a package source from settings, disabling its packaged skills for that scope.
    example: pi remove npm:@scope/package
  - flag: "pi uninstall <source> [-l]"
    description: Alias for `pi remove`.
    example: pi uninstall npm:@scope/package
  - flag: "pi update [source]"
    description: Update Pi and/or installed packages; package updates can change packaged skill files. Bare `pi update` updates only pi; `pi update --all` updates pi and packages.
    example: pi update --all
  - flag: pi list
    description: List installed extensions from settings, including skill-bearing packages and their installed paths.
    example: pi list
  - flag: pi config
    description: "Open the TUI for enabling or disabling package and local resources, including skills. Source: `core/package-manager.ts` `listConfiguredPackages()`."
    example: pi config

env_vars:
  - name: PI_CODING_AGENT_DIR
    effect: Replaces the global agent config directory, moving user settings, trust store, packages, and the default `skills/` directory away from `~/.pi/agent`. The skill paths `~/.pi/agent/skills/` and `~/.agents/skills/` continue to resolve relative to the actual home directory.
  - name: PI_CODING_AGENT_SESSION_DIR
    effect: Replaces session storage only; it does not directly affect skill discovery. Overridden by `--session-dir`.
  - name: PI_OFFLINE
    effect: Disables startup network operations and package/update checks (`1`/`true`/`yes`). Existing local skill discovery still works; package install/update network activity is suppressed.
  - name: PI_PACKAGE_DIR
    effect: Overrides the package directory used to locate the installed Pi package itself; useful for packaged/Nix/Guix layouts, not a user skill root.
  - name: PI_SKIP_VERSION_CHECK
    effect: Set to `1` to disable the Pi version update check (`https://pi.dev/api/latest-version`). Does not affect skill discovery.
  - name: PI_TELEMETRY
    effect: Override install telemetry when set (`1`/`true`/`yes` or `0`/`false`/`no`). Independent of `enableInstallTelemetry`. Does not affect skill discovery.
  - name: PI_SHARE_VIEWER_URL
    effect: Base URL for `/share` command (default `https://pi.dev/session/`). Independent of skill discovery.
  - name: PI_EXPERIMENTAL
    effect: Set to `1` to opt into the experimental first-time setup flow (theme + analytics). Does not affect skill discovery.

changes:
  - "Verified against Pi docs at `https://pi.dev/docs/latest/skills` and source files `packages/coding-agent/src/core/skills.ts`, `core/resource-loader.ts`, `core/package-manager.ts`, `core/trust-manager.ts`, `core/system-prompt.ts`, `core/agent-session.ts`, and `cli/args.ts`."
  - "Confirmed `name` is required by Pi's published docs (with a documented deviation: Pi does not enforce name == directory name) and that descriptions longer than 1024 chars warn but still load."
  - "Confirmed the agent-compatible rule: root `.md` files are ignored in `~/.agents/skills/` and project `.agents/skills/` but accepted in `~/.pi/agent/skills/`, `.pi/skills/`, package `skills/`, settings entries, and CLI paths."
  - "Verified that skill collisions are first-writer-wins and emit a `ResourceDiagnostic` with `winnerPath` and `loserPath`; canonical-path de-duplication prevents symlinked duplicates from loading twice."
  - "Verified the system prompt appends `<available_skills>` only when `read` is in the selected tools; `disable-model-invocation: true` excludes a skill from that block but `/skill:name` still works."
  - "Verified the project-trust flow including `defaultProjectTrust` fallback (`ask`/`always`/`never`) for non-interactive modes, `--approve`/`--no-approve` one-shot overrides, and saved decisions in `~/.pi/agent/trust.json`."
  - "Verified `--skill` is additive even with `--no-skills`, and CLI short aliases `-ns`, `-ne`, `-np`, `-nt`, `-nbt`, `-xt`, `-a`, `-na` from `cli/args.ts`."
  - "Documented `--no-themes`, `--no-context-files`, `-nc`, `--no-builtin-tools`, and the `pi list` subcommand (previously omitted from the frontmatter list)."
  - "Documented `PI_SKIP_VERSION_CHECK`, `PI_TELEMETRY`, `PI_SHARE_VIEWER_URL`, and `PI_EXPERIMENTAL` env vars (previously omitted)."
  - "Added `~/.pi/agent/trust.json` and the extension `resources_discover` / SDK `skillsOverride` paths to the locations list."
  - "Updated `last_updated` and `model` to reflect this 2026-07-03 refresh; `created` preserved as 2026-07-02."

requires_claudine_update: true
reason: |
  Claudine should add Pi as a first-class skill-linking target/source once provider metadata is generated: recognize `~/.pi/agent/skills`, `.pi/skills`, `~/.agents/skills`, project `.agents/skills` ancestor discovery, settings/package skill paths, `--skill` session paths, first-writer-wins collision behavior, project-trust gating, and Pi's direct-root-`*.md` compatibility case. Linking should rewrite direct `.md` skills into directory `SKILL.md` form for stricter providers and should not try to link virtual SDK override skills or extension-generated resources without a real file path.
---

# Pi Agent Skills

## Overview

Pi has first-class Agent Skills support. The official skills documentation describes skills as reusable capability packages loaded on demand, and current source implements a dedicated `Skill` model, scanner, frontmatter parser, system-prompt formatter, `/skill:name` command expander, and SDK/resource-loader override path.

The runtime is intentionally progressive-disclosure based. At startup or reload, Pi scans skill locations and reads frontmatter metadata. Skills visible to model invocation are rendered into the system prompt as XML-like `<available_skills>` entries with `name`, `description`, and `location`. The full `SKILL.md` body is not automatically injected. The model is told to use the `read` tool when a task matches a skill description. Users can force a skill through `/skill:<name>`, which reads the skill file immediately and injects its body into the user message.

Pi documents that it implements the Agent Skills standard but remains lenient. The most important practical difference from the standard is that Pi does not require the `name` field to match the parent directory, because that rule is suboptimal for shared skill directories used across multiple agent harnesses. Pi also still supports direct root `.md` files as Pi-native skills in Pi-managed skill directories, while agent-compatible `.agents/skills` directories only accept nested `SKILL.md` skills.

Local inspection for this run found Pi installed at `/Users/ken/.bun/bin/pi` (CLI version `0.73.1`; latest published `0.80.3`). The host's `HOME` was `/Users/ken/.claudine`, so Pi's default agent directory resolved to `/Users/ken/.claudine/.pi/agent`. That directory contained `sessions/` and `auth.json`, but no observed `skills/` directory. The agent-compatible `~/.agents/skills/find-skills/SKILL.md` was present and is exactly the kind of artifact Pi scans there. This is host evidence only; portable paths above use `~`.

## Locations

Pi's default user config root is `~/.pi/agent`, computed from the current app name as `PI_CODING_AGENT_DIR` when overridden. Under that root, `skills/` is the native user skill directory. Pi also scans `~/.agents/skills/` as a global agent-compatible skill directory.

Project skill loading is trust-gated. After a project is trusted, Pi loads `.pi/skills/` from the current working directory and project `.agents/skills/` directories from the current directory and ancestors. Ancestor scanning stops at the git repository root when one is found; outside a git repo it continues to the filesystem root. The global `~/.agents/skills/` path is explicitly excluded from the project trust scan even when the current directory is the user's home, because `hasTrustRequiringProjectResources()` skips that path.

Settings and packages are additional durable sources:

| Scope | File or Directory | Behavior |
|---|---|---|
| User settings | `~/.pi/agent/settings.json` | `skills` can name skill files/directories and patterns. Paths resolve relative to `~/.pi/agent` unless absolute or `~`-prefixed. |
| Project settings | `.pi/settings.json` | Same `skills` array, loaded only after project trust. Paths resolve relative to `.pi`. |
| User npm packages | `~/.pi/agent/npm/<package>/` | Conventional `skills/` directory or `package.json` `pi.skills` entries; package objects can also filter skills. |
| User git packages | `~/.pi/agent/git/<host>/<repo>/` | Same package resource rules as npm packages. |
| Project npm packages | `.pi/npm/<package>/` | Loaded only after project trust. |
| Project git packages | `.pi/git/<host>/<repo>/` | Loaded only after project trust. |
| CLI | `--skill <path>` | Session-only explicit file or directory. Loads even with `--no-skills`. |
| Extension / SDK | `resources_discover` hook or `skillsOverride` | Can add, filter, or replace skills without a stable provider-owned filesystem location. |
| Trust store | `~/.pi/agent/trust.json` | Per-cwd or per-parent decisions that gate `.pi/` and `.agents/skills` project resources. |

Package directories support two discovery styles. If `package.json` contains a `pi.skills` manifest array, those paths are used relative to the package root. If no `pi` manifest is present, a conventional `skills/` directory is auto-discovered. Package and settings filters can include glob patterns, `!pattern` exclusions, `+path` force-includes, and `-path` force-excludes.

## File Format

The durable skill artifact is either a directory containing `SKILL.md` plus optional sibling files, or a Pi-native root `.md` skill file in locations that allow direct files.

```text
my-skill/
├── SKILL.md
├── scripts/
│   └── process.sh
├── references/
│   └── api-reference.md
└── assets/
    └── template.json
```

`SKILL.md` is Markdown with YAML frontmatter. Pi recognizes these frontmatter fields:

| Field | Required by Pi | Behavior |
|---|---:|---|
| `name` | Yes (with documented leniency) | Skill name. Defaults to the parent directory name when omitted. Invalid names warn but still load. Pi does not enforce `name == directory_name`. |
| `description` | Yes | Routing text. Missing or blank descriptions warn and prevent the skill from loading. Descriptions longer than 1024 characters warn but still load. |
| `license` | No | Standard Agent Skills metadata; Pi preserves it as frontmatter but does not use it for runtime behavior. |
| `compatibility` | No | Standard Agent Skills metadata (max 500 chars per the standard). Pi preserves it but does not use it for runtime behavior. |
| `metadata` | No | Standard arbitrary metadata; Pi preserves it but does not use it for runtime behavior. |
| `allowed-tools` | No | Documented as experimental. Treat as Pi/provider-specific unless a destination provider explicitly supports the same semantics. |
| `disable-model-invocation` | No | When `true`, the skill is omitted from `<available_skills>` and can still be used via `/skill:name`. |

Unknown frontmatter fields are ignored by the Pi loader. Pi validates the standard name shape but treats most violations as warnings: names must be 1–64 characters, lowercase letters, digits, and hyphens only, with no leading/trailing hyphen and no consecutive hyphens. Valid examples from the docs are `pdf-processing`, `data-analysis`, `code-review`; invalid examples are `PDF-Processing`, `-pdf`, `pdf--processing`.

The body is CommonMark/GFM-style Markdown instructions. Supporting files are freeform. Pi's system prompt tells the model that relative paths in a skill file are relative to the skill directory, and `/skill:name` expansion injects an explicit "References are relative to ..." line. Scripts and assets therefore remain file-backed resources beside the skill, not embedded metadata.

Discovery behavior depends on the directory type:

| Source shape | Accepted entries |
|---|---|
| Pi-native roots such as `~/.pi/agent/skills/`, `.pi/skills/`, configured directories, package `skills/`, and CLI directory paths | Direct root `*.md` files and recursive directories containing `SKILL.md`. |
| Agent-compatible roots `~/.agents/skills/` and project `.agents/skills/` | Recursive directories containing `SKILL.md`; direct root `*.md` files are ignored. |
| A directory containing `SKILL.md` | That directory is one skill root and scanning does not recurse deeper below it for more skills. |

Scans honor `.gitignore`, `.ignore`, and `.fdignore` files. Dot-prefixed entries and `node_modules` are skipped during recursion. Valid symlinks are followed; broken symlinks are skipped silently. The loader dedupes by canonical path so a symlink pointing to an already-loaded skill is not added twice.

## Discovery and Precedence

Pi's `DefaultResourceLoader` is the central implementation. It resolves resources from settings, packages, default filesystem locations, CLI paths, and extension-provided paths, then calls `loadSkills`. The loader can be reloaded during an interactive session via `/reload`; the UI reloads keybindings, extensions, skills, prompts, and themes together.

Project trust is part of discovery. A project requires trust when Pi finds `.pi/settings.json`, `.pi/extensions`, `.pi/skills`, `.pi/prompts`, `.pi/themes`, `.pi/SYSTEM.md`, `.pi/APPEND_SYSTEM.md`, or project `.agents/skills` in the current directory or an ancestor. Trusting the project allows those project resources to load. Declining trust skips protected project resources. Non-interactive modes (`-p`, `--mode json`, `--mode rpc`) do not prompt; with no saved decision they obey `defaultProjectTrust` from global settings, where `ask` and `never` skip resources and `always` loads them. `--approve` and `--no-approve` override the trust decision for one run. Saved decisions live in `~/.pi/agent/trust.json`, keyed by canonical path, with nearest-current-or-parent lookup.

Skill name collisions are first-writer-wins. The loader keeps a `Map<string, Skill>` and records a collision diagnostic (`type: "collision"`) when a later skill has an already-loaded name, with explicit `winnerPath` and `loserPath`. The later skill is not loaded and skills are not merged. Canonical-path de-duplication means the same file reached through multiple symlinks is also de-duped.

Within the implemented resource accumulator, trusted project `.pi/skills` are added before trusted project `.agents/skills` ancestor directories, then user `~/.pi/agent/skills`, then user `~/.agents/skills`. CLI `--skill` and extension paths are merged into the effective path list before `loadSkills` runs, with canonical path de-duplication. Package precedence is handled before skill loading: when the same package identity appears in both global and project settings, the project package entry wins.

Runtime application has two paths:

1. **Model-visible skills**: if the `read` tool is available and a skill does not set `disable-model-invocation: true`, Pi appends a skills section to the system prompt. The `formatSkillsForPrompt` filter excludes `disableModelInvocation` skills; the system-prompt builder in `core/system-prompt.ts` skips the entire section if `read` is not selected. If `read` is disabled through `--no-tools`, `--tools` without `read`, `--exclude-tools read`, or equivalent tool selection, Pi does not append the skill list.
2. **Explicit skill commands**: `/skill:<name> [args]` looks up a loaded skill by exact name, reads the file, strips frontmatter, wraps the body in a `<skill name="..." location="...">` block, and appends arguments. This path is controlled by `enableSkillCommands` in settings (default `true`) or `/settings` in interactive mode. Skill commands work in interactive, RPC, JSON, and print modes because the `AgentSession` expands them rather than interactive mode.

Extensions can affect skills in two ways. A CLI `-e` or installed extension can contribute resource paths through the `resources_discover` hook, and SDK consumers can provide `skillsOverride` on `DefaultResourceLoader` to filter, merge, or replace the loaded list. These are runtime/resource-loader behaviors rather than portable skill artifacts.

## Portability

Standard Pi directory skills are portable Agent Skills artifacts when they use `SKILL.md`, YAML frontmatter, Markdown body, and common fields such as `name`, `description`, `license`, `compatibility`, and `metadata`. Claudine can link those as-is into another provider's skill root and copy sibling files.

Several Pi forms need provider-specific handling:

| Artifact | Classification | Claudine behavior |
|---|---|---|
| `<skill>/SKILL.md` with standard fields | Linkable | Preserve directory, body, and sibling files. |
| Direct root `*.md` skill | Rewrite needed | Convert to `<derived-or-frontmatter-name>/SKILL.md` for providers that require directory skills. |
| `disable-model-invocation` | Provider-specific metadata | Map only to destinations with an equivalent auto-invocation disable feature; otherwise preserve only as inert metadata when safe. |
| `allowed-tools` | Provider/tool-specific | Do not blindly map; tool names and permission semantics are provider-specific. |
| `settings.json` `skills` array and package filters | Non-portable wiring | Use as discovery input, not as a linked artifact. |
| `~/.pi/agent/trust.json` | Non-portable state | Do not link. It is host/project trust state. |
| Package `pi.skills` manifest entries | Provider-specific wiring | Resolve to concrete skill files when possible; do not require destination providers to understand Pi package manifests. |
| Extension-discovered or SDK virtual skills | Non-portable unless file-backed | Link only when Claudine can resolve a real source file/directory and stable sibling assets. |

The main semantic portability hazard is that Pi's system prompt includes the absolute skill file location and expects the model to use Pi's `read` tool to load it. That runtime detail should not be copied into the skill body. Skill bodies that mention Pi-specific commands, `pi config`, `PI_CODING_AGENT_DIR`, Pi packages, or Pi's built-in tool names need review before linking elsewhere.

## Claudine Linking Notes

Claudine should model Pi as a first-class skills provider with these rules:

- Discover user skills from `~/.pi/agent/skills/` and `~/.agents/skills/`, honoring `PI_CODING_AGENT_DIR` for the native Pi root.
- Discover repo skills from `.pi/skills/` and `.agents/skills/` in the current directory and ancestors up to the git root.
- Treat project Pi resources as trust-gated metadata. The linker should report project skill paths but should not imply that Pi will load them without project trust.
- Accept `SKILL.md` directory skills and Pi-native direct root `.md` skills. Normalize direct `.md` skills to directory `SKILL.md` form when exporting to stricter providers.
- Preserve sibling files when linking a skill directory. Avoid flattening references, scripts, and assets because Pi relies on relative path behavior.
- Implement collision reporting as first-writer-wins for Pi parity, not repo-over-user override. Surface both winner and loser paths when possible.
- Do not link `trust.json`, package install directories, package manager cache state, or `settings.json` itself as a skill. Use those files only to discover concrete skill paths.
- Treat extension and SDK virtual skills as non-linkable unless they have a concrete `filePath` and `baseDir`.
- Mark `allowed-tools`, `disable-model-invocation`, and Pi-specific body instructions as rewrite/review points in portability metadata.
- Treat Pi's system-prompt `<available_skills>` injection as conditional on `read` being selected. If a destination provider requires skills always-visible, note the divergence.

This research implies Claudine generated provider metadata and linking rules should change when Pi is added to the compiled provider roster. No code change is required for this research document itself.

## Changelog

- **2026-07-03** — Re-verified against current Pi docs (`https://pi.dev/docs/latest/skills`, `settings`, `extensions`) and source files `packages/coding-agent/src/core/{skills,resource-loader,package-manager,trust-manager,system-prompt,agent-session}.ts` and `cli/args.ts`. Confirmed `name` is required by the published docs (with the documented leniency around directory-name parity), and that descriptions longer than 1024 chars warn but still load. Confirmed the agent-compatible rule: root `.md` files are ignored in `~/.agents/skills/` and project `.agents/skills/`, but accepted in `~/.pi/agent/skills/`, `.pi/skills/`, package `skills/`, settings entries, and CLI paths. Verified first-writer-wins collisions with explicit `winnerPath`/`loserPath`, canonical-path de-duplication of symlinked duplicates, and the conditional system-prompt append (only when `read` is selected). Documented `--no-themes`, `--no-context-files`, `-nc`, `--no-builtin-tools`, the `pi list` subcommand, and `PI_SKIP_VERSION_CHECK`/`PI_TELEMETRY`/`PI_SHARE_VIEWER_URL`/`PI_EXPERIMENTAL` env vars (previously omitted). Added `~/.pi/agent/trust.json` and the extension `resources_discover` / SDK `skillsOverride` paths to the locations list. Refreshed `last_updated` and `model`; preserved `created` as 2026-07-02.

## Sources

- [Pi Skills documentation](https://pi.dev/docs/latest/skills)
- [Pi Settings documentation](https://pi.dev/docs/latest/settings)
- [Pi Extensions documentation](https://pi.dev/docs/latest/extensions)
- [Pi Packages documentation](https://pi.dev/docs/latest/packages)
- [Pi repository](https://github.com/earendil-works/pi)
- [Source: `packages/coding-agent/src/core/skills.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/skills.ts)
- [Source: `packages/coding-agent/src/core/resource-loader.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/resource-loader.ts)
- [Source: `packages/coding-agent/src/core/package-manager.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/package-manager.ts)
- [Source: `packages/coding-agent/src/core/trust-manager.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/trust-manager.ts)
- [Source: `packages/coding-agent/src/core/system-prompt.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/system-prompt.ts)
- [Source: `packages/coding-agent/src/core/agent-session.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/core/agent-session.ts)
- [Source: `packages/coding-agent/src/cli/args.ts`](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/src/cli/args.ts)
- [Agent Skills specification](https://agentskills.io/specification)
- [Pi changelog (latest: 0.80.3)](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/CHANGELOG.md)