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


### Compositional Flow Features

One of the newer and cooler features provided by Claudine is what Claudine' calls _compositional flow_ features. There are more than one variant of these but they all have the following characteristics:

- allow you to write and leverage Markdown as a "dynamic template"
- the dynamic nature of these documents is derived from both the 
    - _parameterization_ and _flow control_ provided by Claudine
    - the compositional features of the [**Darkmatter**](../darkmatter/README.md) library
- these features provide meta-agentic processes which orchestrate/operate outside of the Agent platform(s) themselves
- these process favor non-interactive sessions over interactive but allow for both
    - if you do want to run in an interactive session use the `--interactive` / `-i` flag
- all variants support file reference resolution of relative path, absolute paths, repo-relative paths, monorepo-package-relative, and magic paths (represented by the leading `@` character many Agents provide).

#### The `compose` Variant

##### What

The `compose` variant allows you to reference a Markdown file as a prompt for your Agent.

##### How

- it is available in two syntaxes:
    - `claudine <agent> --compose <file-ref>`, and
    - `claudine compose <file-ref>`
- both syntaxes provide the same functionality but the `claudine <agent>` syntax explicitly states the agent provider we will use whereas the `claudine compose` syntax will lead to a lazy evaluation of the agent to use
    - `claudine compose` may be more appropriate for open source projects where you have a diverse set of contributors each using a different Agent or set of Agent providers.
- when you run -- _as an example_ -- `claudine claude --compose "@foobar.md"`; Claudine will:
    - use both "repo", "package", and "user" scoped paths to resolve the `foobar.md` file
    - once resolved (still in a purely deterministic mode), it will _compose_ this Markdown file with Darkmatter's compose pipeline
    - once composed this prompt will be passed to the `claude` agent harness for execution of a non-interactive session (using `-i` CLI switch would have used an interactive session)
    - once the non-interactive session has completed the appropriate exit code is returned based on a successful or unsuccessful outcome

##### Why

- we all know a "better prompt" provides a "better answer"
- but how can we allocate the time to build better prompts?
    - there is no _one answer_ to this question, but
    - where you can gain reuse on a prompt you can justify spending more time on it
        - In some cases that means more time up front because you can see the value in it immediately
        - but probably more importantly if there's a prompt you can keep on tweaking and iterating on each time you use it so that it improves over time
- so far you may be thinking ... thanks Claudine, you've just described a Slash Command
- The analogy is not _wrong_ but it misses the extra capabilities you get with a compose prompt:
    - being able to inject content conditionally, inject shell command output (with a security model), leverage the composition of a graph of sub-documents via transclusion, interpolate state derived from ENV variables, Claudine provided context, or Frontmatter properties on the page (or inherited during transclusion)
    - all of this dynamic behavior is provided in a purely deterministic environment which guarantees a known prompt strategy will be used, in a lightning quick manner, without a single token being used to build your prompt on the fly and all the while doing it in a way which is Agent neutral
    - non-interactive sessions also benefit from:
        - the session will be streamed (so caller gets real-time feedback)
        - the stream will be visually enhanced for the terminal (`**bold**` is actually **bold** text, Markdown tables look like tables, etc.)
        - metadata is injected into the STDERR stream (session id, model, tokens, cost basis, etc.) for context but not in conflict with STDOUT

For more details on the **compose** variant, read the [Compose Prompting](./docs/topics/compose.md) document.

#### The Inline Composition Variant

While the `compose` variant used the frontmatter and body of Markdown file to create a high quality _prompt_ on the fly and then pass that into an Agent. With the _inline composition_ variant we will leverage the frontmatter of a document to create or update the body of the same document.

##### How

The inline composition variant is available in two syntaxes:

- `claudine <agent> --inline-composition <file-ref>`
    - the `--inline-composition` CLI switch also comes with following aliases:
        - `--inline-compose`
        - `--frontmatter-property`
        - `--fp`
    - use whichever cognitively makes the most sense to you
- `claudine inline-compose <file-ref>`; lazily resolves which agent to use

##### What

In order to pull off inline composition in a compact CLI surface we will rely on some conventions:

- the `prompt` property is where we expect to find the "prompt" that we'll use to generate content for the body of the document
    - whatever text is found in this property will be run through Darkmatter's compose pipeline first
    - this allows the same dynamic behaviors discussed in the compose variant
- the `policy` property is reserved for content policy declarations (_coming soon_)
    - this policy describes when the content will become "stale" and need to be _re_-generated
- the `blast_radius` property is an optional property which can be added to technical documents
    - this property is structured as a list of source files
    - these files -- when present -- will be analyzed for changes and will trigger a targeted re-generation of content when source files have been updated after the document was last updated
    - when first setting up a document you plan to use for inline-composition you can simply set it to `[]` and then the blast_radius will be determined by a parallel agentic process.
    - if you prefer to manually stipulate the blast radius you can do that too
    - Note: a blast_radius definition _can_ in effect becomes a specialized "policy" for updates and can be used _in addition_ to other policies should that be desirable.
- the `last_executed` property _will_ be updated each time the file is evaluated
- the `last_updated` property _will_ be updated each time the file's body is updated
- the `_prompt_hash` and `_blast_radius_hash` are used to track changes in the prompt and blast radius (you should not set these but Claudine will when this file is processed)
- the `_transcluded_docs` property will be added by Claudine if the base document uses [_transclusion_](../darkmatter/docs/topics/transclusion.md) and will be a list of underlying documents this document is dependent on.

> **Note:** the only required property is `prompt` and you should feel free to use the YAML `|-` directive to produce larger prompts where having multi-line spacing is useful to legibility.

When you run an inline composition the following steps are executed:

- **File Resolution** - _attempts to resolve the file reference into a real file path, returns immediately with error if file is not found_
- **Prompt Existence** - _validates that the resolved file has a `prompt` frontmatter property and returns immediately with an error if it doesn't_
- **Permissions Check** - _if the current session doesn't have read and write permissions to the file then we return immediately with an error_
- **Composition Error** - _when we run the prompt through Darkmatter's composition pipeline, the pipeline can succeed, fail, or require permissions approval; Claudine will interactively ask for permission when that is required but if any failure occurs we will immediately return with an appropriate error_
- **Freshness Check** - _when a document's body is empty or any of the content policies indicate that the document is stale we will continue processing but if the document looks like it's content is already fresh then we will simply indicate that diagnosis to the caller and exit with a success code_
- **Agentic Harness** - _we will now hand off to the Agentic CLI to update the Markdown bodies content_
    - Note: _the user's prompt will have a small addendum added to it to provide the Agent context that it is to update THIS document and that the inline composition frontmatter properties are NOT to be touched_
- **Agent Failure** - _if we detect that the Agent believes it has failed then we will immediately stop and return an error message_
- **Task Failure** - _if the agent believes it has succeeded but the document's body has not changed or is empty then we will report this as an error_
- **Frontmatter Check** - _the Agent will have been told not to update the Frontmatter but if they do anyway, we'll report this as a warning but convert the frontmatter back to it's original/intended state_

##### Why

This form of composition provide a self-contained way of keeping a document up-to-date and is often used for research and avoiding documentation drift.


#### The `sequence` Variant

The `sequence` variant is the most powerful of the compositional variants as it allows for a multi-step state machine to be setup for long running tasks along with stage gates (aka, validations) used to check that the pipeline is ready to move to the next state.



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

::shell claudine providers



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
