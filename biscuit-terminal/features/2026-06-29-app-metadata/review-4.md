---
ready: false
implemented: true
agent: "codex/default"
created: "2026-07-01T22:14:36"
---

# Review: App Metadata, Iteration 4

## Verdict

Not production ready. The app-metadata implementation now satisfies the previously reported feature gaps I checked: core locators are populated for metadata-covered apps, Alacritty legacy YAML extraction uses the candidate format, `bt about` exposes the spec-shaped JSON/plain report, and the focused app-metadata / about Level 1 tests pass.

The blocker is that the canonical biscuit-terminal Level 1 gate is still red. Even if the failing tests appear unrelated to app metadata, a feature cannot be called production-ready while `just test` for the package area fails.

## Findings

### High: The package's canonical Level 1 test gate fails

`just test` from `biscuit-terminal/` fails in the library suite before the full package-area Level 1 run completes. The failures are:

- [mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/components/prose/mod.rs:625) `components::prose::tests::test_double_underline_degrades_to_straight_when_only_straight_supported`
- [parity.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/terminal/biscuit-terminal/lib/src/components/prose/parity.rs:208) `components::prose::parity::terminal_double_underline_degrades_to_straight`

Both tests expected straight underline SGR (`\x1b[4m...\x1b[0m`) when double underline is unsupported, but the rendered output was plain text (`"important text"` / `"x"`). This is not an app-metadata requirement, but it blocks production readiness because the package's standard Level 1 test gate is failing.

Verification level present: Level 1, and Level 1 is the correct level for this failure because it is an in-process renderer capability/degradation contract. No Level 2 or Level 3 test is needed for the app-metadata feature itself; the spec explicitly says config/env resolution and `--plain` escape suppression are Level 1 concerns.

## Verification

- `just check` from `biscuit-terminal/`: passed.
- `just test` from `biscuit-terminal/`: failed, 915 passed / 2 failed / 86 skipped before fail-fast cancellation.
- `cargo nextest run -p biscuit-terminal app_metadata --color never`: passed, 32/32 Level 1 feature tests.
- `cargo nextest run -p biscuit-terminal-cli --test about --color never`: passed, 18/18 Level 1 CLI integration tests.
- `cargo nextest run -p biscuit-terminal-cli about --color never`: passed, 26/26 focused about tests.

## Notes

No Level 2 or Level 3 mismatch was found for the app-metadata requirements. The user-observable behavior in this feature is file/env resolution, JSON/plain CLI reporting, fuzzy matching, and ANSI-free `--plain` output; the spec classifies those as Level 1.
