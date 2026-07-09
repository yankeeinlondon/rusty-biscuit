---
ready: false
implemented: true
agent: "codex/default"
created: "2026-07-01T15:22:16"
---

# Review: App Metadata, Iteration 2

## Verdict

Not production ready. The iteration fixed the main review-1 gaps around missing setting locators, env fact candidate vars, nested JSON shape, install-status probing for bundle-only apps, and the explicit app-coverage floor test. The remaining issues are narrower, but one still breaks a supported config path.

## Findings

### High: Alacritty legacy YAML candidates resolve but are parsed as TOML

The spec keeps legacy Alacritty YAML candidates in the ordered config list and requires value extraction for YAML fixtures. The seed table includes `$XDG_CONFIG_HOME/alacritty/alacritty.yml` and `~/.alacritty.yml`, but the app has a single `ConfigFormat::Toml`. When the TOML candidates are absent and a legacy YAML file is the first existing candidate, `bt about alacritty` resolves that YAML path and then loads it through the TOML parser, producing unreadable settings instead of extracted raw values.

Relevant code:

- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:174) declares both `.toml` and `.yml` Alacritty candidates.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:192) assigns one `ConfigFormat::Toml` to all Alacritty candidates.
- [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:455) loads the resolved file using only `meta.config.format`.

The fix should make format selection candidate-aware, split Alacritty TOML/YAML metadata, or drop legacy YAML from the supported candidate list and update the spec. As written, the resolver can find a supported path that the extractor cannot parse correctly.

Verification level present: partial Level 1. There are generic YAML extractor tests, but no Level 1 test for an Alacritty `.yml` candidate winning resolution and producing values. Level 1 is the correct level for this file/env behavior; coverage is missing for the app-specific path.

### Medium: Terminal output omits the required per-OS config candidate matrix

The spec requires the default terminal report's Config files section to show candidates per OS target, mark the active host target, show config-relocating env overrides, and show the resolved active file with provenance. The implementation splits this into separate `OS Target`, `Resolved Config`, `Config Candidates`, and `Config Overrides` sections, and the visible candidate list only comes from `app.config_candidate_paths()` for the current OS target.

Relevant code:

- [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:423) collects only current-target candidate paths for terminal output.
- [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:756) renders those paths as a one-column list, not a per-OS table with an active marker.
- [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:443) does collect all OS candidates, but only the JSON adapter uses that data.

This makes `bt about <app>` less useful for the cross-platform metadata browsing the feature is meant to expose. A Level 1 CLI assertion should check, for an app like Windows Terminal or Alacritty, that the terminal/plain report includes multiple OS target labels and marks the active target.

### Medium: Alacritty's reported config overrides omit `$XDG_CONFIG_HOME`

The spec's seed table lists `$XDG_CONFIG_HOME` as Alacritty's location env override. Resolution behavior still honors it because the candidate templates use `$XDG_CONFIG_HOME`, but `bt about alacritty` reports no config-relocating environment variables because Alacritty metadata has an empty `location_env`.

Relevant code:

- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:202) leaves Alacritty `location_env` empty.
- [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:429) renders config overrides exclusively from `metadata.config.location_env`.

Either add the override to metadata or amend the spec to distinguish XDG fallback tokens from app-specific relocation env vars. Until then the report under-describes how Alacritty config location is determined.

## Verification

- `cargo nextest run --manifest-path biscuit-terminal/cli/Cargo.toml --test about --color=never`: passed, 14/14 Level 1 tests.
- `cargo nextest run --manifest-path biscuit-terminal/lib/Cargo.toml discovery::app_metadata discovery::config_paths discovery::detection::app --color=never`: passed, 58/58 Level 1 tests.

No Level 2 or Level 3 tests are required for the new config/env resolution behavior. The feature reads files and environment variables; the appropriate verification level is Level 1. The remaining gaps need more precise Level 1 contract tests, not real-terminal or OS keyboard tests.

## Notes

Review-1 closure looks mostly effective. Settings are now shown without a config file, env facts retain candidate vars, JSON is nested under `config`/`env`, bundle-only install probes have coverage, and the coverage-floor guard is present.
