---
name: rust-devops
description: |
  Use when deploying a Rust project to package managers (cargo, brew, apt,
  nixos, uv, npm), speeding up Rust builds with compilation caching (kache,
  RUSTC_WRAPPER, local/remote S3 caches, CI build cache), or interacting with
  Git from Rust (git2/libgit2 bindings, gitoxide/gix pure-Rust). Covers release
  distribution, build-time tooling, and programmatic Git access for Rust projects.
hash: 81b1157379fa2909-5abdffaf55d47d79
last_updated: 2026-07-18
---
# Rust Devops

This **Agent skill** covers common tools and techniques used on Rust projects.

## Deployment

Rust compiles to self-contained native binaries, which is a gift for distribution: there is no runtime to ship and no interpreter to match. The challenge is **reach** — getting a release in front of users with the least friction, on whatever platform and package manager they already trust.

- [Deployment Platforms](./deployment-platforms.md)

## Build Caching

Rust's clean-build times grow with the dependency graph, and `cargo clean`, branch switches, and ephemeral CI runners all throw that work away. A `RUSTC_WRAPPER` cache restores compiled artifacts instead of recompiling them — content-addressed, shared locally via hardlinks and across machines via S3.

- [kache](./kache.md) — drop-in `RUSTC_WRAPPER` artifact cache: when to use it, OS support, configuration (with and without `mise`), local vs. remote caches, and daemon setup.

## Git Interaction

Reading and writing Git repositories from Rust — status, log, blame, refs, diffing, fetch/push — without shelling out to the `git` binary. Two crates dominate, with a clear trade-off: the mature, C-backed `git2` versus the pure-Rust, increasingly fast `gitoxide`.

- [git2](./git2.md) — safe `libgit2` bindings: the full Git surface (status, log, branch, tag, remote, blame, diff, merge), the three-layer FFI architecture, dated version history, and per-operation gotchas and performance characteristics.
- [gitoxide](./gitoxide.md) — pure-Rust Git via the `gix` facade: the `gix` (library) vs `gitoxide` (CLI) version split, the `Repository` hub, feature-flag and caching model, the same per-operation use cases, hermetic committed-tip merge boundaries, and where it outruns (or still trails) `git2`.