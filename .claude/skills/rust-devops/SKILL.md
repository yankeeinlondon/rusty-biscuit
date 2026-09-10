---
name: rust-devops
description: |
  Use for Rust CI/CD and release engineering: GitHub Actions, affected-package
  scope, pre-push evidence, release-plz, package distribution, compiler caches,
  build storage, or programmatic Git through git2/libgit2 and gitoxide/gix.
  In rusty-biscuit, load this before changing CI scope, local-validation reuse,
  release automation, or repository-wide Rust build caching.
hash: 9fa043b64ab27813-fe3042e2e9114ef7
last_updated: 2026-09-10
---
# Rust DevOps

This skill covers CI orchestration, releases, distribution, build performance,
and Git integration for Rust projects. In rusty-biscuit, repository policy and
measured results override generic ecosystem guidance.

## Choose the owning surface

| Work | Read first |
|---|---|
| CI scope, pre-push behavior, local evidence, GitHub Actions, release-plz | [CI/CD and releases](./ci-cd.md) |
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

For repository scripts and hooks, prefer the installed Git CLI when exact Git
semantics, credentials, signing, hooks, or user configuration are part of the
contract. Embed a library only when the product needs Git operations without a
Git subprocess.

- Choose `git2` when mature libgit2 coverage outweighs the C build and native
  dependency surface.
- Choose `gix` when a pure-Rust implementation, Git-compatible trust model, or
  fine-grained plumbing crates materially improve the product.
- Do not replace a working Git CLI boundary solely to avoid spawning a process.
