When we start non-interactive sessions with Claudine we present the _composed_ Agent prompt. Now that we've refactored the system-prompt so that it too get's composition super-powers we need to present the system prompt in a similar way. However, unlike the Agent Prompt, the System Prompt should be presented for both interactive and non-interactive sessions.

## Presentation

The look of the System Prompt should be nearly identical to how we render/show the Agent Prompt, with these exceptions:

- instead of `<b>Agent Prompt:</b>` we will use `<b>System Prompt(<dim><i>{variant}</i></dim>)</b>` where "variant" refers to whether the system prompt is "appended" or "replaced".
- We use the `BlockQuote` struct to render a green vertical bar to the left of the prompt when we're displaying the Agent Prompt. We want to do the same thing with the System Prompt except that it should be orange.
    - Note: we will use the same block character used in Agent Prompt
- We should only truncate the system prompt after 25 lines.
- If we are _not_ modifying the system prompt at all we can just leave off all reporting under most cases but if the `--verbose`/`-v` mode is used then we should report a one line BlockQuote (orange vertical, same as above) which says `the system prompt has not been modified`

## Ordering

- Claudine Execution Line
- ENV Variables (unless `--quiet` or `--silent`)
- System Prompt
- Agent Prompt
- ...


