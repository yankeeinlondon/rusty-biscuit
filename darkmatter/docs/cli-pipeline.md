# Markdown Pipelining in CLI

This document describes the current Darkmatter CLI output pipeline contract.

## Render Pipeline Contract

Top-level render mode now uses one switch:

- `--output <output>`
  - `auto` (default):
    - TTY stdout: ANSI terminal rendering
    - non-TTY stdout: markdown text
  - `markdown` (alias: `text`)
  - `html`
  - `json` (alias: `ast`)

`--show` is output-neutral and applies to top-level render mode:

- selected output is written to a temp file and opened using the `open` crate
- extension mapping:
  - html -> `.html`
  - markdown/text -> `.md`
  - json/ast -> `.json`
- in `--output auto` + TTY mode, ANSI output is still rendered to stdout and markdown is opened in a temp file

## Subcommands

The TOC and delta operations are subcommands:

- `md toc <input> [--json]`
- `md delta <base> <updated> [--json]`

JSON output for these operations is scoped to subcommands (`--json` is not top-level).

## Terminal Image Behavior

Top-level CLI no longer exposes `--no-images`.

Terminal image behavior is controlled through `TERMINAL_IMAGES`:

- truthy (`true`, `1`, `yes`, `on`) -> force protocol output attempts
- falsy (`false`, `0`, `no`, `off`) -> never render protocol images
- unset/invalid -> capability-driven auto mode

## Markdown Image Semantics

For markdown image parsing/serialization:

- width hints in alt text use only the `|` delimiter: `![alt|15%](./img.png)`
- rich `ImageRef` metadata remains lossless in markdown mode when configured for lossless metadata
- undefined metadata fields are omitted from serialized metadata payloads

## Removed Legacy Flags

The following top-level flags are removed:

- `--html`
- `--show-html`
- `--ast`
- `--json`
- `--no-images`
- `--toc`
- `--delta`
