---
prompt: |-
    Your task is to do a deep dive on the Rust crates that a developer might consider when working on interacting with the audio on a computer (desktop or mobile).

    - what are the crates people turn to for working with audio?
    - list all crates found with the following information:
        - name
        - description of the core feature set
        - strength for supporting macOS? For supporting Windows? For supporting Linux? For supporting IOS? For supporting Android?
        - repo URL, docs URL
        - when to use? when not to use?
        - what features does the package expose to users of this crate? Explain what each feature means, when you should use it, and when you should not.
        - what are some of the gotchas developers report having with this crate and how can these obstacles be avoided?
        - what crates are most like this crate in terms of functional footprint?
    - create a summary table that helps to show the various crate options and the functional reach each has (on which OS's)

    The final deliverable is a idiomatic Markdown document. All code examples should be written in modern Rust (assume 2024 edition). If you feel a Mermaid diagram would help illustrate an idea, please feel free to include that too.

model: GLM 5 (agent)
last_updated: 2026-02-27
update_policy: 
    - Duration(6mo)
---

