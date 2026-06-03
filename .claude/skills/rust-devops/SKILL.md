---
name: rust-devops
description: |
  Use when deploying a Rust project to package managers (cargo, brew, apt,
  nixos, uv, npm) or speeding up Rust builds with compilation caching (kache,
  RUSTC_WRAPPER, local/remote S3 caches, CI build cache). Covers release
  distribution and build-time tooling for Rust projects.
hash: 3c17f9bc012a2b23-c81c1bdddcef5104
last_updated: 2026-06-02
---
# Rust Devops

This **Agent skill** covers common tools and techniques used on Rust projects.

## Deployment

Rust compiles to self-contained native binaries, which is a gift for distribution: there is no runtime to ship and no interpreter to match. The challenge is **reach** — getting a release in front of users with the least friction, on whatever platform and package manager they already trust.

- [Deployment Platforms](./deployment-platforms.md)

## Build Caching

Rust's clean-build times grow with the dependency graph, and `cargo clean`, branch switches, and ephemeral CI runners all throw that work away. A `RUSTC_WRAPPER` cache restores compiled artifacts instead of recompiling them — content-addressed, shared locally via hardlinks and across machines via S3.

- [kache](./kache.md) — drop-in `RUSTC_WRAPPER` artifact cache: when to use it, OS support, configuration (with and without `mise`), local vs. remote caches, and daemon setup.
