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
- install some core CLIs from this monorepo to make sure all your `justfile` configurations will work with full fidelity

The [kache](https://github.com/kunobi-ninja/kache) compiler cache is installed by `just init` on macOS and Linux but never activated by the repository: it is on for macOS (store and `target/` on one APFS volume), on for Linux only when a clone probe passes, and off for Windows and WSL. `just kache-status` reports where this host stands; the ruling is in `docs/kache-strategy.md`.

See [Development Environment Initialization](./docs/initialization.md) for the
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

A local pre-push hook runs `just ci-local --l2`: lint plus L1 and hostable non-focusing L2 for source-changed packages. It detects macOS, Linux, native Windows, or WSL2 with `sniff`. CI compiles unchanged direct reverse dependencies inside the changed package's Ubuntu check; they receive no separate jobs. For a clean outgoing `HEAD`, both passing and failing complete validation outcomes are published per package, environment, and tier. CI reuses qualifying cells and applies reused failures to the owning area's verdict. A comparison containing only documentation changes gates no packages.

Run `just ci-local --plan` to preview execution, reused evidence, and persistent execution constraints before pushing. The hook reviews each pushed branch's committed plan against those constraints and passes the reviewed plan to clean local validation, which skips cells already covered by qualifying passing evidence. Scope is published independently of validation; incomplete runs and explicit package overrides provide no complete reusable validation evidence.

Link the shared hook into your local git repository (`just init` does this):

```sh
ln -s ../../.githooks/pre-push .git/hooks/pre-push
```

The hook's behavior is controlled by the `RUSTY_BISCUIT_PRE_PUSH` environment variable:

| Value | Behavior |
| --- | --- |
| `scope-only` | Review constraints and publish scope; run no local gates |
| `off` | Deprecated alias of `scope-only` |
| `warn` | Publish complete passing or failing outcomes and allow the push |
| `strict` | Publish complete outcomes, then block the push on failure (default) |

For example, to enable strict mode in your shell:

```sh
export RUSTY_BISCUIT_PRE_PUSH=strict
```

`RUSTY_BISCUIT_PRE_PUSH_AREAS` (space-separated package names or area directories) replaces the computed scope with a fixed selection:

```sh
export RUSTY_BISCUIT_PRE_PUSH_AREAS="claudine darkmatter"
```

## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0-or-later).

You are free to use, modify, and redistribute this software under the terms of that license. See the [`LICENSE`](./LICENSE) file for full details.

> **Note:** If you run this software as a service, you must provide a link to the source code of the running version.
