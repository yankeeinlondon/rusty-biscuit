# Watch TODOs

The Watch TODO's package is a simple TUI application which will read in a Markdown file, extract any TODO's (GFM format) which reside in the file and then present these TODO's inside of a TODO Window and then actively changes the states of these TODO's when state changes.

```txt
┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ ┌───────────────────────────────────────────────────────────────────────────────┐ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                      TODO                                     │ ┃
┃ │                                     Window                                    │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ │                                                                               │ ┃
┃ └───────────────────────────────────────────────────────────────────────────────┘ ┃
┃                                                                                   ┃
┃     TODO Watcher | Watching "plans/2026-01-01. do-it.md" | Press ESC to exit      ┃
┃                                                                                   ┃
┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛
```

## Tech Spec

- we will use the `clap` crate to receive and parse the command to start our TUI application
- the TUI itself will be built using the `ratatui` crate
- this application will leverage the Shared Library in this repo for things like:
    - Markdown handling and processing (see `Markdown` struct)
    -

## Skills

All work on this project should use the following skills:

- `ratatui`
- `clap`
- `rust`
- `deckhand-library` (this provides skills on the shared library in this repo)

In addition, all testing should use the `rust-testing` skill.

## Features

- this application uses compiles to the command `watch-todos`
    - it takes a single parameter which is the markdown file it will be watching
    - there are a few CLI switches which can be used:
        - `--nerd` will indicate that nerd fonts are being used in the terminal and that the TUI should use the nerd-fonts representations of TODO states
        -
