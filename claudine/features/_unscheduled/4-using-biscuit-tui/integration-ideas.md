# Using biscuit-tui in `claudine config`

## Summary

`claudine config` should adopt `biscuit-tui` incrementally, starting with the modal-level controls that already match `tui-chrome`'s model: single-selection lists, text inputs, boolean switches, and later the protect-rule matrix. A full rewrite of the app shell is not warranted yet. The current config TUI has useful domain state, dirty tracking, repo/user split handling, modal stacking, webhook test workflow, and redaction rules that should remain owned by Claudine.

The main blocker is dependency alignment: `claudine-cli` currently depends on `ratatui = "0.30"` and `crossterm = "0.29"`, while `tui-chrome` depends on `ratatui = "0.29"` and `crossterm = "0.28"`. Direct embedding will not compile cleanly until `tui-chrome` is upgraded or both crates converge on shared workspace dependency versions.

## Current Shape

`claudine config` is a custom ratatui app under `claudine/cli/src/commands/config_tui/`.

- `mod.rs` owns terminal setup, alternate-screen lifecycle, event polling, rendering, dirty-save behavior, and async webhook-test polling.
- `app.rs` owns `App`, tab selection, modal stack, and the large `ModalState` enum.
- `tabs/*.rs` render tab-specific content and handle tab-specific key input.
- `widgets/modal.rs` provides custom centered modal and list-modal rendering.
- `widgets/toggle.rs` provides a custom boolean display widget.

`biscuit-tui` exposes `tui-chrome`, whose relevant model is:

- zero-sized `StatefulWidget` values,
- caller-owned `*State` structs,
- `HandleEvent` returning `EventOutcome`,
- reusable `TextInput`, `TextAreaInput`, `BooleanSwitch`, `ChooseOne`, `ChooseMany`, and `InputTable`,
- shared `KeyBindings`, `ComponentTheme`, `ValidationState`, `Label`, and `FrameChrome`.

This maps well to embedded use inside Claudine's existing app loop. Claudine should not use `run_standalone` inside `claudine config`, because the config TUI already owns the terminal and needs a single event loop for modal stacking, tabs, dirty state, and background webhook tests.

## Highest-Value Integrations

### 1. Replace List Modals with `ChooseOne`

Most current modals are hand-rolled single-selection lists:

- preferred agent selector,
- user canonical provider selector,
- repo canonical provider selector,
- default sound selector,
- TTS provider selector,
- voice selector,
- messenger active route selector,
- messenger provider add selector,
- action event selector,
- action type selector,
- action sound selector,
- action field list.

These are a direct conceptual fit for `ChooseOne`. This would immediately add consistent keyboard behavior, vim navigation, scrolling, hotkey support, and fuzzy filtering. It would also remove repeated `Up`/`Down`/`Enter`/`Esc` handlers across tabs.

Recommended approach:

- Add a reusable Claudine wrapper such as `ConfigChooseOneModal<T>` or a domain-specific `ModalState::ChooseOne`.
- Store the `ChooseOneState<String>` in the modal state, plus a small `on_submit` discriminator that tells Claudine how to apply the selected value.
- Keep domain-specific conversion in Claudine. `tui-chrome` should provide selection mechanics; Claudine should still decide how selected provider names, events, sounds, and config route names mutate `App`.
- Preserve custom modal chrome for now by rendering `ChooseOne` inside the existing `render_modal` content area.

Avoid trying to make every selector generic on day one. A pragmatic first slice is replacing `AgentSelector`, `UserProviderSelector`, `RepoProviderSelector`, and `TtsProvider`, since they are simple list-to-value selections with no nested modal stack behavior.

### 2. Replace Text Entry Modals with `TextInput`

The current TUI manually edits buffers for:

- action text input,
- action field input,
- messenger provider fields,
- webhook URL entry with masking.

`TextInput` would improve cursor movement, backspace/delete behavior, home/end support, max length support, validation rendering, and testability.

Recommended approach:

- Introduce `ConfigTextInputState` that wraps `tui_chrome::TextInputState` and carries Claudine metadata: label, field name, secret/display mode, validation function, and submit target.
- Add a secret rendering option before using it for webhook URLs. Today `TextInput` renders its buffer directly, so using it as-is for webhook URL fields would violate Claudine's redaction invariants.
- If the secret behavior is generally useful, add it to `tui-chrome` as `TextInputState::with_mask(Some(char))` or a sibling `SecretTextInput`. The masking must only affect rendering; `value()` must return the real buffer.
- Preserve messenger webhook validation in Claudine. `tui-chrome` can display validation errors, but Claudine should own URL/env-only rules and `redact_webhook_urls`.

This is especially valuable for action fields, where the current manual input line is very basic and does not support cursor editing.

### 3. Replace Boolean Toggles with `BooleanSwitch`

Current candidates:

- Services tab: logging enabled.
- Services tab: Protect enabled.
- TTS tab: TTS enabled.
- Protect rules modal: each rule enabled/disabled.

The top-level toggles are straightforward. `BooleanSwitch` already supports labels, toggling, left/right, space, and custom themes.

Recommended approach:

- Use `BooleanSwitch` for display first while keeping existing hotkeys (`L`, `P`, `T`) in the tab handlers.
- Later, if focusable row navigation is introduced within tabs, delegate key handling to `BooleanSwitch::handle_event`.
- Define a Claudine `ComponentTheme` that matches current yellow/cyan emphasis and does not surprise users with a different selected background.

The protect rules modal can use `ChooseMany` before `InputTable` if the only operation remains enable/disable. `ChooseMany` maps naturally to staged toggles and already has multi-select semantics.

### 4. Evaluate `InputTable` for Dense Configuration Screens

`InputTable` is the best long-term fit for multi-field, repeated configuration editing. It is not the first thing to adopt because it changes interaction structure more than the simple modal swaps.

Possible uses:

- Protect rules: columns for category, rule, enabled, and maybe custom pattern eligibility.
- Messenger route editor: rows for provider fields with text inputs and secret fields once masking exists.
- Action field editor: rows of action property name and editable value.
- Future event/action matrix: rows are lifecycle events; columns are sound, TTS, message, shell, report, call.

The event/action matrix is compelling but should be a separate feature. It would make the Actions tab much more powerful, but it would also require a strong domain mapping from `HookAction` variants to table cells and back.

## Integration Constraints

### Dependency Versions

Before embedding components, align dependency versions:

- `claudine-cli`: `ratatui 0.30`, `crossterm 0.29`
- `tui-chrome`: `ratatui 0.29`, `crossterm 0.28`

Recommended fix: upgrade `tui-chrome` to `ratatui 0.30` and `crossterm 0.29`, then run `just test` in `biscuit-tui` and focused `claudine-cli` tests. `queue/cli` already uses the same newer versions as `claudine-cli`, so the workspace has precedent.

### Modal State Ownership

`tui-chrome` states are long-lived mutable widget states. Claudine's current `ModalState` variants store domain primitives like `highlighted`, `buffer`, `fields`, and `error`. Integration should not push `tui-chrome` state into global app fields ad hoc. Keep it inside modal variants or a small modal component enum.

A good target shape:

```rust
enum ModalState {
    ChooseOne(ConfigChooseOneModal),
    TextInput(ConfigTextInputModal),
    ChooseMany(ConfigChooseManyModal),
    // Keep bespoke variants for workflows that are not component-shaped yet.
}
```

Each wrapper should expose:

- `render(frame, area, theme)`,
- `handle_key(key) -> EventOutcome`,
- `apply_submit(app)`,
- `cancel(app)`.

This keeps widget event handling reusable without letting generic UI code mutate Claudine config directly.

### Redaction and Secret Handling

Do not use stock `TextInput` for webhook URLs until masking is supported. Current invariants are strong and should remain non-negotiable:

- raw webhook URLs never render in lists,
- input buffers render masked,
- webhook send errors are redacted before display,
- test connection status is modal-local and does not mark config dirty.

This likely justifies adding first-class masked input support to `tui-chrome`.

### Key Binding Compatibility

`tui-chrome` defaults include vim navigation and component-local submit/cancel behavior. Claudine currently uses tab-specific hotkeys heavily. Adoption should preserve the visible Claudine hotkey contract:

- global overview/detail behavior stays in `App`,
- tab hotkeys remain in tab handlers,
- modal-local arrows/Enter/Esc can move to `tui-chrome`,
- special modal hotkeys like sound preview `P` and webhook test `T` stay in Claudine wrapper code.

Where a component consumes a key, the wrapper should return early. Where it ignores a key, Claudine can handle domain-specific shortcuts.

## Suggested Migration Plan

### Phase 0: Dependency Alignment

Upgrade `tui-chrome` to the same `ratatui` and `crossterm` versions used by `claudine-cli`. Run the `biscuit-tui` test suite and fix any API drift. Do this before touching Claudine integration so failures are localized.

### Phase 1: Shared Theme and Modal Adapter

Add a small adapter module in `claudine/cli/src/commands/config_tui/`:

- `component_theme.rs` for Claudine's `ComponentTheme`,
- `component_modal.rs` for rendering `tui-chrome` widgets inside existing `render_modal`,
- helpers for converting `EventOutcome` into modal stack actions.

Keep the existing app shell and hotkey bar.

### Phase 2: Simple `ChooseOne` Selectors

Replace the lowest-risk selectors:

- preferred agent,
- user provider,
- repo provider,
- TTS provider.

These have simple selected-value-to-config mutations and good test coverage potential. Add rendering snapshots and reducer-style tests for submission/cancel behavior.

### Phase 3: Text Input for Action Fields

Replace `TextInput` and `ActionFieldInput` modal handling with `tui-chrome::TextInput`. This improves editing behavior without touching messenger secrets yet.

After this phase, action field editing should support cursor movement, delete, home/end, and validation display through the component state.

### Phase 4: Secret Input and Messenger Flow

Add masked input support to `tui-chrome`, then migrate `MessengerInput`. Preserve Claudine's provider-field sequencing and webhook test behavior in the wrapper.

This phase should include explicit tests that raw webhook URLs do not appear in rendered buffers, route lists, validation messages, or test failure statuses.

### Phase 5: Protect Rules

Replace the protect-rule modal with either:

- `ChooseMany` for a list of enabled rule groups, or
- `InputTable` if category/group/status display is important enough to justify a table.

`ChooseMany` is the lower-risk option. `InputTable` is preferable only if the modal grows beyond toggling builtin groups.

### Phase 6: Larger Redesign Candidates

Consider using `InputTable` for:

- an event/action matrix in the Actions tab,
- a multi-route messenger editor,
- a compact provider preference editor.

These should be planned as UX changes, not component swaps.

## Work That Should Stay in Claudine

`biscuit-tui` should not learn Claudine domain concepts. Keep these in Claudine:

- user vs repo config merging and dirty flags,
- provider discovery and fuzzy provider semantics,
- `HookAction` construction,
- messenger provider config construction,
- webhook URL validation and redaction,
- webhook test connection workflow,
- sound preview playback,
- TTS provider and voice discovery,
- Protect rule catalog semantics.

`biscuit-tui` should provide reusable mechanics: editing, selection, toggling, validation display, table focus, scrolling, and consistent key handling.

## New `biscuit-tui` Capabilities Worth Adding

These additions would make the Claudine integration cleaner and are likely reusable elsewhere:

- masked/secret `TextInput` rendering,
- a reusable modal/chrome wrapper suitable for embedded apps, not only standalone prompts,
- optional footer/hotkey rendering for embedded components,
- `ChooseOneState` construction from `(label, value)` pairs with stable initial selection helpers,
- richer `InputTable` per-cell validation hooks,
- component render tests that demonstrate embedding inside a parent modal area.

## Recommendation

Proceed with adoption, but treat it as a staged component migration. The best first PR is dependency alignment plus `ChooseOne` replacement for simple provider selectors. The riskiest area is messenger input because redaction is security-sensitive and `TextInput` does not yet provide masking. The largest long-term payoff is an `InputTable`-backed Actions view, but that should wait until the smaller modal integrations prove the embedding pattern.
