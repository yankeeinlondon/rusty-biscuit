# Deckhand

> A practical helper that does common jobs in AI for you

## Modules

1. `lib`

    - provides the AI integration (via `rig` crate)

2. `cli`

    - a simple CLI to expose the `lib` module's AI integrations
    - uses the `clap` crate for basic structure and operations

3. `tui`

    - a `ratatui` based TUI app that exposes a chat interface
    - leverages the `lib` modules for AI integration

## Agent Tasks

1. Research Library

    Helps you get detailed information on a software library you're using (or considering using).

    ```mermaid
    flowchart TD
    U[User Prompt] --> I(Isolate Package and Language)
    I --> A(Summary Information)
    I --> DDF(Library Features)
    I --> R(Related Libraries)
    R --> RP(Pros/Cons)
    R --> RA(When to use; when not to)
    R --> RC(How does it compare?)
    I --> G(Gotchas and Workarounds)
    I --> UC(Use Cases)
    ```

2. Research Software

    Helps you investigate

3. Test Validator

    Evaluates the test delta between two git states and looks for:
        - suspicious/lazy behavior

4. Cross Link

    Cross links various independent documents.
