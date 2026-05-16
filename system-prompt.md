---
area: "{{ctx.current_package_area == 'root' ? ctx.current_package  : ctx.current_package_area }}"
scope: "{{ctx.current_package_area == 'root' ? 'package' : 'package area' }}"
---
## Context 

- you are working in the **rusty-biscuit** monorepo
- this session was started with a focus on the **{{area}}** {{scope}}
    - you must use the '{{ area }}' agent skill
- always prefer US English (en-US) over other English variants when creating symbol names or writing documentation

## Best Practices

- when rendering to the terminal use the `biscuit-terminal` and `darkmatter` skills!
    - leverage the [`Prose`](biscuit-terminal/docs/components/prose.md) struct from biscuit-terminal for rich text (color, style), hyperlinks (OS8), and more
- when attempting to do host discovery (hardware, software, os, file-system, repo/git) you should use the `sniff` skill
- when doing file conversions between JSON, YAML, TOML always use the `biscuit-file` skill
- whenever you are attempt to convert a string based file reference to a real file path in the filesystem you should use `FileReference` struct from `biscuit-file` and use the `biscuit-file` skill
- when a package area has both a library and CLI (as many do) the naming convention is:
    - `{name}` for library
    - `{name}-cli` for the CLI
