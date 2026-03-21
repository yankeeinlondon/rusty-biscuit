# Rusty Biscuit

<img src="./assets/biscuit-and-crab.png" style="position: fixed; max-width: 30%; height: 200px; right: 0; top: 0; opacity: 0.75"></img>

> A set of tools of deterministic tools for a non-deterministic world

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
- For now, however, if you want to use the libraries or CLI's in this monorepo you'll need to clone it:

    ```sh
    git clone https://github.com/yankeeinlondon/rusty-biscuit
    ```

Once you've cloned you're going to want to install the [`just`](https://github.com/casey/just) runner. This is used throughout this monorepo to organize all key devops operations. Install with:

```sh
# macOS
brew install just
# ubuntu / debian
apt install just
```

<details>
<summary>Other Package Managers</summary>
<pre><code lang=sh>
asdf install just
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

Once installed you should run `just init` from the repo's root which will:

- ensure you have all necessary build tools for you operating system
- install some core CLI's from this monorepo to make sure all your `justfile` configurations will work with full fidelity

At this point you're ready to explore, install, test, whatever you like.

- run `just` and it will give you all the "recipes" appropriate for the directory you are in
- it will include things like:
    - testing
    - linting
    - installing
    - documentation drift
    - skill generation
    - _and more_

> **Notes:**
> 
> - **TTS**
>     - we use TTS messages to communicate successful or failing recipes (_typically those which are long running_)
>     - this leverages the `biscuit-speaks` library which in turn leverages what you have installed on your host computer
>     - most computer's will have _some_ TTS software we support but the quality of the TTS can vary widely
>     - run `so-you-say list-providers` to see which TTS providers we can use on your system
>         - on macOS the built-in `say` TTS is not bad but it's quality varies on how you've configured it
>         - on Windows the built-in `XXX` TTS is decent as well
>         - on Linux there is more variance but something like `espeak`/`espeak-ng` is most common. These voices are low quality (but the do have a TON of languages).
>     - if you want better voices, it's a safe bet that installing `kokoro_tts` will be a big improvement:
>         - run `sniff tts-clients install kokoro_tts` to install
> 
>     **Note:** when running `just init` you'll be automatically prompted to install one of the recognized TTS 
>     solutions if the host currently has none. You may also be "recommended" to upgrade if all you have is 
>     the **espeak** solution (common on Linux).
> 
> - **Audio Playback**
>     - several of the just recipes will play sound effects when certain events take place
>     - in addition, some TTS providers, rely on the `playa` library to play their voice audio
>     - We support native audio on macOS, Linux, and Windows
>     - In most cases this is all you'll need but we will fallback to any headless audio players detected on the host which meet the requirements if the native solution can't perform the particular audio task.
>     - You can check what headless audio players exist on your host by running `playa players`
>     - You can install any which are missing with `playa install`




## License

This project is licensed under the GNU Affero General Public License v3.0 (AGPL-3.0-or-later).

You are free to use, modify, and redistribute this software under the terms of that license. See the [`LICENSE`](./LICENSE) file for full details.

> **Note:** If you run this software as a service, you must provide a link to the source code of the running version.

