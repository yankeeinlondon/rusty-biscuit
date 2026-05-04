# Getting Started with Claudine
![claudine](../../../assets/claudine-512.png)
> Claude Code's ex-girlfriend who knows Claude's inner secretes but is now dating other Agents

## Intro

**Claudine** was born out of the desire to be able to move fluidly between different agentic CLI's without having to go through a deep learning curve for each. With these humble beginnings **Claudine** has become a full featured "meta-agent" which allows users to treat Markdown as a _compositional_ primitive for building prompts all the way to long running multi-agent workflows.

## Installation

- Currently you'll need to have Rust installed on your computer and install it

    ```sh
    # clone repo
    git clone https://github.com/yankeeinlondon/rusty-biscuit.git
    # install just runner on your OS, run initializer to compile for your host
    just init
    ```

- This is obviously cumbersome for _users_ rather then developer who want to contribute
- A packaged version will be deployed to **cargo** first and soon afterward to **npm**

## Provider Coverage

**Claudine** currently covers the following agent providers:

- Claude Code
- Codex
- Gemini CLI
- OpenCode
- Kimi Code CLI
- Qwen CLI*
- Goose CLI*
- Roo Code (_deprecated_)

> **Note:** Roo Code was a great plugin solution for VSCode and an innovator during 2025 but their direction has changed and they will not be updating the VSCode plugin going forward. We will be removing support for this at some point but for now it's still there for those who continue to use **Roo Code**.
>
> **Note:** Qwen CLI and Goose CLI have received less testing then others, they pass all the automated tests but I have focused more attention on the others to start.

We are expecting to add the following providers soon:

- Pi
- Kilo Code

## Creating a Level Playing Field

**Claudine**'s original purpose was to create a platform that allows _use_ of many of the most popular agentic CLI's without requiring a mastery of each one. Let's break down how **Claudine** achieves this goal:

1. **Agent Skills**

    - the new champion of providing _expertise_ to your Agent is the **agent skill**
    - introduced by Claude Code and very quickly adopted by everyone as a pseudo standard is a nice way of giving your agent contextual super powers
    - Claudine makes sure that your agent skills (both user-scoped and repo-scoped) are available to all of the agent platforms we support (and are installed on your system)
    - run `claudine skills` for an overview of your catalog
        - run `claudine skills --fix` to fix synchronization issues that may have built up (alternatively run `claudine sync` which syncs skills, agents, and slash commands)

1. **Agent Definitions** _and_ **Slash Commands**

    - The ideas of:
        - providing a definition for an "agent" or "subagent" 
        - being able to define reusable prompts as "slash commands"
        
    Are both concepts that all agents share but the way they are implemented _is_ more variant than with "skills".

    - Claudine makes sure that all providers who share the same basic structure for agent definitions and slash commands 

1. **MCP Services**

    - MCP is a key way to provide skills to your Agent and it was in some ways a precursor to "Agent Skills" (which is not much more popular)
    - However, MCP is still relevant for a number of reasons and the ability to add MCP servers is again something that all Agent providers provide
    - Claudine will capture all your MCP configurations across all the AI agents you use and create a catalog that can be used consistently across all the agents

1. **Hooks and Events**

    - A powerful way to interact and report on your 


## Services

1. **Protect**

    - the **Protect** service will use known **regexp** patterns to identify and block tool calls which are deemed to be too dangerous as well as look for and block MCP prompt injection attempts
    - it is meant to protect but not get in the way of the user; ultimately we want the **protect** service to allow the user to have more confidence in running in YOLO mode (or at least greater permissions) because this reduces the need for human involvement and can dramatically improve speed

1. **Logging**

    - each Agentic provider has their own logging solution but Claudine wraps this into a single view


## Wrapped Execution

For all the _synchronized_ features described in the last section all you would need to use **Claudine** is run `claudine sync` once in a while to keep everything in sync between your providers. That is a starting point for what **Claudine** can do for you but to get more out of it you'll want to leverage the _wrapped execution_ features of **Claudine**:

- instead of running `claude`, `codex`, `opencode`, etc. to start your CLI put **claudine** in front:
    - `claudine claude` - executes Claude Code in wrapped execution mode
    - `claudine codex` - executes Codex in wrapped execution mode
    - `claudine opencode` - executes Opencode in wrapped execution mode
    - `claudine kimi` - executes Kimi Code CLI in wrapped execution mode
    - `claudine qwen` - executes Qwen CLI in wrapped execution mode
    - `claudine goose` - executes Goose CLI in wrapped execution mode

Ok so now you _what_ you're supposed to do and if you're good at "instruction following" (just like you expect your LLM to be) then that's all you need. Sadly, us humans have this annoying tendency to know _why_ they should do things. Well ok, princess ... here's **why** you should wrap your agent execution:

- **Consistent CLI interface**
    
    Instead of dealing with the variations in CLI parameters/switches you now have a _standardized_ set of CLI switch
    which do the same thing regardless of the agent you are using:

    ```sh
    -y, --yolo               Enable provider-specific YOLO/auto-approval mode
        --include <ENV_NAME>  Preserve this env var even when it matches sensitive-name filters
    -i, --interactive         Force interactive mode even when a prompt string is provided
        --edit                Open the prompt in an external editor before launching the provider
    -m, --model <MODEL>       Override the model used by the provider
    -o, --output <FORMAT>     Set the output format (json, text, stream)
        --asp <FILE>             Append a system prompt from a file
        --rsp <FILE>             Replace the providers system prompt with contents from a file
    -t, --timeout <SECONDS>   Timeout in seconds (non-interactive only)
        --dry-run             Show what would be executed without launching the child
    -q, --quiet              Suppress env details and info; still show the system prompt when set
        --silent              Suppress all Claudine preflight output
        --operation <OP>      Set the OPERATION env var for the wrapped session
        --sandbox             Enable provider-specific sandboxing
    ```

    > **Note:** if a more exotic feature that is not found across Agents is used, it will passed through to the agent 
    > so that you'll loose access to any features.

- **Remove Secrets from ENV**

    - Claudine will scan your host's ENV variables and remove any which appear to contain secrets
    - You don't want share secrets with agent's unless they _need_ them
    - You can override this behavior for API Keys you actually want with `--include <ENV_NAME>` 
    - Some "auto" behavior is also leveraged
        - If you are using the Qwen Agent then Qwen API Keys will be preserved
            - The same is true for Kimi Code and Codex
            - We do NOT do this for Anthropic keys with Claude Code because most people use it with a subscription and might have an API Key for other use cases. Including an API Key when claude starts makes it **skip** your subscription so we play it safe and remove it by default. You can _include_ it with `--include` if you want to

- **Simple Non-Interactive Access**

    A vast majority of developers rely on the interactive chat interface that all Agents default to when run. However, all agents also provide ways to run **non-interactive** prompts through them. Each agent platform uses a slightly different syntax and the _output_ which is provided by these non-interactive sessions can be kind of messy and hard to follow.

    Claudine makes any call which includes quoted text into a non-interactive prompt:

    - `claudine codex` - starts an _interactive_ session with Codex
    - `claudine codex 'why is the sky blue'` - starts a **_non-interactive_** session with Codex
    - `claudine codex 'why is the sky blue' -i` - starts an _interactive_ session with Codex and injects your question as the first prompt into that interactive session

    In addition to making it easy to start these interactive sessions, Claudine also improves the output quality dramatically by:

    - Calling out tool calls and their responses
    - Showing subagent invocations and closures
    - Showing _thinking_ as a distinct block
    - Showing the final output as **rendered** Markdown not just markdown syntax:
        - tables are actually tables
        - links are actually links
        - bold fonts are actually bold
        - etc.

## Composition

There are a number of powerful features which Claudine unlocks with what we're calling **Composition**. You can think of compositions as:

- a way to make static Markdown documents behave in a dynamic way and adapt to the environment they are in
- allow **safe** ways to inject the output of shell commands at runtime into your prompts
    - this can allow you to build prompts which eliminate the 
    - all shell commands which are specified in a Markdown file _must_ be approved by the user and added to the **whitelist** prior to them being used
- you can also leverage a number of dynamic "context" variables as well as any ENV variable to **interpolate** or **transclude** your prompt content
- **Interpolation**
    - most developers will recognize _interpolation_ as a means to replace parts of a "template" with real values at runtime and Claudine provides precisely that:

        ```md
        ---
        fun_fact: "Octopuses have three hearts"
        git_status: "$(git status)"
        ---
        # My Prompt

        You are in the {{ctx.repo}} monorepo and focusing on the {{ctx.current_package}} package in this review.

        ## Fun Fact
        {{fun_fact}}

        ## Agent

        You are using the {{env.AGENT}} agent on this task.

        ## Status

        The current git status is:

        {{git_status}}
        ```

        Outside of being a terrible prompt -- _what do you want from an example?_ -- what you see here is that documents can _inject_ content in at runtime. That can be static Frontmatter properties, the output of shell commands (which require approval), or leveraging the `ctx` or `env` dictionaries which provide runtime information.
- **Transclusion**
    - not only is **transclusion** a fun word to drop into a conversation it's also a power tool that all professional note taking tools include as a way to "combine" content.
    - An example might be as follows:

        ```md
        ## My Prompt

        You must do the following:

        ::file do-the-following.md

        Always remember the best practices which this repo recommends:

        ::file best-practices.md
        ```

        In this example we're using the `::file` directive to instruct Claudine that this parent markdown document will need to inject the `do-the-following.md` and `best-practices.md` content into the locations specified.

### Sequences

Claudine provides you a simple way of specifying a **sequence** either in the frontmatter of your Markdown files or as a separate YAML file.

## Communication

Now that _a lot_ if not _all_ of the actual coding is being done by AI the poor humans operating that code generation must be much more _multi-tasking_ in their work. Humans -- god knows we love them -- are great at lots of things but multi-tasking is not really one of them.

> **Pro Tip:** anyone who tells you they are "great at multi-tasking" is lying and might be a psychopath, you should look for the first opportunity to leave the conversation.

In order to help our human friends, Claudine has the ability to communicate with the user via multiple channels including:

- **TTS:** _text to speech functionality allows a user to "hear" status updates without needing to check in on the long running processes they are running_
- **Messaging Apps:** _send messages to apps like Discord, Slack, Whatsapp, etc. about status updates; this allows notifications on status to be available even on the go_
- **Desktop Notifications:** _whether you're on macOS, Windows, or Linux Claudine can tap into your desktop notification system and send message to it_
- **Sound Effects:** _in addition to using TTS for speech, Claudine also provides a sound effects library you can associated to **error**, **success**, or **human-in-the-loop** events as well as just fire them off willy nilly if that's more your style._

Here's an example of a prompt which will includes notification at various parts of the lifecycle:

```md
---
start:
    say: "We are starting to run a very exciting prompt; please fasten your seat belts"
success:
    say: "Wow that was amazing. You are so successful."
    effect: crowd-applause
failure:
    say: "Wow you are such a loser! The {{ctx.current_package}} just couldn't handle the prompt!"
    effect: cartoon-cry
    message: "The {{ctx.current_package}} just couldn't handle the prompt!"
---

Think hard and do something amazing ... but do it really fast.
```

This built-in functionality can be now be embedded into your prompts and allow your humans to live a slightly better life than they did before.


## Scheduling

Not yet ready but will be added soon.

## Local Models

Not yet ready but will be added soon. You can use local models today but you'd need to do the configuration yourself in the Agent platform.

## Worktree Integration

Not yet ready but will be added soon.

