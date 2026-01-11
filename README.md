# Rusty Biscuit
<img src="./assets/biscuit-and-crab.png" style="position: fixed; max-width: 30%; height: 150px; right: 0; top: 0; opacity: 0.75"></img>

> A monorepo for AI-powered research and automation tools

- All libraries, CLI's, and TUI's are written in Rust.
- Many CLI/TUI's are published to `npm` as well as `cargo`

## Packages

This monorepo hosts the following packages:

### Shared Libraries

1. **biscuit** [`./shared`]

    Provides utility functions and a highly capable markdown pipelining engine.

2. **schematic** [`./api`]

    Builds type-strong API's to be consumed by other libraries/apps.

3. **ai-pipeline** [`./ai-pipeline`]

    Provides a set of AI pipeline primitives for Agent composition while re-exporting some `rig` primitives to allow lower level interaction as well.


### Applications

1. **researcher** [`./research`]

    A **CLI** which facilitates the research process and is able to produce content rich "deep dives" and tree-based `skills` for Claude Code and Opencode.

    ```sh
    # do research
    research library chalk
    # list research
    research list
    # link research to Claude Code and Opencode
    research link
    ```

2. **md** CLI [`./md`]

    A Markdown renderer which renders to both the terminal(escape codes) and browser (HTML).

    ```sh
    # render markdown to the terminal with auto-light/dark theming
    md doc.md
    # clean a document to make it a more conformant CommonMark+GFM document
    md doc.md --clean
    # render to HTML
    md doc.md --html
    # render as JSON AST (`mdast`)
    md doc.md --ast
    ```

3. **observer** TUI [`./observer`]

    A TUI which helps you to observe state changes in the terminal.

    ```sh
    # observes changes in status/progress when pointed to either a
    # markdown file (with TODO's in it) or a JSON file which is structured
    # as a observation data file.
    observe <file>
    ```

4. **notable** [`./notable`]

    A CLI (and lib) which interacts with an Obsidian vault.

    ```sh
    # Add a note to your vault, with specified tags
    note #foo #bar hello world
    # Create a new note and fill it with the results of a prompt.
    # The prompt is saved as frontmatter, the LLM's response is saved
    # the body of the message.
    note --prompt "what are the top news stories today?"
    ```

5. **so-you-say** CLI [`./so-you-say`]

    A simple TTS CLI which leverages TTS features on the host or in the cloud.

    ```sh
    # TTS
    so-you-say "hello world"
    # TTS with specific gender voice
    so-you-say "hello world" --gender male
    ```

## More Details

For more functional/usage details on any of the packages in this monorepo refer to the `README.md` files in their respective directories.

> **Note:** if you're a developer and looking for more detailed documentation or context, then look for `README.md` files within folders of the source tree. These files will provide information about their respective module or source sub-tree.

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0-or-later).

You are free to use, modify, and redistribute this software under the terms of that license. See the [`LICENSE`](./LICENSE) file for full details.

> **Note:** If you run this software as a service, you must provide a link to the source code of the running version.
