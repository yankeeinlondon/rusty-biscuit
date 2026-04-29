# Review 4

## Findings

### 1. `claudine config` still does not model repo-scoped action overrides, so the Actions tab can show the wrong effective config

The technical design explicitly gives repo scope an `actions` override layer with per-event replacement semantics, and the loader/runtime implements that merge. The TUI does not. `config_tui::run()` loads the user config only, then loads the repo override separately into `app.repo_config`, but the Actions tab renders and edits only `app.config.actions`. There is no path in the TUI that reads from or writes to `repo_config.actions`.

Relevant code:

- `claudine/features/2026-04-7-refactor-config/tech-design.md:163-170`
- `claudine/lib/src/dispatch/loader.rs:862-885`
- `claudine/cli/src/commands/config_tui/mod.rs:28-61`
- `claudine/cli/src/commands/config_tui/tabs/actions.rs:19-28`
- `claudine/cli/src/commands/config_tui/tabs/actions.rs:513-552`

Impact:

- Running `claudine config` inside a repo can show a stale or incomplete Actions list.
- Editing a user-scoped action for an event that is repo-overridden will not change runtime behavior for that repo.
- The feature now has repo action semantics in the runtime, but not in the user-facing configuration surface that is supposed to manage config ergonomically.

Recommendation:

- Decide whether Actions are user-only or user+repo scoped and make the TUI match that decision.
- If repo-scoped Actions are intended, load an effective merged view for display and add an explicit scope-aware edit path.
- Add tests for merged display behavior and repo replacement semantics in the TUI layer.

### 2. Preferred-agent and user canonical-provider selectors are not actually limited to installed agents

The spec says the preferred agent and user-scoped canonical provider should be chosen from agents installed on the host. The TUI uses `App::available_providers()`, which filters on `AgentInfo::is_available()`. That returns true for either `config_exists` or `on_path`, so a stale config file is enough to make a provider selectable even when the executable is not installed.

Relevant code:

- `claudine/features/2026-04-7-refactor-config/spec.md:177-195`
- `claudine/cli/src/commands/config_tui/app.rs:205-212`
- `claudine/lib/src/config/mod.rs:69-73`
- `claudine/cli/src/commands/config_tui/tabs/preferences.rs:192-223`

Impact:

- Users can set `preferred_agent` or user `canonical_provider` to a provider that is not runnable on the machine.
- This is inconsistent with the initialization flow, which correctly filters preferred-agent choices to `on_path` providers only.

Recommendation:

- Use `on_path` for the installed-provider pickers in Preferences.
- If you still want to surface "configured but not installed" agents, present them separately and non-selectably.
- Add tests that cover stale-config/no-binary cases.

### 3. The interactive TTS installer path is effectively Homebrew-only

The spec says that if the host has no suitable TTS provider, initialization should offer to install a better one. The current implementation hardcodes `brew install espeak-ng` as the installer path while presenting the flow as a general host capability.

Relevant code:

- `claudine/features/2026-04-7-refactor-config/spec.md:139-146`
- `claudine/cli/src/commands/init_wizard.rs:102-120`

Impact:

- The advertised install flow is broken on Linux and Windows.
- It also fails on macOS hosts without Homebrew, despite being framed as a supported remediation path rather than best-effort local convenience.

Recommendation:

- Either make this platform/package-manager aware, or stop pretending it is an installer and switch to guided instructions per host.
- At minimum, gate the Homebrew path behind a `brew` presence check and present a clear fallback message by platform.
- Add tests around the installer decision tree.

### 4. Several TUI keybindings from the spec are still missing or silently remapped

There is still visible drift between the spec and the shipped TUI behavior:

- Messenger spec says `S` opens the select box; the implementation supports `Enter` and `A`, but not `S`.
- Actions spec says `Enter` or `E` opens the event modal; the implementation supports `Enter` only.
- Preferences spec uses `A` for the attention sound, but the implementation remaps attention to `N` without updating the feature docs.

Relevant code:

- `claudine/features/2026-04-7-refactor-config/spec.md:296-303`
- `claudine/features/2026-04-7-refactor-config/spec.md:315-329`
- `claudine/cli/src/commands/config_tui/tabs/messenger.rs:231-308`
- `claudine/cli/src/commands/config_tui/tabs/actions.rs:521-552`
- `claudine/cli/src/commands/config_tui/mod.rs:222-260`
- `claudine/cli/src/commands/config_tui/tabs/preferences.rs:225-274`

Impact:

- The TUI no longer matches the documented contract.
- This makes the spec/design unreliable as user and maintainer documentation.

Recommendation:

- Either implement the documented hotkeys or update the spec/design and help surfaces together.
- Add small state-transition tests for the documented keybindings so this stops drifting.

## Testing Gaps

- `config_tui` still has almost no meaningful test coverage. The only tests under that feature area are reducer helper tests in `claudine/cli/src/commands/config_tui/reducers.rs:92-140`; there are no focused tests for `App`, tab handlers, modal transitions, repo-scope behavior, or hotkey contracts.
- The gaps above need direct tests:
  - repo action override visibility/editing in the TUI
  - installed-provider filtering for Preferences selectors
  - TTS installer branching
  - documented Messenger and Actions hotkeys

## Ergonomics / Performance Suggestions

- Sort messenger configuration names before rendering/selecting them. The current `HashMap` iteration order in the Messenger tab is unstable and will make the UI feel jittery across runs.
- Consider giving the TUI an explicit "effective config view + scoped editor" model. Right now it mixes merged runtime concepts with partially separate user/repo state, which is why Actions fell through the cracks.
- When a repo override becomes empty, consider deleting the repo config file instead of saving an empty object. That keeps the repo state easier to reason about.
