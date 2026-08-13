# Claudine

> Claude Code's ex-girlfriend who knows Claude's inner secrets but is now dating other Agents

<img src="../assets/claudine-512.png" style="position:absolute; top:0; right: 100; width: 100" />

## About

Claudine's primary focus is helping you to move between different Agentic CLI's with as much consistency as possible but over time we've added some features on top that benefit all Agentic CLIs.

### Shared Resources

The very first thing Claudine does is synchronize shared resources across all the Agents you use. We define a "resource" as:

- an **Agentic Skill**
- a **Slash Command**
- or a **Agent**/**Subagent** definitions

### Hooks and Actions

When you initialize Claudine, we'll add a hook to all events across all Agentic CLI's your host has installed. These hooks will by default be no-ops but engaging with all hooks it just helps Claudine to abstract the event handling features fully into Claudine.

Claudine provides a "universal" set of of hooks you can hook into. By abstracting to a universal set of hooks you can describe _what you want_ and we'll map that into the individual Agent systems. This abstraction helps reduce cognitive overload by only requiring you to remember a single set of event names and we'll map them into the underlying system with as much consistency as possible.

> **Note:** there is a good deal of _variance_ across Agents in how many hooks are provided and in some cases whether a given event can stop execution flow or not. Claudine let's you report/query on various aspects of Hooks configuration and provider capabilities via the CLI:
>
> - `claudine hooks` - provides a table of the providers and the events which each provider supports
> - `claudine hooks --support` - is shows the Claudine hooks system in the first column and then each Agent platform is a row with an icon showing the level of support which this platform provides
> - `claudine hooks --describe` - provides _descriptions_ for each of the Claudine hooks as well as the schemas for both the Event data and the what can be returned

#### Taking Action

Like any standard hook based event system, you can attach your own custom actions to events. In addition to "custom" events we provide a small set of actions you can use out of the box:

- **Speak** - you can specify a spoken phrase you'd like spoken on a given hook event; static text is fine but you can also interject dynamic elements from ENV variables, the event payload and more.
- **Message** - you can send a message to a platform like Discord, Slack, WhatsApp, etc. on a particular event (FUTURE, almost ready)
- **SoundEffect** - there is a small sound effects library you can leverage which can be triggered when a particular event fires.

In addition to the two actions above, Claudine provides two _services_ which are **cross-event** in nature:

- **Logging** - Claudine logs what it sees. This includes hook events (as well as _wrapped execution_ or _composition_ when that is used). Logging can be turned off and we ask you whether you'd like this on when you first run `claudine init`.
- **Protect** - We provide a _protection_ service which will analyze hook events for dangerous commands and either block these commands or require explicit user approval.

### Agentic Execution Wrapping

All of the features described above can be had by configuring Claudine once and then largely ignoring Claudine after that. Claudine, however, is vein and really doesn't like being ignored. This is where _wrapped execution_ comes in.

Wrapped execution takes the form of you starting your CLI Agent _with_ claudine:

- instead of starting **Claude Code** with `claude`, you would instead run `claudine claude`
    - `claudine codex`, `claudine opencode`, `claudine gemini`, etc.

Now we understand that more characters typed is time _not_ well spent but we suggest adding an alias or a `just` recipe so you get in the habit of using the wrapped variants.

> **Note:** in this monorepo you simply run `just cc` for Claude, `just oc` for Opencode, etc.

But before we try to talk you into typing more characters, forcing you to create an alias, or bend to the will of Claudine for no tangible benefit ... let's talk about why you might want to used wrapped execution.

The immediate benefits of wrapped execution are:

- **ENV variables**
    - **Protect.** Claudine will intelligently strip out potentially dangerous/secret **ENV keys** for you; helping you to not spuriously share secrets that aren't needed to be shared.
    - **More Context.** Claudine will also add a few "contextual" ENV keys for you which might be helpful for event handling and are used for logging purposes. Things like 
        - `AGENT` being set for the Agentic CLI's name
        - `REPO` the name of the repo you're in
        - `PACKAGE_AREA` the name of the package area you are in
        - `PACKAGE` the name of the particular package you are in
        - `INTERACTIVE` is a boolean flag indicating whether the session is interactive or not
        - `YOLO` is a boolean flag indicating whether you are running in **yolo** mode or not

    > **Note:** by default when you start Claudine in wrapped execution mode, it will explicitly share which variables have been removed and which have been added. Later, if you don't want to always see that you can run your commands with the `--quiet` flag and this extra information will be removed.

- **Refined Agent Responses**
    - Claudine normalizes and cleans up the output from all agents, ensuring consistent formatting, spacing, and styling.
    - **Section Model.** Every non-interactive run follows a 9-section rendered model (execution line, env, system prompt, agent prompt, session ID, thinking prose, tool/info events, final STDOUT, and metadata) with strictly enforced spacing rules (at most one blank line between sections).
    - **Thinking Prose.** Reasoning and thinking content from providers (Claude, Codex, etc.) is rendered as a `BlockQuote` with a grey vertical line and dim-italic text on stderr, providing continuous feedback during long turns.
    - **Tool-Call Display.** Tool calls use a canonical contract (`🔧 →` for outgoing, `🔧 ←` for incoming) with humanized names and summarized arguments/results. No more raw JSON dumps to the terminal for known tools.
    - **Markdown Rendering.** Improved markdown support, including a fix for Gemini's mid-list truncation and stray blank lines in unordered lists.

- **Better Logging**
    - By being at the _start_ and _end_ of each session we can provide better logs
- **Consistent CLI Switches**
    - Claudine will take functional elements which are exposed as CLI switches across most/all of the Agentic providers and give it a consistent name.
    - Example:
        - almost all Agents provide some sort of **yolo** mode but the nomenclature for starting in that mode varies widely
        - when you're wrapping execution you can use `--yolo`, `-y` with any Agent and it will map that into the provider's CLI appropriately
    - Note:
        - no CLI functionality is lost via this approach, any CLI switch which is not a "standardized/universal" switch will be passed down to the provider as well so you can still access any feature you need
        - **OpenCode YOLO.** Claudine now correctly forwards `--dangerously-skip-permissions` to OpenCode in non-interactive sessions when `--yolo` is used.
- **Compositional Flow Features**
    - We'll cover this in the next section


### Composition

Claudine's composition features let you use Markdown as a dynamic template for agentic CLI sessions, leveraging [Darkmatter](../darkmatter/README.md)'s composition pipeline (transclusion, interpolation, conditionals, shell commands).

Three canonical commands:

- **`claudine compose <file-ref> [key=value ...]`** — compose a Markdown file and send it as a prompt (no file mutation)
- **`claudine inline-compose <file-ref> [key=value ...]`** — use the frontmatter `prompt` property to generate content and replace the document body, preserving frontmatter byte-for-byte
- **`claudine sequence <file-ref> [key=value ...]`** — run a serial sequence of composition steps declared in one document, with a shared shell approval cache and `FAIL_FAST` propagation on failure

**Inline Shorthand.** You can override frontmatter values using `key=value` positional arguments. Values are parsed as JSON5 first (supporting numbers, booleans, arrays) and fall back to plain strings. These shorthand overrides win over `--set` JSON blobs.

```sh
claudine compose @prompts/review.md review="review.md" count=3 draft=true
```

All three commands share a wrapper-grade execution pipeline with full support for environment setup, system prompt resolution, harness detection, structured streaming, and lifecycle-stack recovery.

**Unified Harness Execution.** Every non-dry-run `compose` and `inline-compose` run flows through `run_harness_loop` with `HarnessPromptMode::Compose` or `HarnessPromptMode::Inline`. Documents without harness frontmatter yield the empty/bare plan; the plan now carries only timeout configuration (the pre/post validation and handler-recovery DSL has been retired in favor of lifecycle stacks). The loop handles structured streaming, captured/non-structured fallback, inline closure, summary emission, and lifecycle-stack recovery (`Retry`/`Resume`/`Proxy`) through one code path.

Provider selection uses explicit flags (`--claude`, `--codex`, etc.), frontmatter hints, config favorites, or interactive chooser. Use `-i` for interactive sessions, `--exclude` to filter providers.

For full details, see [Composition](./docs/topics/composition.md).

### Performance Reporting

All wrapper and composition commands support an opt-in `--perf` flag that emits a detailed performance report to **stderr** after the command completes:

- `claudine {agent} --perf ...`
- `claudine compose --perf ...`
- `claudine inline-compose --perf ...`
- `claudine sequence --perf ...`

The report is a reconciled timing tree with these major branches:

1. **CLI Overhead** — time spent on arg parsing, config loading, tracing init, and environment setup.
2. **Source Context Timing** — overlapping diagnostic timings for invocation
   capture, repository observation, topology initialization, launch-context
   capture, and system-prompt preparation. Stable Git/topology work counts are
   reported alongside the tree.
3. **Composition Report** — when document composition occurred, shows Darkmatter pipeline timings (transclusion, interpolation, shell expansion, etc.).
4. **Agent Execution** — provider handoff, number of launches, first-response latency, total execution time, and provider-reported API duration when available.

For `sequence`, a single aggregated report is printed at the very end, averaging first-response latencies across all steps and summing launches and total time. The report is emitted unconditionally when `--perf` is passed, even if `--silent` or `--quiet` are also present — perf is an explicit opt-in that overrides silence settings.

> **Note:** `provider_api_duration` is only available for providers that use the structured-streaming path (e.g., Codex, Gemini, OpenCode). Legacy providers such as Goose do not report this metric.

## Getting Started

The claudine library and CLI will eventually be released to [crates.io](https://crates.io) and will likely also be someday released into the [npm](https://npmjs.com) too but for now if you want to use **Claudine** you'll need to clone the **Rusty Biscuit** monorepo and build locally.

```sh
git clone https://github.com/yankeeinlondon/rusty-biscuit
```

Once cloned, you'll want to make sure you have the [`just`](https://github.com/casey/just) runner installed on your host. If you don't then you can install it via your favorite package manager:

```sh
# macOS
brew install just
# ubuntu / debian
apt install just
```

<details>
<summary>Other Package Managers:</summary>
<pre><code lang=sh>
asdf install just
apk add just
# Red Hat, CentOS, Rocky, etc.
dnf install just
# Arch, Manjaro, etc
pacman -S just
# Snap package manager
snap install --edge --classic just
# Nix Package manager
nix-env -iA nixpkgs.just
# Use NodeJS's **npm** package manager
npm install -g rust-just
# Python's **uv** package manager
uv tool install rust-just
</code></pre>
</details>

Now you'll want to run the following from the repo's root:

```bash
# ensure host has appropriate build tools and a few CLI's from the monorepo
# used for devops are installed on the system
just init
# now install the claudine CLI
just install-claudine
# now you're ready to use Claudine but it's best if you first just initialize it
claudine init # brings you through a short interactive interview
```


## Supported Providers

::shell claudine providers --plain



## More Information

- You can find full documentation on the CLI from [Claudine CLI](./docs/cli/index.md)
    - The help system is always available with `--help`
    - It is also recommended to install shell completions for full laziness/productivity (`claudine completions --help`)


## Configuration

Hook configuration is stored in `~/.claudine/config.json` (user-scoped) or `<repo>/.claudine/config.json` (project-scoped).

MCP state is stored separately in `~/.claudine/mcp/`:

- `catalog.json` - normalized server definitions
- `defaults.json` - user-scope default server IDs
- `provider-state.json` - import/export provenance
- `<repo>/.claudine/mcp.json` - repo-scope default server IDs

## Packages

| Package | Description |
|---------|-------------|
| [claudine (lib)](./lib/) | Event model, provider adapters, dispatch pipeline, structured stream parsing with strongly typed protocol models (6 providers), shared-resource linking, MCP catalog/sync/runtime support, composition pipelines, harness timeouts/shell-audit/attempt-classification, permissions policy engine |
| [claudine-cli](./cli/) | Binary `claudine` — setup wizard, hook inspection, shared-resource commands (`skills`/`commands`/`agents`), MCP management, provider wrappers with structured streaming, and composition pipelines (`compose`/`inline-compose`/`sequence`) |

## Documentation

See [`./docs/topics/`](./docs/topics/) for the full topic index. Key topics include:

- [Unified Events](./docs/topics/unified-events.md) - Universal event abstraction (16 events)
- [Skills](./docs/topics/skills.md), [Commands](./docs/topics/commands.md), [Agents](./docs/topics/agents.md) - Cross-provider shared-resource synchronization
- [MCP Catalog](./docs/topics/mcp-catalog.md) and [MCP Mode](./docs/topics/mcp-mode.md) - Catalog storage, `claudine mcp`, provider support, wrapper runtime behavior
- [Composition](./docs/topics/composition.md) - `compose`, `inline-compose`, `sequence` commands and harness
- [System Prompt](./docs/topics/system-prompt.md) - Discovery and CLI switch resolution
- [Pre-Flight Checks](./docs/topics/pre-flight-checks.md) and [Lifecycle](./docs/topics/lifecycle.md) - Pre-flight shell audit/schema validation and the lifecycle stack (gating, verification, recovery)
- [Policy Engine](./docs/topics/policy-engine.md) and [Protect Service](./docs/topics/protect-service.md) - Permissions and runtime safety
- [Log Reporting](./docs/topics/log-reporting.md) and [Traces and Logging](./docs/topics/traces-and-logging.md) - JSONL-to-SQLite reporting and diagnostics
- [Wrapped Execution Switches](./docs/topics/wrapped-execution-switches.md) - CLI switch translation per provider
- [Non-Interactive Sessions](./docs/topics/non-interactive-sessions.md) and [Mixing Events into Non-Interactive Sessions](./docs/topics/mixing-events-into-non-interactive-sessions.md)
- [Repo Isolation](./docs/topics/repo-isolation.md) - Shadow HOME behavior for `--repo`
- [Stream Parsing](./docs/topics/stream-parsing.md) - Provider-native structured stream handling

## Monorepo Dependencies

Uses the following libraries from this monorepo:

- `biscuit-file` - File reference resolution (`@` magic paths) for composition commands
- `biscuit-hash` - xxHash content hashing for skill deduplication
- `biscuit-speaks` - Text-to-speech for speak actions
- `biscuit-terminal` - Terminal detection and rich output (tables, prose, OSC8 hyperlinks)
- `darkmatter` - Composition pipeline (transclusion, interpolation, `::shell`) and Markdown-to-terminal rendering
- `playa` - Sound effect playback (88 embedded effects)
- `sniff` - System and environment detection (OS, hardware, git, repo context)
