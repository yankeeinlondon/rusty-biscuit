# Agent Selection Feature

_Last updated: 2026-04-14_

When we use commands like `claudine compose`, `claudine inline-compose`, and `claudine sequence` an often overlooked feature is
that we can defer the decision on which agent to use until later. It is true that there are flags like `--claude`, `--opencode`, etc. which move away from this lazy selection of agent. Both modes have their strengths but currently the lazy mode is operating incorrectly and subsequently it is loosing a lot of it's functionality.

When a user runs a claudine command that _composes_ a prompt without a `--codex`/`--claude`/etc. flag specifying the agent to
use we want to resolve the agent through a layered resolver (see "Resolution Order" below). The resolver's shape depends on whether we are running in a TTY (interactive) context or a non-TTY context:

- In **TTY contexts**, a `--<provider>` CLI flag still wins unconditionally, but otherwise an interactive picker is _always_ shown so the user can confirm. Frontmatter `agent` and the configured `favorite_agent` do **not** resolve past the picker in TTY mode — they only influence the picker's highlighted default and its row ordering.
- In **non-TTY contexts**, the picker is not an option, so frontmatter `agent` is promoted to a resolving signal along with the configured `favorite_agent`.

The interactive selection box is implemented with the **`ChooseOne`** component from the `biscuit-tui` library (workspace crate `biscuit-tui`, lives at `biscuit-tui/lib/`), driven by `tui_chrome::run_standalone` as an inline prompt. Behaviorally it:

- shows only those agents which the host computer has installed (one `ChoiceOption` per installed provider)
- pre-selects the user's configured favorite agent (see "Favorite Agent" below) as the highlighted default in the select control, if one is set and installed
- accepts up/down arrow keys (and the standard `ChooseOne` vim `j`/`k`, hotkey, and fuzzy-filter affordances) for changing the selection
- returns `EventOutcome::Submitted` on confirm; `EventOutcome::Cancelled` (Esc) aborts the command

When an agent is selected, we will run the Agent using the "default model" for that provider's platform. The one exception is
OpenCode because -- at least currently -- do not support a "default model" in non-interactive sessions.

So for OpenCode in non-interactive sessions, the OPENCODE_MODEL (and the MODEL as fallback) environment variable will
select which model to use for these sessions.

While OpenCode _requires_ this variable to be set, other providers can also be influenced via ENV variables to choose an
explicit model instead of just the default model:

- CODEX*MODEL, OPENAI_MODEL, \_falling back to* MODEL will all set the model for the **Codex** agent
- CLAUDE*MODEL, ANTHROPIC_MODEL, \_falling back to* MODEL will set the model for the **Claude** agent
- QWEN*MODEL, \_falling back to* MODEL will set the model for the Qwen CLI
- GEMINI*MODEL, \_falling back to* MODEL will set the model for the Gemini CLI
- KIMI*MODEL, \_falling back to* MODEL will set the model for the Kimi Code CLI
- GOOSE_MODEL for the Goose CLI
- ROO_MODEL for Roo Code

## Favorite Agent

The "favorite agent" referenced throughout this spec is stored in the **Claudine user configuration** as a dedicated field (e.g. `favorite_agent: Option<Provider>`). It is not derived, not per-invocation, and not inferred from history.

How it is set:

- `claudine init` should capture the favorite agent during first-run setup
- An explicit setter command exists for later changes (e.g. `claudine config set favorite-agent <name>`)
- Standard config mechanisms (editing the config file directly) also work

Semantics when absent:

- Absence of a configured favorite is **not an error**
- The resolver simply has one fewer signal and moves on to the next step in the chain
- In the interactive picker, no pre-selected default is highlighted beyond what earlier resolver steps (frontmatter) may contribute

Wherever older language in this spec referenced the "user's favorite" informally, it now means specifically this configured field.

## Resolution Order

Agent resolution is **mode-dependent**: TTY and non-TTY contexts use different chains. The key split is that frontmatter `agent` and the configured `favorite_agent` are only _resolving_ signals in non-TTY mode; in TTY mode they are _influence-only_ signals that shape the always-shown picker.

### TTY (interactive) agent resolution, highest priority to lowest:

1. **`--<provider>` CLI flag** — e.g. `--claude`, `--codex`, `--opencode`. If present, this wins unconditionally and the picker is not shown.
2. **Interactive picker** — always shown when no `--<provider>` flag is present. The user always confirms a selection. Signals below do **not** resolve past the picker; they only inform the picker's defaults and row ordering:
    - **Singular frontmatter `agent`** (if installed) → becomes the highlighted default.
    - **List-valued frontmatter `agent`** → the listed agents that are installed are grouped at the top of the select in list order; the first installed entry is the highlighted default.
    - **Configured `favorite_agent`** (if installed and no frontmatter signal applies) → becomes the highlighted default.
    - Otherwise no pre-selection beyond the list's natural first item.
3. _(No further fallback — the picker always produces a selection. If the user cancels the picker, the command aborts.)_

### Non-TTY agent resolution, highest priority to lowest:

1. **`--<provider>` CLI flag** — wins unconditionally.
2. **Singular frontmatter `agent` value** — if the frontmatter specifies a single agent and that provider is installed on the host.
3. **List-valued frontmatter `agent`** — if the frontmatter specifies a list of agents, the first entry in list order that is installed on the host wins.
4. **Configured `favorite_agent`** — from Claudine config, if set and the referenced provider is installed.
5. **Hard error** — the run fails loudly. The error must be structured and must point the user at the available signals they can supply: the `--<provider>` flag, a frontmatter `agent` value, or setting a `favorite_agent` in config.

Model resolution follows a single parallel chain that is independent of TTY/non-TTY. Model precedence, highest to lowest:

1. **`--model <model>` CLI flag** — always wins when provided. Typically paired with an explicit `--<provider>` flag.
2. **Provider-specific ENV variable** — e.g. `CODEX_MODEL`, `CLAUDE_MODEL`, `OPENCODE_MODEL` (see the environment variable list above for the full mapping).
3. **Generic `MODEL` ENV variable** — used as fallback when no provider-specific variable is set.
4. **Frontmatter `model` value** — either a single string or a list of strings. The first entry that is a valid model for the chosen agent wins. Entries that are not valid for the chosen agent are skipped, not errored.
5. **Provider default** — the agent provider's built-in default model (with the OpenCode caveat noted above).

### Non-TTY behavior

- The interactive picker must **never** hang in non-TTY contexts. The runtime detects TTY up front and either resolves via the non-TTY chain above or fails with the structured hard error.
- This TTY-awareness applies equally to `compose`, `inline-compose`, and `sequence`.

### Host Detection

Throughout this spec, "installed on host" has a single concrete meaning: `InstalledAiClients::is_installed(provider.sniff_ai_cli())` from the `sniff` crate (`sniff::programs::InstalledAiClients`) returns `true` for the provider in question.

- This is the same detection mechanism Claudine already uses elsewhere — `claudine init`, `claudine hooks`, and the provider wrappers (see `claudine/cli/src/commands/hooks.rs`, `claudine/cli/src/commands/wrap/composition.rs`, and `claudine/cli/src/commands/wrap/mod.rs`). No new detection code is introduced by this feature.
- Detection is performed **once at command start**, not re-queried per signal during resolution. All "installed?" checks within a single invocation of `compose`, `inline-compose`, or `sequence` observe the same snapshot, so repeated references to "installed" remain consistent even if the host state changed to change mid-run.

## Frontmatter Influence

### The `agent` property

The `agent` frontmatter property has **mode-dependent** behavior. In both modes the singular-vs-list distinction still matters, but what that distinction _does_ differs:

**In TTY (interactive) contexts**, frontmatter `agent` is **influence only** — it never resolves past the picker. It shapes the picker's highlighted default and its row ordering:

- If `agent` is a **string** and that provider is installed, it becomes the initially-highlighted option in the picker. If the host does not have the suggested agent, the suggestion is ignored and the picker falls back to the next default signal (`favorite_agent`, then natural list order).
- If `agent` is a **list**, the listed agents that are installed are grouped at the top of the select list (in list order, with uninstalled entries omitted), and the first installed entry in list order is the initially-highlighted option.
- The user still confirms a selection in the picker regardless of how many frontmatter signals are present.

**In non-TTY contexts**, frontmatter `agent` is promoted to a **resolving signal** (see the non-TTY chain in "Resolution Order"):

- If `agent` is a **string** and maps to a supported, installed provider, it resolves the agent directly. If the host does not have the suggested agent, the suggestion is ignored and the resolver moves to the next step.
- If `agent` is a **list**, the resolver walks the list in order and picks the first installed entry.

### The `model` property

The `model` frontmatter property feeds step 4 of the model resolution chain.

- It accepts either a singular string value or a list of string values.
- If a suggested model is a match for a valid model of the resolved `agent`, it is used.
- Invalid models for the chosen `agent` are ignored/skipped and do not create an error condition.
    - This does mean we need an enumerated list of valid models per provider. See "Model Enumeration" below for how that list is sourced and extended.
- Stronger signals earlier in the chain (CLI `--model`, provider-specific ENV var, `MODEL` ENV var) always override frontmatter suggestions.

## Model Enumeration

The "enumerated list of valid models per provider" referenced above is its own sub-feature. It is **dynamically fetched**, with a **user override** layer in Claudine config. This section specifies _what_ the enumeration is and _where_ it comes from; implementation details (cache location, refresh cadence, storage format) are intentionally left open.

### Sourcing strategy

Model lists are sourced dynamically per provider rather than hardcoded:

- **Codex** and **Claude** — sourced from the `unchained-ai-gen` package, which already maintains provider model catalogs for the monorepo.
- **OpenCode** — sourced by shelling out to `opencode models`. This is viable because, in any context where OpenCode is a resolved provider, the OpenCode CLI is guaranteed to be installed on the host (per the "Host Detection" rule above).
- **Qwen** — sourced from `opencode models` filtered to Qwen entries (e.g., `opencode models | rg qwen`), with per-provider normalization. This is a **pragmatic starting point**, not a permanent contract; a dedicated source may replace it later.
- **Kimi** and **Gemini** — **TBD**. Research still needed. The same "dynamic fetch via best-available source" pattern applies once a source is identified. The OpenRouter model endpoint (accessed via `unchained-ai-gen`) is a candidate, but is not committed to here.
- **Goose** and **Roo** — **TBD**. Not yet researched; same note applies.

### Caching

Fetched model lists are **cached**. The cache is **refreshable**. Where the cache lives, how it is keyed, and when it auto-refreshes are implementation details and are not pinned by this spec.

### User override

Users can extend or replace the enumerated cache through the Claudine configuration. Concretely, a config section (e.g., `models.codex: [...]`) exposes per-provider entries:

- **Additive form (default)** — user-supplied entries are added to the fetched list, producing a combined enumeration. This handles drift when new models are released before a cache refresh, and also covers user-specific custom endpoints that a remote catalog would never know about.
- **Replace/override form** — a separate shape is available for users who want their list to fully replace the fetched list for that provider.

The exact config keys and shape are implementation details; the requirement here is that both modes exist.

### Use in validation

The model enumeration is what powers the "invalid models are skipped, not errored" behavior described under the `model` frontmatter property. For a given resolved agent:

- A model is **valid** if it appears in the **combined enumeration** (fetched list plus user override, per the additive/replace semantics above) for that agent's provider.
- Frontmatter `model` entries that are not valid for the resolved agent are skipped silently; the resolver continues through the list.
- This section fulfils the "enumerated list of valid models per provider" requirement flagged under the `model` property.

## Sequence UX

When Claudine is running non-interactive prompts, it's particularly important that all user-required interactivity be
front-loaded so that the user can address any required questions initially and then leave this session to execute to
completion (versus be paused to wait for HITL interaction mid-stream).

For `claudine compose` and `claudine inline-compose` this is straightforward: the resolver runs once, the picker (if needed) prompts once, and execution proceeds.

For `claudine sequence`, front-loading is more involved because different steps in the sequence commonly want different agents and/or different models, and the user must still sign off on the overall configuration before anything runs.

### Consolidated Review Screen (interactive)

Before any step executes, Claudine presents a **consolidated review screen** implemented with the **`InputTable`** component from `biscuit-tui` (see `biscuit-tui/lib/src/components/input_table/`). The table is driven via `tui_chrome::run_standalone` so it renders inline above the eventual execution output.

Column layout (per row, one row per step):

| Column   | `InputTableColumn` variant                | Notes                                                                                  |
| -------- | ----------------------------------------- | -------------------------------------------------------------------------------------- |
| Step     | `StaticText`                              | Step index/label, not editable.                                                        |
| Agent    | `ChooseOne` over installed providers      | Options are the same installed-provider snapshot used by `compose`'s picker.           |
| Model    | `ChooseOne` over the resolved model list  | Falls back to `TextInput` for providers whose catalog is not enumerable (see "Model Enumeration"). |

Behavior:

- Each row is **pre-populated** using the same signals that drive the TTY picker's defaults (frontmatter `agent`, configured `favorite_agent`, and env vars for the model side). This mirrors the TTY agent resolution chain in "Resolution Order": the signals _inform_ the row's default, they do not unilaterally resolve it.
- **Every row is always presented for user sign-off**, even when frontmatter and favorite together would fully specify the row. Rows are not silently skipped on the basis of "already resolved" — the consolidated review screen _is_ the TTY picker, applied to each step.
- Cell focus follows `InputTable` conventions: arrow keys navigate cells, Tab/Shift+Tab wrap, and entering a cell activates its inner widget (`ChooseOne` or `TextInput`) for in-place editing.
- **`Ctrl+S` is the single sign-off** that submits the whole table; only after sign-off do steps begin executing. **`Esc` cancels** and aborts the sequence command before any step runs.

Because `InputTable` already provides the navigable, in-place-editable table shape this feature wants, **no `inquire`-based MVP is required**. The first implementation can land directly on the end-state UX.

This consolidated review honors all three originally-conflicting goals:

- front-loaded interactivity (everything the user needs to decide is decided before execution starts)
- different agents/models per step are desirable and supported
- the user signs off on the overall configuration in a single action

### Sequence in non-TTY contexts

- The consolidated review screen is **skipped** entirely.
- The resolver runs per-step using the **non-TTY agent resolution chain** defined in "Resolution Order" (frontmatter `agent` and `favorite_agent` are resolving signals here, since no picker is available).
- If **any** step fails to resolve to an installed agent, the entire sequence **errors before any step executes**. This preserves the front-loading semantic in scripted contexts: you either have a fully-specified sequence or you get a clear error up front, never a partial run that blocks mid-stream.
