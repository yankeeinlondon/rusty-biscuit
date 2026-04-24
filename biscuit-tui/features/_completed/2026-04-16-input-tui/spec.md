# TUI Inputs

This feature is about creating a set of reusable input components that will be used in this monorepo.

USE: 'tui' and 'cli' agent skills

## Initialize

This package area, which includes `tui-chrome` (library) and `tui-chrome-cli` (cli) -- are brand new. So the first thing we will need to do is make sure all the plumbing and configuration files are in place.

- Library
    - This library will be a TUI component library for other callers
    - We expect that Claudine or Darkmatter will be the first consumer
- CLI
    - Allows us a lightweight way of instantiating the components found in the library
    - The binary name should be `question`

## Best Practices

- they should all be available to a caller of the library as both
    - a component as part of a larger TUI application 
    - as well as a standalone TUI window
- components should be easily _composable_ together in a TUI application
- the UX is very important and should be a focus for any design, implementation, or testing strategy
- always use the 'tui' skill when working in this package area
- also include the 'cli' skill when working on the CLI

## API Shape

Every input component in this library follows a single, consistent API shape so that callers can embed them in a larger TUI application or run them as a standalone window without ceremony.

- Each component is implemented as a Ratatui `StatefulWidget` with an external `State` struct owned by the caller.
- Event handling is exposed as `handle_event(&mut self, state: &mut State, event: KeyEvent) -> EventOutcome` (or equivalent for components that need a different event type). The `EventOutcome` enum has at least the following variants:
    - `Consumed` — the event was handled; no terminal action required.
    - `Ignored` — the component did not handle the event; the caller may route it elsewhere.
    - `Submitted` — the user committed a value; the caller should read it from `state.value()`.
    - `Cancelled` — the user cancelled (e.g. Esc or Ctrl-C).
- After `EventOutcome::Submitted`, the caller reads the captured value from `state.value()`. The shape of `value()` is component-specific.
- A thin helper `run_standalone(component, initial_state) -> Result<V>` is provided. It owns a temporary terminal, runs an event loop to completion, and returns the submitted value. This is what "standalone TUI window" means technically in this library.
- The CLI binary `question` is built on top of `run_standalone`.

State-ownership principle:

- Components are synchronous and non-async.
- Components do not spawn tasks.
- Components never own the terminal except inside `run_standalone`.

### Validation

Validation in this library follows a two-tier model that is consistent across all inputs.

**Keystroke-time rejection (hard caps).** Certain limits are enforced at the moment of input and simply refuse to accept the keystroke. No error text is surfaced, no event is emitted; the keystroke is silently dropped.

- `TextInput` `max_length`: any keystroke that would cause the value to exceed `max_length` is blocked.
- `ChoiceInput` `max_selections` (applies to `ChooseMany`): toggling an option on is blocked once the cap has been reached.

**Submit-time validation (silent suppression + inline error).** Other constraints are evaluated when the user triggers Submit. If the current value is invalid, `handle_event` returns `EventOutcome::Consumed` (NOT `Submitted`) and the component renders an inline error message below itself describing the violation. The Submit keystroke is not propagated as a submission.

The violations handled at submit time are:

- `ChoiceInput` `required` violated (no selection made on a `required=true` input).
- `ChoiceInput` `min_selections` violated (too few selections on `ChooseMany`).

Callers can retrieve the current error via a new method on the state: `state.validation_error() -> Option<&str>`. Embedding callers that hide the component's inline error rendering are responsible for surfacing the error themselves.

`EventOutcome` remains a 4-variant enum (`Consumed | Ignored | Submitted | Cancelled`). No new variant is introduced for validation failures.

## Components

The components for this feature include:


1. TextInput

    This component is just a text input component for single line input.

    - it should allow for adding a text label (above, left, right, below)
    - it should allow for constraining max length

    > Note: this component wraps / extends the `tui-input` community crate.

1. TextAreaInput

    This component is used when _prose_ based content needs to be captured versus just a single line. It should:

    - should allow for configurable sizes (width x height)
    - should allow for an auto scrollbar visual on right interior of the component when there is overflow of content

    > Note: this component wraps / extends the `tui-textarea` community crate.

1. BooleanSwitch

    A boolean switch which can be switched between two states:

    - states can be `true` / `false` or two other states can be chosen in their place
    - nice UI experience that users will immediately recognize as a toggle switch
    - allows for labels to be added to the switch

1. ChooseOne

    Provides the ability to choose exactly one item from an enumerated list of "options". 

    - allows hotkeys to be associated with particular items in the select list
    - but main navigation will be arrow keys (and vim keys for directional navigation as backup)
        - the actual key bindings should have good defaults but be configurable
    - the UI will provide a clear differentiation between the "selected" state and all other "unselected" ones
        - we should also be able to visually differentiate one of the items as being the "currently selected" choice
    - the _starting_ state (aka, what item is selected) can be chosen based on configuration passed in

1. ChooseMany

    Provides the ability to choose 0:M items from an enumerated list of "options".

    - all comments from `ChooseOne` apply here too


### Containers

1. InputTable

    The columns are defined up front as a `Vec<InputTableColumn>`, where `InputTableColumn` is a data-carrying enum that pairs each variant with its per-cell configuration:

    ```rust
    pub enum InputTableColumn {
        StaticText(String),
        BooleanSwitch(BooleanSwitchConfig),
        TextInput(TextInputConfig),
        TextAreaInput(TextAreaInputConfig),
        ChooseOne(ChoiceInput<String>),
        ChooseMany(ChoiceInput<String>),
    }
    ```

    - `StaticText(String)` — display-only cell; the string is the exact text rendered.
    - `BooleanSwitch(BooleanSwitchConfig)` — payload carries the switch's labels and initial state.
    - `TextInput(TextInputConfig)` — reuses the single-line text input config (label positioning, `max_length`).
    - `TextAreaInput(TextAreaInputConfig)` — reuses the multi-line config (width, height, scrollbar).
    - `ChooseOne(ChoiceInput<String>)` — reuses the existing `ChoiceInput` struct.
    - `ChooseMany(ChoiceInput<String>)` — reuses the existing `ChoiceInput` struct.

    > Note: `BooleanSwitchConfig`, `TextInputConfig`, and `TextAreaInputConfig` are the per-component configuration structs implied by each component's own section — their full shape is not re-spec'd here.

    Column definitions are immutable configuration. Mutable cell state lives in a parallel structure owned by `InputTableState` — conceptually `Vec<Vec<CellState>>`, one inner vec per row and one cell state per column.

    **Row and submit lifecycle:**

    - Rows are fixed and caller-provided. The caller constructs `InputTableState::new(columns, initial_rows)` where `initial_rows: Vec<Row>` is a vector of row values matching the column schema. The user cannot add, delete, or reorder rows interactively in v1.
    - Cell editing happens in place. Left/right arrows navigate between cells within the selected row; up/down arrows move between rows.
    - Submit is a dedicated global key (default **Ctrl-S**, configurable via the same key-binding configuration as other components). `EventOutcome::Submitted` fires only on this explicit key, not by navigating past the last cell.
    - On Submit, `state.value()` returns `&[Row]` — a slice of row structures where each row maps column ids to that cell's captured, typed value.
    - Validation at the table level: if any cell has an active Submit-time validation error (per the Validation model in the API Shape section), Submit is suppressed (`EventOutcome::Consumed`), focus moves to the first offending cell, and the cell's inline error renders as normal. The table itself does not introduce new validation types in v1.

## Select Data Structure

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionMode {
    Single,
    Multiple,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceOption<V = String> {
    pub id: String,
    pub label: String,
    pub value: V,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceInput<V = String> {
    pub id: String,
    pub prompt: String,
    pub help_text: Option<String>,
    pub selection_mode: SelectionMode,
    pub options: Vec<ChoiceOption<V>>,
    pub required: bool,
    pub min_selections: Option<usize>,
    pub max_selections: Option<usize>,
    pub shuffle_options: bool,
}
```

### Naming

The single canonical pair of choice components is **`ChooseOne`** and **`ChooseMany`**. No other names (e.g. `SelectOne`/`SelectMany`) should appear in the public API, documentation, or enum variants.

### Generic value projection

`ChoiceInput<V = String>` remains generic over the value type `V`.

- The CLI always operates with `V = String`.
- Library consumers who want a typed `V` should construct options from strings (via the `choose_*_from_*` helpers below) and then project into their own type using a helper on `ChoiceOption`:

    ```rust
    impl<V> ChoiceOption<V> {
        pub fn map_value<U>(self, f: impl Fn(V) -> U) -> ChoiceOption<U> { /* ... */ }
    }
    ```

It is important that `ChoiceInput` be highly ergonomic, it should allow input from:

- choose_one_from_csv():  A CSV of string values which become the options
- choose_many_from_csv():  A CSV of string values which become the options
- choose_one_from_markdown_list():  A markdown list (ordered or unordered) of string values which become the options
- choose_many_from_markdown_list():  A markdown list (ordered or unordered) of string values which become the options
- choose_one_from_dictionary(): A JSON5 or YAML structure of a dictionary object where the _key_ is the label and the _value_ is the `value` prop on ChoiceOption.


## CLI

In the first version of the CLI it is important to provide enough of a surface area that humans can try out the various forms of input. I'm thinking something like:

### Syntax: `question <command>`

- Commands are 1:1 mapping to the components we have, using the canonical component names as kebab-case subcommands:
    - `text-input`
    - `text-area-input`
    - `boolean-switch`
    - `choose-one`
    - `choose-many`
    - `input-table`
- we provide a `--height {#}` CLI switch which when used switches the UI out of full screen mode

### Output Contract

The `question` binary is built on `run_standalone` and follows this output contract.

**Per-component defaults:**

- Scalar components (`TextInput`, `TextAreaInput`, `BooleanSwitch`, `ChooseOne`): on `EventOutcome::Submitted`, write the captured value as a raw string to stdout, followed by a trailing newline.
- `ChooseMany`: newline-separated list of values on stdout (one value per line), matching the `sort`/`grep` shell convention.
- `InputTable`: JSON on stdout — a JSON array of row objects where each object's keys are the column identifiers and values are the captured cell values.

**Escape hatch flag.** A global `--output {raw|json|null}` flag on `question` overrides the per-component default:

- `raw`: forces the raw/newline-separated defaults above.
- `json`: forces JSON for every component (single JSON string for scalars, JSON array for `ChooseMany`, JSON array of objects for `InputTable`).
- `null`: NUL-byte (`\0`) separated for multi-value outputs (useful when values may contain newlines; pairs with `xargs -0` and `read -d ''`).

**Exit codes:**

- On `EventOutcome::Submitted`: exit code `0`.
- On `EventOutcome::Cancelled` (including Esc and Ctrl-C): exit code `130`. No value is written to stdout.

This makes `question` directly usable in shell pipelines for scalar components (e.g. `NAME=$(question text-input --label "Name")`) and for `ChooseMany` under its default newline emission. Users whose values may contain newlines should use `--output null`.

## Deferred to v2

The following components were in an earlier draft of this spec but are deferred to a follow-up v2 spec:

- **Dynamic row management for `InputTable`** — the ability to add, delete, or reorder rows interactively is deferred; it can land in v2 behind an `allow_add_rows: bool` flag if real demand emerges.
- **StatusBar** — product definition is incomplete; deferred until its role and content model are worked out.
- **Group** — the state-propagation mechanism (normal/disabled/error rippling to contained components) needs its own design pass.
- **Inline TextInput option on `ChooseOne` (and `ChooseMany`)** — the ability for an option in the list to expose an inline `TextInput` so that a user can write additional context (often used for "other" options, or to further specify the selected item).
    - Planned v2 return shape: `ChoiceSelection<V> { value: V, freeform: Option<String> }`. v2 will be additive — a new return type on a new variant or a new component — not a breaking change to v1's `value() -> V`.
    - Planned data-model addition: `ChoiceOption` would gain an optional `freeform: Option<FreeformSpec>` field.
    - Open sub-questions to resolve in the v2 spec:
        - Focus model: how focus moves from the option list to the inline `TextInput`.
        - Tab vs Enter commit semantics.
        - Validation of the freeform field: is it itself `required`? Does it have its own `max_length`?
        - Auto-focus-on-select behavior.
        - Interaction with hotkeys.
