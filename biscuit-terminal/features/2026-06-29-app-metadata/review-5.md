---
ready: false
implemented: true
agent: codex/default
created: "2026-07-01T22:54:27"
---

# Review 5: App Metadata

Not production ready. The focused app-metadata and `bt about` Level 1 tests pass, and the prior review gaps I checked are now covered: settings are reported without a resolved config, environment fact candidate vars are visible for current and non-current apps, Alacritty YAML candidates use the YAML extractor, the JSON report is nested under `config`/`env`, and `--plain` is asserted on the default metadata/content-analysis render paths.

## Findings

### High: Warp resolves a config directory as a YAML config file

The Warp seed entry says `warp_config_path` points at the `~/.warp` directory and that primary settings are "locator-only", but the metadata sets `config.format = ConfigFormat::Yaml` and the only candidate template is `~/.warp`. The resolver accepts any existing filesystem path with `path.exists()`, so a normal Warp host with a `~/.warp` directory resolves that directory as the active config. `bt about warp` then calls `ConfigDocument::load(&resolved.path, ConfigFormat::Yaml)`, which tries to `read_to_string` the directory and reports every setting as unreadable rather than the intended locator-only / managed-in-app state.

- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:551) documents `~/.warp` as a directory and locator-only.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:556) makes that directory the only candidate.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:564) assigns `ConfigFormat::Yaml`, which is extractable rather than locator-only.
- [resolver.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/resolver.rs:85) accepts the directory because it only checks `exists()`.
- [extract.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/extract.rs:98) then loads the resolved path as YAML.

This violates the spec's "config file" resolution contract and the Warp decision in §8: Warp must keep a coverage-floor candidate, but if it is not a parseable flat config, the user-facing report should degrade honestly to locator-only instead of resolving a directory and producing unreadable extraction errors. A good fix is to model the actual parseable Warp YAML file(s) under `~/.warp` if they are the config being reported; otherwise add a metadata-supported locator-only path for Warp that still preserves the legacy wrapper candidate without feeding the directory into structured extraction. Also add a Level 1 regression test with a temp `HOME/.warp` directory asserting that `bt about warp --json` does not mark all core settings unreadable.

Verification level present: none for this Warp directory case. Level 1 is the correct level because the behavior is deterministic path resolution and file loading; no Level 2 or Level 3 terminal harness is required.

## Verification Run

- `cargo check -p biscuit-terminal-cli --color never`: passed.
- `cargo nextest run -p biscuit-terminal app_metadata --color never`: passed, 32/32 Level 1 tests.
- `cargo nextest run -p biscuit-terminal-cli --test about --color never`: passed, 18/18 Level 1 CLI integration tests.
- `cargo nextest run -p biscuit-terminal-cli about --color never`: passed, 26/26 focused Level 1 tests.

No new Level 2 or Level 3 coverage is required for the reviewed app-metadata behavior: it reads static metadata, process environment, and config files. The `--plain` behavior is appropriately covered at Level 1 because it is governed by `ColorDepth::None` in renderable output rather than real terminal emulator rendering.
