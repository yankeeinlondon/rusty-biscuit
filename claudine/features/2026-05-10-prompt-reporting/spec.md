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
                    - `The system prompt was **{action}**; the content was _composed_ from <a href={absolute-path-to-prompt}>{relative-path-to-prompt}</a>. {token-message}`
                    - where `{action}` is:
                        - `appended to`
                        - `replaced`
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
    - The **BODY** is rendered as BlockQuote with a orange vertical line
        - the vertical line should be a centered line which aligns with the center of the icon found in the first line
        - the content of prompt will be rendered for the terminal using the Darkmatter library to ensure that Markdown content is represented in a user friendly way.
        - **Markdown rendering constraint:** rendered output must never contain more than two consecutive blank lines. Any larger gaps must be collapsed to at most two blank lines.
    - If no CLI switches, ENV variables, or Frontmatter hints are found the default behavior for the system prompt is to only render the **Summary** view in the body.

### User Prompt

The agent prompt has an infinite number of variants. This is in contrast to a system prompt which tends to remain effectively the same across a repo (or across a package/package-area of a monorepo). Because of this increased variance there is slightly greater reason to report the action prompt rather than just a summary but structurally and semantically the System Prompt and User Prompt have more similarities than differences.

The User Prompt uses a simpler reporting model than the System Prompt:

- **NO Summary mode** for User Prompt
- **NO `CLAUDINE_USER_PROMPT`** environment variable
- **NO frontmatter verbosity support** for User Prompt
- The User Prompt header (`🗣️ Agent Prompt`) is shown by default
- `--quiet` suppresses the User Prompt **ENTIRELY** (both header and body)
- `--silent` suppresses everything
- The body is driven by length and the `--verbose` override:
    - by default, the body is shown in full if it is 40 lines or fewer
    - when content surpasses 40 lines, the body uses `FrontBack` truncation (first 20 lines, then an `hr` marker, then the last 10 lines)
    - `--verbose` forces the full body to be shown regardless of length
    - The body of a User Prompt is rendered as a BlockQuote with a **green** vertical line at the left
        - all leading whitespace should be removed in all cases
        - it's important that FrontBack strategy _not_ have a blank line at the first or last line of its output; when that happens, advance that section (front or back) by one line to get to a valid condition
        - the same markdown rendering constraint applies: output must never contain more than two consecutive blank lines

> **Note:** The User Prompt section may need continuation beyond this point.
