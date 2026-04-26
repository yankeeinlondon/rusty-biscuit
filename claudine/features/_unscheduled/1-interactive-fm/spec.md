# Interactive Prompts for Claudine

The `interactive` property in Claudine prompt files has a special meaning:

- `interactive` defines a set of properties in frontmatter where a TUI input components will be used to set defined frontmatter properties
- if a _caller_ of an interactive prompt sets a frontmatter property directly then the interactive element will not be executed (preferring the value set by the caller)

## Inline Example

Imagine the file `review.md`:

```md
---
interactive:
    kind:
        type: ChooseOne
        choices:
            - feature
            - performance
            - comprehensive
            - dry
        label: "Choose the type of review you want to conduct"
        default: feature
    implement_after_review:
        type: Boolean
        default: true
---

::file "reviews/{{kind}}.md"
```

In this example if we simply execute it with `claudine compose review.md` it will bring up two TUI dialogs one after the other:

1. the "kind" of review to perform will use the ChooseOne component from `biscuit-tui`
1. the "implement_after_review" will just be a boolean switch

## External Example

Rather than define the questions we should be able to define the questions externally as YAML files. Before we do that let's bring up two new pieces of vernacular:

- **Question**
    - A question can be defined in YAML as:

        ```yaml
        question:
            type: Text | Boolean | ChooseOne | ChooseMany
            choices: _only when ChooseOne or ChooseMany are the type_
            label: The text displayed in the outline around the input control
            label_position: top-left | top-centered | top-right | bottom-left | bottom-centered | bottom-right
        ```

- **Dialog**
    - A dialog combines multiple questions into one dialog window
    - Tab/Shift-Tab as well as up/down arrows navigate between questions
    - you can define in YAML as:


        ```yaml

        ```
