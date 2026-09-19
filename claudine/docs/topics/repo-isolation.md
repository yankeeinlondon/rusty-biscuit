# Repo Isolation

`--repo` on a Claudine wrapper command is a repo-biased launch mode, not a filesystem sandbox.

Its job is to make the wrapped agent behave as though the repository's shared resources are the primary source of truth, while still preserving enough of the user's normal setup for the session to work without repeated re-authentication.

## Why This Exists

Repo isolation is mostly about reducing accidental drift:

- It lowers the chance that a session pulls in unrelated user-scoped skills, commands, or agents that were never meant for the current repository.
- It pushes teams toward repo-local, shareable resources instead of personal one-off customizations.
- It reduces context noise. Even when skills use progressive disclosure, every skill name and description still competes for attention.
- It makes wrapper behavior more predictable when the repository wants a strong opinion about workflow, style, or tooling.

This is especially valuable in larger monorepos where a "general purpose" user profile can easily be much broader than what one package or repo actually needs.

## Four Things That Are Not the Same

Earlier versions of this document treated four concerns as one. They are separate, and each has its own contract:

| Concern | What it means | Who controls it |
|---------|---------------|-----------------|
| **Provider overlay** | Claudine-controlled storage holding one provider's config and resource view for one launch. It lives in a launch root under `~/.claudine/overlays/<provider>/`, but that location does not make it a home directory. | Claudine, through a provider-owned selector |
| **User-home identity** | `HOME` on Unix; `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH`, and a caller-supplied `HOME` on native Windows. | The user. Claudine passes it through unchanged. |
| **Provider authentication preservation** | The provider's own login state (credential files, keychain entries, subscription state) still works inside the overlay. | The overlay's mirror rules and per-provider state pins |
| **Environment credential admission** | Whether an API-key variable such as `OPENAI_API_KEY` reaches the child at all. | The sensitive-environment filter, `--include`, and per-provider auto-allows |

A missing API-key variable and a provider that cannot find its config are different failures, and Claudine reports them differently. Neither is evidence that the user's credentials are invalid.

## What `--repo` Actually Does Today

When `--repo` is enabled on a wrapper command, Claudine does all of the following:

1. It sanitizes the child environment and removes env vars whose names look sensitive unless they were explicitly or automatically allowed.
2. It plans a provider overlay. If the provider has no verified provider-owned mechanism for repo-resource isolation, the launch stops here with `provider.overlay_unsupported`, before anything is spawned.
3. It builds the overlay in a new launch root under `~/.claudine/overlays/<provider>/` from the provider's source root: the directory the same launch would use without Claudine, or the directory the user named through the provider's own selector (for example an ambient `CODEX_HOME`). An absolute `CLAUDINE_OVERLAY_DIR` replaces `~/.claudine/overlays` as the parent of the launch roots; it moves only overlay storage, and a relative value refuses the launch. It exists for disposable homes on native Windows, where the home resolves through the known-folder profile and ignores `USERPROFILE`.
4. It mirrors the source root entry by entry. Stable entries become symbolic links on Unix and recursive copies on native Windows. The provider-specific resource classes in the table below are omitted. Live state (SQLite databases and their sidecars, lock files, sockets) is never linked or copied.
5. It points the provider at the overlay through that provider's own selector, such as `CLAUDE_CONFIG_DIR` or `CODEX_HOME`. **Home variables are not changed.** Git, `gpg`, `gh`, SSH, and package managers started by the provider see the same home the user's terminal has.
6. It resolves the git root when possible and runs the child from that repo root rather than from an arbitrary nested directory.
7. It injects wrapper metadata such as `AGENT`, `YOLO`, `INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`, `CLAUDINE_PID`, and, when monorepo detection succeeds, `PACKAGE_AREA` and `PACKAGE`.

The provider's selector is inherited by everything the provider launches. A same-provider CLI started as a tool inside the session therefore sees the same overlay; no other tool is affected.

### One Launch, One Overlay Root

Every launch builds its overlay in a directory no other launch uses, named from the process id, the clock, and a counter. That is what makes the overlay exactly the current plan:

- Nothing an earlier launch placed there can reappear. A launch without `--repo` that mirrored `skills`, or an entry the user has since deleted from the source root, is simply absent from the next launch's root.
- Concurrent launches do not interfere. Two sessions in different repositories, or with different MCP server sets, each read their own prompts and configuration, and one ending does not remove what the other is reading.
- A same-provider retry or resume inside one invocation reuses that invocation's root; a retry that moves to another provider builds its own. The session-compatibility key describes the configuration view, not the root's name, so a new root alone never makes a resume incompatible.

The root is removed when the launch ends, after the guarded write-back described below, unless that write-back failed. Claudine holds an OS file lock on a sibling `<root>.lock` for the launch's lifetime; a root whose lock file exists and is free when the next launch starts (the process crashed or was force-exited before cleaning up) is removed then. A root without a lock file, or with a `<root>.retained` marker, is never removed by a sweep. Removal never follows the Unix mirror's links into the user's source root.

What persists follows from that lifetime:

- **Through a link (Unix).** A mirrored entry is a link, so a write into an existing file or a linked directory (history, session transcripts) reaches the user's source root as it would without Claudine.
- **Guarded write-back.** When the launch ends, each top-level *file* that was mirrored from the source root (plus Claude's `~/.claude.json`) is compared with its source. If the provider replaced it — renamed a new file over the Unix link, or rewrote the native-Windows copy — and the bytes differ, Claudine atomically replaces the source with the overlay's version and keeps the source's permissions. This is what keeps a rotated OAuth token (`auth.json`, `.credentials.json`, `oauth_creds.json`) valid for the next launch. A symbolic link in the source root is followed, so a dotfiles link stays a link.
- **Refused.** If the source's length or modification time changed during the launch — another session wrote it — the overlay's version is discarded and a warning names the provider and the file, never its contents.
- **Never written back.** New entries the provider created, removals, `--repo`-excluded and materialized entries (Codex `prompts`, Gemini's private MCP OAuth and enablement sidecars), the MCP configuration Claudine injected, live state, and anything inside a directory. On native Windows a change deeper inside a copied directory (a session transcript) therefore ends with the launch.
- **Write-back failed.** If a changed file cannot be written to its source — a permission error, a Windows sharing violation, a full disk — or its overlay entry, source, or source metadata cannot even be read (anything but the entry being gone), the overlay copy is the only copy of that state, so the launch root is kept rather than removed. Claudine prints a notice on stderr naming the provider, the file, the error, the kept copy's path, and the source path (never the contents). While still holding the lock it renames the lock file to a sibling `<root>.retained` marker — a rename needs no free space, so this holds on a full disk — and then writes the notice into it. If the rename fails the lock file is removed instead, and failing that a fresh marker is created; any of the three keeps later sweeps away. Copy the kept file over its source, then delete the root and its marker, if any. If the notice cannot be saved in the marker, the stderr notice says the root is still protected; only if all three steps fail does it warn that a later launch may remove the root.
- **After a crash.** A root reclaimed by the next launch's sweep is removed without write-back: after an arbitrary delay the source-unchanged check cannot be trusted. A token rotated during a crashed launch is lost.

Codex SQLite state is unaffected because it never lives in the overlay.

Storage left by earlier Claudine versions directly under `~/.claudine/<agent-offset>` (and `~/.claudine/.gemini`) is not read, changed, or removed. It is safe to delete by hand once nothing depends on it.

### When the Overlay Cannot Be Built

There is no fallback launch. Any failure is a typed diagnostic raised before the provider is spawned:

- `provider.overlay_unsupported` names the provider, the activation reason, and a next action (for `--repo`, "run without `--repo`").
- `provider.overlay_failed` names the provider, the reason, and the stage that failed, such as `source_root`, `storage_root`, `materialization`, or `mcp_injection`.

A provider source root that does not exist is handled by where it came from:

- **Default root missing** (the provider has never run, so there is no `~/.codex`): Claudine builds an empty overlay and the launch proceeds. The user's root is not created.
- **Explicit root missing** (for example `CLAUDE_CONFIG_DIR` names a directory that does not exist): the launch stops with `provider.overlay_failed` at `source_root`, because continuing would silently drop the configuration the user asked for.
- **Root is a file**: the launch stops with `provider.overlay_failed` at `source_root`.

## What Is Actually Masked

Masking is provider-specific. `--repo` does not mean "hide user skills, commands, agents, and MCP everywhere" in a universal sense.

| Provider | Selector (shape) | Source root | User-scope content excluded from the overlay |
|----------|------------------|-------------|----------------------------------------------|
| Claude | `CLAUDE_CONFIG_DIR` (the config dir) | `~/.claude` | `skills`, `commands`, `agents`, `hooks` |
| Codex | `CODEX_HOME` (the config dir) | `~/.codex` | `skills`, `agents`, `prompts` |
| Gemini | `GEMINI_CLI_HOME` (parent of `.gemini`) | `~/.gemini` | `skills`, `agents` |
| Kimi | `KIMI_CODE_HOME` (the config dir) | `~/.kimi-code` | `skills`, `agents` |
| Pi | `PI_CODING_AGENT_DIR` (the config dir) | `~/.pi/agent` | `skills`, `commands`, `agents`, `hooks` |
| Qwen | `QWEN_HOME` (the config dir) | `~/.qwen` | `skills`, `commands` |

For a parent-shaped selector the selector names the parent and the provider appends its own directory: Gemini gets `GEMINI_CLI_HOME=<launch root>` and reads `<launch root>/.gemini`.

Two consequences follow from that table:

- Some providers are fully masked only for a subset of resource types.
- Scripts are not masked by the current exclusion rules.

So, for example:

- Gemini repo isolation currently hides user skills and agents, but user `.gemini/commands` remain visible.
- Qwen repo isolation currently hides user skills and commands, but user agents remain visible.

### Providers That Refuse `--repo`

These providers stop with `provider.overlay_unsupported` before spawning. The same command without `--repo` works unchanged.

| Provider | Why there is no verified mechanism |
|----------|------------------------------------|
| Antigravity | Its only redirection surface is `HOME` itself, which also moves authentication and the keyring lookup. |
| Goose | `GOOSE_PATH_ROOT` is a real selector, but it collapses three separate per-OS trees (config, data, state), so there is no single source root to build an overlay from. |
| OpenCode | `OPENCODE_CONFIG_DIR` is additive: the user's `~/.config/opencode` still contributes agents, commands, and skills. |
| Kilo | `KILO_CONFIG_DIR` is additive in the same way. |

Before this contract, Goose and OpenCode appeared to support `--repo`. The launches ran, but the old mechanism either pointed Goose at an empty tree or left OpenCode's user resources visible. A visible refusal replaces a launch that looked isolated and was not.

The evidence for every verdict is in `claudine/fixes/2026-09-12-shadow-home/audit.md`; the verdicts themselves are generated provider metadata (see [Provider Metadata → Provider Overlay](./provider-metadata.md#provider-overlay)).

## Codex Special Case

Codex has two extra behaviors beyond the generic overlay masking.

Codex custom prompts are user-scoped rather than repo-scoped in the same way as other resource types, so Claudine materializes a prompt directory inside the overlay:

- user prompts come from `<source root>/prompts`
- repo prompts come from `<repo>/.codex/prompts` when present
- if `<repo>/.codex/prompts` is absent, Claudine also accepts `<repo>/.claude/commands`

Repo prompts win on a name collision. When `--repo` is active, user prompt files are excluded and only the repo-side prompt source is materialized. A repository with prompts gets this Codex overlay even without `--repo`, with the user's prompts kept.

Codex SQLite state is not part of that overlay. Claudine sets
`CODEX_SQLITE_HOME` to the directory Codex would have used without the
overlay, preserving an explicit `CODEX_SQLITE_HOME` or `CODEX_HOME` when
present. It resolves that value from the launch-time environment, never from
the child environment that already carries the overlay's `CODEX_HOME`. Codex's
configured `sqlite_home` retains its native higher precedence. This keeps plain
and wrapped Codex sessions in one state database and ensures the database, WAL,
and shared-memory files are opened through one directory. Claudine never copies
or links those live files into the overlay.

Databases left under `~/.claudine/.codex` by older Claudine versions are
preserved as recoverable legacy state but are no longer opened: a launch never
builds its overlay there.

## Claude Special Case

Setting `CLAUDE_CONFIG_DIR` has two side effects that an overlay must undo:

- Claude reads `.claude.json` from the selected directory instead of the home root. For a default-rooted overlay, Claudine places the user's `~/.claude.json` inside the overlay. An explicit root already contains its own.
- Claude derives its credential-store name from the selected directory, so the overlay alone would look signed out. Claudine sets `CLAUDE_SECURESTORAGE_CONFIG_DIR` to the pre-overlay store: the user's explicit value, else an explicit `CLAUDE_CONFIG_DIR`, else empty, which selects the default entry.

`CLAUDE_SECURESTORAGE_CONFIG_DIR` was observed in Claude Code 2.1.273 and is not publicly documented.

## What We Intentionally Preserve

The current implementation tries hard to preserve the parts of a user's setup that are necessary for a working session:

- the user-home identity, for the provider and every tool it starts
- provider authentication files and session state
- provider settings files
- color/theme preferences
- normal non-sensitive environment variables
- provider-required API-key env vars that Claudine auto-allows for some providers
- the agent's normal filesystem access

That last point matters: `--repo` is not a sandbox. It does not stop the wrapped agent from reading or writing outside the repository if the underlying provider allows it. It only changes what provider-level resources and config paths are visible by default.

Provider-required env vars that are automatically preserved today include:

- Codex: `OPENAI_API_KEY`, `CODEX_API_KEY`
- Gemini: `GEMINI_API_KEY`, `GOOGLE_API_KEY`
- Kimi: `KIMI_API_KEY`
- Qwen: `DASHSCOPE_API_KEY`, `QWEN_API_KEY`

Everything else still follows the normal sensitive-name filter unless the user passes `--include <ENV_NAME>`.

## MCP Is Separate

Repo isolation and MCP composition are related, but they are not the same feature.

`--repo` by itself does not currently promise that user-configured MCP servers disappear. The overlay preserves most provider config files, and native MCP configuration can remain visible unless a provider-specific runtime injector replaces that configuration for the session.

If you also use Claudine-managed MCP mode:

- `--mcp` composes a session from Claudine's catalog
- repo defaults in `<repo>/.claudine/mcp.json` replace user defaults
- `--use` appends explicit server IDs or aliases

That is the mechanism that gives Claudine an actual repo-scoped MCP session story. Repo isolation alone should be understood as "resource-path masking plus preserved auth", not "MCP wipeout."

## Technical Approach to Masking

The current masking strategy is intentionally pragmatic:

1. Preserve the user's real provider config root as the source of truth for auth and settings.
2. Build a provider overlay that mirrors most of that root.
3. Omit only the directories that would reintroduce user-scoped repo resources.
4. Materialize repo-scoped replacements only where the provider needs help seeing them.
5. Point only the provider at the overlay, through its own selector, and launch it with the sanitized environment and the user's home unchanged.

This gives Claudine a low-friction isolation model:

- authentication usually keeps working
- provider startup behavior stays close to normal
- nested tools keep the user's identity and credential context
- repo-local resources remain available through the provider's native lookup rules
- the wrapper does not need to fully reimplement each provider's config system

The tradeoff is that isolation is only as strong as the provider-specific exclusion list and overlay logic. Today that is good enough for focused sessions, but it is not yet a complete "everything user-scoped is hidden" guarantee.

## Preserved Authentication Is King

This is the design principle that matters most.

If `--repo` forced a clean-room session every time, users would have to re-authenticate constantly, providers that store subscription state outside of simple API keys would break, and the feature would be too expensive to use in normal workflows.

So Claudine deliberately prefers:

- preserving auth
- preserving settings
- masking only the resource classes that cause repo drift

That is why the implementation redirects one provider's config root instead of replacing the process-wide home. An earlier version set `HOME` to the overlay's parent, which every nested tool inherited: `gh` lost its keyring account and `git verify-commit` could not find public keys. A provider without a safe provider-owned selector is refused rather than launched that way.

## Agent Exceptions

Some provider-specific nuances are worth calling out explicitly:

- Claude's exclusions line up with the resource types Claudine wants to isolate, but it needs the two state pins described above.
- Codex is also strong, but prompt isolation is handled through a dedicated overlay path rather than through repo-scoped command directories.
- Gemini and Qwen are only partially isolated today because some user resource directories are still preserved in the overlay.
- Kimi does not have user command masking because its command story differs from markdown slash-command directories.
- Antigravity, Goose, OpenCode, and Kilo refuse `--repo`; see [Providers That Refuse `--repo`](#providers-that-refuse---repo).

## Non-Goals

`--repo` does not currently try to do any of the following:

- restrict filesystem access to the repository
- remove every user preference or provider setting
- fully hide native MCP config without `--mcp`
- normalize all providers to an identical isolation contract

If we want those properties, they should be documented and implemented as stronger features rather than implied by `--repo`.
