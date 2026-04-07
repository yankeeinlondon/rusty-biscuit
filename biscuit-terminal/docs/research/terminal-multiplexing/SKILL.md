---
name: terminal-multiplexing
description: Information on how structurally think about the feature "terminal multiplexing", overview of common libraries that can help in working with multiplexing, details on what the "biscuit-terminal" library is able to detect, and details on the various common multiplexer solutions like tmux, zellij, wezterm's native multiplexer, cmux, iterm2's tmux support, and more.
prompt: |-
    ## Context

    - You are building an Agent Skill for the topic of 'terminal-multiplexing' which will be published locally to the **Rusty Biscuit** monorepo (which you are in)
    - You will act as an Orchestrator in your role to complete this task, spawning subagents to perform most of the actual work.

    ## Overview of Process

    - An "agent skill" is a linked tree of Markdown documents which are always rooted in the 

last_updated: 2026-03-17
source: biscuit-terminal
---

## About Multiplexing

For more information about what terminal multiplexing is and what features to expect check out the links below:

::toc-linking about.md

## The Top **Multiplexer** Solutions

The grand-daddy of all multiplexers is **tmux**, find details on it and other popular solutions by following the links below:

### The Cross Terminal Solutions

1. [tmux](./tmux.md) - the OG of multiplexers
2. [zellij](./zellij.md) - a newer solution with better UI but less power user features

### Terminal App Support

Both **wezterm** and **cmux** provide a rich set of programmatic (and config/key-binding) based multiplexing

1. [wezterm](./wezterm.md) - Wezterm is a terminal application with multiplexing built in
2. [cmux](./cmux.md) - A fork of Ghostty that adds full multiplexing support

Here is a table of 

| Terminal           | Split Panes    | Resize Panes        | Focus Panes         | Execute in Pane | Save Layouts |
| ------------------ | -------------- | ------------------- | ------------------- | --------------- | ------------ |
| **WezTerm**        | ✅              | ✅                   | ✅                   | ✅               | ⚠️            |
| **Ghostty**        | ✅              | ✅                   | ✅                   | ⚠️               | ✅            |
| **iTerm2**         | ✅              | ✅                   | ✅                   | ✅               | ✅            |
| **Apple Terminal** | ⚠️ (view split) | ⚠️ (view split only) | ⚠️ (view focus only) | ❌               | ⚠️            |
| **Warp**           | ✅              | ✅                   | ✅                   | ✅               | ✅            |
| **Kitty**          | ✅              | ✅                   | ✅                   | ✅               | ✅            |
| **Alacritty**      | ❌              | ❌                   | ❌                   | ❌               | ❌            |
| **Konsole**        | ⚠️ (view split) | ⚠️                   | ⚠️                   | ❌               | ✅            |

Legend: ✅ supported, ⚠️ partial/qualified, ❌ not supported.



## Library Support


