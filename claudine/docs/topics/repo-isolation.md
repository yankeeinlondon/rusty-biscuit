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

## What `--repo` Actually Does Today

When `--repo` is enabled on a wrapper command, Claudine currently does all of the following:

1. It sanitizes the child environment and removes env vars whose names look sensitive unless they were explicitly or automatically allowed.
2. It creates a shadow home under `~/.claudine/<agent-offset>` for the wrapped provider.
3. It symlinks or copies most of the provider's normal home-directory content into that shadow home.
4. It excludes a provider-specific set of resource directories from that shadow home so the wrapped agent cannot see those user-scoped resources through its normal config path.
5. It sets `HOME=~/.claudine` so the wrapped provider resolves its config from the shadow tree instead of the real user home.
6. It resolves the git root when possible and runs the child from that repo root rather than from an arbitrary nested directory.
7. It injects wrapper metadata such as `AGENT`, `YOLO`, `INTERACTIVE`, `AGENT_PARAMS`, `CLAUDINE_SESSION_ID`, `CLAUDINE_PID`, and, when monorepo detection succeeds, `PACKAGE_AREA` and `PACKAGE`.

If shadow-home creation fails, Claudine currently falls back to `HOME=/dev/null`. That keeps the session launch moving, but it is a degraded path and can break authentication or provider startup.

## What Is Actually Masked

The important detail is that masking is provider-specific today. `--repo` does not yet mean "hide user skills, commands, agents, and MCP everywhere" in a universal sense.

Current shadow-home exclusions:

| Provider | User-scope content excluded from shadow home |
|----------|----------------------------------------------|
| Claude | `skills`, `commands`, `agents`, `hooks` |
| Codex | `skills`, `agents`, `prompts` |
| Gemini | `skills`, `agents` |
| Goose | `skills`, `agents` |
| Kimi | `skills`, `agents` |
| OpenCode | `skills` |
| Qwen | `skills`, `commands` |

Two consequences follow from that table:

- Some providers are fully masked only for a subset of resource types.
- Scripts are not masked by the current shadow-home exclusion rules.

So, for example:

- Gemini repo isolation currently hides user skills and agents, but user `.gemini/commands` remain visible.
- OpenCode repo isolation currently hides user skills, but user commands and agents remain visible.
- Qwen repo isolation currently hides user skills and commands, but user agents remain visible.

That is the current implementation, even if the broader design goal is stricter isolation.

## Codex Special Case

Codex has one extra behavior beyond the generic shadow-home masking.

Codex custom prompts are user-scoped rather than repo-scoped in the same way as other resource types, so Claudine materializes a prompt overlay inside the shadow home:

- user prompts come from `~/.codex/prompts`
- repo prompts come from `<repo>/.codex/prompts` when present
- if `<repo>/.codex/prompts` is absent, Claudine also accepts `<repo>/.claude/commands`

When `--repo` is active, user prompt files are excluded and only the repo-side prompt source is materialized into the shadow home.

Codex SQLite state is not part of that overlay. Claudine sets
`CODEX_SQLITE_HOME` to the directory Codex would have used before `HOME` was
changed, preserving an explicit `CODEX_SQLITE_HOME` or `CODEX_HOME` when
present. Codex's configured `sqlite_home` retains its native higher precedence.
This keeps plain and wrapped Codex sessions in one state database and ensures
the database, WAL, and shared-memory files are opened through one directory.
Claudine never copies or links those live files into the shadow home.

Regular databases left under the shadow home by older Claudine versions are
preserved as recoverable legacy state but are no longer opened. Legacy database
symbolic links are removed with their sidecars because they can split SQLite's
locking paths.

## What We Intentionally Preserve

The current implementation tries hard to preserve the parts of a user's setup that are necessary for a working session:

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

`--repo` by itself does not currently promise that user-configured MCP servers disappear. In the shadow-home model, most provider config files are preserved, and native MCP configuration can remain visible unless a provider-specific runtime injector replaces that configuration for the session.

If you also use Claudine-managed MCP mode:

- `--mcp` composes a session from Claudine's catalog
- repo defaults in `<repo>/.claudine/mcp.json` replace user defaults
- `--use` appends explicit server IDs or aliases

That is the mechanism that gives Claudine an actual repo-scoped MCP session story. Repo isolation alone should be understood as "resource-path masking plus preserved auth", not "MCP wipeout."

## Technical Approach to Masking

The current masking strategy is intentionally pragmatic:

1. Preserve the user's real provider home as the source of truth for auth and settings.
2. Build a shadow home that mirrors most of that provider home.
3. Omit only the directories that would reintroduce user-scoped repo resources.
4. Materialize repo-scoped replacements only where the provider needs help seeing them.
5. Launch the child with the shadow home and sanitized environment.

This gives Claudine a low-friction isolation model:

- authentication usually keeps working
- provider startup behavior stays close to normal
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

That is why the implementation uses a shadow home instead of trying to null-route the user's entire home directory.

## Agent Exceptions

Some provider-specific nuances are worth calling out explicitly:

- Claude is the cleanest implementation today because the shadow-home exclusions line up with the resource types Claudine wants to isolate.
- Codex is also strong, but prompt isolation is handled through a dedicated overlay path rather than through repo-scoped command directories.
- Gemini, OpenCode, and Qwen are only partially isolated today because some user resource directories are still preserved in the shadow home.
- Goose and Kimi do not have user command masking because their command story differs from markdown slash-command directories.

## Non-Goals

`--repo` does not currently try to do any of the following:

- restrict filesystem access to the repository
- remove every user preference or provider setting
- fully hide native MCP config without `--mcp`
- normalize all providers to an identical isolation contract

If we want those properties, they should be documented and implemented as stronger features rather than implied by `--repo`.
