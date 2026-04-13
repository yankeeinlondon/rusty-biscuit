# Review 1

## Findings

1. High: the initialization contract from the spec/design is only enforced from `claudine config`, not from "any non-help command". `claudine/cli/src/main.rs:48-83` dispatches commands directly with no global config check, while `claudine/cli/src/commands/config_tui/mod.rs:28-32` is the only place that triggers `run_initialization()`. That means commands like `sync`, `actions`, `compose`, `sequence`, and the wrappers can still run without first materializing the new config.

2. High: repo-scoped config is still modeled as a full `ClaudineConfig`, so override-only repo config files from the spec/design will not load. `claudine/lib/src/dispatch/loader.rs:733-741` deserializes repo config into `ClaudineConfig`, and `claudine/cli/src/commands/config_tui/tabs/preferences.rs:363-365` creates a full default config just to set `canonical_provider`. Inference: a repo file that only contains `canonical_provider` and/or `actions` will fail deserialization because `preferred_agent` is required. This also means repo config carries user-only fields that the spec explicitly said should not be stored there.

3. High: the new `bash` action is not executing the designed validation path and is still vulnerable to broken command resolution. `claudine/lib/src/actions/bash_executor.rs:20-120` implements blocked-command checks plus JS/TS interpreter resolution, but `claudine/lib/src/dispatch/runner.rs:556-577` never calls it. The runner interpolates `command`, escapes only `params`, then executes `sh -c`. That leaves the `command` side unvalidated and unsafely shell-interpreted, even though the design called out command discovery and strict escaping.

4. Medium: the Actions tab is only a partial editor for the new action model. `claudine/cli/src/commands/config_tui/tabs/actions.rs:11-17` exposes only five action types and omits `call`. `claudine/cli/src/commands/config_tui/tabs/actions.rs:321-338` and `claudine/cli/src/commands/config_tui/tabs/actions.rs:637-703` only let the user edit one text field per action, so there is no UI for `bash.params`, `speak.voice`, `speak.gender`, `message.image`, or `report.handler`. Adding a sound effect also hardcodes the recommended sound instead of presenting the richer selector described in the spec (`claudine/cli/src/commands/config_tui/tabs/actions.rs:540-550`).

5. Medium: the Messenger tab is incomplete. `claudine/cli/src/commands/config_tui/tabs/messenger.rs:138-147` shows an "Add Messenger Provider" picker, but `claudine/cli/src/commands/config_tui/tabs/messenger.rs:243-272` only inserts a stub config with empty destination fields and immediately marks it active. There is no modal to enter channel IDs / recipients / env var names, and no repo-scoped active override, both of which were part of the feature spec.

6. Medium: several TUI interactions drift from the specified UX. The preferred-agent picker uses all providers instead of installed providers (`claudine/cli/src/commands/config_tui/tabs/preferences.rs:145-147`, `claudine/cli/src/commands/config_tui/tabs/preferences.rs:209-215`, `claudine/cli/src/commands/config_tui/mod.rs:297-302`). Default sound selection uses `1`/`2`/`3` instead of `S`/`A`/`E` (`claudine/cli/src/commands/config_tui/tabs/preferences.rs:241-275`). TTS uses `G` to toggle gender and treats `F`/`M` and `Shift+F`/`Shift+M` identically (`claudine/cli/src/commands/config_tui/tabs/tts.rs:315-388`), which is not the interaction model described in the spec. `Shift-Tab` is also broken in Messenger because it moves focus forward instead of backward (`claudine/cli/src/commands/config_tui/tabs/messenger.rs:153-158`).

7. Medium: headless initialization does not follow the design's CI-safe defaults. The design said headless mode should silently write `TTS: off, Logging: on, Protect: default`. Instead `claudine/cli/src/commands/init_wizard.rs:31-36` delegates to `build_default_config()`, and `claudine/cli/src/commands/init_wizard.rs:242-254` enables TTS automatically when `say`/`espeak` is present.

8. Low: `init` was not fully removed from the product workflow. The help text still advertises it (`claudine/cli/src/commands/help.rs:87-96`), user-facing messages still point people at it (`claudine/cli/src/commands/actions.rs:55-61`, `claudine/cli/src/commands/uninstall.rs:45-48`), and internal flows still invoke it for repo canonical-provider setup (`claudine/cli/src/commands/skills.rs:46-56` and the analogous agents/commands flows). That contradicts the refactor goal of consolidating configuration changes behind initialization + `claudine config`.

## Test coverage gaps

- I could not find any `claudine/cli/tests` coverage for `config_tui`, its modal state machine, or its tab-level reducers/rendering. The design explicitly called for TUI rendering/state-transition validation.
- The `bash_executor` helper has unit tests, but there is no end-to-end test proving the runner actually uses that validation path. The current implementation does not.
- There is no integration coverage for the "missing config triggers initialization" behavior on commands other than `config`.
- There is no coverage for repo-scope override-only config files or repo-scope migration/backup behavior in the new loader path.
- `cargo test -p claudine --quiet` passes, which is useful signal here: these gaps are mostly untested behavior/spec mismatches rather than already-failing tests.

## Ergonomics / Performance

- Introduce a dedicated repo-override type instead of serializing a full `ClaudineConfig` for repo scope. That will make the repo file smaller, closer to the spec, and easier to merge safely.
- Replace `sh -c` execution for `bash` actions with direct executable + argv spawning from `ValidatedCommand`. That improves safety and avoids an extra shell hop.
- Make `tts.provider` a typed provider enum instead of a free-form `String`, so bad values fail at parse time instead of degrading later in runtime bridging.
- Add focused reducer/state-machine tests for the TUI. The current modal logic is dense enough that stronger tests will improve developer ergonomics immediately.
