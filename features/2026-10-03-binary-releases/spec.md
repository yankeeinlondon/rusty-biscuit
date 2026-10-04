---
status: draft
created: 2026-10-03
owner: Ken Snyder <ken@ken.net>
origin: request to ship installable binaries on release (binstall, brew, uv, npm)
packages:
    - darkmatter-cli
    - darkmatter-lsp
    - sniff-detection-cli
    - git-wt-cli
    - tree-hugger-cli
    - claudine-cli
    - biscuit-file-cli
    - biscuit-hash-cli
---

# Installable binaries on release

## Outcome

When release-plz tags a CLI package (`{package}-v{version}`), a release
workflow builds that CLI for every supported target, attaches the archives to
the package's GitHub Release, and publishes it to each channel below. A user
installs it with any of:

| Channel | Command (example: `md`) |
|---|---|
| cargo-binstall | `cargo binstall darkmatter-cli` |
| Homebrew | `brew install yankeeinlondon/tap/md` |
| shell / PowerShell | `curl -LsSf <release>/darkmatter-cli-installer.sh \| sh` |
| npm | `npm i -g @yankeeinlondon/darkmatter-cli` |
| uv / pipx (PyPI) | `uv tool install darkmatter-cli` |

First release set (decided 2026-10-03): `md` (`darkmatter-cli`) + `dmls`
(`darkmatter-lsp`), `sniff` (`sniff-detection-cli`), `wt` (`git-wt-cli`),
`hug` (`tree-hugger-cli`), `claudine` (`claudine-cli`), `bf`
(`biscuit-file-cli`), `bh` (`biscuit-hash-cli`).

## Decisions taken

- **crates.io: publish where the name is free, use the git route where it
  is taken.** As of 2026-10-03, these names belong to unrelated projects:
  `darkmatter`, `dmls`, `messenger`, `playa`, `sniff`, `sniff-cli`,
  `worktree`, `worktree-cli`.
- **The three CLI packages with taken names were renamed** (decided
  2026-10-03) so every CLI in the first set can go to crates.io: `dmls` →
  `darkmatter-lsp`, `sniff-cli` → `sniff-detection-cli`, `worktree-cli` →
  `git-wt-cli`. Only the package name changed; the binaries (`dmls`,
  `sniff`, `wt`) and library crate names (`dmls`, `sniff_cli`,
  `worktree_cli`) are unchanged. All three new names were free on crates.io
  and PyPI on 2026-10-03. `git-wt-cli` is taken on unscoped npm, which does
  not matter because npm packages live under `@yankeeinlondon`. The git
  route (`cargo binstall --git …`) remains available for every CLI.
- **Channels:** GitHub Release archives, cargo-binstall, Homebrew tap,
  shell/PowerShell installers, npm, PyPI (for `uv tool install`).
- **Tooling:** `dist` (axodotdev, formerly cargo-dist; v0.33.0 at the time
  of writing) builds the archives and generates the installers, the
  Homebrew formula, and the npm packages. It understands release-plz's
  `{package}-v{version}` tags without changes, and binstall detects its
  artifact layout automatically. dist cannot build Python wheels, so PyPI
  uses maturin with `bindings = "bin"`, run as a dist custom publish job.
- **Every first-party crate is `AGPL-3.0-only`** (decided 2026-10-03),
  including the six that previously declared MIT (`biscuit-contract`,
  `biscuit-file`, `reaper`, `reaper-cli`, `model-citizen`,
  `model-citizen-cli`). Applied to every workspace member and the excluded
  `zed-dmls`; test fixtures and spikes are not crates we ship.
- **Prebuilt binaries include native audio** (decided 2026-10-04).
  `claudine` ships in R1 with `playa`'s `native-playback` and
  `biscuit-speaks`, on macOS, Windows, and glibc Linux. On Linux that links
  `libasound` (`alsa-sys` via `rodio` → `cpal`), the binary's only dynamic
  system-library dependency now that OpenSSL is gone. The Linux installers
  and docs state the `libasound2` requirement. There is no musl build of
  `claudine`. Runtime discovery of TTS providers and host players is
  unchanged; it starts only once the binary has loaded, which is why the
  ALSA library itself must be present.
- **release-plz keeps versioning, changelogs, and tags; dist owns the GitHub
  Release.** Set `git_release_enable = false` in `release-plz.toml`.

## Constraints discovered

1. **Tags pushed with `GITHUB_TOKEN` trigger no workflow.** The release-plz
   job therefore has to start the dist workflow explicitly, via
   `workflow_dispatch` (exempt from that rule) with the tag as input, using
   dist's `dispatch-releases = true`. The dist build must check out the tag,
   not `main`. The alternative is a GitHub App token for release-plz.
   For the same reason, every downstream publisher (Homebrew, npm, PyPI)
   runs as a dist `publish-jobs` entry, never as a separate
   `on: release` workflow.
2. **The dependency closure is 32 workspace crates.** `claudine-cli` alone
   pulls in 20, including `darkmatter-cli` as a library. `sniff` and
   `biscuit-speaks` reach into the excluded `schematic/schema` workspace
   (`schematic-schema` → `schematic-definitions` → `schematic-define`).
3. **Publishing to crates.io is blocked in every crate today:**
   - no crate sets `repository`;
   - no workspace path dependency carries a `version`;
   - 9 crates had no license (resolved below);
   - 17 crates have no description.
4. **Test infrastructure ships as optional normal dependencies.**
   `biscuit-terminal`, `darkmatter`, `darkmatter-cli`, `sniff-detection-cli`,
   `tree-hugger-cli`, and `git-wt-cli` depend on `test-toolkit`,
   `biscuit-test-harness`, `biscuit-browser-harness`, and
   `biscuit-visualized` behind features. Two of those are `publish = false`,
   and crates.io requires every optional dependency to be published.
5. **Some crates embed files from outside their own directory:**
   - `claudine` and `claudine-cli` embed `prompts/` and `claudine/docs/`;
   - `darkmatter` and `darkmatter-cli` embed `darkmatter/docs/{schemas,topics}`;
   - `playa`'s `build.rs` reads `../effects`, it embeds 88 audio files, and
     it is at risk of crates.io's 10 MB limit;
   - `sniff`'s tests embed the root `Cargo.lock`.

   `cargo package` rejects all of these. They don't affect the git route.
6. **Native build requirements:**
   - `bh` and `bf` need only a C compiler at most.
   - `hug` compiles 17 tree-sitter C grammars.
   - `md`, `dmls`, `sniff`, and `wt` need cmake for `aws-lc-sys` and build
     `pcre2-sys`; on macOS, `sniff` pulls in `coreaudio-sys`, which needs
     libclang.
   - `claudine` adds ALSA (`libasound2-dev`) and OpenSSL. The OpenSSL comes
     from `messenger`, whose `reqwest 0.12` uses native-tls, and the
     `2026-09-20-upgrade-reqwest` spec removes it. Until then, a musl build
     of `claudine` is not realistic.
   - Unstripped sizes reach 120 MB (`claudine`), above PyPI's 100 MB
     per-file default, so the dist profile must strip and use LTO.

## Plan

### R1 — release pipeline, git route only (no registry)

Delivers: GitHub Release archives, binstall `--git`, shell/PowerShell
installers, and Homebrew, for the 8 CLIs.

- `dist init` at the workspace root. Opt in only the 8 CLI packages
  (`[package.metadata.dist] dist = true`) and set `precise-builds = true`,
  so a tag builds only its own package.
- Targets: `x86_64`/`aarch64` × `apple-darwin`, `unknown-linux-gnu`, plus
  `x86_64-pc-windows-msvc`. musl and Windows ARM come later.
- Add a `[profile.dist]` with `strip = true` and `lto = "thin"`.
- System dependencies through dist's `dependencies` key: cmake everywhere,
  and `libasound2-dev` on apt for `claudine`.
- `release-plz.toml`: set `git_release_enable = false`. In
  `release-plz.yml`, after `release` creates tags, dispatch the dist
  workflow once per created tag.
- Homebrew: `tap = "yankeeinlondon/homebrew-tap"`, `publish-jobs =
  ["homebrew"]`.
- **Needs from Ken:** create the `yankeeinlondon/homebrew-tap` repository
  and add a `HOMEBREW_TAP_TOKEN` secret (a fine-grained PAT with contents
  write on the tap).
- Verify with a dry run first (`dist plan`, then a manual dispatch against
  an existing tag), before any real tag publishes.

### R2 — npm and PyPI

- npm: add the `npm` installer, scope `@yankeeinlondon`, and add `"npm"` to
  `publish-jobs`. **Needs:** an `NPM_TOKEN` secret.
- PyPI: one `pyproject.toml` per CLI (`[tool.maturin] bindings = "bin"`,
  `manifest-path` pointing at the CLI crate), built by `maturin-action`
  across the same targets, and published with PyPI trusted publishing from a
  dist custom publish job (`./.github/workflows/publish-pypi.yml`,
  `workflow_call`). **Needs:** a pending trusted publisher registered on
  PyPI for each project name.
- **PyPI project name = the Cargo package name** (decided 2026-10-04),
  falling back to a `biscuit-` prefix only where the Cargo name is taken on
  PyPI. All eight are free as of 2026-10-04 (`darkmatter-cli`,
  `darkmatter-lsp`, `git-wt-cli`, `sniff-detection-cli`, `tree-hugger-cli`,
  `claudine-cli`, `biscuit-file-cli`, `biscuit-hash-cli`), so no prefix is
  needed today: `uv tool install sniff-detection-cli`.

### R3 — crates.io for the free names

**Starts early, in parallel with R1** (decided 2026-10-04). Names are
protected by publishing real crates soon, not placeholders: crates.io's usage
policy prohibits name squatting. `biscuit-hash`/`biscuit-hash-cli` and
`biscuit-file`/`biscuit-file-cli` go first because they need only metadata
fixes. The CLIs whose names matter most follow as their blockers clear.

Order by cost: `bh` and `bf` first (metadata only), then `hug`, then `md`,
`dmls` (`darkmatter-lsp`), `sniff` (`sniff-detection-cli`), `wt`
(`git-wt-cli`), and `claudine`.

- Declare every workspace crate once in `[workspace.dependencies]` with both
  `path` and `version`, and have members use `{ workspace = true }`.
- Add `repository` and `description` to every published crate.
- Move the test-infrastructure optional dependencies (constraint 4) out of
  the normal dependency graph.
- Move out-of-crate embeds (constraint 5) inside the crate, or behind
  `cfg(test)` in `tests/`.
- Rename the taken library crates at the package level only:
  `biscuit-darkmatter`, `biscuit-sniff`, `biscuit-worktree`,
  `biscuit-messenger`, `biscuit-playa`. Dependents write
  `darkmatter = { package = "biscuit-darkmatter", … }`, so `use` paths and
  binary names stay the same.
- The renamed packages have no `{package}-v{version}` tags under their new
  names, so release-plz treats each as an initial release at its current
  version.
- release-plz: `publish = true` for the published set. `git_only = true`
  cannot be combined with publishing, so it is dropped for those crates;
  once the names on crates.io are ours, the "stranger's baseline" problem
  that motivated `git_only` no longer applies to them.
- **Needs:** a `CARGO_REGISTRY_TOKEN` secret. Publishing is permanent, so
  the first `cargo publish` of each crate is a manual, confirmed step.

## Open questions

None as of 2026-10-04.
