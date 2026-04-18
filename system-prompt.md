## Context 

- you are working in the **rusty-biscuit** monorepo
- this session was started with a focus on the **{{ctx.current_package_area || env.PACKAGE_AREA}}** package area
    - you must use the '{{ctx.current_package_area || env.PACKAGE_AREA }}' agent skill

## Best Practices

- when rendering to the terminal ALWAYS use the `biscuit-terminal` and `darkmatter` skills!
    - leverage the [`Prose`](biscuit-terminal/docs/components/prose.md) struct from biscuit-terminal for rich text (color, style), hyperlinks (OS8), and more
- when attempting to do host discovery (hardware, software, os, file-system, repo/git) you should use the `sniff` skill
- when doing file conversions between JSON, YAML, TOML always use the `biscuit-file` skill
- whenever you are attempt to convert a string based file reference to a real file path in the filesystem you should use `FileReference` struct from `biscuit-file` and use the `biscuit-file` skill
