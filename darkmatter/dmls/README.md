# Darkmatter Language Server (DMLS)

## Feature Support

The DMLS _language server_ provides guard rails for author's wanting to create Darkmatter content. At it's core it starts as a full featured Markdown LSP and should be able to serve not only as the default for all "Darkmatter content" but also all "Markdown content".

Darkmatter content is a CommonMark Markdown plus GFM extensions plus Darkmatter's own DSL. The DSL includes the features provide by:

- the [composition pipeline](../docs/darkmatter-compose-pipeline.md) of Darkmatter
- the [render pipeline](../docs/darkmatter-rendering-pipeline.md) of Darkmatter

And one of the key areas of scope that DMLS provides that you wouldn't typically expect to see in a Markdown language server is support not just for the prose content in the body of the document but Darkmatter also provides schema support to YAML-based Frontmatter blocks.

## Editor Support

It's primary editor targets are:

- [VSCode](https://code.visualstudio.com/) - the most popular editor in the market and the OG for LSP implementations
- [Neovim](https://neovim.io/) - one of the most -- _if not **the most**_ -- popular solution for developers who like editing in the terminal
- [Helix](https://helix-editor.com/):  a Rust written terminal editor that is hyper fast and could be compared to a highly opinionated Neovim (though of course it has no technical Neovim underpinning)
- [Zed](https://zed.dev/) - a high performance modern desktop editor that requires an additional wrinkle for conformance (e.g., compilation to WASM)

While these three targets are the focus, LSP's are a standard that most editors support and the expectations is that DMLS should work across almost all editors, agents, etc. which support LSPs.

## Claudine as a 1st Class Citizen

One of the key users of Darkmatter is [Claudine](../../claudine/README.md) and the solution we come up with for DMLS needs to not only meet the direct needs of Darkmatter's DSL but also extend to the schema addon which Claudine imposes.

By doing this we will not only confidently be able to support Claudine feature set but the _mechanism_ we use to extend the features of Darkmatter should provide a basis for any other desired extension.

The additional features that Claudine 

- Lifecycle Events:

    - Claudine introduces _lifecycle events_ like `initialize`, `start`, `success`, `failure`, etc. and each event has a defined structure that defines what is valid/invalid configuration for this lifecycle event

    - Each lifecycle event is defined by the same "schema" which can be described as a `SimpleSchema` (which in turn can be easily converted to a JSON Schema)
