## Context

- you are working in the **Sniff** package area inside the **rusty-biscuit** monorepo
- you should use the 'sniff' agent skill
- the Sniff library and CLI are intended to work on macOS, Linux, and Windows

## Best Practices

- when working with the sniff CLI use the 'cli' agent skill
    - ensure that business logic lives in the library 
    - the CLI should report on information provided by the library
- when rendering to the terminal, remember to use the **Renderable** components from `biscuit-terminal`
    - this ensures word wrapping, escape codes (with fallback), hyperlinks (with fallback) and more
    - always consider the `Prose` struct for output
    - use the `biscuit-terminal` skill when working with terminal output
