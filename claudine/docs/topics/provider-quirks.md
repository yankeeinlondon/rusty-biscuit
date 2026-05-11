# Provider Quirks

## OpenCode

- When you run a non-interactive session you must explicitly state the **model** you want to use rather than it falling back to a "default" model like it does in interactive sessions; this is not true if you have explicitly defined the default model in `~/.config/opencode/config.json`
    - Note: in interactive sessions the "default model" is the last model you used
    - If the `~/.config/opencode/config.json` file defines a "model" property, this model will become the default model for opencode 
    - The model in `~/.config/opencode/config.json` can be overriden by the CLI using `--model <model>`
- if OPENCODE_CONFIG_DIR is set then OpenCode will also search that custom directory for agents, commands, modes, and plugins.
    - this is additive/merged, does not replace normal directories
- Agent/Subagent
    - Primary agents are the agent you start with (like `~/.config/opencode.json`); I believe they are more likely to be configured in JSON but apparently can also be configured in 
        - pressing "tab" in an interactive session will switch you between agents
    - OpenCode subagents can be defined in Markdown files or in JSON config files 
    - The `mode` property of either `primary` or `subagent` is considered a best practice though 
    - OpenCode comes with two built-in agents (build, plan) and two built-in subagents (general, explore)

## Codex

- `codex exec --json` emits more than one terminal error event for a single underlying failure. For example, a ChatGPT usage-limit hit typically produces a `turn.failed` event **and** a top-level `error` event, both carrying the same resolved kind/message (`rate_limit` / "You've hit your usage limit..."). `CodexSemanticStreamParser::handle_error` de-duplicates by (kind, message) so the live stderr surface renders one `Agent Error` block per distinct failure. `SessionStart`/auth failures and billing errors behave the same way.
- The exec stream does **not** expose model name, auth mode, cost basis, or ChatGPT rate-limit percentages as stable documented fields — rate-limit classification relies on text matching in the error message.

## Kimi Code

- provides a "raw mode" which provides a direct interface to the agent, even lower level then ACP


