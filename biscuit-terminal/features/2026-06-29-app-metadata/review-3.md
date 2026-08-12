---
ready: false
implemented: true
agent: "codex/default"
created: "2026-07-01T22:03:49"
---

# Review: App Metadata, Iteration 3

## Verdict

Not production ready. The iteration fixed the three issues from review 2: Alacritty legacy YAML candidates now carry candidate-level format metadata, the terminal report includes a per-OS candidate table, and Alacritty reports `XDG_CONFIG_HOME` as a config-location override. The remaining blocker is seed-table coverage for the spec's required core setting locators.

## Findings

### High: Several supported apps still omit required core setting locators

The spec defines six v1 core setting locators (`ipc`, `font`, `font_size`, `theme`, `background_color`, `opacity`) and says v1 guarantees those core locators are populated and extracted for supported apps. The implementation still leaves core fields as `None` or uses `SettingLocators::EMPTY` for multiple metadata-covered apps. Examples:

- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:333) gives iTerm2 `ipc: None`, `font_size: None`, and `theme: None`.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:391) gives Apple Terminal `SettingLocators::EMPTY`.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:490) gives Warp `SettingLocators::EMPTY`.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:605) gives Konsole `SettingLocators::EMPTY`.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:638) gives foot `font_size: None` and `theme: None`.
- [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:685) gives Contour `theme: None` and `background_color: None`.

This means `bt about <app>` can render "No settings extracted" or omit core rows entirely for apps that the metadata model otherwise claims to support. That breaks the user-facing Settings requirement: metadata should be reportable even when host extraction is unavailable, with the value column showing a reason such as "no config file", "Lua", or absent.

The fix should either populate the six core locators for every supported metadata entry, or explicitly revise the spec to narrow which apps are considered v1-supported for setting extraction. If some terminals truly combine values, use a locator with a note, as the current iTerm2/foot font examples already do, rather than dropping the core row.

Verification level present: partial Level 1. Current Level 1 CLI tests prove this behavior only for Kitty and Alacritty. There is no seed-table contract test asserting that every metadata-covered app has the required core locator rows. Level 1 is the correct verification level for this requirement; no Level 2 or Level 3 terminal harness is needed.

## Verification

- `cargo nextest run --manifest-path biscuit-terminal/cli/Cargo.toml --test about --color=never`: passed, 18/18 Level 1 tests.
- `cargo nextest run --manifest-path biscuit-terminal/lib/Cargo.toml discovery::app_metadata discovery::config_paths discovery::detection::app --color=never`: passed, 59/59 Level 1 tests.

No Level 2 or Level 3 tests are required for the reviewed config/env metadata behavior. The feature reads files and environment variables, and the spec explicitly calls Level 1 sufficient for `--plain` escape suppression.

## Notes

Review-2 closure looks effective: candidate-specific Alacritty YAML extraction is now tested, the default terminal report shows all OS target candidate templates with the active host marked, and `XDG_CONFIG_HOME` appears in both plain and JSON reports.
