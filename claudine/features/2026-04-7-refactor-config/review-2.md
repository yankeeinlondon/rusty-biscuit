# Code Review: Refactor Config

The implementation of the `refactor-config` feature makes significant structural changes, successfully introducing `ClaudineConfig`, the new `ratatui`-based TUI, and the canonical dispatch model. However, there are major gaps in functionality, incomplete migrations, and broken components that prevent this feature from being considered fully implemented.

## 1. Broken Functionality

### 1.1 `claudine handle` is Broken
The core dispatch entrypoint, `claudine handle` (`cli/src/commands/handle.rs`), currently triggers `claudine::dispatch::dispatch`. This function relies on the legacy `dispatch_preparsed` path, which calls `loader::load_runtime_config`.
`load_runtime_config` still expects the old `HookerConfig` and cannot parse the new `ClaudineConfig` format. Since `load_claudine_config` actively renames the old format to `.bak` and errors out, running `claudine handle` with a new configuration file will instantly crash or bypass execution entirely.
**Recommendation:** Update `claudine handle` to use the new `dispatch_canonical` path and remove the legacy `RuntimeConfig` and its associated loading logic as intended by the spec.

### 1.2 Event Logging is Non-Functional
The `HookAction::Log` enum variant was removed in favor of a global `logging: bool` toggle in `ClaudineConfig`. However, the new canonical dispatch pipeline (`dispatch_canonical_with_runtime` in `lib/src/dispatch/mod.rs`) does not implement any logging logic. The previous behavior of writing events to daily-rotated JSONL files is entirely lost, effectively breaking the reporting and analytics services.
**Recommendation:** Re-implement the JSONL logging mechanism within `dispatch_canonical_with_runtime`, governed by the `runtime.config().logging` toggle. Ensure this writes to the correct `~/.claudine/logs/` path.

## 2. Incomplete Implementations

### 2.1 TUI Ignores Repo-Scoped Configuration
The `config` TUI (`cli/src/commands/config_tui/mod.rs`) only loads and saves the user-scoped configuration (`~/.claudine/config.json`).
- While the **Preferences** tab contains UI for a "Repo Provider", modifying it erroneously attempts to save the value to the user config due to a shared `canonical_provider` field abstraction.
- The **Actions** tab provides no mechanism to view or define repo-scoped action overrides.
**Recommendation:** Update the TUI's `App` state to distinctly track and edit repo-scoped values when `is_in_repo` is true. Ensure the save operation explicitly writes repo-scoped changes to the local `.claudine/config.json` file without polluting the global user config.

### 2.2 TUI TTS Voices are Stubbed
In `cli/src/commands/config_tui/mod.rs`, the `get_tts_voices()` function returns an empty vector (`vec![]`). Consequently, the voice selection modals in the **TTS** tab are non-functional, preventing users from customizing male or female voices.
**Recommendation:** Implement `get_tts_voices()` to query the available voices from the currently selected `TtsProvider` (via `biscuit-speaks`). Ensure that switching providers automatically resets the selected voices to the new provider's defaults as per the spec.

### 2.3 Initialization Wizard Skips Messenger
The `Initialization Process` outlined in the spec dictates that the user should be introduced to the Messenger feature and offered the opportunity to configure it. `run_interactive_initialization` in `cli/src/commands/init_wizard.rs` completely omits this step.
**Recommendation:** Add a `configure_messenger()` step to the initialization wizard to align with the spec.

### 2.4 TUI Provider Lists are Unfiltered
The **Preferences** tab's user-scoped canonical provider modal presents all supported providers via `get_provider_list()`. The spec requires this specific list to be filtered to only show agents that are actually installed on the host.
**Recommendation:** Update the modal population logic in `handle_user_provider_modal` to filter `get_provider_list()` based on the `is_available()` status from `discover_agents_full()`.

## 3. Ergonomics and Performance

### 3.1 `Bash` Action Parameter Interpolation Flaw
In `lib/src/dispatch/runner.rs`, `execute_bash` performs interpolation and then splits the parameters by whitespace before applying `shell_escape`:
```rust
let parts: Vec<String> = escaped_params
    .split_whitespace()
    .map(bash_executor::shell_escape)
    .collect();
```
While this prevents shell injection, it destructively breaks single arguments that intentionally contain spaces (e.g., an interpolated git commit message).
**Recommendation:** Revise the `Bash` executor to allow robust passing of arguments. Consider changing the config schema to accept an array of strings (`params: Vec<String>`) rather than a single string, allowing discrete interpolation and escaping per argument without relying on flawed whitespace splitting.

### 3.2 TUI Tab Navigation
The **Messenger** tab currently uses custom keybindings (`s`, `a`, `e`) for navigation, defying the spec's requirement to use `Tab` and `Shift-Tab` to move focus between the Select Box and the Add button.
**Recommendation:** Implement standard `Tab`/`Shift-Tab` focus management within the `Messenger` tab to match the specified ergonomic design.

## 4. Test Coverage
- **Migration Logic:** Tested and functional.
- **Serialization/Deserialization:** Adequate coverage for the new JSON5 structures.
- **Reporting Ingestion:** `lib/src/reporting/ingest.rs` contains zero unit tests. This was previously flagged in archived reviews and remains a critical risk given the fragility of incremental SQLite ingestion.
- **Canonical Dispatch:** There is a severe lack of integration testing verifying that `dispatch_canonical` properly executes `Bash`, `Speak`, and `SoundEffect` actions using the flattened `actions` map.

## Summary
The UI and data structure foundations for this feature are solid, but the failure to fully integrate the new configuration into the core dispatch loop makes the system non-functional. Priority must be given to removing the legacy `RuntimeConfig`, wiring `dispatch_canonical` into `claudine handle`, and restoring the JSONL logging service.