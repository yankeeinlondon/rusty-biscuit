# Research

## Modules for Research

This area of the `deckhand` monorepo is focused on research and is broken up into two discrete modules:

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

### Commands

1. **Library Research** (`research library <pkg> [prompt] [prompt]`)

    - perform research on a software library
    - all library research will use a set of fixed prompts to start the research:
        - strong overview of the package and how it is used (`overview.md`)
        - 

