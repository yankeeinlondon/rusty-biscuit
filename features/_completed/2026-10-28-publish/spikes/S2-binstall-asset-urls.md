---
spike: S2
spec: 2026-10-28-publish
question: Which asset URLs does binstall try for a `{package}-v{version}` tag, and do they match `dist`'s asset names?
answer: binstall's defaults try only tags `{version}` and `v{version}`, never `{package}-v{version}`; the file names match dist's, the tag path does not — every crate needs an explicit `pkg-url`
confidence: high
date: 2026-11-07
---

## Answer

No match without configuration. binstall's default templates combine ten file
name shapes with two GitHub tag paths, `releases/download/{version}/` and
`releases/download/v{version}/`. One of the file names,
`{ name }-{ target }{ archive-suffix }`, is exactly dist's asset name
(`biscuit-hash-cli-aarch64-apple-darwin.tar.xz`), but under the wrong tag.
With this repository's `biscuit-hash-cli-v0.1.0` tags, binstall finds nothing.

This one template, set in each crate's manifest, produces the dist URL:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/{ name }-v{ version }/{ name }-{ target }{ archive-suffix }"
pkg-fmt = "txz"
bin-dir = "{ name }-{ target }/{ bin }{ binary-ext }"

[package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
pkg-fmt = "zip"
```

So yes, each crate needs an explicit `pkg-url`. The block is the same text for
every crate because `{ name }` is the package name, so it can be generated or
checked from `docs/deployment.yaml`. Whether binstall reads a
`[workspace.metadata.binstall]` default was not determined.

## Evidence

Scratch manifest (`name = "biscuit-hash-cli"`, `version = "0.1.0"`,
`repository = "https://github.com/yankeeinlondon/rusty-biscuit"`,
`[[bin]] name = "bh"`), run with
`cargo binstall --manifest-path <dir> biscuit-hash-cli --dry-run --no-confirm
--strategies crate-meta-data --log-level trace --targets <triple>`.

| binstall | Default tags tried | `{name}-v{version}` tried |
|---|---|---|
| 1.19.1 (`~/.cargo/bin`) | `0.1.0` (300 URLs), `v0.1.0` (300 URLs) | no |
| 1.26.2 (latest, 2026-10-31, scratch copy) | same | no |

Among the candidates for `v0.1.0`:
`v0.1.0/biscuit-hash-cli-aarch64-apple-darwin.tar.xz` — dist's file name
under the wrong tag. The run ended with `Fallback to cargo-install is
disabled`, as expected with no assets.

Source confirmation, `crates/binstalk-fetchers/src/gh_crate_meta/hosting.rs`
at `v1.26.2`: `GITHUB_RELEASE_PATHS` are `{ repo }/releases/download/{ version }`,
`…/v{ version }`, `…/{ subcrate }%2F{ version }`, and
`…/{ subcrate }%2Fv{ version }`. The `subcrate` forms would match a
`{package}/v{version}` tag (slash), not release-plz's `{package}-v{version}`
(hyphen).

With the `pkg-url` above, the attempted URLs were:

```text
aarch64-apple-darwin:
  …/releases/download/biscuit-hash-cli-v0.1.0/biscuit-hash-cli-aarch64-apple-darwin.tar.xz   ← dist asset
  …/releases/download/biscuit-hash-cli-v0.1.0/biscuit-hash-cli-aarch64-apple-darwin.txz
  …/releases/download/bh-v0.1.0/bh-aarch64-apple-darwin.tar.xz
  …/releases/download/bh-v0.1.0/bh-aarch64-apple-darwin.txz
x86_64-pc-windows-msvc:
  …/releases/download/biscuit-hash-cli-v0.1.0/biscuit-hash-cli-x86_64-pc-windows-msvc.zip   ← dist asset
  …/releases/download/bh-v0.1.0/bh-x86_64-pc-windows-msvc.zip
```

binstall also substitutes the binary name (`bh`) for `{ name }`; those extra
probes are harmless 404s.

Not tested: a real download and unpack (no assets exist), so the `bin-dir`
value is matched to the archive layout S1 observed
(`biscuit-hash-cli-<target>/bh`), not proven.

## What this changes in the spec

- **Spikes needed → binstall**: settled. Add to Scope: "every published crate
  carries the `[package.metadata.binstall]` block above, generated or checked
  from `docs/deployment.yaml`."
- **Outcome table, cargo-binstall row**: `cargo binstall --git … biscuit-hash-cli`
  reads the crate's manifest from the clone, so the crate needs both
  `repository` (S1) and this `pkg-url` block in the tagged commit.

## Surprises

- The locally installed binstall is 1.19.1, seven minor versions behind
  1.26.2. Both behave the same for this question.
- Renaming the tags to `{package}/v{version}` would let binstall's defaults
  match through `subcrate` (not tested), but release-plz's `git_tag_name` and
  every existing tag use the hyphen form; not a recommendation.
