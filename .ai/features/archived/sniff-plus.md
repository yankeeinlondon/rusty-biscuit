# Sniff Plus

The Sniff library and CLI are still very immature. There are still mistakes and limited features. This feature attempts to improve both feature reach and fix problems.

## Filesystem

### Formatting

This property doesn't currently exist but we should

- add `filesystem.formatting` and add the property `editorconfig`
- the `filesystem.formatting.editorconfig` will provide the `.editorconfig` file information in an intuitive interface if the file exists in the root of the repo

### Languages

- in the filesystem.languages section we have a list of all the languages found across the entire repo (regardless of which package it is in if a monorepo)
- we currently list the `language`, the `file_count`, and the `percentage`.
- we need to add a `files` property which enumerates the files for the given file type (with relative path from root)

### Git

- in the "status" section of git we need a `dirty` property added which is a list of all the files which are dirty.
    - each entry in the `dirty` list should include:
        - `filepath`: relative filepath (relative from repo root)
        - `absolute_filepath`: absolute filepath
        - `diff` - the diff from last local commit
        - `last_local` - the commit hash for the last commit of this file
        - `origin` - the commit hash for the last commit pushed to origin remote
- in the "status" section of git we need a `untracked` property which:
    - is a list of every untracked file
    - each item in the list has the following properties:
        - `filepath`: relative filepath (relative from repo root)
        - `absolute_filepath`: absolute filepath

### Monorepo

- in the filesystem.monorepo.packages section we report `name` and `path` for each module found. We need to add:
    - `primary_language` - the primary programming language in this package
    - `languages` - an array of the languages found in this section (same structure, including new `files` enumeration)
    - `detected_managers` - a list of the package manager found in use in this package of the monorepo

## CLI Switches

- we have `-skip-XXX` switches already which filter out certain sections
- let's add the inverse:
    - `--hardware`
    - `--network`
    - `--filesystem`

## Hardware

Finally, in the hardware section, I do not see anything about:

- GPU hardware
- SIMD, AVX, AVX2, AVX512 and other optimizations which might be useful to know

These should be added.
