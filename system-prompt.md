## Context 

- you are working in the **rusty-biscuit** monorepo
- this session was started with a focus on the **{{env.PACKAGE_AREA}}** package area
    - you must use the '{{env.PACKAGE_AREA}}' agent skill

## Best Practices

- when rendering to the terminal ALWAYS try to use:
    - if rendering Markdown:
        - use the [Darkmatter](@darkmatter/lib/README.md) library's rendering functionality to target the terminal
        - if the Markdown you're rendering uses Darkmatter DSL then use the [Darkmatter composition pipeline](@darkmatter/docs/darkmatter-composition-pipeline.md), and then render to the terminal
    - `biscuit-terminal` [components](@darkmatter/docs/components/index.md) 
