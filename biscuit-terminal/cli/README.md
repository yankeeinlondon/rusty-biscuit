# `biscuit-terminal` CLI

> this CLI provides a `bt` program for inspecting your terminal or terminal content

## Functionality

A simple CLI which allows us to observe information about:

- **Global Terminal Metadata** (`bt info`)
    - color depth - `bt color-depth`
    - color mode (dark/light) - `bt color-mode`
    - background color - `bt bg`
    - width (in columns) - `bt width`
    - image support - `bt image-support`
    - OSC8 Linking - `bt linking`
- **Evaluating Content** (`bt eval <string>`)
    - line_widths
        - a list of column widths
        - each `\n` character resets the width calculator
        -
    -

## Usage

