---
name: Drift
description: this command will evaluate a package in this monorepo for "drift" (aka, documentation which has become out of sync with the source code it tries to describe). This script will first evaluate all of the README.md files within the given package and align each document with the current state of the code.
---

# Drift

## The Process

1. Looking at the prompt the user provided you, you must determine the package in this monorepo which you are being asked to evaluate. Refer to the `## Packages in this Monorepo` for a breakdown on the packages which exist in this monorepo.
2. If you're NOT SURE about which package the user wanted you to evaluate then let the user know you need clarification on the package and exit immediately
3. Now that you've established the package you're being asked to evaluate for "documentation drift" you need to establish two things:

    - **Skills** - you must determine the "skills" to use for this task using this formula:
        - you will ALWAYS use the `rust`, `rust-testing`, and `clap` (all of the CLI's use clap) skills
        - you will also use the skills for all _dependencies_ the package has with _other packages_ in this monorepo:
            - typically the user will have provided you the list in a format that looks like `{package}: {dependency 1} {dependency 2}, etc.`
            - if -- and only if -- the users did not provide the dependencies as part of their prompt you can simply run `just repo-deps | ripgrep "${package}*:"` where `{package}` is the package you're focusing on.

4. You must first run `sniff docs --package {package}`

## Packages in this Monorepo

- **biscuit-file**
    - a library and CLI which help read and convert file types from one to another
- **biscuit-hash**
    - a library and CLI which provider best in class hashing features (xxHash, Blake3, and Argon2)
- **biscuit-speaks**
    - a library which abstracts a host's TTS programs and provides a unified TTS interface
- **biscuit-terminal**
    - a library and CLI which interrogates/detects features in terminals as well as provides a lot of highly useful "components" for rendering to the terminal including: Table, TwoColumns, OrderedList, UnorderedList, TerminalImage and more
- **claudine**
    - a library and CLI which attempt to provide a more unified event, skill, and "slash command" environment across various Agentic CLIs
- **darkmatter**
    - a library and CLI which parse and render markdown content and provide a small DSL on top of the Markdown standard to allow for greater composability as well as enhance rendering features like Mermaid diagrams, etc.
- **homelab**
    - a library, CLI, and HTTP server focused on the interactions with commonly found items in a Homelab.
- **model-citizen**
    - a library and ClI which help manage local LLM models and runners
- **playa**
    - a library and CLI which leverages the host's headless audio programs to play audio as well as a curated set of sound effects
- **queue**
    - a Ratatui based TUI which _queues_ programs for execution sometime in the future
- **research**
    - a library and CLI which provides a structured way to do research which results in both a "skill" content tree as well as a "Deep Dive" document containing all of the research on a given topic.
- **schematic**
    - a set of sub-packages which
- **sniff**
    - a library and CLI which detects hardware, network, services, and installed applications on the host machine. It also evaluates the current working directory to give insight into the current repo, packages, etc.
- **so-you-say**
    - A CLI which provides TTS functionality (by leveraging the `biscuit-speaks` library)
- **tree-hugger**
    - A static analysis library and CLI (`hug`) which provides code analysis via the popular tree-hugger library
- **unchained-ai**
    - A library and CLI which provides a wrapper around the popular `rig` crate for AI but extends this with a set of "primitives" used for creating chained AI interactions
