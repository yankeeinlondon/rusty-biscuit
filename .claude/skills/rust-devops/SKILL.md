---
name: rust-devops
description: |
  Use for Rust CI/CD and release engineering: GitHub Actions, affected-package
  scope, pre-push evidence, release-plz, package distribution, compiler caches,
  build storage, or programmatic Git through git2/libgit2 and gitoxide/gix.
  In rusty-biscuit, load this before changing CI scope, local-validation reuse,
  release automation, or repository-wide Rust build caching.
---
# Rust DevOps

This skill covers CI orchestration, releases, distribution, build performance,
and Git integration for Rust projects. In rusty-biscuit, repository policy and
measured results override generic ecosystem guidance.

## Choose the owning surface

| Work | Read first |
|---|---|
| CI scope, pre-push behavior, local evidence, GitHub Actions, release-plz | [CI/CD and releases](./ci-cd.md) |
| Bumping or reading the version of a CI document (plan, receipts, manifests, results, baseline, environment table) | `docs/cicd/schema-versions.md` — the one changelog, its bump rules, and every mirror that must move together |
| How release-plz itself works, and how to configure its workflow | `docs/cicd/release-plz-ci.md` (tool-level, repository-neutral) |
| Distribution channels and installers | [Deployment platforms](./deployment-platforms.md) |
| Compiler caching and build storage | [kache](./kache.md), then `docs/kache-strategy.md` for rusty-biscuit |
| Embedded Git implementation | [git2](./git2.md) or [gitoxide](./gitoxide.md), according to the decision below |

The supporting deployment and Git-library pages are dated research snapshots,
not dependency pins or authorization to publish. Verify current versions and
external service requirements against primary sources before adopting them.

## Deployment

Rust can ship native binaries without a language runtime, but native libraries,
target triples, signing, and installer conventions still determine portability.
Choose one canonical artifact source, then make package-manager manifests thin
projections of those immutable artifacts.

For rusty-biscuit, release-plz owns versioning, changelogs, tags, and GitHub
releases. Crates.io publication remains disabled; do not add a registry or
installer merely because the deployment survey lists it.

## Build Caching

Treat caching as a measured storage-and-filesystem decision, not a default Rust
optimization. Compare total wall time and storage amplification against a
no-cache control, including restore/save overhead and immutable CI cache
behavior.

Rusty-biscuit tracks no `RUSTC_WRAPPER` and CI explicitly clears it. Kache is a
host opt-in only where `just kache-status` proves clone-capable storage between
the store and that checkout's `target/`; Windows and WSL remain off. Never mix
wrapped and unwrapped builds in one `target/`.

## Git Interaction

Programmatic Git goes through a library, in-process. Shelling out to `git`
from Rust is the exception, not the default: every process costs ~5 ms on
macOS and ~47 ms on Windows (measured 2026-10-05), its output is text to
parse, and its behavior moves with the installed Git version.

- **Default to `gix`** for anything a program reads or writes: refs, config,
  remotes, objects, status, diffs. It is pure Rust, honors Git's trust model
  and `GIT_CONFIG_*` environment, and is already in the build through
  `sniff` and `worktree`.
- **Use `git2`** only where `gix` has no API yet (`sniff` keeps it for fixture
  writes such as checkout and worktree creation).
- **Use the CLI only for non-programmatic interactions**: an operation done on
  the user's behalf exactly as they would do it, where their Git setup is the
  contract (a signed commit with their hooks, credential helpers and
  prompts, a fetch or push over their SSH or HTTPS configuration). Keep each
  such call behind one boundary and say why it is there.
- Shell scripts and Git hooks have no library; the CLI is simply their
  interface, and this rule does not apply to them.
- A library read that replaces a CLI read gets a parity test against that
  command (`worktree`'s `git_metadata` tests are the model).
