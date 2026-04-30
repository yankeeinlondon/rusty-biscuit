# Getting Started with Claudine
![claudine](../../../assets/claudine-512.png)
> Claude Code's ex-girlfriend who knows Claude's inner secretes but is now dating other Agents

## Intro

**Claudine** was born out of the desire to be able to move fluidly between different agentic CLI's without having to go through a deep learning curve for each. With these humble beginnings we **Claudine** has become a full featured "meta-agent" which allows users to treat Markdown as a _compositional_ primitive for building prompts all the way to long running multi-agent workflows.

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

## Creating a Level Playing Field

**Claudine**'s original purpose was to create a platform that allows _use_ of many of the most popular agentic CLI's without requiring a mastery of each one. Let's break down how **Claudine** achieves this goal:

1. **Agent Skills**

    - the new champion of providing _expertise_ to your Agent is the **agent skill**
    - introduced by Claude Code and very quickly adopted by everyone as a pseudo standard is a nice way of giving your agent contextual super powers
    - Claudine makes sure that your agent skills (both user-scoped and repo-scoped) are available to all of the agent platforms we support (and are installed on your system)
    - run `claudine skills` for an overview of your catalog
        - run `claudine skills --fix` to fix synchronization issues that may have built up (alternatively run `claudine sync` which syncs skills, agents, and slash commands)

1. **Agent Definitions** _and_ **Slash Commands**

    - The idea of providing a definition for an "agent" or "subagent" is also a concept that all agents share
    - The way the various agent platforms implement this 

1. **MCP Services**

1. **Hooks and Events**


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

    A vast majority of developers rely on the interactive chat interface that an 

### Leveling up with Composition

**Claudine** leverages the **Darkmatter** library to provide 1st class _composition_ features. What is composition you say? Composition in this context means:

- 
