---
phases: 6
created: 2026-05-03
start_phase: 1
---

# Execution Plan: Centralized Terminal App Profiles

## Phase 1: Establish Profile Types

Goal: Add the new static profile vocabulary without changing runtime behavior.

1. Create `biscuit-terminal/lib/src/discovery/profile.rs`.
2. Define `TerminalAppProfile` with fields for rendering capabilities, image policy, IPC/scriptability, instance model, vendor metadata, platforms, and config paths.
3. Define supporting enums and structs:
   - `Platform`
   - `GraphicsCapability`
   - `TriState`
   - `ImageRounding`
   - `ScrollPolicy`
   - `IpcCapability`
   - `InstanceModel`
   - `ConfigPathSet`
4. Add helper methods where they remove call-site ambiguity, especially:
   - `GraphicsCapability::primary_image_support() -> ImageSupport`
   - `GraphicsCapability::supports(ImageSupport) -> bool`
   - `ConfigPathSet::for_current_platform()` or equivalent existing-platform helpers
   - strict yes/no helpers for `TriState` if existing callers need booleans
5. Wire the new module through `discovery/mod.rs`.
6. Add rustdoc summaries for public types using the repo rustdoc convention.

Parallelizable: No. This phase establishes shared types that all later work depends on.

Validation checkpoint:

1. Run `cargo check -p biscuit-terminal`.
2. Confirm there are no behavior changes outside new, unused public types.

## Phase 2: Build Profile Registry

Goal: Create the single source of truth for known terminal app static facts.

1. Create `biscuit-terminal/lib/src/discovery/profile_registry.rs`.
2. Implement one profile constant per known app plus an unknown/default profile:
   - WezTerm
   - Kitty
   - Alacritty
   - Apple Terminal
   - iTerm2
   - Ghostty
   - Konsole
   - GNOME Terminal
   - Foot
   - Contour
   - Warp
   - Wast
   - VS Code
   - Unknown/Other
3. Implement `TerminalApp::profile() -> TerminalAppProfile`.
4. Keep `TerminalApp::Other(String)` supported by returning an owned synthesized profile or a profile whose `app` value preserves the current `Other` behavior.
5. Add a registry-completeness test that covers every concrete `TerminalApp` variant and asserts the returned profile is internally consistent.
6. Add internal consistency tests for profile fields that can be validated without changing existing detection paths.

Parallelizable: Partially. After the profile type is compiled, one developer can fill profile constants while another writes registry tests, as long as they coordinate field names.

Validation checkpoint:

1. Run `cargo test -p biscuit-terminal profile`.
2. Run `cargo check -p biscuit-terminal`.
3. Confirm the registry tests fail if a known `TerminalApp` variant is omitted.

## Phase 3: Add Sixel Capability Support

Goal: Extend image capability modeling before migrating static image logic.

1. Add `ImageSupport::Sixel` to the existing enum in `discovery/detection.rs`.
2. Update every workspace `match` over `ImageSupport` to handle `Sixel`.
3. Set Foot and Contour profile graphics to `GraphicsCapability::Sixel`.
4. Add detection data for known Sixel terminals if the current detection path uses terminal-name lists.
5. Ensure code that renders images treats Sixel as unsupported for byte emission until a future Sixel renderer exists.
6. Add focused tests for Sixel classification and non-rendering fallback behavior.

Parallelizable: Yes. One track can update compile errors from the enum addition while another adds the profile/test changes.

Validation checkpoint:

1. Run `cargo check --workspace`.
2. Run `cargo test -p biscuit-terminal image_support`.
3. Confirm no code path attempts to emit Sixel image bytes in this feature.

## Phase 4: Migrate Static Fact Call Sites

Goal: Replace scattered per-app static knowledge with profile lookups while preserving behavior.

1. Migrate `discovery/clipboard.rs`.
   - Replace per-app OSC52 support matches with `app.profile().supports_osc52`.
   - Add or update parity tests for every app previously named in the match.
2. Migrate `discovery/mode_2027.rs`.
   - Replace per-app mode 2027 matches with `app.profile().supports_mode_2027`.
   - Add parity tests for every previously handled app.
3. Migrate `discovery/osc_queries.rs`.
   - Add the missing profile field for OSC color query support if it is not already present from Phase 1.
   - Replace background-color and cursor-color support matches with profile-driven checks.
   - Preserve runtime query/probe behavior.
4. Migrate `discovery/config_paths.rs`.
   - Replace per-app config path matches with `app.profile().config_paths`.
   - Preserve platform-specific path expansion and existing public return types.
5. Migrate image protocol detection in `discovery/detection.rs`.
   - Replace `KITTY_TERMINALS` and the iTerm2 special case with registry iteration or profile helpers.
   - Keep terminal identification logic unchanged.
6. Migrate `components/terminal_image.rs`.
   - Replace rounding conditionals with `app.profile().image_rounding`.
   - Replace Ghostty/Warp scroll compensation branches with `app.profile().scroll_compensation`.
   - Preserve cursor movement and fallback rendering behavior.
7. Remove legacy constants/helpers only after parity tests cover their old answers or after they are made test-only shims.

Parallelizable: Yes. The file migrations are mostly independent after Phase 2 and Phase 3. Recommended split:

1. Clipboard and mode 2027 together.
2. OSC query migration.
3. Config path migration.
4. Detection image support migration.
5. Terminal image rounding and scroll migration.

Validation checkpoint:

1. After each file migration, run the nearest package tests for that module.
2. After all migrations, run `cargo test -p biscuit-terminal`.
3. Run `rg "TerminalApp::(Wezterm|Kitty|Alacritty|AppleTerminal|ITerm2|Ghostty|Konsole|GnomeTerminal|Foot|Contour|Warp|Wast|VsCode)" biscuit-terminal/lib/src/discovery biscuit-terminal/lib/src/components/terminal_image.rs` and inspect remaining matches.
4. Confirm remaining app-name matches are runtime detection branches, enum lookup code, or tests, not duplicated static fact tables.

## Phase 5: Expose Public API And CLI Output

Goal: Make profiles available to consumers without changing existing `Terminal` runtime fields.

1. Add `Terminal::profile(&self) -> TerminalAppProfile`, delegating to `self.app.profile()`.
2. Re-export profile types from the crate prelude:
   - `TerminalAppProfile`
   - `GraphicsCapability`
   - `IpcCapability`
   - `InstanceModel`
   - `ImageRounding`
   - `ScrollPolicy`
   - `Platform`
   - `TriState`
   - `ConfigPathSet`
3. Confirm `Terminal::new()` still populates existing fields from the same runtime detection flow and does not require callers to touch profiles.
4. Update the `bt` no-argument output with a Profile section that shows:
   - display name
   - vendor URL
   - graphics capability
   - IPC capability
   - instance model
5. Update `bt --json` to include a top-level `"profile"` object with stable, serde-friendly field names.
6. Add CLI output tests or snapshot tests if the existing CLI test style supports them.

Parallelizable: Partially. Library API wiring and CLI presentation can proceed in parallel after the profile registry is stable.

Validation checkpoint:

1. Run `cargo test -p biscuit-terminal`.
2. Run the relevant CLI package tests for `bt`.
3. Manually run `cargo run -p biscuit-terminal-cli -- --json` or the existing `bt --json` invocation and verify the `"profile"` object is present.
4. Manually run the no-argument `bt` invocation and verify the Profile section is readable and sourced from the active app profile.

## Phase 6: Documentation And Final Hardening

Goal: Close documentation, drift, and workspace-level compatibility.

1. Update `biscuit-terminal/README.md` if public API or CLI output examples mention terminal capabilities.
2. Update `.claude/skills/biscuit-terminal/SKILL.md` with a "Terminal app profiles" section that points to `TerminalApp::profile()` for static facts.
3. Update `docs/dependencies.md` only if the implementation added or removed crates.
4. Add final migration parity tests covering all old static match answers listed in the spec:
   - image protocol
   - OSC52 clipboard support
   - mode 2027 support
   - OSC color query support
   - config path support
   - image rounding
   - scroll compensation
5. Run formatting with the repo-standard command for the area.
6. Run final validation commands:
   - `cargo fmt --check`
   - `cargo test -p biscuit-terminal`
   - `cargo check --workspace`
   - root or area `just test` if it covers the changed packages without excessive unrelated scope
7. Review public API docs for rustdoc section order and absence of `# H1` headings inside doc comments.
8. Inspect the final diff for unrelated changes and remove any accidental churn.

Parallelizable: Yes. Documentation updates, parity test expansion, and workspace compile validation can be split once implementation is complete.

Validation checkpoint:

1. All final validation commands pass or have documented, unrelated failures.
2. Acceptance criteria from `biscuit-terminal/features/2026-05-03-app-profiles/spec.md` are checked off.
3. The final diff contains the new profile source of truth and no remaining duplicated static per-app fact tables in migrated files.

## Dependency Order Summary

1. Phase 1 must finish before any profile registry or migration work.
2. Phase 2 must finish before profile-driven call-site migration.
3. Phase 3 should finish before detection and terminal image migration because `GraphicsCapability` and `ImageSupport` need their final shape.
4. Phase 4 must finish before public CLI/profile output is considered stable.
5. Phase 5 must finish before documentation examples are finalized.
6. Phase 6 closes the feature with docs, workspace validation, and acceptance review.

## Completion Checklist

1. `TerminalAppProfile` and support enums exist in `discovery/profile.rs`.
2. `profile_registry.rs` contains one authoritative profile per known terminal plus unknown/other handling.
3. `TerminalApp::profile()` and `Terminal::profile()` are implemented and tested.
4. `ImageSupport::Sixel` exists and all matches compile.
5. Static per-app fact matches are removed from the migrated modules.
6. Behavioral parity tests pass for every old static fact.
7. `bt` text and JSON output expose profile data.
8. Prelude exports include the new public profile types.
9. README, skill docs, and dependency docs are updated where applicable.
10. Final cargo and just validation commands pass or have documented unrelated failures.
