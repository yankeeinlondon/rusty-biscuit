This feature will focus no cleanup of the Claudine CLI:

## Removals

- remove the `dry-run` subcommand
- remove the `about` subcommand

## Help System Tidy Up

- remove "help" from the help system's menu (the subcommand should remain but we don't need to see it in the help system)
- remove "handle" from the help system. It's an important command and obviously needs to exist but it is not user facing and should not be in the help system's reporting
- reorganize:
    - To make the commands make more sense we're going reorganize the "Commands:" section into groups
    - Shared Resource:
        - skills
        - commands
        - agents
        - mcp
        - hooks
    - Wrapped Execution:
        - claude
        - codex
        - gemini
        - goose
        - kimi
        - opencode
        - qwen
    - Composition
        - compose
        - compose-inline (this is currently a subcommand of compose, but it should be it's own command)
        - sequence (FUTURE)
    - Administration: 
        - init
        - sync
        - uninstall
- Use the `Renderable` components from `biscuit-terminal` library to polish the look and feel of the help system
- review the shell completions for every subcommand and try to identify ways to improve them
    - one common pattern which tends to add value is finding a parameter which can be enumerated into a set of valid/suggested values.

