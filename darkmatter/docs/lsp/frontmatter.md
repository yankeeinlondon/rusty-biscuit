---
related:
    - "@darkmatter/docs/topics/schema-definitions.md"
    - "@darkmatter/docs/rendering/hr.md"
---

The **Frontmatter** for Darkmatter documents is an authoritative configuration layer. Authors interact with these top-level key/values:

| Key                       | Purpose                                                                   | Notable behaviour                                                                                                |
|---------------------------|---------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------|
| `replace`                 | Literal-string replacements applied to the body.                          | Map of scalar → scalar. Longest-key-wins, then lexicographic tie-break. Single-pass.                             |
| `prologue` / `epilogue`   | Implicit transclusions prepended / appended to the document body.         | Path string; resolved via the same machinery as `::file` transclusion                                                        |
| `interpolate_code_blocks` | Boolean opt-in to interpolate fenced/indented code blocks.                | Defaults to `false`                                                                                             |
| `hr`                      | Horizontal-rule defaults.                                                 | Object with `style`, `alignment`, `weight`, `width`, `color`                                                    |
| `$schema`                 | Optional pointer to a _schema definition_ which further constrains the Frontmatter  | LSP enforcement target                                                                                          |
| Arbitrary author keys     | Visible to interpolation as `{{ key }}` and nested `{{ a.b.c }}`.         | Schemas (when present) constrain shape.                                                                          |

The _values_ of the YAML object can be a mixture of:

- **Static Content** (_string, numeric, boolean_)
- **Dynamic Content** _via_:
    - **Interpolation**
        - the use of "handlebars" based interpolation where `{{ variable or expression }}` is interpolated into a finalized value lazily at execution time
        - an LSP we would **not** want nor need to know the _finalized_ value that this interpolation will take but rather that the syntax of the interpolation is valid syntactically and structurally
            - we make the distinction between "syntax" and "structure" because:
                - **syntax** will require a use of some static rules for evaluation where broken syntax should be highlighted boldly in the DSL's implementation as an error
                - **structure** most commonly refers to a file reference and validation requires an external check but also the file reference doesn't strictly have to be there any time other then when it is "executed/composed"
        - the simplest (_and most common_) interpolation is to point at a Frontmatter property. For example:
            - `{{ title }}` would have the template reference replaced with the frontmatter's `title` in Frontmatter
            - this would be an empty string if `title` is not set
        - users are also allowed to perform logical fallback like:
            - `{{ title || env.TITLE }}`
            - 
    - **Shell Expansion**
        - values which fit the format of `"$(...)"`
        - will be treated as a shell command and if approved during the [pre-flight checks](@darkmatter/docs/topics/pre-flight-checks.md) it will be executed and both STDOUT and STDERR will be used to replace the template tag


> **Note:** the topic of **schema definitions** is it's own topic in and of itself and you can find out more about this in the [Schema Definitions](@darkmatter/docs/topics/schema-definition.md) document.
> 
> **Note:** interpolation can point to frontmatter properties defined on the page but it also _always_ gets two lookup variables defined for it:
> 
> - `env` - the **env** property is a dictionary of environment variables
> - `ctx` - the **ctx** (_short for "context"_) property is a dictionary of contextual information about the environment the page is being executed in. This is defined in detail in [Context Variables](@darkmatter/docs/topics/context-variables.md) document.
