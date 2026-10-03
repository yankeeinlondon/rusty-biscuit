---
status: draft
created: 2026-10-03
owner: Ken Snyder <ken@ken.net>
origin: request to ship installable binaries on release (binstall, brew, uv, npm)
packages:
    - darkmatter-cli
    - dmls
    - sniff-cli
    - worktree-cli
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
| cargo-binstall (crates.io name is ours) | `cargo binstall darkmatter-cli` |
| cargo-binstall (crates.io name is taken) | `cargo binstall --git https://github.com/yankeeinlondon/rusty-biscuit sniff-cli` |
| Homebrew | `brew install yankeeinlondon/tap/md` |
| shell / PowerShell | `curl -LsSf <release>/darkmatter-cli-installer.sh \| sh` |
| npm | `npm i -g @yankeeinlondon/darkmatter-cli` |
| uv / pipx (PyPI) | `uv tool install darkmatter-cli` |

First release set (decided 2026-10-03): `md` + `dmls` (darkmatter),
`sniff`, `wt` (worktree), `hug` (tree-hugger), `claudine`, `bf`
(biscuit-file), `bh` (biscuit-hash).

## Decisions taken

- **crates.io: publish where the name is free, use the git route where it
  is taken.** As of 2026-10-03, these names belong to unrelated projects:
  `darkmatter`, `dmls`, `messenger`, `playa`, `sniff`, `sniff-cli`,
  `worktree`, `worktree-cli`.
- **Channels:** GitHub Release archives, cargo-binstall, Homebrew tap,
  shell/PowerShell installers, npm, PyPI (for `uv tool install`).
- **Tooling:** `dist` (axodotdev, formerly cargo-dist; v0.33.0 at the time
  of writing) builds the archives and generates the installers, the
  Homebrew formula, and the npm packages. It understands release-plz's
  `{package}-v{version}` tags without changes, and binstall detects its
  artifact layout automatically. dist cannot build Python wheels, so PyPI
  uses maturin with `bindings = "bin"`, run as a dist custom publish job.
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
   - 9 crates have no license;
   - 17 crates have no description.
4. **Test infrastructure ships as optional normal dependencies.**
   `biscuit-terminal`, `darkmatter`, `darkmatter-cli`, `sniff-cli`,
   `tree-hugger-cli`, and `worktree-cli` depend on `test-toolkit`,
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
- PyPI and npm names are free for `darkmatter-cli`, `dmls`, `worktree-cli`,
  `claudine-cli`, `biscuit-file`, and `biscuit-hash`. They are taken for
  `sniff`, `sniff-cli`, `tree-hugger`, and `claudine`, so those need a
  prefix on PyPI (open question 3).

### R3 — crates.io for the free names

Order by cost: `bh` and `bf` first (metadata only), then `hug`, then `md`
and `claudine`.

- Declare every workspace crate once in `[workspace.dependencies]` with both
  `path` and `version`, and have members use `{ workspace = true }`.
- Add `repository`, `license`, and `description` to every published crate.
- Move the test-infrastructure optional dependencies (constraint 4) out of
  the normal dependency graph.
- Move out-of-crate embeds (constraint 5) inside the crate, or behind
  `cfg(test)` in `tests/`.
- Rename the taken library crates at the package level only:
  `biscuit-darkmatter`, `biscuit-sniff`, `biscuit-worktree`,
  `biscuit-messenger`, `biscuit-playa`. Dependents write
  `darkmatter = { package = "biscuit-darkmatter", … }`, so `use` paths and
  binary names stay the same.
- release-plz: `publish = true` for the published set. `git_only = true`
  cannot be combined with publishing, so it is dropped for those crates;
  once the names on crates.io are ours, the "stranger's baseline" problem
  that motivated `git_only` no longer applies to them.
- **Needs:** a `CARGO_REGISTRY_TOKEN` secret. Publishing is permanent, so
  the first `cargo publish` of each crate is a manual, confirmed step.

## Open questions

1. **License** for the 9 unlicensed crates (`biscuit-hash`,
   `biscuit-hash-cli`, `biscuit-file-cli`, `biscuit-visualized`, `dmls`,
   `messenger`, `playa`, `tree-hugger`, `tree-hugger-cli`). Should they
   match the rest at `AGPL-3.0-only`?
2. **Taken CLI names:** rename the packages `sniff-cli`, `worktree-cli`,
   and `dmls` (e.g. `biscuit-sniff-cli`) so they can go to crates.io too, or
   keep the git route for them permanently?
3. **PyPI prefix** for taken names: `biscuit-sniff`, or `rb-sniff`?
4. **`claudine` timing:** ship it in R1 with glibc-only Linux builds, or
   hold it until the `2026-09-20-upgrade-reqwest` spec removes OpenSSL?
5. **Name squatting:** 24 of our crate names on crates.io are free today.
   Should placeholder versions be reserved before R3?
