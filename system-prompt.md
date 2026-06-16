---
area: "{{ctx.current_package_area == 'root' ? ctx.current_package  : ctx.current_package_area }}"
scope: "{{ctx.current_package_area == 'root' ? 'package' : 'package area' }}"
instructions: |-
    ## Context 
    
    - you are working in the **rusty-biscuit** monorepo
    - this session was started with a focus on the **{{area}}** {{scope}}
        - you must use the '{{ area }}' agent skill
    - always prefer US English (en-US) over other English variants when creating symbol names or writing documentation
    - the host computer is on the {{ctx.os}} operating system; consider this when running shell commands
    ::block when="area == 'biscuit-tui'"
    - use the 'tui' and 'biscuit-tui' skills
    - use the 'cli' skills too when working with 'biscuit-tui-cli'
    ::end-block
    
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
    - if you are ever sending raw escape codes to the terminal you are doing something wrong! You should be using a `TerminalRenderable` components!
    
    ## Hashing Content
    
    - all Markdown files which take a hash Frontmatter property representing the state of file should use the hashing functionality provided in **Darkmatter** (library and CLI)
        - when using the CLI the syntax is `md hash <file>`
        - The library and CLI both use a very fast implementation of **xxHash**
        - The Markdown file is segmented into a hash for it's frontmatter which is distinct from the body of the page (the `-` character delimits them)
    - if you need to hash for non-markdown content, unless this is related to git or some other domain which has it's own hashing rules, then you should use the **biscuit-hash** library for hashing using xxHash
        - this content doesn't have the same Frontmatter versus Body hashing strategy but it uses the same **xxHash** hashing algorithm
---
::block when="env.AGENT == 'xxx'"
::file ~/.claudine/system-prompts/fable-5.md exclude_product_info=true safety=none fable="Opus 4.8"  
::end-block
::block when="env.AGENT != 'claude'"

{{instructions}}
::end-block
