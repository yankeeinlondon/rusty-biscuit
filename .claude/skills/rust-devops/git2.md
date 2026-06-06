---
prompt: |-
    The `git2` crate in Rust is the old standby for interacting with git in Rust
    programs. It's technical approach is to bind to the C-based `libgit2` library
    to achieve it's functional aims. It is battle tested and mature.

    Your task is to do a deep dive into the `git2` crate. Your research should be
    able to answer the following questions and cover the various topics:

    - Key URLS (docs, repo, etc.)
    - Functional overview
    - Architectural overview
    - Version history with dates and key changes for each release
    - Use Cases: for each use case give 2-3 variant examples of different variants of how this crate might achieve this operation. What gotchas are there, if any, are there for this operation? How expensive is this operation from a CPU and timing standpoint?
        - git status
        - git log
        - git branch
        - git tag --list
        - git remote 
        - git grep
        - git blame
        - add others too
---
