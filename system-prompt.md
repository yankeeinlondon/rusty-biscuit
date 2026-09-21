---
area: "{{ ctx.area }}"
scope: "{{ctx.area == 'root' ? 'package' : 'package area' }}"
mode: "append"
---

## Context

- you are working in the **rusty-biscuit** monorepo
- this session was _started_ in the **{{area}}** {{scope}}
::block when="has_skill(area)"
    - you must use the '{{ area }}' agent skill
::end-block
- always prefer US English (en-US) over other English
- the host computer is on the {{ctx.os}} operating system
- all packages in this monorepo MUST compile and work on:
    - macOS,
    - Windows,
    - and Linux
::block when="area == 'biscuit-tui'"
- use the 'tui' and 'biscuit-tui' skills
- use the 'cli' skills too when working with 'biscuit-tui-cli'
- all scripts used in hook events or resources for slash commands/prompts should be saved to `.claudine/scripts` - prefer Typescript (executed by tsx or bun) over other language choices - bash script is an ok alternative where it's a better fit
::end-block
::block when="has_command(gitnexus)"
- **IMPORTANT:** never add gitnexus indexing information to CLAUDE.md or AGENTS.md
::end-block
- kinded docs
    - many Markdown documents use the `kind` keyword to describe what kind of document they are
    - you can review this at [kind catalog](docs/kind-documents.md) _or_ go directly to the schema [schema](^schemas/kind.yaml)

::file {{ctx.repo_root}}/.system-prompt.md when="file_exists('{{ctx.repo_root}}/.system-prompt.md')"

## Best Practices

- when rendering to the terminal always consult the `biscuit-terminal` library's `TerminalRenderable` (trait) components
    - [`Prose`](biscuit-terminal/docs/components/prose.md) component:
        - rich text (color, style), hyperlinks (OS8), word wrap, and more
    - [`UnorderedList`](biscuit-terminal/docs/components/list.md) component:
        - create markdown-like unordered lists with nested word wrap, ergonomic support for Prose content, and more
    - many more including `OrderedList`, `Table`, `BlockQuote`, `MermaidDiagram`, `TwoColumns`, `CodeBlock`, ...
    - Note: many of the components which implement `TerminalRenderable` also implement `BrowserRenderable` meaning you can easily render to both terminal and browser.
- when attempting to do host discovery -- hardware, software, os, file-system, repo -- you should use the `sniff` library (and associated `sniff` **agent skill**)
- use the `biscuit-file` skill when:
    - doing file conversions between JSON, YAML, TOML
    - whenever you are attempt to convert a file reference to a real file path in the filesystem you should use the `FileReference` struct
- when a package area has both a library and CLI (as many do) the naming convention is:
    - `{name}` for library
    - `{name}-cli` for the CLI
- NEVER run `cargo fmt` unless told explicitly to do so
- NEVER commit to **git** unless you are told to explicitly in the prompt (this will typically be done as a separate operation)
- prioritize solving solutions in a strategic, long term focused manner versus tactical wins:
    - this monorepo is a large code base and we need to guard against technical debt where possible
    - this monorepo is also a new codebase without any established users so the cost of refactoring (to achieve a more design advantageous goal) is far lower than it would be if there were a large install base
- when you are using hashing functionality always use `biscuit-hash` (and `biscuit-hash` agent skill)
    - Darkmatter uses `biscuit-hash` and it's CLI exposes `md hash` commands

## Context on Repo Lifecycle

This monorepo is early in its lifecycle. We are building it to be low in technical debt and we want to build things correctly and with reasonable checks in place for quality. However, there are no current customers so do not overly onerous in sign-offs, we don't need to "deprecate" functionality in 99% of cases we just need to agree to remove something. 

## Testing

- we use **nextest** for unit and integration tests (not `cargo test`)
- when in a package area:
    - use `just test` (for unit tests)
    - use `just test-l2` (for integration tests)
    - use `just lint` to run linter
- when in the repo root:
    - use `just test {pkg}`
- use the **Test Toolkit** provided by this monorepo: [testing toolkit](tools/test-toolkit/README.md)
- when writing or updating L2 or L3 tests always make sure that terminal or browser windows do NOT gain focus!
