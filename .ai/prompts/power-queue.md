# Queue CLI additions

## Overview

This project will focus on the `queue` package of this monorepo.

- currently the implementation is just a simple CLI which queues tasks up to be executed at some later date/time.
- the CLI runs the queue as a background task allowing the user to continue using the terminal they're currently in
- there is a concept of foreground/background tasks which the CLI associates with the queued job
- read the @queue/README.md for more context

## Phase 0

Before we do anything else we will need to split this project from one CLI package into two submodules:

1. Library Module [ `./queue/lib` ]
2. CLI Module [ `./queue/cil` ]

To start in this initial Phase 0, we can simply move the current implementation into the new CLI module.

## Functional Change

Following our restructuring work we will:

- add a TUI module to the library module which is based on `ratatui` (you must use the `ratatui` skill)
- we will use the `crossterm` crate together with `ratatui` to provide a lot of the technical framing for our TUI
- functionally:
    - When the user calls `queue` or `queue --at 9:45pm 'echo "hi"'` or any other combination of the parameters and switches our current CLI provides we will load the main window of our TUI:
        - if the terminal is either Wezterm (which has a fully programmatic multiplexers built in via the CLI) then we'll create a pane directly below the current window just enough rows to fit the TUI into.
        - if the terminal is not Wezterm then we'll open the TUI in the current pane

          ```txt
          ┌─────────────────────────────────────────────────────────────────┐
          │                                                                 │
          │  ID      WHEN       Command                Where                │
          │ -───-   -─────-    -──────────────────-   -─────────────-       │
          │                                                                 │
          │   1     9:00pm      echo 'hi'              new pane             │
          │                                                                 │
          │   2     10:00p      echo 'bye'             background           │
          │                                                                 │
          │   3     10:00p      echo 'over here'       new window           │
          │                                                                 │
          ├─────────────────────────────────────────────────────────────────┤
          │     N (new), E# (edit), R# (remove), H (history), Q (quit)      │
          └─────────────────────────────────────────────────────────────────┘
          ```

        - The hotkeys are (letter hotkeys are case-insensitive):

            - `Q` or `CTRL+C` for quit
                - when `Q` is used we confirm that the user really wants to quit the queue (as that will mean all jobs will no longer be queued)
                - when `CTRL+C` is used we'll just exit
            - `N` will bring up a input modal for a **new** task to be queued
                - the modal will always fit into the window size we have set, however, once a new task has been added we will need to increase the height of the pane of the TUI.
            - `E` will colorize the bottom menu with a light yellow background and the `E# (edit)` option boldfaced
                - we will then wait for a number to indicate _which_ task the user wants to edit
                - if there is only one task then we do not need to wait for a number
                - if a user presses `ESC` then the bottom menu returns to it's normal grey background
                - when we get a valid number key (or there is only one task), we will bring up the edit modal
            - `R` will colorize the bottom menu with a light red background and the `R# (remove)` text boldfaced
                - we will then wait for a valid number to be pressed (even if there is only one task)
                - if the user presses `ESC` we return the bottom menu to it's normal grey background
                - if we get a valid task number we will remove that task from the queue and then return the bottom menu to it's normal grey
            - `H` will bring up a history window
                - the history for the queue will be stored in a `~/.queue-history.jsonl` file
                - any new item added to the queue is also added to the history file
                - when the user pressed `H` they are shown a history modal which lists prior commands which have been queued in the past
                - while in that modal:
                    - up and down arrows highlight the history items
                    - pressing `N` will replace the history modal with the Input Modal for inputting a new command and the history item which had been selected will already be filled in as the task
                    - pressing `ESC` will bring the user back to the main window
                    - pressing `F` will bring up a temporary "filter" modal which will allow the user to

### Showing Status on Main window

- The main window shows the tasks upcoming but also those which have completed
- When a queued task is executed the task line is italicized and dimmed


### The "where" parameter

When a task is setup it will always default to:

- `new pane` for Wezterm
- `new window` for other terminal apps

The only other option is `background`. Let's discuss what each means:

- `new pane`
    - means that when a task is activated the **queue** program will execute the task in a vertically split pane from where the TUI resides.
    - The new task will be placed _above_ the TUI application.
    - It is common for the `queue` TUI to reside at the bottom of tab and then all it's tasks will be placed above it and each task will get it's own STDOUT and STDERR.
    - Having a TTY session for the process is critical for any interactive program you run.

- `new window`
    - when a task's ready to be executed it will be started in another window and/or instance of the terminal application
    - like with `new pane` the task will have a full TTY session and therefore can comfortably host interactive programs

- `background`
    - when a task is ready to be executed it will be run as a background process
    - this kind of execution is good for non-interactive programs that do not need to report important information to STDOUT/STDERR (TTS notifications are a good example)

### Input Modal

This is a low fidelity draft of what the Input modal might look like:

```txt
┌─────────────────────────────────────────────────────────────────┐
│                                                                 │
│  New Queued Task                                                │
│ ┌────────────┐ ┌─────────────────────┐ ┌────────────────┐       │
│ │   time    ▾│ │       9:45pm        │ │   new pane    ▾│       │
│ └────────────┘ └─────────────────────┘ └────────────────┘       │
│ ┌────────────────────────────────────────────────────────────┐  │
│ │echo 'hi'                                                   │  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│         ENTER (add task), ESC (main menu), H (history)          │
└─────────────────────────────────────────────────────────────────┘
```

### History Modal

This is a low fidelity draft of what the History modal might look like:

```txt
┌─────────────────────────────────────────────────────────────────┐
│ ┌───────────────────────────────────────────────────────┐ ┌───┐ │
│ │echo 'hi'                                              │ │ △ │ │
│ └───────────────────────────────────────────────────────┘ │   │ │
│ ┌───────────────────────────────────────────────────────┐ │   │ │
│ │claude 'do something impressive'                       │ │   │ │
│ └───────────────────────────────────────────────────────┘ │   │ │
│ ┌───────────────────────────────────────────────────────┐ │   │ │
│ │so-you-say "job well done"                             │ │   │ │
│ └───────────────────────────────────────────────────────┘ │ ▽ │ │
│                                                           └───┘ │
├─────────────────────────────────────────────────────────────────┤
│   ▲▼ (select), F (filter), ENTER (copy task), ESC (main menu)   │
└─────────────────────────────────────────────────────────────────┘
```


## Skills

- use the `wezterm` skill for working with Wezterm's CLI and multiplexing functionality
- use the `ratatui` and `crossterm` skills for working with the TUI
- ALWAYS use the `rust` and `rust-testing` skills
