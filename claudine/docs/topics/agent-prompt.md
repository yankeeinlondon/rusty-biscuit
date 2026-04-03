---
blast_radius:
---
# Agent Prompts

When we run a `compose` or `inline-compose` operation we are abstracted from the actual prompt (as it's the _composition_ of the file reference). For that reason it's important to show the user the prompt (or at least part of it) before we hand off to the Agent. This gives the caller context/reminder/validation of what this task is doing before what might be a long running agent process is kicked off.

Some reusable prompts can be quite long and unless the user states that they always want to see the entire prompt by providing the `-v` / `--verbose` flag, we will truncate it after a "reasonable" number lines.

## Truncation Policy

By default we'll aim to truncate the prompt at 15 lines, however, if the prompt is less than 20 we'll show the entire prompt. Beyond this base rule, there are a few variants to be aware of:

- the last line(s) we display of a prompt should NEVER be a blank line. this can lead to the user seeing multiple blank lines and then truncation which immediately raises the question for people that "maybe the rest of the prompt was cut off"
- if the 15th line is in the middle of a Markdown table then we should complete the table before truncating

## Formatting

The Agent prompt should look like:

- `<b>Agent Prompt:</b>\n`
- and now the Agent Prompt goes here
- the agent prompt is all wrapped in `BlockQuote` with a left margin of 2 and a wide green block character used for the vertical line
- after the block quoted agent prompt, we add a blank line
- then, if we did truncate, we'll add `<dim>- remaining prompt truncated for brevity, use <blue>--verbose</blue> to show entire prompt</dim>`

> **Note:** typically -- at least in non-interactive prompts -- we will then render the Agent's session ID. Whether we or not is not the point but instead we should be aware that whatever is next may be considered in the same "section" as our truncation message so we should not add blank line after our truncation message. In contrast, if there was no truncation then we DO add a blank line after the agent prompt because this is considered the end of the section.
