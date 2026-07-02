---
ready: false
implemented: true
agent: "codex/default"
created: "2026-06-30T20:34:31"
---

# Review: App Metadata

## Verdict

Not production ready. The implementation has a solid library skeleton for metadata, resolution, and extraction, and the new `bt about` tests pass at Level 1. However, multiple user-facing requirements from the spec are either not implemented or only lightly asserted.

## Findings

### High: `bt about` omits required setting locators when no config file is resolved

The spec requires the Settings section to report metadata unconditionally: `Setting | Dot path | Value`, with the value shown as extracted when available or as a short reason such as "no config file" / "Lua". In the implementation, settings are only collected after `get_config_file_resolved()` succeeds; otherwise the report stores an empty settings vector and renders "No settings extracted." See [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:306) and [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:563).

This means `bt about kitty --plain` on a host without `~/.config/kitty/kitty.conf` hides the `font_size`, `allow_remote_control`, `background_opacity`, etc. locators the feature exists to expose. The same applies to JSON output: the `settings` array is empty instead of carrying known locators with `null` / "no config file" values.

Verification level present: Level 1 integration tests only assert that the report contains broad section headings; they do not assert locator presence for an absent config. See [about.rs test](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/tests/about.rs:7). Level 1 is the appropriate level for this file/env behavior, but the assertions are incomplete.

### High: Environment facts drop required candidate-variable metadata

The spec requires an Environment Facts table of `Fact | Candidate vars | Live value`, with live values shown only when the queried app is current. The implementation only stores `name` and `value` in `EnvFactReport`, discarding the candidate variable list (`KITTY_PID`, `KITTY_LISTEN_ON`, etc.), and it returns an empty vector for non-current apps. See [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:263), [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:317), and [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:421).

For non-current apps the UI shows only a sentence saying live facts are hidden, so users cannot learn which variables the app exports. For current apps, users see values but still cannot see which env vars were checked. This is a functional gap in both terminal and JSON modes.

Verification level present: Level 1 asserts the current/non-current branch text only. It does not assert candidate vars in plain or JSON output. Level 1 is sufficient, but coverage is missing.

### High: macOS bundle install detection is explicitly not implemented

The spec says install detection is delegated to `sniff::programs::find_program_with_source`, covering PATH plus macOS `.app` bundle scan and Windows install sources, and reports installed true/false. The seed metadata declares bundle IDs for iTerm2, Apple Terminal, Warp, Kitty, WezTerm, Ghostty, and VS Code, but the CLI only probes `bin_name`; when only `bundle_id` is present it returns `Unknown`. See [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:349), [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:385), [seed.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/discovery/app_metadata/seed.rs:483), and [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:364).

On macOS this makes common GUI-only terminals report `Unknown` instead of installed/not installed, which is directly contrary to the CLI Identity requirement.

Verification level present: no feature test covers install detection. Level 1 is appropriate for the light sniff test the spec requested, but it is missing.

### Medium: `--json` shape does not match the specified contract

The spec defines a nested JSON object with `config.location_env`, `config.candidates`, `config.resolved_file`, `config.resolved_source`, `config.settings`, and `env` fact entries containing `vars` and `value`. The implementation emits a different flat model (`config_candidates`, `env_overrides`, `resolved_config`, `settings`, `env_facts`) and `EnvFactReport` does not include candidate vars. See [about.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/src/commands/about.rs:185).

This is likely to break consumers because `--json` is a public machine-readable surface. Either the implementation should match the spec or the spec must be amended before the feature is accepted.

Verification level present: Level 1 only checks that output is parseable JSON and that a few top-level fields exist. See [about.rs test](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/cli/tests/about.rs:27).

### Medium: Metadata coverage is not protected by the requested regression gate

The spec calls for an explicit app-coverage floor: every app that the legacy resolver answered `Some` for must still answer `Some` from the seed-table-backed wrapper, including `VsCode`, `Konsole`, `Foot`, `Contour`, and `Warp`. The seed table appears to include those apps, but there is no single regression test that encodes the floor as a table of variants and fails on `Some -> None`. Current config path tests are host-conditional spot checks and do not make the floor obvious as a contract.

Verification level present: partial Level 1 spot checks in `config_paths.rs`; missing the explicit Level 1 coverage-floor test.

## Verification

- `cargo nextest run --manifest-path biscuit-terminal/cli/Cargo.toml --test about --color=never`: passed, 8/8 Level 1 tests.
- `just test` from `biscuit-terminal`: failed before completion. The failures were in existing Prose double-underline degradation tests, not in the new app-metadata tests:
  - `components::prose::parity::terminal_double_underline_degrades_to_straight`
  - `components::prose::tests::test_double_underline_degrades_to_straight_when_only_straight_supported`

No Level 2 or Level 3 tests are required for the new config/env resolution behavior per the spec; this feature reads files and environment variables. The `--plain` behavior is appropriately Level 1, but it should include assertions on a previously legacy-styled output path, not only `bt about`.

## Notes

The implementation follows the intended dependency direction: `biscuit-terminal/lib` depends on `biscuit-file` and `plist`, while `sniff` is only in the CLI. Dependency docs were updated. The main remaining work is tightening the CLI report model and adding the missing Level 1 contract tests.
