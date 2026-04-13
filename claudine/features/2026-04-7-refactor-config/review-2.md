# Review 2

## Findings

1. High: old-format user configs are backed up but do not actually trigger the new initialization flow. `claudine/cli/src/main.rs:31-37` only checks whether `~/.claudine/config.json` exists. If an old-format file exists, `load_claudine_config()` backs it up and returns `ConfigNotFound` (`claudine/lib/src/dispatch/loader.rs:723-725`), but the command then just fails instead of re-running initialization. This misses the migration contract from the spec/design: old config should be treated as “no config” and drop into initialization.

2. High: repo-scoped old-format configs are not migrated or ignored safely. The repo override path in `claudine/lib/src/dispatch/loader.rs:733-741` and `claudine/lib/src/dispatch/loader.rs:770-778` parses repo config directly into `RepoOverrideConfig` with no `migration::is_old_format()` check. A stale repo `.claudine/config.json` will therefore hard-fail loading instead of being backed up and treated as “no repo override”.

3. High: bash shebang support is still broken for common `#!/usr/bin/env ...` scripts. `validate_js_ts()` only keeps the first token after `#!` (`claudine/lib/src/actions/bash_executor.rs:78-104`), and `execute_bash()` later runs only `interpreter + script` (`claudine/lib/src/dispatch/runner.rs:593-600`). For a script with `#!/usr/bin/env bun`, Claudine will execute `/usr/bin/env script.ts` instead of `/usr/bin/env bun script.ts`.

4. Medium: bash `params` handling is still functionally incomplete for anything beyond trivial whitespace-separated argv. `execute_bash()` interpolates the params string and then does `split_whitespace()` (`claudine/lib/src/dispatch/runner.rs:575-583`). That breaks quoted arguments and any interpolated value containing spaces, so config like `params: "--message '{{notification_message}}'"` or a path with spaces will be split incorrectly.

5. Medium: the TTS tab still cannot reliably build and preserve separate male/female voice selections. `apply_voice_selection()` only preserves the opposite gender when the current value is already `VoiceSelection::Gendered`; otherwise it collapses back to `Single` (`claudine/cli/src/commands/config_tui/reducers.rs:23-47`). In practice, picking a female voice and then a male voice overwrites the first choice instead of producing the gendered pair required by the spec.

6. Medium: the Messenger tab still cannot support multiple named configurations for the same provider. The spec explicitly allows multiple configurations, including more than one for Discord/Slack/etc., but the add flow stores each new config under the provider slug itself (`claudine/cli/src/commands/config_tui/tabs/messenger.rs:457-462`). Adding a second Discord config overwrites the first one because there is no separate user-defined name.

7. Medium: the Actions tab remains a partial editor for the refactored action schema. The editable surface is still effectively one text field per action (`claudine/cli/src/commands/config_tui/tabs/actions.rs:334-355`, `claudine/cli/src/commands/config_tui/tabs/actions.rs:675-742`). That leaves no way to manage `bash.params`, `speak.voice`, `speak.gender`, `message.image`, `report.handler`, `call.args`, `call.timeout_ms`, `call.mapper`, or `sound_effect.volume/speed`, even though those are part of the designed action model.

8. Medium: the new initialization flow still mishandles the “no TTS provider present” branch. When no provider is detected, the wizard asks whether to proceed without TTS and then returns `TtsValue::Boolean(!proceed)` (`claudine/cli/src/commands/init_wizard.rs:103-109`). If the user answers “no”, Claudine sets `tts: true` even though no working provider was configured. This also skips the spec’s intended “offer to install a better provider” path.

9. Medium: headless initialization writes config but skips hook registration. Interactive initialization saves config and then calls `register_hooks_all_providers()` (`claudine/cli/src/commands/init_wizard.rs:72-76`), while headless initialization only writes the file and returns (`claudine/cli/src/commands/init_wizard.rs:31-36`). That does not match the feature’s “immediately add a hook into every event for every installed provider” behavior.

## Test Coverage Gaps

- I could not find any `claudine/cli/tests` coverage for the `config_tui` state machine, tab reducers, modal transitions, or rendering. The only direct TUI-adjacent tests I found are small reducer tests in `claudine/cli/src/commands/config_tui/reducers.rs`.
- There is no test covering the “old-format config detected on startup should re-run initialization” flow from the CLI entrypoint.
- There is no test for repo-scoped old-format config backup behavior.
- There is no bash-action coverage for shebangs with interpreter arguments (for example `#!/usr/bin/env bun`) or for quoted/interpolated params containing spaces.
- There is no test covering the TTS “female selection then male selection should produce a `Gendered` voice pair” interaction.
- There is no test covering the Messenger requirement that multiple named configs can exist for the same provider without overwriting each other.
- `cargo test -p claudine --quiet` passed during review. `cargo test -p claudine-cli --quiet` did not finish within the review window, so I did not rely on it as a verification signal.

## Ergonomics / Performance Suggestions

- Make bash actions store structured argv internally, or parse `params` with a shell-words parser instead of `split_whitespace()`. That fixes quoting bugs and removes ambiguity around spaces in interpolated values.
- Preserve the full shebang command line in `ValidatedCommand` instead of only a single interpreter token. That fixes `/usr/bin/env ...` and avoids special-casing common wrappers.
- Add `#[serde(deny_unknown_fields)]` to `RepoOverrideConfig` in `claudine/lib/src/config/claudine_config.rs:280-288`. Right now forbidden repo keys are silently ignored, which will be confusing when users try to author repo overrides by hand.
- Consider making `tts.provider` a typed provider enum rather than a free-form `String`. The runtime already has provider parsing; moving that validation to deserialization would make config errors earlier and clearer.
