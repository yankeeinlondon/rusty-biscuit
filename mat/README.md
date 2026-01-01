# `mat` CLI

A CLI markdown renderer for the terminal.

## Syntax

> **mat \<file\>** [_options_]

The `mat` command will render a markdown file with rich color themes.

### Options

- `--html` - instead of showing it in the _terminal_ it will be rendered to HTML (_in a temp directory_) and opened in your default browser
- `--theme` - by default it will use the `one-dark` theme if your terminal is in dark mode (_as it should be_) or `one-light` theme if you're preparing on moving to the sun. Other options are:

    | **Dark Theme** | **Light Theme** |
    | ---- | ----- |
    | **one-dark** | **one-light** |
    | gruvbox-dark | gruvbox-light |
    | tomorrow-night | tomorrow |
    | base-ocean-dark | base-ocean-light |
    | github-dark | github-light | 
    | solarized-dark | solarized-light |
    | two-dark | ??? |
    | cold-dark | cold-ark |
    | monokai | monokai-light |

