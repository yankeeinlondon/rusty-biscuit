# Agent Selection Feature

When we use commands like `claudine compose`, `claudine inline-compose`, and `claudine sequence` an often overlooked feature is
that we can defer the decision on which agent to use until later. It is true that there are flags like `--claude`, `--opencode`, etc. which move away from this lazy selection of agent. Both modes have their strengths but currently the lazy mode is operating incorrectly and subsequently it is loosing a lot of it's functionality.

When a user runs a claudine command that _composes_ a prompt without a `--codex`/`--claude`/etc. flag specifying the agent to
use we should default to providing an interactive select box for the user to choose which agent provider they'd like to use
at runtime.

This selection box will:

- show only those agents which the host computer has installed
- the user's "favorite" agent will be selected as the default in the select control
- the user can change their selection by using the up/down arrow keys to move between the available options

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

## Frontmatter Influence

### The `agent` property

If the prompt being referenced has defined the `agent` frontmatter property as a string, and that string can be
mapped to a supported Agent then this Agent will override the user's "favorite" to become the initially selected
Agent to use (selected not chosen). Note, if the host computer does not have the suggested Agent then the frontmatter
suggestion if ignored and we go back to the user's favorite agent being selected first.

If the page's `agent` frontmatter is a list of Agents then we will:

- put all of the suggested Agents at the top of the select list (ignoring those not installed on host)
- the first selection which is installed on the host becomes the initially selected item

### The `model` property

- If the ENV variable for the chosen agent is set (not `MODEL` but the specific ENV variables) then that will be used over any suggestions coming from the frontmatter.
- The CLI also provides a `--model <model>` CLI switch which will always be given highest priority for model selection; but note that this CLI switch is typically paired with an explicit agent like `--codex`, etc.
- if, however, these other stronger hints at model selection have not been used then the `model` property in the frontmatter can provide influence
- similarly to the `agent` frontmatter, we will accept either a singular string value or a list of string values:
    - if we find a suggested model which is a match for a valid model of the `agent` being used then we will use this suggestion
    - invalid model's for the chosen `agent` are ignored/skipped but do not create an error condition
        - this does mean, however, that we need an enumerated list of valid models per provider

## Timing

When Claudine is running non-interactive prompts, it's particularly important that all user required interactivity be
front-loaded so that the user can address any required questions initially and then leave this session to execute to
completion (versus be paused to wait for HITL interaction mid-stream).

For `claudine compose` and `claudine inline-compose` this is really not an issue as the question of which **agent** to use
will only need to be asked once. For `claudine sequence` it requires a little more consideration.

- the key thing to remember with a sequence is that the `agent` we will use for every state in the sequence needs to be defined. Using _different_ agents or models for different steps is a very common and desirable outcome.
- the hinting we can provide in the Frontmatter should help shape the "defaults" for each state but the user must still sign-off on the overall configuration
