# `--dry-run` touch up

The `--dry-run` CLI switch for claudine appears to be largely working but there are a few things which
this feature do to improve it:

## Scope

This is a **full-scope, single-spec** feature. It covers **both**:

1. The `--dry-run` presentation — how each agent-resolution state is communicated in the metadata table.
2. The **real (non-`--dry-run`) live execution path** — audited so that the dry-run table is a **faithful prediction** of what actually happens at run time.

The two paths are specified together precisely so they cannot drift: every agent-resolution state documented for the dry-run table below has a corresponding live-path behavior specified in [Live-Path Behavior](#live-path-behavior). The dry-run table must describe what the live path will do, and the live path must do what the dry-run table describes.

## Agent Resolution States (dry-run presentation)

- currently if you don't specify the Agent to use on the CLI (or Document) then it will interactively ask you to specify the Agent
    - instead of that we should skip asking the user and simply report this in the metadata table
    - we already have a `Agent` key/value but when the user didn't pass in the agent as a CLI switch:
        - if the markdown document has a `agent` property:
            - if the agent was set to a single valid agent then it should be the selected agent and just listed in the table by name
            - if the agent was set to a single value but there's no match to any of the agent providers then:
                - the table should show `<red>Invalid Agent</red>(<dim>{agent}</dim>) <i>defined in Markdown's <inverse>agent</inverse> Frontmatter! Caller will be prompted to choose a valid Agent when run interactively; otherwise the run aborts with the same message.</i>`
                - we also should make sure that in this situation -- when no `--dry-run` is being used -- that we:
                    - report "<red><b>Invalid Agent:</b></red> the <a href={markdown-path}>{agent}</a> references an invalid Agent provider '{agent}'. Choose from the installed agents on this host:"
                    - use the choose_one TUI component to presents the agent's that the current host has installed
            - if the agent was set to a single value but the user doesn't have that agent installed:
                - the table should show `<yellow>Agent Not Installed:</yellow>(<dim>{agent}</dim>) <i>the Markdown document's <inverse>agent</inverse> specifies an Agent platform which is not installed on this host. When run interactively, caller will be asked to choose an Agent; otherwise the run aborts with the same message.</i>`
                - make sure that the actual runtime behavior without the `--dry-run` flag matches the communicated behavior
            - if the agent has a list of available agents:
                - if two or more Agents listed are installed on the host (and valid):
                    - `<green>✓</green> caller will be asked to choose interactively between suggested Agents:`
                    - an unordered list of the suggested agents
                    - if any of the suggestions are not installed on host then use `<dim>{agent}</dim>` styling
                    - after all VALID agent suggested are listed, if there were suggested agents which were invalid then:
                        - `The following agents were suggested but are <b><red>NOT</red></b> valid Agents:`
                        - list all invalid suggested agents
                - if suggested Agents contain exactly one valid and installed Agent on the host:
                    - `<green>✓</green> the <b>{agent}</b> will be used without the need for interactive prompting`
                    - `the Markdown document suggested multiple agent's but only <b>{agent}</b> is installed on this host`
                    - if there are any suggested agents which are invalid:
                        - `The following agents were suggested but are <b><red>NOT</red></b> valid Agents:`
                        - list all invalid suggested agents
                - if the suggested list resolves to **zero installed agents** (all suggestions are not-installed, all are invalid, or a not-installed + invalid mix with nothing installed):
                    - `None of the suggested agents are installed/valid; caller will choose from all installed agents when run interactively; otherwise the run aborts with the same message:`
                    - an unordered list of the suggested-but-not-installed agents, each styled `<dim>{agent}</dim>`
                    - where any suggestions are invalid:
                        - `The following agents were suggested but are <b><red>NOT</red></b> valid Agents:`
                        - list all invalid suggested agents
            - if the Frontmatter doesn't have an "agent" definition and the caller didn't specify the agent in the call then (unordered list):
                - `CLI caller didn't specify the Agent`
                - `the Markdown document didn't suggest any Agents in <inverse>agent</inverse> Frontmatter property`
                - `the caller will be interactively asked to choose an agent when <i>composing</i> called without the <green>--dry-run</green> flag; otherwise the run aborts with the same message`

> **Non-interactive (no-TTY) gate (clarification, Decision 4):** The agent re-prompt gate is **TTY presence only** — it must NOT use the four-signal `InteractiveSchemaOptions::allowed()` gate (`schema_validation.rs:87-89`), because that folds in `--silent` and `prompt_for_missing`, which must NOT affect agent resolution. For **every** agent-resolution state that would otherwise prompt (no-agent, invalid frontmatter agent, not-installed agent, and the zero-installed-list state above): when a TTY is present, show the `choose_one` re-prompt; when no TTY is present, emit the **same** styled message the TTY path would show (`Invalid Agent:` / `Agent Not Installed:` / the no-agent message / the zero-installed-list message, as applicable) to **stderr**, then abort with a structured **non-zero exit**. Do NOT auto-pick a substitute agent, and do NOT fall back to a generic/raw resolver error. `--silent` governs only status-message verbosity and has **NO** effect on whether or how an undefined, invalid, or not-installed agent is resolved or reported.

> **Note:**
>
> - more attention was paid to how to communicate the various possible states of the agent's assignment (via CLI and `agent` property) then was spent mentioning that the execution of claudine _without_ the `--dry-run` flag must also match up with what we're communicating will happen. Make sure this is either already implemented or that the change in implementation is discussed and fully specified during the "clarification" process
>     - **RESOLVED (clarification):** this concern is now addressed by the [Scope](#scope) commitment. The live (non-`--dry-run`) path is fully specified per agent-resolution state in [Live-Path Behavior](#live-path-behavior), and the dry-run table is required to be a faithful prediction of it.
> - a nice looking output is really important and many stylistic hints have been provided but when clarifying this specification you should look for ways to make the output as nice and as consistent as possible for the caller. If you need clarification on what has been already been stated or you have suggestions to do more (or something else) be sure to bring this up in the clarification process.

## Live-Path Behavior

This section specifies the **real, non-`--dry-run`** behavior for every agent-resolution state. The dry-run table text above is the prediction; this is the behavior being predicted. They must agree.

**TTY-only prompting gate.** Every prompting state below is gated on **TTY presence only** (not the four-signal `InteractiveSchemaOptions::allowed()` gate). When a TTY is present, the `choose_one` re-prompt is shown as specified. When no TTY is present, the **same** styled message the TTY path would show is emitted to **stderr** and the run aborts with a structured **non-zero exit** — no auto-pick, no generic/raw resolver error. `--silent` affects only status-message verbosity and has **no** effect on agent resolution or its reporting.

- **No agent at all** (neither CLI nor frontmatter `agent`):
    - **TTY** → the caller is shown the interactive picker (`choose_one`) over installed agents.
    - **No TTY** → the styled no-agent message is emitted to stderr and the run aborts (non-zero exit).
- **Invalid frontmatter `agent`** (a single value that matches no provider):
    - There is **no live abort on the invalid value itself** — the invalid value is non-fatal and routed into the resolution state.
    - **TTY** → the styled `Invalid Agent:` error message (the message in the agent-states list above, beginning `<red><b>Invalid Agent:</b></red> …`) is emitted, then the caller is re-prompted with the `choose_one` TUI over installed agents.
    - **No TTY** → the same styled `Invalid Agent:` message is emitted to stderr and the run aborts (non-zero exit).
- **Not-installed agent** (a single valid provider that is not installed on this host):
    - **TTY** → the caller is asked to choose an agent (re-prompt via `choose_one` over installed agents), matching the dry-run text communicated for this state.
    - **No TTY** → the styled `Agent Not Installed:` message is emitted to stderr and the run aborts (non-zero exit).
- **List with exactly one valid + installed agent:**
    - That agent is **silently auto-selected**; no prompt is shown (this state never prompts, so the TTY gate does not apply).
- **List with two or more valid + installed agents:**
    - **TTY** → the interactive picker is shown, scoped to the suggested installed agents.
    - **No TTY** → the styled message for this state is emitted to stderr and the run aborts (non-zero exit).
- **List resolving to zero installed agents** (all-not-installed, all-invalid, or a not-installed + invalid mix with nothing installed):
    - **TTY** → the styled zero-installed-list message is emitted, then `choose_one` is shown over **all installed agents** (scoping is to all installed, since none of the suggestions are installed).
    - **No TTY** → the same styled zero-installed-list message is emitted to stderr and the run aborts (non-zero exit).

## Implementation Requirements

The current code shape makes these states impossible to render faithfully without two structural changes. Both are required:

1. **Make the invalid-agent case non-fatal — for single values AND list entries.** Today an invalid `agent` frontmatter value becomes a hard `CompositionError::AgentHintInvalid` which aborts composition **before any table or picker can run**. This hard abort happens in **both** the single-value case (`claudine/lib/src/composition/prepare.rs:281-286`, raised at ~line 285) **and** for an invalid entry within a frontmatter `agent` **list** (`claudine/lib/src/composition/prepare.rs:294`). The non-fatal routing requirement covers **both** sites: invalid single values and invalid list entries must instead be routed into the new agent-resolution/render state (see [Renderer Data Model](#renderer-data-model)) so that:
    - under `--dry-run` they render as the `Invalid Agent` cell (single value) or contribute to the `NOT valid` list / zero-installed-list state (list entries), and
    - under live execution they emit the styled message and re-prompt or abort per the TTY gate (per [Live-Path Behavior](#live-path-behavior)).
2. **Gate provider selection on `--dry-run`.** The interactive picker (`claudine/cli/src/commands/wrap/composition/mod.rs`, picker ~line 734; selection ~671-794) currently runs **before** the dry-run early-return seam (~line 1455). Under `--dry-run`, **no picker may fire**: unresolved, invalid, and not-installed agents must be captured as **render states**, not as prompts or errors. The dry-run seam must sit ahead of (or otherwise short-circuit) provider selection so that resolution outcomes are recorded for the table rather than acted upon.
3. **Gate the agent prompt/error on TTY presence — not the four-signal interactive gate — and key off the correct channel.** The agent re-prompt/abort gate must check **TTY presence only**, and must NOT reuse `InteractiveSchemaOptions::allowed()` (`schema_validation.rs:87-89`), which folds in `--silent` and `prompt_for_missing`. There is an existing TTY-detection inconsistency the implementor must resolve: provider selection currently checks `stdin && stdout` (`mod.rs:700`), whereas the styled messages and the dry-run table target **stderr** (`mod.rs:1451-1452`). The agent prompt/error gate should key off the channel actually used for prompting (**stderr**), not stdout. This is an implementation requirement to make the gate consistent with the channel in use — not a redesign of the prompting model.

## Formatting

- there should be a full width `hr` after the prompt but before the YAML frontmatter and metadata table
    - when dry-running a "sequence" then each prompt should be delimited by a `hr`
- the YAML frontmatter should use the "inverse" theme for code highlighting (just like we always do when rendering code blocks in Markdown); this means that a dark themed terminal will use a light theme and visa versa. This makes code blocks stand out better from the page.
    - the code block should have a left margin of `1ch` so that's offset matches the tables visual appearance
    - directly before the YAML code block we should add a `<b>Frontmatter</b>(<i>resolved</i>):` heading should be added with a bottom margin of 1
- The metadata table should have a top margin of 1 to separate it from the YAML Frontmatter

## Agent Cell Layout

The agent-resolution breakdown is rendered as **multi-line content inside the single `Agent` table row's value cell** — not as additional table rows. The value cell holds a rendered multi-line Prose block consisting of:

- a header line (e.g. the `<green>✓</green> …` suggestion header, the `Invalid Agent` line, the zero-installed-list header `None of the suggested agents are installed/valid; caller will choose from all installed agents:`, or the no-agent unordered list),
- a bulleted suggestion list, with **per-item dim styling** (`<dim>{agent}</dim>`) for suggestions that are not installed, and
- where applicable, a `NOT valid` header (`The following agents were suggested but are <b><red>NOT</red></b> valid Agents:`) followed by a list of the invalid suggestions.

For the **zero-installed-list** state, the cell renders the zero-installed-list header, then the suggested-but-not-installed list (per-item `<dim>{agent}</dim>`), then — where any suggestions are invalid — the `NOT valid` header and the invalid list.

### Renderer Data Model

- The renderer data model must widen from the current `agent: Option<Provider>` (in `claudine/cli/src/commands/wrap/composition/dry_run.rs`, table built ~line 137) to a richer `AgentResolutionState` enum. `Option<Provider>` is too narrow to carry the breakdown above.
- `AgentResolutionState` must carry:
    - the selected provider (when one is auto-selected),
    - the suggested-installed list,
    - the suggested-not-installed list,
    - the invalid list,
    - the distinct no-agent / single-invalid / single-not-installed cases, and
    - the **zero-installed-list** case (a frontmatter `agent` list resolving to zero installed agents — covering all-not-installed, all-invalid, and a not-installed + invalid mix with nothing installed), as a dedicated variant.

### Table Component Confirmation

- Implementation must **confirm** that the biscuit-terminal table component preserves embedded newlines and bulleted lists inside a single cell **without breaking two-column alignment**, and that the existing `1ch` left-margin offset still holds with a multi-line cell.
- **L2 capture risk:** SGR can collapse across captures, and multi-line styled cells are exactly where alignment and SGR-collapse assertions are fragile. The test strategy must account for this (see [Verification Levels](#verification-levels)) — assert semantically rather than by byte-equality across captures.

## Acceptance Criteria

Definition of done is **L1 logic + L2 styling**. Each criterion below is mapped to a verification level (see [Verification Levels](#verification-levels)).

**Agent-resolution states (dry-run table) — L1:**

- [ ] No-agent state renders the no-agent unordered list — L1
- [ ] Single-valid (installed) agent renders by name as the selected agent — L1
- [ ] Single-invalid agent renders the `Invalid Agent` cell — L1
- [ ] Single-not-installed agent renders the `Agent Not Installed` cell — L1
- [ ] List with ≥2 installed agents renders the "choose interactively" header plus the suggested list — L1
- [ ] List with exactly one installed agent renders the auto-select header and explanatory line — L1
- [ ] A list containing invalids renders the `NOT valid` header and the invalid list (in addition to the valid breakdown) — L1
- [ ] Zero-installed-list state renders correctly (header + dim not-installed list + `NOT valid` list) and routes to the all-installed picker (TTY) / abort (no TTY) — L1

**Live-path resolution / picker behavior — L1:**

- [ ] No-agent → interactive picker over installed agents — L1
- [ ] Invalid agent → no abort; styled `Invalid Agent:` message emitted, then `choose_one` re-prompt — L1
- [ ] Not-installed agent → `choose_one` re-prompt over installed agents — L1
- [ ] List with exactly one valid+installed → silent auto-select, no prompt — L1
- [ ] List with ≥2 valid+installed → interactive picker scoped to suggested installed agents — L1
- [ ] Zero-installed-list → `choose_one` scoped to **all** installed agents (TTY) — L1
- [ ] No-TTY arm: for EACH prompting state, the styled message is emitted to stderr with a non-zero exit (no auto-pick, no generic error) — L1
- [ ] `--silent` does not change agent resolution or its reporting — L1

**Structural formatting (presence + ordering) — L1:**

- [ ] Full-width `hr` rendered before the frontmatter (and a per-prompt `hr` between prompts in a sequence) — L1
- [ ] `Frontmatter (resolved):` heading present with bottom margin 1 — L1
- [ ] YAML code block rendered with a `1ch` left margin — L1
- [ ] Metadata table rendered with a top margin of 1 — L1
- [ ] Multi-line agent cell preserves two-column alignment and the `1ch` offset — L1

**Styling (real-terminal capture) — L2:**

- [ ] Red styling for invalid-agent states asserted from `frame.raw` — L2
- [ ] Yellow / dim styling for not-installed states asserted from `frame.raw` — L2
- [ ] `hr` rendering asserted in a real terminal — L2
- [ ] Inverse-theme YAML code block asserted in a real terminal — L2
- [ ] `Frontmatter (resolved):` heading and margin spacing asserted in a real terminal — L2

## Verification Levels

Modeled on the original feature's rigor (`claudine/features/2026-06-03-dry-run/`).

- **L1 — unit / integration, ANSI-stripped, deterministic.** One assertion per agent-resolution state (no-agent, single-valid, single-invalid, single-not-installed, list-with-≥2-installed, list-with-exactly-one-installed, list-containing-invalids, zero-installed-list); the live-path resolution / picker behavior per state, including the **TTY-only gate** — TTY present routes to `choose_one` (scoped to all installed agents for the zero-installed-list state), no TTY emits the same styled message to stderr and aborts with a non-zero exit (no auto-pick, no generic error) — for every prompting state; that `--silent` does not change agent resolution or its reporting; and the structural presence / ordering of each formatting rule (full-width `hr` before frontmatter, per-prompt `hr` in sequences, `Frontmatter (resolved):` heading with bottom margin 1, YAML code block with `1ch` left margin, table top margin 1).
- **L2 — real-terminal tmux / WezTerm capture** asserting SGR / OSC8 from `frame.raw`, following the existing `level2_dry_run_*` pattern: the red / yellow / dim color styling for invalid & not-installed states; `hr` rendering; inverse-theme YAML; and the heading and margin spacing. Because multi-line styled cells are fragile under SGR collapse, L2 assertions must be **semantic** (e.g. presence of the relevant SGR in both semicolon and ITU colon forms) rather than byte-equality across captures.
