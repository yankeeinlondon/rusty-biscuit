---
spike: S1
spec: 2026-10-28-publish
question: Does `dist` 0.34, with `precise-builds`, plan and build one CLI from a `{package}-v{version}` tag in this workspace?
answer: yes for `bh` — `dist 0.34.0` plans and builds only `biscuit-hash-cli` from `biscuit-hash-cli-v0.1.0`; but three of the other seven first-set CLIs cannot be built by dist until their test-helper binaries move out
confidence: high (run on this host, aarch64-apple-darwin only)
date: 2026-11-07
---

## Answer

Yes. In a scratch clone, with `precise-builds = true`, `dist plan
--tag=biscuit-hash-cli-v0.1.0` selected exactly one release
(`biscuit-hash-cli 0.1.0`), and `dist build` ran
`cargo build --profile dist --target aarch64-apple-darwin --package biscuit-hash-cli`
and produced `biscuit-hash-cli-aarch64-apple-darwin.tar.xz` in 8 s wall time.
The 77-member workspace, path dependencies without versions, and the nested
excluded workspaces caused no problem in plan or build.

Two manifest fixes are required first (they are already in the spec's scope),
and one new blocker affects `md`, `claudine`, and `sniff`.

dist `0.34.0` was released 2026-11-02, five days before this run; its only
change from 0.33.1 is the Apple Silicon runner (`macos-15`).

## Evidence

Tool: `cargo-dist-aarch64-apple-darwin.tar.xz` from the `v0.34.0` GitHub
release, checksum verified, unpacked into the scratch directory
(`dist --version` → `cargo-dist 0.34.0`). Nothing was installed globally.

### Steps and results

1. `dist init --yes --ci github --installer shell --installer powershell
   --installer homebrew -t aarch64-apple-darwin -t x86_64-apple-darwin
   -t x86_64-unknown-linux-gnu -t aarch64-unknown-linux-gnu
   -t x86_64-pc-windows-msvc` wrote `dist-workspace.toml` and added
   `[profile.dist]` (`inherits = "release"`, `lto = "thin"`) to the root
   `Cargo.toml`, then **failed** in `dist generate`:
   `Github CI support requires your crates to agree on the URL of your
   repository` — `agent-sandbox/cli` says `rusty-biscuit`,
   `darkmatter/cli` says `yankeeinlondon/dockhand`. The check spans every
   distributable package, not just the one being released.
2. After correcting `darkmatter/cli` in the scratch clone, `dist plan
   --tag=biscuit-hash-cli-v0.1.0` **failed**: `Github CI support requires you
   to specify the URL of your repository … biscuit-hash/cli/Cargo.toml`.
   `git-wt-cli`, `sniff-detection-cli`, and `claudine-cli` fail the same way.
3. With `repository` set on `biscuit-hash-cli`, the JSON plan has
   `announcement_tag: biscuit-hash-cli-v0.1.0`, `releases:
   [biscuit-hash-cli 0.1.0]`, and 16 artifacts; download path
   `/yankeeinlondon/rusty-biscuit/releases/download/biscuit-hash-cli-v0.1.0`.
4. `dist build --tag=biscuit-hash-cli-v0.1.0 --artifacts=local
   --target=aarch64-apple-darwin` succeeded (exit 0). The `bh` binary is
   1.2 MB, links only `libiconv` and `libSystem`, and prints `bh 0.1.0`.
5. With `precise-builds = false` the same build ran
   `cargo build --profile dist … --workspace` (all 77 members); it was
   stopped. `precise-builds = true` is required here.
6. Opt-in check: with `dist = false` at workspace level and
   `[package.metadata.dist] dist = true` (plus `formula = "bh"`) on
   `biscuit-hash-cli`, an untagged `dist plan` releases only
   `biscuit-hash-cli` and names the Homebrew formula `bh.rb`. A tag for a
   package that is not opted in (`darkmatter-cli-v0.1.0`) **errors**:
   `This workspace doesn't have anything for dist to Release!`.
7. `dist build --tag=darkmatter-cli-v0.1.0 …` compiled for 3 min 51 s, then
   **failed**: `failed to find bin darkmatter-argv-recorder`. That binary is
   declared with `required-features = ["test-fixtures"]`; dist counts it as
   part of the app but cargo does not build it. dist 0.34's config keys
   (read from the binary) include no per-binary exclusion.

### Asset names (from the plan)

```text
biscuit-hash-cli-aarch64-apple-darwin.tar.xz        (+ .sha256)
biscuit-hash-cli-x86_64-apple-darwin.tar.xz         (+ .sha256)
biscuit-hash-cli-aarch64-unknown-linux-gnu.tar.xz   (+ .sha256)
biscuit-hash-cli-x86_64-unknown-linux-gnu.tar.xz    (+ .sha256)
biscuit-hash-cli-x86_64-pc-windows-msvc.zip         (+ .sha256)
biscuit-hash-cli-installer.sh
biscuit-hash-cli-installer.ps1
biscuit-hash-cli.rb        (bh.rb with formula = "bh")
sha256.sum
source.tar.gz              (+ .sha256)
```

Archive layout: `biscuit-hash-cli-aarch64-apple-darwin/{bh,README.md,LICENSE}`.
README and LICENSE are the **monorepo root** files, because `biscuit-hash/cli`
has neither.

### First-set CLIs: binaries dist would ship

| Package | Extra binaries | Effect under dist |
|---|---|---|
| `biscuit-hash-cli` | none | builds |
| `biscuit-file-cli`, `tree-hugger-cli`, `darkmatter-lsp` | none | not built; no blocker seen |
| `darkmatter-cli` | `darkmatter-argv-recorder` (required-features) | **build fails** (observed) |
| `claudine-cli` | six `claudine-fake-*`/`argv-recorder` (required-features) | **build fails** (same mechanism; not run) |
| `sniff-detection-cli` | `render_cicd_fixture`, `render_git_status_fixture` (required-features) | **build fails** (same mechanism; not run) |
| `git-wt-cli` | `src/bin/render_dirty_tree.rs` (no required-features) | builds, but **ships a test helper** (not run) |

By default dist treats all 41 workspace packages that have a binary and no
`publish = false` as apps, including `test-toolkit`, `homelab-server`, and
the `messenger` library's stub binaries.

### Generated config (scratch clone, final state)

```toml
[workspace]
members = ["cargo:."]

[dist]
cargo-dist-version = "0.34.0"
ci = "github"
installers = ["shell", "powershell", "homebrew"]
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"]
install-path = "CARGO_HOME"
install-updater = false
# added by hand during the spike:
precise-builds = true
dispatch-releases = true
pr-run-mode = "skip"
create-release = false
dist = false
```

Copies of both generated workflows are in the scratch directory
(`release.generated.yml`, `release.dispatch.yml`), not in the worktree.

### What the generated workflow needs to be dispatched per tag by release-plz

The default `release.yml` triggers on `pull_request` and on `push: tags:
'**[0-9]+.[0-9]+.[0-9]+*'`. In this repository that cannot work as is:

1. **Tag pushes made with `GITHUB_TOKEN` trigger no workflow**
   (`docs/cicd/release-plz-ci.md` §"The token decides…"). release-plz runs
   with `GITHUB_TOKEN`. `dispatch-releases = true` replaces both triggers with
   `workflow_dispatch` and a `tag` input, which `GITHUB_TOKEN` *can* start.
   The release-plz job then loops over its `releases` output (the action
   exposes `releases` and `releases_created`) and runs
   `gh workflow run release.yml --ref <tag> -f tag=<tag>` once per **opted-in**
   package; it needs `actions: write`.
2. **`--ref <tag>` is required**, not optional: the dispatched workflow checks
   out the dispatch ref and passes `github.sha` as the release target.
   Dispatching on `main` would build `main`'s head under the tag's name.
3. **Library and non-opted-in tags must not be dispatched**: release-plz tags
   every package (79 tags exist, mostly libraries), and dist errors on a tag
   with nothing to release. The filter list should come from
   `docs/deployment.yaml`.
4. **Who creates the GitHub Release.** release-plz already creates one per
   tag (`git_release_enable = true`; e.g. `biscuit-hash-cli-v0.1.0`
   was published 2026-05-27 with no assets). With `create-release = false`
   dist runs `gh release upload <tag> artifacts/*` and
   `gh release edit <tag> --draft=false`, i.e. it attaches assets to
   release-plz's release and keeps its changelog body. The alternative
   in the spec (`git_release_enable = false`, dist creates the release) loses
   release-plz's notes. Either works mechanically; the first leaves a window
   where a published release has no assets.
5. **`pr-run-mode = "skip"`**: the default adds a `dist plan` job to every pull
   request, which this repository's CI-scope rules would not want.
6. Hand edits to `release.yml` (for example the dispatch loop, or a
   `RUSTC_WRAPPER: ""` env) make dist's CI-freshness check fail unless
   `allow-dirty = ["ci"]` is set; the dispatch loop belongs in
   `release-plz.yml`, which dist does not own.

## What this changes in the spec

- **Scope, "Adopt `dist` with `precise-builds`"**: confirmed; `dist` stays the
  tool. Add `dist = false` at workspace level with per-package opt-in (or
  generate opt-ins from `docs/deployment.yaml`), `dispatch-releases = true`,
  `pr-run-mode = "skip"`, and a decision on `create-release`.
- **Scope, "Fix each published crate's metadata"**: `repository` must be set on
  every opted-in crate *and* agree across every distributable crate, or
  `dist generate` refuses. The `darkmatter-cli` value blocks `dist init`
  outright. Add `readme` (or a package README) so archives do not ship the
  monorepo README, and `description`/`homepage` for Homebrew.
- **Scope, "widen to `md` … `claudine` … `sniff` … `wt`"**: new prerequisite —
  move test-helper binaries out of the shipped crates (for example into a
  `publish = false` test-support crate). `required-features` alone is not
  enough for dist.
- **Outcome table, Homebrew**: the formula is named after the package unless
  `formula = "bh"` is set per package.
- **Scope, release profile**: `dist init` adds `lto = "thin"` and no `strip`;
  the stripped, LTO profile is a hand edit (sizes are S3's question).
- **Set `git_release_enable = false` if `dist` owns the GitHub Release**: this
  is now a choice between two working designs (point 4 above), not a
  requirement.

## Surprises

- **Order moves**: the test-helper binaries in `darkmatter-cli`,
  `claudine-cli`, and `sniff-detection-cli` block widening past `bh`. They
  should be moved before the widening step, not discovered in a release run.
- With every package distributable (no opt-in), human-format `dist plan
  --tag=biscuit-hash-cli-v0.1.0` printed `announcing v0.1.0` and listed all 41
  apps, while `--output-format=json` and `dist build` for the same tag
  correctly selected only `biscuit-hash-cli`. The cause was not determined;
  with opt-in configured the human output was correct. Read the JSON.
- Each release also uploads a whole-repository `source.tar.gz`; with ~40
  releasable packages this repeats the whole repo per release.
  `source-tarball = false` exists and was not tested.
