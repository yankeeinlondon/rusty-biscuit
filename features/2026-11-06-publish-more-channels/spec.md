---
kind: feature
name: publish-more-channels
date: 2026-11-06
status: draft-spec
depends-on:
  - 2026-10-28-publish
next_spec: 2026-11-06-rust-ci-planner
related:
  - 2026-10-03-binary-releases
  - 2026-10-27-release-process
reviewed: false
implemented: false
---

# npm, PyPI (uv), and crates.io

## Why

`2026-10-28-publish` delivers the easiest channels. These three cost more per
channel, but they reach users who will not otherwise find the tools. They carry no
dependency on the evidence work, so they can be picked up any time after
`2026-10-28-publish`.

## Outcome

Approved CLIs are also available through:

| Channel | Example | Main cost |
|---|---|---|
| npm | `npm i -g @yankeeinlondon/darkmatter-cli` | One token; `dist` generates the wrapper |
| PyPI, for `uv` | `uv tool install darkmatter-cli` | Custom job: per-CLI maturin wheels and trusted-publisher setup |
| crates.io | `cargo binstall darkmatter-cli` | Packaging work; publishing is permanent |

`docs/deployment.yaml` decides which package goes where.

## Scope

In order of cost:

1. **npm.**
2. **PyPI.** `dist` lists PyPI only as a future installer, so this is a maturin
   `bindings = "bin"` job. PyPI's default per-file limit is 100 MB.
3. **crates.io.** Starting with `bh` and `bf`, per `2026-10-03-binary-releases`:
   versioned path dependencies, descriptions, moving test infrastructure out of normal
   dependencies, and moving out-of-crate embeds inside. Each crate's first publish is
   a manual, confirmed step.

## Spikes needed

- **ALSA and manylinux.** `claudine` links `libasound`, which may fail manylinux wheel
  rules. Test this before promising `uv` for it.
- **crates.io name availability.** The earlier check is a month old, and names can go
  at any time.

## Open question

Is crates.io worth doing soon? It is the costliest of the three, and `cargo binstall
--git` already serves Rust users.
