---
area: "{{ ctx.area }}"
scope: "{{ctx.area == 'root' ? 'package' : 'package area' }}"
mode: "append"
---

## Context

- you are working in the **rusty-biscuit** monorepo
- this session was started with a focus on the **{{area}}** {{scope}}
    - you must use the '{{ area }}' agent skill
- always prefer US English (en-US) over other English variants when creating symbol names or writing documentation
- the host computer is on the {{ctx.os}} operating system; consider this when running shell commands
- all packages in this monorepo MUST compile and work on:
    - macOS,
    - Windows,
    - and Linux
- you maybe limited to testing on just {{ctx.os}} with this host but make every effort to consider all three OS's when designing and implementing
  ::block when="area == 'biscuit-tui'"
- use the 'tui' and 'biscuit-tui' skills
- use the 'cli' skills too when working with 'biscuit-tui-cli'
- all scripts used in hook events or resources for slash commands/prompts should be saved to `.claudine/scripts` - prefer Typescript (executed by tsx or bun) over other language choices - bash script is an ok alternative where it's a better fit
  ::end-block
  ::block when="has_command(gitnexus)"
- **IMPORTANT:** never add gitnexus indexing information to CLAUDE.md or AGENTS.md
  ::end-block

::block when="ctx.area == claudine || ctx.area == darkmatter"

## Kind Formalism

We are in the process of being more "formal" with the use of _kinded_ YAML or Markdown documents. A _kinded_ document
is any document that defines the `kind` property at the root level of it's structured data. By doing so it declares
formally what kind of document it is. This declaration then has strong tie-in to the Darkmatter schema support that
both Claudine and Darkmatter use.

Kind catalog:

- `schema` - define a **SimpleSchema** schema
- `schema-trigger` - a declared way of pattern matching a document and apply a schema to it when the pattern matches
- `sequence` - a sequence definition in Claudine
- `group` - a schema for defining a task group for use in Claudine sequences
- `task` - a task definition

More to come.
::end-block

## Best Practices

- when rendering to the terminal always use `TerminalRenderable` (trait) components:
    - these are largely found in `biscuit-terminal` and `darkmatter` libraries
    - [`Prose`](biscuit-terminal/docs/components/prose.md) component:
        - rich text (color, style), hyperlinks (OS8), word wrap, and more
    - [`UnorderedList`](biscuit-terminal/docs/components/list.md) component:
        - create markdown-like unordered lists with nested word wrap, ergonomic support for Prose content, and more
    - many more including `OrderedList`, `Table`, `BlockQuote`, `MermaidDiagram`, `TwoColumns`, `CodeBlock`, and many more
- Note: many of the components which implement `TerminalRenderable` also implement `BrowserRenderable` meaning you can easily render to both terminal and browser.
- when attempting to do host discovery -- hardware, software, os, file-system, repo -- you should use the `sniff` library (and associated `sniff` **agent skill**)
- when doing file conversions between JSON, YAML, TOML always use the `biscuit-file` skill
- whenever you are attempt to convert a file reference to a real file path in the filesystem you should use the `FileReference` struct from `biscuit-file` (use the `biscuit-file` agent skill for help with this)
- when a package area has both a library and CLI (as many do) the naming convention is:
    - `{name}` for library
    - `{name}-cli` for the CLI
- never run `cargo fmt` unless told explicitly to do so
- never commit to **git** unless you are told to explicitly in the prompt (this will typically be done as a separate operation)
- prioritize solving solutions in a strategic, long term focused manner versus tactical wins:
    - this monorepo is a large code base and we need to guard against technical debt where possible
    - this monorepo is also a new codebase without any established users so the cost of refactoring (to achieve a more design advantageous goal) is far lower than it would be if there were a large install base

## Hashing Content

- all Markdown files which take a hash Frontmatter property representing the state of file should use the hashing functionality provided in **Darkmatter** (library and CLI)
    - when using the CLI the syntax is `md hash <file>`
    - The library and CLI both use a very fast implementation of **xxHash**
    - The Markdown file is segmented into a hash for it's frontmatter which is distinct from the body of the page (the `-` character delimits them)
- if you need to hash for non-markdown content, unless this is related to git or some other domain which has it's own hashing rules, then you should use the **biscuit-hash** library for hashing using xxHash
    - this content doesn't have the same Frontmatter versus Body hashing strategy but it uses the same **xxHash** hashing algorithm

## Testing

- we use **nextest** for unit and integration tests (not `cargo test`)
- when in a package area:
    - use `just test` (for unit tests)
    - use `just test-l2` (for integration tests)
    - use `just lint` to run linter
- when in the repo root:
    - use `just test {pkg}`
- when writing or updating L2 or L3 tests always make sure that terminal or browser windows do NOT gain focus!
