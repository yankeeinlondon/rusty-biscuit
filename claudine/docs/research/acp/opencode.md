---
prompt: |-
    Do a deep dive on the ACP implementation that Opencode provides.

    - make sure to mention any quirks or gotchas that developers mention facing when interacting with Opencode as well any workarounds or ways to avoid issues

    After completing the deep dive, provide the following additional sections which cover writing code examples with Rust:

    1. Show how a Rust client can interact programmatically with the Agent using ACP
    2. Show explicitly how to handle "Reverse Requests" where the Agent asks the client to fulfill a tool request, a file read, etc. (as an Agent is not allowed to do this directly when operating via ACP)
    3. Show how a Rust client can respond to requests to execute commands on the host system
    4. Show how the Rust client we've created can use things like `mpsc` channels to send the Agent's streaming text to a desktop desktop app framework like Tauri or iced

    ## Frontmatter:

    - make sure to update `last_updated` property every time the `prompt` is executed (format: YYYY-MM-DD)

    ## Research

    Your research content should be added to the body of this document along with ensuring that the Frontmatter properties above are updated while preserving all other markdown properties.

last_updated: 2026-02-21
update_policy:
    - Duration(6 mo)
---

