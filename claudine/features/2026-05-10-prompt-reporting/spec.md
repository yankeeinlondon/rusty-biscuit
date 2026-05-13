# Prompt Reporting

## Context

The **System Prompt** and **User Prompt** are both critical instructions which an Agent platform uses when executing a session and Claudine reports on them (by default) like this currently:

```sh
System Prompt(appended):
  ▌ ██ Context
  ▌
  ▌ - you are working in the rusty-biscuit monorepo
  ▌ - this session was started with a focus on the claudine package area
  ▌     - you must use the 'claudine' agent skill
  ▌
  ▌
  ▌ ██ Best Practices
  ▌
  ▌ - when rendering to the terminal use the biscuit-terminal and darkmatter skills!
  ▌     - leverage the Prose struct from biscuit-terminal for rich text (color, style), hyperlinks (OS8), and more
  ▌
  ▌ - when attempting to do host discovery (hardware, software, os, file-system, repo/git) you should use the sniff skill
  ▌ - when doing file conversions between JSON, YAML, TOML always use the biscuit-file skill
  ▌ - whenever you are attempt to convert a string based file reference to a real file path in the filesystem you should use
  ▌   FileReference struct from biscuit-file and use the biscuit-file skill
  ▌
  ▌ ██ Non-Interactive Prompt
  ▌
  ▌ You are running in a non-interactive session — the user cannot answer prompts for feedback or permissions.
  ▌
  ▌ - If blocked (tool call denied, file unreadable, etc.): tell the user, then find an alternative way to get the information.
  ▌ - If truly unable to complete the task: report what happened, note how to avoid it going forward, and exit with an error code.
  ▌ - Never run commands that need an interactive terminal or stdin follow-up (REPLs, editors, pagers, prompts, anything waiting
  ▌   on input).
  ▌ - Prefer one-shot commands with explicit non-interactive flags; if an approach would require sending more input to a running
  ▌   command, pick a different approach.
  ▌
  ▌ ████ Credential and signing blockers (this is very likely why you're hanging)
  ▌
  ▌ - Git commits with GPG/SSH signing enabled will block waiting for a passphrase via gpg-agent / ssh-agent / pinentry. Before
  ▌   running git commit in a non-interactive session, assume the passphrase is NOT cached. If a commit appears to hang, abort the
  ▌   command, report it, and do not retry.
  ▌ - Never run gpg, ssh, ssh-add, sudo, op signin, aws sso login, gh auth login, docker login, npm login, or any credential
  ▌   helper that may prompt — even if stdin is closed, many of these open /dev/tty directly.
  ▌ - If a shell command does not complete within ~60s, treat it as stuck and abandon that approach; do not wait longer and do not
  ▌   assume progress.
  ▌
  ▌ ████ Sub-agent propagation
  ▌
  ▌ - When spawning sub-agents (Task tool or equivalent), always repeat in their brief that the entire session tree is
  ▌   non-interactive. Sub-agents inherit nothing from this system prompt unless you include it.
  ▌ - Give each sub-agent a narrow, bounded task. Never dispatch a sub-agent that depends on waiting for another sub-agent's
  ▌   output unless you are acting as an explicit orchestrator and can collect results yourself.
  ▌
  ▌ ████ Subprocess hygiene
  ▌
  ▌ - Export GIT_TERMINAL_PROMPT=0 before running any git command that could prompt for credentials.
  ▌ - Prefer command forms that declare non-interactivity explicitly: apt-get -y, brew --quiet, npm install --no-audit --no-fund,
  ▌   cargo --color=never.
  ▌ - Never run a command in a background shell with &; if you need parallelism, use the framework's concurrent tool-call feature
  ▌   instead.Progress reporting (fights the silence-looks-like-hang problem)
  ▌ - Emit a short status message to the user at least once every ~30 seconds of work. If you are about to start a long-running
  ▌   tool call, say so first.

Agent Prompt:
  ▌ You are a planning agent. Convert the following documents into a high confidence execution plan:
  ▌
  ▌ - Functional Specification: claudine/features/_unscheduled/1-remove-agent-capabilities/spec.md
  ▌
  ▌ ██ Requirements
  ▌
  ▌ - Break work into phases and tasks
  ▌ - Order tasks by dependency
  ▌ - Flag parallelizable work
  ▌ - Include validation checkpoints

- remaining prompt truncated for brevity, use --verbose to show entire prompt
```

## Changes

In this release our focus will be on how we're going to _change_ how we report these two prompts:


### System Prompt

- **Line 1:** 
    - `Prose::new(println!("{icon} <b>System Prompt(<dim><i>{action}</i></dim>)</b>", icon, action))`
    - where `icon` is:
        - `📕`
    - where `action` is 'appended' | 'replaced'
    - this is shown whenever the resolved body mode is anything other than `Silent`
    - the default condition (when no CLI flag, ENV variable, or frontmatter has selected a body mode) is to show Line 1 when the system prompt has changed or when the caller has used the `verbose` flag
    - CLI switches, ENV variables, and frontmatter can override the default and force Line 1 to appear even when the prompt is unchanged
    - the `--silent` flag always resolves the body mode to `Silent`, so Line 1 is NEVER shown when `--silent` is used
- **Body:**
    - Line 1 is shown whenever the resolved body mode is not Silent, and the body is only shown when Line 1 is shown
    - what is shown in the "body" section is based CLI flags and other conditions used but these variants exist:
        - **Summary**
            - The system prompt can often be quite long and the user might like _seeing_ it at first but it can become quite repetitive so we will often choose instead prefer the "summary" mode of reporting
            - The summary presents:
                - if there is a system prompt adjustment:
                    - `The system prompt was **{action}**; the content was _composed_ from <a href={absolute-path-to-prompt}>{display-label}</a>. {token-message}`
                    - where `{action}` is:
                        - `appended to`
                        - `replaced`
                    - where `{display-label}` is resolved in this order:
                        1. **Nerd Font glyph as repo-root prefix (in-repo only):** when the terminal reports Nerd Font support (`Terminal::is_nerd_font == Some(true)`) **and** the prompt file resolves inside the supplied base directory, render the visible label as **`\u{f02a2}/{relative-path}`** — the Nerd Font glyph (codepoint `f02a2`) substitutes for the repo-root `.` so the reader gets a strong visual cue that the path is relative to the repo, followed by the path-from-base (e.g., `\u{f02a2}/system-prompt.md`, `\u{f02a2}/.claude/system-prompt.md`). The absolute path is carried by the OSC8 target.
                        2. **Relative path with `./` prefix:** otherwise, when the prompt file resolves inside the supplied base directory, render the relative path prefixed with `./` (e.g., `./system-prompt.md`, `./.claude/system-prompt.md`, `./prompts/agents/foo.md`). The path is dynamic — any subdirectory depth is supported; only the `./` prefix and the path-from-base are guaranteed.
                        3. **Absolute path:** when the prompt file is outside the base or no base was supplied, render the absolute path (no `./` prefix).
                    - **Hyperlink styling:** the visible label is rendered in **blue** (`Tailwind::Blue400`) so the reader gets a visual hint that it is a link. This applies to every `{display-label}` variant above, including the Nerd Font glyph. The OSC8 target is always the absolute `file://` URL. On terminals without OSC8 support, the blue-styled label still renders as plain text (and if Nerd Fonts are also unsupported, variant 1 falls through to variant 2 or 3).
                    - where `{token-message}` is:
                        - token estimation uses **biscuit-terminal's FileTree utility** (not a simple character-count heuristic)
                        - the token count measures the **composed system-prompt.md content** (the portion Claudine has access to)
                        - **Limitation:** Claudine cannot measure the agent platform's original/default system prompt. The reported count reflects only what was composed from the `system-prompt.md` file and any appendix.
                        - if the action is "appended to" then `The composed system prompt is roughly {#} tokens.` (this reflects the total composed size, not only the delta)
                        - if the action is "replaced" then `The replacement system prompt is roughly {#} tokens.`
        - **Partial Prompt**
            - Prompts can be quite long and reporting the whole prompt may be seeing as polluting the output section
            - A partial prompt shows either:
                - Truncate: prompt start (up to fixed number of rows), then truncated
                - FrontBack: prompt start (up to a fixed number of rows), then a `hr` marker and then a fixed number of last/trailing lines of the prompt
        - **Full Prompt**
            - Sometimes it is not desirable to succinct and people want to see the whole prompt (that's what we do currently)
            - The full prompt renders the full prompt as markdown formatted content.
    - The format of reporting for the BODY is determined by (in order of precedence):
        - **CLI Switches**
            - use of the `--verbose`/`-v` flag will always show the **Summary** information and then the **Full Prompt**
            - use of the `--quiet` flag will ensure that _only_ the **Summary** information is shown
            - use of the `--silent` flag will never show anything regarding the system prompt
        - **CLAUDINE_SYSTEM_PROMPT**
            - if the environment variables is set to any of the following values then the reporting style will be set with only the CLI switches having the ability to override
            - recognized values (capitalization is ignored, all values evaluated as their lowercase variant):
                - `verbose`
                - `quiet`
                - `silent`
        - **Prompt Length**
            - if the prompt's length is less than 10 lines then we will _never_ use a Partial Prompt, favoring a **Full Prompt** whenever a prompt is shown
        - **Frontmatter** in the targeted `system-prompt.md` file (aka, the content which will be composed to create the system prompt)
            - the `verbosity` Frontmatter property on a `system-prompt.md` file -- if set -- can take one of the following values: 
                - `verbose`
                    - suggests that this system prompt should report the Summary and the Full Prompt 
                - `quiet` 
                    - suggests that only the Summary Report should be presented 
                - `silent`
                    - suggests that nothing is reported
    - The **BODY** is rendered as a single `biscuit_terminal::components::BlockQuote` with an **orange** vertical line.
        - **One BlockQuote covers everything below the header.** The Summary sentence (when shown) and any Partial/Full prompt content (when shown) live inside the **same** BlockQuote so the orange bar runs continuously from below the icon to the end of the body. The Summary sentence must never be emitted as a bare `Prose` line.
        - **Border glyph and alignment:** the BlockQuote uses the **heavy** box-drawing vertical `┃` (U+2503) — the thickest box-drawing vertical that still occupies a single, horizontally-centered cell column. `left_margin = 0` so the glyph lands at column 0, directly under the left edge of the 2-cell-wide 📕 emoji on the header line above; a non-zero left margin would push the bar one column right of the icon and break the visual "the bar terminates at the icon" effect.
        - **Body width:** content is rendered by `darkmatter` at `max_width = term.width() - (left_margin + border_width)` so it fits the BlockQuote's child area without needing further wrap. The BlockQuote itself uses `WordWrap::None` so the pre-wrapped, ANSI-rendered content is not re-wrapped (which would chop trailing color resets and break list/block-quote indentation produced by darkmatter).
        - the content of the prompt is rendered for the terminal using the `darkmatter` library so Markdown is represented in a user-friendly way.
        - **Markdown horizontal rules** (e.g. the truncation `---` separator) are rendered by darkmatter using `biscuit_terminal::components::HorizontalRule`. When the **outer** prompt-reporting terminal advertises Kitty or iTerm2 image support, `render_markdown_for_terminal` passes `TerminalImageMode::Force` to darkmatter so the Tier 1 SVG→PNG image is emitted even when darkmatter's own internal capability detection (which runs in a subprocess-style builder context) loses Kitty support. Otherwise `TerminalImageMode::Auto` is used so Unicode/ASCII fallbacks still apply. `TerminalImageMode::Never` must not be used — it suppresses the image HR.
        - **Markdown rendering constraint:** rendered output must never contain more than **one** consecutive blank line. darkmatter's `HorizontalRule` writer emits `<rule>\n\n` and the following paragraph contributes its own leading `\n`, so without this cap a rule would be followed by two visible blank rows instead of the expected single separator.
    - If no CLI switches, ENV variables, or Frontmatter hints are found the default behavior for the system prompt is to only render the **Summary** view in the body.

### User Prompt

The agent prompt has an infinite number of variants. This is in contrast to a system prompt which tends to remain effectively the same across a repo (or across a package/package-area of a monorepo). Because of this increased variance there is slightly greater reason to report the action prompt rather than just a summary but structurally and semantically the System Prompt and User Prompt have more similarities than differences.

The User Prompt uses a simpler reporting model than the System Prompt:

- **NO Summary mode** for User Prompt
- **NO `CLAUDINE_USER_PROMPT`** environment variable
- **NO frontmatter verbosity support** for User Prompt
- The User Prompt header (`🗣️ Agent Prompt`) is shown by default
- `--quiet` **does NOT suppress the User Prompt.** It is a no-op for the Agent Prompt; both the header and the body still render. (`--quiet` is a System-Prompt-only control.)
- `--silent` suppresses everything (header and body).
- The body is driven by length and the `--verbose` override:
    - by default, the body is shown in full if it is 40 lines or fewer
    - when content surpasses 40 lines, the body uses `FrontBack` truncation (first 20 lines, then an `hr` marker, then the last 10 lines)
    - `--verbose` forces the full body to be shown regardless of length
    - The body of a User Prompt is rendered as a single `biscuit_terminal::components::BlockQuote` with a **green** vertical line.
        - **Border glyph and alignment:** same contract as the System Prompt body — the BlockQuote uses the heavy box-drawing vertical `┃` (U+2503) with `left_margin = 0` so the bar lands at column 0, directly under the left edge of the 2-cell-wide 🗣️ emoji on the header line above.
        - **Body width:** content is rendered by `darkmatter` at `max_width = term.width() - (left_margin + border_width)` so it fits the BlockQuote's child area without needing further wrap. The BlockQuote uses `WordWrap::None` so pre-wrapped content is not re-wrapped.
        - **Markdown horizontal rules** (including the truncation `---` separator) render via Kitty graphics on capable terminals and fall back to Unicode/ASCII otherwise (image rendering must stay enabled for the User Prompt body).
        - all leading whitespace should be removed in all cases
        - it's important that FrontBack strategy _not_ have a blank line at the first or last line of its output; when that happens, advance that section (front or back) by one line to get to a valid condition
        - the same markdown rendering constraint applies: output must never contain more than two consecutive blank lines
