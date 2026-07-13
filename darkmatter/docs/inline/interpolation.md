# Interpolation

The Darkmatter compose pipeline provides interpolation of frontmatter, context, and environment values into a document.

Interpolation happens in two stages during the compose pipeline (see the [pipeline overview](./darkmatter-compose-pipeline.md)):

1. **Frontmatter Interpolation** — resolves `{{ }}` expressions inside frontmatter values using seed (non-templated) frontmatter, the `doc` / `doc.*` namespace, `ctx.*`, and `env.*`. This stage itself runs in **two passes** that bracket frontmatter shell expansion (pass 1 pre-shell, pass 2 post-shell). See [Frontmatter Interpolation](./fm-interpolation.md) for full details.
2. **Body Interpolation** — resolves `{{ }}` expressions in the document body using the effective state (frontmatter + external state + context).

Both stages also expose the [read-side functions](../topics/darkmatter-expressions.md#read-side-functions) (`file_exists`, `frontmatter`, `absolute`, `relative`, …) and the `doc.*` namespace — the same grammar resolves identically across every surface.

Body interpolation runs after text replacement and page blocks have been applied. Within the body, all handlebar placeholders like `{{foo}}` or `{{bar}}` are replaced with their resolved values.
- **Fallback Values**
    - if a template placeholder in the document refers to a frontmatter property that has no value then the default value of an empty string will be used.
    - this default is suitable for some situations but not others so you are allowed to express a fallback you'd like to use instead with the following syntax:

      ```md
      Bob's favorite color is {{ color || "unknown" }}.
      ```

- **Boolean Switch**

    - instead of just having a fallback, it is also possible to use a _truthy_ test to provide a value:

      ```md
      Bob's favorite color is {{ color ? "known" : "unknown" }}.
      ```

    - in this example if the frontmatter property `color` is _truthy_ then we'll replace with `known` otherwise `unknown`.

- **Nested Ternary**

    - ternary expressions can be nested in either branch without extra parentheses:

      ```md
      {{ show_details ? has_name ? name : "unnamed" : "hidden" }}
      ```

    - the expression above is parsed as `show_details ? (has_name ? name : "unnamed") : "hidden"`
    - parentheses may still be used for visual clarity when desired:

      ```md
      {{ show_details ? (has_name ? name : "unnamed") : "hidden" }}
      ```

- **Comparison Switch**

    - rather than relying on the truthiness of a particular property, you may sometimes want to use an explicit comparison operation
    - comparison operators supported are:
        - `==` equality
        - `!=` inequality
        - `>` greater than
        - `>=` greater than or equal
        - `<` less than
    - to make the numeric operators more effective we also provide the following conversion utilities:
        - `length(property)` returns a numeric value representing the string length of the property
            - if the property is an array then the numeric value represents the length of the array
            - if the property is a dictionary then the numeric value represents the number _keys_ in the dictionary
        - `number(property, default = 0)` converts a "number like string" to it's numeric value
        - `round(property, default = 0)`
            - rounds a number up or down to an integer value
            - non-numeric properties are converted to the "default" value
    - any numeric comparison where one or both of the values _can not_ be converted into a numeric quantity resolve to a `false` outcome
    - NOTE: if a string value of "6" is used in a numeric comparison we will automatically convert it to the number 6 for the comparison.

      ```md
      Bob's favorite color is {{ color || blue == "blue" ? blue (how original) : nice choice! }}
      ```

- **Context Variables**

    - there are a certain set of properties that will always be provided to a page as the `ctx` frontmatter value
    - Details on all of the available information provided is found in the document: [Context Variables](../topics/context-variables.md)

- **Environment Variables**

    - environment variables will be passed through as the `env` variable
    - for example:

        ```md
        Bob's favorite color is {{ env.FAVORITE_COLOR || "unknown" }}
        ```

- **Quoting**

    - when we use a want to express a string literal value we MUST quote the string
        - both single and double quotes are fine (just be consistent on start and end)
    - we _need_ quotations around string literal values because otherwise we would not be able to distinguish between a string literal and a _reference_ to a frontmatter variable.
    - the same is **not** true for numeric values because a numeric value will always be a numeric literal as frontmatter properties can not start with a number
    - For example:

      ```md
      - Bob's favorite color is {{ color ? color : unknown }}
      - Bob's favorite color is {{ color ? color : "unknown" }}
      ```

    - in this example both lines will resolve the frontmatter `color` if it's set but if it's not the two lines will vary:
        - the first line will resolve to the frontmatter property `unknown` which if not set will default to an empty string
        - the second line will resolve to the string literal "unknown"


## Interpolation Literals

When you want to *show* the `{{ ... }}` syntax rather than evaluate it, wrap the span in an extra pair of braces: `{{{ ... }}}`. This interpolation literal composes to the literal text `{{ ... }}` and the content is never evaluated.

```md
Use `{{{ name }}}` to reference the `name` frontmatter value.
```

After compose, the body above becomes:

```md
Use `{{ name }}` to reference the `name` frontmatter value.
```

### Recognition rules

- A literal opens only at **exactly three consecutive `{` characters**. Four or more braces in a row (e.g. `{{{{`) fall through to the existing `{{` scanner behavior.
- A literal closes at the **first subsequent `}}}`**. Because the first `}}}` terminates the literal, content cannot itself contain `}}}`; use a fenced code block to document such a span.
- An **unclosed** `{{{` with no later `}}}` is not a literal. The scanner falls back to the legacy `{{` behavior at the same position, preserving the current malformed-expression diagnostic.
- Literals inside **fenced and indented code blocks** are treated as plain text and are not converted. Inline code spans are scanned, so `{{{ ... }}}` is the correct way to write literal interpolation syntax inside backticks.
- Empty content is allowed: `{{{}}}` becomes `{{}}` and `{{{ }}}` becomes `{{ }}`.

### Examples

```md
Tight form: {{{x}}} becomes {{x}}.
Empty form: {{{}}} becomes {{}}.
Adjacent: {{ a }}{{{ b }}} evaluates a and emits {{ b }} literally.
Nested expression: {{{ {{ x }} }}} becomes {{ {{ x }} }} with x unevaluated.
```

### Frontmatter literals

A literal in a frontmatter value is always text. `key: "{{{ x }}}"` resolves to the string `{{ x }}` and survives both frontmatter interpolation passes, including the pass that brackets frontmatter shell expansion.


## Implementation

The current implementation uses a source-first scanner approach (single-pass rewrite):

- A scanner finds `{{{ ... }}}` spans in the document body, and also recognizes `{{{ ... }}}` interpolation literals. Inline code spans (single backticks) are interpolated by default, since the templating pattern `` `var_{{ phase }}` `` is a common use case, and literals inside inline code convert to literal `{{{ ... }}}` text. Fenced and indented code blocks are skipped.
- Each expression is parsed with a dedicated tokenizer and evaluator
- The interpolation context is built from the effective state (frontmatter + external state), `ctx.*` runtime values, and `env.*` environment variables
- Replacements are applied from the end of the string backward to preserve offsets
- Literal conversion (`{{{ ... }}}` → `{{{ ... }}}`) happens after the final scan pass over a surface, so a literal introduced by a replacement value is also converted exactly once

See the source modules:

- `darkmatter/lib/src/markdown/compose/interpolation/` — lexer, evaluator, rewriter
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs` — frontmatter-specific interpolation engine
