# Shell Expansion

We allow the output of shell commands to be injected into a Markdown page using the syntax:

```md
::shell <command> <params>
```

## Pre-Flight Approval

All shell commands must pass through Darkmatter's pre-flight security check before execution. This covers blacklist validation, whitelist lookup, and interactive approval. See [Pre-Flight Shell Approval](../topics/pre-flight-checks.md) for the full policy, blacklisted commands list, whitelist/blacklist file locations, and interactive approval options.

## Execution Behavior

When the Darkmatter compose pipeline reaches the Shell Expansion stage, it will iterate over all `::shell` lines and:

- if the command does not exist on the host system we exit the pipeline in error
    - `<red><b>ERROR:</b></red> the shell command '{command}' does not exist on this host but was referenced in a shell expansion operation during the <b>compose</b> pipeline in <blue>{file}</blue>!`
- if the command matches the blacklisted commands and syntax we exit in error
    - `<red><b>ERROR:</b></red> the shell command '{command}' is not allowed as a shell expansion command in Darkmatter's compose pipeline! This command is considered a globally blacklisted command.`
- if the command does not exist in the repo's (or user's) whitelist file the user will be asked to approve this command (see [Pre-Flight Shell Approval](../topics/pre-flight-checks.md))
- if the command _does_ exist in the whitelist then we execute the command and both STDOUT and STDERR are captured and added to the Markdown document in place of the `::shell` instruction.

    - If a command does not complete in 10 seconds (by default) then we will exit with an error
        - `<red><b>ERROR:</b></red> the shell command '{command}' in {file} took too long to complete (10 seconds) and was terminated. The Darkmatter pipeline has exited.`
    - The timeout can be changed with `--timeout <seconds>` in the CLI or `ComposeOptions::with_shell_timeout()` in the library.
    - If timeout fallback is enabled via `--allow-shell-timeout` or `ComposeOptions::with_allow_shell_timeout(true)`, a timed out command is replaced with an empty string and compose emits a warning instead of failing.
    - If the command outputs nothing in STDOUT or STDERR while returning a 0 exit code (aka, no error) then we simply remove the `::shell` instruction line.
    - If the shell command's exit code is _not_ 0 (aka, there was an error when running the command) then we will exit the pipeline with an error:
        - `<red><b>ERROR:</b></red> the shell command '{command}' in {file} exited with an error code of {error_code}. The Darkmatter pipeline has exited.\n\n<b>STDOUT:</b> {stdout}\n\n<b>STDERR:</b>{stderr}`

## Frontmatter Variant

Darkmatter also supports shell expansion in top-level frontmatter string values using `$(...)`.

- Documentation: [Frontmatter Shell Expansion](./fm-shell-expansion.md)
- Frontmatter shell expansion stores trimmed `stdout` only
- Body `::shell` expansion stores combined `stdout` + `stderr`
- Both variants share the same policy, whitelist/blacklist, approval, and timeout infrastructure


## Handling Error Exit Codes

Sometimes we'll want to run a shell expansion command that _can_ return an error code. We do that in several distinct ways:

1. Default Error handler(`--when-error <string>`)

    - this manner of handling will ensure all error exit codes will not result in an error but instead with the text in the parameter provided by this switch

2. Handle Specific Error Codes(`--when-exit-code <#> <string>`, `--except-exit-code <#> <string>`)

    - Allows you to respond to only a specific error code `--when-exit-code`, or
    - Respond to all exit code except one `--except-exit-code`

3. Handle Based on STDERR(`--stderr-contains <string:find> <string:replace>`, `--stderr-lacks <string:find> <string:replace>`)

    - Allows you to to handle errors based on the content found in STDERR

4. Enrich Error Message (`--enrich-error <string>`, `--enrich-error-on <#> <string>`)

    - Allows you to enrich the error message which will be presented to the user if this command fails
    - The string provided will be passed through the `Prose` struct to allow users to use terminal escape code easily

--- 

> Return to [Darkmatter Pipeline](../darkmatter-compose-pipeline.md)
