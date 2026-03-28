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

- **Better Logging**
    - By being at the _start_ and _end_ of each session we can provide better logs
- **Consistent CLI Switches**
    - Claudine will take functional elements which are exposed as CLI switches across most/all of the Agentic providers and give it a consistent name.
    - Example:
        - almost all Agents provide some sort of **yolo** mode but the nomenclature for starting in that mode varies widely
        - when you're wrapping execution you can use `--yolo`, `-y` with any Agent and it will map that into the provider's CLI appropriately
    - Note:
        - no CLI functionality is lost via this approach, any CLI switch which is not a "standardized/universal" switch will be passed down to the provider as well so you can still access any feature you need
- **Compositional Flow Features**
    - We'll cover this in the next section


### Composition

Claudine's composition features let you use Markdown as a dynamic template for agentic CLI sessions, leveraging [Darkmatter](../darkmatter/README.md)'s composition pipeline (transclusion, interpolation, conditionals, shell commands).

Two canonical commands:

- **`claudine compose <file-ref>`** — compose a Markdown file and send it as a prompt (no file mutation)
- **`claudine inline-compose <file-ref>`** — use the frontmatter `prompt` property to generate and replace the document body

Both commands share a wrapper-grade execution pipeline with full support for environment setup, harness detection, structured streaming, and handler-driven recovery.

Provider selection uses explicit flags (`--claude`, `--codex`, etc.), frontmatter hints, config favorites, or interactive chooser. Use `-i` for interactive sessions, `--exclude` to filter providers.

For full details, see [Composition](./docs/topics/composition.md).


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
| [claudine (lib)](./lib/) | Event model, provider adapters, dispatch pipeline, structured stream parsing (6 providers), skill linking, MCP catalog/sync/runtime support |
| [claudine-cli](./cli/) | Binary `claudine` — setup wizard, hook inspection, link management, MCP commands, provider wrapper with structured streaming, composition pipelines |

## Documentation

- [Shared Event Model](./docs/shared-event-model.md) - Universal event abstraction (16 events)
- [Agent Configuration](./docs/agent-configuration.md) - Per-provider setup details
- [Skill Linking](./docs/skill-linking.md) - Cross-provider skill synchronization
- [MCP Support](./docs/mcp-support.md) - Catalog storage, `claudine mcp`, provider support, and wrapper runtime behavior
- [Log Reporting](./docs/log-reporting.md) - JSONL-to-SQLite reporting model and `claudine logs`
- [Provider Hooks](./docs/hooks/) - Per-provider hook specifications

## Monorepo Dependencies

Uses the following libraries from this monorepo:

- `biscuit-hash` - xxHash content hashing for skill deduplication
- `biscuit-speaks` - Text-to-speech for speak actions
- `biscuit-terminal` - Terminal detection and rich output (tables, prose)
- `darkmatter` - Markdown rendering for `about` command
- `playa` - Sound effect playback (88 embedded effects)
- `sniff` - System and environment detection (OS, hardware, git, repo context)
