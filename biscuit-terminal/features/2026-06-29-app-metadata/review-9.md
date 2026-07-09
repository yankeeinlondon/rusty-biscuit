---
ready: true
agent: codex/default
created: "2026-07-02T06:50:46"
---

# Review 9: App Metadata

Production ready. I did not find remaining implementation gaps against the app-metadata specification.

## Findings

None.

The review-8 blocker is resolved: Warp is no longer modeled with `ConfigFormat::None`. The seed data now keeps Warp in the coverage floor as a directory candidate with `ConfigFormat::Yaml`, and it carries locator-only notes for primary settings managed in-app. The focused tests also now prevent floor-bound apps and their candidate overrides from using `ConfigFormat::None`.

## Verification Level Assessment

- Config path resolution, token expansion, WSL candidate expansion, env override precedence, first-existing candidate selection, and back-compat candidate paths are deterministic filesystem/environment behavior. Level 1 is appropriate and present.
- Value extraction for KittyConf, KeyValue, TOML, YAML, JSON/JSON5, plist, Lua locator-only, Dconf locator-only, unreadable files, and directory candidates is deterministic parser behavior. Level 1 is appropriate and present.
- `bt about` JSON/plain output, fuzzy app matching, invalid app exit code, default-to-current behavior, current-vs-non-current env fact values, and `--plain` escape suppression are CLI process-output contracts. Level 1 is appropriate and present.
- The spec does not require real terminal rendering, terminal input encoding, modifier-key visibility, mouse/paste/IME behavior, or OS keyboard injection for this feature. No Level 2 or Level 3 coverage is required for production readiness.

## Verification Run

- `cargo check -p biscuit-terminal --color never`: passed.
- `cargo nextest run -p biscuit-terminal app_metadata --color never`: passed, 36/36 focused Level 1 tests.
- `cargo nextest run -p biscuit-terminal-cli --test about --color never`: passed, 19/19 Level 1 CLI integration tests.
- `cargo nextest run -p biscuit-terminal-cli test_graph_expression_example_plain_overrides_force_color test_block_plain_overrides_force_color --color never`: passed, 2/2 Level 1 `--plain` regressions.
- `cargo nextest run -p biscuit-terminal config_paths::tests::test_app_coverage_floor_no_regression detection::app::tests::test_wt_session_detects_windows_terminal --color never`: passed, 2/2 Level 1 acceptance gates.
