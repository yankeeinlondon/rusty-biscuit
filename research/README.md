# Research

## Modules for Research

This area of the `deckhand` monorepo is focused on **research** and is broken up into two discrete modules:

- **Research Library** (`/research/lib`)

    Exposes functions which allow the research into various topics and items.

    > **Note:** _will leverage the `shared` module in this monorepo for highly generalizable operations_

- **CLI** (`/research/cli`)

    Exposes the `research` CLI command and leverages the research library to achieve it's goals.

## Types of Research

1. `Libraries`

    - research code libraries found on various package managers like `create.io`, `npm`, etc.

2. `Software`


## Using the CLI

### Global Switches

- `verbose` / `v` - verbose output
- `help` - show the help system

### Commands

1. **Library Research** (`research library <pkg> [prompt] [prompt]`)

    - Perform research on a software a particular software library
    - **Underlying Research**
        - all library research will start with a set of _underlying_ research:
            - strong overview of how the package is used and what it's functional footprint is (`overview.md`)
            - a
            - b
            - c
            - d
        - in additional to all of the default research you can add additional prompts which will be evaluated to create additional documents in the _underlying research_.
    - **Summary Deliverables**
        - once all _underlying research_ is complete two deliverables will be produced.
        - both deliverables are intended to be comprehensive in terms of information about the given library but the way the information is structured varies:

          1. **Deep Dive Document**

            - a single document which covers everything
            - it starts with a table of contents to aid navigation
            - the document contains a combination of rich prose and code examples
            - the intended audience for this document is both humans and LLM's which haven't been trained on a skill-based knowledge tree like that introduced by Anthropic for Claude Code.

          2. **Skill**

            - a tree shaped linked structure of documents which contain concise but complete coverage of the given software library
            - the information created is modelled after the Claude Code **skill** structure and:
                - has an entry point of `SKILL.md`
                - while the `SKILL.md` should be fairly short it will provide links to other sub-areas which go into greater detail
                - the idea of this format is to allow an LLM to selectively use the parts of the skill that it needs to complete it's task; thereby using the context window wisely rather than just putting all information into the context window right away.

