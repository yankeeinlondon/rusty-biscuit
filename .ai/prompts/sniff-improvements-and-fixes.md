# Sniff Improvements and Fixes

## CLI

### Output Formats

We have two output formats for the CLI: `JSON` and `TEXT` so:

- instead of having a `--format` switch we should just default to text output and add a `--json` output if the user wants JSON output

The JSON output format is correctly reporting on everything, however, the terminal and text are not well suited to dumping raw data to and having it be consumable so for the text based output we need to refine how we report results.

The text output by itself should be rather minimal in what it shows. Then allow the `--verbose`/`-v` flag to increase the amount of information that is presented.

In text output always:

- use relative file paths from the repo root not full paths
- `3996329328640` is unreadable for the number of bytes; role it up to `mb`, `gb`, etc. and use commas, etc. to make it more readable
    - the same logic should be considered for any large numeric value


### Fixes

- I ran the `sniff` CLI from the root of this monorepo and got this for dependencies:

  ```json
  "dependencies": {
      "detected_managers": [
        "Cargo"
      ],
      "packages": []
    }
  ```

  The problem is there are TONS of dependencies scattered across this monorepo. Maybe this was just reporting on the root `Cargo.toml`? We need to report on ALL Cargo.toml's and in a monorepo these dependencies need to be organized by

- under the `monorepo` section the packages list looks like this:

    ```json
    "packages": [
        {
          "name": "md",
          "path": "/Volumes/coding/personal/dockhand/md"
        },
        {
          "name": "cli",
          "path": "/Volumes/coding/personal/dockhand/research/cli"
        },
        {
          "name": "lib",
          "path": "/Volumes/coding/personal/dockhand/research/lib"
        },
        {
          "name": "shared",
          "path": "/Volumes/coding/personal/dockhand/shared"
        },
        {
          "name": "cli",
          "path": "/Volumes/coding/personal/dockhand/sniff/cli"
        },
        {
          "name": "lib",
          "path": "/Volumes/coding/personal/dockhand/sniff/lib"
        },
        {
          "name": "so-you-say",
          "path": "/Volumes/coding/personal/dockhand/so-you-say"
        },
        {
          "name": "tabby",
          "path": "/Volumes/coding/personal/dockhand/tabby"
        },
        {
          "name": "ui",
          "path": "/Volumes/coding/personal/dockhand/tabby/ui"
        },
        {
          "name": "tui",
          "path": "/Volumes/coding/personal/dockhand/tui"
        }
      ]
    }
    ```

    This shows lots of modules just called "lib" or "cli"! You have forgotten to provide the directory name spacing that will keep them unique!
- the languages section has file counts for each language and that's ok but you have 252 Markdown files? That seems too high. Possibly its right but make sure that the files in `.gitignore` are NOT being counted!
- the primary "language" for this monorepo is "Markdown"! That is nonsensical, the only correct answer would be a _programming language_!
    - Furthermore when you have a monorepo it's important to report each module
    - In this monorepo almost all the modules (if not all) should be reporting Rust as the primary language!
- when we report on storage we're only showing a subset of the mounts, things like `VeniceRack` which is an SMB mount are missing.

