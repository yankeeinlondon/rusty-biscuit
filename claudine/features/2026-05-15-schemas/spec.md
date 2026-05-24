# Schema Support in Claudine

- the Darkmatter library now has **schema support**, you can read about it here: [Darkmatter Schemas](@darkmatter/docs/topics/schema-definition.md).
- in this feature the primary objective will be to incorporate schemas into **Claudine**

## Functionality

In claudine we want schemas to:

- schema _definitions_ are no different than in Darkmatter and should be able to get full reuse
    - use the `$schema` property and define or reference a SimpleSchema definition.
- for Claudine a schema definition is highly useful for any `compose` prompt or `sequence` (and to a lesser degree for a `inline-compose` operation)
- when a schema expresses a property as being "required" and claudine is asked use that prompt file:
    - Shell Completions
        - shell completions needs to be tuned so that properties which are defined (required or otherwise) are available to shell completions autocomplete
        - if there's a way to put required properties _ahead_ of non-required ones we should do that
    - Configuration and new TUI Behavior
        - we need a new user-scoped configuration item called `prompt_for_missing` and can be assigned a boolean value
        - this configuration item, if not present in the user's configuration file should default to being `true`
        - when `prompt_for_missing` is set to **true** (or _undefined_)
            - and claudine is pointed to a prompt file that has required properties (which the page doesn't fulfill itself)
            - and the caller does NOT provide the required properties when calling ...
            - we will go into an **interactive** mode where claudine will leverage the biscuit-tui components to ask the user for the required properties (see [Interactive Mode](#interactive-mode))
        - Non-TTY Fallback
            - when `prompt_for_missing` is set to **true** (or _undefined_) but stdin is **not a TTY** (e.g., CI environments, scripts, piped stdin), Interactive Mode is skipped entirely
            - the `MissingProperties` error is emitted instead of attempting TUI
            - this prevents hung processes in non-interactive environments
        - when `prompt_for_missing` is set to **false** 
            - and claudine is pointed to a prompt file that has required properties (which the page doesn't fulfill itself)
            - and the caller does NOT provide the required properties when calling ...
            - we will return a `MissingProperties` error:
                - this error should including the following information:
                    - OSC8 link to the prompt file
                    - the property (or properties) which were missing
                    - if the prompt file has a `description` then we should present this too in `<i><dim>{description}</dim></i>`
                    - make sure that the error is well presented both in terms of being complete but also in presentation


### Interactive Mode

- As soon as we start Interactive Mode we'll start by logging to the screen a contextual message:

    - `- The [{prompt-relative-path}]({prompt-absolute-path}) prompt has the following schema:`
        - for each required property in the schema, present as a nested UnorderedList item (margin-top: 1, margin-left: 4, margin-bottom: 1):
            - property has been fulfilled with correct type => `<green>✓</green> <inverse>{property}</inverse>: {type} <i><dim>- was defined correctly</dim></i>`
            - property has a value but of wrong type => `⛔️ <inverse>{property}</inverse>: {type} <i><dim>- was defined but with the wrong type!</dim></i>`
            - property was missing => `<red>⍉</red> <inverse>{property}</inverse>: {type} <i><dim>- was not defined but is required!</dim></i>`
        - then any optional properties in the schema should be iterated over, using the same nested UnorderedList:
            - `{status}<dim><i> <inverse>{property}</inverse>: {type}</i></dim>`
            - where `{status}` is:
                - `<green>✓</green>` if the properties value was assigned by either the caller or the page itself
                - `<grey>⍉</grey>` if the property was not defined by caller or on the page
                - `<yellow>⚠</yellow>` if the property was of the wrong type
    - if there were any properties that were NOT required but had the wrong type then we'll report:
        - `- **Note:** _properties which are not required and of the wrong type will be dropped and prompt will execute without them_`

- **Wrong-Type Required Properties**
    - Interactive Mode **only** triggers for **missing** required properties
    - if any **required** property has the **wrong type** (regardless of `prompt_for_missing` setting), the process aborts immediately with a hard error
    - the ⛔️ symbol in the status report is purely informational / diagnostic and is emitted before the hard error
    - Interactive Mode is **not** entered when required properties have the wrong type

- **Widget Mapping**
    - during Interactive Mode, the biscuit-tui component used to prompt for each missing required property is determined by its schema type:
        - if the property type is an **enumeration** → use `choose_one`
        - if the property type is an **enumeration array** → use `choose_many`
        - if the property type is a **string or number** → use `text_input`
            - for number types, attempt to convert the user's input from string; if conversion fails, re-attempt the prompt with an error message rather than aborting
        - if the property type is a **boolean** → use `boolean_switch`
