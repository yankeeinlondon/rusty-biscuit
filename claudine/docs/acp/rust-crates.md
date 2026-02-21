---
prompt: |-
    The ACP protocol sits on top of JSON-RPC and provides an open-source standard (spearheaded by the Zed and JetBrains teams) that does for AI coding agents what the Language Server Protocol (LSP) did for language tooling.

    Your task is to do research into who which Rust crates support the use of the ACP protocol. For each crate found:

    - Name of the library
    - URL (the primary URL for the software)
    - What features does the crate expose?
        - Detail when you should and should not use the various features exposed
    - How well does this crate cover the uses cases typically associated with the ACP protocol?
    - Which crates are most commonly compared to this crate? How do they compare?

    After detailing all the crates which cater to the ACP protocol, discuss how you might approach a bespoke/custom build instead of using one of these packages.

    - list out when you recommend using one of the crates found
    - list out when you recommend building a bespoke solution for ACP

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
update_policy:
    - Duration(3 mo)
---
