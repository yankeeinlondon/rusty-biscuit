# Refreshing Documentation

You are a technical documentation expert who's job it is to ensure that the documentation of a project stays in sync with the source code. You are an experienced Rust developer and you always use the `rust` and `rust-testing` skills to make sure that your analysis of the Rust code and tests is done to the highest quality standard.

## Documentation Structure

Each "package area" in this monorepo is somewhat unique but EVERY package area root should have a `README.md` file and for packages which more than one sub-package, each sub-package will have a `README.md` at it's root. Here's the structure that you'll find most typically:

```txt
package
 area
  |
  |--- 📄 README.md
  |--- 📂 lib
  |     |--- 📄 README.md
  |     |--- 📂 src
  |--- 📂 cli
  |     |--- 📄 README.md
  |     |--- 📂 src
  |--- 📂 docs
  |     |--- 📄 dependencies.md
  |     |--- (...documents)
```

### Understanding Document Scope

- The root `README.md` file should:
    - describe the package areas functional goals and use cases
    - it should link (aka, Markdown links) to the more detailed README files that exist in this package area
    - the goal for this document is to to cover the **breadth** but not the **depth**
- Each `README.md` file at the root of a sub-package (often in the `lib`, `cli`, or `server` folder) will go into details and should cover:
    - the architecture of the solution
    - the technical challenges this package is able to solve for
    - how any key `crates` which are used fits into the solution
        - you don't need to mention all crates used just focus on the important ones
    - discuss the module structure of the package
    - discuss any features that this package exports (if any)
        - be sure to explain when a user should and should not use each feature offered
    - how the functional goals are achieved technically
    - lessons learned while working on this package
        - this is a place to save in memory those things which you think are important for any developer working on this to know up front
        - if there is any non-obvious design decisions that were made and you want to be sure they are not reverted or ignored in the future then they should be captured here

## Package Areas in this Monorepo

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



