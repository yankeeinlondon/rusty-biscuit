# TUI Inputs

This feature is about creating a set of reusable input components that will be used in this monorepo.

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

## Components

The components for this feature include:


1. TextInput

    This component is just a text input component for single line input.

    - it should allow for adding a text label (above, left, right, below)
    - it should allow for constraining max length

    > Note: this component should likely extend the `tui-input` community component

1. TextAreaInput

    This component is used when _prose_ based content needs to be captured versus just a single line. It should:

    - should allow for configurable sizes (width x height)
    - should allow for an auto scrollbar visual on right interior of the component when there is overflow of content

    > Note: this components should likely extend the `

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
    - there should be able to be a way to an option in the list expose a TextInput component so that a user can write additional context
        - often used for "other" options but can be used as a way to further specify the item selected
    - the _starting_ state (aka, what item is selected) can be chosen based on configuration passed in

1. ChooseMany

    Provides the ability to choose 0:M items from an enumerated list of "options".

    - all comments from SelectOne apply here too


### Containers

1. InputTable

    - a row can be selected and once selected editable columns can be navigated between with left/right arrows
    - the columns all need to be defined up front by a vector of `InputTableColumn` enum:
        - Static Text
        - Boolean Switch
        - Text Input
        - Text Area Input
        - SelectOne
        - SelectMany

1. StatusBar

    Provides 

1. Group

    - provides a border around one or more TUI components
    - has a **normal**, **disabled**, and **error** state which is used to effect UI appearance as well as message to contained UI elements so that they too can respond to this state.

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceAssessment<V = String> {
    pub input: ChoiceInput<V>,
    pub correct_option_ids: Vec<String>,
    pub explanation: Option<String>,
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

### Syntax: `tui <command>`

- Commands are 1:1 mapping to the components we have:
    - InputText
    - InputTextArea
    - BooleanSwitch
    - etc.
- we provide a `--height {#}` CLI switch which when used switches the UI out of full screen mode 
