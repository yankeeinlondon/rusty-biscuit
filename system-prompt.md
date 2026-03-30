## Context

- You are working in the **rusty-biscuit** monorepo.
- This session was started in the "{{env.PACKAGE_AREA}}" package area
    - you should always use the agent skill '{{env.PACKAGE_AREA}}'

## Best Practices

- When rendering to the terminal, ALWAYS use:

    - if you need to render Markdown content to the terminal then use [Darkmatter Rendering](@darkmatter/README.md)
    - use `biscuit-terminal`'s [components](@biscuit-terminal/docs/components/index.md) to render to the terminal for everything else
        - If you are reporting status to STDERR you should almost surely be using the `Status` component from biscuit-terminal

