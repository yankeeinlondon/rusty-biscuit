# Interpolation

The Darkmatter DSL provides a mechanism for _interpolation_ of frontmatter into the document's body.

- within the body of the document all "handlebar" placeholders like `{{foo}}` or `{{bar}}` will be considered targets for interpolation.
- when the Markdown pipeline is run with `.transform()`:
    - the document is first transformed with the [Text Replacement](./text-replacement.md) functionality
    - but immediately afterward we replace all `{{variable}}` segments with their frontmatter value throughout the document
    - see the [pipeline](./darkmatter-pipeline.md) for an overview of all items in the pipeline.
- **Fallback Values**
    - if a template placeholder in the document refers to a frontmatter property that has no value then the default value of an empty string will be used.
    - this default is suitable for some situations but not others so you are allowed to express a fallback you'd like to use instead with the following syntax:

      ```md
      Bob's favorite color is {{ color | "unknown" }}.
      ```

- **Boolean Switch**

    - instead of just having a fallback, it is also possible to use a _truthy_ test to provide a value:

      ```md
      Bob's favorite color is {{ color ? "known" : "unknown" }}.
      ```

    - in this example if the frontmatter property `color` is _truthy_ then we'll replace with `known` otherwise `unknown`.

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
      Bob's favorite color is {{ color | blue == "blue" ? blue (how original) : nice choice! }}
      ```

- **Context Properties**

    - there are a certain set of properties that will always be provided to a page
    - these properties are attached to the `ctx` property and include:
        - `ctx.today` - provides an ISO Date string for the date when this was rendered (`YYYY-MM-DD` format)
        - `ctx.now` - provides an ISO Datetime string for the host's locale (`YYYY-MM-DD hh:mm:ss.xxxT...`)
        - `ctx.utc` - provides an ISO Datetime string for the UTC time when this was rendered (`YYYY-MM-DD hh:mm:ss.xxxTZ`)
        - `ctx.yesterday`
        - `ctx.tomorrow`
        - `ctx.dow` - the day of the week (e.g., Monday, Tuesday, etc.)
        - `ctx.dow_abbr` - an abbreviation for the day of the week (e.g., Mon, Tue, etc.)
        - `ctx.year` - tod
        - `ctx.date` - today's date
        - `ctx.month` - the numeric value for today's month
        - `ctx.month_name` - the name for today's month (e.g., January, February, etc.)
        - `ctx.month_name_abbr` - an abbreviated name for today's month (e.g., Jan, Feb, etc.)

- **Environment Variables**

    - environment variables will be passed through as the `env` variable
    - for example:

        ```md
        Bob's favorite color is {{ env.FAVORITE_COLOR | "unknown" }}
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


## Technical Design Options

### Option 1: Source-First Scanner + Interpolation Parser (Recommended for v1)

Implement interpolation as a direct rewrite stage inside the transform pipeline:

- take the current stage input markdown body (the output from the prior transform stage)
- produce the next stage output markdown body after interpolation

- run a single-pass scanner over the source to find `{{ ... }}` spans
- parse each expression with a small dedicated parser (tokenizer + recursive descent / Pratt parser)
- evaluate against an interpolation context built from:
    - document frontmatter
    - `ctx.*` values captured once per transform call
    - `env.*` values (from process env)
- collect `(start, end, replacement)` edits and apply from the end of the string backward

Pros:

- preserves document formatting exactly (no markdown re-serialization churn)
- straightforward to place directly after Text Replacement in the transform pipeline
- very fast and easy to test at the string level

Cons:

- markdown-agnostic unless we add lightweight guards (for example, skipping fenced code or inline code)
- we own the interpolation grammar/parser implementation end-to-end

### Option 2: pulldown-cmark Event-Scoped Interpolation (Blessed Parser Path)

Use `pulldown-cmark` inside the interpolation transform stage to decide where interpolation is allowed, then rewrite only those source spans:

- parse the stage input buffer with `Parser::new_ext(...).into_offset_iter()`
- identify eligible events (typically `Event::Text`; optionally include/exclude code/html events by policy)
- detect `{{ ... }}` inside those event ranges and parse/evaluate expressions
- patch the original source with offset-based replacements (preferred), or re-emit markdown through `pulldown-cmark-to-cmark` (higher churn)

Pros:

- markdown-aware targeting with an already-adopted parser in darkmatter
- easier policy control over "interpolate everywhere" vs "skip code/pre/code span"
- can avoid full re-serialization if we patch source by offsets

Cons:

- placeholders can span awkward boundaries if split by parser events
- offset bookkeeping is more complex than a pure string scanner
- if re-serialized, formatting normalization side effects are likely

### Option 3: markdown-rs MDAST Transform Pass (Best Long-Term Pipeline Model)

Treat interpolation as an AST transform step:

- parse the stage input buffer with `markdown::to_mdast(..., ParseOptions::gfm())`
- walk the tree and apply interpolation to text-bearing nodes
- keep expression parsing/evaluation shared with other options
- serialize to the stage output buffer for downstream transforms

Pros:

- strong foundation for a multi-stage transform pipeline (interpolation, transclusion, consolidation, etc.)
- explicit tree semantics make complex future transforms safer
- leverages darkmatter's existing `markdown-rs` usage

Cons:

- markdown round-trip fidelity depends on serializer quality/availability
- higher complexity and memory cost for first milestone
- likely overkill for initial interpolation-only delivery

### Recommendation

For first implementation, use **Option 1** with a reusable expression parser/evaluator module.
If interpolation scoping becomes important early, evolve to **Option 2 (offset-based pulldown-cmark hybrid)** without throwing away parser/evaluator code.
Reserve full **Option 3** for the stage where multiple structural transforms are implemented and the broader pipeline architecture is ready.
