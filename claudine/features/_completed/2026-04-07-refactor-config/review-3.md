# Review 3

## Findings

### 1. Logging and Protect post-scan are still incorrectly coupled to `actions[...]`

The spec/design split Logging and Protect into services that should run independently of whether a user configured any explicit actions for a given event. The canonical dispatch path still returns early when an event has no binding in `config.actions`, which means:

- `logging: true` does **not** log events unless that canonical event also has at least one configured action
- `protect_post` never runs for `after_tool` / `turn_complete` / `subagent_stop` unless those events also exist in `config.actions`

Relevant code:

- `claudine/lib/src/dispatch/mod.rs:289-366`
- `claudine/lib/src/dispatch/mod.rs:344-346`
- `claudine/lib/src/dispatch/mod.rs:1446-1472` (the current test suite explicitly locks in `no binding means default outcome`)

This is a functional regression against the refactor goals. A user who enables logging or protect but keeps `actions` sparse will silently lose service coverage.

Recommendation:

- decouple service processing from action binding lookup
- run logging and post-protect evaluation for every dispatched canonical event when the service is enabled
- add dispatch tests for:
  - logging on + no binding still writes a log
  - protect enabled + `AfterTool` with no binding still evaluates `protect_post`

### 2. Repo-scoped messenger override from the spec is not implemented

The Messenger section of the spec explicitly allows a repo-local override for the active messenger choice when `claudine config` is run inside a git repo. The implementation does not support that model:

- `RepoOverrideConfig` only allows `canonical_provider` and `actions`
- repo merge logic only merges those two fields
- the Messenger TUI only reads/writes `app.config.messenger` and never touches `app.repo_config`

Relevant code:

- `claudine/features/2026-04-7-refactor-config/spec.md:291-303`
- `claudine/lib/src/config/claudine_config.rs:274-289`
- `claudine/lib/src/dispatch/loader.rs:857-872`
- `claudine/cli/src/commands/config_tui/tabs/messenger.rs:8-208`
- `claudine/cli/src/commands/config_tui/tabs/messenger.rs:262-504`

This is a spec gap, not just missing polish: there is no schema or runtime path for the repo override.

Recommendation:

- extend `RepoOverrideConfig` with the minimal repo-scoped messenger override needed by the spec
- keep provider configurations user-scoped and let repo scope override only the active config key
- add round-trip tests for user+repo merge behavior and TUI save/load behavior

### 3. Protect rules modal cannot honor “ESC rejects changes”

The spec says the Protect modal should stage edits and only commit them on `Enter`; `Esc` must reject the changes. The implementation mutates `app.config.protect.rules` immediately on every space press, then treats `Enter` and `Esc` identically by just closing the modal.

Relevant code:

- `claudine/cli/src/commands/config_tui/tabs/services.rs:383-408`

This means the modal advertises cancel semantics that it does not actually provide.

Recommendation:

- stage rule toggles in modal-local state and commit them only on `Enter`
- add TUI state tests for:
  - space toggles a staged value
  - `Esc` restores original config
  - `Enter` commits the staged config

### 4. Headless initialization does more than the design allows

The technical design says non-interactive/CI mode should bypass prompts and silently write the default config so the CLI does not hang. The current headless path also registers hooks into detected providers.

Relevant code:

- `claudine/features/2026-04-7-refactor-config/tech-design.md:187-189`
- `claudine/cli/src/commands/init_wizard.rs:31-54`

That is a meaningful behavior difference because it mutates provider state during a code path that was supposed to be the “safe, non-blocking default” path.

Recommendation:

- either stop after writing the config in headless mode, or explicitly update the design/spec to allow silent provider registration in CI/headless sessions
- add a test around the headless path so this behavior is intentional rather than incidental

### 5. `bash` interpolation still does not implement the designed escaping contract

The design calls out shell-escaping interpolated variables as security-critical. The current implementation:

- interpolates `command` and `params` as raw strings
- validates the command
- parses `params` with `shell_words::split`
- never uses the `shell_escape()` helper

Relevant code:

- `claudine/lib/src/dispatch/runner.rs:562-606`
- `claudine/lib/src/actions/bash_executor.rs:149-156`

Using `Command::new(...).args(...)` is better than `sh -c`, but it does **not** satisfy the documented contract. Interpolated values containing whitespace can still change argv shape unless the config author quoted them exactly right, and the helper that was meant to support escaping is currently unused.

Test coverage is also too shallow here:

- `claudine/lib/src/dispatch/runner.rs:1234-1259` only asserts that a bash action returns `Ok(())`
- there are no tests for interpolated variables containing spaces, quotes, or shell metacharacters

Recommendation:

- decide on one contract and implement it explicitly:
  - either escape/interpolate to preserve variable boundaries, or
  - document that `params` is shell-words syntax and placeholders are inserted raw
- add focused tests for argv preservation, JS/TS interpreter fallback, and blocked-command behavior through the full dispatch path

### 6. `default_sounds` looks write-only

The refactor added `default_sounds` to the config model and the TUI/init wizard expose it, but I could not find a runtime consumer inside `claudine/lib/src` or the CLI paths other than validation and config editing.

Relevant code:

- `claudine/lib/src/config/claudine_config.rs:257-259`
- `claudine/cli/src/commands/init_wizard.rs:273-276`
- `claudine/cli/src/commands/config_tui/tabs/preferences.rs`

If these defaults are meant to drive success/attention/error notifications, that wiring still appears to be missing.

Recommendation:

- either wire `default_sounds` into the relevant lifecycle notifications, or remove/defer the setting until there is a concrete runtime consumer
- add an end-to-end test showing where each default sound is used

## Testing Gaps

- There are good serialization tests for `ClaudineConfig` and loader behavior, but there are still no high-value tests for the user-visible TUI contracts in `Preferences`, `Services`, `Messenger`, and `Actions`.
- The canonical dispatch tests currently reinforce the wrong service behavior by asserting that “no binding means default outcome” instead of asserting that services still run.
- `bash` action tests cover validation helpers, not real dispatch-time interpolation and argv behavior.

## Ergonomics / Performance Suggestions

- Model runtime config as `service state + sparse action map` instead of treating “missing binding” as “nothing to do”. That matches the design better and removes several early-return footguns.
- Use staged modal state for all cancelable dialogs, not just Protect. That gives consistent `Enter` commits / `Esc` reverts semantics across the TUI.
- Keep repo overrides minimal. For Messenger, a repo-local active-config key is probably enough and avoids duplicating full provider configs into repo state.
