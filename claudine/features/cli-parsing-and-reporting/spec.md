# Claudine's CLI Parsing and Claudine Badges

I feel like we need to be more explicit about how the claudine CLI's Agent wrapper commands (e.g., claude, codex, gemini, opencode, qwen, kimi, goose) are parsing the parameters and CLI switches the user is providing.

I also think we should apply the same level of explicitness to how we badge and display the Claudine execution line (e.g., `Claudine ▸ Gemini  Non-Interactive   Verbose  --prompt 'Why is the sky blue?'`).

## Parsing

Claudine provides a set of CLI switches which largely fit into two buckets:

1. a concept/feature like **YOLO mode** which is shared across most/all Agents but where many agents use different nomenclature for the same thing. Here are those which fit this category:

    - `--yolo`, `-y`
    - `--model`, `-m`
    - `--system-prompt`, `-s`
    - `--sandbox`

2. an additional feature not provided by the Agents themselves. Example include:

    - `--verbose`, `-v`
    - `--include <ENV_NAME>`
    - `--silent` and `--quiet`
    - composition features:
        - `--compose <FILE>`
        - `--frontmatter-prompt <FILE>`
        - `--prompt-file <FILE>`
    - `--operation <OP>`
    - `--mcp`, `--use <ID>`, `--strict`

**BUSINESS RULES:**

- Parameters and CLI switches should be able to appear in _any_ order and have no impact on outcome
- The ONLY non-switch based parameter is the prompt string
    - Prompt strings can be used with both interactive and non-interactive sessions
- The Claudine CLI switches are always evaluated first and NEVER passed onto a provider directly
    - Often a Claudine CLI will go through a simple adapter pattern to ensure the call to the provider uses the right nomenclature for that provider
    - In cases where the client has asked for a particular behavior or feature with a Claudine CLI switch and the underlying provider doesn't support that feature then we'll add a `Info: {msg}` list item after the claudine execution report line.
- Any remaining switches which yet to be consumed as Claudine switches or a parameter will be passed down to the provider "as is"

**KEY CHANGES:**

- Current State:
    - Up until now we've had a default assumption that the session will be interactive _unless_ the user provides the `--non-interactive`, `-n` switch stating that they want the session to be non-interactive.
    - Composition switches like `--compose`, `--frontmatter-prompt`, and `--prompt-file` were also treated as a proxy for a non-interactive session
- New Logic:
    - We will remove the `--non-interactive`, `-n` switch
    - We will add a `--interactive`, `-i` switch which _explicitly_ states the desire to use an interactive session
    - When the user **does not** provide a prompt string, then we will also start as an interactive session
    - However, when a user **does** provide a prompt string, the default switches to a non-interactive session
        - if the user wants to provide a "startup prompt" to an interactive session then this is where they would use the `--interactive`/`-i` switch.
    - The composable switches -- `--compose`, `--frontmatter-prompt`, and `--prompt-file` -- will also trigger the default session type to be non-interactive but using the `--interactive`/`-i` switch will allow these composition transactions to be sent into an interactive session as the first prompt in that session.

## Reporting

### Claudine Execution Line

The line starting with `Claudine ▸ {agent}` is what we'll refer to as the "execution line".

- if a user provides `--silent` then this execution line is not reported, in all other cases it is
- when the execution line is visible it is printed to STDERR and includes a blank line before and after
- the format of the execution line is `<b><blue-500>Claudine</blue-500> ▸ {agent} </b>{badges} <dim>{prompt}</dim>`
- CHANGES:
    - today we tend to drop in vendor specific CLI switches like `--print` or `--prompt` or `--dangerously-skip-permissions`
    - none of those should be written out to the execution line
    - the `{prompt}` is only shown if there IS a prompt and we will truncate it with a trailing ellipsis to keep reporting to one line`
        - to do this effectively we do need to look for `\n` in the prompt text and show them as `\n` and **not** actually render the new line.
