# Interpolation

The Darkmatter compose pipeline provides interpolation of frontmatter, context, and environment values into a document.

Interpolation happens in two stages during the compose pipeline (see the [pipeline overview](../darkmatter-compose-pipeline.md)):

1. **Frontmatter Interpolation** — resolves `{{ }}` expressions inside frontmatter values using seed (non-templated) frontmatter, the `doc` / `doc.*` namespace, `ctx.*`, and `env.*`. This stage itself runs in **two passes** that bracket frontmatter shell expansion (pass 1 pre-shell, pass 2 post-shell). See [Frontmatter Interpolation](./fm-interpolation.md) for full details.
2. **Body Interpolation** — resolves `{{ }}` expressions in the document body using the effective state (frontmatter + external state + context).

Both stages also expose the [read-side functions](../topics/darkmatter-expressions.md#read-side-functions) (`file_exists`, `frontmatter`, `absolute`, `relative`, …) and the `doc.*` namespace — the same grammar resolves identically across every surface.

Body interpolation runs after text replacement and page blocks have been applied. Within the body, all handlebar placeholders like `{{foo}}` or `{{bar}}` are replaced with their resolved values.

### Nullable directive targets

A whole-value `{{ ... }}` target on `::file`, `::code`, or `::url` has special
absence semantics. If it evaluates to `null` or the empty string, Darkmatter
skips that directive and records a compose warning instead of turning the
value into an authored empty target. Literal missing targets, `::file ""`, and
mixed targets such as `::file "{{dir}}/log.md"` keep their ordinary syntax and
path behavior.

Page blocks run before body interpolation, so the recommended optional-file
form removes the directive before its target is evaluated and suppresses the
runtime warning:

```md
::block when="file_exists(log)"
::file {{log}}
::end-block
```

Shell-approval preflight remains condition-blind: it scans every branch for
commands, but an evaluated-absent target contributes no transclusion edge.
Targets that depend on pending frontmatter shell expansion fail closed before
approval rather than disappearing and later revealing unapproved content.

- **Fallback Values**
    - if a template placeholder in the document refers to a frontmatter property that has no value then the default value of an empty string will be used.
    - when nothing defines that property — no frontmatter key, caller input, or schema property — composition also warns, because the name is most likely a typo (see [Missing Variables](#missing-variables)).
    - this default is suitable for some situations but not others so you are allowed to express a fallback you'd like to use instead with the following syntax, which also tells Darkmatter the absence is intended:

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
        - the first line will resolve to the frontmatter property `unknown` which if not set will default to an empty string (and warn, since nothing defines `unknown`)
        - the second line will resolve to the string literal "unknown"

- **Kebab-case Keys**

    - a frontmatter key such as `spec-name` is referenced by its name: `{{ spec-name }}` and `{{ doc.spec-name }}` both read it
    - a `-` joins an identifier only when it sits between identifier characters, so subtraction whose left operand is a name needs whitespace: `{{ iteration - 1 }}` subtracts, while `{{ iteration-1 }}` reads a key named `iteration-1`
    - bracket access still reaches keys the identifier form cannot spell, such as `{{ doc['release.channel'] }}`
    - see [Lexing § `-` inside an identifier](../topics/parsing/lexing.md#--inside-an-identifier) for the full rule

## Missing Variables

A reference to a property that has no value renders as an empty string, and
composition continues. Whether it also warns depends on whether anything
**knows** the name:

| The name is | Result |
| --- | --- |
| present in frontmatter, `--set`, inherited state, or another caller input — even as `null` or `""` | silent |
| declared by the effective schema (the document's `$schema`, the configured baseline, or a matched trigger) | silent; a `required` property left unset already failed schema validation |
| a reserved namespace (`ctx`, `env`, `doc`, …) or a runtime context name | silent |
| none of the above | warning `dm.expression.unknown_identifier`, once per name per source document |

```text
unknown identifier 'colour' at docs/page.md:5: no frontmatter key, caller
input, or schema property defines it, so it resolves to null
```

The warning is decided by the name's root: `{{ user.name }}` warns when nothing
defines `user`. It is suppressed where the author has handled the absence:

| Expression | Unresolved `x` warns? |
| --- | --- |
| `{{ x }}` | yes |
| `{{ x \|\| "d" }}` | no — primary of a fallback |
| `{{ a \|\| x }}` | yes, when the right-hand side is evaluated |
| `{{ x ? a : b }}` | no — ternary condition |
| `{{ x ? x : b }}` | no — the branch is guarded by `x` |
| `{{ a ? x : b }}` | yes, when the `x` branch is evaluated |
| `{{ is_null(x) }}`, `{{ is_empty(x) }}` | no — direct argument of an absence predicate or one of its aliases |
| `{{ is_empty(lower(x)) }}` | yes — the predicate no longer guards the lookup directly |
| `::block when="x"` | yes — a misspelled gate would silently hide content |

Composition warns only for what it evaluates, so an unchosen ternary branch or a
short-circuited fallback never warns. The same rule applies to frontmatter
interpolation, `when="…"` conditions, and `$()` ternaries. The language server
applies the same suppressions while you edit, but checks both ternary branches
(see [DMLS diagnostics](../../dmls/docs/diagnostics.md)).

## Failures

An expression that cannot be parsed, or that fails to evaluate (an unknown
function, a wrong argument type), is an authoring error. It **fails
composition** with exit code 1, naming the file, the authored line and
column, and the expression. Nothing is written to stdout, and the `{{ … }}` never reaches the
output. `ComposeOptions::with_fail_fast(false)` does not relax this; it governs
recoverable non-expression stages such as TOC linking.

A missing *value* is not a failure — see [Missing Variables](#missing-variables).


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


## Escaping an Opener with a Backslash

A document that discusses another template language (Handlebars, Liquid, Jinja, Mustache) can opt a span out of scanning without switching to `{{{ … }}}`:

```md
Handlebars writes a variable as \{{ name }} and a partial as \{\{> header }}.
```

- `\{{` and `\{\{` are both inert. `\{{` is an explicit scanner rule; `\{\{` never forms a `{{` opener in the first place.
- The scanner counts the run of backslashes directly before `{{`. An **odd** run escapes the opener. An **even** run escapes itself, so `\\{{ x }}` still interpolates `x`.
- Compose **preserves every backslash**. The Markdown renderer resolves the escape under CommonMark's backslash-before-punctuation rule, so `\{{` displays as `{{`.
- Parity is counted on the text the scanner receives, after its owning format has decoded it. Inside a double-quoted YAML scalar, write `"\\{{ x }}"` so the decoded value holds one backslash.

## Braces Inside String Literals

A quoted string literal inside an expression is text. The lexer copies it verbatim, so `{{ "in {{ area }}" }}` does not contain a nested expression at parse time. Whether those braces are ever interpolated depends on the surface:

- **Rescanning surfaces** — the document body and mixed frontmatter strings (`"Hello {{ name }}"`) — run a fixpoint loop that rescans the text each pass produced, up to a fixed depth. The literal's braces land in the output and resolve on the next pass.
- **Single-pass surfaces** — a scalar whose trimmed content is **exactly one** `{{ … }}` span — take the whole-value path, which parses and evaluates once and keeps the typed result. Nothing rescans the result, so a literal's braces survive as raw text. Claudine's lifecycle values (communication fields, stack operands, `proxy … with` values, and `when`/`while`/`until` predicates) are single-pass, and Claudine refuses a nested span there before any provider starts. An ordinary whole-value frontmatter key keeps the raw braces too (`r: "{{ 'b {{ name }}' }}"` composes to `b {{ name }}`), although a body reference such as `{{ r }}` then resolves them on its own rescan.

Write the value with `+` instead. It resolves identically on both kinds of surface:

```yaml
# Never resolves on a single-pass surface
say: '{{ area ? "Review in {{ area }} completed" : "Review completed" }}'

# Resolves everywhere
say: '{{ area ? "Review in " + area + " completed" : "Review completed" }}'
```

`lint_expression` (in `compose::expression::lint`) finds the defect in authored source and offers the complete `+` rewrite. The rewrite does the following:

- keeps each literal piece byte-for-byte in its original quote character;
- parenthesizes any lifted span that is not atomic, so a ternary or `+` expression keeps its meaning;
- anchors a span with `"" + …` wherever `+` could otherwise add two numbers instead of concatenating;
- is accepted only when it re-parses to the intended tree.

Array- and object-valued spans receive a suggestion like any scalar: both rendering paths stringify aggregates as compact JSON, so the rewrite composes the same text. The suggestion is withheld only when no equivalent rewrite exists. That covers a nested span that does not parse, a literal used as an object key, a `{{{ … }}}` literal sharing the string, and a doubly nested literal.

Callers decide whether a surface is single-pass; the lint only describes syntax. A `{{{ … }}}` inside a quoted literal is not a nested span.


## Implementation

The current implementation uses a source-first scanner approach, rescanned to a fixed point (see [Braces Inside String Literals](#braces-inside-string-literals) for the whole-value exception):

- A scanner finds `{{ ... }}` spans in the document body, and also recognizes `{{{ ... }}}` interpolation literals. Inline code spans (single backticks) are interpolated by default, since the templating pattern `` `var_{{ phase }}` `` is a common use case, and literals inside inline code convert to literal `{{{ ... }}}` text. Fenced and indented code blocks are skipped.
- Each expression is parsed with a dedicated tokenizer and evaluator
- The interpolation context is built from the effective state (frontmatter + external state), `ctx.*` runtime values, and `env.*` environment variables
- Replacements are applied from the end of the string backward to preserve offsets
- Literal conversion (`{{{ ... }}}` → `{{ ... }}`) happens after the final scan pass over a surface, so a literal introduced by a replacement value is also converted exactly once
- A failing expression fails document composition. Under the lenient policy that only `compose_subtree(..., SubtreeStrictness::Lenient)` and preflight discovery use, it is left in place and reported once, and later scan passes do not evaluate it again. Coded warnings are reported once per issue: an unknown `ctx.*` group warns once per source document however often it is referenced, and a transcluded document's issues stay separate from its parent's

See the source modules:

- `darkmatter/lib/src/markdown/compose/interpolation/` — lexer, evaluator, rewriter
- `darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs` — frontmatter-specific interpolation engine
