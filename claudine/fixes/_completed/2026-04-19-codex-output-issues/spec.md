---
fixed: 2026-04-19
agent: codex
---

It seems the output for claudine's non-interactive sessions with Codex CLI as the agent:

- there was TONS of text being reported ... a lot of code blocks
- it did complete successfully
- it is reporting "thinking token" well
- it is reporting the final output well though it had multiple blank lines between the final output and the metadata line(s)

Here are some example sections I saw:

````sh
 → Zsh(-lc "sed -n '1,260p' claudine/fixes/2026-04-19-claude-vertical-spacing/spec.md")
 ← Zsh(successful, The claudine CLI does a pretty good job of returning data from a non-interactive session with Claude Code but
    it's not perfect. The move obvious issue is the introduction of unexpected blank lines in the output.

    - we DO want a blank line between different sections but otherwise we do not and we NEVER want multiple blank lines
        - `remaining prompt truncated ...` and `Claude session ID ...` are the same section and should not have a blank line
    between them
        - blank lines between a series of tool calls never makes sense
    - the line in the example starting with: `Let me investigate the spacing issue in the live semantic sink and related code. Now
    let me look at how thinking/reasoning is` is a new section and I believe this section is part of the "thinking tokens" and
    thinking tokens are supposed to be rendered with BlockQuote with a gray vertical line to demarcate the thinking text. The
    thinking tokens should have a blank line before and after.

    ## Example

    Here's a recently example of what I got:

    ```sh
    - remaining prompt truncated for brevity, use --verbose to show entire prompt

    - Claude session ID 9fd1d072-a68 · claude-opus-4-7[1m]
````

- in this example it looks like it called `zsh sed -n ...` and the next line looks like it's reporting the success of that command. Is the remaining text the STDOUT from that command? If that's what it is then we probably do want to report it but this format is messy if we're going to get large blobs of text like this.
    - My diagnose might be wrong but if it's right then the successful results should be rendered as:
        - First line: `Zsh(<dim><green><i>successful</i></green></dim>)`
        - Then:
            - if the user used the `--quiet` CLI flag then that it, nothing more needed
            - if the user didn't use the `--quiet` CLI flag then:
                - use the BlockQuote struct, with purple vertical bar, and text should all be rendered in grey text color
                - if the text should be rendered with Prose struct and I think we turn off word-wrapping (with truncation)
                - if the text surpasses 10 lines then we should stop rendering at that point and add this immediately below (no blank space):
                    - BlockQuote with orange vertical bar and text of `<b>tool call</b>'s response truncated for brevity`

The more I look at it the more sure this is indeed that we're getting the tool call's response and that is a good thing but we just need to make sure format it better (as described above).
