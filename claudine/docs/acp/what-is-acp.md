---
prompt: |-
    The ACP protocol sits on top of JSON-RPC and provides an open-source standard (spearheaded by the Zed and JetBrains teams) that does for AI coding agents what the Language Server Protocol (LSP) did for language tooling.

    Provide a full overview of the APC protocol.

    - describe the general semantics and syntax that APC uses
    - describe any/all major versions of the specification along with dates these versions became available
    - what endpoints or operations does the APC specification provide?
    - what are the uses-cases supported by the APC specification?
    - describe any common gotchas that developers describe hitting when using the APC specification along with any solutions or workarounds that help in avoiding these gotchas.
    - are there any similar specifications which ACP is competing with (for developer and product attention)
    - provide a simple code example of using APC in:
        - Typescript
        - Python
        - Rust

    Frontmatter:
    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)
    - make sure to set a `latest_version` property which should be the LATEST version of the specification

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
update_policy:
    - MajorVersion(latest_version)
    - Duration(1 year)
---

