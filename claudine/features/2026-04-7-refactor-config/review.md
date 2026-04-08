# Code Review: Refactor Config and Actions

## 1. Gaps in Functionality (Designed but Not Implemented)

*   **Global Initialization Check:** The specification states that running *any* `claudine` subcommand (except `--help`) without a config file must trigger the Initialization Process. Currently, only `claudine config` triggers the `init_wizard` (in `claudine/cli/src/commands/config_tui/mod.rs`). Commands like `claudine hooks` or `claudine compose` will fail or behave incorrectly if the config is missing.
*   **TUI Actions Tab is a Stub:** The Actions tab (`tabs/actions.rs`) only allows a user to add a hardcoded "doorbell" `SoundEffect` to an event. The spec requires a comprehensive interface to add, edit, and delete multiple actions of various types (Bash, Speak, Message, etc.) per event.
*   **TUI Messenger Tab Configuration:** The Messenger tab (`tabs/messenger.rs`) allows selecting a provider to add, but it only inserts stub configurations with empty fields and default ENV var names. There is no UI implemented to actually input the required fields (like `channel_id`, `recipient`, etc.).
*   **TUI Preferences Keybindings & Features:** The Sound Effects modal in the Preferences tab is missing the `P` keybinding to preview sounds. Additionally, the spec mandates `S`, `A`, and `E` keybindings for Success, Attention, and Error sound modals, but the implementation uses `1`, `2`, and `3`.

## 2. Broken or Incomplete Implementations

*   **CRITICAL: Bash Action Shell Injection & Ignored Validation:** In `claudine/lib/src/dispatch/runner.rs`, `execute_bash` directly interpolates parameters into `sh -c "{command} {params}"` without applying `bash_executor::shell_escape`. Furthermore, it completely bypasses the `bash_executor::validate_command` function outlined in `tech-design.md`. This results in severe issues:
    *   **Security Vulnerability:** Malicious or malformed template variables can cause shell injection.
    *   **Broken Blocklist:** Blocked commands (like `rm`) defined in `bash_executor.rs` are not actually blocked at runtime.
    *   **Broken Script Execution:** The JS/TS script execution fallback logic (shebang -> bun -> node) is never invoked.
*   **Incorrect Default Logging Config:** The spec explicitly states the logging service is turned on by default. While `init_wizard` sets it to `true` during onboarding, the `Default` trait implementation for `ClaudineConfig` sets `logging: false`. This causes inconsistencies if the config is instantiated programmatically or falls back to defaults.

## 3. Test Coverage Gaps

*   **Bash Execution Integration:** While `bash_executor::validate_command` has unit tests, there are no integration tests verifying that `Bash` actions in the dispatcher actually use this logic to respect the blocklist or execute JS/TS files correctly.
*   **Shell Escaping Validation:** There are no tests verifying that interpolated event variables are safely escaped before being executed in the shell environment.
*   **TUI State Mutations:** The ratatui components lack tests verifying that state mutations (like adding an action or modifying a messenger config) correctly apply to the underlying `ClaudineConfig` and serialize properly.

## 4. Ergonomic and Performance Improvements

*   **Centralize Initialization Logic:** Instead of calling `init_wizard::run_initialization()` from inside `config_tui::run`, move the config existence check to `claudine/cli/src/main.rs` (immediately after argument parsing and before dispatching to subcommands). This guarantees the initialization process runs globally for all commands without code duplication, ensuring a consistent user experience.
*   **Consolidate `shell_escape` (DRY):** There are multiple identical `shell_escape` implementations duplicated across the monorepo (e.g., in `claudine/cli/src/output.rs`, `model-citizen/cli/src/commands/run.rs`, and `claudine/lib/src/actions/bash_executor.rs`). Extract this into a shared utility location (such as `biscuit-terminal/cli/src/types.rs` where `unescape_shell_escapes` lives) to adhere to DRY principles.
*   **TUI Action Summary Performance:** In `tabs/actions.rs`, the `summarize_actions` function iterates over actions manually to build a deduplicated string of action types. This could be written more cleanly and performantly using iterator combinators like `.map(|a| a.type_pascal_case()).unique().join(", ")` (if `itertools` is available) or a `HashSet`.