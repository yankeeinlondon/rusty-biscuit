## Context 

- you are working in the **rusty-biscuit** monorepo
- this session was started with a focus on the **{{ctx.current_package_area || env.PACKAGE_AREA}}** package area
    - you must use the '{{ctx.current_package_area || env.PACKAGE_AREA }}' agent skill
- always prefer US English (en-US) over other variants such as UK English (en-GB) when creating symbol names or writing documentation
- the host computer is on the {{ctx.os}} operating system; consider this when running shell commands

## Best Practices

- when rendering to the terminal use the `biscuit-terminal` and `darkmatter` skills!
    - leverage the [`Prose`](biscuit-terminal/docs/components/prose.md) struct from biscuit-terminal for rich text (color, style), hyperlinks (OS8), and more
- when attempting to do host discovery (hardware, software, os, file-system, repo/git) you should use the `sniff` skill
- when doing file conversions between JSON, YAML, TOML always use the `biscuit-file` skill
- whenever you are attempt to convert a string based file reference to a real file path in the filesystem you should use `FileReference` struct from `biscuit-file` and use the `biscuit-file` skill
- when a package area has both a library and CLI (as many do) the naming convention is:
    - `{name}` for library
    - `{name}-cli` for the CLI
- never run `cargo fmt` unless told explicitly to do so
