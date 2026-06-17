# Config TUI Refresh: Reusable `biscuit-tui` Components

## Context

Claudine's `config` command is a custom Ratatui application under `claudine/cli/src/commands/config_tui/`. It owns terminal setup, tab state, modal stack state, dirty tracking for user and repo configs, and all key routing. The UI is functional, but it duplicates input widgets and interaction patterns that now exist in the `biscuit-tui` library.

The reusable component set currently available in `biscuit-tui/lib` is:

- `TextInput`: single-line input backed by `tui_input`, with labels, length caps, validation errors, and `Enter` submit / `Esc` cancel.
- `TextAreaInput`: multi-line input backed by `tui_textarea`, with scrollbar support and `Ctrl+S` submit.
- `BooleanSwitch`: reusable boolean toggle with direct left/right force-set behavior.
- `ChooseOne`: single-select list with typed values, disabled options, initial selection, hotkeys, fuzzy filtering, scrolling, and vertical or horizontal orientation.
- `ChooseMany`: multi-select list with typed values, selection limits, fuzzy filtering, scrolling, and `Ctrl+A` / `Ctrl+D`.
- `FrameChrome`: wrapper container for borders, titles, margins, and padding.
- `InputTable`: heterogeneous editable grid with `StaticText`, `TextInput`, `TextAreaInput`, `BooleanSwitch`, `ChooseOne`, and `ChooseMany` cells.

All components follow the same integration model: a zero-sized `StatefulWidget`, caller-owned `*State`, and a `HandleEvent` implementation that returns `EventOutcome::{Consumed, Ignored, Submitted, Cancelled}`. This fits Claudine's existing single event loop and does not require using `run_standalone`.

## Goals

- Replace hand-written modal list, text input, and toggle behavior with `biscuit-tui` components where the behavior is semantically equivalent.
- Keep Claudine's config loading, user/repo merge semantics, dirty tracking, async webhook test workflow, and save behavior in Claudine.
- Preserve existing config TUI workflows and hotkeys unless a `biscuit-tui` behavior is clearly better and documented in the implementation plan.
- Make reusable widget state explicit in `App` instead of deriving all interactive state from ad hoc `highlighted`, `buffer`, and `field_index` fields.
- Improve testability by moving modal-specific value transformation into small builders/reducers and testing the component state transitions separately from rendering.

## Non-Goals

- Do not replace the whole config TUI with standalone `question` invocations.
- Do not move Claudine-specific domain logic into `biscuit-tui`.
- Do not rewrite save, repo override, provider discovery, sound playback, or webhook test behavior as part of the component migration.
- Do not adopt `ChooseOne` blindly for workflows where `Esc` must mean cancel; `ChooseOne`'s current standalone semantics restore the initial selection and submit. Embedded callers can interpret `EventOutcome`, but this needs explicit handling and tests.

## Current Config TUI Surface

The current app has five tabs:

- `Preferences`: favorite agent, user canonical provider, repo canonical provider, and default sounds.
- `Services`: logging toggle, Protect toggle, Protect rule editor.
- `Actions`: event action list, add/edit/delete action flows, text inputs for action fields, and sound pickers.
- `TTS`: TTS enablement, provider selection, gender default, and voice selection.
- `Messenger`: active messenger, repo override, route creation, masked webhook input, validation, and test connection.

The custom reusable pieces inside Claudine today are:

- `widgets::modal::render_modal`: centered bordered popup with title and padding.
- `widgets::modal::render_list_modal`: plain Ratatui `List` with `highlighted` index.
- `widgets::modal::build_modal_hotkey_line`: footer help line.
- `widgets::toggle::Toggle`: display-only `On / Off` row.

These are the immediate duplication points against `biscuit-tui`.

## Recommended Architecture

Introduce a narrow adapter layer in `claudine/cli/src/commands/config_tui/components.rs` or `config_tui/biscuit_tui_adapters.rs`.

The adapter should provide:

- Builders that convert Claudine domain choices into `ChoiceInput<T>` and initial `ChooseOneState<T>` / `ChooseManyState<T>`.
- Builders for `TextInputState`, `TextAreaInputState`, and `BooleanSwitchState` using Claudine's current colors and labels.
- A small modal wrapper that renders `FrameChrome` into a centered `Rect` and then renders the embedded component inside it.
- Helper functions to map `EventOutcome` back to existing Claudine modal lifecycle operations: commit, cancel, pop, or keep editing.

Do not make the main `App` generic over widgets. Instead, store concrete component states in modal variants where needed.

Example modal-state direction:

```rust
ModalState::ProviderPicker {
    target: ProviderPickerTarget,
    state: ChooseOneState<Option<Provider>>,
}

ModalState::TextField {
    target: TextFieldTarget,
    state: TextInputState,
}

ModalState::ProtectRules {
    state: ChooseManyState<RuleGroup>,
}
```

This lets each modal own its render and event state, while reducers remain responsible for applying a submitted value to `app.config` or `app.repo_config`.

## Migration Targets

### 1. List Modals to `ChooseOne`

Replace one-off list modal state for these flows:

- Favorite agent picker.
- User provider picker.
- Repo provider picker.
- TTS provider picker.
- Messenger active config picker.
- Repo messenger override picker.
- Add messenger provider picker.
- Add event picker.
- Action type picker.
- Sound effect picker.
- Voice picker.

Specification:

- Use typed values instead of indexing back into a parallel array.
- Represent clear/none/inherit/disabled as explicit enum variants, not magic index offsets.
- Preserve current selected value with `with_initial_value` or `with_initial_selection`.
- Keep current special hotkeys outside the component when they perform side effects:
  - Sound picker `P` should still play the highlighted sound without submitting.
  - Webhook input `T` remains a modal-local test action, not a choice action.
- Disable fuzzy filtering initially for small fixed lists to preserve current keystroke behavior. Enable it later only for long lists such as voices, events, and sound effects.
- Treat `Esc` as cancel in Claudine modal routing. If the embedded `ChooseOne` returns `Submitted` for Esc because it restored the initial selection, the adapter must detect an Esc key before calling `handle_event` or compare against initial state and cancel explicitly. Add regression tests for every picker where cancel must avoid dirtying config.

Acceptance criteria:

- Existing picker workflows keep the same commit/cancel behavior.
- Pickers no longer manually maintain `highlighted` indexes.
- Picker tests assert selected typed values, not positions.
- Long lists still scroll.

### 2. Protect Rules to `ChooseMany`

Replace `ModalState::ProtectRules { highlighted, staged_rules }` with a `ChooseManyState<RuleGroup>`.

Specification:

- Build one option per builtin `RuleGroup`.
- Initial selected values are the enabled rules in the staged `ProtectRuleToggles`.
- `Space` toggles the active rule.
- `Enter` submits the selected set and converts it back to `ProtectRuleToggles`.
- `Esc` discards staged changes.
- Keep the Services tab summary grid as Claudine-specific display code for now.

Acceptance criteria:

- The existing `protect_rules_modal_commit_updates_config` and `protect_rules_modal_escape_discards_staged_changes` behavior remains intact.
- The conversion from selected `RuleGroup`s to `ProtectRuleToggles` preserves default/explicit semantics where possible. If exact preservation is not possible, document whether the resulting config becomes an explicit custom config.

### 3. Boolean Rows to `BooleanSwitch`

Replace the custom `Toggle` widget for:

- Services: `Logging`.
- Services: `Protect`.
- TTS: `Text to Speech (TTS)`.

Specification:

- Use `BooleanSwitchState` for the active detail row only if the row itself owns focus.
- If the tab keeps the current hotkey-driven model (`L`, `P`, `T`), `BooleanSwitch` can initially be used as a rendering component with state synchronized from config each draw.
- Longer term, introduce per-tab focus rows so `Space`, `Left`, and `Right` can drive the switch through `HandleEvent`.

Acceptance criteria:

- Existing hotkeys still toggle the config.
- The visual switch style is consistent with other `biscuit-tui` controls.
- Dirty tracking remains scoped to the changed config layer.

### 4. Text Inputs to `TextInput`

Replace hand-rendered input buffers in:

- `ModalState::TextInput` for action creation.
- `ModalState::ActionFieldInput`.
- `ModalState::MessengerInput` for non-secret fields.

Specification:

- Store `TextInputState` in the modal.
- Use labels rather than hand-rendered label rows where layout permits.
- On submit, read `state.value()` and call existing reducer logic.
- Use `set_validation_error` for invalid action fields and webhook URL validation.
- Preserve current behavior where empty action creation input does not submit.
- Keep BackTab field navigation in messenger input at the modal adapter layer.

Limitations:

- Current `TextInput` does not support masked rendering. Webhook URL fields should remain on Claudine's existing masked renderer until `biscuit-tui` gets a sensitive input mode.
- Current `TextInput` is single-line only. Message/template fields that may grow should move to `TextAreaInput`, not stay single-line.

Acceptance criteria:

- Cursor movement, delete, home/end, and Unicode width handling come from `tui_input`.
- Existing modal input tests are updated to verify `EventOutcome` and submitted values.
- Webhook URL redaction invariants remain unchanged.

### 5. Longer Text Fields to `TextAreaInput`

Use `TextAreaInput` for fields that naturally contain templates or multi-line content:

- Speak message.
- Message text.
- Bash command or params when editing an existing action.
- Report template.
- Call args if we keep comma-separated editing.
- Future custom Protect patterns.

Specification:

- Use `Ctrl+S` submit and `Esc` cancel in multi-line editors.
- Render with `FrameChrome` title containing the field name.
- Enable scrollbar for fields where content can exceed the viewport.
- Convert the `Vec<String>` value back to a string with `\n` joins.

Acceptance criteria:

- Users can enter newlines where the target config field accepts arbitrary text.
- Single-line fields still use `TextInput`.
- Help/footer text clearly distinguishes `Enter` newline from `Ctrl+S` submit.

### 6. Action Field Editor to `InputTable`

The current `ActionFieldList` plus nested `ActionFieldInput` flow can be converted to a one-row `InputTable` per action, or a small multi-row table where each row is a field.

Recommended first implementation: use one row with one column per field for each action type.

Specification:

- Build `InputTableColumn`s from `HookAction` field metadata.
- Use `TextInput` cells for scalar strings, numbers, mapper strings, and optional paths.
- Use `BooleanSwitch` cell for `Report.include_metadata`.
- Use `ChooseOne` cell for constrained enum fields:
  - Report format: text/json/compact.
  - Speak gender: unset/female/male.
  - Mapper kind if this is split in a later pass.
- Use a separate sound picker for `SoundEffect.effect` until `InputTable` supports cell-specific command hooks like preview/play.
- Submit with `Ctrl+S`, then apply the typed cells back through `apply_action_field` or a new typed reducer.

Acceptance criteria:

- Every field currently exposed by `get_action_fields` remains editable.
- Invalid numeric values are either rejected with validation or retain the previous clamped/no-op behavior with visible feedback.
- Editing an action is one screen rather than a nested list plus text modal for every field.

### 7. Messenger Route Creation

Messenger route creation is currently a sequential wizard with provider selection, configuration name, provider-specific fields, secret masking, inline webhook validation, BackTab, and async test connection.

Recommended first implementation: partial adoption.

Specification:

- Convert provider selection to `ChooseOne`.
- Convert non-secret fields to `TextInput`.
- Keep secret webhook URL fields on custom masked input until `biscuit-tui` supports masked `TextInput`.
- Keep the sequential wizard shape until `InputTable` supports secret cells and field-level side-effect actions.
- Keep async test connection in Claudine; component state should only render status and validation errors.

Potential later implementation:

- Use `InputTable` for route creation once it supports masked text cells and per-cell validation/hints.
- Include a static provider/name row plus editable route-specific rows.

Acceptance criteria:

- Inline webhook URLs are never rendered raw.
- Env-only webhook routes remain valid.
- Test connection never marks config dirty.
- Error messages remain redacted.

## Rendering and Layout

Use `FrameChrome` for component modals, but keep the centered popup geometry in Claudine unless `biscuit-tui` gains a reusable modal container.

Default modal chrome:

- Border: rounded.
- Border color: current Claudine modal yellow, unless the shared `ComponentTheme` is updated project-wide.
- Padding: one cell.
- Title: current modal title.
- Width/height: keep current percentages per modal for the first migration to avoid layout churn.

The top-level tab overview and detail pages should remain custom Claudine rendering. `biscuit-tui` should be used for interactive controls, not for the entire application shell.

## Event Routing

Embedded component routing should follow this shape:

1. If a modal owns a `biscuit-tui` state, call its `handle_event(key)`.
2. If the outcome is `Consumed`, redraw.
3. If `Submitted`, convert and commit through a Claudine reducer, then close or pop the modal.
4. If `Cancelled`, close or pop without dirtying.
5. If `Ignored`, route Claudine-specific hotkeys such as sound preview, BackTab wizard navigation, action add/delete, or webhook test.

For components whose current default key conflicts with Claudine behavior, use `KeyBindings` rather than ad hoc checks where possible.

## Testing Requirements

- Keep existing reducer tests and add component-state builder tests.
- Add regression tests for every cancel path that must not set `dirty` or `repo_dirty`.
- Use Ratatui `TestBackend` snapshots sparingly for modal wrappers and changed tab rendering.
- Add tests for typed conversion:
  - provider picker values to config fields.
  - `RuleGroup` selected values to `ProtectRuleToggles`.
  - action `InputTable` cells back to `HookAction`.
  - messenger fields to `MessengerProviderConfig`.
- Preserve webhook redaction tests and add one render test covering a masked inline URL after migration.

## Phased Plan

### Phase 1: Shared Adapters and Low-Risk Pickers

- Add `biscuit_tui_adapters` module.
- Convert favorite agent, user provider, repo provider, TTS provider, messenger select, repo messenger select, and messenger add provider to `ChooseOne`.
- Add cancel/dirty regression tests.

### Phase 2: Services and Simple Inputs

- Convert Services and TTS toggles to `BooleanSwitch` rendering.
- Convert Protect rule modal to `ChooseMany`.
- Convert simple action and non-secret messenger inputs to `TextInput`.

### Phase 3: Rich Action Editing

- Convert action field editing to `InputTable`.
- Move long freeform text to `TextAreaInput`.
- Keep sound effect preview in a custom picker adapter.

### Phase 4: Messenger Form Refinement

- Revisit messenger route creation after `biscuit-tui` supports masked input and richer form hints.
- Convert the wizard to either an `InputTable` form or a reusable form component, depending on what lands in `biscuit-tui`.

## Open Decisions

- Whether picker fuzzy filtering should be enabled for all lists or only long lists.
- Whether `ChooseOne` should gain a cancel-on-Esc mode in `biscuit-tui`, or Claudine should always intercept Esc before calling `handle_event`.
- Whether action editing should be modeled as one-row-many-columns or many-rows-with-field/value columns. One-row-many-columns maps better to typed cells; many-rows is easier to read on narrow terminals.
- Whether `BooleanSwitch` should become fully focus-driven in tabs now, or initially remain hotkey-driven with reusable rendering only.

## Reusable Components to Consider Adding to `biscuit-tui`

The config TUI exposes several reusable needs that are not fully covered by the current component set:

- `ModalChrome`: centered popup/container with `Clear`, percentage or fixed sizing, title, border, padding, and optional footer. Claudine, `question`, and future TUIs could all reuse this instead of hand-rolling centered modal geometry around `FrameChrome`.
- `ActionFooter` or `HotkeyBar`: reusable footer line for key/action pairs, including wrapping or truncation on narrow terminals.
- Masked `TextInput`: a sensitive mode for passwords, webhook URLs, and tokens that preserves the real buffer while rendering bullets or asterisks. This is the biggest blocker for fully migrating messenger inputs.
- `Form` / `FieldList`: vertical heterogeneous fields with labels, validation, optional secret fields, and field-level hints. This would fit messenger route creation better than `InputTable`.
- `ConfirmDialog`: small yes/no modal with typed result, configurable labels, and default/cancel choice.
- Choice option descriptions: secondary dim text per option would improve provider, voice, messenger, and action pickers without custom list renderers.
- Choice side-effect hooks or preview actions: sound pickers need `P` to preview the highlighted item without submitting.
- Tri-state switch: repo overrides such as inherit/disabled/specific active config are common enough that a reusable inherit/on/off or unset/true/false control would be useful.
- Dynamic `InputTable` rows and per-cell commands: route/action editors would benefit from add/delete row support and cell-specific actions such as preview, validate, or test.
