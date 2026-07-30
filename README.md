# Rusty Biscuit

<img src="./assets/biscuit-and-crab.png" style="position: fixed; max-width: 30%; height: 200px; right: 0; top: 0; opacity: 0.75"></img>

> A set of deterministic tools for a non-deterministic world

## Package Areas

This monorepo is broken up into _package areas_ many of which combine a Library package for programmatic access and a CLI for terminal access.

| Capability | Communicate | Inspect |
| ---                                      | ---         | --- |
| [biscuit-file](./biscuit-file/README.md) - _file utils_   | [biscuit-speaks](./biscuit-speaks/README.md) (TTS) | [tree-hugger](./tree-hugger/README.md) - _static-analysis_ |
| [biscuit-hash](./biscuit-hash/README.md) - _hash utils_ | [messenger](./messenger/README.md) - _Discord, Slack, ..._  | [sniff](./sniff/README.md) - _host detection_  |
|  [biscuit-terminal](./biscuit-terminal/README.md) - _term detect & render_ | [playa](./playa/README.md) - _headless audio_ | |
| [biscuit-visualized](./biscuit-visualized/README.md) - _data viz_ | | |
| [schematic](./schematic/README.md) - _API clients_ | | |

Then there are two libraries centered around the ideas of _composition_ and _orchestration_:

- [darkmatter](./darkmatter/README.md) - provide a DSL on top of Markdown to provide powerful composition patterns in Markdown
- [unchained-ai](./unchained-ai/README.md) - provides a compositional tool that allows the chaining and parallelization of many AI related tasks (both deterministic and non-deterministic).

It being 2026, it feels wrong not to have a more AI related package areas, and largely to meet your expectations we have added:

- [agent-sandbox](./agent-sandbox/README.md) - FUTURE
- [claudine](./claudine/README.md) - allows working cross-agent more consistent while offering some nice compositional strategies
- [research](./research/README.md) - research and organize topics, publish as either an "agent skill", a "deep dive doc" or both.
- [model-citizen](./model-citizen/README.md) - aids in the download, management, and serving of local LLM models

Everyone knows that _naming_ is hard but no one talks about how _grouping_ is equally as hard. Due to this we have decided not to name the category/group of the remaining packages:

- [homelab](./homelab/README.md) - integrations with popular automation platforms and standards as well as some AV equipment for those with universal remotes.
- [tabby](./tabby/README.md)
- [queue](./queue/README.md) - a TUI for queuing work to start later

> **Note:** it was pointed out the _convention_ would have had us use the `other` category name but then our well made point about naming and grouping being so hard would have fallen on deaf ears.

## Usage

- We have not yet pushed any of these packages to `crates.io` (though that is the eventual plan)
- For many of the CLI's we also _plan_ on publishing to the **npm** package manager too
- For now, however, if you want to use the libraries or CLI's in this monorepo you'll need to clone this monorepo:

    ```sh
    git clone https://github.com/yankeeinlondon/rusty-biscuit
    ```

Once you've cloned you're going to want to install the [`just`](https://github.com/casey/just) runner. This is used throughout this monorepo to organize all key devops operations. Install with:

```sh
# macOS
brew install just
# ubuntu / debian
apt install just
# windows
winget install Casey.Just
```

> **Windows:** recipes run through bash, so Cygwin or Git for Windows must be
> on PATH before `just` can do anything. Run the PowerShell preflight, which
> checks that (and tells you exactly what to fix) before delegating:
> `powershell -ExecutionPolicy Bypass -File scripts\init.ps1`


<details>
<summary>Other Package Managers</summary>
<pre><code lang=sh>
asdf install just
# Alpine
apk add just
# Red Hat, CentOS, Rocky, etc.
dnf install just
# Arch, Manjaro, etc
pacman -S just
# Snap package manager
snap install --edge --classic just
# Nix Package manager
nix-env -iA nixpkgs.just
# Use NodeJS's **npm** package manager
npm install -g rust-just
# Python's **uv** package manager
uv tool install rust-just
</code></pre>
</details>
<br><p>

Once installed run `just init` from the repo's root which will:

- ensure you have all necessary build tools for your operating system
- install and verify the repository-pinned kache compiler cache
- install some core CLIs from this monorepo to make sure all your `justfile` configurations will work with full fidelity

See [Development Environment Initialization](./docs/initization.md) for the
complete process, platform behavior, and troubleshooting guidance.

At this point you're ready to explore, install, test, whatever you like.

- run `just` and it will give you all the "recipes" appropriate for the directory you are in
- it will include things like:
  - testing
  - linting
  - installing
  - documentation drift
  - skill generation
  - _and more_

## Shell Completions

Shell completions help people learn new CLI's as well as navigate a CLI they don't use that often. All of the CLI's in this monorepo have shell completions included for all the major shells (bash, zsh, fish). How to include the shell completions for each CLI is available as part of the CLI's help system but if you are using zsh or bash you can use my conditional script which will add shell completions for the CLI's in this monorepo (and `just`) which you have installed: [Shell Completions](./docs/shell-completions.md).

## Local Development

### Pre-push Hook

A local pre-push hook is available to run fast feedback tests before pushing to remote.

Link the shared hook into your local git repository:

```sh
ln -s ../../.githooks/pre-push .git/hooks/pre-push
```

The hook's behavior is controlled by the `RUSTY_BISCUIT_PRE_PUSH` environment variable:

| Value | Behavior |
| --- | --- |
| `off` | Skip tests entirely and allow the push |
| `warn` | Run tests, print failures in red, but still allow the push (default) |
| `strict` | Run tests and block the push if any test fails |

For example, to enable strict mode in your shell:

```sh
export RUSTY_BISCUIT_PRE_PUSH=strict
```

The hook resolves the area list with this priority order:

1. **Explicit override** — `RUSTY_BISCUIT_PRE_PUSH_AREAS` (space-separated area names) is used verbatim if set.
2. **Top-level-directory heuristic** — `just changed-areas` runs `git diff --name-only` against the configured upstream branch (`@{u}`), then matches the first path segment of each changed file against the curated area list in the root `justfile`. This is a coarse detector: it does not inspect `Cargo.toml` path dependencies, so a change to a workspace member outside one of the curated top-level directories will not be detected.
3. **Fallback** — when there is no upstream branch (e.g. a first push of a new branch) or when no changed files map to a curated area, the hook falls back to testing `claudine` and `darkmatter`.

Override with:

```sh
export RUSTY_BISCUIT_PRE_PUSH_AREAS="claudine darkmatter"
```

Fully dependency-aware detection (mapping changed files to workspace members via `cargo metadata`) is a planned follow-up — see requirement R2 in [`features/2026-05-19-ci-cd/spec.md`](./features/2026-05-19-ci-cd/spec.md).

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0-or-later).

You are free to use, modify, and redistribute this software under the terms of that license. See the [`LICENSE`](./LICENSE) file for full details.

> **Note:** If you run this software as a service, you must provide a link to the source code of the running version.
