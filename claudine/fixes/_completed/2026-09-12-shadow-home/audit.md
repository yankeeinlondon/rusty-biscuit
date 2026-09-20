---
created: 2026-09-15
phase: 1
area: claudine
spec: ./spec.md
plan: ./plan.md
status: complete
providers_audited: 10
reasons_audited: 3
rows: 30
---

# Phase 1 Audit — Provider Overlay Capability Ground Truth

Establishes, per (provider, activation reason), whether Claudine can redirect
provider configuration through a **verified provider-owned selector** instead of
replacing the child's `HOME`. Every verdict below is backed by named evidence;
no verdict is inferred from a variable's name.

The single highest-risk unknown this audit exists to close is the **path shape**
of each selector: whether the variable names the provider's own config directory
or a *parent* under which the provider creates that directory. Getting it
backwards produces a silently ignored overlay and a provider that reads the
user's real config.

That hazard is not hypothetical. It is **live in production today** — see
[Finding F1](#f1-runtime-mcp-injection-writes-to-a-doubly-nested-path-and-is-a-no-op).

> **Amended by Phase 2 (2026-09-15).** Phase 1 left one verdict open: the three
> `‡` rows could not be materialized without a source-root fact
> ([D3](#d3-source-root-is-a-new-provider-owned-fact-not-agent_offset)). Phase 2
> landed the fact; Kimi and Pi cleared, Goose did not and is now `Unsupported`
> for `repo_resources`. The [refusal list](#pre-spawn-refusal-list) is four pairs,
> not three — it is again fixed, and Phase 6 and Phase 10 test that set.

## Method

1. Read the evidence already in the repo before probing: `docs/providers/facts/*.yaml`,
   `docs/research/{mcp,system-prompt,agent-cli,subagents,plugins,agent-logging,
   non-interactive-sessions,skills,resume,agent-permissions}/*.md`, and the
   current `original_home()` / `codex_sqlite_home()` special cases in
   `cli/src/commands/wrap/repo_home.rs:34-42,219-238`.
2. Probe only the genuine gaps.
3. For shape, require a statement that names the *config file path relative to
   the selector value*. A statement that a variable "relocates state" is not
   sufficient to establish shape.

Evidence confidence is labeled per row:

| Label | Meaning |
|---|---|
| `observed` | Reproduced on this host against the real Claudine binary |
| `documented` | Provider documentation or Claudine research recording provider docs |
| `source` | Provider source code as recorded in Claudine research |

## Findings

### F1. Runtime MCP injection writes to a doubly nested path and is a no-op

**Confidence: `observed`.** Reproduced 2026-09-15 on macOS with the repo's
`target/debug/claudine`, a scratch `HOME`, and fake `codex` / `gemini` binaries
on `PATH`. Claudine's own pre-flight MCP report printed:

```
• servers=probe
• files=<scratch-home>/.claudine/.codex/.codex/config.toml
```

```
• servers=probe
• files=<scratch-home>/.claudine/.gemini/.gemini/settings.json
```

The code path is unambiguous:

1. `repo_home.rs:25` — the overlay storage root is `~/.claudine/<agent_offset>`,
   e.g. `~/.claudine/.codex`.
2. `repo_home.rs:322` / `env/mod.rs:314` — that value, **the agent directory
   itself**, is what lands on `EnvPlan::shadow_home_path`.
3. `repo_home.rs:309` — the child's `HOME` is set to the storage root's
   *parent*, `~/.claudine`.
4. `wrapper_mcp.rs:204` passes `shadow_home_path` to `McpInjector::inject`, and
   `inject.rs:155` / `inject.rs:293` join the agent offset **again**:
   `home.join(".codex")` / `home.join(".gemini")`.

So the injected config is written one level below where the provider looks. With
`HOME=~/.claudine`, Codex reads `~/.claudine/.codex/config.toml` — which
`sync_shadow_home` has symlinked to the user's *real* `~/.codex/config.toml`.
The injected servers are never seen, and the provider silently runs on the
user's unmodified configuration.

The existing unit tests do not catch this because they call `inject` directly
with a `TempDir` root, encoding the *intended* contract
(`inject.rs:562-580,601-625`) rather than the value production passes. No L1
test drives runtime MCP injection end-to-end through the wrapper.

**Consequence for the plan.** Phase 7's signature change (`shadow_home:
Option<&Path>` → `config_root: Option<&Path>`, with the value supplied by
`OverlaySelector::provider_visible_root`) is the fix, not merely a rename.
Phase 10 tests (5) and (6) must assert the on-disk config path end to end, and
they will fail against today's behavior — making them non-vacuous by
construction.

> **Resolved in Phase 7 (2026-09-16).** `McpInjector::inject` takes
> `config_root` (the plan's provider-visible root) and the Codex and Gemini
> injectors no longer join the agent offset. L1
> `level1_provider_overlay_home::{codex_mcp_injects_servers_into_the_config_codex_home_names,
> gemini_mcp_injects_servers_under_the_gemini_cli_home_root,
> compose_gemini_mcp_injects_servers_under_the_gemini_cli_home_root}` failed
> against the doubly nested path and pass after the fix.

### F2. Two selectors, two shapes — and the existing storage layout already fits both

`CODEX_HOME` is `ProviderDir`: `$CODEX_HOME/config.toml`, default
`~/.codex/config.toml`. `GEMINI_CLI_HOME` is `ParentOfProviderDir`: the CLI
"creates or uses a `.gemini` folder inside it".

Applied to the **existing** storage root `~/.claudine/<agent_offset>`, this
means:

| Provider | Storage root (unchanged) | Selector value | Provider-visible config dir |
|---|---|---|---|
| Codex | `~/.claudine/.codex` | `CODEX_HOME=~/.claudine/.codex` | `~/.claudine/.codex` |
| Gemini | `~/.claudine/.gemini` | `GEMINI_CLI_HOME=~/.claudine` | `~/.claudine/.gemini` |

Both resolve to the directory Claudine already materializes. **No on-disk
migration is needed for either provider**, satisfying spec → Documentation and
Migration. The shape distinction shows up only in the selector's *value*, which
is exactly the one place Phase 4's `provider_visible_root(storage_root)` is
meant to express it.

### F3. `agent_offset` is not the provider's source root for five providers

`RepoHomeManager::original_home()` (`repo_home.rs:34-42`) assumes the provider's
real config lives at `~/<agent_offset>`. That is true for Claude, Codex, and
Gemini. It is **false** for five of the remaining seven:

| Provider | `agent_offset` | Actual user config root | Evidence |
|---|---|---|---|
| Goose | `.goose` | `~/Library/Application Support/Block/goose/` (macOS), `~/.config/goose/` (Linux), `%APPDATA%\Block\goose\config\` (Windows) | `docs/research/agent-logging/goose.md:197-205`; `docs/research/skills/goose.md:492-500` |
| OpenCode | `.opencode` | `~/.config/opencode/` | `docs/research/subagents/opencode.md:269-271` |
| Kilo | `.kilo` | `~/.config/kilo/` (all OSes, incl. `%USERPROFILE%\.config\kilo\`) | `docs/research/mcp/kilo.md:35-58` |
| Kimi | `.kimi` | `~/.kimi-code/` (current runtime; `~/.kimi/` is the retired `kimi-cli`) | `docs/research/mcp/kimi.md:37-47,117-122,364` |
| Pi | `.pi` | `~/.pi/agent/` | `docs/research/mcp/pi.md:334-339`; `docs/research/system-prompt/pi.md:529` |
| Antigravity | `.agents` | `~/.gemini/` | `docs/research/mcp/antigravity.md:137,201,263` |

For these providers, today's `--repo` overlay mirrors a directory that is not
the provider's config root, and the `HOME` move points the provider at an
*empty* relocated root. The launch is not "isolated"; it is **blank**. Phase 4's
source-root resolution must therefore be a provider-owned fact, not
`home.join(agent_offset)`.

`agent_offset` remains correct for its *other* consumers (resource linking); it
must not be repurposed as the overlay source root. This is recorded as
[Decision D3](#d3-source-root-is-a-new-provider-owned-fact-not-agent_offset).

### F4. Additive config directories cannot claim `repo_resources`

`OPENCODE_CONFIG_DIR` and `KILO_CONFIG_DIR` are **additive**: each names an
extra directory "loaded after global config and `.opencode`/`.kilo`
directories, so it can override their settings". Overriding is not masking — the
user's `~/.config/{opencode,kilo}/` tree still contributes agents, commands,
skills, and plugins.

No environment variable closes the remaining discovery paths. The available
`OPENCODE_DISABLE_*` / `KILO_DISABLE_*` family covers default plugins,
autoupdate, Claude-Code compatibility, external skills, and (Kilo only)
*project* config — never the global user config directory.

Per spec → "an additive config directory cannot claim repo-only isolation until
the remaining user discovery paths are explicitly disabled or filtered", both
providers' `repo_resources` verdict is **`Unsupported`**.

### F5. Codex `CODEX_SQLITE_HOME` resolution is unaffected by the shape finding

`codex_sqlite_home()` (`repo_home.rs:219-238`) resolves `CODEX_SQLITE_HOME` →
`CODEX_HOME` → `~/.codex` from the **ambient process environment**, which
Claudine never mutates in-process. Because `CODEX_HOME` is `ProviderDir`, the
chain's second rung reads the user's real Codex directory — the correct
pre-overlay value — and stays correct once Claudine begins *writing*
`CODEX_HOME` into the child environment.

There is one new constraint the current code does not state: after Phase 4,
`CODEX_HOME` becomes a key Claudine sets on the child. `codex_sqlite_home()`
must therefore resolve from the Phase 3 `EnvBaseline`, never from the assembled
child environment, or it would recurse the SQLite directory into the overlay and
breach Invariant 6. Recorded as [Decision D4](#d4-codex_sqlite_home-resolves-from-the-baseline).

### F6. `~/.claude.json` does not follow `CLAUDE_CONFIG_DIR`

`CLAUDE_CONFIG_DIR` "relocates the default `~/.claude` tree"
(`docs/research/agent-cli/claude.md:832-846,1135`). `~/.claude.json` — mutable
install/projects/OAuth-account/usage state — is a *sibling* at the home root
(`docs/research/agent-cli/claude.md:758-768,1111`) and is not described as
moving with it.

Today Claudine compensates by linking it into the shadow home
(`repo_home_root_files: &[".claude.json"]`, `claude/data.rs:295`, consumed by
`materialize_root_level_state`, `repo_home.rs:344-361`). Once `HOME` stops
moving, Claude reads the user's real `~/.claude.json` directly — which is both
simpler and a strict improvement under Invariant 6, since that file is mutable
state that should never have been linked.

`repo_home_root_files` therefore becomes dead for Claude, the only provider
that populates it. Phases 5/6 should confirm this rather than port the
mechanism forward. Flagged for the phase that owns the materialization rewrite;
not acted on here.

> **Corrected in Phase 6 — the premise above is wrong.** The shipped Claude
> Code bundle (2.1.273, read-only `strings` inspection) resolves the state file
> as `join(process.env.CLAUDE_CONFIG_DIR || homedir(), ".claude.json")`, so it
> *does* follow the selector. Setting `CLAUDE_CONFIG_DIR` also renames the
> credential store: the macOS keychain service becomes
> `Claude Code-credentials-<sha256(dir)[..8]>` and the plaintext credentials
> directory moves with it, unless `CLAUDE_SECURESTORAGE_CONFIG_DIR` is set (an
> empty value selects the default, unsuffixed entry and `~/.claude`).
>
> Consequences, as implemented: `repo_home_root_files` is **not** dead — a
> default-rooted Claude overlay places the user's `~/.claude.json` inside the
> provider-visible root (`provider_overlay::materialize_root_level_state`), and
> the Claude profile's `overlay_strategy` pins
> `CLAUDE_SECURESTORAGE_CONFIG_DIR` to the pre-overlay store. The variable is
> undocumented; see the Phase 6 `human_review_items`.

## Verdict Matrix

Vocabulary per plan → Naming Decisions:
`NativeRoot` (filesystem overlay via a provider-owned root selector),
`ComposableInjection` (reason satisfied with **no** filesystem overlay),
`Unsupported` (refuse the reason before spawn).

| Provider | `repo_resources` | `repo_prompt` | `mcp` |
|---|---|---|---|
| Claude | `NativeRoot` | `Unsupported` † | `Unsupported` † |
| Codex | `NativeRoot` | `NativeRoot` | `NativeRoot` |
| Gemini | `NativeRoot` | `Unsupported` † | `NativeRoot` |
| Goose | `Unsupported` ‡ | `Unsupported` † | `Unsupported` † |
| Kilo | `Unsupported` | `Unsupported` † | `ComposableInjection` |
| Kimi | `NativeRoot` | `Unsupported` † | `Unsupported` † |
| OpenCode | `Unsupported` | `Unsupported` † | `ComposableInjection` |
| Pi | `NativeRoot` | `Unsupported` † | `Unsupported` † |
| Qwen | `NativeRoot` | `Unsupported` † | `Unsupported` † |
| Antigravity | `Unsupported` | `Unsupported` † | `Unsupported` † |

† **Inert verdict.** The reason is never raised for this provider today, so the
verdict changes no behavior. See [Decision D1](#d1-a-reason-is-raised-only-when-the-launch-actually-needs-a-config-root)
and the [refusal list](#pre-spawn-refusal-list).

‡ **Selector verified, source root unavailable.** `GOOSE_PATH_ROOT` is a
verified, exclusive override, but Goose has no single pre-overlay root to build
an overlay *from* — the three trees it collapses are separate directories that
differ per OS. Resolved in Phase 2; see
[D3](#d3-source-root-is-a-new-provider-owned-fact-not-agent_offset).

Kimi and Pi carried the same conditional marker at Phase 1 and cleared it:
Phase 2 landed the `source_root` fact and both have a single, home-relative
root (`~/.kimi-code`, `~/.pi/agent`).

## Full Matrix — 30 Rows

Columns per plan → Phase 1 task 1. "Classes" abbreviates the relocated resource
classes: `cfg` (config/settings), `auth` (credentials/tokens), `sess`
(sessions/history), `cache`, `state` (databases/journals).

### Claude

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `CLAUDE_CONFIG_DIR` | `ProviderDir` | cfg, sess, cache, plugins (not `~/.claude.json`; macOS Keychain auth is outside it) | Exclusive — "relocates the default `~/.claude` tree" | `NativeRoot` | `docs/research/agent-cli/claude.md:832-846,1116,1135,991`; `docs/research/resume/claude.md:184,414`; `docs/research/agent-logging/claude.md:234` |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only: `repo_home.rs:247,367-374`, `materialize_repo_scoped_resources` handles only `Provider::Codex` (`repo_home.rs:332`) |
| `mcp` | — | `None` | — | — | `Unsupported` † | No Claudine injector (`inject.rs:671`, `injector_for_provider(Claude).is_none()`); wrapper routes to `claudine mcp export`. Claude's own `--mcp-config` is a per-invocation flag needing no root (`docs/research/mcp/claude.md` → `runtime_injection`) |

Note: `CLAUDE_CONFIG_DIR` does not relocate macOS Keychain credentials
(`agent-cli/claude.md:991`). That is acceptable — the overlay must *preserve*
provider authentication (Invariant 4), and leaving Keychain lookup on the real
user identity is precisely the outcome this fix wants.

### Codex

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `CODEX_HOME` | `ProviderDir` | cfg, auth, sess, cache, state (state overridden separately by `sqlite_home`/`CODEX_SQLITE_HOME`) | Exclusive — `$CODEX_HOME/config.toml` replaces `~/.codex/config.toml` | `NativeRoot` | `docs/research/mcp/codex.md` → `config_files[user]`: "`$CODEX_HOME/config.toml` (default `~/.codex/config.toml`)"; `docs/research/system-prompt/codex.md:445,476,484,522`; `docs/research/subagents/codex.md:18-26,369` |
| `repo_prompt` | `CODEX_HOME` | `ProviderDir` | prompts only (`$CODEX_HOME/prompts/`) | Exclusive | `NativeRoot` | `docs/research/system-prompt/codex.md:522` (`$CODEX_HOME/prompts/*.md`); current behavior `repo_home.rs:376-394` |
| `mcp` | `CODEX_HOME` | `ProviderDir` | cfg (`[mcp_servers]` in `config.toml`) | Exclusive | `NativeRoot` | `docs/research/mcp/codex.md` → `runtime_injection`, corrected 2026-09-15 (see [Research Corrections](#research-corrections)) |

`CODEX_HOME` shape is `observed` as well as `documented`: Claudine's own
`original_home()` already treats it as the `.codex` directory
(`repo_home.rs:34-42` returns it in the same position as
`user_home.join(".codex")`), and `codex_sqlite_home()` joins nothing onto it
(`repo_home.rs:220-230`).

### Gemini

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `GEMINI_CLI_HOME` | **`ParentOfProviderDir`** | cfg, auth (OAuth), sess, cache, state, trusted-folders — "relocates all Gemini CLI state for the process" | Exclusive — the user `.gemini` directory is rooted there *instead* | `NativeRoot` | `docs/research/agent-cli/gemini.md:782-783` ("the CLI creates or uses a `.gemini` folder inside it"); `docs/research/non-interactive-sessions/gemini.md:223,229,235`; `docs/research/mcp/gemini.md:179` |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only (`repo_home.rs:247`) |
| `mcp` | `GEMINI_CLI_HOME` | **`ParentOfProviderDir`** | cfg (`.gemini/settings.json`) plus the OAuth/enablement sidecars the caller must carry | Exclusive | `NativeRoot` | `docs/research/mcp/gemini.md:243` ("a temporary directory containing generated `.gemini/settings.json`"), `:468-471`, `:551` |

**This is the row the audit exists for.** `GEMINI_CLI_HOME` is the only selector
in the fleet with `ParentOfProviderDir` shape. Two independent research
documents state the `.gemini` segment explicitly, and the sidecar copies at
`inject.rs:296-297` must land under `<GEMINI_CLI_HOME>/.gemini/`, not under
`<GEMINI_CLI_HOME>/`. This is load-bearing for Phase 10 test (6).

Limitation carried forward: because the override relocates OAuth tokens and
enablement state too, `mcp-server-enablement.json` and `mcp-oauth-tokens.json`
must be materialized into the overlay (`docs/research/mcp/gemini.md:244,551`) —
which is what Phase 4's `profile/gemini.rs` `overlay_strategy` is for.

### Goose

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `GOOSE_PATH_ROOT` | `ParentOfProviderDir` (child segments are `config/`, `data/`, `state/`, `.agents/` — **not** `.goose`) | cfg, data, state, agents, plugins, skills — "overridden entirely" | Exclusive | `Unsupported` ‡ | `docs/research/skills/goose.md:370-385,492-500`; `docs/research/agent-logging/goose.md:197`; `docs/research/subagents/goose.md:371-372`, `:437` |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only |
| `mcp` | — | `None` | — | — | `Unsupported` † | No Claudine injector (`inject.rs:674`). Goose's native runtime injection is repeatable `--with-extension` / `--with-builtin` **argv**, needing no config root (`docs/research/mcp/goose.md` → `runtime_injection`) |

Goose's shape does not fit `ProviderDir` cleanly: the selector names a root and
the provider appends a fixed internal layout, which is structurally
`ParentOfProviderDir` with a non-dot child segment. See
[D2](#d2-parentofproviderdir-carries-the-child-segment-name).

`GOOSE_PATH_ROOT` is documented as intended for "tests and CI sandboxes"
(`subagents/goose.md:372`). That is a suitability caveat, not a support
limitation; it is a supported public override with `source`-grade evidence
(`crates/goose/src/config/paths.rs`).

### Kilo

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `KILO_CONFIG_DIR` | `ProviderDir` | cfg, agents, commands, modes, skills, plugins | **Additive** — "loaded after global and `.kilo` directories, so it can override their settings" | `Unsupported` | `docs/research/subagents/kilo.md:89-97,348-353,436`; no `KILO_DISABLE_*` closes global config (`subagents/kilo.md:353-359`) |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only |
| `mcp` | `KILO_CONFIG_CONTENT` | `Inline` | cfg (`mcp` map, `local` precedence) | Additive overlay — correct for MCP | `ComposableInjection` | `docs/research/mcp/kilo.md` → `runtime_injection`; `docs/research/subagents/kilo.md:437` |

Claudine has no Kilo MCP injector wired today (`inject.rs` has Codex, Gemini,
OpenCode only). The `ComposableInjection` verdict records the **capability**,
not a claim that it is implemented. If a Kilo injector is added later it needs
no filesystem overlay — the same posture as OpenCode.

### Kimi

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `KIMI_CODE_HOME` | `ProviderDir` | cfg, auth (`credentials/`), sess, plugins, skills | Exclusive — "overrides the Kimi Code data directory (default `~/.kimi-code`)" | `NativeRoot` | `docs/research/mcp/kimi.md:117-122,249-250`; `docs/research/plugins/kimi.md:229,278-284` |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only |
| `mcp` | — | `None` | — | — | `Unsupported` † | `docs/research/mcp/kimi.md` → `runtime_injection.supported: false` ("does NOT expose `--mcp-config-file`, `--mcp-config`, or any equivalent"); no Claudine injector |

Source root is `~/.kimi-code`, not `~/.kimi` — see
[F3](#f3-agent_offset-is-not-the-providers-source-root-for-five-providers).
`KIMI_SHARE_DIR` is the *legacy* `kimi-cli` variable and is not the selector
(`docs/research/mcp/kimi.md:370`).

### OpenCode

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `OPENCODE_CONFIG_DIR` | `ProviderDir` | cfg, agents, commands, modes, plugins | **Additive** — "loaded after the global config and after `.opencode` directories, so it can override their settings" | `Unsupported` | `docs/research/mcp/opencode.md:222-239,388-391`; `docs/research/subagents/opencode.md:53-61,269,448-449`; no `OPENCODE_DISABLE_*` closes global config (`subagents/opencode.md:215-221`) |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only |
| `mcp` | `OPENCODE_CONFIG_CONTENT` | `Inline` | cfg (`mcp` object) | Additive overlay — correct for MCP | `ComposableInjection` | `docs/research/mcp/opencode.md` → `runtime_injection`; implemented at `inject.rs:50-63` (already ignores the root parameter) |

This is the pair the spec calls out by name. OpenCode's `mcp` support is
unchanged and must acquire **no** storage root and **no** home override
(spec → Design; plan → Phase 4). Phase 10 test (7) asserts the storage root
never appears on disk.

### Pi

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `PI_CODING_AGENT_DIR` | `ProviderDir` | cfg (`settings.json`, `models.json`), auth (`auth.json`), trust, extensions, skills, prompts, sessions | Exclusive — "changes the global agent directory from `~/.pi/agent` to a wrapper-chosen directory" | `NativeRoot` | `docs/research/system-prompt/pi.md:529,623`; `docs/research/model-config/pi.md:157,349`; `docs/research/resume/pi.md:382`; `docs/research/subagents/pi.md:22,307` |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only |
| `mcp` | — | `None` | — | — | `Unsupported` † | `docs/research/mcp/pi.md` → `runtime_injection.supported: false` ("there is no Pi-native one-run MCP injection flag"); no Claudine injector |

Source root is `~/.pi/agent`, not `~/.pi`. `PI_CODING_AGENT_SESSION_DIR` is a
separate, narrower selector for session storage
(`docs/research/resume/pi.md:382`) — the Pi analogue of `CODEX_SQLITE_HOME`, and
the natural external-state selector should Pi ever need one.

### Qwen

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | `QWEN_HOME` | `ProviderDir` | cfg (`settings.json`), agents, commands, memory/context | Exclusive — "overrides the global Qwen config directory"; `~/.qwen/agents/` → `$QWEN_HOME/agents/` | `NativeRoot` | `docs/research/subagents/qwen.md:310-311,360,365,541`; `docs/research/system-prompt/qwen.md:478`; `docs/research/mcp/qwen.md:47` |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only |
| `mcp` | — | `None` | — | — | `Unsupported` † | `docs/research/mcp/qwen.md` → `runtime_injection.supported: false` ("no documented one-run CLI flag"); no Claudine injector |

`QWEN_RUNTIME_DIR` explicitly does **not** relocate definitions — only runtime
output (`docs/research/subagents/qwen.md:365`). It is Qwen's external-state
selector, not a config selector; do not confuse the two.

### Antigravity

| Reason | Selector | Shape | Classes relocated | Discovery | Verdict | Evidence |
|---|---|---|---|---|---|---|
| `repo_resources` | — | `None` | — | — | `Unsupported` | `docs/providers/facts/antigravity.yaml:239-241`; `docs/research/mcp/antigravity.md:201` ("No MCP-specific environment variable was verified") |
| `repo_prompt` | — | `None` | — | — | `Unsupported` † | Reason is Codex-only |
| `mcp` | — | `None` | — | — | `Unsupported` | `docs/research/mcp/antigravity.md:137,257,301`; `runtime_injection.supported: false` |

Antigravity's only redirection surface is `HOME` itself, and the research is
explicit that using it "also redirects authentication, project cache, settings,
plugin state, and OAuth stores, so it is not a safe one-run MCP injection
mechanism" (`mcp/antigravity.md:137`), and that changing `HOME` breaks keyring
auth (`antigravity.yaml:239`). That is precisely the practice this fix removes.
Confirmed as predicted by plan → Phase 1.

## Pre-Spawn Refusal List

The authoritative set for Phase 6 (implementation) and Phase 10 test (8). A pair
is on this list when the reason **can actually be raised today** and its verdict
is `Unsupported`.

| # | Provider | Reason | Trigger | Post-fix behavior |
|---|---|---|---|---|
| 1 | Antigravity | `repo_resources` | `claudine antigravity --repo` | Typed `ProviderOverlayUnsupported` before spawn |
| 2 | OpenCode | `repo_resources` | `claudine opencode --repo` | Typed `ProviderOverlayUnsupported` before spawn |
| 3 | Kilo | `repo_resources` | `claudine kilo --repo` | Typed `ProviderOverlayUnsupported` before spawn |
| 4 | Goose | `repo_resources` | `claudine goose --repo` | Typed `ProviderOverlayUnsupported` before spawn |

**That is the entire behavior tightening: four pairs, all reached only via
`--repo`.** Every other `Unsupported` cell in the matrix is marked † and is
inert — the reason is never raised for that provider, so no currently working
command begins to fail.

Row 4 was added in Phase 2 by decision [D3](#d3-source-root-is-a-new-provider-owned-fact-not-agent_offset).
It is not a working command that starts failing: per
[F3](#f3-agent_offset-is-not-the-providers-source-root-for-five-providers),
`claudine goose --repo` today mirrors `~/.goose` — which is not Goose's config
root — and points Goose at an empty relocated tree. The launch is blank, not
isolated. A visible pre-spawn refusal replaces a silent one.

In all three cases the same command **without** `--repo` continues to work
unchanged, per spec → "refuse only that mode".

For OpenCode and Kilo specifically, `--mcp` / `--use` keeps working with or
without `--repo` being refused, because their MCP path is
`ComposableInjection` and carries no overlay.

## Decisions Recorded for Later Phases

### D1. A reason is raised only when the launch actually needs a config root

`OverlayReason::Mcp` means "MCP delivery requires a Claudine-provided provider
config root". It is raised only for providers whose runtime injector needs one
(today: Codex, Gemini). Providers with an inline injector satisfy MCP with
`ComposableInjection` and no storage root. Providers with **no** runtime
injector never raise the reason at all; their existing `claudine mcp export`
guidance path is untouched.

**This refines plan → Phase 6, which says "`--mcp`/`--use` → `Mcp`"
unconditionally.** Taken literally, that would make `claudine claude --mcp`
refuse before spawn — a regression well beyond what the spec authorizes.
Phase 6 must gate the reason on the provider having a root-requiring injector.
Phase 10 should add a guard test that `claudine claude --mcp` still reaches the
export guidance.

### D2. `ParentOfProviderDir` carries the child segment name

Gemini appends `.gemini`; Goose appends `config/`, `data/`, `state/`,
`.agents/`. A bare two-variant enum cannot express Goose. Phase 2's
`OverlaySelectorShape` should therefore be:

```
ProviderDir                       // selector value IS the config dir
ParentOfProviderDir { child }     // provider appends `child`
Inline                            // env-carried config content, no directory
```

with `provider_visible_root(storage_root)` returning `storage_root` for
`ProviderDir` and `storage_root.join(child)` for `ParentOfProviderDir`. For
Gemini `child = ".gemini"`. Goose's multi-segment layout is then a `profile`
concern (`overlay_strategy`), not a shape concern — the selector value is the
root and Goose owns what it creates inside.

Recording `child` in facts also makes [F1](#f1-runtime-mcp-injection-writes-to-a-doubly-nested-path-and-is-a-no-op)
unrepresentable: the offset is stated once, in data, instead of being hard-coded
at `inject.rs:155` and `inject.rs:293`.

### D3. Source root is a new provider-owned fact, not `agent_offset`

Per [F3](#f3-agent_offset-is-not-the-providers-source-root-for-five-providers),
Phase 2's `overlay_selector` record needs the **default source root** alongside
`env_var`, `shape`, `relocates`, and `additive` — because for five providers it
is neither `~/<agent_offset>` nor derivable from the selector name. It is also
OS-dependent for Goose (three distinct roots).

The three ‡ verdicts (Goose, Kimi, Pi) are `NativeRoot` on selector evidence but
cannot be *materialized* until this fact exists. If Phase 2 lands the fact, they
work. If Phase 2 elects to defer OS-dependent source roots, Goose must be
downgraded to `Unsupported` for `repo_resources` — in which case it joins the
refusal list and Phase 10 test (8) must cover it. **This is the one decision
Phase 2 cannot make silently.**

Claude, Codex, and Gemini are unaffected: their source root is
`~/<agent_offset>` and their overlays work with no new fact.

**Resolved in Phase 2 (2026-09-15).** The fact landed, as
`overlay_selector.source_root`, typed `Option<PathTemplate>` and required to be
home-relative. Kimi and Pi keep `NativeRoot`. Goose is downgraded to
`Unsupported` for `repo_resources` and joins the
[refusal list](#pre-spawn-refusal-list) as row 4.

The downgrade is not the "deferred OS-dependent roots" branch this decision
anticipated — a per-OS `source_root` record would not have helped. Goose has no
single pre-overlay root on *any* OS: `GOOSE_PATH_ROOT` collapses three trees
(`config/`, `data/`, `state/`) that, unset, are three distinct directories
(`~/.config/goose`, `~/.local/share/goose`, `~/.local/state/goose` on Linux).
Materializing that overlay needs a Goose-specific `overlay_strategy` in the
wrapper profile, which plan → Phase 4 explicitly does not authorize ("Leave the
other eight profiles on the default"). Recording one path would have been a
statement the evidence does not support.

The constraint is enforced structurally rather than by judgment: the generated
metadata invariant `repo_resource_isolation_requires_a_single_source_root`
(`lib/src/provider/tests.rs`) fails any `NativeRoot` verdict for
`repo_resources` without a non-null `source_root`.

### D4. `codex_sqlite_home()` resolves from the baseline

Per [F5](#f5-codex-codex_sqlite_home-resolution-is-unaffected-by-the-shape-finding):
once Claudine writes `CODEX_HOME` into the child environment, the SQLite
resolution chain must read the Phase 3 `EnvBaseline`, not the assembled child
environment. Phase 5's "keep `codex_sqlite_home()` byte-for-byte" instruction
should be read as *keep the resolution semantics*; the input source becomes the
baseline. This is required by Invariant 6 and by the completed
separate-state contract.

### D5. `additive: true` and `NativeRoot` for `repo_resources` are mutually exclusive

Confirmed by [F4](#f4-additive-config-directories-cannot-claim-repo_resources) and
directly implementable as the Phase 2 generated-metadata invariant that risk R2
calls for. Both additive selectors (OpenCode, Kilo) are `Unsupported` for
`repo_resources` and `ComposableInjection` for `mcp`, so the invariant has live
positive and negative cases.

## Research Corrections

One research document contradicted the rest of the evidence and was corrected in
this phase.

| Document | Was | Now | Why |
|---|---|---|---|
| `docs/research/mcp/codex.md` → `runtime_injection.mechanism` | "Set `CODEX_HOME` to a temporary directory containing a generated `.codex/config.toml`" | "Set `CODEX_HOME` to a generated directory containing `config.toml`" | The same document's own `config_files[user]` record states `$CODEX_HOME/config.toml (default ~/.codex/config.toml)`. Four other research documents agree. The `.codex/` segment in the prose describes `ParentOfProviderDir` shape and is wrong; it is the likely origin of [F1](#f1-runtime-mcp-injection-writes-to-a-doubly-nested-path-and-is-a-no-op). |

No provider facts file needed a change in this phase: the two new keys
(`overlay_selector`, `overlay_capabilities`) are Phase 2's work, and this audit
is their input. No generated `lib/src/provider/<slug>/data.rs` was touched.

## What Would Change an `Unsupported` Verdict

Per plan → Checkpoint 1, every `Unsupported` verdict states its falsifier.

| Provider / reason | Would become supported if… |
|---|---|
| OpenCode `repo_resources` | OpenCode gains a documented variable that **disables** global user config/agents/commands/skills discovery (not merely overrides it), or `OPENCODE_CONFIG_DIR` is verified to replace rather than augment `~/.config/opencode`. Verify by launching with the variable set to an empty directory and confirming a user-scope agent is *not* listed. |
| Kilo `repo_resources` | Same falsifier against `KILO_CONFIG_DIR` and `~/.config/kilo`. Kilo's `KILO_DISABLE_PROJECT_CONFIG` is not sufficient — it closes the project layer, not the global one. |
| Goose `repo_resources` | Claudine gains a Goose `overlay_strategy` in the wrapper profile that materializes the three source trees (`config/`, `data/`, `state/`) into the single `GOOSE_PATH_ROOT` layout. The selector itself is already verified and exclusive; only the source-root side is missing. Out of scope for this fix (plan → Phase 4 leaves eight profiles on the default). |
| Antigravity `repo_resources`, `mcp` | Antigravity ships a config-root or config-file environment variable scoped to the provider. `HOME` does not qualify: `antigravity.yaml:239` records that changing it breaks keyring auth. Re-verify against a new `agy --help` and its documented environment table. |
| Claude / Goose / Kimi / Qwen / Pi `mcp` | Claudine adds a runtime MCP injector for the provider. Claude and Goose would then be `ComposableInjection` (`--mcp-config` argv; `--with-extension` argv) needing no root. Kimi, Qwen, and Pi need a provider-side one-run mechanism that does not exist today. |
| Any provider `repo_prompt` | Claudine extends repo-prompt materialization beyond Codex. This is out of scope for this fix (spec → Scope excludes broadening documented isolation classes). |

## Evidence Not Gathered, and Why

- **No provider binary was executed for selector verification.** Every shape
  finding rests on `documented` / `source`-grade research already in the repo,
  corroborated across at least two documents. Running the real CLIs would touch
  the developer's real credential stores, which the spec forbids
  (Invariant 8, spec → Testing).
- **The one empirical probe run** ([F1](#f1-runtime-mcp-injection-writes-to-a-doubly-nested-path-and-is-a-no-op))
  used a scratch `HOME`, fake provider stubs on `PATH`, `PLAYA_DRY_RUN=1`, and a
  private audio spool. It read no real credentials and opened no terminal
  window.
- **No Windows or Linux probing.** Nothing in this phase is OS-conditional: the
  selector names and shapes are identical across the three platforms in every
  research document consulted. The OS-specific risk in this fix is
  materialization (Phase 5) and evidence collection (Phase 11), not capability.
  The one OS-dependent *fact* discovered — Goose's three per-OS source roots
  ([F3](#f3-agent_offset-is-not-the-providers-source-root-for-five-providers)) —
  is recorded from research and flagged for Phase 2 in
  [D3](#d3-source-root-is-a-new-provider-owned-fact-not-agent_offset).
