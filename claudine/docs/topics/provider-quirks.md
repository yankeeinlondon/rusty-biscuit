# Provider Quirks

## OpenCode

- When you run a non-interactive session you must explicitly state the **model** you want to use rather than it falling back to a "default" model like it does in interactive sessions; this is not true if you have explicitly defined the default model in `~/.config/opencode/config.json`
    - Note: in interactive sessions the "default model" is the last model you used
    - If the `~/.config/opencode/config.json` file defines a "model" property, this model will become the default model for opencode 
    - The model in `~/.config/opencode/config.json` can be overriden by the CLI using `--model <model>`

## Kimi Code

- provides a "raw mode" which provides a direct interface to the agent, even lower level then ACP


