# Publishing to crates.io from Rusty Biscuit

This document is a practical guide for publishing Rust crates from this monorepo to [crates.io](https://crates.io/), starting with `biscuit-hash` and `biscuit-terminal`.

The goal is not just "how do I run `cargo publish`?" but how to set up a workflow that stays fast, safe, and repeatable as more crates become public.

## Short Recommendation

For this repo, the lowest-friction path is:

1. Publish library crates first, not every crate in the workspace.
2. Use **independent crate versions**, not lockstep versions across the whole monorepo.
3. Keep using native Cargo commands, but wrap the repetitive checks in `just`.
4. Add `cargo-semver-checks` before publishing updates to already-published libraries.
5. Start with **manual publishing plus strong preflight automation**.
6. Add `release-plz` later if you want CI-driven release PRs and changelog automation.

That gives you a strong baseline without forcing a heavyweight release process before you need it.

## Mental Model

Publishing on crates.io has a few properties that shape everything else:

- Publishing is **append-only**. You cannot replace an uploaded version.
- You can **yank** a version to stop new downloads from selecting it, but yanking does not delete it.
- Each published crate needs a unique name on crates.io.
- `docs.rs` usually builds documentation automatically after publish.
- Crates can depend on each other across the same workspace, but published crates must use **versioned dependencies**, not path-only dependencies.

That means your release workflow should optimize for:

- catching mistakes before upload,
- keeping version bumps intentional,
- making dependency relationships explicit,
- and reducing the number of crates you publish unless they genuinely need to be public.

## Repo-Specific Starting Point

I validated the current state with Cargo locally.

### `biscuit-hash`

`cargo publish --dry-run -p biscuit-hash` currently succeeds, but Cargo warns that the manifest is missing important package metadata:

- `description`
- `license` or `license-file`
- `documentation`
- `homepage`
- `repository`

It also does not currently declare a package `readme`.

### `biscuit-terminal`

`cargo publish --dry-run -p biscuit-terminal` currently fails because:

- `biscuit-terminal` depends on `biscuit-hash` via a **path-only dependency**
- published crates must specify a dependency version

In practice, this dependency should look more like:

```toml
[dependencies]
biscuit-hash = { version = "0.1.0", path = "../../biscuit-hash/lib" }
```

That keeps local workspace development working while allowing Cargo to strip the path and use the crates.io version during publish.

There is also a metadata inconsistency today:

- `biscuit-terminal/lib/Cargo.toml` points `repository` at `https://github.com/yankeeinlondon/dockhand`
- `biscuit-terminal/cli/Cargo.toml` points `repository` at `https://github.com/yankeeinlondon/rusty-biscuit`

That should be made consistent before publishing.

## Recommended Publishing Strategy for This Monorepo

### 1. Publish only the crates that should be public

Do not assume every workspace member should go to crates.io.

A healthy split is:

- **public library crates**: meant for external reuse
- **public CLI crates**: only if you want `cargo install <name>` support
- **private/internal crates**: keep unpublished with `publish = false`

For this repo, a sensible first wave is:

- `biscuit-hash`
- `biscuit-terminal`

Potentially later:

- `biscuit-terminal-cli`
- `biscuit-hash-cli`

But I would treat the CLIs as optional for the first release pass. Publishing the libraries first keeps the surface area smaller.

### 2. Use independent crate versioning

Do not version the whole monorepo in lockstep unless you explicitly want a "single product release" model.

For this workspace, independent versions are the better fit because:

- crates have different maturity levels,
- not every crate will be published,
- some crates will change much more often than others,
- and lockstep versions create unnecessary churn.

Examples:

- `biscuit-hash` can move from `0.1.0` to `0.2.0` without forcing `biscuit-terminal` to do the same.
- `biscuit-terminal` can depend on `biscuit-hash = "0.1"` or `=0.1.3`, depending on how strict you want compatibility to be.

### 3. Keep path dependencies, but always pair them with versions for publishable crates

Inside a monorepo, path dependencies are convenient and should remain.

For any crate that might be published, use this pattern:

```toml
[dependencies]
some-workspace-crate = { version = "0.3.0", path = "../some-workspace-crate" }
```

That gives you:

- local development via workspace paths,
- publishability via crates.io versions,
- and an explicit compatibility contract between crates.

### 4. Mark non-public crates explicitly

For crates you do not want on crates.io, add:

```toml
[package]
publish = false
```

That removes ambiguity and prevents accidental publication.

### 5. Centralize shared package metadata at the workspace root

Cargo supports inheriting selected package metadata from `[workspace.package]`.

This is a good fit for fields that should be consistent across many published crates, such as:

- `edition`
- `license`
- `repository`
- `homepage`
- `documentation`
- `rust-version`

Using workspace inheritance reduces manifest drift and makes publishability easier to maintain across many crates.

## Version Management

### SemVer in practice

Rust follows normal Semantic Versioning, with an important twist for pre-1.0 crates:

- `1.x.y`:
  - breaking change -> bump `x`
  - backward-compatible feature -> bump `y`
  - bug fix -> bump `z`
- `0.x.y`:
  - breaking change -> bump `x`
  - backward-compatible feature or fix -> usually bump `y`

The key rule is: **the left-most non-zero component communicates compatibility**.

So for pre-1.0 crates:

- `0.1.0` -> `0.1.1` for very small fixes only
- `0.1.0` -> `0.2.0` for changes that users may need to react to

If you want a simpler discipline for early releases, use this:

- patch: docs-only changes, packaging fixes, trivial bug fixes
- minor: any API addition, behavior change, dependency policy change, or possible break

That is conservative, but it keeps you out of trouble.

## Suggested policy for this repo

For the first public phase:

- keep new public crates at `0.y.z`
- treat **any potentially breaking public API change** as a minor bump
- avoid frequent patch churn unless the change is truly tiny
- do not publish "just because code changed"; publish when a crate reached a meaningful external improvement

## Dependency version ranges

For internal relationships between published crates, choose deliberately:

- `version = "0.1.0"` means compatible-within-series selection
- `version = "=0.1.0"` means exact pin

In most libraries, prefer the normal compatible requirement unless you have a strong reason to force exact versions.

## First Publish Checklist

Use this for any crate’s initial publish.

1. Confirm the crate name is available on crates.io.
2. Add complete package metadata:
   - `description`
   - `license` or `license-file`
   - `repository`
   - `homepage` if useful
   - `documentation` if you want an explicit docs URL
   - `keywords`
   - `categories`
   - `readme`
   - `rust-version`
3. Make sure the README explains what the crate does, how to install it, and a small usage example.
4. Ensure public library APIs have enough rustdoc to make the docs.rs page useful.
5. Replace path-only dependencies with `version + path` for any published dependency.
6. Mark internal-only crates with `publish = false`.
7. Run the normal quality gates:
   - `just test`
   - `just lint`
8. Inspect exactly what will be shipped:
   - `cargo package --list -p <crate>`
9. Run a full dry run:
   - `cargo publish --dry-run --locked -p <crate>`
10. Publish:
    - `cargo publish --locked -p <crate>`
11. Verify:
    - crates.io page
    - docs.rs build
    - install/use from a clean environment

## Authentication and Ownership

Before the first real publish, decide how you want release credentials handled.

- For local publishing, `cargo login` is the simplest starting point.
- For CI later, prefer environment-based secrets such as `CARGO_REGISTRY_TOKEN` instead of storing tokens in repo files.
- Use a dedicated crates.io token for publishing rather than reusing one everywhere.
- Add at least one additional owner with `cargo owner` once a crate matters to you operationally.
- If you later move to GitHub Actions based publishing, evaluate trusted publishing so long-lived registry tokens are no longer required.

## Recommended Command Workflow

For `biscuit-hash` first:

```bash
just -f biscuit-hash/justfile test
just -f biscuit-hash/justfile lint
cargo package --list -p biscuit-hash
cargo publish --dry-run --locked -p biscuit-hash
cargo publish --locked -p biscuit-hash
```

Then wait until the new version is visible enough for downstream resolution, and publish `biscuit-terminal`:

```bash
just -f biscuit-terminal/justfile test
just -f biscuit-terminal/justfile lint
cargo package --list -p biscuit-terminal
cargo publish --dry-run --locked -p biscuit-terminal
cargo publish --locked -p biscuit-terminal
```

For the first release, I would not batch multiple crates into one command. Publish them deliberately, in dependency order.

## Tooling

You do not need a giant release system on day one. Use tools in layers.

### Native Cargo

Cargo already gives you the core release commands:

- `cargo package --list`
- `cargo publish --dry-run`cd .
- `cargo publish`
- `cargo owner`
- `cargo yank`

This should remain the foundation no matter what else you adopt.

### `cargo-semver-checks`

Use this for already-published libraries before releasing a new version.

Why it matters:

- it compares your current public API to a published baseline,
- it catches accidental breaking changes,
- and it is especially helpful once external users exist.

Good fit for:

- `biscuit-hash`
- `biscuit-terminal`

Less important for:

- unpublished crates
- purely internal crates

### `cargo-release`

This is a good **local, command-driven** release tool.

It helps with:

- version bumping
- tagging
- changelog/release steps
- publish orchestration

Good fit when:

- you want a single local release command,
- you prefer human-controlled releases from your machine,
- and you do not yet want CI-driven release PRs.

### `release-plz`

This is the better fit once you want **CI-assisted releases**.

It is designed around:

- release PRs,
- changelog generation,
- version bump suggestions,
- and workspace-aware publishing from CI

Good fit when:

- you want releases to become more automatic,
- you are comfortable with conventional commits,
- and you want publishing to happen from GitHub Actions instead of only locally.

### `cargo set-version`

This is a small, useful tool for directly updating versions in manifests without hand-editing everything.

It is helpful even if you do not adopt a full release tool yet.

### `cargo-smart-release`

This is worth evaluating later if many interdependent workspace crates become public.

Its value is graph-aware workspace release handling. For the current scope, it is probably more tool than you need.

## My Recommendation on Tool Adoption

### Phase 1: now

Adopt:

- native Cargo publish commands
- `just` recipes for publish checks
- `cargo-semver-checks`
- optional `cargo set-version`

Avoid for now:

- fully automated publish pipelines
- publishing lots of crates at once

### Phase 2: once 2-5 crates are public

Choose one:

- `cargo-release` if you want local-first releases
- `release-plz` if you want PR/CI-driven releases

If I were optimizing for **not slowing down your flow**, I would start with:

- manual publish,
- automated preflight checks,
- and only later adopt `release-plz`.

That keeps the mental model simple while still removing the tedious parts.

## Suggested `just` Automation

Once you want to automate the repetitive parts, add recipes like these:

```just
publish-check crate:
    just test
    just lint
    cargo package --list -p {{crate}}
    cargo publish --dry-run --locked -p {{crate}}

publish crate:
    cargo publish --locked -p {{crate}}
```

For published library crates, extend `publish-check` with a `cargo-semver-checks` step against the last released version.

I would keep actual publish separate from publish-check. That separation reduces accidental releases.

## Recommended Improvements Before First Publish

### `biscuit-hash`

- add package metadata in `biscuit-hash/lib/Cargo.toml`
- add a library README under `biscuit-hash/lib/README.md` or another explicit `readme` target
- consider adding `keywords`, `categories`, and `rust-version`

### `biscuit-terminal`

- change the `biscuit-hash` dependency from path-only to `version + path`
- fix the `repository` URL mismatch
- verify README, examples, and docs are what you want users to see on docs.rs/crates.io

### Workspace-wide

- add `[workspace.package]` for shared metadata
- mark non-public crates with `publish = false`
- decide which crates are intended to be public and document that policy

## A Sensible First Release Sequence

1. Make `biscuit-hash` publishable and release `0.1.0`.
2. Update `biscuit-terminal` to depend on the published `biscuit-hash` version.
3. Make `biscuit-terminal` publishable and release `0.1.0`.
4. Use each publish once from a clean consumer project to confirm the onboarding experience is good.
5. Only after that decide whether the CLI crates should also be published.

That keeps the first rollout narrow and gives you real feedback before you scale the process up.

## Primary References

- Cargo manifest metadata: [doc.rust-lang.org/cargo/reference/manifest.html](https://doc.rust-lang.org/cargo/reference/manifest.html)
- Cargo publishing guide: [doc.rust-lang.org/cargo/reference/publishing.html](https://doc.rust-lang.org/cargo/reference/publishing.html)
- `cargo publish`: [doc.rust-lang.org/cargo/commands/cargo-publish.html](https://doc.rust-lang.org/cargo/commands/cargo-publish.html)
- Cargo workspaces: [doc.rust-lang.org/cargo/reference/workspaces.html](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- Cargo SemVer reference: [doc.rust-lang.org/cargo/reference/semver.html](https://doc.rust-lang.org/cargo/reference/semver.html)
- `release-plz`: [release-plz.dev](https://release-plz.dev/)
- `cargo-release`: [github.com/crate-ci/cargo-release](https://github.com/crate-ci/cargo-release)
- `cargo-semver-checks`: [cargo-semver-checks.sigp.io](https://cargo-semver-checks.sigp.io/)
- `cargo-smart-release`: [github.com/Byron/cargo-smart-release](https://github.com/Byron/cargo-smart-release)
- `cargo-edit` / `cargo set-version`: [github.com/killercup/cargo-edit](https://github.com/killercup/cargo-edit)
